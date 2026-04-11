# Incremental Parsing Benchmarks

This directory contains benchmarks for verifying performance improvements from issue #3527:
- Segment-based token cache implementation
- Two-sided checkpoint window implementation
- Enhanced metrics tracking

## Quick Start

```bash
# Run all benchmarks
cargo bench -p perl-incremental-parsing

# Run specific benchmark group
cargo bench -p perl-incremental-parsing single_char_insertion

# Run with verbose output
cargo bench -p perl-incremental-parsing -- --verbose
```

## Benchmark Files

- [`incremental_parsing_benchmarks.rs`](incremental_parsing_benchmarks.rs) - Main benchmark suite
- [`README.md`](README.md) - This file

## Benchmark Groups

### Core Editing Scenarios

#### `single_char_insertion`
Benchmarks single character insertions at various positions (beginning, middle, end).

**Purpose**: Verify that incremental parsing provides 10-20% improvement over full re-lex for typical typing scenarios.

**Key Metrics**:
- `duration_ns` - Time taken for incremental parse
- `tokens_reused` - Number of tokens reused from cache
- `tokens_relexed` - Number of tokens re-lexed

#### `single_char_deletion`
Benchmarks single character deletions at various positions.

**Purpose**: Verify that deletions benefit from cache reuse similar to insertions.

**Key Metrics**:
- `duration_ns` - Time taken for incremental parse
- `tokens_reused` - Number of tokens reused from cache
- `tokens_relexed` - Number of tokens re-lexed

#### `large_paste`
Benchmarks large paste operations (100, 500, 1000, 5000 bytes).

**Purpose**: Verify 30-50% improvement for pasting repeated content.

**Key Metrics**:
- `duration_ns` - Time taken for incremental parse
- `throughput` - Bytes processed per second
- `tokens_reused` - Number of tokens reused from cache
- `bytes_relexed` - Total bytes re-lexed

#### `undo_redo`
Benchmarks edit-undo-redo pattern.

**Purpose**: Verify 70-90% improvement when content is already cached.

**Key Metrics**:
- `duration_ns` - Time taken for each phase
- `cache_hits` - Number of cache hits
- `cache_misses` - Number of cache misses

### Advanced Scenarios

#### `repeated_edits`
Benchmarks multiple sequential edits (10, 50, 100 edits) in a large file.

**Purpose**: Verify cache efficiency across multiple edits.

**Key Metrics**:
- `duration_ns` - Total time for all edits
- `segments_invalidated` - Segments invalidated by edits
- `efficiency()` - Overall cache efficiency

#### `checkpoint_boundaries`
Benchmarks edits near checkpoint positions (before, at, after checkpoints).

**Purpose**: Verify checkpoint placement and two-sided window effectiveness.

**Key Metrics**:
- `left_checkpoint_distance` - Distance to left checkpoint
- `right_checkpoint_distance` - Distance to right checkpoint
- `segments_reused_before` - Segments reused before edit
- `segments_reused_after` - Segments reused after edit

#### `repeated_patterns`
Benchmarks edits in files with repeated code patterns (5, 10, 20 pattern repetitions).

**Purpose**: Verify 40-60% token reuse rate for repeated patterns.

**Key Metrics**:
- `duration_ns` - Time taken for incremental parse
- `tokens_reused` - Number of tokens reused from cache
- `efficiency()` - Cache efficiency percentage

### Baseline and Comparison

#### `full_relex_baseline`
Benchmarks full re-lex for various file sizes (1000, 5000, 10000, 50000 bytes).

**Purpose**: Establish baseline for comparing incremental improvements.

**Key Metrics**:
- `duration_ns` - Time for full re-lex
- `throughput` - Bytes processed per second

#### `incremental_vs_full`
Direct comparison of incremental parsing vs full re-lex for the same edit.

**Purpose**: Quantify performance improvement in a controlled scenario.

**Key Metrics**:
- `duration_ns` - Time for each approach
- `efficiency()` - Cache efficiency for incremental

### Metrics Verification

#### `metrics_tracking`
Verifies that all metrics are being tracked correctly.

**Purpose**: Ensure instrumentation is working as expected.

**Key Metrics**: All metrics (tokens_reused, tokens_relexed, segments_reused_before, etc.)

#### `cache_efficiency`
Measures cache hit rates for different edit sizes.

**Purpose**: Verify cache efficiency across different scenarios.

**Key Metrics**:
- `efficiency()` - Cache efficiency percentage
- `tokens_reused` - Number of tokens reused
- `tokens_relexed` - Number of tokens re-lexed

#### `segment_reuse`
Verifies that segment-level reuse is working.

**Purpose**: Ensure segment-based cache is functioning.

**Key Metrics**:
- `segments_reused_before` - Segments reused before edit
- `segments_reused_after` - Segments reused after edit

#### `checkpoint_distance`
Measures impact of checkpoint distance on performance.

**Purpose**: Understand how checkpoint placement affects performance.

**Key Metrics**:
- `left_checkpoint_distance` - Distance to left checkpoint
- `right_checkpoint_distance` - Distance to right checkpoint
- `duration_ns` - Time taken for incremental parse

## Running Benchmarks

### Basic Usage

```bash
# Run all benchmarks
cargo bench -p perl-incremental-parsing

# Run specific benchmark group
cargo bench -p perl-incremental-parsing <group_name>

# Run with verbose output
cargo bench -p perl-incremental-parsing -- --verbose
```

### Saving and Comparing Baselines

```bash
# Save a baseline
cargo bench -p perl-incremental-parsing -- --save-baseline main

# Compare against baseline
cargo bench -p perl-incremental-parsing -- --baseline main

# Compare against multiple baselines
cargo bench -p perl-incremental-parsing -- --baseline main --baseline previous
```

### Filtering Benchmarks

```bash
# Run benchmarks matching a pattern
cargo bench -p perl-incremental-parsing <pattern>

# Example: Run only insertion benchmarks
cargo bench -p perl-incremental-parsing insertion
```

### Performance Profiling

```bash
# Run with profiling
cargo bench -p perl-incremental-parsing -- --profile-time 5

# Generate flamegraph (requires flamegraph tool)
cargo bench -p perl-incremental-parsing -- --profile-time 5 --flamegraph
```

### CI Integration

```bash
# Run with regression detection
cargo bench -p perl-incremental-parsing -- --test

# Save CI baseline
cargo bench -p perl-incremental-parsing -- --save-baseline ci
```

## Interpreting Results

### Criterion Output

Criterion produces detailed output including:

1. **Mean/Average**: Average time per iteration
2. **Std Dev**: Standard deviation (measures consistency)
3. **Median**: Median time (less affected by outliers)
4. **Min/Max**: Best and worst case times

Example output:
```
single_char_insertion/beginning
                        time:   [1.2345 ms 1.2567 ms 1.2789 ms]
                        change: [-12.345% -10.123% -8.901%] (p = 0.000 < 0.05)
                        Performance has improved.
```

### Performance Improvement Calculation

```
improvement = (baseline_time - new_time) / baseline_time * 100%
```

Example:
- Baseline: 1.5 ms
- New: 1.2 ms
- Improvement: (1.5 - 1.2) / 1.5 * 100% = 20%

### Key Indicators

#### Good Performance
- `tokens_reused > 0`: Cache is being used
- `efficiency() >= 40%`: Good cache utilization
- `segments_reused_before + segments_reused_after > 0`: Segment reuse working
- `left_checkpoint_distance < 1000`: Well-placed checkpoints

#### Performance Issues
- `tokens_reused == 0`: Cache not being used
- `full_tail_fallbacks > 0`: Cache coverage gaps
- `efficiency() < 20%`: Poor cache utilization
- `left_checkpoint_distance > 5000`: Sparse checkpoints

## Performance Targets

### Minimum Acceptable

| Scenario | Max Duration | Min Efficiency |
|----------|--------------|----------------|
| Single char insert | 1ms | 20% |
| Single char delete | 1ms | 20% |
| Large paste (1KB) | 10ms | 30% |
| Undo/redo | 5ms | 70% |

### Stretch Goals

| Scenario | Max Duration | Min Efficiency |
|----------|--------------|----------------|
| Single char insert | 500µs | 40% |
| Single char delete | 500µs | 40% |
| Large paste (1KB) | 5ms | 50% |
| Undo/redo | 2ms | 90% |

## Troubleshooting

### Benchmarks Fail to Compile

```bash
# Install cargo-criterion
cargo install -f cargo-criterion

# Or use regular cargo bench
cargo bench -p perl-incremental-parsing
```

### Benchmarks Run Slowly

```bash
# Reduce sample size
cargo bench -p perl-incremental-parsing -- --sample-size 10

# Reduce warm-up time
cargo bench -p perl-incremental-parsing -- --warm-up-time 1

# Run specific groups
cargo bench -p perl-incremental-parsing single_char_insertion
```

### Inconsistent Results

- Close other applications
- Run multiple times and average
- Use consistent power settings (disable CPU throttling)
- Run on a dedicated machine if possible

### Regression Detection Fails

```bash
# Check if regression is real or noise
cargo bench -p perl-incremental-parsing -- --sample-size 100

# Compare against previous baseline
cargo bench -p perl-incremental-parsing -- --baseline previous

# Update baseline if regression is acceptable
cargo bench -p perl-incremental-parsing -- --save-baseline main
```

## Advanced Usage

### Custom Benchmark Config

Create `.cargo/config.toml`:
```toml
[bench]
# Reduce iterations for faster development
sample-size = 10
warm-up-time = 1
measurement-time = 3
```

### Plotting Results

```bash
# Generate plots
cargo bench -p perl-incremental-parsing -- --plotting-backend gnuplot

# View plots in target/criterion/
```

### HTML Report

```bash
# Generate HTML report
cargo bench -p perl-incremental-parsing -- --output-format html

# Open report
open target/criterion/report/index.html
```

## Contributing

When adding new benchmarks:

1. **Follow naming convention**: Use descriptive, snake_case names
2. **Track all relevant metrics**: Include `tokens_reused`, `tokens_relexed`, etc.
3. **Add documentation**: Explain purpose and key metrics
4. **Test locally**: Run benchmarks before committing
5. **Update documentation**: Add to this README and BENCHMARK_REPORT.md

Example:
```rust
fn bench_new_scenario(c: &mut Criterion) {
    let mut group = c.benchmark_group("new_scenario");
    
    // Add documentation
    group.bench_function("description", |b| {
        b.iter(|| {
            // Benchmark code
        })
    });
    
    group.finish();
}
```

## Related Documentation

- [BENCHMARK_REPORT.md](../BENCHMARK_REPORT.md) - Detailed benchmark report
- [Issue #3527](https://github.com/EffortlessMetrics/perl-lsp/issues/3527) - Original issue
- [ISSUE_3527_RESCOPE.md](../../docs/project/ISSUE_3527_RESCOPE.md) - Implementation scope
- [incremental_checkpoint.rs](../src/incremental/incremental_checkpoint.rs) - Implementation
- [segment_cache_checkpoint_window_tests.rs](../tests/segment_cache_checkpoint_window_tests.rs) - Tests

## Performance Tips

### For Development

- Use `--sample-size 10` for faster iteration
- Focus on specific benchmark groups
- Use `--test` for regression detection

### For Production

- Run full benchmark suite
- Save baselines for comparison
- Monitor for regressions
- Use CI integration

### For Analysis

- Generate HTML reports
- Plot results over time
- Compare against baselines
- Profile slow benchmarks

## License

See [LICENSE-APACHE](../LICENSE-APACHE) and [LICENSE-MIT](../LICENSE-MIT) for details.
