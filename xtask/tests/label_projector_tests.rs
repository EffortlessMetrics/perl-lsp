use anyhow::{Context, Result, anyhow};
use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

fn repo_root() -> Result<PathBuf> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    Ok(manifest_dir
        .parent()
        .ok_or_else(|| anyhow!("xtask manifest should have a workspace parent"))?
        .to_path_buf())
}

#[test]
fn dry_run_needs_builder_fix_projects_expected_add_remove() -> Result<()> {
    let tmp = tempdir()?;
    let receipt = tmp.path().join("label-projection.json");
    let state = repo_root()?.join("xtask/tests/fixtures/label-projector/needs-builder-fix.json");

    let mut cmd = cargo_bin_cmd!("xtask");
    let output = cmd
        .arg("queue")
        .arg("project-labels")
        .arg("--state")
        .arg(&state)
        .arg("--dry-run")
        .arg("--receipt")
        .arg(&receipt)
        .output()
        .context("running xtask queue project-labels for NEEDS_BUILDER_FIX")?;

    assert!(output.status.success(), "command failed: {}", String::from_utf8_lossy(&output.stderr));

    let value: Value = serde_json::from_str(&fs::read_to_string(&receipt)?)?;
    let projection = &value["projections"][0];

    assert_eq!(projection["projected_apply"], serde_json::json!(["needs-builder-fix"]));
    assert_eq!(
        projection["projected_remove"],
        serde_json::json!(["review-reviewed", "ci-green", "merge-ready"])
    );
    assert_eq!(projection["skipped"], serde_json::json!(false));

    Ok(())
}

#[test]
fn apply_writes_receipt_for_skipped_entries() -> Result<()> {
    let tmp = tempdir()?;
    let receipt = tmp.path().join("label-projection.json");
    let state =
        repo_root()?.join("xtask/tests/fixtures/label-projector/merge-ready-missing-receipt.json");

    let mut cmd = cargo_bin_cmd!("xtask");
    let output = cmd
        .arg("queue")
        .arg("project-labels")
        .arg("--state")
        .arg(&state)
        .arg("--apply")
        .arg("--receipt")
        .arg(&receipt)
        .env("GH_TOKEN", "dummy-token-not-used-for-skipped")
        .output()
        .context("running xtask queue project-labels for MERGE_READY in apply mode")?;

    assert!(output.status.success(), "command failed: {}", String::from_utf8_lossy(&output.stderr));

    let value: Value = serde_json::from_str(&fs::read_to_string(&receipt)?)?;

    assert_eq!(value["dry_run"], serde_json::json!(false));
    assert_eq!(value["verdict"], serde_json::json!("skipped"));

    let projection = &value["projections"][0];
    assert_eq!(projection["skipped"], serde_json::json!(true));
    assert_eq!(projection["dry_run"], serde_json::json!(false));
    assert_eq!(projection["verdict"], serde_json::json!("skipped"));

    Ok(())
}

#[test]
fn dry_run_merge_ready_without_receipt_is_skipped() -> Result<()> {
    let tmp = tempdir()?;
    let receipt = tmp.path().join("label-projection.json");
    let state =
        repo_root()?.join("xtask/tests/fixtures/label-projector/merge-ready-missing-receipt.json");

    let mut cmd = cargo_bin_cmd!("xtask");
    let output = cmd
        .arg("queue")
        .arg("project-labels")
        .arg("--state")
        .arg(&state)
        .arg("--dry-run")
        .arg("--receipt")
        .arg(&receipt)
        .output()
        .context("running xtask queue project-labels for MERGE_READY")?;

    assert!(output.status.success(), "command failed: {}", String::from_utf8_lossy(&output.stderr));

    let value: Value = serde_json::from_str(&fs::read_to_string(&receipt)?)?;
    let projection = &value["projections"][0];

    assert_eq!(projection["skipped"], serde_json::json!(true));
    assert_eq!(projection["verdict"], serde_json::json!("skipped"));
    assert_eq!(projection["reason"], serde_json::json!("missing valid merge-ready receipt"));
    assert_eq!(projection["projected_apply"], serde_json::json!([]));

    Ok(())
}
