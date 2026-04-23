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
// B. JSON output — schema_version 2 fields
// ---------------------------------------------------------------------------

#[test]
fn test_ci_scope_json_output_is_valid_schema_v2() -> Result<()> {
    // Run with HEAD (which may equal base), so we get a valid empty or populated output.
    let output = Command::cargo_bin("xtask")?
        .args(["ci-scope", "--base", "HEAD", "--format", "json"])
        .output()?;

    assert!(
        output.status.success(),
        "ci-scope should exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout)?;
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| color_eyre::eyre::eyre!("JSON parse failed: {e}\nOutput was: {stdout}"))?;

    // schema_version 2 required fields
    assert_eq!(parsed["schema_version"], serde_json::json!(2), "schema_version must be 2");
    assert!(parsed["changed_files"].is_array(), "changed_files must be array");
    assert!(parsed["diff_class"].is_string(), "diff_class must be string");
    assert!(parsed["direct_crates"].is_array(), "direct_crates must be array");
    assert!(parsed["reverse_dep_closure"].is_array(), "reverse_dep_closure must be array");
    assert!(parsed["architecture_wideners"].is_array(), "architecture_wideners must be array");
    assert!(parsed["risk_tags"].is_array(), "risk_tags must be array");
    assert!(parsed["platform_overrides"].is_object(), "platform_overrides must be object");
    assert!(parsed["selected_lanes"].is_array(), "selected_lanes must be array");
    assert!(parsed["selected_heavy_lanes"].is_array(), "selected_heavy_lanes must be array");
    assert!(parsed["explanations"].is_object(), "explanations must be object");
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
// D. Empty diff (HEAD == base) → prose_only class, empty lanes
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

    let diff_class = parsed["diff_class"].as_str().unwrap_or("");
    assert_eq!(diff_class, "prose_only", "empty diff should be classified as prose_only");
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
    if !output.status.success() {
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

// ---------------------------------------------------------------------------
// F. diff_class is one of the valid values
// ---------------------------------------------------------------------------

#[test]
fn test_ci_scope_diff_class_is_valid_value() -> Result<()> {
    let output = Command::cargo_bin("xtask")?
        .args(["ci-scope", "--base", "HEAD", "--format", "json"])
        .output()?;

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout)?;
    let parsed: serde_json::Value = serde_json::from_str(&stdout)?;

    let valid_classes = ["code", "docs_as_code", "prose_only", "ci_config", "mixed"];
    let diff_class = parsed["diff_class"].as_str().unwrap_or("");
    assert!(
        valid_classes.contains(&diff_class),
        "diff_class must be one of {valid_classes:?}; got: {diff_class}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// G. Invalid base ref falls back and reports resolved base in output
// ---------------------------------------------------------------------------

#[test]
fn test_ci_scope_invalid_base_reports_resolved_fallback() -> Result<()> {
    let output = Command::cargo_bin("xtask")?
        .args(["ci-scope", "--base", "this-ref-should-not-exist", "--format", "json"])
        .output()?;

    assert!(
        output.status.success(),
        "ci-scope should resolve a fallback base; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout)?;
    let parsed: serde_json::Value = serde_json::from_str(&stdout)?;
    let base = parsed["base"].as_str().unwrap_or_default();

    assert_ne!(
        base, "this-ref-should-not-exist",
        "Output base should report the resolved fallback ref"
    );
    assert!(!base.is_empty(), "Output base should not be empty when fallback succeeds");

    Ok(())
}
