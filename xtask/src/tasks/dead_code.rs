//! Dead code detection for the perl-lsp workspace
//!
//! Combines cargo-machete (unused dependencies), cargo-udeps (deep unused dep detection),
//! and clippy dead_code lints. Supports three modes:
//! - `check`: Compare current state against baseline thresholds
//! - `baseline`: Generate a new baseline YAML file
//! - `report`: Generate a JSON report for CI integration

use std::fs;
use std::io::BufRead;
use std::path::{Path, PathBuf};

use chrono::Utc;
use color_eyre::eyre::{Context, Result, eyre};
use duct::cmd;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// The mode in which to run dead code detection.
#[derive(Clone, Debug, clap::ValueEnum)]
pub enum DeadCodeMode {
    /// Check current state against baseline (default)
    Check,
    /// Generate a new baseline YAML
    Baseline,
    /// Generate a JSON report for CI
    Report,
}

/// Configuration for the dead-code subcommand.
pub struct DeadCodeConfig {
    pub mode: DeadCodeMode,
    pub strict: bool,
}

/// Entry point called from main.
pub fn run(config: DeadCodeConfig) -> Result<()> {
    let root = crate::utils::project_root()?;
    std::env::set_current_dir(&root).context("Failed to change to project root")?;

    println!("[INFO] Dead Code Detection (mode: {:?})", config.mode);
    println!();

    check_tools()?;

    match config.mode {
        DeadCodeMode::Check => run_check(&root, config.strict),
        DeadCodeMode::Baseline => run_baseline(&root),
        DeadCodeMode::Report => run_report(&root),
    }
}

// ---------------------------------------------------------------------------
// Baseline YAML model (read-only — we write it as a template string)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct BaselineFile {
    #[serde(default)]
    thresholds: Thresholds,
    #[serde(default)]
    baseline: BaselineCounts,
}

#[derive(Debug, Deserialize)]
struct Thresholds {
    #[serde(default = "default_max_5")]
    max_unused_dependencies: u64,
    #[serde(default = "default_max_10")]
    max_dead_code_items: u64,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            max_unused_dependencies: default_max_5(),
            max_dead_code_items: default_max_10(),
        }
    }
}

fn default_max_5() -> u64 {
    5
}
fn default_max_10() -> u64 {
    10
}

/// Baseline counts parsed from the YAML file.
///
/// All four fields are deserialized for completeness. The check phase currently
/// compares only `unused_dependencies` and `dead_code_items` against thresholds.
#[derive(Debug, Default, Deserialize)]
#[allow(dead_code)]
struct BaselineCounts {
    #[serde(default)]
    unused_dependencies: u64,
    #[serde(default)]
    dead_code_items: u64,
    #[serde(default)]
    unused_imports: u64,
    #[serde(default)]
    unused_variables: u64,
}

// ---------------------------------------------------------------------------
// JSON report model
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct JsonReport {
    schema_version: u32,
    timestamp: String,
    results: JsonReportResults,
    details: JsonReportDetails,
}

#[derive(Serialize)]
struct JsonReportResults {
    unused_dependencies: u64,
    dead_code_items: u64,
    unused_imports: u64,
    unused_variables: u64,
    total_issues: u64,
}

#[derive(Serialize)]
struct JsonReportDetails {
    udeps_output: String,
    clippy_output: String,
}

// ---------------------------------------------------------------------------
// Tool checks
// ---------------------------------------------------------------------------

fn check_tools() -> Result<()> {
    // cargo must be on PATH — if it isn't, duct will fail anyway, but give a
    // better error message.
    cmd("cargo", ["--version"])
        .stdout_null()
        .stderr_null()
        .run()
        .context("cargo is not available on PATH")?;

    // Check that nightly toolchain exists (needed for cargo-udeps)
    let rustup_output = cmd("rustup", ["toolchain", "list"])
        .read()
        .unwrap_or_default();

    if !rustup_output.contains("nightly") {
        println!("[WARN] Nightly toolchain not installed (required for cargo-udeps)");
        println!("[INFO] Install with: rustup toolchain install nightly");
    }

    Ok(())
}

fn ensure_udeps_installed() -> Result<()> {
    let probe = cmd("cargo", ["+nightly", "udeps", "--version"])
        .stdout_null()
        .stderr_null()
        .run();

    if probe.is_err() {
        println!("[INFO] Installing cargo-udeps...");
        cmd("cargo", ["install", "cargo-udeps", "--locked"])
            .run()
            .context("Failed to install cargo-udeps")?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Running external tools and counting output
// ---------------------------------------------------------------------------

/// Ensure the `target/dead-code/` directory exists and return its path.
fn output_dir(root: &Path) -> Result<PathBuf> {
    let dir = root.join("target/dead-code");
    fs::create_dir_all(&dir).context("Failed to create target/dead-code directory")?;
    Ok(dir)
}

/// Run cargo-machete and return (exit_ok, output_path).
fn run_machete(root: &Path) -> Result<(bool, PathBuf)> {
    let dir = output_dir(root)?;
    let out_path = dir.join("machete-output.txt");

    println!("[INFO] Checking for unused dependencies with cargo-machete...");

    let result = cmd("cargo", ["machete"])
        .stdout_capture()
        .stderr_capture()
        .unchecked()
        .run()
        .context("Failed to run cargo-machete")?;

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    fs::write(&out_path, &combined)
        .with_context(|| format!("Failed to write {}", out_path.display()))?;

    // Print the output so users see it
    print!("{combined}");

    let crate_count = count_occurrences(&combined, "Cargo.toml:");
    if crate_count > 0 {
        println!("[WARN] Found {crate_count} crates with unused dependencies");
    } else {
        println!("[SUCCESS] No unused dependencies detected by machete");
    }

    Ok((result.status.success(), out_path))
}

/// Run cargo-udeps and write output. Returns output path.
fn run_udeps(root: &Path) -> Result<PathBuf> {
    let dir = output_dir(root)?;
    let out_path = dir.join("udeps-output.txt");

    println!("[INFO] Checking for unused dependencies with cargo-udeps...");

    let result = cmd(
        "cargo",
        [
            "+nightly",
            "udeps",
            "--workspace",
            "--all-targets",
            "--locked",
        ],
    )
    .stdout_capture()
    .stderr_capture()
    .unchecked()
    .run()
    .context("Failed to run cargo-udeps")?;

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    fs::write(&out_path, &combined)
        .with_context(|| format!("Failed to write {}", out_path.display()))?;

    Ok(out_path)
}

/// Run clippy with dead-code lints and write output. Returns output path.
fn run_clippy_dead_code(root: &Path) -> Result<PathBuf> {
    let dir = output_dir(root)?;
    let out_path = dir.join("clippy-dead-code.txt");

    println!("[INFO] Checking for dead code with clippy...");

    let result = cmd(
        "cargo",
        [
            "clippy",
            "--workspace",
            "--lib",
            "--bins",
            "--locked",
            "--",
            "-W",
            "dead_code",
            "-W",
            "unused_imports",
            "-W",
            "unused_variables",
        ],
    )
    .stdout_capture()
    .stderr_capture()
    .unchecked()
    .run()
    .context("Failed to run clippy")?;

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    fs::write(&out_path, &combined)
        .with_context(|| format!("Failed to write {}", out_path.display()))?;

    Ok(out_path)
}

// ---------------------------------------------------------------------------
// Counting helpers
// ---------------------------------------------------------------------------

/// Count how many lines in `text` contain `needle`.
fn count_occurrences(text: &str, needle: &str) -> u64 {
    text.as_bytes()
        .lines()
        .map_while(Result::ok)
        .filter(|line| line.contains(needle))
        .count() as u64
}

/// Count occurrences of `needle` in a file. Returns 0 if the file cannot be read.
fn count_in_file(path: &Path, needle: &str) -> u64 {
    match fs::read_to_string(path) {
        Ok(content) => count_occurrences(&content, needle),
        Err(_) => 0,
    }
}

// ---------------------------------------------------------------------------
// Modes
// ---------------------------------------------------------------------------

fn run_check(root: &Path, strict: bool) -> Result<()> {
    let baseline_path = root.join(".ci/dead-code-baseline.yaml");

    if baseline_path.exists() {
        check_against_baseline(root, &baseline_path, strict)
    } else {
        println!("[INFO] No baseline file, running basic checks...");
        run_machete(root)?;
        run_clippy_dead_code(root)?;
        println!("[SUCCESS] Basic dead code checks completed");
        Ok(())
    }
}

fn check_against_baseline(root: &Path, baseline_path: &Path, strict: bool) -> Result<()> {
    println!("[INFO] Checking against baseline...");

    // Parse baseline
    let baseline_content = fs::read_to_string(baseline_path)
        .with_context(|| format!("Failed to read {}", baseline_path.display()))?;
    let baseline_file: BaselineFile = serde_yaml_ng::from_str(&baseline_content)
        .with_context(|| format!("Failed to parse {}", baseline_path.display()))?;

    // Run tools and capture output files
    let udeps_path = run_udeps(root)?;
    let clippy_path = run_clippy_dead_code(root)?;

    // Count current issues
    let current_unused_deps = count_in_file(&udeps_path, "unused");
    let current_dead_code = count_in_file(&clippy_path, "dead_code");

    let bl = &baseline_file.baseline;
    let th = &baseline_file.thresholds;

    println!("[INFO] Comparison:");
    println!(
        "  Unused dependencies: {} (baseline: {}, max: {})",
        current_unused_deps, bl.unused_dependencies, th.max_unused_dependencies
    );
    println!(
        "  Dead code items:     {} (baseline: {}, max: {})",
        current_dead_code, bl.dead_code_items, th.max_dead_code_items
    );

    let mut failed = false;

    // Check thresholds
    if current_unused_deps > th.max_unused_dependencies {
        println!(
            "[ERROR] Unused dependencies ({}) exceeds threshold ({})",
            current_unused_deps, th.max_unused_dependencies
        );
        failed = true;
    }
    if current_dead_code > th.max_dead_code_items {
        println!(
            "[ERROR] Dead code items ({}) exceeds threshold ({})",
            current_dead_code, th.max_dead_code_items
        );
        failed = true;
    }

    // Check regression against baseline
    if current_unused_deps > bl.unused_dependencies {
        println!(
            "[WARN] Unused dependencies increased from {} to {}",
            bl.unused_dependencies, current_unused_deps
        );
        if strict {
            failed = true;
        }
    }
    if current_dead_code > bl.dead_code_items {
        println!(
            "[WARN] Dead code increased from {} to {}",
            bl.dead_code_items, current_dead_code
        );
        if strict {
            failed = true;
        }
    }

    if failed {
        Err(eyre!("Dead code checks failed"))
    } else {
        println!("[SUCCESS] Dead code checks passed");
        Ok(())
    }
}

fn run_baseline(root: &Path) -> Result<()> {
    println!("[INFO] Generating dead code baseline...");

    ensure_udeps_installed()?;

    let udeps_path = run_udeps(root)?;
    let clippy_path = run_clippy_dead_code(root)?;

    let unused_deps = count_in_file(&udeps_path, "unused");
    let dead_code = count_in_file(&clippy_path, "dead_code");
    let unused_imports = count_in_file(&clippy_path, "unused_imports");
    let unused_vars = count_in_file(&clippy_path, "unused_variables");

    let now = Utc::now();
    let date_str = now.format("%Y-%m-%d").to_string();
    let datetime_str = now.format("%Y-%m-%d %H:%M:%S UTC").to_string();

    // Compute next review date (30 days from now)
    let next_review = now + chrono::Duration::days(30);
    let next_review_str = next_review.format("%Y-%m-%d").to_string();

    let baseline_content = format!(
        r##"# Dead Code Detection Baseline
# Generated: {datetime_str}
#
# This file tracks the baseline for dead code detection.
# When new dead code is introduced, the checks will fail.
#
# To update this baseline: just dead-code-baseline

schema_version: 1
last_updated: "{date_str}"

# Thresholds (fail if exceeded)
thresholds:
  # Maximum allowed unused dependencies
  max_unused_dependencies: 5

  # Maximum allowed dead code items (functions, types, etc.)
  max_dead_code_items: 10

  # Maximum allowed unused imports
  max_unused_imports: 20

  # Maximum allowed unused variables
  max_unused_variables: 10

# Current baseline counts
baseline:
  unused_dependencies: {unused_deps}
  dead_code_items: {dead_code}
  unused_imports: {unused_imports}
  unused_variables: {unused_vars}

# Allowed exceptions (items that are intentionally unused)
allowed_exceptions:
  # Example: functions that are part of public API but not used internally
  # - crate: perl-parser
  #   type: function
  #   name: parse_legacy_syntax
  #   reason: "Public API for backward compatibility"

  # Example: dependencies used only in specific build configurations
  # - crate: perl-lsp
  #   dependency: tokio
  #   reason: "Used in async runtime feature"

# Known issues to be addressed
known_issues:
  # Track specific dead code items that need cleanup
  # - crate: perl-parser-core
  #   type: dead_code
  #   item: legacy_parser_function
  #   issue: "#XXX"
  #   notes: "Remove after migration to v3 parser"

# Policy
policy:
  # Enforcement level: strict, warn, or disabled
  enforcement: warn

  # Auto-update baseline on PR (requires manual approval)
  auto_update_baseline: false

  # Fail CI if baseline is exceeded
  fail_on_baseline_exceeded: true

  # Warn if approaching threshold (80% of max)
  warn_threshold_percent: 80

# Maintenance
maintenance:
  # Review dead code baseline every N days
  review_interval_days: 30

  # Next scheduled review
  next_review: "{next_review_str}"
"##
    );

    let baseline_path = root.join(".ci/dead-code-baseline.yaml");
    fs::create_dir_all(
        baseline_path
            .parent()
            .ok_or_else(|| eyre!("baseline path has no parent directory"))?,
    )
    .context("Failed to create .ci/ directory")?;
    fs::write(&baseline_path, baseline_content)
        .with_context(|| format!("Failed to write {}", baseline_path.display()))?;

    println!("[SUCCESS] Baseline saved to {}", baseline_path.display());
    println!();
    println!("[INFO] Current counts:");
    println!("  Unused dependencies: {unused_deps}");
    println!("  Dead code items:     {dead_code}");
    println!("  Unused imports:      {unused_imports}");
    println!("  Unused variables:    {unused_vars}");

    Ok(())
}

fn run_report(root: &Path) -> Result<()> {
    println!("[INFO] Generating JSON report...");

    ensure_udeps_installed()?;

    let udeps_path = run_udeps(root)?;
    let clippy_path = run_clippy_dead_code(root)?;

    let unused_deps = count_in_file(&udeps_path, "unused");
    let dead_code = count_in_file(&clippy_path, "dead_code");
    let unused_imports = count_in_file(&clippy_path, "unused_imports");
    let unused_vars = count_in_file(&clippy_path, "unused_variables");

    let report = JsonReport {
        schema_version: 1,
        timestamp: Utc::now().to_rfc3339(),
        results: JsonReportResults {
            unused_dependencies: unused_deps,
            dead_code_items: dead_code,
            unused_imports,
            unused_variables: unused_vars,
            total_issues: unused_deps + dead_code + unused_imports + unused_vars,
        },
        details: JsonReportDetails {
            udeps_output: udeps_path.display().to_string(),
            clippy_output: clippy_path.display().to_string(),
        },
    };

    let report_path = output_dir(root)?.join("report.json");
    let json = serde_json::to_string_pretty(&report).context("Failed to serialize report")?;
    fs::write(&report_path, &json)
        .with_context(|| format!("Failed to write {}", report_path.display()))?;

    println!("[SUCCESS] Report saved to {}", report_path.display());
    println!("{json}");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_occurrences_empty() {
        assert_eq!(count_occurrences("", "unused"), 0);
    }

    #[test]
    fn test_count_occurrences_no_match() {
        assert_eq!(count_occurrences("hello\nworld\n", "unused"), 0);
    }

    #[test]
    fn test_count_occurrences_single_match() {
        assert_eq!(
            count_occurrences("line1\nunused dep foo\nline3\n", "unused"),
            1
        );
    }

    #[test]
    fn test_count_occurrences_multiple_matches() {
        let text = "unused dep a\nok\nunused dep b\nunused dep c\n";
        assert_eq!(count_occurrences(text, "unused"), 3);
    }

    #[test]
    fn test_count_occurrences_dead_code() {
        let text = "warning: dead_code in foo\nwarning: dead_code in bar\nother line\n";
        assert_eq!(count_occurrences(text, "dead_code"), 2);
    }

    #[test]
    fn test_baseline_deserialization() -> color_eyre::eyre::Result<()> {
        let yaml = r#"
schema_version: 1
last_updated: "2026-01-28"
thresholds:
  max_unused_dependencies: 5
  max_dead_code_items: 10
baseline:
  unused_dependencies: 2
  dead_code_items: 3
  unused_imports: 4
  unused_variables: 1
"#;
        let parsed: BaselineFile = serde_yaml_ng::from_str(yaml)?;
        assert_eq!(parsed.thresholds.max_unused_dependencies, 5);
        assert_eq!(parsed.thresholds.max_dead_code_items, 10);
        assert_eq!(parsed.baseline.unused_dependencies, 2);
        assert_eq!(parsed.baseline.dead_code_items, 3);
        assert_eq!(parsed.baseline.unused_imports, 4);
        assert_eq!(parsed.baseline.unused_variables, 1);
        Ok(())
    }

    #[test]
    fn test_baseline_deserialization_defaults() -> color_eyre::eyre::Result<()> {
        let yaml = r#"
schema_version: 1
"#;
        let parsed: BaselineFile = serde_yaml_ng::from_str(yaml)?;
        assert_eq!(parsed.thresholds.max_unused_dependencies, 5);
        assert_eq!(parsed.thresholds.max_dead_code_items, 10);
        assert_eq!(parsed.baseline.unused_dependencies, 0);
        assert_eq!(parsed.baseline.dead_code_items, 0);
        Ok(())
    }

    #[test]
    fn test_count_in_file_missing_file() {
        assert_eq!(count_in_file(Path::new("/nonexistent/file.txt"), "foo"), 0);
    }
}
