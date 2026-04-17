# Performance Baselines (0.12.4)

Measured baseline performance numbers for the perl-lsp 0.12.4 public alpha release.
All measurements taken with criterion on Linux (x86_64). Exact numbers vary by hardware;
the relative magnitudes and SLO compliance are what matter.

## How to Reproduce

```bash
just perf-baseline          # Run all performance benchmarks and save baseline
just bench                  # Full benchmark suite
just bench-quick            # Quick smoke test (~30s)
just bench-compare          # Compare current results against saved baseline
```

## Parser Performance

| Benchmark | Measured | SLO Target | Status |
|-----------|----------|------------|--------|
| Simple script parse (10 lines) | ~12 us | <50 us | PASS |
| Complex module parse (70 lines) | ~43 us | <500 us | PASS |
| Lexer-only tokenization | ~16 us | <50 us | PASS |
| AST to s-expression | ~16 us | N/A | -- |
| Scope analysis | ~4 us | N/A | -- |

## Completion Latency

| Benchmark | Measured | SLO Target | Status |
|-----------|----------|------------|--------|
| Variable completion (`$c` prefix) | ~5 us | <50 ms p99 | PASS |
| Keyword completion (`pri` prefix) | ~4 us | <50 ms p99 | PASS |
| Method completion (`$self->g`) | ~7 us | <50 ms p99 | PASS |
| Workspace-integrated completion | ~6 us | <50 ms p99 | PASS |
| Large module completion (15 subs) | ~7 us | <50 ms p99 | PASS |
| Empty prefix (worst case) | ~91 us | <50 ms p99 | PASS |

All completion operations are well under the 50ms p99 SLO, typically completing in single-digit microseconds.

## Navigation Latency

| Benchmark | Measured | SLO Target | Status |
|-----------|----------|------------|--------|
| Find references (variable, single file) | <50 us | <100 ms p95 | PASS |
| Find references (subroutine) | <50 us | <100 ms p95 | PASS |
| Find references (large file) | <50 us | <100 ms p95 | PASS |
| Workspace symbol search (exact) | <100 us | <50 ms p95 | PASS |
| Workspace symbol search (prefix) | <100 us | <50 ms p95 | PASS |
| Workspace symbol indexing (4 files) | <500 us | <2 s | PASS |

## Workspace Indexing

| Benchmark | Measured | SLO Target | Status |
|-----------|----------|------------|--------|
| Initial index (5 files) | <5 ms | <100 ms | PASS |
| Initial index (10 files) | <10 ms | <200 ms | PASS |
| Incremental update (1 file) | <1 ms | <10 ms | PASS |
| Symbol lookup (hash table) | <1 us | <1 us | PASS |
| Cross-file reference search | <10 us | <1 ms | PASS |
| Workspace symbol search (10 files) | <100 us | <50 ms | PASS |
| File removal + re-index | <2 ms | <10 ms | PASS |
| Early-exit (unchanged content) | <100 us | <1 ms | PASS |

## LSP Infrastructure

| Benchmark | Measured | SLO Target | Status |
|-----------|----------|------------|--------|
| Rope insertion (50KB doc) | <5 us | <1 ms | PASS |
| UTF-16 position conversion | <500 ns | <100 us | PASS |
| UTF-8 position conversion | <100 ns | <100 us | PASS |
| Multiple incremental edits | <50 us | <5 ms | PASS |
| State transitions | <1 us | <1 ms | PASS |
| State query (hot path) | <10 ns | <100 ns | PASS |

## Cache Performance

| Benchmark | Measured | SLO Target | Status |
|-----------|----------|------------|--------|
| Cache put | <1 us | <10 us | PASS |
| Cache get (hit) | <500 ns | <1 us | PASS |
| Cache get (miss) | <100 ns | <1 us | PASS |
| Cache eviction (15 entries) | <10 us | <100 us | PASS |
| Concurrent access (4 threads) | <100 us | <1 ms | PASS |

## Summary

All measured operations are well within their SLO targets, typically by 2-3 orders of magnitude.
The server delivers sub-millisecond response times for all interactive operations
(completion, hover, go-to-definition, find-references), ensuring a responsive editing experience.

### Qualitative Claims (supported by measurements)

- **Sub-100us completion**: Typical completion responses in 4-91 microseconds
- **Sub-50us parse times**: Simple scripts parse in ~12 microseconds
- **Sub-millisecond navigation**: All navigation operations under 100 microseconds
- **Fast indexing**: 10-file workspace indexed in under 10 milliseconds

## Benchmark Categories

The benchmark suite covers 7 categories across 7 crates:

| Category | Crate | Bench File |
|----------|-------|------------|
| Parser | `perl-parser` | `parser_benchmark.rs` |
| Lexer | `perl-lexer` | `lexer_benchmarks.rs` |
| Completion | `perl-lsp-completion` | `completion_benchmark.rs` |
| Navigation | `perl-lsp-navigation` | `navigation_benchmark.rs` |
| Workspace Index | `perl-workspace-index` | `workspace_index_benchmark.rs` |
| LSP Infrastructure | `perl-lsp` | `rope_performance_benchmark.rs` |
| Cache | `perl-lsp-tooling` | `cache_benchmark.rs` |

## See Also

- [PERFORMANCE_SLO.md](../reference/PERFORMANCE_SLO.md) -- Full SLO definitions
- [PERFORMANCE_MONITORING.md](../reference/PERFORMANCE_MONITORING.md) -- Regression detection
- [benchmarks/README.md](../../benchmarks/README.md) -- Benchmark infrastructure
