use anyhow::{Context, Result};
use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;
use std::path::PathBuf;

#[test]
fn dry_run_needs_builder_fix_projects_expected_changes() -> Result<()> {
    let fixture = fixture_path("needs-builder-fix.json");
    let tmp = tempfile::NamedTempFile::new()?;

    let mut cmd = cargo_bin_cmd!("xtask");
    let output = cmd
        .args([
            "queue",
            "project-labels",
            "--state",
            fixture.to_string_lossy().as_ref(),
            "--dry-run",
            "--receipt",
            tmp.path().to_string_lossy().as_ref(),
        ])
        .output()?;
    assert!(output.status.success(), "command should succeed in dry-run mode");

    let receipt_raw = fs::read_to_string(tmp.path())?;
    let receipt: serde_json::Value = serde_json::from_str(&receipt_raw)?;

    assert_eq!(receipt["dry_run"], true);
    assert_eq!(receipt["verdict"], "ok");

    let projected_apply =
        receipt["projected_apply"].as_array().context("projected_apply should be array")?;
    assert_eq!(projected_apply.len(), 1);
    assert_eq!(projected_apply[0], "needs-builder-fix");

    let projected_remove =
        receipt["projected_remove"].as_array().context("projected_remove should be array")?;
    assert!(projected_remove.iter().any(|v| v == "review-reviewed"));
    assert!(projected_remove.iter().any(|v| v == "ci-green"));
    assert!(projected_remove.iter().any(|v| v == "merge-ready"));

    Ok(())
}

#[test]
fn dry_run_merge_ready_without_receipt_is_blocked() -> Result<()> {
    let fixture = fixture_path("merge-ready-missing-receipt.json");
    let tmp = tempfile::NamedTempFile::new()?;

    let mut cmd = cargo_bin_cmd!("xtask");
    let output = cmd
        .args([
            "queue",
            "project-labels",
            "--state",
            fixture.to_string_lossy().as_ref(),
            "--dry-run",
            "--receipt",
            tmp.path().to_string_lossy().as_ref(),
        ])
        .output()?;

    assert!(!output.status.success(), "merge-ready without receipt must be blocked");

    let receipt_raw = fs::read_to_string(tmp.path())?;
    let receipt: serde_json::Value = serde_json::from_str(&receipt_raw)?;

    assert_eq!(receipt["verdict"], "blocked");
    assert_eq!(receipt["skipped"], true);
    assert_eq!(receipt["reason"], "merge-ready receipt missing or invalid");

    Ok(())
}

fn fixture_path(name: &str) -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.join("tests").join("fixtures").join("label-projector").join(name)
}
