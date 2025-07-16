# Tree-sitter Perl Implementation Comparison

**Generated:** 2025-07-16T23:54:31.828072824+00:00

## Summary

- **Performance:** Rust implementation is 0.0% slower than C implementation
- **Success Rate:** C: 13/14 (92%), Rust: 0/14 (0%)

## Detailed Results

| Metric | C Implementation | Rust Implementation | Difference |
|--------|------------------|---------------------|------------|
| Avg Time (μs) | 0.00 | 0.00 | 0.0% |
| Total Time (μs) | 2343.13 | 125493.56 | 5255.8% |

## Test Case Results

| Test Case | C Time (μs) | Rust Time (μs) | Difference |
|-----------|-------------|----------------|------------|
| test/corpus/subroutines | 140.03 | 11512.64 | 8121.6% |
| test/corpus/pod | 706.94 | 1181.95 | 67.2% |
| test/corpus/operators | 174.44 | 13398.40 | 7580.8% |
| test/corpus/statements | 210.00 | 18860.72 | 8881.3% |
| test/corpus/regexp | 74.32 | 5671.15 | 7530.7% |
| test/corpus/autoquote | 128.47 | 8733.97 | 6698.5% |
| test/corpus/simple | 73.85 | 4798.41 | 6397.5% |
| test/corpus/map-grep | 105.06 | 8102.78 | 7612.5% |
| test/corpus/heredocs | 68.05 | 6287.98 | 9140.2% |
| test/corpus/interpolation | 124.59 | 8989.85 | 7115.5% |
| test/corpus/variables | 136.67 | 8849.69 | 6375.2% |
| test/corpus/expressions | 165.00 | 11080.99 | 6615.8% |
| test/corpus/literals | 100.63 | 7738.16 | 7589.7% |
| test/corpus/functions | 135.08 | 10286.87 | 7515.4% |
