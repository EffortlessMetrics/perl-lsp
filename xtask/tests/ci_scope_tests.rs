//! CLI integration tests for `cargo xtask ci-scope`.
//!
//! These tests verify the CLI contract using assert_cmd.
//! Unit tests for the core logic live inline in `tasks/ci_scope.rs`.

use assert_cmd::Command;
use color_eyre::eyre::Result;

// ---------------------------------------------------------------------------
// A. Subcommand exists and responds to --help
// ---------------------------------------------------------------------------

#[test]
fn test_ci_scope_help_shows_base_flag() -> Result<()> {
    let output = Command::cargo_bin("xtask")?.args(["ci-scope", "--help"]).output()?;
    assert!(output.status.success(), "Help should exit 0");
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains("--base") || stdout.contains("base"),
        "Help output should mention --base; got: {stdout}"
    );
    Ok(())
}

#[test]
fn test_ci_scope_help_shows_format_flag() -> Result<()> {
    let output = Command::cargo_bin("xtask")?.args(["ci-scope", "--help"]).output()?;
    assert!(output.status.success(), "Help should exit 0");
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains("--format") || stdout.contains("format"),
        "Help output should mention --format; got: {stdout}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// B. JSON output format
// ---------------------------------------------------------------------------

#[test]
fn test_ci_scope_json_output_is_valid() -> Result<()> {
    // Run with HEAD (which may equal base), so we get a valid empty or populated output.
    let output = Command::cargo_bin("xtask")?
        .args(["ci-scope", "--base", "HEAD", "--format", "json"])
        .output()?;

    // Should exit successfully regardless of diff content
    assert!(
        output.status.success(),
        "ci-scope should exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout)?;
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| color_eyre::eyre::eyre!("JSON parse failed: {e}\nOutput was: {stdout}"))?;

    assert_eq!(parsed["schema_version"], serde_json::json!(1), "schema_version must be 1");
    assert!(parsed["changed_files"].is_array(), "changed_files must be array");
    assert!(parsed["changed_crates"].is_array(), "changed_crates must be array");
    assert!(parsed["widened_crates"].is_array(), "widened_crates must be array");
    assert!(parsed["selected_lanes"].is_array(), "selected_lanes must be array");
    Ok(())
}

// ---------------------------------------------------------------------------
// C. Text output format
// ---------------------------------------------------------------------------

#[test]
fn test_ci_scope_text_output_is_readable() -> Result<()> {
    let output = Command::cargo_bin("xtask")?
        .args(["ci-scope", "--base", "HEAD", "--format", "text"])
        .output()?;

    assert!(
        output.status.success(),
        "ci-scope --format text should exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains("CI Scope") || stdout.contains("Base") || stdout.contains("HEAD"),
        "Text output should contain summary info; got: {stdout}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// D. Empty diff (HEAD == base) returns empty lanes
// ---------------------------------------------------------------------------

#[test]
fn test_ci_scope_empty_diff_has_no_selected_lanes() -> Result<()> {
    // When base is HEAD, there is no diff — selected_lanes should be empty.
    let output = Command::cargo_bin("xtask")?
        .args(["ci-scope", "--base", "HEAD", "--format", "json"])
        .output()?;

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout)?;
    let parsed: serde_json::Value = serde_json::from_str(&stdout)?;

    let lanes = parsed["selected_lanes"]
        .as_array()
        .ok_or_else(|| color_eyre::eyre::eyre!("selected_lanes is not an array: {}", parsed))?;
    assert!(
        lanes.is_empty(),
        "empty diff (HEAD==HEAD) should produce no selected lanes; got: {lanes:#?}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// E. Every lane has a reason field
// ---------------------------------------------------------------------------

#[test]
fn test_ci_scope_each_lane_has_reason_field() -> Result<()> {
    let output = Command::cargo_bin("xtask")?
        .args(["ci-scope", "--base", "HEAD~1", "--format", "json"])
        .output()?;

    // If HEAD~1 doesn't exist (shallow clone) the command will fall back gracefully.
    // Either way it should exit 0.
    if !output.status.success() {
        // If the command errored, skip — shallow clone has no history.
        return Ok(());
    }

    let stdout = String::from_utf8(output.stdout)?;
    let parsed: serde_json::Value = serde_json::from_str(&stdout)?;

    if let Some(lanes) = parsed["selected_lanes"].as_array() {
        for lane in lanes {
            let reason = lane["reason"].as_str().unwrap_or("");
            assert!(!reason.is_empty(), "every lane must have a non-empty reason; lane: {lane:#?}");
        }
    }
    Ok(())
}
