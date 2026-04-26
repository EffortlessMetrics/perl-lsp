use anyhow::Result;
use assert_cmd::Command;
use tempfile::tempdir;

#[test]
fn generated_file_changed_without_receipt_fails() -> Result<()> {
    let fixture = "tests/fixtures/generated-files/changed-without-receipt.json";
    let temp = tempdir()?;
    let receipt = temp.path().join("generated-files.json");

    let output = Command::cargo_bin("xtask")?
        .args([
            "generated-files",
            "check",
            "--fixture",
            fixture,
            "--receipt",
            receipt.to_str().ok_or_else(|| anyhow::anyhow!("utf8 path expected"))?,
        ])
        .output()?;

    assert!(!output.status.success(), "check should fail without matching receipt");
    Ok(())
}

#[test]
fn generator_receipt_present_passes() -> Result<()> {
    let fixture = "tests/fixtures/generated-files/changed-with-receipt.json";
    let temp = tempdir()?;
    let receipt = temp.path().join("generated-files.json");

    let output = Command::cargo_bin("xtask")?
        .args([
            "generated-files",
            "check",
            "--fixture",
            fixture,
            "--receipt",
            receipt.to_str().ok_or_else(|| anyhow::anyhow!("utf8 path expected"))?,
        ])
        .output()?;

    assert!(output.status.success(), "check should pass with matching receipt");
    Ok(())
}
