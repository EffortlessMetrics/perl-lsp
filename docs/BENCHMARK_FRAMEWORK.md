# Benchmark Framework Documentation

## Overview

This document describes the comprehensive benchmarking framework for comparing C and Rust parser implementations. The framework provides statistical analysis, configurable performance gates, and detailed reporting capabilities.

## Architecture

### Components

1. **Rust Benchmark Runner** (`crates/tree-sitter-perl-rs/src/bin/benchmark_parsers.rs`)
   - Comprehensive benchmarking of Rust parser implementations
   - Statistical analysis with confidence intervals
   - JSON output compatible with comparison tools

2. **C Benchmark Harness** (`tree-sitter-perl/test/benchmark.js`)
   - Node.js-based benchmarking for C implementation
   - Standardized JSON output format

3. **Comparison Generator** (`scripts/generate_comparison.py`)
   - Statistical comparison between implementations
   - Configurable performance thresholds
   - Markdown and JSON report generation

4. **Integration Layer** (`xtask/src/tasks/bench.rs`)
   - Orchestrates the complete benchmark workflow
   - Integrates C and Rust benchmarking
   - Automated regression detection

5. **Memory Profiling System** (`xtask/src/tasks/compare.rs`)
   - **Dual-mode memory measurement** using procfs RSS and peak_alloc integration
   - **Statistical memory analysis** with min/max/avg/median calculations
   - **Memory estimation** for subprocess operations with size-based heuristics
   - **Comprehensive validation** with workload simulation testing

6. **Corpus Comparison Infrastructure** (v0.8.8+) ⭐ **NEW** (**Diataxis: Reference**)
   - **C vs V3 Scanner Comparison**: Direct benchmarking between legacy C scanner and V3 native parser
   - **Performance Optimization Validation**: Measure improvements from lexer optimizations (PR #102)
   - **Multi-implementation Analysis**: Compare performance characteristics across different parser versions
   - **Regression Detection**: Automated detection of performance degradation across parser implementations

## Usage

### Quick Start

```bash
# Run complete benchmark suite
cargo xtask bench

# Run with custom configuration
cargo xtask bench --save --output benchmark_results.json
```

### Individual Components

#### Rust Benchmarking

```bash
# Run Rust parser benchmarks
cargo run -p tree-sitter-perl --bin ts_benchmark_parsers --features pure-rust

# With custom configuration
echo '{"iterations": 200, "warmup_iterations": 20}' > benchmark_config.json
cargo run -p tree-sitter-perl --bin ts_benchmark_parsers --features pure-rust
```

#### Corpus Comparison Benchmarking ⭐ **NEW** (**Diataxis: How-to**)

```bash
# Run V3 vs C scanner comparison
cargo run -p perl-parser --bin corpus_comparison_benchmark

# Compare lexer optimization impact
cargo xtask bench --save --output lexer_before.json
# (Apply PR #102 optimizations)
cargo xtask bench --save --output lexer_after.json

# Generate optimization impact report
python3 scripts/generate_comparison.py \
  --baseline lexer_before.json \
  --optimized lexer_after.json \
  --report optimization_impact.md \
  --verbose

# Validate specific optimization categories
cargo run -p perl-lexer --example whitespace_benchmark
cargo run -p perl-lexer --example operator_disambiguation_benchmark  
cargo run -p perl-lexer --example string_interpolation_benchmark
```

#### C Benchmarking

```bash
# Run C implementation benchmarks
cd tree-sitter-perl
TEST_CODE="$(cat ../test/benchmark_simple.pl)" ITERATIONS=100 node test/benchmark.js
```

#### Comparison Generation

```bash
# Generate comparison report
python3 scripts/generate_comparison.py \
  --c-results c_benchmark.json \
  --rust-results rust_benchmark.json \
  --output comparison.json \
  --report comparison_report.md

# Create configuration template
python3 scripts/generate_comparison.py --create-config comparison_config.json

# With custom thresholds
python3 scripts/generate_comparison.py \
  --c-results c_benchmark.json \
  --rust-results rust_benchmark.json \
  --output comparison.json \
  --report comparison_report.md \
  --parse-threshold 3.0 \
  --memory-threshold 15.0 \
  --verbose
```

## Configuration

### Rust Benchmark Configuration

Create `benchmark_config.json` in the project root:

```json
{
  "iterations": 100,
  "warmup_iterations": 10,
  "test_files": [
    "test/benchmark_simple.pl",
    "test/corpus"
  ],
  "output_path": "benchmark_results.json",
  "detailed_stats": true,
  "memory_tracking": true
}
```

**Configuration Options:**

- `iterations`: Number of benchmark iterations per test
- `warmup_iterations`: Number of warmup runs before benchmarking
- `test_files`: List of test files or directories to benchmark
- `output_path`: Path for JSON results output
- `detailed_stats`: Include detailed statistical analysis
- `memory_tracking`: Enable dual-mode memory usage measurement (production-ready)

### Comparison Configuration

Create `comparison_config.json`:

```json
{
  "parse_time_regression_threshold": 5.0,
  "parse_time_improvement_threshold": 5.0,
  "memory_usage_regression_threshold": 20.0,
  "minimum_test_coverage": 90.0,
  "confidence_level": 0.95,
  "include_detailed_stats": true,
  "generate_charts": false,
  "output_formats": ["json", "markdown"]
}
```

**Configuration Options:**

- `parse_time_regression_threshold`: Threshold (%) for flagging parse time regressions
- `parse_time_improvement_threshold`: Threshold (%) for flagging parse time improvements
- `memory_usage_regression_threshold`: Threshold (%) for flagging memory usage regressions
- `minimum_test_coverage`: Minimum test coverage (%) required to pass gates
- `confidence_level`: Statistical confidence level for confidence intervals
- `include_detailed_stats`: Include detailed statistics in output
- `generate_charts`: Generate performance charts (requires matplotlib)
- `output_formats`: List of output formats to generate

## Performance Gates

The framework includes configurable performance gates that automatically detect regressions:

### Parse Time Gates
- **Threshold**: Configurable (default: 5% regression)
- **Status**: PASS/FAIL based on regression count
- **Action**: Fails CI if regressions detected

### Memory Usage Gates
- **Threshold**: Configurable (default: 20% regression)
- **Status**: WARNING/FAIL for memory increases
- **Action**: Warns on memory regressions
- **Dual-mode Tracking**: procfs RSS measurement with peak_alloc fallback
- **Statistical Analysis**: Memory usage patterns with confidence intervals
- **Subprocess Estimation**: Size-based memory estimation for external processes

### Test Coverage Gates
- **Threshold**: Configurable (default: 90% coverage)
- **Status**: PASS/WARNING based on test count
- **Action**: Warns if insufficient tests

### Statistical Confidence Gates
- **Threshold**: Based on sample size and confidence level
- **Status**: PASS/WARNING for statistical validity
- **Action**: Warns if sample size too small

## Output Formats

### Benchmark Results JSON

```json
{
  "metadata": {
    "generated_at": "1630000000",
    "parser_version": "0.8.9",
    "rust_version": "1.89",
    "total_tests": 10,
    "total_iterations": 1000,
    "configuration": { ... }
  },
  "tests": {
    "simple_script": {
      "name": "simple_script",
      "file_size": 1226,
      "iterations": 100,
      "durations_ns": [125000, 130000, ...],
      "mean_duration_ns": 127500.0,
      "std_dev_ns": 2500.0,
      "min_duration_ns": 120000,
      "max_duration_ns": 135000,
      "median_duration_ns": 127000.0,
      "success_rate": 1.0,
      "tokens_per_second": 15000.0,
      "avg_memory": 0.85,
      "min_memory": 0.75,
      "max_memory": 0.95,
      "median_memory": 0.83,
      "memory_tracking_mode": "dual_mode_rss_peak_alloc"
    }
  },
  "summary": {
    "overall_mean_ns": 127500.0,
    "overall_std_dev_ns": 2500.0,
    "fastest_test": "simple_script",
    "slowest_test": "complex_script",
    "total_runtime_seconds": 12.5,
    "success_rate": 1.0,
    "performance_categories": {
      "fast_parsing": ["simple_script"],
      "moderate_parsing": ["medium_script"],
      "slow_parsing": ["complex_script"]
    }
  }
}
```

### Comparison Report Markdown

The generated markdown report includes:

- **Executive Summary**: Test counts, regression summary
- **Overall Performance**: Statistical analysis of performance differences
- **Detailed Test Results**: Per-test comparison table
- **Performance Gates Status**: Pass/fail status for all gates
- **Statistical Analysis**: Confidence intervals, significance tests

## Test Files

### Benchmark Test Cases

The framework uses a hierarchical set of test cases:

1. **Simple Test** (`test/benchmark_simple.pl`)
   - Basic Perl constructs
   - ~75 lines, ~1.2KB
   - Baseline performance test

2. **Corpus Tests** (`test/corpus/`)
   - Real-world Perl files
   - Various sizes and complexity
   - Edge case coverage

### Test Categories

Tests are automatically categorized by:

- **File Size**: small (<1KB), medium (1-10KB), large (>10KB)
- **Parse Time**: fast (<1ms), moderate (1-10ms), slow (>10ms)
- **Success Rate**: successful parsing vs. error recovery

## Performance Targets

### Rust Implementation Targets

- **Simple files**: <100μs average parse time
- **Medium files**: <1ms average parse time  
- **Large files**: <10ms average parse time
- **Success rate**: >99% for valid Perl code
- **Memory usage**: <1MB peak memory for typical files (measured with dual-mode tracking)

### Lexer Optimization Targets (v0.8.8+) ⭐ **NEW** (**Diataxis: Reference**)

**Achieved Performance Improvements (PR #102):**
- **Whitespace Processing**: 18.779% improvement through batch processing
- **Slash Disambiguation**: 14.768% improvement via optimized byte operations
- **String Interpolation**: 22.156% improvement using fast-path ASCII parsing
- **Comment Scanning**: Significant improvement through direct position advancement
- **Number Parsing**: Enhanced performance via unrolled loops and bounds checking

**Optimization Categories:**
- **ASCII-Heavy Code**: 15-25% performance improvement expected
- **Whitespace-Dense Files**: 18-20% faster processing
- **Operator-Heavy Expressions**: 14-16% improvement in disambiguation
- **Template/Interpolation Code**: 20-22% faster variable extraction

### Regression Detection

- **Parse Time**: >5% slowdown triggers regression warning
- **Memory Usage**: >20% increase triggers memory warning (dual-mode measurement)
- **Success Rate**: Any decrease in parsing success rate
- **Memory Accuracy**: ±10% precision with fallback mechanisms
- **Statistical Significance**: Confidence intervals for memory measurements

## Integration with CI/CD

### Automated Testing

```yaml
# Example GitHub Actions integration
- name: Run Benchmarks
  run: cargo xtask bench --save

- name: Check Performance Gates
  run: |
    if grep -q "❌ FAIL" benchmark_report.md; then
      echo "Performance gates failed"
      exit 1
    fi
```

### Performance Monitoring

The framework can be integrated with performance monitoring systems:

1. **JSON Output**: Parse results for metric ingestion
2. **Exit Codes**: Non-zero exit for gate failures
3. **Structured Logging**: Machine-readable performance data

## Troubleshooting

### Common Issues

#### "No test files found"
- Ensure test files exist in specified paths
- Check file permissions and extensions (.pl, .pm, .t)
- Verify working directory is project root

#### "Failed to parse C benchmark output"
- Ensure Node.js and tree-sitter-perl C library are installed
- Check that benchmark.js has execute permissions
- Verify TEST_CODE environment variable is set

#### "Memory measurement returned zero"
- Memory tracking uses dual-mode measurement (procfs RSS + peak_alloc)
- Small operations may show minimal memory usage (normal behavior)
- Check that `/proc` filesystem is available (Linux systems)
- Fallback to peak_alloc measurement if procfs unavailable

#### "Memory profiling validation failed"
- Run `cargo run --bin xtask -- validate-memory-profiling` for diagnosis
- Check system permissions for /proc filesystem access
- Verify peak_alloc crate is properly initialized

#### Performance Gate Failures
- Review regression threshold configuration
- Check if performance changes are expected
- Analyze detailed test results for specific failing tests

### Debug Mode

Enable verbose logging for troubleshooting:

```bash
# Verbose Rust benchmarks
RUST_LOG=debug cargo run -p tree-sitter-perl --bin ts_benchmark_parsers

# Verbose comparison
python3 scripts/generate_comparison.py --verbose [other args]
```

### Performance Analysis

For detailed performance analysis:

1. **Increase Iterations**: Higher iteration counts for statistical significance
2. **Enable Memory Tracking**: Monitor memory usage patterns with dual-mode measurement
3. **Detailed Stats**: Enable comprehensive statistical analysis
4. **Profiling**: Use `cargo flamegraph` or similar tools

### Memory Profiling System (v0.8.9+)

The framework includes advanced memory profiling capabilities:

#### Dual-Mode Memory Measurement
- **procfs RSS Tracking**: Real-time process memory usage from `/proc/[pid]/statm`
- **peak_alloc Integration**: Local memory allocation tracking with fallback support
- **Automatic Fallback**: Uses peak_alloc when procfs unavailable or returns zero
- **Statistical Analysis**: Comprehensive min/max/avg/median calculations

#### Memory Profiling Commands
```bash
# Validate memory profiling functionality
cargo run --bin xtask -- validate-memory-profiling

# Run comparison with memory tracking enabled
cargo xtask compare --report  # Includes memory metrics in output
```

#### Memory Measurement Process
1. **Pre-operation Baseline**: Measures RSS memory before operation
2. **Peak Allocator Reset**: Resets local allocation tracking
3. **Operation Execution**: Runs the target parsing operation
4. **Post-operation Measurement**: Captures final RSS memory state
5. **Intelligent Selection**: Uses delta RSS or falls back to peak_alloc
6. **Statistical Processing**: Calculates comprehensive memory statistics

#### Memory Estimation for Subprocesses
- **File-size Based Heuristics**: Estimates memory usage for external processes
- **Conservative Scaling**: Uses ~8x file size plus 0.5MB base overhead
- **Minimum Guarantees**: Ensures at least 0.1MB reported for tiny files
- **Fallback Values**: Returns 0.5MB default for inaccessible files

## Contributing

### Adding New Tests

1. Add test files to `test/corpus/` directory
2. Update benchmark configuration to include new tests
3. Verify tests work with both C and Rust implementations
4. Update performance targets if necessary

### Extending Analysis

1. Modify comparison script for new metrics
2. Add statistical tests for significance analysis
3. Implement new output formats (CSV, XML, etc.)
4. Add visualization capabilities

### Performance Optimization

1. Profile benchmark runners for overhead
2. Optimize statistical calculations
3. Add caching for repeated benchmarks
4. Implement parallel test execution

## References

- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Statistical Methods for Performance Analysis](https://en.wikipedia.org/wiki/Performance_analysis)
- [Criterion.rs Documentation](https://docs.rs/criterion/)
- [Tree-sitter Documentation](https://tree-sitter.github.io/tree-sitter/)