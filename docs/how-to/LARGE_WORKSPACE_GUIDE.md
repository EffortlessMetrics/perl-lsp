# Large-Workspace Testing, Profiling, and Troubleshooting Guide

This guide is for contributors working with or testing against large Perl workspaces
(5 000–10 000+ files). Performance issues often only surface at scale, so this document
covers how to generate realistic test data, profile the server, interpret memory
behaviour, and diagnose the failures that appear only after hours of editor use.

## Table of Contents

- [Generating Synthetic Large Workspaces](#generating-synthetic-large-workspaces)
  - [Quick Script](#quick-script)
  - [Fixture Size Guidelines](#fixture-size-guidelines)
  - [Realistic Module Shapes](#realistic-module-shapes)
- [Performance Profiling](#performance-profiling)
  - [Criterion Benchmarks](#criterion-benchmarks)
  - [cargo flamegraph](#cargo-flamegraph)
  - [DHAT Heap Profiling](#dhat-heap-profiling)
  - [tracing / tokio-console](#tracing--tokio-console)
  - [Interpreting Results](#interpreting-results)
  - [Common Performance Pitfalls](#common-performance-pitfalls)
- [Memory Patterns at Scale](#memory-patterns-at-scale)
  - [WorkspaceIndex Scaling](#workspaceindex-scaling)
  - [AST Cache Behaviour](#ast-cache-behaviour)
  - [Common Memory Anti-Patterns](#common-memory-anti-patterns)
  - [Optimization Techniques](#optimization-techniques)
- [Troubleshooting Large Workspaces](#troubleshooting-large-workspaces)
  - [Slow Startup](#slow-startup)
  - [High Memory After Hours of Use](#high-memory-after-hours-of-use)
  - [Slow Completion Latency](#slow-completion-latency)
  - [Degraded After Long Sessions](#degraded-after-long-sessions)
  - [Diagnosis Workflow](#diagnosis-workflow)
- [See Also](#see-also)

---

## Generating Synthetic Large Workspaces

### Quick Script

The fastest way to produce a test corpus is a short shell script. The script below
writes `N` Perl modules under `tmp/large-workspace/` so you can point the LSP server
at them without touching real project code.

```bash
#!/usr/bin/env bash
# scripts/gen-large-workspace.sh — generate a synthetic Perl workspace
# Usage: bash scripts/gen-large-workspace.sh [file-count] [out-dir]
set -euo pipefail

N=${1:-1000}
DEST=${2:-tmp/large-workspace}

mkdir -p "$DEST/lib/App"

for i in $(seq 1 "$N"); do
  NAMESPACE="App::Module${i}"
  FILE="$DEST/lib/App/Module${i}.pm"

  cat > "$FILE" <<PERL
package ${NAMESPACE};
use strict;
use warnings;

our \$VERSION = '0.01';

sub new {
    my (\$class, %args) = @_;
    return bless { id => ${i}, %args }, \$class;
}

sub process {
    my (\$self, \$data) = @_;
    return \$self->_transform(\$data);
}

sub _transform {
    my (\$self, \$value) = @_;
    return \$value * \$self->{id};
}

sub describe {
    my \$self = shift;
    return "Module ${i}: id=\$self->{id}";
}

1;
PERL
done

echo "Generated $N modules in $DEST/lib/"
```

Run with:

```bash
# 1 000 files  (~5 MB, good for quick profiling)
bash scripts/gen-large-workspace.sh 1000

# 5 000 files  (~25 MB, mid-scale test)
bash scripts/gen-large-workspace.sh 5000 tmp/workspace-5k

# 10 000 files (~50 MB, stress test)
bash scripts/gen-large-workspace.sh 10000 tmp/workspace-10k
```

Then open the workspace in your editor and watch the LSP logs during indexing.

### Fixture Size Guidelines

| Scale | File Count | Approx. Symbols | Purpose |
|-------|-----------|-----------------|---------|
| Small | 10–100 | 500–5 000 | Unit tests, CI fast path |
| Medium | 1 000 | 50 000 | Regression checks, benchmark baselines |
| Large | 5 000 | 250 000 | Performance profiling |
| Stress | 10 000+ | 500 000+ | Memory ceiling, SLO validation |

Keep CI test fixtures in the small–medium range. Large and stress fixtures live in
`tmp/` (git-ignored) and are run manually or in a dedicated slow-CI lane.

### Realistic Module Shapes

The script above produces uniform stubs. For more realistic tests, vary the shape:

```perl
# Heavy inheritance — stresses module resolution
package App::Deep::Nesting::Module1;
use parent qw(App::Base App::Role::Printable);
```

```perl
# Many symbols per file — stresses symbol table
package App::Constants;
use constant FOO_A => 1;
use constant FOO_B => 2;
# ... 200 more constants
```

```perl
# Large file (>1 000 lines) — stresses incremental re-parse cost
package App::Monolith;
# single package, many subs
```

Mixing shapes exposes different bottlenecks than a uniform corpus.

---

## Performance Profiling

### Criterion Benchmarks

Several crates include Criterion benchmarks. The canonical entry points:

```bash
export CARGO_TARGET_DIR="/tmp/bench-target"

# Workspace indexing at different corpus sizes
cargo bench -p perl-workspace-index --features workspace

# Parser throughput
cargo bench -p perl-parser

# Completion latency
cargo bench -p perl-lsp-completion

# Lexer throughput
cargo bench -p perl-lexer
```

Save a baseline before your change and compare afterwards:

```bash
# Save baseline on the current branch
cargo bench -p perl-workspace-index --features workspace -- --save-baseline before

# Make your change, then compare
cargo bench -p perl-workspace-index --features workspace -- --baseline before
```

The project-level `just bench` target runs all benchmarks and writes structured output
to `benchmarks/results/latest.json`:

```bash
just bench          # full suite
just bench-quick    # smoke run (~30 s)
just bench-compare  # diff vs. stored baseline
```

### cargo flamegraph

`cargo flamegraph` records CPU time with perf (Linux) or DTrace (macOS) and produces
an SVG flame graph you can open in a browser.

```bash
# Install once
cargo install flamegraph

# On Linux, allow perf events for your user (if needed)
echo -1 | sudo tee /proc/sys/kernel/perf_event_paranoid

# Profile workspace indexing
cargo flamegraph --root \
  -p perl-lsp-rs \
  -- --stdio < scripts/lsp-index-replay.json \
  > flamegraph.svg

# Open the result
xdg-open flamegraph.svg          # Linux
open flamegraph.svg               # macOS
```

Where `scripts/lsp-index-replay.json` is a recorded sequence of LSP JSON-RPC messages
(initialize + didOpen for each file). You can record one with:

```bash
RUST_LOG=perl_lsp=trace perllsp --stdio 2>trace.log
```

Then extract the request stream from the trace log and replay it.

To profile a specific benchmark instead:

```bash
cargo flamegraph --root \
  -p perl-workspace-index \
  --bench workspace_index_benchmark \
  --features workspace \
  -- --bench
```

### DHAT Heap Profiling

DHAT tracks heap allocations and is the fastest way to find "who is allocating the
most bytes" in a run.

```bash
# Build with DHAT support (Valgrind must be installed)
RUSTFLAGS="-g" cargo build --release -p perl-lsp-rs

# Run under DHAT — produces dhat.out.<pid>
valgrind --tool=dhat --dhat-out-file=dhat.out \
  ./target/release/perllsp --stdio < scripts/lsp-index-replay.json

# View the result in the DHAT viewer
# Upload dhat.out to https://nnethercote.github.io/dh_view/dh_view.html
```

For targeted heap profiling without Valgrind, use the `dhat` crate directly in a
benchmark or test binary (add it as a `[dev-dependencies]`):

```toml
# Cargo.toml
[dev-dependencies]
dhat = "0.3"
```

```rust
// in your test/bench
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

fn main() {
    let _profiler = dhat::Profiler::new_heap();
    // ... run workload
}
```

### tracing / tokio-console

The LSP server uses `tracing` for structured logging. Enable spans to see where async
time goes:

```bash
# Enable trace-level spans for workspace and parser subsystems
RUST_LOG=perl_lsp::workspace=trace,perl_lsp::handler=trace \
  perllsp --stdio

# Full trace with timestamps
RUST_LOG=perl_lsp=trace RUST_LOG_STYLE=always \
  perllsp --stdio 2>trace.log
```

For real-time async task inspection, connect `tokio-console`:

```bash
# In one terminal — start the server with tokio-console support
RUSTFLAGS="--cfg tokio_unstable" \
  cargo run -p perl-lsp-rs --features tokio-console -- --stdio

# In another terminal
cargo install tokio-console
tokio-console
```

`tokio-console` shows live task timings, waker counts, and poll durations — useful for
finding tasks that hold locks too long or are polled at high frequency.

### Interpreting Results

**Flame graphs**: Wide frames at the top are hot. Look for:
- `WorkspaceIndex::index_file` taking more than 20 ms per file
- `Parser::parse` called repeatedly on unchanged source
- HashMap operations showing up wide (signals table resizing)

**DHAT output**: Sort by "total bytes allocated". Look for:
- String allocations in tight loops (indicates missing string interning)
- Vec allocations with `capacity 0 -> 1 -> 2 -> 4 -> ...` (no pre-allocation)
- `Arc` clones of large AST nodes

**Criterion output**: Focus on `thrpt` (throughput) for bulk operations and `time`
for individual request latency. A regression is anything above the hard limits in
`docs/how-to/PERFORMANCE_TUNING.md`.

### Common Performance Pitfalls

| Pattern | Problem | Fix |
|---------|---------|-----|
| `clone()` on `String` in hot path | Heap allocation per call | Use `&str` or `Arc<str>` |
| `HashMap::insert` without `with_capacity` | Repeated resizing | Pre-size with `HashMap::with_capacity(n)` |
| `Vec::push` without pre-allocation | O(n log n) realloc | Use `Vec::with_capacity(n)` |
| Re-parsing on every keypress | Full parse per edit | Gate on document version change |
| Holding `RwLock` across `await` | Async deadlock / contention | Drop lock before `.await` |
| Cloning `Arc<Node>` for each caller | Unnecessary ref-counting | Return `&Node` where lifetimes allow |
| `.to_string()` in symbol lookup | Alloc on every lookup | Intern symbols with `StringInterner` |

---

## Memory Patterns at Scale

### WorkspaceIndex Scaling

`WorkspaceIndex` stores symbols under both their qualified name (`Package::sub`) and
their bare name (`sub`) for dual-indexed lookup (see CLAUDE.md "Dual indexing"). This
means memory scales at approximately **2x the symbol count**.

Rough empirical baselines (these will vary with module shape):

| Files | Symbols | Approx. RSS |
|-------|---------|-------------|
| 100 | 5 000 | ~30 MB |
| 1 000 | 50 000 | ~80 MB |
| 5 000 | 250 000 | ~250 MB |
| 10 000 | 500 000 | ~480 MB |

If RSS grows beyond these values, suspect unbounded caches or string duplication.

The resource limits that cap this growth are in `IndexResourceLimits`:

```rust
// crates/perl-workspace-index/src/workspace/workspace_index.rs
pub struct IndexResourceLimits {
    pub max_indexed_files: usize,      // default: 10 000
    pub max_total_symbols: usize,      // default: 500 000
    pub workspace_scan_deadline_ms: u64,
}
```

Tune these via the LSP configuration:

```json
{
  "perl": {
    "limits": {
      "maxIndexedFiles": 5000,
      "maxTotalSymbols": 250000
    }
  }
}
```

### AST Cache Behaviour

After parsing, the server stores ASTs in a `BoundedLruCache` keyed by URI. The cache
evicts least-recently-used entries when it reaches `astCacheMaxEntries`. Key facts:

- **Cache hit**: No reparse, constant time lookup
- **Cache miss**: Full reparse from source string (O(n) in source length)
- **Eviction**: LRU with optional TTL; evicted entries are reparsed on next access
- **Large files**: A single 10 000-line file can consume 5–20 MB of AST cache memory

If the cache is too large, memory grows; if too small, latency spikes. A good starting
point is `astCacheMaxEntries = 100` (roughly 1 AST per open editor tab, plus headroom).

To check effective cache behaviour, look for the `ast_cache` span in trace logs:

```
TRACE perl_lsp::workspace: ast_cache hit uri="file:///lib/Foo.pm"
TRACE perl_lsp::workspace: ast_cache miss uri="file:///lib/Bar.pm" reason=evicted
```

### Common Memory Anti-Patterns

**Unbounded symbol accumulation**

Symbols are never removed unless the file is closed or `maxTotalSymbols` is hit. If
your workflow opens many files and never closes them, the index grows unboundedly.
Ensure editors send `textDocument/didClose` on buffer close.

**String duplication in the index**

Each symbol name stored as a `String` is a separate heap allocation. With 500 000
symbols, even 20 bytes average per name is 10 MB. The `StringInterner` pattern
replaces owned `String` with an integer key into a shared pool:

```rust
// Before
struct Symbol { name: String }

// After — symbols with the same text share one allocation
use string_interner::{StringInterner, DefaultSymbol};
let mut interner = StringInterner::default();
let key: DefaultSymbol = interner.get_or_intern("MyPackage::helper");
```

**Circular Arc references**

If an `Arc<WorkspaceIndex>` holds a `Vec<Arc<Symbol>>` and those symbols hold back-
references to the index, the reference count never reaches zero. Use `Weak<T>` for
back-references.

**Cache without capacity hints**

A `Vec` or `HashMap` grown from empty doubles in size on each resize. Pre-sizing with
`with_capacity` halves allocations for large collections:

```rust
// Before
let mut map = HashMap::new();

// After
let mut map = HashMap::with_capacity(expected_symbol_count);
```

### Optimization Techniques

| Technique | When to Use | Expected Gain |
|-----------|------------|---------------|
| `StringInterner` | Many repeated symbol names | 30–60% string memory |
| `with_capacity` on maps/vecs | Known size at construction time | Halves resizing |
| `Arc<str>` instead of `String` | Symbols shared across subsystems | Avoids clone cost |
| Capacity hints on `BoundedLruCache` | Predictable working set | Fewer evictions |
| `Weak<T>` back-references | Parent-child cycles in AST / index | Prevents leaks |
| Lazy indexing | Files outside active include paths | Skips unreachable code |

---

## Troubleshooting Large Workspaces

### Slow Startup

**Symptoms**: LSP takes > 30 s before responding to the first completion or hover.

**Diagnosis steps**:

1. Enable debug logging and measure time-to-ready:

   ```bash
   RUST_LOG=perl_lsp=debug perllsp --stdio 2>startup.log
   grep "workspace.*ready\|index.*complete\|IndexPhase" startup.log | head -20
   ```

2. Count files the server is trying to index:

   ```bash
   find . -name "*.pm" -o -name "*.pl" | wc -l
   ```

3. Check whether the workspace root is set too broadly (e.g., the home directory
   instead of the project root).

**Common causes and fixes**:

| Cause | Fix |
|-------|-----|
| Workspace root too broad | Set a narrower `includePaths` |
| `useSystemInc: true` on large `@INC` | Set `useSystemInc: false` |
| Network filesystem | Copy sources to local SSD for development |
| `maxIndexedFiles` not capped | Set `maxIndexedFiles` to a sensible limit |
| Deep `node_modules` or `vendor` in path | Add ignore patterns for non-Perl dirs |

### High Memory After Hours of Use

**Symptoms**: RSS grows beyond ~500 MB after a long session; editor becomes sluggish;
system swap activity increases.

**Diagnosis steps**:

1. Watch RSS over time:

   ```bash
   # Poll every 30 s while the editor is open
   while true; do
     ps -o pid,rss,vsz,comm -p "$(pgrep perllsp)" 2>/dev/null
     sleep 30
   done
   ```

2. Trigger a workspace re-index and watch if memory drops (cache invalidation) or
   keeps growing (leak):

   ```
   # VS Code: Ctrl+Shift+P → "Perl: Restart Language Server"
   ```

3. If memory does not drop after restart, the leak is in the on-disk cache or a
   persistent external resource.

**Common causes and fixes**:

| Cause | Fix |
|-------|-----|
| `astCacheMaxEntries` too high | Reduce to 50–100 |
| Files never closed (`didClose` not sent) | Check editor LSP plugin version |
| Unbounded symbol accumulation | Cap with `maxTotalSymbols` |
| String duplication | Profile with DHAT, apply `StringInterner` |

### Slow Completion Latency

**Symptoms**: Completion popup appears after > 500 ms; typing feels laggy.

**Diagnosis steps**:

1. Time individual completion requests in the LSP log:

   ```bash
   RUST_LOG=perl_lsp::handler::completion=debug perllsp --stdio 2>completion.log
   grep "textDocument/completion" completion.log | grep -E "[0-9]+ms"
   ```

2. Check whether the latency is in symbol lookup or in result serialization:

   ```bash
   RUST_LOG=perl_lsp::workspace=trace perllsp --stdio 2>trace.log
   grep "find_symbols\|completion_items" trace.log | tail -20
   ```

**Common causes and fixes**:

| Cause | Fix |
|-------|-----|
| `completionCap` too high (thousands of items) | Reduce to 50–100 |
| Symbol lookup doing linear scan | Verify dual-index is built (check for `index_file` errors in log) |
| `resolutionTimeout` too permissive | Reduce to 25–50 ms |
| Cache miss on every keystroke | Increase `astCacheMaxEntries` |

### Degraded After Long Sessions

**Symptoms**: The `IndexStateMachine` enters `Degraded` state; completions become
stale; hover shows incorrect type information.

The state machine has eight states: `Idle`, `Initializing`, `Building`, `Updating`,
`Invalidating`, `Ready`, `Degraded`, `Error`. It enters `Degraded` when resource limits
are exceeded or when incremental updates fail to apply cleanly.

**Diagnosis steps**:

1. Look for state transitions in the log:

   ```bash
   grep -E "state.*Degraded|IndexState|transition" /path/to/lsp.log | tail -30
   ```

2. Trigger a full re-index to exit `Degraded`:

   ```
   # VS Code: Ctrl+Shift+P → "Perl: Reindex Workspace"
   # Neovim: :LspRestart
   # Any editor: restart the LSP server
   ```

3. If the server frequently enters `Degraded`, it is hitting resource limits. Review
   `maxTotalSymbols` and `maxIndexedFiles` in your configuration.

**Remediation**:

- Increase `maxTotalSymbols` if the workspace is legitimately large
- Reduce `maxIndexedFiles` and add explicit `includePaths` to stay in `Ready`
- File a bug if `Degraded` is entered without hitting documented limits

### Diagnosis Workflow

Use this flowchart as a starting point for any large-workspace problem:

```
Is startup slow (>30s)?
  Yes → count files, check workspace root breadth, check network FS
  No  → continue

Is RSS growing over time?
  Yes → run DHAT, check astCacheMaxEntries, check for missing didClose
  No  → continue

Is completion latency >500ms?
  Yes → enable completion trace logs, check completionCap, check dual-index health
  No  → continue

Is IndexState = Degraded?
  Yes → check resource limits, trigger re-index, increase limits if needed
  No  → problem is likely in editor integration, not the LSP server
```

**Metrics to collect for a bug report**:

```bash
# Server version
perllsp --version

# Health check
perllsp --health

# File count in workspace
find . \( -name "*.pm" -o -name "*.pl" -o -name "*.t" \) | wc -l

# Peak RSS
/usr/bin/time -v perllsp --stdio < /dev/null 2>&1 | grep "Maximum resident"

# Startup trace (first 30 lines mentioning index)
RUST_LOG=perl_lsp=debug perllsp --stdio 2>&1 | grep -E "index|phase|state" | head -30
```

---

## See Also

- [Performance Tuning Guide](PERFORMANCE_TUNING.md) — configuration knobs by workspace size
- [Troubleshooting Guide](TROUBLESHOOTING.md) — general server troubleshooting
- [Incremental Parsing Guide](INCREMENTAL_PARSING_GUIDE.md) — how re-parse cost scales with edits
- [Workspace Refactoring Guide](WORKSPACE_REFACTORING_GUIDE.md) — structural workspace changes
- [Performance SLO](../reference/PERFORMANCE_SLO.md) — latency targets and hard limits
