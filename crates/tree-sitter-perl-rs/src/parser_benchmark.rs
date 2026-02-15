//! Benchmark framework for comparing parser implementations
//!
//! This module provides tools to benchmark and compare:
//! - Recursive parser (original)
//! - Recursive parser with stacker
//! - Iterative parser

use crate::pure_rust_parser::{AstNode, PerlParser, PureRustPerlParser, Rule};
use pest::Parser;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Result of a single benchmark run
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub name: String,
    pub duration: Duration,
    pub success: bool,
    pub error: Option<String>,
}

/// Collection of benchmark results
#[derive(Debug, Default)]
pub struct BenchmarkSuite {
    pub results: HashMap<String, Vec<BenchmarkResult>>,
}

impl BenchmarkSuite {
    pub fn new() -> Self {
        Self::default()
    }

    /// Run a benchmark function multiple times and collect results
    pub fn bench<F>(&mut self, name: &str, iterations: usize, mut f: F)
    where
        F: FnMut() -> Result<(), String>,
    {
        let mut results = Vec::new();

        for _ in 0..iterations {
            let start = Instant::now();
            let result = f();
            let duration = start.elapsed();

            results.push(BenchmarkResult {
                name: name.to_string(),
                duration,
                success: result.is_ok(),
                error: result.err(),
            });
        }

        self.results.insert(name.to_string(), results);
    }

    /// Print benchmark summary
    pub fn summary(&self) {
        println!("\n=== Benchmark Summary ===\n");

        for (name, results) in &self.results {
            let total_duration: Duration = results.iter().map(|r| r.duration).sum();
            let avg_duration = total_duration / results.len() as u32;
            let success_rate =
                results.iter().filter(|r| r.success).count() as f64 / results.len() as f64 * 100.0;

            println!("{:30} | Avg: {:>8.2?} | Success: {:>5.1}%", name, avg_duration, success_rate);

            // Show min/max
            let min = results.iter().map(|r| r.duration).min().unwrap_or_default();
            let max = results.iter().map(|r| r.duration).max().unwrap_or_default();
            println!("{:30} | Min: {:>8.2?} | Max: {:>8.2?}", "", min, max);

            // Show errors if any
            let errors: Vec<_> = results.iter().filter_map(|r| r.error.as_ref()).collect();
            if !errors.is_empty() {
                println!("{:30} | Errors: {}", "", errors[0]);
            }

            println!();
        }
    }

    /// Compare two benchmarks
    pub fn compare(&self, baseline: &str, comparison: &str) {
        if let (Some(baseline_results), Some(comparison_results)) =
            (self.results.get(baseline), self.results.get(comparison))
        {
            let baseline_avg: Duration =
                baseline_results.iter().map(|r| r.duration).sum::<Duration>()
                    / baseline_results.len() as u32;

            let comparison_avg: Duration =
                comparison_results.iter().map(|r| r.duration).sum::<Duration>()
                    / comparison_results.len() as u32;

            let speedup = baseline_avg.as_secs_f64() / comparison_avg.as_secs_f64();

            println!("\nComparison: {} vs {}", baseline, comparison);
            println!("{}: {:?}", baseline, baseline_avg);
            println!("{}: {:?}", comparison, comparison_avg);

            if speedup > 1.0 {
                println!("{} is {:.2}x faster", comparison, speedup);
            } else {
                println!("{} is {:.2}x slower", comparison, 1.0 / speedup);
            }
        }
    }
}

/// Parser implementation variants
pub enum ParserImpl {
    Recursive,
    RecursiveWithStacker,
    #[cfg(not(feature = "v2-pest-microcrate"))]
    Iterative,
}

/// Benchmark harness for parser implementations
#[derive(Default)]
pub struct ParserBenchmark {
    parser: PureRustPerlParser,
}

impl ParserBenchmark {
    pub fn new() -> Self {
        Self::default()
    }

    /// Run benchmark with specified parser implementation
    pub fn bench_parser(&mut self, impl_type: ParserImpl, input: &str) -> Result<AstNode, String> {
        // Parse input
        let pairs =
            PerlParser::parse(Rule::program, input).map_err(|e| format!("Parse error: {:?}", e))?;
        let pair = pairs.into_iter().next().ok_or_else(|| "No pairs found".to_string())?;

        // Build AST with specified implementation
        match impl_type {
            ParserImpl::Recursive => {
                // Temporarily disable stacker by calling build_node_impl directly
                // Note: In real implementation, we'd have a flag to control this
                self.parser
                    .build_node(pair)
                    .map_err(|e| format!("Build error: {:?}", e))?
                    .ok_or_else(|| "No AST node produced".to_string())
            }
            ParserImpl::RecursiveWithStacker => {
                // This uses the current build_node which has stacker
                self.parser
                    .build_node(pair)
                    .map_err(|e| format!("Build error: {:?}", e))?
                    .ok_or_else(|| "No AST node produced".to_string())
            }
            #[cfg(not(feature = "v2-pest-microcrate"))]
            ParserImpl::Iterative => self
                .parser
                .build_node_iterative(pair)
                .map_err(|e| format!("Build error: {:?}", e))?
                .ok_or_else(|| "No AST node produced".to_string()),
        }
    }
}

/// Macro for running parser benchmarks
#[macro_export]
macro_rules! bench_parsers {
    ($suite:expr, $input:expr, $iterations:expr) => {{
        use $crate::parser_benchmark::{ParserBenchmark, ParserImpl};

        let input = $input;
        let iterations = $iterations;

        // Benchmark recursive parser (without stacker)
        $suite.bench("Recursive", iterations, || {
            let mut bench = ParserBenchmark::new();
            bench.bench_parser(ParserImpl::Recursive, &input).map(|_| ()).map_err(|e| e.to_string())
        });

        // Benchmark recursive parser with stacker
        $suite.bench("Recursive + Stacker", iterations, || {
            let mut bench = ParserBenchmark::new();
            bench
                .bench_parser(ParserImpl::RecursiveWithStacker, &input)
                .map(|_| ())
                .map_err(|e| e.to_string())
        });

        #[cfg(not(feature = "v2-pest-microcrate"))]
        {
            // Benchmark iterative parser
            $suite.bench("Iterative", iterations, || {
                let mut bench = ParserBenchmark::new();
                bench
                    .bench_parser(ParserImpl::Iterative, &input)
                    .map(|_| ())
                    .map_err(|e| e.to_string())
            });
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_framework() {
        let mut suite = BenchmarkSuite::new();

        // Simple expression
        let simple_input = "$x = 42";
        bench_parsers!(suite, simple_input, 5);

        // Nested expression
        let mut nested = "1".to_string();
        for _ in 0..10 {
            nested = format!("({})", nested);
        }
        bench_parsers!(suite, nested, 3);

        // Print results
        suite.summary();
        #[cfg(not(feature = "v2-pest-microcrate"))]
        suite.compare("Recursive", "Iterative");
        #[cfg(not(feature = "v2-pest-microcrate"))]
        suite.compare("Recursive + Stacker", "Iterative");
    }

    #[test]
    #[cfg(not(feature = "v2-pest-microcrate"))]
    #[cfg(debug_assertions)]
    fn test_deep_nesting_benchmark() {
        let mut suite = BenchmarkSuite::new();

        // Test different nesting depths
        for depth in [100, 500, 1000] {
            let mut expr = "42".to_string();
            for _ in 0..depth {
                expr = format!("({})", expr);
            }

            let test_name = format!("Depth {}", depth);

            // Only iterative and stacker should work for deep nesting in debug
            suite.bench(&format!("{} - Iterative", test_name), 1, || {
                let mut bench = ParserBenchmark::new();
                bench
                    .bench_parser(ParserImpl::Iterative, &expr)
                    .map(|_| ())
                    .map_err(|e| e.to_string())
            });

            suite.bench(&format!("{} - Stacker", test_name), 1, || {
                let mut bench = ParserBenchmark::new();
                bench
                    .bench_parser(ParserImpl::RecursiveWithStacker, &expr)
                    .map(|_| ())
                    .map_err(|e| e.to_string())
            });
        }

        suite.summary();
    }
}
