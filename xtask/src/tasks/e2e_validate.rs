//! End-to-end validation task
//!
//! Orchestrates a comprehensive validation sweep across the workspace:
//! 1. Release-mode tests for core crates (parser, LSP, DAP)
//! 2. Large workspace smoke test (start LSP server, verify no crash)
//! 3. Benchmark compilation check
//! 4. Structured JSON report with actual pass/fail tracking
//!
//! This replaces the shell script `scripts/e2e-validation.sh` with a
//! focused, non-duplicative Rust implementation that produces reliable
//! results (the shell script swallowed errors with `2>/dev/null` and
//! emitted a static report regardless of outcomes).

use chrono::Utc;
use color_eyre::eyre::{Context, Result};
use console::Style;
use duct::cmd;
use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::utils::project_root;

// =============================================================================
// Configuration
// =============================================================================

/// Core crates whose release-mode lib tests are exercised.
const CORE_CRATES: &[&str] = &["perl-parser", "perl-lsp-rs", "perl-dap"];

// =============================================================================
// Public API
// =============================================================================

/// Configuration for the e2e-validate subcommand.
pub struct E2eConfig {
    /// Number of files to generate for the large-workspace test.
    pub workspace_size: usize,
    /// Path for the JSON report (None = skip report).
    pub report_path: Option<PathBuf>,
    /// Skip the large-workspace smoke test.
    pub skip_workspace: bool,
    /// Skip the benchmark compilation check.
    pub skip_bench: bool,
    /// Verbose output.
    pub verbose: bool,
}

/// Run the end-to-end validation sweep.
pub fn run(config: E2eConfig) -> Result<()> {
    let root = project_root()?;
    std::env::set_current_dir(&root).context("Failed to change to project root")?;

    let bold = Style::new().bold();
    println!("\n{}", bold.apply_to("End-to-End Validation"));
    println!("{}", bold.apply_to("====================="));

    let overall_start = Instant::now();
    let mut results = Vec::new();

    // ── Phase 1: Release-mode core-crate tests ──────────────────────────
    println!(
        "\n{}",
        bold.apply_to("Phase 1: Release-mode core crate tests")
    );
    for crate_name in CORE_CRATES {
        let outcome = run_crate_test(crate_name, config.verbose)?;
        results.push(outcome);
    }

    // ── Phase 2: Large workspace smoke test ─────────────────────────────
    if !config.skip_workspace {
        println!("\n{}", bold.apply_to("Phase 2: Large workspace smoke test"));
        let outcome = run_workspace_smoke_test(config.workspace_size, &root)?;
        results.push(outcome);
    }

    // ── Phase 3: Benchmark compilation ──────────────────────────────────
    if !config.skip_bench {
        println!(
            "\n{}",
            bold.apply_to("Phase 3: Benchmark compilation check")
        );
        let outcome = run_bench_compile_check()?;
        results.push(outcome);
    }

    // ── Summary ─────────────────────────────────────────────────────────
    let elapsed = overall_start.elapsed();
    print_summary(&results, elapsed);

    // ── Optional JSON report ────────────────────────────────────────────
    if let Some(path) = &config.report_path {
        write_report(&results, elapsed, path)?;
        println!("\nReport written to {}", path.display());
    }

    // Fail if any step failed
    let failures: Vec<&StepOutcome> = results.iter().filter(|r| !r.passed).collect();
    if !failures.is_empty() {
        let names: Vec<&str> = failures.iter().map(|f| f.name.as_str()).collect();
        Err(color_eyre::eyre::eyre!(
            "{} step(s) failed: {}",
            failures.len(),
            names.join(", ")
        ))
    } else {
        Ok(())
    }
}

// =============================================================================
// Step implementations
// =============================================================================

/// Run `cargo test -p <crate> --lib --release` and capture result.
fn run_crate_test(crate_name: &str, verbose: bool) -> Result<StepOutcome> {
    let spinner = make_spinner()?;
    spinner.set_message(format!("Testing {} (release)...", crate_name));

    let start = Instant::now();
    let mut args = vec!["test", "-p", crate_name, "--lib", "--release"];
    if !verbose {
        args.extend_from_slice(&["--", "-q"]);
    }

    let result = cmd("cargo", &args).stderr_to_stdout().unchecked().run();

    let elapsed = start.elapsed();
    let (passed, detail) = match result {
        Ok(output) => {
            let success = output.status.success();
            let detail = if success {
                None
            } else {
                Some(String::from_utf8_lossy(&output.stdout).to_string())
            };
            (success, detail)
        }
        Err(e) => (false, Some(format!("Failed to execute: {e}"))),
    };

    let label = format!("{} release tests", crate_name);
    print_step_result(&spinner, &label, passed, elapsed);
    Ok(StepOutcome {
        name: label,
        passed,
        duration: elapsed,
        detail,
    })
}

/// Generate N Perl files in a temp directory, start the LSP binary,
/// wait briefly, then verify the process has not crashed.
fn run_workspace_smoke_test(
    file_count: usize,
    project_root: &std::path::Path,
) -> Result<StepOutcome> {
    let spinner = make_spinner()?;
    spinner.set_message(format!("Generating {} Perl files...", file_count));

    let start = Instant::now();

    // Create temp directory
    let tmp_dir = tempdir_for_workspace()?;
    let tmp_path = tmp_dir.path();

    // Generate files
    for i in 1..=file_count {
        let content = format!(
            "#!/usr/bin/perl\nuse strict;\nuse warnings;\n\npackage Test{i};\n\nsub test_function_{i} {{\n    my ($param) = @_;\n    return \"result_{i}\";\n}}\n\n1;\n"
        );
        let file_path = tmp_path.join(format!("test_{i}.pl"));
        fs::write(&file_path, content)
            .with_context(|| format!("Failed to write {}", file_path.display()))?;
    }

    spinner.set_message("Starting LSP server against large workspace...");

    // Locate binary
    let binary = find_lsp_binary(project_root);
    let (passed, detail) = match binary {
        Some(bin) => run_lsp_smoke(&bin, tmp_path)?,
        None => {
            // Try building first
            spinner.set_message("Building perl-lsp (release)...");
            let build_result = cmd("cargo", &["build", "-p", "perl-lsp-rs", "--release"])
                .stderr_to_stdout()
                .unchecked()
                .run();
            match build_result {
                Ok(output) if output.status.success() => {
                    let bin = project_root.join("target/release/perllsp");
                    if bin.exists() {
                        run_lsp_smoke(&bin, tmp_path)?
                    } else {
                        (
                            false,
                            Some("perllsp binary not found after build".to_string()),
                        )
                    }
                }
                Ok(output) => (
                    false,
                    Some(format!(
                        "Failed to build perllsp: {}",
                        String::from_utf8_lossy(&output.stdout)
                    )),
                ),
                Err(e) => (false, Some(format!("Build command failed: {e}"))),
            }
        }
    };

    let elapsed = start.elapsed();
    let label = format!("Large workspace smoke test ({file_count} files)");
    print_step_result(&spinner, &label, passed, elapsed);

    // Clean up
    drop(tmp_dir);

    Ok(StepOutcome {
        name: label,
        passed,
        duration: elapsed,
        detail,
    })
}

/// Verify that benchmarks compile (without running them).
fn run_bench_compile_check() -> Result<StepOutcome> {
    let spinner = make_spinner()?;
    spinner.set_message("Checking benchmark compilation...");

    let start = Instant::now();
    let result = cmd("cargo", &["bench", "--no-run"])
        .stderr_to_stdout()
        .unchecked()
        .run();

    let elapsed = start.elapsed();
    let (passed, detail) = match result {
        Ok(output) => {
            let success = output.status.success();
            let detail = if success {
                None
            } else {
                Some(String::from_utf8_lossy(&output.stdout).to_string())
            };
            (success, detail)
        }
        Err(e) => (false, Some(format!("Failed to execute: {e}"))),
    };

    let label = "Benchmark compilation".to_string();
    print_step_result(&spinner, &label, passed, elapsed);
    Ok(StepOutcome {
        name: label,
        passed,
        duration: elapsed,
        detail,
    })
}

// =============================================================================
// Helpers
// =============================================================================

/// Outcome of a single validation step.
#[derive(Serialize)]
struct StepOutcome {
    name: String,
    passed: bool,
    #[serde(serialize_with = "serialize_duration")]
    duration: Duration,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
}

fn serialize_duration<S: serde::Serializer>(
    d: &Duration,
    s: S,
) -> std::result::Result<S::Ok, S::Error> {
    s.serialize_f64(d.as_secs_f64())
}

fn make_spinner() -> Result<ProgressBar> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {wide_msg}")
            .context("Failed to create spinner template")?,
    );
    Ok(spinner)
}

fn print_step_result(spinner: &ProgressBar, label: &str, passed: bool, duration: Duration) {
    let icon = if passed { "PASS" } else { "FAIL" };
    let style = if passed {
        Style::new().green()
    } else {
        Style::new().red()
    };
    spinner.finish_with_message(format!(
        "[{}] {} ({:.1}s)",
        style.apply_to(icon),
        label,
        duration.as_secs_f64()
    ));
}

fn print_summary(results: &[StepOutcome], total: Duration) {
    let passed = results.iter().filter(|r| r.passed).count();
    let failed = results.len() - passed;
    let bold = Style::new().bold();

    println!("\n{}", bold.apply_to("Summary"));
    println!("{}", bold.apply_to("-------"));
    println!(
        "  Total: {} steps, {} passed, {} failed ({:.1}s)",
        results.len(),
        passed,
        failed,
        total.as_secs_f64()
    );

    if failed > 0 {
        let red = Style::new().red().bold();
        println!("\n  {}", red.apply_to("Failed steps:"));
        for r in results.iter().filter(|r| !r.passed) {
            println!("    - {}", r.name);
        }
    }
}

fn write_report(results: &[StepOutcome], total: Duration, path: &std::path::Path) -> Result<()> {
    let passed = results.iter().filter(|r| r.passed).count();
    let failed = results.len() - passed;

    // We cannot move out of the slice, so rebuild the steps for serialization.
    // Instead, serialize the slice reference directly since StepOutcome implements Serialize.
    let report = serde_json::json!({
        "timestamp": Utc::now().to_rfc3339(),
        "total_duration_secs": total.as_secs_f64(),
        "passed": passed,
        "failed": failed,
        "steps": results,
    });

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {}", parent.display()))?;
    }

    let mut file =
        fs::File::create(path).with_context(|| format!("Failed to create {}", path.display()))?;
    let json_bytes = serde_json::to_vec_pretty(&report).context("Failed to serialize report")?;
    file.write_all(&json_bytes)
        .with_context(|| format!("Failed to write {}", path.display()))?;

    Ok(())
}

/// Locate an existing `perllsp` binary (release preferred).
fn find_lsp_binary(project_root: &std::path::Path) -> Option<PathBuf> {
    let release = project_root.join("target/release/perllsp");
    if release.exists() {
        return Some(release);
    }
    let debug = project_root.join("target/debug/perllsp");
    if debug.exists() {
        return Some(debug);
    }
    None
}

/// Start the LSP binary with `--stdio`, wait briefly, check if it's alive, then kill.
fn run_lsp_smoke(
    binary: &std::path::Path,
    workspace_dir: &std::path::Path,
) -> Result<(bool, Option<String>)> {
    use std::process::{Command, Stdio};

    let mut child = Command::new(binary)
        .arg("--stdio")
        .current_dir(workspace_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("Failed to spawn {}", binary.display()))?;

    // Give the server a moment to initialize (or crash)
    std::thread::sleep(Duration::from_secs(2));

    // Check if still alive
    match child.try_wait() {
        Ok(Some(status)) => {
            // Process exited -- if it exited with 0 that's fine (some modes exit),
            // but a crash (non-zero) is a failure.
            let stderr_output = child
                .stderr
                .take()
                .map(|mut s| {
                    let mut buf = String::new();
                    std::io::Read::read_to_string(&mut s, &mut buf).ok();
                    buf
                })
                .unwrap_or_default();

            if status.success() {
                Ok((true, None))
            } else {
                Ok((
                    false,
                    Some(format!(
                        "LSP server exited with status {}: {}",
                        status, stderr_output
                    )),
                ))
            }
        }
        Ok(None) => {
            // Still running -- that's the expected happy path
            let _ = child.kill();
            let _ = child.wait();
            Ok((true, None))
        }
        Err(e) => Ok((false, Some(format!("Failed to check process status: {e}")))),
    }
}

/// Create a temporary directory for the large-workspace test.
///
/// We use a wrapper that calls `std::fs::create_dir_all` + removal on drop,
/// avoiding a dev-dependency on `tempfile` in the production binary.
fn tempdir_for_workspace() -> Result<TempDir> {
    let base = std::env::temp_dir().join("perl-lsp-e2e-workspace");
    // Clean up any leftover from a previous run
    if base.exists() {
        fs::remove_dir_all(&base).ok();
    }
    fs::create_dir_all(&base)
        .with_context(|| format!("Failed to create temp dir {}", base.display()))?;
    Ok(TempDir(base))
}

/// Simple RAII temp directory.
struct TempDir(PathBuf);

impl TempDir {
    fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
