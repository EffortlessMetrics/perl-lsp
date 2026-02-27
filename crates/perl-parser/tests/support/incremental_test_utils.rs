//! Test utilities for incremental parsing performance measurement and validation
//!
//! Provides helper functions and structures for comprehensive testing of
//! incremental parsing functionality with detailed performance metrics.

use perl_parser::incremental_v2::{IncrementalMetrics, IncrementalParserV2};
use perl_parser::{edit::Edit, position::Position};
use std::time::{Duration, Instant};

/// Comprehensive performance measurement utilities for incremental parsing
///
/// Provides static methods for testing incremental parsing performance
/// with comprehensive statistical analysis and validation.
pub struct IncrementalTestUtils;

impl IncrementalTestUtils {
    /// Create a performance edit that changes a value in the source
    #[allow(dead_code)]
    pub fn create_value_edit(source: &str, old_value: &str, new_value: &str) -> (String, Edit) {
        use perl_tdd_support::must_some;
        let pos = must_some(source.find(old_value));
        let end_pos = pos + old_value.len();
        let new_end = pos + new_value.len();

        let new_source = source.replace(old_value, new_value);

        let edit = Edit::new(
            pos,
            end_pos,
            new_end,
            Position::new(pos, 1, (pos + 1) as u32),
            Position::new(end_pos, 1, (end_pos + 1) as u32),
            Position::new(new_end, 1, (new_end + 1) as u32),
        );

        (new_source, edit)
    }

    /// Measure the performance of a single incremental parse operation
    #[allow(dead_code)]
    pub fn measure_incremental_parse(
        parser: &mut IncrementalParserV2,
        _source: &str,
        edit: Edit,
        new_source: &str,
    ) -> IncrementalParseResult {
        // Apply edit
        parser.edit(edit);

        // Time the incremental parse
        let start = Instant::now();
        let result = parser.parse(new_source);
        let parse_time = start.elapsed();

        IncrementalParseResult {
            parse_time,
            success: result.is_ok(),
            nodes_reused: parser.reused_nodes,
            nodes_reparsed: parser.reparsed_nodes,
            metrics: parser.get_metrics().clone(),
            error: result.err(),
        }
    }

    /// Run a performance test with multiple iterations and statistical analysis
    #[allow(dead_code)]
    pub fn performance_test_with_stats<F>(
        name: &str,
        initial_source: &str,
        edit_generator: F,
        iterations: usize,
    ) -> PerformanceTestResult
    where
        F: Fn(&str) -> (String, Edit),
    {
        let mut results = Vec::new();
        let mut initial_times = Vec::new();

        println!("\n🧪 Running performance test: {}", name);
        println!("Iterations: {}", iterations);

        for i in 0..iterations {
            let mut parser = IncrementalParserV2::new();

            // Initial parse timing
            let start = Instant::now();
            must(parser.parse(initial_source));
            let initial_time = start.elapsed();
            initial_times.push(initial_time);

            // Generate edit and measure incremental parse
            let (new_source, edit) = edit_generator(initial_source);
            let result =
                Self::measure_incremental_parse(&mut parser, initial_source, edit, &new_source);

            println!(
                "  Run {:>2}: init={:>5}µs, incr={:>5}µs, reused={:>2}, reparsed={:>2}, eff={:>5.1}%",
                i + 1,
                initial_time.as_micros(),
                result.parse_time.as_micros(),
                result.nodes_reused,
                result.nodes_reparsed,
                result.efficiency_percentage()
            );

            results.push(result);
        }

        Self::analyze_performance_results(name, results, initial_times)
    }

    /// Analyze performance results and generate comprehensive report
    fn analyze_performance_results(
        name: &str,
        results: Vec<IncrementalParseResult>,
        initial_times: Vec<Duration>,
    ) -> PerformanceTestResult {
        let incremental_times: Vec<u128> =
            results.iter().map(|r| r.parse_time.as_micros()).collect();
        let initial_times_micros: Vec<u128> = initial_times.iter().map(|t| t.as_micros()).collect();

        // Statistical calculations
        let avg_incremental =
            incremental_times.iter().sum::<u128>() / incremental_times.len() as u128;
        let avg_initial =
            initial_times_micros.iter().sum::<u128>() / initial_times_micros.len() as u128;

        let min_incremental = must_some(incremental_times.iter().min().cloned());
        let max_incremental = must_some(incremental_times.iter().max().cloned());

        let median_incremental = {
            let mut sorted = incremental_times.clone();
            sorted.sort();
            if sorted.len().is_multiple_of(2) {
                (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) / 2
            } else {
                sorted[sorted.len() / 2]
            }
        };

        // Node reuse statistics
        let total_reused: usize = results.iter().map(|r| r.nodes_reused).sum();
        let total_reparsed: usize = results.iter().map(|r| r.nodes_reparsed).sum();
        let avg_efficiency = if total_reused + total_reparsed > 0 {
            total_reused as f64 / (total_reused + total_reparsed) as f64 * 100.0
        } else {
            0.0
        };

        // Performance metrics
        let speedup_ratio = avg_initial as f64 / avg_incremental as f64;
        let sub_ms_count = incremental_times.iter().filter(|&&t| t < 1000).count();
        let sub_ms_rate = sub_ms_count as f64 / incremental_times.len() as f64;

        // Consistency analysis (coefficient of variation)
        let variance = incremental_times
            .iter()
            .map(|&t| (t as f64 - avg_incremental as f64).powi(2))
            .sum::<f64>()
            / incremental_times.len() as f64;
        let std_dev = variance.sqrt();
        let coefficient_of_variation = std_dev / avg_incremental as f64;

        PerformanceTestResult {
            test_name: name.to_string(),
            iterations: results.len(),
            avg_incremental_micros: avg_incremental,
            median_incremental_micros: median_incremental,
            min_incremental_micros: min_incremental,
            max_incremental_micros: max_incremental,
            avg_initial_micros: avg_initial,
            speedup_ratio,
            sub_millisecond_rate: sub_ms_rate,
            avg_efficiency_percentage: avg_efficiency,
            coefficient_of_variation,
            std_deviation_micros: std_dev as u128,
            success_rate: results.iter().filter(|r| r.success).count() as f64
                / results.len() as f64,
            individual_results: results,
        }
    }

    /// Validate performance against specific criteria
    #[allow(dead_code)]
    pub fn validate_performance_criteria(
        result: &PerformanceTestResult,
        criteria: &PerformanceCriteria,
    ) -> ValidationReport {
        let mut violations = Vec::new();
        let mut warnings = Vec::new();

        // Sub-millisecond requirement
        if result.avg_incremental_micros >= criteria.max_avg_micros {
            violations.push(format!(
                "Average parse time {}µs exceeds maximum {}µs",
                result.avg_incremental_micros, criteria.max_avg_micros
            ));
        }

        // Efficiency requirement
        if result.avg_efficiency_percentage < criteria.min_efficiency_percentage {
            violations.push(format!(
                "Node reuse efficiency {:.1}% below minimum {:.1}%",
                result.avg_efficiency_percentage, criteria.min_efficiency_percentage
            ));
        }

        // Speedup requirement
        if result.speedup_ratio < criteria.min_speedup_ratio {
            violations.push(format!(
                "Speedup ratio {:.1}x below minimum {:.1}x",
                result.speedup_ratio, criteria.min_speedup_ratio
            ));
        }

        // Consistency requirement
        if result.coefficient_of_variation > criteria.max_coefficient_of_variation {
            warnings.push(format!(
                "Performance variation {:.2} exceeds recommended {:.2}",
                result.coefficient_of_variation, criteria.max_coefficient_of_variation
            ));
        }

        // Success rate requirement
        if result.success_rate < criteria.min_success_rate {
            violations.push(format!(
                "Success rate {:.1}% below minimum {:.1}%",
                result.success_rate * 100.0,
                criteria.min_success_rate * 100.0
            ));
        }

        // Sub-millisecond rate warning
        if result.sub_millisecond_rate < 0.8 {
            warnings.push(format!(
                "Only {:.1}% of runs were sub-millisecond",
                result.sub_millisecond_rate * 100.0
            ));
        }

        ValidationReport { passed: violations.is_empty(), violations, warnings }
    }

    /// Generate detailed performance report
    #[allow(dead_code)]
    pub fn print_performance_summary(result: &PerformanceTestResult) {
        println!("\n📊 Performance Test Summary: {}", result.test_name);
        println!("═════════════════════════════════════════════");
        println!("📈 Timing Statistics:");
        println!("  Average Incremental: {}µs", result.avg_incremental_micros);
        println!("  Median Incremental:  {}µs", result.median_incremental_micros);
        println!(
            "  Range: {}µs - {}µs",
            result.min_incremental_micros, result.max_incremental_micros
        );
        println!("  Standard Deviation: {}µs", result.std_deviation_micros);
        println!("  Coefficient of Variation: {:.3}", result.coefficient_of_variation);

        println!("\n⚡ Performance Metrics:");
        println!("  Speedup Ratio: {:.2}x faster", result.speedup_ratio);
        println!("  Sub-millisecond Rate: {:.1}%", result.sub_millisecond_rate * 100.0);
        println!("  Success Rate: {:.1}%", result.success_rate * 100.0);

        println!("\n🔄 Node Reuse Statistics:");
        println!("  Average Efficiency: {:.1}%", result.avg_efficiency_percentage);

        let category = match result.avg_incremental_micros {
            0..=100 => "🟢 Excellent (<100µs)",
            101..=500 => "🟡 Very Good (<500µs)",
            501..=1000 => "🟠 Good (<1ms)",
            1001..=5000 => "🔴 Acceptable (<5ms)",
            _ => "🚨 Needs Optimization (>5ms)",
        };
        println!("  Performance Category: {}", category);
        println!("═════════════════════════════════════════════");
    }

    /// Create standard performance criteria for incremental parsing
    #[allow(dead_code)]
    pub fn standard_criteria() -> PerformanceCriteria {
        PerformanceCriteria {
            max_avg_micros: 1000,              // <1ms average
            min_efficiency_percentage: 70.0,   // ≥70% node reuse
            min_speedup_ratio: 2.0,            // ≥2x faster than full parse
            max_coefficient_of_variation: 0.5, // Consistent performance
            min_success_rate: 0.95,            // ≥95% successful parses
        }
    }

    /// Create relaxed criteria for complex scenarios
    #[allow(dead_code)]
    pub fn relaxed_criteria() -> PerformanceCriteria {
        PerformanceCriteria {
            max_avg_micros: 5000,              // <5ms average
            min_efficiency_percentage: 50.0,   // ≥50% node reuse
            min_speedup_ratio: 1.5,            // ≥1.5x faster than full parse
            max_coefficient_of_variation: 1.0, // Allow more variation
            min_success_rate: 0.90,            // ≥90% successful parses
        }
    }
}

/// Results of a single incremental parse operation
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct IncrementalParseResult {
    pub parse_time: Duration,
    pub success: bool,
    pub nodes_reused: usize,
    pub nodes_reparsed: usize,
    pub metrics: IncrementalMetrics,
    pub error: Option<perl_parser::error::ParseError>,
}

impl IncrementalParseResult {
    pub fn efficiency_percentage(&self) -> f64 {
        if self.nodes_reused + self.nodes_reparsed == 0 {
            return 0.0;
        }
        self.nodes_reused as f64 / (self.nodes_reused + self.nodes_reparsed) as f64 * 100.0
    }
}

/// Comprehensive performance test results with statistical analysis
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PerformanceTestResult {
    pub test_name: String,
    pub iterations: usize,
    pub avg_incremental_micros: u128,
    pub median_incremental_micros: u128,
    pub min_incremental_micros: u128,
    pub max_incremental_micros: u128,
    pub avg_initial_micros: u128,
    pub speedup_ratio: f64,
    pub sub_millisecond_rate: f64,
    pub avg_efficiency_percentage: f64,
    pub coefficient_of_variation: f64,
    pub std_deviation_micros: u128,
    pub success_rate: f64,
    pub individual_results: Vec<IncrementalParseResult>,
}

/// Performance criteria for validation
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PerformanceCriteria {
    pub max_avg_micros: u128,
    pub min_efficiency_percentage: f64,
    pub min_speedup_ratio: f64,
    pub max_coefficient_of_variation: f64,
    pub min_success_rate: f64,
}

/// Validation report for performance criteria
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ValidationReport {
    pub passed: bool,
    pub violations: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationReport {
    #[allow(dead_code)]
    pub fn print_report(&self) {
        if self.passed {
            println!("✅ Performance validation PASSED");
        } else {
            println!("❌ Performance validation FAILED");
            for violation in &self.violations {
                println!("  🚫 {}", violation);
            }
        }

        if !self.warnings.is_empty() {
            println!("⚠️  Performance warnings:");
            for warning in &self.warnings {
                println!("  ⚠️  {}", warning);
            }
        }
    }
}

/// Macro for easy performance testing
#[macro_export]
macro_rules! perf_test {
    ($name:expr, $source:expr, $edit_fn:expr) => {
        perf_test!($name, $source, $edit_fn, 10)
    };
    ($name:expr, $source:expr, $edit_fn:expr, $iterations:expr) => {{
        use $crate::support::incremental_test_utils::IncrementalTestUtils;

        let result = IncrementalTestUtils::performance_test_with_stats(
            $name,
            $source,
            $edit_fn,
            $iterations,
        );

        IncrementalTestUtils::print_performance_summary(&result);

        // Apply standard validation criteria
        let criteria = IncrementalTestUtils::standard_criteria();
        let validation = IncrementalTestUtils::validate_performance_criteria(&result, &criteria);
        validation.print_report();

        assert!(validation.passed, "Performance test '{}' failed validation", $name);
        result
    }};
}

/// Macro for relaxed performance testing (complex scenarios)
#[macro_export]
macro_rules! perf_test_relaxed {
    ($name:expr, $source:expr, $edit_fn:expr) => {
        perf_test_relaxed!($name, $source, $edit_fn, 10)
    };
    ($name:expr, $source:expr, $edit_fn:expr, $iterations:expr) => {{
        use $crate::support::incremental_test_utils::IncrementalTestUtils;

        let result = IncrementalTestUtils::performance_test_with_stats(
            $name,
            $source,
            $edit_fn,
            $iterations,
        );

        IncrementalTestUtils::print_performance_summary(&result);

        // Apply relaxed validation criteria
        let criteria = IncrementalTestUtils::relaxed_criteria();
        let validation = IncrementalTestUtils::validate_performance_criteria(&result, &criteria);
        validation.print_report();

        assert!(validation.passed, "Performance test '{}' failed validation", $name);
        result
    }};
}
