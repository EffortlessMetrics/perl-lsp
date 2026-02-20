# Perl LSP Benchmark Framework

This document describes the benchmark framework for the Perl LSP project, including how to run benchmarks, interpret results, and maintain baseline performance data.

## Overview

The Perl LSP benchmark suite measures parsing and LSP server performance to:
- Establish performance baselines for regression detection
- Validate 4-19x speed improvements over legacy implementations
- Track incremental parsing efficiency and node reuse rates
- Measure LSP server revolutionary performance targets

## Machine Configuration

Baseline benchmarks were collected on the following system:

```
Machine: AMD Ryzen 9 9950X3D 16-Core Processor
CPU Cores: 16 physical cores, 32 threads (2 threads per core)
Memory: 196 GiB
OS: Linux 6.6.87.2-microsoft-standard-WSL2 (WSL2)
Rust: rustc 1.91.1 (ed61e7d7e 2025-11-07)
Cargo: cargo 1.91.1 (ea2d97820 2025-10-10)
```

## Benchmark Suites

### 1. Parser Benchmarks (`perl-parser`)

Location: `/crates/perl-parser/benches/parser_benchmark.rs`

**Benchmarks:**
- `parse_simple_script`: Basic Perl script with variables, conditionals, and functions
- `parse_complex_script`: OO Perl with packages, regex, and complex control flow
- `ast_to_sexp`: AST serialization performance
- `lexer_only`: Isolated lexer performance

**Run command:**
```bash
cargo bench -p perl-parser --bench parser_benchmark
```

**Save baseline:**
```bash
cargo bench -p perl-parser --bench parser_benchmark -- --save-baseline v0.9.x (Production-Ready)-baseline
```

**Compare against baseline:**
```bash
cargo bench -p perl-parser --bench parser_benchmark -- --baseline v0.9.x (Production-Ready)-baseline
```

### 2. Incremental Parsing Benchmarks (`perl-parser`)

Location: `/crates/perl-parser/benches/incremental_benchmark.rs`

**Benchmarks:**
- `incremental small edit`: Single edit with incremental reparse
- `full reparse`: Complete reparse for comparison
- `incremental multiple edits`: Batch edit performance
- `incremental_document single edit`: IncrementalDocument API single edit
- `incremental_document multiple edits`: IncrementalDocument API batch edits

**Run command:**
```bash
cargo bench -p perl-parser --bench incremental_benchmark --features incremental
```

**Performance Targets:**
- Incremental updates: <1ms with 70-99% node reuse efficiency
- Full reparse baseline: Establish for comparison ratio

### 3. LSP Server Benchmarks

**Test-based benchmarks:**
```bash
# LSP behavioral tests (revolutionary 0.31s target)
RUST_TEST_THREADS=2 cargo test --release -p perl-lsp --test behavioral

# LSP user story tests (revolutionary 0.32s target)
RUST_TEST_THREADS=2 cargo test --release -p perl-lsp --test user_stories

# Workspace navigation tests (230x improvement, 0.26s target)
RUST_TEST_THREADS=2 cargo test --release -p perl-lsp --test workspace_navigation
```

**Comprehensive LSP E2E performance:**
```bash
cargo test -p perl-parser --test lsp_comprehensive_e2e_test --release
```

### 4. Additional Benchmarks

**Lexer benchmarks:**
```bash
cargo bench -p perl-lexer
```

**Workspace indexing:**
```bash
cargo bench -p perl-workspace-index
```

**DAP protocol:**
```bash
cargo bench -p perl-dap
```

**Rope performance:**
```bash
cargo bench -p perl-lsp --bench rope_performance_benchmark
```

## Performance Baseline Targets

### Parser Performance
- **Target**: 1-150μs parsing time (4-19x faster than legacy)
- **Baseline**: Established in `/benchmarks/results/`

### LSP Server Performance
- **Behavioral Tests**: <1s execution (revolutionary 0.31s target, 5000x improvement)
- **User Story Tests**: <1s execution (revolutionary 0.32s target, 4700x improvement)
- **Workspace Navigation**: <1s individual tests (230x improvement, 0.26s target)

### Incremental Parsing
- **Updates**: <1ms with 70-99% node reuse efficiency
- **Statistical Validation**: Node reuse percentages tracked in metrics

### Unicode Processing
- **Performance**: >10k chars/sec Unicode classification
- **Instrumentation**: Atomic counter tracking for emoji/complex Unicode

### Adaptive Threading
- **LSP Harness Timeouts**: 200-500ms based on thread count
- **CI Optimization**: `RUST_TEST_THREADS=2` for consistent results

## Interpreting Results

### Criterion Output

Criterion benchmarks report:
- **Mean time**: Average execution time across samples
- **Std dev**: Standard deviation of measurements
- **Outliers**: Measurements significantly different from the mean

Example output:
```
parse_simple_script     time:   [21.979 µs 23.820 µs 25.843 µs]
                        change: [-5.123% -2.456% +0.789%] (p = 0.05)
```

**Reading the results:**
- First value: Lower bound of 95% confidence interval
- Second value: Mean (best estimate)
- Third value: Upper bound of 95% confidence interval
- Change percentage: Difference from baseline (if comparing)

### Regression Detection

Performance regressions are detected by comparing against baselines:
1. Run benchmarks with `--save-baseline <name>` to establish baseline
2. Future runs use `--baseline <name>` to compare
3. Significant changes (>5%) trigger investigation

## Result Storage

Benchmark results are stored in `/benchmarks/results/` with the following naming convention:

```
<date>-<machine>-<component>.json
```

Example:
```
2026-01-22-ryzen9-9950x3d-parser.json
2026-01-22-ryzen9-9950x3d-incremental.json
```

## Running Benchmarks

### Quick Benchmark Run

For a quick performance check:
```bash
cargo bench -p perl-parser
```

### Comprehensive Benchmark Suite

For full performance validation:
```bash
# Parser benchmarks
cargo bench -p perl-parser --bench parser_benchmark

# Incremental parsing (requires feature flag)
cargo bench -p perl-parser --bench incremental_benchmark --features incremental

# LSP performance (via release tests)
RUST_TEST_THREADS=2 cargo test --release -p perl-lsp

# Lexer performance
cargo bench -p perl-lexer
```

### CI/Automated Benchmarks

For consistent CI results with threading constraints:
```bash
RUST_TEST_THREADS=2 cargo bench -p perl-parser
```

## Cross-Language Comparison

The benchmark framework supports comparison with legacy implementations:

**Legacy parsers for comparison:**
- **v1 (C-based)**: Tree-sitter C parser (benchmarking only)
- **v2 (Pest)**: Pest-based parser (legacy, kept for comparison)
- **v3 (Native)**: Current recursive descent parser

**Comparison tools:**
```bash
# Legacy comparison (if available)
cargo xtask bench --save --output baseline_results.json
```

## Maintenance

### Updating Baselines

When intentional performance improvements are made:
1. Run benchmarks with new baseline name
2. Document the change in git commit
3. Update this file with new baseline reference

### Adding New Benchmarks

1. Add benchmark function to the appropriate `crates/*/benches/*.rs` file
2. Follow Criterion harness structure
3. Use `black_box()` to prevent compiler optimization
4. Document expected performance targets
5. Update this file with new benchmark details

## Troubleshooting

### Benchmarks Fail to Compile

- Check feature flags: Some benchmarks require `--features incremental`
- Verify Criterion dependency in `Cargo.toml`

### Inconsistent Results

- Ensure system is idle during benchmarks
- Use release builds: `cargo bench` automatically uses release mode
- For CI: Use `RUST_TEST_THREADS=2` to limit parallelism
- Disable dynamic CPU scaling if available

### Missing Baseline

If comparing without saved baseline:
```bash
# Save current state as baseline
cargo bench -p perl-parser -- --save-baseline current
```

## References

- [Criterion.rs User Guide](https://bheisler.github.io/criterion.rs/book/)
- [Perl LSP Performance Requirements](../docs/ROADMAP.md)
- [Parser Performance Targets](../crates/perl-parser/README.md)
- [LSP Revolutionary Performance](../docs/LSP_IMPLEMENTATION_GUIDE.md)

## History

- **2026-01-22**: Initial benchmark framework documentation
- **v0.9.x (Production-Ready) baseline**: Established on AMD Ryzen 9 9950X3D system

---

Last Updated: 2026-01-22
