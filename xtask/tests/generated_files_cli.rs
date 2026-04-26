use std::fs;

use assert_cmd::Command;
use color_eyre::eyre::Result;
use tempfile::tempdir;

#[test]
fn generated_files_changed_without_receipt_fails() -> Result<()> {
    let temp = tempdir()?;
    let receipt_path = temp.path().join("generated-files.json");

    let fixture = "tests/fixtures/generated-files/changed-without-receipt.json";
    let output = Command::cargo_bin("xtask")?
        .args([
            "generated-files",
            "check",
            "--fixture",
            fixture,
            "--receipt",
            receipt_path.to_str().ok_or_else(|| color_eyre::eyre::eyre!("non-utf8 path"))?,
        ])
        .output()?;

    assert!(
        !output.status.success(),
        "check should fail when receipt is missing; stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let receipt_raw = fs::read_to_string(&receipt_path)?;
    let receipt: serde_json::Value = serde_json::from_str(&receipt_raw)?;
    assert_eq!(receipt["verdict"], "fail");
    assert_eq!(receipt["missing_receipts"], serde_json::json!(["status-docs"]));

    Ok(())
}

#[test]
fn generated_files_with_receipt_passes() -> Result<()> {
    let temp = tempdir()?;
    let receipt_path = temp.path().join("generated-files.json");

    let fixture = "tests/fixtures/generated-files/changed-with-receipt.json";
    let output = Command::cargo_bin("xtask")?
        .args([
            "generated-files",
            "check",
            "--fixture",
            fixture,
            "--receipt",
            receipt_path.to_str().ok_or_else(|| color_eyre::eyre::eyre!("non-utf8 path"))?,
        ])
        .output()?;

    assert!(
        output.status.success(),
        "check should pass when matching receipt owner exists; stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let receipt_raw = fs::read_to_string(&receipt_path)?;
    let receipt: serde_json::Value = serde_json::from_str(&receipt_raw)?;
    assert_eq!(receipt["verdict"], "pass");
    assert_eq!(receipt["missing_receipts"], serde_json::json!([]));

    Ok(())
}
