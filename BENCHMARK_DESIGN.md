# Benchmark Design Document

This document outlines the design and implementation plan for comprehensive C vs Rust performance benchmarking of the tree-sitter-perl implementation.

## 🎯 Design Goals

### Primary Objectives
1. **Accurate Performance Comparison**: Fair and statistically valid C vs Rust comparison
2. **Comprehensive Coverage**: All critical performance aspects (parsing, memory, scalability)
3. **Regression Detection**: Automated performance regression testing
4. **CI Integration**: Performance gates for continuous integration
5. **Transparency**: Clear methodology and reproducible results

### Success Criteria
- **Statistical Validity**: 95% confidence intervals for all comparisons
- **Fair Comparison**: Identical test conditions for both implementations
- **Automation**: Fully automated benchmarking pipeline
- **Documentation**: Clear methodology and result interpretation

## 🏗️ Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    Benchmark Pipeline                           │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │
│  │   Rust Tests    │  │   C Tests       │  │   Comparison    │  │
│  │   (Criterion)   │  │   (Node.js)     │  │   Engine        │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘  │
│           │                     │                     │          │
│           ▼                     ▼                     ▼          │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │                 Statistical Analysis                        │  │
│  │              (Confidence Intervals)                         │  │
│  └─────────────────────────────────────────────────────────────┘  │
│           │                                                     │
│           ▼                                                     │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │                 Regression Detection                        │  │
│  │              (Performance Gates)                            │  │
│  └─────────────────────────────────────────────────────────────┘  │
│           │                                                     │
│           ▼                                                     │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │                 Result Storage                              │  │
│  │              (Historical Data)                              │  │
│  └─────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

## 📊 Test Case Design

### Core Test Categories

#### **1. Basic Parsing Performance**
```perl
# Simple variable declaration
my $var = 42;

# Function call
print 'Hello, World!';

# Control structure
if ($condition) { $action = 1; }

# Loop construct
for my $i (1..10) { print $i; }
```

#### **2. Complex Parsing Performance**
```perl
# Class definition
package MyClass;
use strict;
use warnings;

sub new {
    my ($class, %args) = @_;
    bless \%args, $class;
}

sub method {
    my ($self, @params) = @_;
    return $self->{value} + @params;
}
```

#### **3. Unicode Performance**
```perl
# Unicode identifiers
my $変数 = "値";
my $über = "cool";
my $naïve = "simple";
sub 関数 { return "関数です"; }
```

#### **4. String Processing**
```perl
# Heredoc
my $heredoc = <<"EOF";
This is a here document
with multiple lines
of content
EOF

# String interpolation
my $template = qq{
<html>
<head><title>$title</title></head>
<body>
<h1>$heading</h1>
<p>$content</p>
</body>
</html>
};
```

#### **5. Regex Performance**
```perl
# Complex regex patterns
my $pattern1 = qr/^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$/;
my $pattern2 = qr{\d{3}-\d{3}-\d{4}};
my $pattern3 = qr/^(https?:\/\/)?([\da-z\.-]+)\.([a-z\.]{2,6})([\/\w \.-]*)*\/?$/;
```

### Scalability Test Cases

#### **Input Size Scaling**
- **Small**: 1KB (basic constructs)
- **Medium**: 10KB (functions and classes)
- **Large**: 100KB (complex applications)
- **Very Large**: 1MB (real-world files)

#### **Complexity Scaling**
- **Simple**: Basic Perl constructs
- **Moderate**: Functions and control structures
- **Complex**: Classes, modules, and advanced features
- **Very Complex**: Real-world Perl applications

## 🔧 Implementation Plan

### Phase 1: Rust Criterion Benchmarks ✅ **Complete**

#### **Current Implementation**
- Criterion-based benchmarking
- Statistical analysis with confidence intervals
- Automated result storage
- Xtask integration

#### **Files**
- `benches/scanner_benchmarks.rs`
- `benches/parser_benchmarks.rs`
- `xtask/src/tasks/bench.rs`

### Phase 2: C Implementation Benchmarks 🔄 **Planned**

#### **Node.js Benchmark Setup**
```javascript
// test/benchmark.js
const Parser = require('tree-sitter');
const Perl = require('./tree-sitter-perl');

const parser = new Parser();
parser.setLanguage(Perl);

const code = process.env.TEST_CODE;
const iterations = parseInt(process.env.ITERATIONS) || 100;

const start = Date.now();
for (let i = 0; i < iterations; i++) {
    parser.parse(code);
}
const duration = Date.now() - start;

console.log(JSON.stringify({
    duration,
    iterations,
    average: duration / iterations
}));
```

#### **Test Case Runner**
```bash
#!/bin/bash
# scripts/run_c_benchmarks.sh

TEST_CASES=(
    "simple_variable:my \$var = 42;"
    "function_call:print 'Hello, World!';"
    "control_structure:if (\$condition) { \$action = 1; }"
    "unicode_code:my \$変数 = '値';"
    "heredoc:my \$heredoc = <<\"EOF\";\nContent\nEOF"
)

for test_case in "${TEST_CASES[@]}"; do
    IFS=':' read -r name code <<< "$test_case"
    echo "Running $name..."
    
    export TEST_CODE="$code"
    export ITERATIONS=100
    
    result=$(node test/benchmark.js)
    echo "$name: $result" >> results/c_benchmarks.json
done
```

### Phase 3: Comparison Engine 🔄 **Planned**

#### **Statistical Comparison**
```rust
#[derive(Debug, Clone, serde::Serialize)]
pub struct BenchmarkComparison {
    pub test_name: String,
    pub rust_result: BenchmarkResult,
    pub c_result: BenchmarkResult,
    pub speedup: f64,
    pub confidence_interval: (f64, f64),
    pub statistical_significance: bool,
}

impl BenchmarkComparison {
    pub fn calculate_speedup(&self) -> f64 {
        self.c_result.duration.as_nanos() as f64 / 
        self.rust_result.duration.as_nanos() as f64
    }
    
    pub fn is_statistically_significant(&self) -> bool {
        // Implement statistical significance testing
        // Using t-test or similar statistical method
        true
    }
}
```

#### **Regression Detection**
```rust
pub struct PerformanceGate {
    pub test_name: String,
    pub baseline_performance: Duration,
    pub regression_threshold: f64, // e.g., 0.05 for 5% regression
    pub improvement_threshold: f64, // e.g., 0.10 for 10% improvement
}

impl PerformanceGate {
    pub fn check_regression(&self, current_performance: Duration) -> GateResult {
        let ratio = current_performance.as_nanos() as f64 / 
                   self.baseline_performance.as_nanos() as f64;
        
        if ratio > (1.0 + self.regression_threshold) {
            GateResult::Regression(ratio)
        } else if ratio < (1.0 - self.improvement_threshold) {
            GateResult::Improvement(ratio)
        } else {
            GateResult::Stable(ratio)
        }
    }
}
```

## 📈 Measurement Methodology

### Statistical Requirements

#### **Sample Size**
- **Minimum**: 100 iterations per test case
- **Target**: 1000 iterations for statistical significance
- **Confidence Level**: 95%
- **Significance Level**: 5%

#### **Measurement Process**
1. **Warm-up Phase**: 10 iterations (discarded)
2. **Measurement Phase**: 100-1000 iterations (recorded)
3. **Cooldown Phase**: 5 iterations (discarded)

#### **Environment Consistency**
- **Hardware**: Same machine for all comparisons
- **Load**: Minimal system load during testing
- **Temperature**: Consistent thermal conditions
- **Memory**: Sufficient available memory

### Data Collection

#### **Metrics Collected**
- **Execution Time**: Wall-clock time for parsing
- **Memory Usage**: Peak memory consumption
- **CPU Usage**: CPU time and utilization
- **Cache Performance**: Cache hit/miss ratios (if available)

#### **Result Format**
```json
{
  "test_name": "simple_variable",
  "implementation": "rust",
  "timestamp": "2024-01-01T00:00:00Z",
  "environment": {
    "cpu": "Intel i7-10700K",
    "memory": "32GB",
    "os": "Linux 5.15.0",
    "rust_version": "1.75.0"
  },
  "results": {
    "iterations": 1000,
    "mean_duration_ns": 12345,
    "std_dev_ns": 123,
    "confidence_interval": [12200, 12490],
    "memory_usage_mb": 2.1
  }
}
```

## 🚦 Performance Gates

### Gate Configuration

#### **Regression Thresholds**
- **Critical**: 10% performance regression (fails CI)
- **Warning**: 5% performance regression (warns in CI)
- **Improvement**: 10% performance improvement (celebrates)

#### **Gate Implementation**
```yaml
# .github/workflows/performance-gates.yml
name: Performance Gates

on: [push, pull_request]

jobs:
  performance-gates:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Run benchmarks
        run: cargo xtask bench --save
      - name: Check performance gates
        run: cargo xtask bench --check-gates
      - name: Generate report
        run: cargo xtask bench --report
```

### Gate Results

#### **Pass/Fail Criteria**
- **Pass**: All tests within regression thresholds
- **Fail**: Any test exceeds critical regression threshold
- **Warning**: Any test exceeds warning threshold but not critical

#### **Action Items**
- **Pass**: Continue with deployment
- **Warning**: Notify team, investigate
- **Fail**: Block deployment, require investigation

## 📊 Result Analysis

### Statistical Analysis

#### **Confidence Intervals**
- **95% Confidence**: Standard statistical confidence level
- **Margin of Error**: Calculated based on sample size and variance
- **Interpretation**: True performance lies within interval with 95% confidence

#### **Significance Testing**
- **Null Hypothesis**: No performance difference between implementations
- **Alternative Hypothesis**: Rust implementation is faster/slower
- **P-value**: Probability of observing results if null hypothesis is true
- **Significance**: P-value < 0.05 indicates statistical significance

### Visualization

#### **Performance Trends**
- Time-series plots of performance over commits
- Regression detection visualization
- Improvement tracking

#### **Comparison Charts**
- Side-by-side C vs Rust performance
- Confidence interval visualization
- Statistical significance indicators

## 🔄 Continuous Integration

### CI Pipeline Integration

#### **Automated Benchmarking**
1. **Trigger**: On every push and pull request
2. **Execution**: Run full benchmark suite
3. **Analysis**: Statistical comparison and regression detection
4. **Reporting**: Generate performance report
5. **Gating**: Apply performance gates

#### **Performance Monitoring**
- **Historical Tracking**: Store results over time
- **Trend Analysis**: Identify performance trends
- **Alert System**: Notify on regressions
- **Dashboard**: Visual performance monitoring

### Result Storage

#### **Data Retention**
- **Raw Results**: Store all benchmark data
- **Aggregated Results**: Store statistical summaries
- **Historical Data**: Maintain performance history
- **Metadata**: Store environment and configuration data

#### **Access Patterns**
- **CI/CD**: Automated access for gates
- **Development**: Manual access for investigation
- **Reporting**: Automated report generation
- **Analysis**: Data export for external analysis

## 🎯 Success Metrics

### Quantitative Metrics
- **Statistical Validity**: 95% confidence intervals for all comparisons
- **Automation**: 100% automated benchmarking pipeline
- **Coverage**: All critical performance aspects measured
- **Reliability**: <1% false positive/negative rate for regressions

### Qualitative Metrics
- **Transparency**: Clear methodology and result interpretation
- **Reproducibility**: Results can be reproduced by others
- **Actionability**: Clear action items for performance issues
- **Integration**: Seamless integration with development workflow

## 📋 Implementation Checklist

### Phase 1: Rust Benchmarks ✅
- [x] Criterion benchmark setup
- [x] Basic performance measurement
- [x] Xtask integration
- [x] Result storage

### Phase 2: C Benchmarks 🔄
- [ ] Node.js benchmark setup
- [ ] Test case implementation
- [ ] Fair comparison methodology
- [ ] Statistical analysis framework

### Phase 3: Comparison Engine 🔄
- [ ] Statistical comparison implementation
- [ ] Regression detection algorithms
- [ ] Performance gates
- [ ] CI/CD integration

### Phase 4: Advanced Features 🔄
- [ ] Memory usage measurement
- [ ] Scalability analysis
- [ ] Historical tracking
- [ ] Performance dashboard

---

**Status**: Design complete, Phase 1 implemented, Phase 2-4 planned and documented. 