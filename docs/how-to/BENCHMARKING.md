# Benchmarking Guide

This guide documents the LSP latency benchmarking infrastructure, including the real-project latency suite introduced in issue #4196.

## Table of Contents

- [Overview](#overview)
- [Benchmark Types](#benchmark-types)
- [Real-Project Latency Suite](#real-project-latency-suite)
  - [Fixtures](#fixtures)
  - [Metrics](#metrics)
  - [Running the Suite](#running-the-suite)
  - [Baseline File](#baseline-file)
- [Component Benchmarks](#component-benchmarks)
- [CI Gate](#ci-gate)
- [Performance Targets](#performance-targets)

---

## Overview

perl-lsp maintains two complementary benchmark layers:

1. **Component benchmarks** (`cargo bench`) — criterion-based micro-benchmarks for parser, incremental parsing, completion, and navigation subsystems. These measure isolated algorithmic performance.

2. **Real-project latency suite** (`cargo test --test real_project_latency`) — integration-level benchmarks that measure user-felt latency on representative Perl project fixtures. These capture cold-start cost, workspace indexing, and realistic request patterns.

The parser corpus (`test_corpus/`, `cpan-corpus-baseline.json`) measures syntax correctness, not LSP response times. The real-project latency suite fills the gap between parser correctness and editor UX.

---

## Benchmark Types

| Suite | Command | Measures | When to run |
|-------|---------|----------|-------------|
| Component benchmarks | `cargo bench --workspace` | Parser/completion/navigation throughput | `ci:bench` label |
| Real-project latency | `cargo test -p perl-lsp-rs --test real_project_latency -- --include-ignored` | End-to-end LSP p50/p95/p99 | Nightly (`run_on = "nightly"`) |
| Parser corpus | `just cpan-corpus-check` | Parser clean rate on 9,372+ CPAN files | Post-merge |

---

## Real-Project Latency Suite

### Fixtures

Three sparse project skeletons are stored under `test_corpus/real_projects/`:

| Fixture | Source | Description | Entry file |
|---------|--------|-------------|------------|
| `mojolicious_skeleton/` | [mojolicious/mojo](https://github.com/mojolicious/mojo) | Modern async Perl web framework | `lib/Mojolicious.pm` |
| `dancer2_skeleton/` | [PerlDancer/Dancer2](https://github.com/PerlDancer/Dancer2) | Lightweight Dancer framework (Perl 5.10+) | `lib/Dancer2.pm` |
| `catalyst_skeleton/` | [perl-catalyst/catalyst-runtime](https://github.com/perl-catalyst/catalyst-runtime) | Full MVC framework, Moose-based | `lib/Catalyst.pm` |

Each skeleton is extracted from the upstream project and trimmed to core module structure (10–30 files). Files preserve:
- Real package declarations and `use`/`require` chains
- Actual method signatures and OO patterns (Moo, Moose, Mojo::Base)
- Representative symbol density

License headers are included in each file per upstream project terms.

### Metrics

Five operations are measured per project, each sampled `LATENCY_SAMPLES` (10) times:

| Metric | What is timed |
|--------|--------------|
| `cold_start_to_hover` | `start_lsp_server()` + `initialize` + `textDocument/didOpen` + first `textDocument/hover` |
| `first_completion` | `textDocument/completion` on a warmed server |
| `first_goto_definition` | `textDocument/definition` on a warmed server |
| `incremental_reparse` | `textDocument/didChange` (1-line edit) + next `textDocument/hover` round-trip |
| `workspace_symbol_query` | `workspace/symbol` with query `"new"` on a warmed server |

For each metric, p50, p95, and p99 are computed from the sample distribution.

### Running the Suite

```bash
# Run all 3 projects (writes combined baseline)
cargo test -p perl-lsp-rs --test real_project_latency full_suite -- --include-ignored --nocapture

# Run a single project
cargo test -p perl-lsp-rs --test real_project_latency mojolicious -- --include-ignored --nocapture

# Verify fixtures exist (non-ignored, runs in normal test suite)
cargo test -p perl-lsp-rs --test real_project_latency test_real_project_fixtures_exist

# Verify baseline schema
cargo test -p perl-lsp-rs --test real_project_latency test_real_project_latency_baseline_schema
```

The non-ignored sanity tests (`test_real_project_fixtures_exist`, `test_real_project_entry_files_are_valid_perl`, `test_real_project_latency_baseline_schema`) run in the normal test suite and do NOT start the LSP server.

### Baseline File

Results are written to `.ci/metrics/real_project_latency.json` after each nightly run.

Schema (version 1):

```json
{
  "schema_version": 1,
  "measured_at": "2026-04-14T00:00:00Z",
  "commit": "abc1234",
  "projects": {
    "mojolicious": {
      "file_count": 12,
      "metrics": {
        "cold_start_to_hover": { "p50_ms": 150, "p95_ms": 280, "p99_ms": 320, "samples": 10 },
        "first_completion":    { "p50_ms":  45, "p95_ms":  89, "p99_ms":  95, "samples": 10 },
        "first_goto_definition":{ "p50_ms": 35, "p95_ms":  72, "p99_ms":  80, "samples": 10 },
        "incremental_reparse": { "p50_ms":   2, "p95_ms":   5, "p99_ms":   8, "samples": 10 },
        "workspace_symbol_query":{ "p50_ms": 20, "p95_ms":  50, "p99_ms":  60, "samples": 10 }
      }
    },
    "dancer2":   { "file_count": 8,  "metrics": { ... } },
    "catalyst":  { "file_count": 10, "metrics": { ... } }
  },
  "tolerance_pct": 10
}
```

The initial baseline ships with `null` values pending the first nightly run. Once populated, regressions beyond `tolerance_pct` (10%) are flagged as informational alerts.

---

## Component Benchmarks

Located in `crates/*/benches/`:

| Crate | Benchmark | What it measures |
|-------|-----------|-----------------|
| `perl-parser` | `parser_benchmark` | Simple/complex script parse time |
| `perl-incremental-parsing` | `incremental_*` | Parse update and tree-rebuild time |
| `perl-lsp-completion` | `completion_*` | Completion provider throughput |
| `perl-lsp-navigation` | `navigation_*` | Go-to-definition navigation |

Run all benchmarks:

```bash
cargo bench --workspace --no-fail-fast
```

Run a specific benchmark:

```bash
cargo bench -p perl-parser -- parser_benchmark
```

---

## CI Gate

The real-project latency suite is registered as a nightly gate in `.ci/GATE_REGISTRY.toml`:

```toml
[[gate]]
id = "real-project-latency"
name = "Real Project Latency Benchmark"
type = "performance"
blocking = false
run_on = "nightly"
timeout_seconds = 1800
command = "cargo test -p perl-lsp-rs --test real_project_latency -- --include-ignored --nocapture --test-threads=1"
```

- **blocking**: `false` — informational only, does not gate merges
- **run_on**: `nightly` — not run on every PR
- **tolerance_pct**: `10` — 10% regression triggers an alert

---

## Performance Targets

From `docs/how-to/PERFORMANCE_TUNING.md`:

| Operation | P50 Target | P95 Target | P99 Target | Hard Limit |
|-----------|------------|------------|------------|------------|
| `textDocument/hover` | 5ms | 20ms | 50ms | 100ms |
| `textDocument/definition` | 10ms | 30ms | 75ms | 150ms |
| `textDocument/completion` | 20ms | 50ms | 100ms | 200ms |
| `workspace/symbol` | 20ms | 50ms | 150ms | 300ms |

Cold-start-to-hover is a composite metric; targets depend on workspace size. For the skeleton fixtures (10–30 files), cold start should be under 500ms p95.

---

See also: [PERFORMANCE_TUNING.md](PERFORMANCE_TUNING.md) | [PERFORMANCE_SLO.md](../reference/PERFORMANCE_SLO.md)
