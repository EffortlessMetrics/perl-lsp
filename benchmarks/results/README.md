# Benchmark Results

This directory contains published benchmark results for the Perl LSP project.

## File Naming Convention

```
<date>-<machine>-<component>.json
```

Examples:
- `2026-01-22-ryzen9-9950x3d-parser.json` - Parser benchmarks on AMD Ryzen 9 9950X3D
- `2026-01-22-ryzen9-9950x3d-incremental.json` - Incremental parsing benchmarks
- `2026-01-22-ryzen9-9950x3d-lsp.json` - LSP server performance benchmarks

## Published Results

### 2026-01-22: v0.9.x (Production-Ready) Baseline (AMD Ryzen 9 9950X3D)

**Machine Configuration:**
- CPU: AMD Ryzen 9 9950X3D 16-Core Processor (32 threads)
- Memory: 196 GiB
- OS: Linux 6.6.87.2-microsoft-standard-WSL2
- Rust: rustc 1.91.1 (ed61e7d7e 2025-11-07)

**Results:**
- **Parser Benchmarks**: `/benchmarks/results/2026-01-22-ryzen9-9950x3d-parser.json`
  - `parse_simple_script`: 21.26μs mean (meets 4-19x speed target)
  - Performance: ~15-20x faster than legacy implementations

**Status:**
- Parser performance baseline established
- Incremental parsing benchmarks pending
- LSP server performance tests pending
- Workspace indexing data available in `/target/criterion/`

## Performance Targets

### Parser Performance
- **Target**: 1-150μs parsing time (4-19x faster than legacy)
- **v0.9.x (Production-Ready) Baseline**: 21.26μs mean (ACHIEVED)

### Incremental Parsing
- **Target**: <1ms updates with 70-99% node reuse
- **v0.9.x (Production-Ready) Baseline**: Pending

### LSP Server Performance
- **Behavioral Tests**: <1s execution (0.31s revolutionary target)
- **User Story Tests**: <1s execution (0.32s revolutionary target)
- **Workspace Navigation**: <1s individual tests (0.26s revolutionary target)
- **v0.9.x (Production-Ready) Baseline**: Pending

## Running Benchmarks

See `/benchmarks/BENCHMARK_FRAMEWORK.md` for comprehensive documentation.

**Quick reference:**
```bash
# Parser benchmarks
cargo bench -p perl-parser --bench parser_benchmark

# Incremental parsing
cargo bench -p perl-parser --bench incremental_benchmark --features incremental

# LSP performance (via release tests)
RUST_TEST_THREADS=2 cargo test --release -p perl-lsp
```

## Interpreting Results

All times in the JSON files are reported in nanoseconds unless otherwise specified.

**Conversion reference:**
- 1,000 nanoseconds = 1 microsecond (μs)
- 1,000 microseconds = 1 millisecond (ms)
- 1,000 milliseconds = 1 second (s)

**Confidence intervals:**
- All measurements include 95% confidence intervals
- Lower/upper bounds represent the range of likely true values

**Outliers:**
- Measurements significantly different from the mean
- Categorized as mild or severe
- High outliers may indicate GC pauses or system interference

## Historical Baselines

Future benchmark runs will be added to this directory for regression tracking.

**Regression detection:**
```bash
# Compare against v0.9.x (Production-Ready) baseline
cargo bench -p perl-parser --bench parser_benchmark -- --baseline v0.9.x (Production-Ready)-baseline
```

---

Last Updated: 2026-01-22
