# Tree-sitter Perl Implementation Comparison

**Generated:** 2025-08-27T11:00:10.021971931+00:00

## Summary

- **Time Performance:** Rust implementation is 89.7% faster than C implementation
- **Memory Usage:** Rust implementation uses 0.0% more memory than C implementation
- **Success Rate:** C: 8/21 (38%), Rust: 4/21 (19%)

## Detailed Results

| Metric | C Implementation | Rust Implementation | Difference |
|--------|------------------|---------------------|------------|
| Avg Time (μs) | 915.16 | 94.46 | -89.7% |
| Avg Memory (MB) | 0.54 | 0.54 | 0.0% |
| Total Time (μs) | 19218.41 | 1983.70 | -89.7% |
| Total Memory (MB) | 11.41 | 11.41 | 0.0% |

## Test Case Results

| Test Case | C Time (μs) | Rust Time (μs) | C Memory (MB) | Rust Memory (MB) | Time Diff | Memory Diff |
|-----------|-------------|----------------|---------------|------------------|-----------|-------------|
| benchmark_tests/medium.pl | 198.74 | 61.96 | 0.50 | 0.50 | -68.8% | 0.0% |
| benchmark_tests/large_50kb.pl | 2981.62 | 191.28 | 0.67 | 0.67 | -93.6% | 0.0% |
| benchmark_tests/complex_operators.pl | 1078.57 | 38.93 | 0.54 | 0.54 | -96.4% | 0.0% |
| benchmark_tests/simple.pl | 42.85 | 21.57 | 0.50 | 0.50 | -49.7% | 0.0% |
| benchmark_tests/obscure_syntax.pl | 94.05 | 60.87 | 0.53 | 0.53 | -35.3% | 0.0% |
| benchmark_tests/complex.pl | 263.88 | 93.69 | 0.51 | 0.51 | -64.5% | 0.0% |
| benchmark_tests/large_5kb.pl | 1393.44 | 90.57 | 0.57 | 0.57 | -93.5% | 0.0% |
| benchmark_tests/edge_cases.pl | 1206.47 | 37.33 | 0.53 | 0.53 | -96.9% | 0.0% |
| benchmark_tests/complex_nested.pl | 650.35 | 111.23 | 0.54 | 0.54 | -82.9% | 0.0% |
| benchmark_tests/complex_regex.pl | 756.89 | 64.86 | 0.54 | 0.54 | -91.4% | 0.0% |
| benchmark_tests/fuzzed/fuzz_complex_000.pl | 377.30 | 110.23 | 0.51 | 0.51 | -70.8% | 0.0% |
| benchmark_tests/fuzzed/fuzz_complex_nested_000.pl | 819.55 | 140.48 | 0.55 | 0.55 | -82.9% | 0.0% |
| benchmark_tests/fuzzed/fuzz_complex_operators_000.pl | 1076.64 | 53.96 | 0.55 | 0.55 | -95.0% | 0.0% |
| benchmark_tests/fuzzed/fuzz_complex_regex_000.pl | 785.49 | 62.27 | 0.54 | 0.54 | -92.1% | 0.0% |
| benchmark_tests/fuzzed/fuzz_edge_cases_000.pl | 1211.31 | 43.00 | 0.54 | 0.54 | -96.5% | 0.0% |
| benchmark_tests/fuzzed/fuzz_large_50kb_000.pl | 3471.33 | 229.65 | 0.68 | 0.68 | -93.4% | 0.0% |
| benchmark_tests/fuzzed/fuzz_large_5kb_000.pl | 2030.44 | 83.56 | 0.57 | 0.57 | -95.9% | 0.0% |
| benchmark_tests/fuzzed/fuzz_medium_000.pl | 246.82 | 25.39 | 0.50 | 0.50 | -89.7% | 0.0% |
| benchmark_tests/fuzzed/fuzz_obscure_syntax_000.pl | 101.80 | 64.18 | 0.53 | 0.53 | -37.0% | 0.0% |
| benchmark_tests/fuzzed/fuzz_simple_000.pl | 149.64 | 26.96 | 0.50 | 0.50 | -82.0% | 0.0% |
| benchmark_tests/fuzzed/stress_deep_nesting.pl | 281.23 | 371.73 | 0.50 | 0.50 | 32.2% | 0.0% |
