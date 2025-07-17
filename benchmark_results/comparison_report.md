# Tree-sitter Perl Implementation Comparison

**Generated:** 2025-07-17T01:02:55.990201772+00:00

## Summary

- **Performance:** Rust implementation is 0.5% slower than C implementation
- **Success Rate:** C: 13/14 (92%), Rust: 13/14 (92%)

## Detailed Results

| Metric | C Implementation | Rust Implementation | Difference |
|--------|------------------|---------------------|------------|
| Avg Time (μs) | 0.00 | 0.00 | 0.5% |
| Total Time (μs) | 2356.18 | 2368.26 | 0.5% |

## Test Case Results

| Test Case | C Time (μs) | Rust Time (μs) | Difference |
|-----------|-------------|----------------|------------|
| test/corpus/subroutines | 135.81 | 135.05 | -0.6% |
| test/corpus/pod | 714.96 | 711.69 | -0.5% |
| test/corpus/operators | 178.99 | 176.45 | -1.4% |
| test/corpus/statements | 208.06 | 211.11 | 1.5% |
| test/corpus/regexp | 70.93 | 75.42 | 6.3% |
| test/corpus/autoquote | 123.26 | 122.80 | -0.4% |
| test/corpus/simple | 76.98 | 75.13 | -2.4% |
| test/corpus/map-grep | 110.52 | 110.28 | -0.2% |
| test/corpus/heredocs | 68.84 | 71.21 | 3.4% |
| test/corpus/interpolation | 123.98 | 125.01 | 0.8% |
| test/corpus/variables | 132.51 | 139.11 | 5.0% |
| test/corpus/expressions | 172.88 | 175.40 | 1.5% |
| test/corpus/literals | 104.86 | 106.36 | 1.4% |
| test/corpus/functions | 133.60 | 133.24 | -0.3% |
