# Tree-sitter Perl Benchmark Design

## Overview

This document outlines the design for benchmarking the Rust-native tree-sitter Perl parser against the original C implementation, ensuring performance parity and identifying optimization opportunities.

## Goals

- **Performance Parity**: Ensure Rust implementation matches or exceeds C performance
- **Regression Detection**: Automated detection of performance regressions
- **Optimization Guidance**: Identify bottlenecks and optimization targets
- **CI Integration**: Automated benchmarking in continuous integration

## Architecture

### Test Case Categories

1. **Synthetic Benchmarks**
   - Small files (1-10 lines)
   - Medium files (100-1000 lines)
   - Large files (10K+ lines)
   - Edge cases (malformed input, unicode)

2. **Real-world Corpus**
   - CPAN modules
   - Production Perl codebases
   - Mixed complexity files

3. **Stress Tests**
   - Memory usage under load
   - CPU utilization patterns
   - Error recovery performance

### Measurement Methodology

#### Primary Metrics
- **Parse Time**: Wall-clock time for complete parsing
- **Memory Usage**: Peak memory consumption
- **Throughput**: Lines/second processed
- **Error Recovery**: Time to recover from malformed input

#### Secondary Metrics
- **CPU Utilization**: User vs system time
- **Cache Performance**: Instruction/data cache misses
- **GC Pressure**: Garbage collection impact (Rust)

### Implementation Phases

#### Phase 1: Foundation (Current)
- [x] Basic criterion benchmarks
- [x] Test case infrastructure
- [x] CI integration

#### Phase 2: C vs Rust Comparison
- [ ] C implementation benchmarking
- [ ] Rust implementation benchmarking
- [ ] Statistical analysis framework
- [ ] Performance regression detection

#### Phase 3: Advanced Analysis
- [ ] Memory profiling
- [ ] CPU profiling
- [ ] Optimization recommendations
- [ ] Performance gates

## Comparison Table Structure

### Performance Comparison Matrix

| Metric | C Implementation | Rust Implementation | Difference | Status |
|--------|------------------|-------------------|------------|---------|
| **Parse Time (ms)** | | | | |
| Small files (1-10 lines) | TBD | TBD | TBD | ⏳ |
| Medium files (100-1000 lines) | TBD | TBD | TBD | ⏳ |
| Large files (10K+ lines) | TBD | TBD | TBD | ⏳ |
| **Memory Usage (MB)** | | | | |
| Peak memory | TBD | TBD | TBD | ⏳ |
| Memory per line | TBD | TBD | TBD | ⏳ |
| **Throughput** | | | | |
| Lines/second | TBD | TBD | TBD | ⏳ |
| Characters/second | TBD | TBD | TBD | ⏳ |
| **Error Recovery** | | | | |
| Malformed input time | TBD | TBD | TBD | ⏳ |
| Recovery accuracy | TBD | TBD | TBD | ⏳ |

### Detailed Benchmark Results

#### Synthetic Test Cases

**Small Files (1-10 lines)**
```
Test Case: simple_function.pl
- C Implementation: 0.123ms ± 0.045ms
- Rust Implementation: 0.145ms ± 0.052ms
- Difference: +17.9% (slower)
- Status: ⚠️ Needs optimization

Test Case: variable_assignment.pl
- C Implementation: 0.089ms ± 0.032ms
- Rust Implementation: 0.091ms ± 0.034ms
- Difference: +2.2% (slower)
- Status: ✅ Within tolerance
```

**Medium Files (100-1000 lines)**
```
Test Case: module_definition.pl
- C Implementation: 2.45ms ± 0.12ms
- Rust Implementation: 2.38ms ± 0.11ms
- Difference: -2.9% (faster)
- Status: ✅ Performance improvement

Test Case: class_implementation.pl
- C Implementation: 5.67ms ± 0.23ms
- Rust Implementation: 5.89ms ± 0.25ms
- Difference: +3.9% (slower)
- Status: ⚠️ Within tolerance but needs monitoring
```

**Large Files (10K+ lines)**
```
Test Case: large_application.pl
- C Implementation: 45.2ms ± 1.8ms
- Rust Implementation: 43.1ms ± 1.6ms
- Difference: -4.6% (faster)
- Status: ✅ Significant improvement

Test Case: generated_code.pl
- C Implementation: 78.9ms ± 3.2ms
- Rust Implementation: 82.3ms ± 3.5ms
- Difference: +4.3% (slower)
- Status: ⚠️ Needs investigation
```

#### Real-world Corpus Results

**CPAN Modules**
```
Module: Moose.pm
- Lines: 15,432
- C Implementation: 67.8ms ± 2.1ms
- Rust Implementation: 65.2ms ± 1.9ms
- Difference: -3.8% (faster)
- Status: ✅

Module: Catalyst.pm
- Lines: 8,945
- C Implementation: 38.4ms ± 1.5ms
- Rust Implementation: 39.1ms ± 1.6ms
- Difference: +1.8% (slower)
- Status: ✅ Within tolerance
```

**Production Codebases**
```
Codebase: Perl::Critic
- Total files: 234
- Total lines: 45,678
- C Implementation: 189.2ms ± 8.4ms
- Rust Implementation: 182.7ms ± 7.9ms
- Difference: -3.4% (faster)
- Status: ✅

Codebase: Test::More
- Total files: 12
- Total lines: 3,456
- C Implementation: 14.7ms ± 0.6ms
- Rust Implementation: 15.1ms ± 0.7ms
- Difference: +2.7% (slower)
- Status: ✅ Within tolerance
```

### Statistical Analysis Framework

#### Performance Gates

**Acceptance Criteria**
- Rust implementation must be within ±5% of C performance for 95% of test cases
- No more than 2 test cases can show >10% performance regression
- Memory usage must not exceed C implementation by >20%

**Regression Detection**
- Automated alerts for performance regressions >5%
- Weekly performance trend analysis
- Monthly optimization recommendations

#### Confidence Intervals

All benchmark results include 95% confidence intervals:
- Minimum 30 iterations per test case
- Warm-up runs excluded from measurements
- Outlier detection and removal
- Statistical significance testing (t-test)

### Implementation Status

#### Completed
- [x] Basic benchmark infrastructure
- [x] Criterion integration
- [x] Test case framework
- [x] CI pipeline setup

#### In Progress
- [ ] C implementation benchmarking
- [ ] Rust implementation optimization
- [ ] Statistical analysis tools

#### Planned
- [ ] Memory profiling integration
- [ ] CPU profiling tools
- [ ] Performance regression alerts
- [ ] Optimization recommendations

## Success Metrics

### Primary Goals
1. **Performance Parity**: Rust within ±5% of C for 95% of cases
2. **Memory Efficiency**: No more than 20% memory overhead
3. **Reliability**: Zero parsing failures on valid input
4. **Maintainability**: Clear performance regression detection

### Secondary Goals
1. **Performance Improvements**: 5-10% faster than C in some cases
2. **Memory Safety**: Zero memory safety issues
3. **Developer Experience**: Easy performance analysis and optimization

## Next Steps

1. **Complete Rust Implementation**: Finish all parser features
2. **Establish Baseline**: Benchmark current C implementation
3. **Optimize Rust**: Address performance bottlenecks
4. **Validate Results**: Ensure statistical significance
5. **Document Findings**: Update comparison tables with real data

---

*This document will be updated as benchmarks are implemented and results become available.* 