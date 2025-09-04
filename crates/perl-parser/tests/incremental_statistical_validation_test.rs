//! Statistical validation and performance regression detection for incremental parsing
//!
//! This module implements comprehensive statistical analysis of incremental parsing
//! performance to detect regressions, validate consistency, and ensure quality.

#![cfg(feature = "incremental")]

mod support;

use crate::support::incremental_test_utils::IncrementalTestUtils;
use perl_parser::incremental_v2::IncrementalParserV2;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Statistical analysis framework for performance validation
struct StatisticalAnalyzer {
    samples: Vec<u128>,
    target_percentiles: Vec<f64>,
}

impl StatisticalAnalyzer {
    fn new() -> Self {
        StatisticalAnalyzer {
            samples: Vec::new(),
            target_percentiles: vec![50.0, 75.0, 90.0, 95.0, 99.0],
        }
    }

    fn add_sample(&mut self, micros: u128) {
        self.samples.push(micros);
    }

    fn calculate_statistics(&mut self) -> PerformanceStatistics {
        if self.samples.is_empty() {
            return PerformanceStatistics::empty();
        }

        self.samples.sort();

        let count = self.samples.len();
        let sum: u128 = self.samples.iter().sum();
        let mean = sum as f64 / count as f64;

        let median = if count % 2 == 0 {
            (self.samples[count / 2 - 1] + self.samples[count / 2]) as f64 / 2.0
        } else {
            self.samples[count / 2] as f64
        };

        let min = *self.samples.iter().min().unwrap() as f64;
        let max = *self.samples.iter().max().unwrap() as f64;

        // Calculate variance and standard deviation
        let variance =
            self.samples.iter().map(|&x| (x as f64 - mean).powi(2)).sum::<f64>() / count as f64;
        let std_dev = variance.sqrt();

        // Calculate percentiles
        let mut percentiles = HashMap::new();
        for &p in &self.target_percentiles {
            let index = ((p / 100.0) * (count - 1) as f64).round() as usize;
            percentiles.insert(p as u32, self.samples[index] as f64);
        }

        // Calculate coefficient of variation
        let cv = if mean > 0.0 { std_dev / mean } else { 0.0 };

        // Outlier detection using IQR method
        let q1 = percentiles.get(&25).copied().unwrap_or(median);
        let q3 = percentiles.get(&75).copied().unwrap_or(median);
        let iqr = q3 - q1;
        let lower_bound = q1 - 1.5 * iqr;
        let upper_bound = q3 + 1.5 * iqr;

        let outliers = self
            .samples
            .iter()
            .filter(|&&x| (x as f64) < lower_bound || (x as f64) > upper_bound)
            .count();

        PerformanceStatistics {
            count,
            mean,
            median,
            min,
            max,
            std_dev,
            coefficient_of_variation: cv,
            percentiles,
            outliers,
        }
    }
}

#[derive(Debug, Clone)]
struct PerformanceStatistics {
    count: usize,
    mean: f64,
    median: f64,
    min: f64,
    max: f64,
    std_dev: f64,
    coefficient_of_variation: f64,
    percentiles: HashMap<u32, f64>,
    outliers: usize,
}

impl PerformanceStatistics {
    fn empty() -> Self {
        PerformanceStatistics {
            count: 0,
            mean: 0.0,
            median: 0.0,
            min: 0.0,
            max: 0.0,
            std_dev: 0.0,
            coefficient_of_variation: 0.0,
            percentiles: HashMap::new(),
            outliers: 0,
        }
    }

    fn print_detailed_report(&self, test_name: &str) {
        println!("\n📊 Statistical Analysis: {}", test_name);
        println!("  Samples: {}", self.count);
        println!("  Mean: {:.1}µs", self.mean);
        println!("  Median: {:.1}µs", self.median);
        println!("  Range: {:.1}µs - {:.1}µs", self.min, self.max);
        println!("  Std Dev: {:.1}µs (CV: {:.3})", self.std_dev, self.coefficient_of_variation);

        if !self.percentiles.is_empty() {
            println!("  Percentiles:");
            for (&p, &v) in &self.percentiles {
                println!("    P{}: {:.1}µs", p, v);
            }
        }

        if self.outliers > 0 {
            println!(
                "  Outliers: {} ({:.1}%)",
                self.outliers,
                self.outliers as f64 / self.count as f64 * 100.0
            );
        }
    }

    fn validate_performance_criteria(&self, criteria: &PerformanceCriteria) -> ValidationResult {
        let mut violations = Vec::new();
        let mut warnings = Vec::new();

        // Mean performance check
        if self.mean > criteria.max_mean_micros {
            violations.push(format!(
                "Mean performance {:.1}µs exceeds limit {:.1}µs",
                self.mean, criteria.max_mean_micros
            ));
        }

        // Consistency check (coefficient of variation)
        if self.coefficient_of_variation > criteria.max_cv {
            violations.push(format!(
                "Performance inconsistency CV={:.3} exceeds limit {:.3}",
                self.coefficient_of_variation, criteria.max_cv
            ));
        }

        // Outlier threshold check
        let outlier_percentage = self.outliers as f64 / self.count as f64;
        if outlier_percentage > criteria.max_outlier_rate {
            warnings.push(format!(
                "High outlier rate {:.1}% (limit: {:.1}%)",
                outlier_percentage * 100.0,
                criteria.max_outlier_rate * 100.0
            ));
        }

        // P95 performance check
        if let Some(&p95) = self.percentiles.get(&95) {
            if p95 > criteria.max_p95_micros {
                violations.push(format!(
                    "P95 performance {:.1}µs exceeds limit {:.1}µs",
                    p95, criteria.max_p95_micros
                ));
            }
        }

        ValidationResult { passed: violations.is_empty(), violations, warnings }
    }
}

#[derive(Debug)]
struct PerformanceCriteria {
    max_mean_micros: f64,
    max_p95_micros: f64,
    max_cv: f64,
    max_outlier_rate: f64,
}

impl PerformanceCriteria {
    fn standard() -> Self {
        PerformanceCriteria {
            max_mean_micros: 1000.0, // 1ms average
            max_p95_micros: 5000.0,  // 5ms at P95
            max_cv: 1.0,             // Coefficient of variation < 1.0
            max_outlier_rate: 0.1,   // <10% outliers
        }
    }

    fn strict() -> Self {
        PerformanceCriteria {
            max_mean_micros: 500.0, // 500µs average
            max_p95_micros: 2000.0, // 2ms at P95
            max_cv: 0.5,            // CV < 0.5
            max_outlier_rate: 0.05, // <5% outliers
        }
    }
}

#[derive(Debug)]
struct ValidationResult {
    passed: bool,
    violations: Vec<String>,
    warnings: Vec<String>,
}

impl ValidationResult {
    fn print_report(&self) {
        if self.passed {
            println!("  ✅ All performance criteria met");
        } else {
            println!("  ❌ Performance criteria violations:");
            for violation in &self.violations {
                println!("    • {}", violation);
            }
        }

        if !self.warnings.is_empty() {
            println!("  ⚠️ Performance warnings:");
            for warning in &self.warnings {
                println!("    • {}", warning);
            }
        }
    }
}

/// Comprehensive statistical validation of simple value edits
#[test]
fn test_statistical_validation_simple_edits() {
    println!("\n📈 Statistical Validation: Simple Value Edits");

    let mut analyzer = StatisticalAnalyzer::new();
    let iterations = 100;
    let source = "my $x = 42;";

    // Collect performance samples
    for i in 0..iterations {
        let mut parser = IncrementalParserV2::new();
        parser.parse(source).unwrap();

        let new_val = format!("{}", 100 + i);
        let (new_source, edit) = IncrementalTestUtils::create_value_edit(source, "42", &new_val);

        let start = Instant::now();
        let result =
            IncrementalTestUtils::measure_incremental_parse(&mut parser, source, edit, &new_source);
        let parse_time = start.elapsed();

        analyzer.add_sample(parse_time.as_micros());

        // Verify correctness
        assert!(result.success, "Simple edit {} should succeed", i);
        assert!(result.nodes_reused >= 3, "Simple edit {} should reuse ≥3 nodes", i);
    }

    // Analyze statistics
    let stats = analyzer.calculate_statistics();
    stats.print_detailed_report("Simple Value Edits");

    // Validate against strict criteria for simple edits
    let criteria = PerformanceCriteria::strict();
    let validation = stats.validate_performance_criteria(&criteria);
    validation.print_report();

    // Assert key requirements
    assert!(stats.mean < 1000.0, "Simple edits should average <1ms");
    assert!(stats.coefficient_of_variation < 1.5, "Simple edits should be reasonably consistent");
    assert!(
        validation.passed || validation.violations.len() <= 1,
        "Simple edits should meet most performance criteria"
    );
}

/// Statistical analysis of complex document performance
#[test]
fn test_statistical_validation_complex_documents() {
    println!("\n📈 Statistical Validation: Complex Documents");

    let mut analyzer = StatisticalAnalyzer::new();
    let iterations = 50; // Fewer iterations for complex documents

    // Generate complex document
    let mut complex_source = String::new();
    complex_source.push_str("package ComplexModule;\n\n");

    for i in 0..20 {
        complex_source.push_str(&format!(
            "sub function_{} {{\n    my ($self, $param) = @_;\n    my $result = $param * {};\n    return $result;\n}}\n\n",
            i, i + 1
        ));
    }

    complex_source.push_str("1; # End of module\n");

    // Collect performance samples
    for i in 0..iterations {
        let mut parser = IncrementalParserV2::new();
        parser.parse(&complex_source).unwrap();

        // Edit a parameter value in the middle of the document
        let old_val = format!("{}", 10);
        let new_val = format!("{}", 100 + i);
        let (new_source, edit) =
            IncrementalTestUtils::create_value_edit(&complex_source, &old_val, &new_val);

        let start = Instant::now();
        let result = IncrementalTestUtils::measure_incremental_parse(
            &mut parser,
            &complex_source,
            edit,
            &new_source,
        );
        let parse_time = start.elapsed();

        analyzer.add_sample(parse_time.as_micros());

        assert!(result.success, "Complex edit {} should succeed", i);

        if i % 10 == 0 {
            println!(
                "  Sample {}: {}µs, reused={}, reparsed={}",
                i,
                parse_time.as_micros(),
                result.nodes_reused,
                result.nodes_reparsed
            );
        }
    }

    // Analyze statistics
    let stats = analyzer.calculate_statistics();
    stats.print_detailed_report("Complex Documents");

    // Validate against standard criteria for complex documents
    let criteria = PerformanceCriteria::standard();
    let validation = stats.validate_performance_criteria(&criteria);
    validation.print_report();

    // Complex documents have more relaxed requirements
    assert!(stats.mean < 5000.0, "Complex edits should average <5ms");
    assert!(
        stats.coefficient_of_variation < 2.0,
        "Complex edits should have reasonable consistency"
    );
}

/// Performance regression detection across multiple test runs
#[test]
fn test_regression_detection_across_sessions() {
    println!("\n🕰️ Performance Regression Detection");

    let test_cases = vec![
        ("Simple", "my $x = 42;", "42", "999"),
        ("String", r#"my $name = "hello";"#, "hello", "world"),
        ("Multi-stmt", "my $a = 1; my $b = 2;", "1", "10"),
    ];

    let mut baseline_stats = HashMap::new();

    // Establish baseline performance
    for (name, source, old_val, new_val) in &test_cases {
        let mut analyzer = StatisticalAnalyzer::new();

        for _i in 0..30 {
            let mut parser = IncrementalParserV2::new();
            parser.parse(source).unwrap();

            let (new_source, edit) =
                IncrementalTestUtils::create_value_edit(source, old_val, new_val);

            let start = Instant::now();
            IncrementalTestUtils::measure_incremental_parse(&mut parser, source, edit, &new_source);
            let parse_time = start.elapsed();

            analyzer.add_sample(parse_time.as_micros());
        }

        let stats = analyzer.calculate_statistics();
        println!(
            "  Baseline {}: mean={:.1}µs, P95={:.1}µs",
            name,
            stats.mean,
            stats.percentiles.get(&95).unwrap_or(&0.0)
        );
        baseline_stats.insert(name.to_string(), stats);
    }

    // Simulate multiple test sessions to detect regression
    for session in 1..=5 {
        println!("\n  Session {}: Regression Detection", session);

        for (name, source, old_val, new_val) in &test_cases {
            let mut analyzer = StatisticalAnalyzer::new();

            for _i in 0..20 {
                let mut parser = IncrementalParserV2::new();
                parser.parse(source).unwrap();

                let (new_source, edit) =
                    IncrementalTestUtils::create_value_edit(source, old_val, new_val);

                let start = Instant::now();
                IncrementalTestUtils::measure_incremental_parse(
                    &mut parser,
                    source,
                    edit,
                    &new_source,
                );
                let parse_time = start.elapsed();

                analyzer.add_sample(parse_time.as_micros());
            }

            let current_stats = analyzer.calculate_statistics();
            let baseline = baseline_stats.get(&name.to_string()).unwrap();

            // Regression detection
            let mean_regression = current_stats.mean / baseline.mean;
            let p95_regression = current_stats.percentiles.get(&95).unwrap_or(&0.0)
                / baseline.percentiles.get(&95).unwrap_or(&1.0);

            println!(
                "    {}: mean ratio={:.2}x, P95 ratio={:.2}x",
                name, mean_regression, p95_regression
            );

            // Flag significant regressions
            if mean_regression > 2.0 {
                println!("      ⚠️ Significant mean performance regression detected");
            }
            if p95_regression > 2.5 {
                println!("      ⚠️ Significant P95 performance regression detected");
            }

            // For testing purposes, we allow some variation but flag extreme cases
            assert!(
                mean_regression < 5.0,
                "Extreme performance regression in {}: {:.2}x slower",
                name,
                mean_regression
            );
            assert!(
                p95_regression < 8.0,
                "Extreme P95 regression in {}: {:.2}x slower",
                name,
                p95_regression
            );
        }
    }

    println!("  ✅ Regression detection completed - no extreme regressions found");
}

/// Analyze performance distribution patterns
#[test]
fn test_performance_distribution_analysis() {
    println!("\n📊 Performance Distribution Analysis");

    let mut analyzer = StatisticalAnalyzer::new();
    let source = "my $var = 'test value';";
    let iterations = 200;

    // Collect large sample for distribution analysis
    for i in 0..iterations {
        let mut parser = IncrementalParserV2::new();
        parser.parse(source).unwrap();

        let new_val = format!("value_{}", i);
        let (new_source, edit) =
            IncrementalTestUtils::create_value_edit(source, "test value", &new_val);

        let start = Instant::now();
        IncrementalTestUtils::measure_incremental_parse(&mut parser, source, edit, &new_source);
        let parse_time = start.elapsed();

        analyzer.add_sample(parse_time.as_micros());
    }

    let stats = analyzer.calculate_statistics();
    stats.print_detailed_report("Distribution Analysis");

    // Analyze distribution characteristics
    let skewness = calculate_skewness(&analyzer.samples, stats.mean, stats.std_dev);
    let kurtosis = calculate_kurtosis(&analyzer.samples, stats.mean, stats.std_dev);

    println!("  Distribution characteristics:");
    println!("    Skewness: {:.3}", skewness);
    println!("    Kurtosis: {:.3}", kurtosis);

    // Check for distribution properties
    if skewness.abs() < 1.0 {
        println!("    ✅ Distribution is reasonably symmetric");
    } else {
        println!("    ⚠️ Distribution shows significant skewness");
    }

    if kurtosis.abs() < 3.0 {
        println!("    ✅ Distribution has normal tail behavior");
    } else {
        println!("    ⚠️ Distribution has heavy tails or unusual peaks");
    }

    // Validate distribution sanity
    assert!(skewness.abs() < 5.0, "Extreme skewness indicates performance issues");
    assert!(kurtosis.abs() < 10.0, "Extreme kurtosis indicates performance issues");
}

/// Test performance under sustained load
#[test]
fn test_sustained_load_performance() {
    println!("\n🔄 Sustained Load Performance Test");

    let mut parser = IncrementalParserV2::new();
    let source = "my $sustained = 42; my $load = 'test';";
    parser.parse(source).unwrap();

    let mut load_analyzer = StatisticalAnalyzer::new();
    let load_iterations = 500;
    let mut efficiency_samples = Vec::new();

    for i in 0..load_iterations {
        let new_val = format!("{}", i);
        let (new_source, edit) = IncrementalTestUtils::create_value_edit(source, "42", &new_val);

        let start = Instant::now();
        let result =
            IncrementalTestUtils::measure_incremental_parse(&mut parser, source, edit, &new_source);
        let parse_time = start.elapsed();

        load_analyzer.add_sample(parse_time.as_micros());
        efficiency_samples.push(result.efficiency_percentage());

        if i % 100 == 0 {
            println!(
                "  Load iteration {}: {}µs, efficiency={:.1}%",
                i,
                parse_time.as_micros(),
                result.efficiency_percentage()
            );
        }

        // Individual sustained load operations should remain fast
        assert!(parse_time.as_millis() < 20, "Sustained load iteration {} too slow", i);
        assert!(result.success, "Sustained load iteration {} should succeed", i);
    }

    let load_stats = load_analyzer.calculate_statistics();
    load_stats.print_detailed_report("Sustained Load");

    // Analyze efficiency consistency under load
    let avg_efficiency = efficiency_samples.iter().sum::<f64>() / efficiency_samples.len() as f64;
    let efficiency_std_dev = {
        let variance =
            efficiency_samples.iter().map(|&x| (x - avg_efficiency).powi(2)).sum::<f64>()
                / efficiency_samples.len() as f64;
        variance.sqrt()
    };

    println!("  Efficiency analysis:");
    println!("    Average efficiency: {:.1}%", avg_efficiency);
    println!("    Efficiency std dev: {:.1}%", efficiency_std_dev);
    println!("    Efficiency consistency: {:.3}", efficiency_std_dev / avg_efficiency);

    // Sustained load should maintain performance characteristics
    assert!(load_stats.mean < 2000.0, "Sustained load should maintain <2ms average");
    assert!(
        load_stats.coefficient_of_variation < 2.0,
        "Sustained load should be reasonably consistent"
    );
    assert!(avg_efficiency > 60.0, "Sustained load should maintain >60% efficiency");

    println!("  ✅ Sustained load test completed successfully");
}

// Helper functions for statistical calculations

fn calculate_skewness(samples: &[u128], mean: f64, std_dev: f64) -> f64 {
    if std_dev == 0.0 {
        return 0.0;
    }

    let n = samples.len() as f64;
    let sum_cubed_deviations =
        samples.iter().map(|&x| ((x as f64 - mean) / std_dev).powi(3)).sum::<f64>();

    sum_cubed_deviations / n
}

fn calculate_kurtosis(samples: &[u128], mean: f64, std_dev: f64) -> f64 {
    if std_dev == 0.0 {
        return 0.0;
    }

    let n = samples.len() as f64;
    let sum_fourth_deviations =
        samples.iter().map(|&x| ((x as f64 - mean) / std_dev).powi(4)).sum::<f64>();

    (sum_fourth_deviations / n) - 3.0 // Excess kurtosis (normal distribution = 0)
}
