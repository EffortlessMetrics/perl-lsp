# Large-Workspace Testing Guide

Practical guidance for contributors who need to test and validate behavior at
scale. CI fixtures are intentionally small; large-workspace behavior must be
exercised separately.

## When You Need This Guide

- You are optimizing workspace indexing performance
- You changed `WorkspaceIndex`, `DocumentStore`, or `ProductionIndexCoordinator`
- A performance issue was reported against a workspace with 5k-10k+ Perl files
- You want to establish a baseline before and after a change

---

## Generating a Synthetic Workspace

The benchmarks in `crates/perl-workspace-index/benches/workspace_index_benchmark.rs`
show the standard pattern: write Perl source into a `TempDir` using the
helpers already in `perl-tdd-support`.

For ad-hoc manual testing, the following shell snippet generates a realistic
workspace at different scales. Save it to `scripts/gen-large-workspace.sh`
and run it once:

```bash
#!/usr/bin/env bash
# Usage: bash scripts/gen-large-workspace.sh <output-dir> <file-count>
# Example: bash scripts/gen-large-workspace.sh /tmp/big-workspace 5000
set -euo pipefail

DIR="${1:?output dir required}"
COUNT="${2:-1000}"
mkdir -p "$DIR/lib/App"

for i in $(seq 1 "$COUNT"); do
  PKG="App::Module${i}"
  FILE="$DIR/lib/App/Module${i}.pm"
  cat > "$FILE" <<EOF
package $PKG;
use strict;
use warnings;
our \$VERSION = '0.01';

sub new {
    my \$class = shift;
    return bless {}, \$class;
}

sub compute_${i} {
    my (\$self, \$x) = @_;
    return \$x * ${i};
}

sub describe_${i} {
    return "Module number ${i}";
}

1;
EOF
done

echo "Generated $COUNT files in $DIR"
```

Size guide:

| Scale       | File count | Approx. symbols | Expected index time |
|-------------|------------|-----------------|---------------------|
| Small       | 100        | ~500            | <50ms               |
| Medium      | 1 000      | ~5 000          | <500ms              |
| Large       | 5 000      | ~25 000         | <3s                 |
| Extra-large | 10 000     | ~50 000         | <10s                |

The numbers above are targets, not guarantees. Measure on your machine.

---

## Writing a Large-Workspace Integration Test

Add a test under `crates/perl-workspace-index/tests/` that mirrors the
benchmark pattern but asserts correctness rather than timing:

```rust
use perl_workspace_index::workspace_index::WorkspaceIndex;
use perl_tdd_support::must;
use tempfile::TempDir;
use url::Url;
use std::fs;

#[test]
fn large_workspace_symbol_lookup_finds_all_subs() {
    let dir = must(TempDir::new());
    let index = WorkspaceIndex::new();
    let n = 500; // keep CI fast; use 5_000 locally

    for i in 0..n {
        let path = dir.path().join(format!("module{i}.pm"));
        let src = format!(
            "package App::Mod{i};\nsub func_{i} {{ return {i}; }}\n1;\n"
        );
        must(fs::write(&path, &src));
        let uri = must(Url::from_file_path(&path));
        must(index.index_file(uri, src));
    }

    // Every sub must be discoverable
    for i in 0..n {
        let name = format!("func_{i}");
        let hits = index.find_symbols(&name);
        assert!(!hits.is_empty(), "missing symbol {name}");
    }
}
```

Run it in isolation before adding to the test suite:

```bash
export CARGO_TARGET_DIR="/tmp/agent-$(git branch --show-current | tr '/' '-')-target"
cargo test -p perl-workspace-index large_workspace_symbol_lookup -- --nocapture
```

---

## Performance Baselines by Workspace Size

The authoritative baselines live in `benchmarks/results/baseline.json`.
To record a new one:

```bash
just bench
just bench-baseline "$(date +%Y-%m-%d)"
```

To compare your branch against the baseline:

```bash
just bench-compare
```

Expected latency targets (from `docs/reference/PERFORMANCE_SLO.md`):

| Operation              | P95 target | Hard limit |
|------------------------|------------|------------|
| Symbol lookup          | 50µs       | 200µs      |
| Incremental index update | 1ms      | 5ms        |
| Initial index (1 000 files) | 500ms | 3s        |
| Workspace symbol search | 50ms      | 150ms      |

---

## Resource Limit Configuration During Testing

`IndexResourceLimits` controls when the index enters the `Degraded` state.
For stress testing you may want to raise or lower these:

```rust
use perl_workspace_index::workspace_index::IndexResourceLimits;

let limits = IndexResourceLimits {
    max_files: 50_000,
    max_total_symbols: 1_000_000,
    max_symbols_per_file: 500,
    workspace_scan_deadline_ms: 60_000,
    reference_search_deadline_ms: 5_000,
    ..Default::default()
};
```

For testing degradation behavior specifically, set `max_files` below the
size of your test workspace to force a controlled transition to `Degraded`.

---

## CI Integration Notes

Large-workspace tests are not part of the default `cargo test` run because
they exceed the 30-second CI budget. Options:

1. **Label-gated**: Add a `#[ignore]` attribute and run with `cargo test --
   large_workspace_symbol_lookup_finds_all_subs --ignored` in a separate CI
   lane.
2. **Benchmark lane**: Add to the criterion benchmark suite under the
   `workspace` feature flag (already gated in `Cargo.toml`).
3. **Nightly only**: Register the lane in `.ci/` alongside `ci-full`.

Prefer option 2 for performance-sensitive cases; prefer option 1 when you
need correctness assertions across many files.

---

## See Also

- `PROFILING_GUIDE.md` — how to measure where time is actually spent
- `MEMORY_PATTERNS.md` — how memory scales with symbol count
- `docs/reference/PERFORMANCE_SLO.md` — authoritative SLO targets
- `docs/how-to/PERFORMANCE_TUNING.md` — end-user configuration guide
