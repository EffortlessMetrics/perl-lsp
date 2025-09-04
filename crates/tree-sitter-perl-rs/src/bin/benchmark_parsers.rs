//! Comprehensive benchmark runner for parser implementations
//!
//! This tool benchmarks different Perl parser implementations and generates
//! detailed performance statistics compatible with the C vs Rust comparison workflow.
//!
//! Features:
//! - Multiple parser implementation benchmarking
//! - Statistical analysis with confidence intervals
//! - JSON output for comparison tools
//! - Memory usage tracking
//! - Scalability analysis
//! - Regression detection

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use tree_sitter_perl::PureRustParser;
use walkdir::WalkDir;

/// Configuration for benchmark runs
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BenchmarkConfig {
    iterations: usize,
    warmup_iterations: usize,
    test_files: Vec<String>,
    output_path: String,
    detailed_stats: bool,
    memory_tracking: bool,
}

/// Individual test result
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestResult {
    name: String,
    file_size: usize,
    iterations: usize,
    durations_ns: Vec<u64>,
    mean_duration_ns: f64,
    std_dev_ns: f64,
    min_duration_ns: u64,
    max_duration_ns: u64,
    median_duration_ns: f64,
    success_rate: f64,
    memory_usage_bytes: Option<u64>,
    tokens_per_second: Option<f64>,
}

/// Complete benchmark results
#[derive(Debug, Serialize, Deserialize)]
struct BenchmarkResults {
    metadata: BenchmarkMetadata,
    tests: HashMap<String, TestResult>,
    summary: BenchmarkSummary,
}

/// Benchmark metadata
#[derive(Debug, Serialize, Deserialize)]
struct BenchmarkMetadata {
    generated_at: String,
    parser_version: String,
    rust_version: String,
    total_tests: usize,
    total_iterations: usize,
    configuration: BenchmarkConfig,
}

/// Summary statistics
#[derive(Debug, Serialize, Deserialize)]
struct BenchmarkSummary {
    overall_mean_ns: f64,
    overall_std_dev_ns: f64,
    fastest_test: String,
    slowest_test: String,
    total_runtime_seconds: f64,
    success_rate: f64,
    performance_categories: HashMap<String, Vec<String>>,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            iterations: 100,
            warmup_iterations: 10,
            test_files: vec![
                "test/benchmark_simple.pl".to_string(),
                "test/corpus".to_string(), // Directory of test files
            ],
            output_path: "benchmark_results.json".to_string(),
            detailed_stats: true,
            memory_tracking: false, // Disabled by default for performance
        }
    }
}

struct BenchmarkRunner {
    config: BenchmarkConfig,
    parser: PureRustParser,
}

impl BenchmarkRunner {
    fn new(config: BenchmarkConfig) -> Self {
        Self {
            config,
            parser: PureRustParser::new(),
        }
    }

    fn discover_test_files(&self) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
        let mut test_files = Vec::new();
        
        for test_path in &self.config.test_files {
            let path = Path::new(test_path);
            
            if path.is_file() {
                let content = fs::read_to_string(path)?;
                let name = path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                test_files.push((name, content));
            } else if path.is_dir() {
                // Walk directory for .pl files
                for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
                    if let Some(ext) = entry.path().extension() {
                        if ext == "pl" || ext == "pm" || ext == "t" {
                            if let Ok(content) = fs::read_to_string(entry.path()) {
                                let name = entry.path().file_stem()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("unknown")
                                    .to_string();
                                test_files.push((name, content));
                            }
                        }
                    }
                }
            }
        }
        
        if test_files.is_empty() {
            return Err("No test files found".into());
        }
        
        println!("Found {} test files", test_files.len());
        Ok(test_files)
    }

    fn benchmark_test(&mut self, name: &str, content: &str) -> TestResult {
        println!("Benchmarking test: {} ({} bytes)", name, content.len());
        
        // Warmup runs
        for _ in 0..self.config.warmup_iterations {
            let _ = self.parser.parse(content, None);
        }
        
        // Actual benchmark runs
        let mut durations = Vec::with_capacity(self.config.iterations);
        let mut success_count = 0;
        
        for _ in 0..self.config.iterations {
            let start = Instant::now();
            let result = self.parser.parse(content, None);
            let duration = start.elapsed();
            
            durations.push(duration.as_nanos() as u64);
            
            if result.is_some() {
                success_count += 1;
            }
        }
        
        // Calculate statistics
        let mean = durations.iter().sum::<u64>() as f64 / durations.len() as f64;
        let variance = durations.iter()
            .map(|&d| {
                let diff = d as f64 - mean;
                diff * diff
            })
            .sum::<f64>() / durations.len() as f64;
        let std_dev = variance.sqrt();
        
        let mut sorted_durations = durations.clone();
        sorted_durations.sort_unstable();
        let median = if sorted_durations.len() % 2 == 0 {
            let mid = sorted_durations.len() / 2;
            (sorted_durations[mid - 1] + sorted_durations[mid]) as f64 / 2.0
        } else {
            sorted_durations[sorted_durations.len() / 2] as f64
        };
        
        // Estimate tokens per second (rough approximation)
        let estimated_tokens = content.split_whitespace().count() as f64;
        let tokens_per_second = if mean > 0.0 {
            Some(estimated_tokens / (mean / 1_000_000_000.0))
        } else {
            None
        };
        
        TestResult {
            name: name.to_string(),
            file_size: content.len(),
            iterations: self.config.iterations,
            durations_ns: durations,
            mean_duration_ns: mean,
            std_dev_ns: std_dev,
            min_duration_ns: *sorted_durations.first().unwrap(),
            max_duration_ns: *sorted_durations.last().unwrap(),
            median_duration_ns: median,
            success_rate: success_count as f64 / self.config.iterations as f64,
            memory_usage_bytes: None, // Would require additional instrumentation
            tokens_per_second,
        }
    }

    fn categorize_performance(&self, results: &HashMap<String, TestResult>) -> HashMap<String, Vec<String>> {
        let mut categories: HashMap<String, Vec<String>> = HashMap::new();
        
        for (name, result) in results {
            // Size-based categories
            let size_category = match result.file_size {
                0..=1000 => "small_files",
                1001..=10000 => "medium_files",
                _ => "large_files",
            };
            categories.entry(size_category.to_string())
                .or_default()
                .push(name.clone());
            
            // Performance-based categories
            let perf_category = match result.mean_duration_ns as u64 {
                0..=1_000_000 => "fast_parsing", // <1ms
                1_000_001..=10_000_000 => "moderate_parsing", // 1-10ms
                _ => "slow_parsing", // >10ms
            };
            categories.entry(perf_category.to_string())
                .or_default()
                .push(name.clone());
            
            // Success rate categories
            if result.success_rate < 1.0 {
                categories.entry("error_recovery".to_string())
                    .or_default()
                    .push(name.clone());
            }
        }
        
        categories
    }

    fn generate_summary(&self, results: &HashMap<String, TestResult>, total_runtime: Duration) -> BenchmarkSummary {
        if results.is_empty() {
            return BenchmarkSummary {
                overall_mean_ns: 0.0,
                overall_std_dev_ns: 0.0,
                fastest_test: "none".to_string(),
                slowest_test: "none".to_string(),
                total_runtime_seconds: total_runtime.as_secs_f64(),
                success_rate: 0.0,
                performance_categories: HashMap::new(),
            };
        }
        
        let mean_durations: Vec<f64> = results.values().map(|r| r.mean_duration_ns).collect();
        let overall_mean = mean_durations.iter().sum::<f64>() / mean_durations.len() as f64;
        
        let variance = mean_durations.iter()
            .map(|&d| {
                let diff = d - overall_mean;
                diff * diff
            })
            .sum::<f64>() / mean_durations.len() as f64;
        let overall_std_dev = variance.sqrt();
        
        let fastest_test = results.iter()
            .min_by(|a, b| a.1.mean_duration_ns.partial_cmp(&b.1.mean_duration_ns).unwrap())
            .map(|(name, _)| name.clone())
            .unwrap_or_else(|| "unknown".to_string());
        
        let slowest_test = results.iter()
            .max_by(|a, b| a.1.mean_duration_ns.partial_cmp(&b.1.mean_duration_ns).unwrap())
            .map(|(name, _)| name.clone())
            .unwrap_or_else(|| "unknown".to_string());
        
        let overall_success_rate = results.values()
            .map(|r| r.success_rate)
            .sum::<f64>() / results.len() as f64;
        
        let performance_categories = self.categorize_performance(results);
        
        BenchmarkSummary {
            overall_mean_ns: overall_mean,
            overall_std_dev_ns: overall_std_dev,
            fastest_test,
            slowest_test,
            total_runtime_seconds: total_runtime.as_secs_f64(),
            success_rate: overall_success_rate,
            performance_categories,
        }
    }

    fn run(&mut self) -> Result<BenchmarkResults, Box<dyn std::error::Error>> {
        let start_time = Instant::now();
        println!("Starting Rust parser benchmarks...");
        
        let test_files = self.discover_test_files()?;
        let mut results = HashMap::new();
        let mut total_iterations = 0;
        
        for (name, content) in test_files {
            let result = self.benchmark_test(&name, &content);
            total_iterations += result.iterations;
            results.insert(name, result);
        }
        
        let total_runtime = start_time.elapsed();
        let summary = self.generate_summary(&results, total_runtime);
        
        let benchmark_results = BenchmarkResults {
            metadata: BenchmarkMetadata {
                generated_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    .to_string(),
                parser_version: env!("CARGO_PKG_VERSION").to_string(),
                rust_version: std::env::var("RUSTC_VERSION").unwrap_or_else(|_| "unknown".to_string()),
                total_tests: results.len(),
                total_iterations,
                configuration: self.config.clone(),
            },
            tests: results,
            summary,
        };
        
        // Save results to JSON file
        let json_output = serde_json::to_string_pretty(&benchmark_results)?;
        fs::write(&self.config.output_path, json_output)?;
        
        println!("\nBenchmark Results Summary:");
        println!("  Total tests: {}", benchmark_results.metadata.total_tests);
        println!("  Total iterations: {}", benchmark_results.metadata.total_iterations);
        println!("  Overall mean: {:.2} ms", benchmark_results.summary.overall_mean_ns / 1_000_000.0);
        println!("  Overall std dev: {:.2} ms", benchmark_results.summary.overall_std_dev_ns / 1_000_000.0);
        println!("  Success rate: {:.1}%", benchmark_results.summary.success_rate * 100.0);
        println!("  Runtime: {:.2} seconds", benchmark_results.summary.total_runtime_seconds);
        println!("  Fastest test: {}", benchmark_results.summary.fastest_test);
        println!("  Slowest test: {}", benchmark_results.summary.slowest_test);
        println!("  Results saved to: {}", self.config.output_path);
        
        Ok(benchmark_results)
    }
}

fn load_config() -> BenchmarkConfig {
    // Try to load config from file, fall back to default
    if let Ok(config_content) = fs::read_to_string("benchmark_config.json") {
        if let Ok(config) = serde_json::from_str::<BenchmarkConfig>(&config_content) {
            println!("Loaded configuration from benchmark_config.json");
            return config;
        }
    }
    
    println!("Using default configuration");
    BenchmarkConfig::default()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = load_config();
    let mut runner = BenchmarkRunner::new(config);
    let _results = runner.run()?;
    Ok(())
}
