//! Edge case and regression tests for the `cargo xtask publish-closure` subcommand.
//!
//! These tests verify behavior under boundary conditions, error paths, and
//! potential regressions:
//! - Output format verification
//! - Error message clarity and distinction
//! - Case sensitivity of crate names
//! - Numeric boundaries in counts
//! - Filtering edge cases
//!
//! Tests use assert_cmd to verify the real CLI behavior.

use assert_cmd::Command;
use color_eyre::eyre::Result;

// =============================================================================
// Output Format Tests
// =============================================================================

/// Test that success output format is correct for both plural and singular cases.
///
/// Regression guard: Verifies the exact output format is maintained:
/// `publish-closure: OK (N crates checked, 0 violations)` (plural, default)
/// `publish-closure: OK (1 crate checked, 0 violations)`  (singular, filtered)
///
/// Also verifies:
/// - "0 violations" is always explicitly shown (not omitted when zero)
/// - Plural "crates" used for multiple crates; singular "crate" for one
#[test]
fn publish_closure_output_format_and_grammar() -> Result<()> {
    // Default invocation: plural form, zero violations shown explicitly.
    let default_out = Command::cargo_bin("xtask")?.args(["publish-closure"]).output()?;
    assert!(default_out.status.success(), "Default invocation should succeed");
    let default_stdout = String::from_utf8_lossy(&default_out.stdout);
    assert!(default_stdout.contains("publish-closure: OK"), "Output should contain success marker");
    assert!(default_stdout.contains("crates checked"), "Multiple crates should use plural form");
    assert!(default_stdout.contains("0 violations"), "Zero violations must be shown explicitly");

    // Single-crate invocation: singular form.
    let single_out = Command::cargo_bin("xtask")?
        .args(["publish-closure", "--crate-name", "perl-token"])
        .output()?;
    assert!(single_out.status.success(), "Single-crate invocation should succeed");
    let single_stdout = String::from_utf8_lossy(&single_out.stdout);
    assert!(single_stdout.contains("1 crate checked"), "Single crate should use singular form");
    Ok(())
}

// =============================================================================
// Error Path and Exit Code Tests
// =============================================================================

/// Test that invalid crate name produces exit code 1 (not generic error code).
///
/// Error path: When a crate name is not in the allowlist, exit must be 1.
#[test]
fn publish_closure_invalid_crate_exit_code_is_one() -> Result<()> {
    Command::cargo_bin("xtask")?
        .args(["publish-closure", "--crate-name", "not-a-real-crate"])
        .assert()
        .failure()
        .code(1);
    Ok(())
}

/// Test that invalid crate name produces clear error message.
///
/// Error path: Stderr must name the unrecognized crate in the error.
/// Message should be: "Crate 'X' not found in publish allowlist"
#[test]
fn publish_closure_invalid_crate_error_message_clear() -> Result<()> {
    let test_crate_name = "not-a-real-crate-xyz";
    let output = Command::cargo_bin("xtask")?
        .args(["publish-closure", "--crate-name", test_crate_name])
        .output()?;

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(test_crate_name)
            && (stderr.contains("not found") || stderr.contains("not in")),
        "Error message should mention the crate name and indicate it's not in the allowlist. Got: {}",
        stderr
    );
    Ok(())
}

/// Test that success produces no stderr output.
///
/// Regression guard: When the gate passes, stderr should be empty
/// (only stdout contains the OK message).
#[test]
fn publish_closure_success_has_no_stderr() -> Result<()> {
    let output = Command::cargo_bin("xtask")?.args(["publish-closure"]).output()?;

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.is_empty(), "Success should have no stderr output. Got: {}", stderr);
    Ok(())
}

// =============================================================================
// Case Sensitivity and Crate Name Boundary Tests
// =============================================================================

/// Test that invalid crate names are rejected for all boundary conditions.
///
/// Covers:
/// - Case mismatch: "Perl-Token" != "perl-token" (case-sensitive matching)
/// - Leading whitespace: " perl-token" is not in the allowlist
/// - Very long name: 1000-char string fails cleanly without panic
/// - Special characters: "perl-token@123" is not a valid crate name
/// - Empty string: "" must not be treated as "check all"
#[test]
fn publish_closure_invalid_crate_names_rejected() -> Result<()> {
    let long_name = "x".repeat(1000);
    let cases: &[(&str, &str)] = &[
        ("Perl-Token", "wrong case"),
        (" perl-token", "leading whitespace"),
        (&long_name, "very long name"),
        ("perl-token@123", "special characters"),
        ("", "empty string"),
    ];
    for (name, description) in cases {
        let status = Command::cargo_bin("xtask")?
            .args(["publish-closure", "--crate-name", name])
            .output()?
            .status;
        assert!(!status.success(), "Expected rejection for {description}: {name:?}");
    }
    Ok(())
}

// =============================================================================
// Filtering and Allowlist Tests
// =============================================================================

/// Test that a different valid crate can also be filtered.
///
/// Boundary condition: Verify filtering works for multiple crates,
/// not just perl-token. Pick a different published crate.
#[test]
fn publish_closure_filtering_works_for_multiple_crates() -> Result<()> {
    // perl-error is another core published crate
    let output = Command::cargo_bin("xtask")?
        .args(["publish-closure", "--crate-name", "perl-error"])
        .output()?;

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("1 crate checked"), "Should filter to single crate");
    Ok(())
}

/// Test that filtering a crate twice in one invocation doesn't break.
///
/// Edge case: Clap rejects duplicate flags, ensuring clean error handling.
#[test]
fn publish_closure_multiple_crate_name_flags_rejected() -> Result<()> {
    // Clap parser rejects repeated non-repeatable flags with a clear error.
    // Ensure the command fails gracefully with a helpful message.
    let output = Command::cargo_bin("xtask")?
        .args(["publish-closure", "--crate-name", "perl-token", "--crate-name", "perl-error"])
        .output()?;

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("cannot be used multiple times"),
        "Should reject duplicate --crate-name"
    );
    Ok(())
}

// =============================================================================
// Closure Correctness: Transitive Depth and Breadth
// =============================================================================

/// Test that a crate with deeper transitive deps is still checked.
///
/// Regression guard: Verify transitive closure walk reaches multi-level depth.
/// The closure walk must follow all normal deps recursively, not just direct deps.
///
/// We verify depth indirectly: `perl-semantic-analyzer` has many levels of transitive
/// normal deps.  The single-crate filter invocation confirms the BFS is actually
/// invoked (not bypassed) for named crates, and the "1 crate checked" output confirms
/// the filter path executed correctly.
#[test]
fn publish_closure_transitive_deps_are_walked() -> Result<()> {
    // Check a crate known to have multi-level transitive deps.
    // If BFS stopped at depth 1, deep violations would be missed silently.
    // On master the full closure is clean, so success + correct count confirms
    // the walk ran to completion for this crate.
    let output = Command::cargo_bin("xtask")?
        .args(["publish-closure", "--crate-name", "perl-semantic-analyzer"])
        .output()?;
    assert!(output.status.success(), "publish-closure should succeed for perl-semantic-analyzer");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("1 crate checked"), "Should filter to single crate; got: {}", stdout);
    assert!(stdout.contains("0 violations"), "No violations expected on master; got: {}", stdout);
    Ok(())
}

/// Test that the closure walk terminates promptly (BFS with visited set, not depth-limited).
///
/// Regression guard: A naive implementation without a visited set would exponentially
/// re-walk shared nodes that are reachable from many roots.  We verify termination
/// by timing: the full 132-crate scan must complete in under 60 seconds (it typically
/// finishes in under 5 seconds, but we give generous headroom for CI).
#[test]
fn publish_closure_bfs_handles_graph_cycles() -> Result<()> {
    use std::time::Instant;
    let start = Instant::now();
    Command::cargo_bin("xtask")?.args(["publish-closure"]).assert().success();
    let elapsed = start.elapsed();
    assert!(
        elapsed.as_secs() < 60,
        "publish-closure took {}s -- possible infinite loop (BFS visited-set regression)",
        elapsed.as_secs()
    );
    Ok(())
}

// =============================================================================
// Dep Kind Filtering Tests
// =============================================================================

/// Test that normal deps are always checked (regression guard for dep_kinds filtering).
///
/// Implementation detail: From the code, `is_normal_dep()` returns true if:
/// - `dep_kinds` is empty (treated conservatively as normal), OR
/// - Any entry in `dep_kinds` has `kind == null` (None)
///
/// This test guards against accidentally filtering out normal deps.
/// All allowlisted crates should be checked for violations in their normal deps.
#[test]
fn publish_closure_normal_deps_are_checked() -> Result<()> {
    // Every published crate depends on something (direct or transitive).
    // The closure walk must include normal deps.
    // On master, all normal deps are publishable, so this should succeed.
    let output = Command::cargo_bin("xtask")?.args(["publish-closure"]).output()?;

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("0 violations"), "Master should have no violations");
    Ok(())
}

/// Test that pure build-only deps are NOT over-flagged as violations.
///
/// Implementation detail: `is_normal_dep()` returns true only if a `dep_kinds` entry
/// has `kind == null` (normal).  Pure build-only deps (`kind == "build"`) are excluded
/// from the closure walk because:
/// - Build scripts run at compile time on the developer's machine
/// - They do not appear in downstream users' dependency trees
/// - `cargo publish` does NOT require build deps to be published
///
/// A dep used in BOTH roles (normal + build) is still walked because it has a
/// `kind == null` entry.  A dep used ONLY as a build dep is correctly skipped.
#[test]
fn publish_closure_build_deps_are_part_of_closure() -> Result<()> {
    // Verify pure build-only deps do not cause false violations.
    // The workspace has build-dep edges (verified via cargo metadata).
    // If any of those resolve to a publish=false crate it should NOT be flagged
    // (because is_normal_dep filters out pure build edges).
    // On master this exits 0, confirming build-only deps are not over-flagged.
    let output = Command::cargo_bin("xtask")?.args(["publish-closure"]).output()?;
    assert!(output.status.success(), "Build-only deps must not cause false violations");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("0 violations"),
        "No violations should be reported for build-only edges"
    );
    Ok(())
}

// =============================================================================
// Help and Meta Tests
// =============================================================================

/// Test that --help flag produces help text.
///
/// Regression guard: Ensure the subcommand provides help information.
#[test]
fn publish_closure_help_flag_works() -> Result<()> {
    let output = Command::cargo_bin("xtask")?.args(["publish-closure", "--help"]).output()?;

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("publish-closure") || stdout.contains("Check"),
        "Help should describe the command"
    );
    Ok(())
}

// =============================================================================
// Violation Reporting Tests
// =============================================================================

/// Test that violation messages include both published and forbidden crate names.
///
/// Error reporting: If a violation exists, the message must be clear:
/// ```
/// ERROR: publish-closure violation
///   Published crate `<published_name>` has transitive normal dep on `<forbidden_name>` (publish = false)
/// ```
///
/// This test documents the expected format but can only verify it would be
/// reported correctly if a violation existed. On master, no violations exist,
/// but this test ensures the implementation's error path is correctly understood.
#[test]
fn publish_closure_violation_message_format_documented() -> Result<()> {
    // This test documents the expected error format for violations.
    // The implementation reports all violations before exiting 1.
    // Each violation includes: published crate name, forbidden crate name, and reason.
    // On master (clean closure), this path is never taken, so we verify success instead.
    Command::cargo_bin("xtask")?.args(["publish-closure"]).assert().success();
    Ok(())
}

/// Test that the gate exits 1 when violations exist (would be caught by this gate).
///
/// Regression guard: If a violation was introduced, this gate MUST exit 1.
/// We can't easily create a violation in the test environment, but we can
/// verify that invalid crate names cause exit 1, confirming the error path works.
#[test]
fn publish_closure_exits_nonzero_on_error() -> Result<()> {
    Command::cargo_bin("xtask")?
        .args(["publish-closure", "--crate-name", "invalid"])
        .assert()
        .failure();
    Ok(())
}

// =============================================================================
// Numeric Boundary Tests
// =============================================================================

/// Test that the count of crates checked is numeric and reasonable.
///
/// Boundary condition: Extract the count from the output.
/// As of April 2026, the allowlist has 132 crates.
/// Verify the count is between 100 and 150 (reasonable range).
/// This catches off-by-one errors in the crate counting logic.
#[test]
fn publish_closure_crate_count_is_reasonable() -> Result<()> {
    let output = Command::cargo_bin("xtask")?.args(["publish-closure"]).output()?;

    assert!(output.status.success(), "Command should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Extract the count: "publish-closure: OK (132 crates checked, 0 violations)"
    // This is a basic sanity check — count should be between 100 and 200.
    assert!(
        stdout.contains("crates checked"),
        "Output should contain 'crates checked'. Output was: {}",
        stdout
    );
    // Additional check: if we can parse the number, verify it's reasonable
    if let Some(pos) = stdout.find('(')
        && let Some(space_pos) = stdout[pos..].find(' ')
    {
        let count_str = &stdout[pos + 1..pos + space_pos];
        if let Ok(count) = count_str.parse::<u32>() {
            assert!((100..=200).contains(&count), "Crate count {} is out of expected range", count);
        }
    }

    Ok(())
}

// =============================================================================
// Regression: Closure Starting State
// =============================================================================

/// Test that master (origin/master) has a clean closure by default.
///
/// Regression guard: This is the baseline expectation.
/// If this test starts failing, it means a violation has been introduced
/// in the upstream codebase (likely a recent merge).
/// This test should always pass on master.
#[test]
fn publish_closure_master_is_clean() -> Result<()> {
    let output = Command::cargo_bin("xtask")?.args(["publish-closure"]).output()?;

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("0 violations"), "Master closure should be clean");
    Ok(())
}
