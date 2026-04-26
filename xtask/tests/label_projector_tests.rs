use assert_cmd::Command;
use color_eyre::eyre::Result;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn fixture_path(name: &str) -> String {
    Path::new("tests/fixtures/label-projector").join(name).to_string_lossy().to_string()
}

#[test]
fn dry_run_needs_builder_fix_projects_expected_add_and_remove() -> Result<()> {
    let receipt_dir = tempdir()?;
    let receipt_path = receipt_dir.path().join("label-projection.json");

    let output = Command::cargo_bin("xtask")?
        .args([
            "queue",
            "project-labels",
            "--state",
            &fixture_path("needs-builder-fix.json"),
            "--dry-run",
            "--receipt",
            &receipt_path.to_string_lossy(),
        ])
        .output()?;

    assert!(output.status.success(), "command failed: {}", String::from_utf8_lossy(&output.stderr));

    let receipt = fs::read_to_string(receipt_path)?;
    let json: serde_json::Value = serde_json::from_str(&receipt)?;

    assert_eq!(json["dry_run"], serde_json::json!(true));
    assert_eq!(json["verdict"], serde_json::json!("dry-run"));
    assert_eq!(json["projected_apply"], serde_json::json!(["needs-builder-fix"]));
    assert_eq!(
        json["projected_remove"],
        serde_json::json!(["review-reviewed", "ci-green", "merge-ready"])
    );

    Ok(())
}

#[test]
fn dry_run_merge_ready_refuses_without_merge_readiness_receipt() -> Result<()> {
    let receipt_dir = tempdir()?;
    let receipt_path = receipt_dir.path().join("label-projection.json");

    let output = Command::cargo_bin("xtask")?
        .args([
            "queue",
            "project-labels",
            "--state",
            &fixture_path("merge-ready-missing-receipt.json"),
            "--dry-run",
            "--receipt",
            &receipt_path.to_string_lossy(),
        ])
        .output()?;

    assert!(output.status.success(), "command failed: {}", String::from_utf8_lossy(&output.stderr));

    let receipt = fs::read_to_string(receipt_path)?;
    let json: serde_json::Value = serde_json::from_str(&receipt)?;

    assert_eq!(json["skipped"], serde_json::json!(true));
    assert_eq!(json["verdict"], serde_json::json!("refused"));
    let reason = json["reason"].as_str().unwrap_or_default();
    assert!(reason.contains("missing valid merge-ready receipt"));

    Ok(())
}
