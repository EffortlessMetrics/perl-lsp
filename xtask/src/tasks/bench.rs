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

use chrono::Utc;
use color_eyre::eyre::{Context, Result};
use duct::cmd;
use indicatif::{ProgressBar, ProgressStyle};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Validates the output path for benchmark results
fn validate_output_path(output_path: &Path) -> Result<()> {
    // Prevent path traversal attacks
    if output_path.to_string_lossy().contains("..") {
        return Err(color_eyre::eyre::eyre!("Path traversal not allowed in output path"));
    }

    // Ensure the filename has a reasonable extension for text output
    if let Some(extension) = output_path.extension() {
        let ext_str = extension.to_string_lossy();
        if !matches!(ext_str.as_ref(), "txt" | "log" | "out" | "bench" | "json" | "md") {
            return Err(color_eyre::eyre::eyre!(
                "Unsupported file extension '{}'. Use txt, log, out, bench, json, or md",
                ext_str
            ));
        }
    }

    // Ensure parent directory is writable if it exists
    if let Some(parent) = output_path.parent()
        && parent.exists()
        && parent.metadata()?.permissions().readonly()
    {
        return Err(color_eyre::eyre::eyre!(
            "Output directory '{}' is read-only",
            parent.display()
        ));
    }

    Ok(())
}

pub fn run(name: Option<String>, save: bool, output: Option<PathBuf>) -> Result<()> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(ProgressStyle::default_spinner().template("{spinner:.green} {wide_msg}")?);

    spinner.set_message("Running benchmarks");

    // Build arguments for cargo bench
    let mut args = vec!["bench"];

    if let Some(bench_name) = &name {
        args.push("--bench");
        args.push(bench_name);
    }

    // Execute benchmarks and capture output
    let result = cmd("cargo", &args)
        .stderr_to_stdout()
        .stdout_capture()
        .run()
        .context("Failed to run benchmarks")?;

    if result.status.success() {
        spinner.finish_with_message("✅ Benchmarks completed");
    } else {
        spinner.finish_with_message("❌ Benchmarks failed");
        return Err(color_eyre::eyre::eyre!("Benchmarks failed with status: {}", result.status));
    }

    if save {
        spinner.set_message("Saving benchmark results");

        if let Some(output_path) = output {
            // Validate the output path before proceeding
            validate_output_path(&output_path)
                .with_context(|| format!("Invalid output path: {}", output_path.display()))?;

            // Create parent directories if needed
            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent).with_context(|| {
                    format!("Failed to create output directory: {}", parent.display())
                })?;
            }

            // Write benchmark results to file
            fs::write(&output_path, &result.stdout).with_context(|| {
                format!("Failed to write benchmark results to: {}", output_path.display())
            })?;

            spinner.finish_with_message(format!(
                "✅ Benchmark results saved to {}",
                output_path.display()
            ));
        } else {
            // Default behavior: save to timestamped file in current directory
            let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
            let default_path = PathBuf::from(format!("benchmark_results_{}.txt", timestamp));

            fs::write(&default_path, &result.stdout).with_context(|| {
                format!("Failed to write benchmark results to: {}", default_path.display())
            })?;

            spinner.finish_with_message(format!(
                "✅ Benchmark results saved to {} (Criterion data also available in target/criterion)",
                default_path.display()
            ));
        }
    }

    // Phase 2: C vs Rust comparison flow (optional - only if files exist)
    if std::path::Path::new("test/benchmark_simple.pl").exists()
        && std::path::Path::new("tree-sitter-perl/test/benchmark.js").exists()
    {
        let c_result = run_c_benchmarks()?;
        let rust_mean = extract_rust_mean()?;
        let comparison = compare_implementations(rust_mean, c_result.average);
        detect_regressions(&comparison)?;
        let report = generate_report(&comparison);
        println!("{}", report);
    } else {
        println!("⚠️  C benchmark files not found - skipping C vs Rust comparison");
    }

    Ok(())
}

/// Result from running the C benchmark harness
#[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_validate_output_path_valid_extensions() {
        let valid_extensions = ["txt", "log", "out", "bench", "json", "md"];

        for ext in &valid_extensions {
            let path = PathBuf::from(format!("test.{}", ext));
            assert!(validate_output_path(&path).is_ok(), "Extension {} should be valid", ext);
        }
    }

    #[test]
    fn test_validate_output_path_invalid_extension() {
        let path = PathBuf::from("test.exe");
        assert!(validate_output_path(&path).is_err());
    }

    #[test]
    fn test_validate_output_path_no_extension() {
        let path = PathBuf::from("test");
        // Should be valid - no extension is allowed
        assert!(validate_output_path(&path).is_ok());
    }

    #[test]
    fn test_validate_output_path_prevents_traversal() {
        let paths = [
            PathBuf::from("../test.txt"),
            PathBuf::from("../../test.txt"),
            PathBuf::from("test/../../../etc/passwd"),
        ];

        for path in &paths {
            assert!(
                validate_output_path(path).is_err(),
                "Path traversal should be blocked for: {}",
                path.display()
            );
        }
    }

    #[test]
    fn test_validate_output_path_readonly_directory() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let readonly_dir = temp_dir.path().join("readonly");
        fs::create_dir(&readonly_dir)?;

        // Set directory to readonly
        let mut perms = fs::metadata(&readonly_dir)?.permissions();
        perms.set_readonly(true);
        fs::set_permissions(&readonly_dir, perms)?;

        let output_path = readonly_dir.join("test.txt");
        let result = validate_output_path(&output_path);

        // Should fail due to readonly directory
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_validate_output_path_writable_directory() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let output_path = temp_dir.path().join("test.txt");

        // Should succeed for writable directory
        assert!(validate_output_path(&output_path).is_ok());

        Ok(())
    }

    #[test]
    fn test_validate_output_path_deep_nested_path() {
        let path = PathBuf::from("deeply/nested/directory/structure/test.txt");
        // Should be valid - deep nesting is allowed
        assert!(validate_output_path(&path).is_ok());
    }

    #[test]
    fn test_validate_output_path_with_special_characters() {
        // Test various special characters that should be allowed
        let valid_paths =
            ["test_file.txt", "test-file.txt", "test file.txt", "test.file.txt", "test123.txt"];

        for path_str in &valid_paths {
            let path = PathBuf::from(path_str);
            assert!(validate_output_path(&path).is_ok(), "Path should be valid: {}", path_str);
        }
    }

    // Integration test would require mocking cargo bench, which is complex
    // So we focus on unit tests for the validation logic

    #[test]
    fn test_benchmark_with_mock_command() {
        // This would require substantial mocking infrastructure
        // For now, we test the validation logic which is the main improvement
        // In a real scenario, we might use a test framework like mockall

        // Test that we can at least call the validation function
        use perl_tdd_support::must;
        let temp_dir = must(tempfile::tempdir());
        let output_path = temp_dir.path().join("benchmark_results.txt");

        assert!(validate_output_path(&output_path).is_ok());
    }
}
