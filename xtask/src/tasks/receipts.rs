//! Generate canonical receipts for documentation truth
//!
//! This module replaces `scripts/generate-receipts.sh` with a Rust implementation
//! that runs workspace tests and doc builds, parses their output, and produces
//! consolidated JSON artifacts.
//!
//! # Usage
//!
//! ```bash
//! cargo xtask receipts                    # Run all receipt generation
//! cargo xtask receipts --tests-only       # Only generate test receipts
//! cargo xtask receipts --docs-only        # Only generate doc receipts
//! cargo xtask receipts --output-dir path  # Custom output directory
//! ```

use color_eyre::eyre::{Context, Result};
use duct::cmd;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::utils::project_root;

// =============================================================================
// Configuration
// =============================================================================

/// Configuration for receipt generation
pub struct ReceiptsConfig {
    /// Only generate test receipts
    pub tests_only: bool,
    /// Only generate doc receipts
    pub docs_only: bool,
    /// Output directory (default: artifacts/)
    pub output_dir: Option<PathBuf>,
    /// Thread count for test execution
    pub test_threads: u32,
}

impl Default for ReceiptsConfig {
    fn default() -> Self {
        Self {
            tests_only: false,
            docs_only: false,
            output_dir: None,
            test_threads: 2,
        }
    }
}

// =============================================================================
// Output Types
// =============================================================================

#[derive(Debug, Serialize)]
struct TestSummary {
    passed: u64,
    failed: u64,
    ignored: u64,
    active_tests: u64,
    total_all_tests: u64,
    pass_rate_active: f64,
    pass_rate_total: f64,
}

#[derive(Debug, Serialize)]
struct DocSummary {
    missing_docs: u64,
}

#[derive(Debug, Serialize)]
struct ConsolidatedState {
    version: String,
    tests: TestSummary,
    docs: DocSummary,
    generated_at: String,
}

// =============================================================================
// Main Entry Point
// =============================================================================

/// Run receipt generation
pub fn run(config: ReceiptsConfig) -> Result<()> {
    let root = project_root()?;
    std::env::set_current_dir(&root).context("Failed to change to project root")?;

    let artifacts_dir = config
        .output_dir
        .clone()
        .unwrap_or_else(|| root.join("artifacts"));
    fs::create_dir_all(&artifacts_dir).with_context(|| {
        format!(
            "Failed to create artifacts dir: {}",
            artifacts_dir.display()
        )
    })?;

    let test_summary = if !config.docs_only {
        println!("=== Generating Test Receipts ===");
        let summary = generate_test_receipts(&artifacts_dir, config.test_threads)?;
        println!(
            "Test summary: {} passed, {} failed, {} ignored",
            summary.passed, summary.failed, summary.ignored
        );
        summary
    } else {
        TestSummary {
            passed: 0,
            failed: 0,
            ignored: 0,
            active_tests: 0,
            total_all_tests: 0,
            pass_rate_active: 0.0,
            pass_rate_total: 0.0,
        }
    };

    let doc_summary = if !config.tests_only {
        println!();
        println!("=== Generating Doc Receipts ===");
        let summary = generate_doc_receipts(&artifacts_dir)?;
        println!("Doc summary: {} missing docs", summary.missing_docs);
        summary
    } else {
        DocSummary { missing_docs: 0 }
    };

    println!();
    println!("=== Generating Consolidated State ===");
    let state = generate_consolidated_state(test_summary, doc_summary)?;
    let state_path = artifacts_dir.join("state.json");
    let state_json =
        serde_json::to_string_pretty(&state).context("Failed to serialize consolidated state")?;
    fs::write(&state_path, &state_json)
        .with_context(|| format!("Failed to write state to {}", state_path.display()))?;
    println!("Consolidated state saved to {}", state_path.display());
    println!("{state_json}");

    println!();
    println!("=== Receipt Generation Complete ===");
    println!("Artifacts:");
    println!(
        "  - {}/test-output.txt     (raw test output)",
        artifacts_dir.display()
    );
    println!(
        "  - {}/test-summary.json   (parsed test metrics)",
        artifacts_dir.display()
    );
    println!(
        "  - {}/rustdoc.log         (doc build output)",
        artifacts_dir.display()
    );
    println!(
        "  - {}/doc-summary.json    (doc metrics)",
        artifacts_dir.display()
    );
    println!(
        "  - {}/state.json          (consolidated truth)",
        artifacts_dir.display()
    );

    Ok(())
}

// =============================================================================
// Test Receipt Generation
// =============================================================================

/// Run workspace tests and parse results into a summary
fn generate_test_receipts(artifacts_dir: &Path, test_threads: u32) -> Result<TestSummary> {
    let start = Instant::now();
    let test_output_path = artifacts_dir.join("test-output.txt");
    let test_summary_path = artifacts_dir.join("test-summary.json");

    let threads_str = test_threads.to_string();

    // Run cargo test, capturing output
    // Exclude xtask which may have compilation issues in some configurations
    let result = cmd!(
        "cargo",
        "+stable",
        "test",
        "--workspace",
        "--exclude",
        "xtask",
        "--all-features",
        "--no-fail-fast",
        "--",
        "--test-threads",
        &threads_str
    )
    .env("RUST_TEST_THREADS", &threads_str)
    .env("LC_ALL", "C")
    .stderr_to_stdout()
    .stdout_capture()
    .unchecked()
    .run()
    .context("Failed to execute cargo test")?;

    let output = String::from_utf8_lossy(&result.stdout);

    // Write raw output
    fs::write(&test_output_path, output.as_bytes()).with_context(|| {
        format!(
            "Failed to write test output to {}",
            test_output_path.display()
        )
    })?;

    let elapsed = start.elapsed();
    println!(
        "Tests completed in {:.1}s (exit code: {})",
        elapsed.as_secs_f64(),
        result.status.code().unwrap_or(-1)
    );

    // Parse test output
    let summary = parse_test_output(&output);

    // Write test summary
    let summary_json =
        serde_json::to_string_pretty(&summary).context("Failed to serialize test summary")?;
    fs::write(&test_summary_path, &summary_json).with_context(|| {
        format!(
            "Failed to write test summary to {}",
            test_summary_path.display()
        )
    })?;
    println!("Test summary saved to {}", test_summary_path.display());

    Ok(summary)
}

/// Parse cargo test output to extract aggregate test counts
///
/// Looks for lines matching: `test result: ok. N passed; N failed; N ignored; ...`
/// and sums across all crate test runs.
fn parse_test_output(output: &str) -> TestSummary {
    let mut total_passed: u64 = 0;
    let mut total_failed: u64 = 0;
    let mut total_ignored: u64 = 0;

    for line in output.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("test result:") {
            continue;
        }

        // Parse "test result: ok. 272 passed; 0 failed; 818 ignored; 0 measured; 0 filtered out"
        // The numbers appear as: N passed, N failed, N ignored
        if let Some(passed) = extract_count_before(trimmed, "passed") {
            total_passed += passed;
        }
        if let Some(failed) = extract_count_before(trimmed, "failed") {
            total_failed += failed;
        }
        if let Some(ignored) = extract_count_before(trimmed, "ignored") {
            total_ignored += ignored;
        }
    }

    let active_tests = total_passed + total_failed;
    let total_all_tests = active_tests + total_ignored;

    let pass_rate_active = if active_tests > 0 {
        (total_passed as f64 / active_tests as f64) * 100.0
    } else {
        0.0
    };

    let pass_rate_total = if total_all_tests > 0 {
        (total_passed as f64 / total_all_tests as f64) * 100.0
    } else {
        0.0
    };

    // Round to 1 decimal place
    let pass_rate_active = (pass_rate_active * 10.0).round() / 10.0;
    let pass_rate_total = (pass_rate_total * 10.0).round() / 10.0;

    TestSummary {
        passed: total_passed,
        failed: total_failed,
        ignored: total_ignored,
        active_tests,
        total_all_tests,
        pass_rate_active,
        pass_rate_total,
    }
}

/// Extract the number immediately before a keyword in a test result line
///
/// For input "272 passed; 0 failed; 818 ignored" and keyword "passed",
/// returns Some(272).
fn extract_count_before(line: &str, keyword: &str) -> Option<u64> {
    // Find the keyword position
    let keyword_pos = line.find(keyword)?;
    let before = &line[..keyword_pos];

    // The number is the last whitespace-delimited token before the keyword
    before.split_whitespace().last()?.parse().ok()
}

// =============================================================================
// Doc Receipt Generation
// =============================================================================

/// Run cargo doc and count missing documentation warnings
fn generate_doc_receipts(artifacts_dir: &Path) -> Result<DocSummary> {
    let rustdoc_log_path = artifacts_dir.join("rustdoc.log");
    let doc_summary_path = artifacts_dir.join("doc-summary.json");

    // Run cargo doc, capturing stderr (where warnings go)
    let result = cmd!(
        "cargo",
        "+stable",
        "doc",
        "--no-deps",
        "--workspace",
        "--exclude",
        "xtask"
    )
    .stdout_capture()
    .stderr_capture()
    .unchecked()
    .run()
    .context("Failed to execute cargo doc")?;

    let stderr = String::from_utf8_lossy(&result.stderr);

    // Write rustdoc log
    fs::write(&rustdoc_log_path, stderr.as_bytes()).with_context(|| {
        format!(
            "Failed to write rustdoc log to {}",
            rustdoc_log_path.display()
        )
    })?;

    // Count "warning: missing documentation" lines
    let missing_docs = stderr
        .lines()
        .filter(|line| line.starts_with("warning: missing documentation"))
        .count() as u64;

    let summary = DocSummary { missing_docs };

    // Write doc summary
    let summary_json =
        serde_json::to_string_pretty(&summary).context("Failed to serialize doc summary")?;
    fs::write(&doc_summary_path, &summary_json).with_context(|| {
        format!(
            "Failed to write doc summary to {}",
            doc_summary_path.display()
        )
    })?;
    println!("Doc summary saved to {}", doc_summary_path.display());

    Ok(summary)
}

// =============================================================================
// Consolidated State
// =============================================================================

/// Extract version from cargo metadata and build consolidated state
fn generate_consolidated_state(tests: TestSummary, docs: DocSummary) -> Result<ConsolidatedState> {
    let version = extract_version()?;
    let generated_at = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    Ok(ConsolidatedState {
        version,
        tests,
        docs,
        generated_at,
    })
}

/// Extract perl-parser version from cargo metadata
fn extract_version() -> Result<String> {
    let metadata_output = cmd!("cargo", "metadata", "-q", "--format-version=1")
        .stdout_capture()
        .run()
        .context("Failed to run cargo metadata")?;

    let metadata_str = String::from_utf8_lossy(&metadata_output.stdout);

    // Parse JSON to find perl-parser package version
    let metadata: serde_json::Value =
        serde_json::from_str(&metadata_str).context("Failed to parse cargo metadata JSON")?;

    let packages = metadata
        .get("packages")
        .and_then(|p| p.as_array())
        .ok_or_else(|| color_eyre::eyre::eyre!("No 'packages' array in cargo metadata"))?;

    for package in packages {
        let name = package.get("name").and_then(|n| n.as_str()).unwrap_or("");
        if name == "perl-parser" {
            let version = package
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            return Ok(version.to_string());
        }
    }

    Ok("unknown".to_string())
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_test_output_extracts_counts_from_single_crate() {
        let output = "test result: ok. 10 passed; 2 failed; 3 ignored; 0 measured; 0 filtered out";
        let summary = parse_test_output(output);
        assert_eq!(summary.passed, 10);
        assert_eq!(summary.failed, 2);
        assert_eq!(summary.ignored, 3);
        assert_eq!(summary.active_tests, 12);
        assert_eq!(summary.total_all_tests, 15);
    }

    #[test]
    fn parse_test_output_aggregates_multiple_crates() {
        let output = "\
running 5 tests
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

running 10 tests
test result: ok. 8 passed; 1 failed; 1 ignored; 0 measured; 0 filtered out
";
        let summary = parse_test_output(output);
        assert_eq!(summary.passed, 13);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.ignored, 1);
        assert_eq!(summary.active_tests, 14);
        assert_eq!(summary.total_all_tests, 15);
    }

    #[test]
    fn parse_test_output_handles_empty_output() {
        let summary = parse_test_output("");
        assert_eq!(summary.passed, 0);
        assert_eq!(summary.failed, 0);
        assert_eq!(summary.ignored, 0);
        assert_eq!(summary.pass_rate_active, 0.0);
        assert_eq!(summary.pass_rate_total, 0.0);
    }

    #[test]
    fn parse_test_output_handles_no_test_results() {
        let output = "Compiling foo v0.1.0\nFinished test\n";
        let summary = parse_test_output(output);
        assert_eq!(summary.passed, 0);
        assert_eq!(summary.total_all_tests, 0);
    }

    #[test]
    fn extract_count_before_finds_passed() {
        let line = "test result: ok. 272 passed; 0 failed; 818 ignored; 0 measured; 0 filtered out";
        assert_eq!(extract_count_before(line, "passed"), Some(272));
        assert_eq!(extract_count_before(line, "failed"), Some(0));
        assert_eq!(extract_count_before(line, "ignored"), Some(818));
    }

    #[test]
    fn extract_count_before_returns_none_for_missing_keyword() {
        let line = "test result: ok. 5 passed; 0 failed";
        assert_eq!(extract_count_before(line, "ignored"), None);
    }

    #[test]
    fn pass_rate_calculations_are_correct() {
        let output = "test result: ok. 90 passed; 10 failed; 0 ignored; 0 measured; 0 filtered out";
        let summary = parse_test_output(output);
        assert!((summary.pass_rate_active - 90.0).abs() < 0.01);
        assert!((summary.pass_rate_total - 90.0).abs() < 0.01);
    }

    #[test]
    fn pass_rate_with_ignored_tests() {
        let output = "test result: ok. 80 passed; 0 failed; 20 ignored; 0 measured; 0 filtered out";
        let summary = parse_test_output(output);
        assert!((summary.pass_rate_active - 100.0).abs() < 0.01);
        assert!((summary.pass_rate_total - 80.0).abs() < 0.01);
    }
}
