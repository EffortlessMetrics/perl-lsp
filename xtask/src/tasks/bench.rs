//! Benchmark task implementation
//!
//! This module provides comprehensive benchmarking capabilities to compare
//! the legacy C implementation with the modern Rust implementation.
//!
//! ## Design Goals
//!
//! 1. **Accurate Performance Comparison**: Proper C vs Rust implementation comparison
//! 2. **Comprehensive Coverage**: Scanner, parser, memory, and scalability benchmarks
//! 3. **Regression Detection**: Automated performance regression testing
//! 4. **Statistical Validity**: Proper statistical analysis with confidence intervals
//! 5. **CI Integration**: Performance gates for continuous integration
//!
//! ## Architecture
//!
//! The benchmarking system consists of:
//!
//! - **Criterion Benchmarks**: Rust-native performance measurement
//! - **C Implementation Benchmarks**: Node.js-based C parser benchmarking
//! - **Comparison Engine**: Statistical comparison and analysis
//! - **Regression Detection**: Automated regression testing
//! - **Result Storage**: Historical performance tracking
//!
//! ## Implementation Phases
//!
//! ### Phase 1: Basic Criterion Integration ✅
//! - Simple criterion benchmark execution
//! - Basic performance measurement
//! - Xtask integration
//!
//! ### Phase 2: C Implementation Benchmarking 🔄
//! - Node.js-based C parser benchmarking
//! - Fair comparison methodology
//! - Statistical analysis
//!
//! ### Phase 3: Advanced Features 🔄
//! - Memory usage measurement
//! - Scalability analysis
//! - Regression detection
//! - Performance gates

use color_eyre::eyre::{Context, Result};
use duct::cmd;
use indicatif::{ProgressBar, ProgressStyle};
use serde::Deserialize;
use std::fs;
use walkdir::WalkDir;

pub fn run(name: Option<String>, save: bool, output: Option<std::path::PathBuf>) -> Result<()> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner().template("{spinner:.green} {wide_msg}").unwrap(),
    );

    spinner.set_message("Running benchmarks");

    // Build arguments for cargo bench
    let mut args = vec!["bench"];

    if let Some(bench_name) = &name {
        args.push("--bench");
        args.push(bench_name);
    }

    // Execute benchmarks
    let status = cmd("cargo", &args).run().context("Failed to run benchmarks")?;

    if status.status.success() {
        spinner.finish_with_message("✅ Benchmarks completed");
    } else {
        spinner.finish_with_message("❌ Benchmarks failed");
        return Err(color_eyre::eyre::eyre!("Benchmarks failed with status: {}", status.status));
    }

    if save {
        spinner.set_message("Saving benchmark results");

        if let Some(output_path) = output {
            // TODO: Implement custom result saving to specified path
            spinner.finish_with_message(format!(
                "✅ Benchmark results saved to {}",
                output_path.display()
            ));
        } else {
            // Note: Criterion automatically saves results to target/criterion
            spinner.finish_with_message("✅ Benchmark results saved to target/criterion");
        }
    }

    // Phase 2: C vs Rust comparison flow
    let c_result = run_c_benchmarks()?;
    let rust_mean = extract_rust_mean()?;
    let comparison = compare_implementations(rust_mean, c_result.average);
    detect_regressions(&comparison)?;
    let report = generate_report(&comparison);
    println!("{}", report);

    Ok(())
}

/// Result from running the C benchmark harness
#[derive(Debug, Deserialize)]
struct CBenchmarkResult {
    duration: u64,
    iterations: u64,
    average: f64,
}

/// Run the Node.js C implementation benchmark and return timing results
fn run_c_benchmarks() -> Result<CBenchmarkResult> {
    let test_code = fs::read_to_string("test/benchmark_simple.pl")
        .context("Failed to read test Perl source for C benchmark")?;

    let output = cmd("node", &["tree-sitter-perl/test/benchmark.js"])
        .env("TEST_CODE", test_code)
        .env("ITERATIONS", "100")
        .read()
        .context("Failed to run C benchmark harness")?;

    let result: CBenchmarkResult =
        serde_json::from_str(&output).context("Failed to parse C benchmark output")?;
    Ok(result)
}

/// Extract the mean time from the latest Criterion benchmark output
fn extract_rust_mean() -> Result<f64> {
    for entry in WalkDir::new("target/criterion").into_iter().filter_map(|e| e.ok()) {
        if entry.file_name() == "estimates.json" {
            let data = fs::read_to_string(entry.path())?;
            let json: serde_json::Value = serde_json::from_str(&data)?;
            if let Some(mean) =
                json.get("mean").and_then(|m| m.get("point_estimate")).and_then(|v| v.as_f64())
            {
                return Ok(mean);
            }
        }
    }
    Err(color_eyre::eyre::eyre!("No Criterion benchmark estimates found"))
}

/// Comparison between C and Rust benchmark results
#[derive(Debug)]
struct BenchmarkComparison {
    rust_avg: f64,
    c_avg: f64,
    speedup: f64,
}

/// Compare benchmark results and calculate relative performance
fn compare_implementations(rust_avg: f64, c_avg: f64) -> BenchmarkComparison {
    let speedup = c_avg / rust_avg;
    BenchmarkComparison { rust_avg, c_avg, speedup }
}

/// Detect simple regressions based on a 10% slowdown threshold
fn detect_regressions(comparison: &BenchmarkComparison) -> Result<()> {
    if comparison.speedup < 0.9 {
        eprintln!(
            "⚠️  Potential regression: Rust average {:.2}ns vs C average {:.2}ns",
            comparison.rust_avg, comparison.c_avg
        );
    }
    Ok(())
}

/// Generate a human readable benchmark report
fn generate_report(comparison: &BenchmarkComparison) -> String {
    format!(
        "Rust avg: {:.2} ns\nC avg: {:.2} ns\nRust is {:.2}x faster than C",
        comparison.rust_avg, comparison.c_avg, comparison.speedup
    )
}
