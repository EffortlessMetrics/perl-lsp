# Benchmark Infrastructure

This directory contains the benchmark infrastructure for tracking perl-lsp performance over time.

> **Note**: For comprehensive documentation on running benchmarks and interpreting results,
> see [BENCHMARK_FRAMEWORK.md](./BENCHMARK_FRAMEWORK.md).

## Quick Start

```bash
# Run all benchmarks
just bench

# Quick smoke benchmarks (fast, ~30s)
just bench-quick

# Compare against baseline
just bench-compare

# Generate performance alerts
just bench-alert

# Check for critical regressions (CI gate)
just bench-alert-check

# Save current run as a new baseline
just bench-baseline
```

## Automated Performance Regression Alerts

The perl-lsp project now includes automated performance regression detection with configurable thresholds and PR comment integration. See [docs/PERFORMANCE_MONITORING.md](../docs/PERFORMANCE_MONITORING.md) for comprehensive documentation.

### Alert Levels

- **WARNING** (10% slower): Yellow flag, investigate
- **REGRESSION** (20% slower): Red flag, fix before merge
- **CRITICAL** (50% slower): Blocks merge (optional)
- **IMPROVED** (10% faster): Celebrated in PR comments

### Configuration

Thresholds are configured in `.ci/benchmark-thresholds.yaml`:

```yaml
defaults:
  warn_threshold_pct: 10
  regression_threshold_pct: 20
  critical_threshold_pct: 50

categories:
  lsp:  # Stricter for LSP operations
    warn_threshold_pct: 5
    regression_threshold_pct: 15
    critical_threshold_pct: 30
```

## Directory Structure

```
benchmarks/
├── README.md              # This file
├── baselines/             # Committed baseline results (tagged by version)
│   └── v0.9.0.json        # Baseline from release
├── results/               # Current run results (gitignored except releases)
│   └── latest.json        # Most recent benchmark run
└── scripts/
    ├── run-benchmarks.sh  # Main benchmark runner
    ├── compare.sh         # Compare current vs baseline
    └── format-results.py  # Format results for display
```

## Benchmark Categories

### 1. Parser Performance

Measures parsing speed across different file sizes and complexity levels.

| Benchmark | Description | Target |
|-----------|-------------|--------|
| `parse_simple_script` | Basic variable declarations | < 50us |
| `parse_complex_script` | Full module with OO code | < 500us |
| `large_file_parsing` | 5000-line generated file | < 50ms |

### 2. Lexer Performance

Measures tokenization speed for various input patterns.

| Benchmark | Description | Target |
|-----------|-------------|--------|
| `simple_tokens` | Basic tokenization | < 10us |
| `slash_disambiguation` | Context-aware / handling | < 50us |
| `large_file` | 1000-line file tokenization | < 10ms |

### 3. LSP Response Times

Measures LSP operation latency (critical for editor responsiveness).

| Benchmark | Description | Target |
|-----------|-------------|--------|
| `document_insertions` | Text edit operations | < 1ms |
| `position_conversions` | LSP position mapping | < 100us |
| `incremental_edits` | Multiple small edits | < 5ms |

### 4. Workspace Indexing

Measures indexing performance (affects startup time and find-references).

| Benchmark | Description | Target |
|-----------|-------------|--------|
| `initial_index_small` | 5 file workspace | < 100ms |
| `initial_index_medium` | 10 file workspace | < 200ms |
| `incremental_update` | Single file change | < 10ms |
| `symbol_lookup` | Definition lookup | < 1us |

## Running Benchmarks

### Full Benchmark Suite

```bash
# Via just (recommended)
just bench

# Via cargo directly
cargo bench --workspace --locked
```

### Quick Smoke Test

For CI or quick validation:

```bash
just bench-quick
```

This runs a subset of benchmarks with reduced iterations.

### Specific Benchmark

```bash
# Run parser benchmarks only
cargo bench -p perl-parser --bench parser_benchmark

# Run LSP rope benchmarks only
cargo bench -p perl-lsp --bench rope_performance_benchmark

# Run workspace index benchmarks only
cargo bench -p perl-workspace-index --bench workspace_index_benchmark
```

## Comparing Results

### Against Baseline

```bash
# Compare current vs committed baseline
just bench-compare

# Compare two specific files
./benchmarks/scripts/compare.sh benchmarks/baselines/v0.9.0.json benchmarks/results/latest.json
```

### Output Format

```
Benchmark Comparison: v0.9.0 vs current
========================================

Parser Performance:
  parse_simple_script:    45.2us -> 43.1us  (-4.6%)  [OK]
  parse_complex_script:   412us  -> 398us   (-3.4%)  [OK]
  large_file_parsing:     42.3ms -> 58.2ms  (+37.6%) [REGRESSION]

LSP Response Times:
  document_insertions:    0.8ms  -> 0.9ms   (+12.5%) [OK]
  position_conversions:   85us   -> 82us    (-3.5%)  [OK]

Summary: 1 regression(s) detected
```

## Creating Baselines

### For Releases

When preparing a release, save the current benchmark results as a baseline:

```bash
# Run benchmarks and save as baseline
just bench-baseline

# Or manually:
./benchmarks/scripts/run-benchmarks.sh --output benchmarks/baselines/v0.9.0.json
```

### Baseline Format

Baselines are JSON files with the following structure:

```json
{
  "version": "0.9.0",
  "timestamp": "2026-01-24T12:00:00Z",
  "git_sha": "abc123",
  "environment": {
    "os": "Linux",
    "cpu": "Intel i7-9700K",
    "rust_version": "1.92.0"
  },
  "results": {
    "parser": {
      "parse_simple_script": {
        "mean_ns": 45200,
        "stddev_ns": 1200,
        "iterations": 1000
      }
    }
  }
}
```

## CI Integration

### Nightly Benchmarks

Benchmarks run nightly (non-blocking) via `.github/workflows/nightly.yml`:

```yaml
- name: Run benchmarks
  run: just bench

- name: Compare against baseline
  run: just bench-compare --fail-on-regression
```

### PR Benchmarks (Label-Gated)

PRs labeled with `ci:bench` trigger benchmark runs:

```yaml
if: contains(github.event.pull_request.labels.*.name, 'ci:bench')
```

### Performance Regression Detection

A regression is detected when:
- Any benchmark is > 20% slower than baseline
- Memory usage increases > 10%
- New benchmark failures (previously passing)

## Receipt Format

Benchmark results are recorded in receipts for auditing:

```
========================================
BENCHMARK RECEIPT - 2026-01-24
========================================

Run ID: bench-20260124-143022
Git SHA: abc123def456
Baseline: v0.9.0

PARSER BENCHMARKS:
  parse_simple_script:   43.1us (baseline: 45.2us, -4.6%)
  parse_complex_script: 398.0us (baseline: 412us, -3.4%)

SUMMARY:
  Total benchmarks: 24
  Regressions: 0
  Improvements: 2

STATUS: PASS
========================================
```

## Troubleshooting

### Noisy Results

If results vary significantly between runs:

1. Close other applications
2. Run with more iterations: `--measurement-time 10`
3. Use `cargo bench -- --noplot` to skip HTML report generation

### Missing Benchmarks

If benchmarks fail to compile:

```bash
# Check benchmark crates
cargo build --workspace --benches --locked

# Check specific benchmark
cargo bench -p perl-parser --bench parser_benchmark -- --list
```

### Large Variance

High variance (> 10% stddev) indicates:
- System load affecting results
- Input-dependent performance (edge cases)
- Need for more warm-up iterations

## Adding New Benchmarks

1. Add benchmark function to the appropriate `crates/*/benches/*.rs` file
2. Register in `criterion_group!` macro
3. Add target performance to this README
4. Update baseline after review

Example:

```rust
fn bench_new_feature(c: &mut Criterion) {
    c.bench_function("new_feature", |b| {
        b.iter(|| {
            // benchmark code
        });
    });
}

criterion_group!(benches, ..., bench_new_feature);
```

## Performance Targets

These targets are based on editor responsiveness requirements:

| Category | Target | Rationale |
|----------|--------|-----------|
| Keystroke response | < 50ms | Feels instant |
| Completion popup | < 100ms | Tolerable delay |
| Go-to-definition | < 200ms | Quick navigation |
| Initial index | < 2s | Acceptable startup |
| Incremental update | < 50ms | Real-time feel |

## See Also

- [BENCHMARK_FRAMEWORK.md](./BENCHMARK_FRAMEWORK.md) - Comprehensive benchmark documentation
- [results/README.md](./results/README.md) - Published benchmark results
- [docs/CURRENT_STATUS.md](../docs/CURRENT_STATUS.md) - Project metrics
- [docs/ROADMAP.md](../docs/ROADMAP.md) - Performance milestones
- `.github/workflows/benchmark.yml` - CI benchmark workflow
