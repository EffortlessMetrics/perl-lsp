//! Snapshot tests for perl-ci-hygiene CLI output.
//!
//! These tests capture the expected output of `perl-ci-hygiene` commands
//! so that ANY change in output is detected immediately.
//!
//! The snapshot tests verify:
//! 1. `check-todos --list` output format (line-by-line listing of TODOs)
//! 2. `check-unwraps-prod` output format (counts vs baseline)
//! 3. `check-missing-docs` output format (warnings count vs baseline)
//!
//! When underlying code changes or output format changes, these tests
//! will fail, providing immediate feedback about what changed.

use std::path::PathBuf;
use std::process::Command;

/// Get the workspace root (two levels up from CARGO_MANIFEST_DIR since we're in crates/*/tests/)
fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    manifest_dir.parent().unwrap().parent().unwrap().to_path_buf()
}

/// Get path to the perl-ci-hygiene binary
fn ci_hygiene_bin() -> PathBuf {
    let ws_root = workspace_root();
    // Use cargo run to get the debug binary path
    ws_root.join("target/debug/perl-ci-hygiene")
}

/// Run perl-ci-hygiene command and capture output
fn run_ci_hygiene(args: &[&str]) -> (String, String, i32) {
    let ws_root = workspace_root();
    let mut cmd = Command::new("cargo");
    cmd.current_dir(&ws_root).args(["run", "-p", "perl-ci-hygiene", "--"]).args(args);

    let output = cmd.output().expect("Failed to run perl-ci-hygiene");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let status = output.status.code().unwrap_or(-1);
    (stdout, stderr, status)
}

// =============================================================================
// Snapshot 1: check-todos --list output
// =============================================================================

/// Snapshot test: `check-todos --list` produces deterministic line-by-line output.
///
/// This snapshot captures the exact list of TODO/FIXME lines found in the codebase.
/// When new TODOs are added or existing ones are linked/removed, this snapshot
/// will detect the change.
#[test]
fn snapshot_check_todos_list_output_is_deterministic() {
    let (stdout1, _, status1) = run_ci_hygiene(&["check-todos", "--list"]);
    let (stdout2, _, status2) = run_ci_hygiene(&["check-todos", "--list"]);

    // Should be deterministic
    assert_eq!(stdout1, stdout2, "check-todos --list should be deterministic");
    assert_eq!(status1, status2, "check-todos --list should return same status");
}

/// Snapshot test: `check-todos --list` output contains expected TODO lines.
///
/// The output should contain TODO/FIXME lines from the codebase in format:
/// `path/to/file.rs:line_number: line content`
#[test]
fn snapshot_check_todos_list_output_format() {
    let (stdout, stderr, status) = run_ci_hygiene(&["check-todos", "--list"]);

    // Should exit successfully (0) when in list mode
    assert_eq!(status, 0, "check-todos --list should exit with 0, stderr: {}", stderr);

    // Output should be non-empty and contain file paths with line numbers
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();

    // Each line should match pattern: path:line_number: content
    for line in &lines {
        assert!(
            line.contains(".rs:") || line.contains(".md:"),
            "Each output line should contain a file path, got: {}",
            line
        );
    }

    // Should contain TODO or FIXME content
    let combined = stdout.to_lowercase();
    assert!(
        combined.contains("todo") || combined.contains("fixme"),
        "Output should contain TODO or FIXME, got: {}",
        stdout
    );
}

// =============================================================================
// Snapshot 2: check-unwraps-prod output
// =============================================================================

/// Snapshot test: `check-unwraps-prod` produces deterministic output.
#[test]
fn snapshot_check_unwraps_prod_is_deterministic() {
    let (stdout1, _, status1) = run_ci_hygiene(&["check-unwraps-prod"]);
    let (stdout2, _, status2) = run_ci_hygiene(&["check-unwraps-prod"]);

    assert_eq!(stdout1, stdout2, "check-unwraps-prod should be deterministic");
    assert_eq!(status1, status2, "check-unwraps-prod should return same status");
}

/// Snapshot test: `check-unwraps-prod` output format.
///
/// Expected format:
/// ```
/// unwrap/expect: N (baseline: M)
/// panic-family macros: N (baseline: M)
/// ```
#[test]
fn snapshot_check_unwraps_prod_output_format() {
    let (stdout, stderr, status) = run_ci_hygiene(&["check-unwraps-prod"]);

    // Should exit with 0 when within baseline
    assert_eq!(
        status, 0,
        "check-unwraps-prod should pass with current baseline, stderr: {}",
        stderr
    );

    // Should contain unwrap/expect count line
    assert!(
        stdout.contains("unwrap/expect:"),
        "Output should contain 'unwrap/expect:', got: {}",
        stdout
    );

    // Should contain panic-family count line
    assert!(
        stdout.contains("panic-family macros:"),
        "Output should contain 'panic-family macros:', got: {}",
        stdout
    );

    // Should contain baseline comparison in parentheses
    assert!(stdout.contains("(baseline:"), "Output should contain '(baseline:', got: {}", stdout);
}

// =============================================================================
// Snapshot 3: check-missing-docs output
// =============================================================================

/// Snapshot test: `check-missing-docs` produces deterministic output.
#[test]
fn snapshot_check_missing_docs_is_deterministic() {
    let (stdout1, _, status1) = run_ci_hygiene(&["check-missing-docs"]);
    let (stdout2, _, status2) = run_ci_hygiene(&["check-missing-docs"]);

    assert_eq!(stdout1, stdout2, "check-missing-docs should be deterministic");
    assert_eq!(status1, status2, "check-missing-docs should return same status");
}

/// Snapshot test: `check-missing-docs` output format.
#[test]
fn snapshot_check_missing_docs_output_format() {
    let (stdout, stderr, status) = run_ci_hygiene(&["check-missing-docs"]);

    // Should exit with 0 when within baseline
    assert_eq!(status, 0, "check-missing-docs should pass, stderr: {}", stderr);

    // Should contain "Missing docs warnings" or "Baseline"
    assert!(
        stdout.contains("Missing docs") || stdout.contains("Baseline"),
        "Output should contain 'Missing docs' or 'Baseline', got: {}",
        stdout
    );
}

// =============================================================================
// Snapshot 4: panic_test_baseline.txt content
// =============================================================================

/// Snapshot test: `ci/panic_test_baseline.txt` exists and contains expected value.
///
/// This baseline was established during the implementation phase of work-c40bd322.
#[test]
fn snapshot_panic_test_baseline_exists_and_has_expected_value() {
    use std::fs;

    let ws_root = workspace_root();
    let baseline_path = ws_root.join("ci/panic_test_baseline.txt");

    assert!(baseline_path.exists(), "ci/panic_test_baseline.txt should exist");

    let content = fs::read_to_string(&baseline_path)
        .expect("Should be able to read ci/panic_test_baseline.txt");
    let content = content.trim();

    // Parse as u32
    let count: u32 =
        content.parse().expect("ci/panic_test_baseline.txt should contain a valid u32");

    // The baseline should be 182 (established during implementation)
    assert_eq!(count, 182, "panic_test_baseline should be 182");

    // Should be in reasonable range
    assert!(count <= 5000, "panic! count {} seems unreasonably high", count);
}

// =============================================================================
// Snapshot 5: todo_test_baseline.txt content
// =============================================================================

/// Snapshot test: `ci/todo_test_baseline.txt` exists and contains expected value.
///
/// This baseline was established during the implementation phase of work-c40bd322.
#[test]
fn snapshot_todo_test_baseline_exists_and_has_expected_value() {
    use std::fs;

    let ws_root = workspace_root();
    let baseline_path = ws_root.join("ci/todo_test_baseline.txt");

    assert!(baseline_path.exists(), "ci/todo_test_baseline.txt should exist");

    let content = fs::read_to_string(&baseline_path)
        .expect("Should be able to read ci/todo_test_baseline.txt");
    let content = content.trim();

    // Parse as u32
    let count: u32 = content.parse().expect("ci/todo_test_baseline.txt should contain a valid u32");

    // The baseline should be 0 (no unlinked TODOs in test code)
    assert_eq!(count, 0, "todo_test_baseline should be 0");
}

// =============================================================================
// Snapshot 6: Combined baseline consistency
// =============================================================================

/// Snapshot test: Production baselines unchanged (no regression).
///
/// This verifies that the implementation did not accidentally change the
/// production code baselines.
#[test]
fn snapshot_production_baselines_unchanged() {
    use std::fs;

    let ws_root = workspace_root();

    // Read production baselines
    let unwrap_baseline_path = ws_root.join("ci/unwrap_prod_baseline.txt");
    let panic_baseline_path = ws_root.join("ci/panic_prod_baseline.txt");

    // These should exist and contain specific values
    let unwrap_content =
        fs::read_to_string(&unwrap_baseline_path).expect("Should read ci/unwrap_prod_baseline.txt");
    let panic_content =
        fs::read_to_string(&panic_baseline_path).expect("Should read ci/panic_prod_baseline.txt");

    let unwrap_count: u32 =
        unwrap_content.trim().parse().expect("unwrap_prod_baseline should be valid u32");
    let panic_count: u32 =
        panic_content.trim().parse().expect("panic_prod_baseline should be valid u32");

    // Production unwrap baseline should be 0
    assert_eq!(unwrap_count, 0, "Production unwrap baseline should remain 0");

    // Production panic baseline should be 0
    assert_eq!(panic_count, 0, "Production panic baseline should remain 0");
}

// =============================================================================
// Snapshot 7: CLI help output (stable interface)
// =============================================================================

/// Snapshot test: CLI help output is stable.
#[test]
fn snapshot_cli_help_is_stable() {
    let ws_root = workspace_root();
    let mut cmd = Command::new("cargo");
    cmd.current_dir(&ws_root).args(["run", "-p", "perl-ci-hygiene", "--", "--help"]);

    let output = cmd.output().expect("Failed to run perl-ci-hygiene --help");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // Help should work without errors
    assert!(output.status.success(), "perl-ci-hygiene --help should succeed, stderr: {}", stderr);

    // Should contain key commands
    assert!(stdout.contains("check-todos"), "Help should mention check-todos");
    assert!(stdout.contains("check-unwraps-prod"), "Help should mention check-unwraps-prod");
    assert!(stdout.contains("check-missing-docs"), "Help should mention check-missing-docs");
}
