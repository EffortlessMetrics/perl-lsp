# Issue: Performance Benchmarks for Perl Corpus

**Status**: Open  
**Priority**: P2  
**Created**: 2026-01-07  
**Area**: Corpus Testing Infrastructure

## Problem Description

The Perl corpus lacks dedicated performance regression tests that track parser performance over time. While the repository has benchmarking capabilities (see [`benches/`](../benches/)), there are no corpus-specific performance tests that:

1. Track parsing performance for corpus files
2. Detect performance regressions when parser changes are made
3. Establish performance baselines for different file sizes
4. Measure LSP-specific performance on complex corpus files
5. Provide performance data for optimization decisions

Without performance benchmarks in the corpus, we cannot:
- Detect performance regressions early
- Validate performance claims (e.g., "4-19x faster")
- Make informed optimization decisions
- Provide performance guarantees to users
- Compare against future parser improvements

## Impact Assessment

**Why This Matters:**

1. **Regression Prevention**: Performance regressions can make the parser feel sluggish without detection
2. **Optimization Guidance**: Performance data identifies bottlenecks for optimization
3. **Release Confidence**: Quantitative performance metrics support release decisions
4. **User Experience**: Slow parsing impacts LSP responsiveness and user experience
5. **Competitive Positioning**: Performance data helps compare against other parsers

**Current State:**
- [`benches/`](../benches/) directory contains general parser benchmarks
- No corpus-specific performance tracking
- No automated performance regression detection
- No performance baselines for corpus files
- No LSP-specific performance benchmarks

## Current State

**What's Missing:**

1. **Corpus performance tests** - No tests measuring parsing time for corpus files
2. **Performance baselines** - No established baseline metrics for corpus files
3. **Regression detection** - No automated detection of performance regressions
4. **LSP performance tests** - No tests measuring LSP performance on corpus files
5. **Performance trends** - No tracking of performance changes over time
6. **File size correlations** - No analysis of performance vs. file size

**Existing Infrastructure:**
- [`benches/`](../benches/) directory with general parser benchmarks
- [`cargo bench`](../) command for running benchmarks
- Criterion benchmarking framework available
- No corpus-specific benchmark infrastructure

## Recommended Path Forward

### Phase 1: Establish Performance Baselines

**Objective**: Create baseline performance metrics for corpus files

**Steps:**
1. Add performance test module to [`crates/perl-corpus/`](../crates/perl-corpus/)
2. Implement benchmark functions for each corpus file:
   ```rust
   use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
   use perl_parser::Parser;
   use std::fs;
   
   fn parse_corpus_file(c: &mut Criterion) {
       let source = fs::read_to_string("test_corpus/advanced_regex.pl").unwrap();
       c.bench_function("parse_advanced_regex", |b| {
           b.iter(|| {
               let mut parser = Parser::new();
               let _ = parser.parse(&source, "test.pl");
           });
       });
   }
   ```
3. Run benchmarks for all corpus files
4. Document baseline metrics in `docs/performance_baselines.md`
5. Categorize by file size (small, medium, large)

**Deliverable**: Baseline performance metrics for corpus files

### Phase 2: Add LSP Performance Tests

**Objective**: Measure LSP-specific performance on corpus files

**Steps:**
1. Create LSP benchmark suite in [`crates/perl-lsp/benches/`](../crates/perl-lsp/benches/)
2. Implement benchmarks for LSP operations:
   - `textDocument/didOpen` - Time to open and parse
   - `textDocument/documentSymbol` - Time to extract symbols
   - `textDocument/completion` - Time to provide completions
   - `textDocument/definition` - Time to find definitions
   - `textDocument/hover` - Time to provide hover info
3. Use corpus files as test data
4. Measure performance with realistic editor interactions
5. Document LSP performance baselines

**Deliverable**: LSP performance benchmarks using corpus files

### Phase 3: Implement Regression Detection

**Objective**: Automate detection of performance regressions

**Steps:**
1. Add regression detection to CI pipeline
2. Define performance thresholds (e.g., 10% degradation triggers warning)
3. Store baseline metrics in CI artifacts
4. Compare current benchmarks against baselines
5. Fail CI if performance degrades beyond threshold
6. Generate performance trend reports

**Deliverable**: Automated performance regression detection in CI

### Phase 4: Add Performance Trending

**Objective**: Track performance changes over time

**Steps:**
1. Store benchmark results in versioned artifacts
2. Generate performance trend graphs
3. Correlate performance with code changes
4. Identify performance improvements and degradations
5. Document performance history in `docs/performance_history.md`

**Deliverable**: Performance trend tracking and visualization

## Priority Level

**P2 - Medium Priority**

This is a P2 issue because:
1. Important for long-term maintainability but not blocking
2. Performance regressions can be caught through user reports
3. Existing benchmarking infrastructure provides foundation
4. Can be addressed incrementally
5. Lower risk than syntax coverage gaps

## Estimated Effort

**Total Effort**: Medium

- Phase 1 (Performance Baselines): 2-3 days
- Phase 2 (LSP Performance Tests): 3-4 days
- Phase 3 (Regression Detection): 2-3 days
- Phase 4 (Performance Trending): 2-3 days

## Related Issues

- [Test Coverage Metrics](corpus-coverage-test-metrics.md) - Related corpus quality improvement
- [Error Recovery Tests](corpus-coverage-error-recovery.md) - Related robustness testing

## References

- [`benches/`](../benches/) - Existing benchmark infrastructure
- [Criterion Documentation](https://bheisler.github.io/criterion.rs/book/) - Benchmarking framework
- [Performance Benchmark Framework](../docs/benchmarks/BENCHMARK_FRAMEWORK.md) - Project benchmarking guidelines

## Success Criteria

1. Performance baselines established for all corpus files
2. LSP performance benchmarks implemented and passing
3. Regression detection integrated into CI pipeline
4. Performance trend tracking implemented
5. Performance degradation alerts configured
6. Performance history documented

## Open Questions

1. What performance degradation threshold should trigger CI failure? (Suggested: 10-15%)
2. Should performance be measured for incremental parsing updates?
3. How should performance be correlated with file size/complexity?
4. Should there be separate performance targets for different Perl versions?
