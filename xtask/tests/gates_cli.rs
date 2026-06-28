//! CLI contract tests for `cargo xtask gates --list`.
//!
//! These tests exercise policy loading, gate filtering, and list rendering
//! without executing any configured gate commands.

use anyhow::Result;
use assert_cmd::cargo::cargo_bin_cmd;

#[test]
fn gates_list_pr_fast_renders_policy_tier_without_running_gates() -> Result<()> {
    let mut cmd = cargo_bin_cmd!("xtask");
    let output = cmd.args(["gates", "--list", "--tier", "pr-fast"]).output()?;

    assert!(
        output.status.success(),
        "gates --list should succeed; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("Available Gates"), "missing gate catalog header: {stdout}");
    assert!(stdout.contains("pr_fast"), "missing pr_fast tier: {stdout}");
    assert!(stdout.contains("fmt"), "missing fmt gate: {stdout}");
    assert!(stdout.contains("compile_all_targets"), "missing compile_all_targets gate: {stdout}");
    assert!(stdout.contains("* = required gate"), "missing required-gate legend: {stdout}");
    assert!(
        !stdout.contains("merge_gate"),
        "pr-fast list should not render merge_gate tier: {stdout}"
    );

    Ok(())
}

#[test]
fn gates_list_explicit_gate_overrides_tier_filter() -> Result<()> {
    let mut cmd = cargo_bin_cmd!("xtask");
    let output = cmd.args(["gates", "--list", "--tier", "nightly", "--gate", "fmt"]).output()?;

    assert!(
        output.status.success(),
        "explicit gate list should succeed; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("Available Gates"), "missing gate catalog header: {stdout}");
    assert!(stdout.contains("fmt"), "missing explicit fmt gate: {stdout}");
    assert!(
        !stdout.contains("compile_all_targets"),
        "explicit gate list should include only the requested gate: {stdout}"
    );

    Ok(())
}

#[test]
fn gates_list_unknown_gate_fails_actionably() -> Result<()> {
    let mut cmd = cargo_bin_cmd!("xtask");
    let output = cmd.args(["gates", "--list", "--gate", "definitely_missing_gate"]).output()?;

    assert!(
        !output.status.success(),
        "unknown explicit gate should fail instead of rendering an empty list"
    );

    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains("No gate found with name 'definitely_missing_gate'"),
        "missing actionable unknown-gate error: {stderr}"
    );

    Ok(())
}
