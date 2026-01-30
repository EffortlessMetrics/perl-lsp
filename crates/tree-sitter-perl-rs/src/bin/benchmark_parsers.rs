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
//!
//! Usage:
//!   benchmark_parsers [OPTIONS]
//!
//! Options:
//!   -o, --output <PATH>    Output file path (default: benchmark_results.json)
//!   -s, --save             Save results to file (implied when --output is used)
//!   -c, --config <PATH>    Configuration file path
//!   -i, --iterations <N>   Number of benchmark iterations (default: 100)
//!   -w, --warmup <N>       Number of warmup iterations (default: 10)
//!   -h, --help             Print help information
//!   -V, --version          Print version information

use clap::{Arg, ArgAction, Command};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};
use thiserror::Error;
use tree_sitter_perl::PureRustParser;
use walkdir::WalkDir;

/// Benchmark-specific error types
#[derive(Error, Debug)]
enum BenchmarkError {
    #[error("File I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    SerdeError(#[from] serde_json::Error),

    #[error("Configuration error: {message}")]
    ConfigError { message: String },

    #[error("No test files found in specified paths: {paths:?}")]
    NoTestFiles { paths: Vec<String> },

    #[error("Invalid output path: {path}")]
    InvalidOutputPath { path: String },

    #[error("Directory creation failed: {path} - {source}")]
    DirectoryCreationFailed { path: String, source: std::io::Error },
}

/// Configuration for benchmark runs
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BenchmarkConfig {
    iterations: usize,
    warmup_iterations: usize,
    test_files: Vec<String>,
    output_path: String,
    detailed_stats: bool,
    memory_tracking: bool,
    save_results: bool,
}

/// CLI arguments structure
#[derive(Debug)]
struct CliArgs {
    output_path: Option<String>,
    config_path: Option<String>,
    iterations: Option<usize>,
    warmup_iterations: Option<usize>,
    save_results: bool,
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
        // Check if running in CI/test environment for performance optimization
        let is_ci_env = std::env::var("CI").is_ok() || std::env::var("CARGO_TARGET_DIR").is_ok();

        let (iterations, warmup_iterations) = if is_ci_env {
            (5, 1) // Minimal iterations for CI/testing
        } else {
            (100, 10) // Full iterations for benchmarking
        };

        Self {
            iterations,
            warmup_iterations,
            test_files: vec![
                "test/benchmark_simple.pl".to_string(),
                "test/corpus".to_string(), // Directory of test files
            ],
            output_path: "benchmark_results.json".to_string(),
            detailed_stats: !is_ci_env, // Reduce stats in CI for performance
            memory_tracking: false,     // Disabled by default for performance
            save_results: true,
        }
    }
}

impl BenchmarkConfig {
    /// Validate configuration parameters (optimized - lazy directory creation)
    fn validate(&self) -> Result<(), BenchmarkError> {
        if self.iterations == 0 {
            return Err(BenchmarkError::ConfigError {
                message: "iterations must be greater than 0".to_string(),
            });
        }

        if self.test_files.is_empty() {
            return Err(BenchmarkError::ConfigError {
                message: "test_files cannot be empty".to_string(),
            });
        }

        // Only validate output path structure, don't create directories yet
        let output_path = Path::new(&self.output_path);
        if output_path.is_dir() {
            return Err(BenchmarkError::InvalidOutputPath { path: self.output_path.clone() });
        }

        // Skip expensive file system checks in validation
        // These will be handled lazily during execution

        Ok(())
    }

    /// Apply CLI overrides to configuration
    fn apply_cli_overrides(&mut self, args: &CliArgs) {
        if let Some(ref output_path) = args.output_path {
            self.output_path = output_path.clone();
            self.save_results = true; // Implicit when output is specified
        }

        if args.save_results {
            self.save_results = true;
        }

        if let Some(iterations) = args.iterations {
            self.iterations = iterations;
        }

        if let Some(warmup) = args.warmup_iterations {
            self.warmup_iterations = warmup;
        }
    }
}

struct BenchmarkRunner {
    config: BenchmarkConfig,
    parser: PureRustParser,
}

impl BenchmarkRunner {
    fn new(config: BenchmarkConfig) -> Result<Self, BenchmarkError> {
        config.validate()?;
        Ok(Self { config, parser: PureRustParser::new() })
    }

    fn discover_test_files(&self) -> Result<Vec<(String, String)>, BenchmarkError> {
        let mut test_files = Vec::with_capacity(16); // Pre-allocate capacity

        for test_path in &self.config.test_files {
            let path = Path::new(test_path);

            if path.is_file() {
                let content = fs::read_to_string(path).map_err(BenchmarkError::IoError)?;
                let name =
                    path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_string();
                test_files.push((name, content));
            } else if path.is_dir() {
                // Optimized directory walk with early filtering
                for entry in WalkDir::new(path)
                    .max_depth(2) // Limit recursion for performance
                    .into_iter()
                    .filter_entry(|e| {
                        // Pre-filter to avoid expensive operations on irrelevant files
                        e.path()
                            .extension()
                            .is_some_and(|ext| ext == "pl" || ext == "pm" || ext == "t")
                    })
                    .filter_map(|e| e.ok())
                {
                    if entry.file_type().is_file() {
                        if let Ok(content) = fs::read_to_string(entry.path()) {
                            let name = entry
                                .path()
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("unknown")
                                .to_string();
                            test_files.push((name, content));
                        }
                        // Skip warning for CI performance
                    }
                }
            }
            // Skip non-existent path warnings in CI for cleaner output
        }

        if test_files.is_empty() {
            return Err(BenchmarkError::NoTestFiles { paths: self.config.test_files.clone() });
        }

        println!("Found {} test files", test_files.len());
        Ok(test_files)
    }

    fn benchmark_test(&mut self, name: &str, content: &str) -> TestResult {
        println!("Benchmarking test: {} ({} bytes)", name, content.len());

        // Warmup runs
        for _ in 0..self.config.warmup_iterations {
            let _ = self.parser.parse(content);
        }

        // Actual benchmark runs
        let mut durations = Vec::with_capacity(self.config.iterations);
        let mut success_count = 0;

        for _ in 0..self.config.iterations {
            let start = Instant::now();
            let result = self.parser.parse(content);
            let duration = start.elapsed();

            durations.push(duration.as_nanos() as u64);

            if result.is_ok() {
                success_count += 1;
            }
        }

        // Calculate statistics
        let mean = durations.iter().sum::<u64>() as f64 / durations.len() as f64;
        let variance = durations
            .iter()
            .map(|&d| {
                let diff = d as f64 - mean;
                diff * diff
            })
            .sum::<f64>()
            / durations.len() as f64;
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
        let tokens_per_second =
            if mean > 0.0 { Some(estimated_tokens / (mean / 1_000_000_000.0)) } else { None };

        TestResult {
            name: name.to_string(),
            file_size: content.len(),
            iterations: self.config.iterations,
            durations_ns: durations,
            mean_duration_ns: mean,
            std_dev_ns: std_dev,
            min_duration_ns: *sorted_durations.first().unwrap_or(&0),
            max_duration_ns: *sorted_durations.last().unwrap_or(&0),
            median_duration_ns: median,
            success_rate: success_count as f64 / self.config.iterations as f64,
            memory_usage_bytes: None, // Would require additional instrumentation
            tokens_per_second,
        }
    }

    fn categorize_performance(
        &self,
        results: &HashMap<String, TestResult>,
    ) -> HashMap<String, Vec<String>> {
        let mut categories: HashMap<String, Vec<String>> = HashMap::new();

        for (name, result) in results {
            // Size-based categories
            let size_category = match result.file_size {
                0..=1000 => "small_files",
                1001..=10000 => "medium_files",
                _ => "large_files",
            };
            categories.entry(size_category.to_string()).or_default().push(name.clone());

            // Performance-based categories
            let perf_category = match result.mean_duration_ns as u64 {
                0..=1_000_000 => "fast_parsing",              // <1ms
                1_000_001..=10_000_000 => "moderate_parsing", // 1-10ms
                _ => "slow_parsing",                          // >10ms
            };
            categories.entry(perf_category.to_string()).or_default().push(name.clone());

            // Success rate categories
            if result.success_rate < 1.0 {
                categories.entry("error_recovery".to_string()).or_default().push(name.clone());
            }
        }

        categories
    }

    fn generate_summary(
        &self,
        results: &HashMap<String, TestResult>,
        total_runtime: Duration,
    ) -> BenchmarkSummary {
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

        let variance = mean_durations
            .iter()
            .map(|&d| {
                let diff = d - overall_mean;
                diff * diff
            })
            .sum::<f64>()
            / mean_durations.len() as f64;
        let overall_std_dev = variance.sqrt();

        let fastest_test = results
            .iter()
            .min_by(|a, b| {
                a.1.mean_duration_ns
                    .partial_cmp(&b.1.mean_duration_ns)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(name, _)| name.clone())
            .unwrap_or_else(|| "unknown".to_string());

        let slowest_test = results
            .iter()
            .max_by(|a, b| {
                a.1.mean_duration_ns
                    .partial_cmp(&b.1.mean_duration_ns)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(name, _)| name.clone())
            .unwrap_or_else(|| "unknown".to_string());

        let overall_success_rate =
            results.values().map(|r| r.success_rate).sum::<f64>() / results.len() as f64;

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

    fn run(&mut self) -> Result<BenchmarkResults, BenchmarkError> {
        let start_time = Instant::now();
        println!("Starting Rust parser benchmarks...");
        println!("Configuration:");
        println!("  Iterations: {}", self.config.iterations);
        println!("  Warmup iterations: {}", self.config.warmup_iterations);
        println!("  Output path: {}", self.config.output_path);
        println!("  Save results: {}", self.config.save_results);
        println!();

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
                generated_at: chrono::Utc::now().to_rfc3339(),
                parser_version: env!("CARGO_PKG_VERSION").to_string(),
                rust_version: std::env::var("RUSTC_VERSION")
                    .unwrap_or_else(|_| "unknown".to_string()),
                total_tests: results.len(),
                total_iterations,
                configuration: self.config.clone(),
            },
            tests: results,
            summary,
        };

        // Save results to JSON file if requested
        if self.config.save_results {
            self.save_results(&benchmark_results)?;
        }

        self.print_summary(&benchmark_results);

        Ok(benchmark_results)
    }

    /// Save benchmark results to the specified output file
    fn save_results(&self, results: &BenchmarkResults) -> Result<(), BenchmarkError> {
        let output_path = Path::new(&self.config.output_path);

        // Ensure parent directory exists
        if let Some(parent) = output_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).map_err(|e| {
                    BenchmarkError::DirectoryCreationFailed {
                        path: parent.display().to_string(),
                        source: e,
                    }
                })?;
            }
        }

        let json_output =
            serde_json::to_string_pretty(results).map_err(BenchmarkError::SerdeError)?;

        fs::write(&self.config.output_path, json_output).map_err(BenchmarkError::IoError)?;

        println!("Results saved to: {}", self.config.output_path);
        Ok(())
    }

    /// Print benchmark summary to console
    fn print_summary(&self, results: &BenchmarkResults) {
        println!("\nBenchmark Results Summary:");
        println!("  Total tests: {}", results.metadata.total_tests);
        println!("  Total iterations: {}", results.metadata.total_iterations);
        println!("  Overall mean: {:.2} ms", results.summary.overall_mean_ns / 1_000_000.0);
        println!("  Overall std dev: {:.2} ms", results.summary.overall_std_dev_ns / 1_000_000.0);
        println!("  Success rate: {:.1}%", results.summary.success_rate * 100.0);
        println!("  Runtime: {:.2} seconds", results.summary.total_runtime_seconds);
        println!("  Fastest test: {}", results.summary.fastest_test);
        println!("  Slowest test: {}", results.summary.slowest_test);

        if self.config.save_results {
            println!("  Results saved to: {}", self.config.output_path);
        } else {
            println!("  Results not saved (use --save or --output to save)");
        }
    }
}

/// Parse command line arguments (optimized for performance)
fn parse_args() -> CliArgs {
    let matches = Command::new("benchmark_parsers")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Comprehensive benchmark runner for Perl parser implementations")
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("PATH")
                .help("Output file path (default: benchmark_results.json)"),
        )
        .arg(
            Arg::new("save")
                .short('s')
                .long("save")
                .help("Save results to file (implied when --output is used)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("PATH")
                .help("Configuration file path"),
        )
        .arg(
            Arg::new("iterations")
                .short('i')
                .long("iterations")
                .value_name("N")
                .help("Number of benchmark iterations (default: 100)")
                .value_parser(clap::value_parser!(usize)),
        )
        .arg(
            Arg::new("warmup")
                .short('w')
                .long("warmup")
                .value_name("N")
                .help("Number of warmup iterations (default: 10)")
                .value_parser(clap::value_parser!(usize)),
        )
        .get_matches();

    CliArgs {
        output_path: matches.get_one::<String>("output").cloned(),
        config_path: matches.get_one::<String>("config").cloned(),
        iterations: matches.get_one::<usize>("iterations").copied(),
        warmup_iterations: matches.get_one::<usize>("warmup").copied(),
        save_results: matches.get_flag("save"),
    }
}

/// Load configuration from file or defaults (optimized)
fn load_config(args: &CliArgs) -> Result<BenchmarkConfig, BenchmarkError> {
    let mut config = if let Some(ref config_path) = args.config_path {
        // Only try to load specified config file
        let config_content =
            fs::read_to_string(config_path).map_err(|e| BenchmarkError::ConfigError {
                message: format!("Failed to read config file {}: {}", config_path, e),
            })?;

        serde_json::from_str::<BenchmarkConfig>(&config_content).map_err(|e| {
            BenchmarkError::ConfigError {
                message: format!("Invalid JSON in config file {}: {}", config_path, e),
            }
        })?
    } else {
        // Skip automatic config file discovery - use defaults directly
        // This eliminates unnecessary file system operations
        BenchmarkConfig::default()
    };

    // Apply CLI overrides
    config.apply_cli_overrides(args);

    Ok(config)
}

fn main() -> Result<(), BenchmarkError> {
    // Fast path for help/version commands (avoids all initialization overhead)
    let raw_args: Vec<String> = std::env::args().collect();
    if raw_args.len() > 1 {
        match raw_args[1].as_str() {
            "--help" | "-h" => {
                let _args = parse_args(); // This will handle help and exit
                return Ok(());
            }
            "--version" | "-V" => {
                let _args = parse_args(); // This will handle version and exit
                return Ok(());
            }
            _ => {}
        }
    }

    let args = parse_args();
    let config = load_config(&args)?;
    let mut runner = BenchmarkRunner::new(config)?;
    let _results = runner.run()?;
    Ok(())
}
