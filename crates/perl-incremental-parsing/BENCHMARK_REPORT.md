# Incremental Parsing Benchmark Report

## Overview

This report documents the benchmark suite for verifying performance improvements from issue #3527:
- Segment-based token cache implementation
- Two-sided checkpoint window implementation
- Enhanced metrics tracking

## Benchmark Suite

The benchmark suite is located at [`benches/incremental_parsing_benchmarks.rs`](benches/incremental_parsing_benchmarks.rs) and uses the [Criterion](https://github.com/bheisler/criterion.rs) framework for statistical benchmarking.

### Running the Benchmarks

```bash
# Run all benchmarks
cargo bench -p perl-incremental-parsing

# Run a specific benchmark group
cargo bench -p perl-incremental-parsing single_char_insertion

# Run with detailed output
cargo bench -p perl-incremental-parsing -- --verbose

# Save baseline for comparison
cargo bench -p perl-incremental-parsing -- --save-baseline main

# Compare against baseline
cargo bench -p perl-incremental-parsing -- --baseline main
```

### Benchmark Groups

| Group | Description | Key Metrics |
|-------|-------------|-------------|
| `single_char_insertion` | Single character insertions at various positions | Duration, tokens_reused, tokens_relexed |
| `single_char_deletion` | Single character deletions at various positions | Duration, tokens_reused, tokens_relexed |
| `large_paste` | Large paste operations (100-5000 bytes) | Duration, throughput, bytes_relexed |
| `undo_redo` | Edit-undo-redo pattern | Duration, cache efficiency |
| `repeated_edits` | Multiple edits in large file | Duration, segments_invalidated |
| `checkpoint_boundaries` | Edits near checkpoint positions | Checkpoint distances, segment reuse |
| `repeated_patterns` | Edits in files with repeated code patterns | Token reuse rate |
| `full_relex_baseline` | Full re-lex baseline for comparison | Duration, source size |
| `incremental_vs_full` | Direct comparison of incremental vs full parse | Duration, efficiency |
| `metrics_tracking` | Verify all metrics are being tracked | All metric values |
| `cache_efficiency` | Measure cache hit rates | Efficiency percentage |
| `segment_reuse` | Verify segment-level reuse | Segments reused before/after |
| `checkpoint_distance` | Impact of checkpoint distance | Checkpoint distances, duration |

## Performance Expectations

Based on similar implementations (rust-analyzer, swc), the following performance improvements are expected:

### Single Character Edits
- **Expected improvement**: 10-20% faster than full re-lex
- **Key metric**: `tokens_reused > 0` in typical scenarios
- **Best case**: Edit near checkpoint with high cache coverage

### Large Paste Operations
- **Expected improvement**: 30-50% faster (if content repeated)
- **Key metric**: High `tokens_reused` / `tokens_relexed` ratio
- **Best case**: Pasting content that matches cached patterns

### Undo/Redo Operations
- **Expected improvement**: 70-90% faster (content already cached)
- **Key metric**: `cache_hits >> cache_misses`
- **Best case**: Reverting to previously cached state

### Repeated Code Patterns
- **Expected improvement**: 40-60% token reuse rate
- **Key metric**: `efficiency() >= 40.0`
- **Best case**: Files with many similar functions/constructs

## Metrics Tracked

### Core Metrics

| Metric | Description | Expected Value |
|--------|-------------|----------------|
| `tokens_reused` | Number of tokens reused from cache | > 0 for incremental parses |
| `tokens_relexed` | Number of tokens re-lexed | Varies by edit size |
| `segments_reused_before` | Segments reused before edit | > 0 for typical edits |
| `segments_reused_after` | Segments reused after edit | > 0 for typical edits |
| `segments_invalidated` | Segments invalidated by edit | Proportional to edit size |
| `bytes_relexed` | Total bytes re-lexed | < source.len() for incremental |

### Checkpoint Metrics

| Metric | Description | Expected Value |
|--------|-------------|----------------|
| `left_checkpoint_distance` | Distance from edit to left checkpoint | 0-5000 bytes |
| `right_checkpoint_distance` | Distance from edit to right checkpoint | 0-5000 bytes |
| `full_tail_fallbacks` | Times we had to relex entire tail | Should be low |

### Derived Metrics

| Metric | Calculation | Interpretation |
|--------|-------------|----------------|
| `efficiency()` | `tokens_reused / (tokens_reused + tokens_relexed) * 100` | Higher is better (0-100%) |
| `bytes_reuse_ratio` | `tokens_reused / bytes_relexed * 100` | Token density in relexed region |

## Interpreting Results

### Good Performance Indicators

1. **High Token Reuse**: `tokens_reused >> tokens_relexed`
   - Indicates cache is working effectively
   - Expected for small edits in large files

2. **High Segment Reuse**: `segments_reused_before + segments_reused_after > 0`
   - Indicates segment-based cache is functioning
   - Should be true for most non-trivial edits

3. **Low Checkpoint Distances**: `left_checkpoint_distance` and `right_checkpoint_distance` small
   - Indicates checkpoints are well-distributed
   - Should be < 1000 bytes for typical files

4. **High Efficiency**: `efficiency() >= 40.0`
   - Indicates good cache utilization
   - Target: 40-60% for repeated patterns

### Performance Issues to Investigate

1. **Zero Token Reuse**: `tokens_reused == 0`
   - May indicate cache is not being used
   - Check checkpoint placement and cache invalidation logic

2. **High Full Tail Fallbacks**: `full_tail_fallbacks > 0`
   - Indicates cache coverage gaps
   - May need more checkpoints or better segment management

3. **High Checkpoint Distances**: `left_checkpoint_distance > 5000`
   - Indicates sparse checkpoint placement
   - May need to adjust checkpoint generation strategy

4. **Low Efficiency**: `efficiency() < 20.0`
   - Cache not providing significant benefit
   - May need to review cache hit criteria

## Benchmark Results Template

When running benchmarks, use this template to document results:

```markdown
### Benchmark Run: <Date>

#### Environment
- Rust version: <version>
- Platform: <platform>
- CPU: <cpu>
- RAM: <ram>

#### Results Summary

| Benchmark | Duration (ns) | Tokens Reused | Tokens Relexed | Efficiency | Notes |
|-----------|---------------|---------------|----------------|------------|-------|
| single_char_insertion/beginning | <value> | <value> | <value> | <value>% | <notes> |
| single_char_insertion/middle | <value> | <value> | <value> | <value>% | <notes> |
| single_char_insertion/end | <value> | <value> | <value> | <value>% | <notes> |
| ... | ... | ... | ... | ... | ... |

#### Performance Improvements

- Single char insertion: <X>% faster than baseline
- Large paste: <Y>% faster than baseline
- Undo/redo: <Z>% faster than baseline

#### Issues Found

- <any issues or regressions>

#### Recommendations

- <any recommendations for improvement>
```

## CI Integration

To run benchmarks in CI:

```bash
# Add to CI pipeline
cargo bench -p perl-incremental-parsing -- --save-baseline ci

# Store baseline artifacts
# Compare against previous runs
cargo bench -p perl-incremental-parsing -- --baseline previous
```

## Regression Detection

Criterion provides built-in regression detection:

```bash
# Run with regression detection
cargo bench -p perl-incremental-parsing -- --test

# This will fail if performance regresses beyond statistical significance
```

## Performance Targets

### Minimum Acceptable Performance

| Scenario | Max Duration | Min Efficiency |
|----------|--------------|----------------|
| Single char insert | 1ms | 20% |
| Single char delete | 1ms | 20% |
| Large paste (1KB) | 10ms | 30% |
| Undo/redo | 5ms | 70% |
| Repeated edits (10) | 50ms | 40% |

### Stretch Goals

| Scenario | Max Duration | Min Efficiency |
|----------|--------------|----------------|
| Single char insert | 500µs | 40% |
| Single char delete | 500µs | 40% |
| Large paste (1KB) | 5ms | 50% |
| Undo/redo | 2ms | 90% |
| Repeated edits (10) | 25ms | 60% |

## Troubleshooting

### Benchmarks Fail to Compile

Ensure criterion is available:
```bash
cargo install -f cargo-criterion
```

### Benchmarks Run Slowly

- Reduce sample size: `cargo bench -- --sample-size 10`
- Reduce warm-up time: `cargo bench -- --warm-up-time 1`
- Run specific groups: `cargo bench single_char_insertion`

### Inconsistent Results

- Close other applications
- Run multiple times and average
- Use `--plotting-backend disabled` to reduce overhead

## Related Documentation

- [Issue #3527](https://github.com/EffortlessMetrics/perl-lsp/issues/3527)
- [ISSUE_3527_RESCOPE.md](../../docs/project/ISSUE_3527_RESCOPE.md)
- [Incremental Checkpoint Implementation](src/incremental/incremental_checkpoint.rs)
- [Segment Cache Tests](tests/segment_cache_checkpoint_window_tests.rs)

## Contributing

When adding new benchmarks:

1. Follow the existing naming convention
2. Track all relevant metrics
3. Add documentation for the benchmark
4. Update this report with the new benchmark
5. Run benchmarks before and after changes
