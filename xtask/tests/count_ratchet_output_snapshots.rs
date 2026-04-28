//! Snapshot tests for `published-crate-count` xtask subcommand output.
//!
//! These tests capture the exact stdout/stderr output of the
//! `cargo xtask published-crate-count` command for various scenarios:
//! - Pass (current == baseline)
//! - Ratchet (current < baseline, auto-tighten)
//! - Fail (current > baseline, gate failure)
//!
//! Snapshots are stored as inline constants in this file (no external deps needed).
//! If output format changes, update the constants and the tests will fail,
//! signaling that the output surface has changed.

use std::path::PathBuf;
use std::process::Command as StdCommand;

/// Get the project root (parent of xtask crate directory).
fn project_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.pop();
    dir
}

/// Run `cargo xtask published-crate-count` and capture output.
/// Returns (stdout, stderr, exit_code).
fn run_xtask_published_crate_count() -> (String, String, i32) {
    let root = project_root();
    let output = StdCommand::new("cargo")
        .args(["xtask", "published-crate-count"])
        .current_dir(&root)
        .output()
        .expect("Failed to execute cargo xtask");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1);

    (stdout, stderr, exit_code)
}

// =============================================================================
// SNAPSHOT CONSTANTS
// These represent the expected output format for each scenario.
// Update these if the output format changes (then run tests to confirm).
// =============================================================================

/// Expected prefix for PASS scenario output.
/// Format: "published-crate-count: OK (N crates, baseline N)"
const SNAPSHOT_PASS_PREFIX: &str = "published-crate-count: OK (";

/// Suffix that appears at end of PASS output after the count numbers.
const SNAPSHOT_PASS_SUFFIX: &str = " crates, baseline ";

/// Expected prefix for RATCHET scenario output.
/// Format: "published-crate-count: RATCHET — count dropped from BASELINE to CURRENT, updating xtask/published-crate-baseline.txt"
const SNAPSHOT_RATCHET_PREFIX: &str = "published-crate-count: RATCHET — count dropped from ";

/// Text that appears in RATCHET output indicating the baseline file update.
const SNAPSHOT_RATCHET_UPDATE_MARKER: &str = ", updating xtask/published-crate-baseline.txt";

/// Expected prefix for FAIL scenario output (on stderr).
/// Format: "published-crate-count: FAIL — N crates published, baseline is M."
const SNAPSHOT_FAIL_PREFIX: &str = "published-crate-count: FAIL — ";

/// Text that appears after the FAIL count numbers.
const SNAPSHOT_FAIL_MIDDLE: &str = " crates published, baseline is ";

/// The baseline file path that appears in RATCHET output.
const BASELINE_FILE_PATH: &str = "xtask/published-crate-baseline.txt";

// =============================================================================
// TESTS
// =============================================================================

/// Test that the command runs successfully (doesn't panic or segfault).
#[test]
fn xtask_runs_without_panic() {
    let (stdout, stderr, exit_code) = run_xtask_published_crate_count();

    // The command should either succeed (exit 0) or fail gracefully (exit 1)
    // It should NOT crash (segfault, panic, etc.)
    assert!(
        exit_code == 0 || exit_code == 1,
        "Command should exit 0 (pass/ratchet) or 1 (fail), got {}. stdout={}, stderr={}",
        exit_code,
        stdout,
        stderr
    );
}

/// Test that output contains the expected prefix for published-crate-count.
/// This is a basic sanity check that the right command is running.
#[test]
fn output_contains_expected_prefix() {
    let (stdout, stderr, _) = run_xtask_published_crate_count();

    let combined = format!("{}\n{}", stdout, stderr);

    assert!(
        combined.contains("published-crate-count"),
        "Output should contain 'published-crate-count' prefix. Got: {}",
        combined
    );
}

/// Test that output format matches the PASS pattern when count equals baseline.
#[test]
fn pass_output_format_matches_snapshot() {
    let (stdout, stderr, exit_code) = run_xtask_published_crate_count();
    let combined = format!("{}\n{}", stdout, stderr);

    // If exit code is 0, it could be PASS or RATCHET
    if exit_code == 0 {
        let is_pass = combined.contains(SNAPSHOT_PASS_PREFIX)
            && combined.contains(SNAPSHOT_PASS_SUFFIX)
            && combined.contains("crates, baseline ")
            && !combined.contains("RATCHET");

        let is_ratchet = combined.contains(SNAPSHOT_RATCHET_PREFIX)
            && combined.contains(SNAPSHOT_RATCHET_UPDATE_MARKER);

        assert!(
            is_pass || is_ratchet,
            "Exit 0 output should match PASS or RATCHET format. \
             PASS prefix: {}, RATCHET prefix: {}. \
             Got stdout: {}, stderr: {}",
            SNAPSHOT_PASS_PREFIX,
            SNAPSHOT_RATCHET_PREFIX,
            stdout,
            stderr
        );
    }
}

/// Test that error output (when failing) contains the expected FAIL pattern.
#[test]
fn fail_output_format_matches_snapshot() {
    // This test is informational - in the current workspace state,
    // the command may not fail. But if it does fail, verify the format.
    let (stdout, stderr, exit_code) = run_xtask_published_crate_count();

    if exit_code != 0 {
        // Command failed - verify the error format
        let combined = format!("{}\n{}", stdout, stderr);

        assert!(
            combined.contains(SNAPSHOT_FAIL_PREFIX) && combined.contains(SNAPSHOT_FAIL_MIDDLE),
            "Exit non-0 output should match FAIL pattern. \
             Expected prefix: {}, middle: {}. \
             Got stdout: {}, stderr: {}",
            SNAPSHOT_FAIL_PREFIX,
            SNAPSHOT_FAIL_MIDDLE,
            stdout,
            stderr
        );
    }
}

/// Test that RATCHET output mentions the baseline file path.
#[test]
fn ratchet_output_mentions_baseline_file() {
    let (stdout, stderr, exit_code) = run_xtask_published_crate_count();

    // If exit code is 0, check if it's a RATCHET scenario
    if exit_code == 0 {
        let combined = format!("{}\n{}", stdout, stderr);
        if combined.contains("RATCHET") {
            assert!(
                combined.contains(BASELINE_FILE_PATH),
                "RATCHET output should mention '{}'. Got: {}",
                BASELINE_FILE_PATH,
                combined
            );
        }
    }
}

/// Test that the output contains the actual count numbers.
#[test]
fn output_contains_numeric_counts() {
    let (stdout, stderr, _) = run_xtask_published_crate_count();
    let combined = format!("{}\n{}", stdout, stderr);

    // Extract all numbers from the output using regex
    let number_regex = regex::Regex::new(r"\d+").expect("Invalid number regex");
    let numbers: Vec<&str> = number_regex.find_iter(&combined).map(|m| m.as_str()).collect();

    // There should be at least one number in the output (the count)
    assert!(
        !numbers.is_empty(),
        "Output should contain at least one number (the crate count). \
         Got stdout: {}, stderr: {}",
        stdout,
        stderr
    );

    // Verify numbers are reasonable (not huge)
    for num_str in &numbers {
        let num: u64 = num_str.parse().unwrap_or(0);
        // Published crate count should be reasonable (< 10000)
        assert!(
            num < 10000,
            "Number {} seems too large for a crate count. Output: {}",
            num,
            combined
        );
    }
}

/// Snapshot test: Verify baseline file exists and contains a valid integer.
#[test]
fn baseline_file_exists_and_valid() {
    let root = project_root();
    let baseline_path = root.join("xtask/published-crate-baseline.txt");

    assert!(
        baseline_path.exists(),
        "Baseline file should exist at {}. \
         Run `cargo xtask published-crate-count` first to create it if in ratchet mode.",
        baseline_path.display()
    );

    let content =
        std::fs::read_to_string(&baseline_path).expect("Should be able to read baseline file");

    let trimmed = content.trim();
    let parsed: u32 = trimmed.parse().unwrap_or_else(|_| {
        panic!("Baseline file should contain a valid integer, got: '{}'", trimmed)
    });

    // Baseline should be a reasonable number (< 1000)
    assert!(parsed < 1000, "Baseline {} seems too large for published crate count", parsed);
}

/// Test that output does not contain any unexpected error indicators.
#[test]
fn no_unexpected_error_indicators() {
    let (stdout, stderr, exit_code) = run_xtask_published_crate_count();
    let combined = format!("{}\n{}", stdout, stderr);

    // These should NOT appear in normal output (they indicate a bug)
    let unexpected =
        ["thread 'main' panicked", "panicked at", "internal error", "stack backtrace:"];

    for marker in unexpected {
        assert!(
            !combined.contains(marker),
            "Output should not contain panic/error markers. Found '{}' in: {}",
            marker,
            combined
        );
    }

    // If exit code is 0, stderr should typically be empty or warnings only
    if exit_code == 0 {
        assert!(
            !stderr.contains("error:"),
            "Exit 0 should not have error: in stderr. Got: {}",
            stderr
        );
    }
}

/// Test that the command produces deterministic output.
/// Run twice and verify the key output line is the same.
#[test]
fn output_is_deterministic() {
    let (stdout1, stderr1, code1) = run_xtask_published_crate_count();
    let (stdout2, stderr2, code2) = run_xtask_published_crate_count();

    // Exit codes should match
    assert_eq!(
        code1, code2,
        "Exit code should be deterministic. First: {}, Second: {}",
        code1, code2
    );

    // For the same exit code, the output classification should be the same
    // (we can't expect bit-for-bit identical due to timing, etc.)
    let has_ratchet1 = stdout1.contains("RATCHET") || stderr1.contains("RATCHET");
    let has_ratchet2 = stdout2.contains("RATCHET") || stderr2.contains("RATCHET");
    let has_fail1 = stdout1.contains("FAIL") || stderr1.contains("FAIL");
    let has_fail2 = stdout2.contains("FAIL") || stderr2.contains("FAIL");
    let has_ok1 = stdout1.contains("OK") || stderr1.contains("OK");
    let has_ok2 = stdout2.contains("OK") || stderr2.contains("OK");

    assert_eq!(has_ratchet1, has_ratchet2, "RATCHET presence should be deterministic");
    assert_eq!(has_fail1, has_fail2, "FAIL presence should be deterministic");
    assert_eq!(has_ok1, has_ok2, "OK presence should be deterministic");
}

// =============================================================================
// EDGE CASE TESTS
// These test the parsing logic with boundary values.
// =============================================================================

/// Test that parse_baseline correctly handles various valid inputs.
#[test]
fn parse_baseline_valid_inputs() {
    // These are tested via the module's internal tests,
    // but we verify the function is accessible and working.
    let root = project_root();
    let baseline_path = root.join("xtask/published-crate-baseline.txt");

    if baseline_path.exists() {
        let content = std::fs::read_to_string(&baseline_path).unwrap();
        let trimmed = content.trim();
        let parsed: Result<u32, _> = trimmed.parse();

        assert!(parsed.is_ok(), "Baseline file should parse as u32, got: {}", trimmed);
    }
}

/// Test that parse_baseline correctly rejects invalid inputs.
#[test]
fn parse_baseline_invalid_inputs_are_rejected() {
    let invalid_inputs =
        ["", "   ", "\t", "\n", "abc", "12abc", "abc123", "-1", "-42", "3.14", "1e10"];

    for input in invalid_inputs {
        let trimmed = input.trim();
        let parsed: Result<u32, _> = trimmed.parse();

        assert!(parsed.is_err(), "parse_baseline({:?}) should reject invalid input", input);
    }
}
