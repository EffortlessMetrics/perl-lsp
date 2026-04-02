# Profiling Guide for Contributors

Step-by-step instructions for identifying where time is spent in the LSP
server and workspace indexer. Read this before filing a performance-related
issue or submitting a performance fix.

---

## Tool Overview

| Tool | What it measures | Platform |
|------|-----------------|----------|
| `cargo criterion` | Statistical micro-benchmarks | All |
| `cargo flamegraph` | CPU time, call-tree | Linux/macOS |
| `heaptrack` | Heap allocations and lifetimes | Linux |
| `DHAT` (via Valgrind) | Heap profiling | Linux |
| `samply` | CPU sampling, works on macOS | macOS |
| `perf` + `hotspot` | CPU sampling | Linux |

Start with criterion because it requires no extra tooling and is already
integrated. Move to flamegraph or heaptrack when you need to understand
_where_ inside the hot path allocations or CPU time accumulate.

---

## Quick Start: Criterion Benchmarks

The workspace index has a benchmark suite in
`crates/perl-workspace-index/benches/workspace_index_benchmark.rs`.

```bash
# Run all benchmarks and save results
just bench

# Quick smoke test (~30s)
just bench-quick

# Compare against saved baseline
just bench-compare

# Strict mode: fail if regression detected
just bench-compare-strict
```

To run a single benchmark by name:

```bash
cargo bench -p perl-workspace-index --features workspace \
    -- "initial index small workspace"
```

Criterion writes HTML reports to `target/criterion/`. Open
`target/criterion/report/index.html` in a browser to inspect the
violin plots and regression analysis.

---

## CPU Profiling with Flamegraph

### Linux

```bash
# Install once
cargo install flamegraph
# Some distributions also need: apt install linux-perf / pacman -S perf

# Run the workspace index benchmark under flamegraph
CARGO_PROFILE_RELEASE_DEBUG=true \
cargo flamegraph -p perl-workspace-index \
    --features workspace \
    --bench workspace_index_benchmark \
    -- --bench

# Open the SVG
xdg-open flamegraph.svg
```

### macOS

`cargo flamegraph` calls `dtrace` on macOS. It usually requires `sudo`:

```bash
sudo cargo flamegraph -p perl-workspace-index \
    --features workspace \
    --bench workspace_index_benchmark \
    -- --bench
```

Alternatively, use `samply`:

```bash
cargo install samply
samply record cargo bench -p perl-workspace-index --features workspace \
    -- --bench
# samply opens a browser-based flamechart automatically
```

### Reading the Flamegraph

- Wide bars = high cumulative CPU time
- Look for unexpected depth in `WorkspaceIndex::index_file` or
  `WorkspaceIndex::find_symbols`
- `parking_lot::RwLock` contention shows as `lock_contended` or
  `futex_wait` in the stacks

---

## Memory Profiling with Heaptrack

Heaptrack traces every allocation and shows heap usage over time. It is
available on Linux via most package managers.

```bash
# Install
apt install heaptrack heaptrack-gui  # Debian/Ubuntu
# or build from source: https://github.com/KDE/heaptrack

# Build with debug symbols
CARGO_PROFILE_RELEASE_DEBUG=true cargo build \
    -p perl-workspace-index --release

# Run the LSP server under heaptrack while exercising a large workspace
heaptrack ./target/release/perllsp --stdio < /dev/null
# OR: attach a profiling harness binary

# Open the GUI
heaptrack_gui heaptrack.perllsp.*.gz
```

For a more focused view, write a small harness binary that indexes a
synthetic workspace and exits:

```rust
// bin/heaptrack-harness.rs (temporary, not committed)
use perl_workspace_index::workspace_index::WorkspaceIndex;
use std::fs;
use tempfile::TempDir;
use url::Url;

fn main() {
    let dir = TempDir::new().unwrap();
    let index = WorkspaceIndex::new();
    for i in 0..10_000 {
        let path = dir.path().join(format!("m{i}.pm"));
        let src = format!("package M{i};\nsub s_{i} {{}}\n1;\n");
        fs::write(&path, &src).unwrap();
        let uri = Url::from_file_path(&path).unwrap();
        index.index_file(uri, src).ok();
    }
    eprintln!("Done indexing {} files", 10_000);
}
```

The heaptrack GUI shows:
- Total allocated bytes over time
- Leaked allocations at exit
- Peak heap usage
- Per-call-stack allocation counts

---

## Profiling a Specific LSP Handler

To profile one handler (e.g., `workspace/symbol`), use the LSP integration
tests with a large fixture and a timing wrapper:

```bash
# Enable trace logging to measure handler latency
RUST_LOG=perl_lsp=trace cargo test -p perl-lsp-rs \
    -- workspace_symbol --nocapture 2>&1 | grep "handler.*ms"
```

For systematic measurement, add a `Span` around the handler under test and
read the output from the SLO tracker:

```rust
// In your test — use the re-export path from workspace::mod
use perl_workspace_index::workspace::{OperationResult, OperationType, SloConfig, SloTracker};
use std::sync::Arc;

let tracker = Arc::new(SloTracker::new(SloConfig::default()));
let start = tracker.start_operation(OperationType::WorkspaceSymbol);
let _ = coordinator.find_symbols("query");
tracker.record_operation(start, OperationResult::Success);
let stats = tracker.statistics(OperationType::WorkspaceSymbol);
println!("stats = {stats:?}");
```

---

## Common Bottlenecks and How to Spot Them

### HashMap Resize Storms

`WorkspaceIndex` uses `HashMap` for its symbol tables. When many files are
indexed in rapid succession, the maps may resize repeatedly.

**Symptom**: `std::collections::hash_map::resize` appears wide in the
flamegraph during `index_file` calls.

**Mitigation**: Pre-allocate with `HashMap::with_capacity`. When you know
the approximate symbol count upfront (e.g., from a previous run stored in
the state machine), pass it to `WorkspaceIndex::with_capacity`.

### Clone vs. Borrow

String cloning shows as `alloc::string::String::clone` in the flamegraph.

**Where to look**: The symbol extraction pass in `workspace_index.rs` builds
`String` keys for every symbol. If you see many clones in this path, audit
whether an `Arc<str>` or a string-interning pass would help.

### Async Task Overhead

The LSP server dispatches handler tasks via Tokio. In a large workspace,
the `workspace/symbol` handler may spawn many subtasks.

**Symptom**: `tokio::runtime::task::harness` appears repeatedly at the top
of flamegraph stacks for short-lived futures.

**Mitigation**: Batch sub-queries inside a single task rather than spawning
one per file.

### Lock Contention

`WorkspaceIndex` uses `parking_lot::RwLock`. If many threads try to take a
write lock simultaneously (e.g., during parallel indexing), they queue up.

**Symptom**: `parking_lot::raw_rwlock::RawRwLock::lock_exclusive` appears
in the flamegraph alongside `futex_wait`.

**Mitigation**: Reduce write-lock scope; prefer coarse-grained batch writes
over per-symbol locking.

---

## Interpreting Criterion Output

Criterion prints:

```
initial index small workspace (5 files)
                        time:   [1.2345 ms 1.2567 ms 1.2789 ms]
                        change: [-3.4512% -1.2345% +0.8765%] (p = 0.21 > 0.05)
                        No change in performance detected.
```

- The three values are the lower bound, estimate, and upper bound of the
  95% confidence interval.
- `change` shows the percentage difference versus the saved baseline.
- `p = 0.21 > 0.05`: the change is not statistically significant.

A regression is flagged when the lower bound of the change interval is
positive and `p < 0.05`. `just bench-compare-strict` exits non-zero in
this case.

---

## See Also

- `TESTING_GUIDE.md` — how to generate large test workspaces
- `MEMORY_PATTERNS.md` — memory scaling characteristics
- `docs/reference/PERFORMANCE_MONITORING.md` — automated regression alerts
- `docs/benchmarks/BENCHMARK_DESIGN.md` — benchmark architecture
- `docs/reference/PERFORMANCE_SLO.md` — SLO targets
