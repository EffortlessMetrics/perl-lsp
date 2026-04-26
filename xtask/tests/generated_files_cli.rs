use std::path::Path;

use anyhow::Result;
use assert_cmd::cargo::cargo_bin_cmd;

#[test]
fn generated_file_check_fails_without_receipt_fixture() -> Result<()> {
    let fixture = fixture_path("missing-receipt.json");
    let mut cmd = cargo_bin_cmd!("xtask");
    let output = cmd
        .arg("generated-files")
        .arg("check")
        .arg("--fixture")
        .arg(&fixture)
        .arg("--receipt")
        .arg("target/receipts/generated-files-test-fail.json")
        .output()?;

    assert!(!output.status.success());
    Ok(())
}

#[test]
fn generated_file_check_passes_with_receipt_fixture() -> Result<()> {
    let fixture = fixture_path("with-receipt.json");
    let mut cmd = cargo_bin_cmd!("xtask");
    let output = cmd
        .arg("generated-files")
        .arg("check")
        .arg("--fixture")
        .arg(&fixture)
        .arg("--receipt")
        .arg("target/receipts/generated-files-test-pass.json")
        .output()?;

    assert!(output.status.success());
    Ok(())
}

fn fixture_path(name: &str) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap_or_else(|| Path::new("."));
    root.join("xtask")
        .join("tests")
        .join("fixtures")
        .join("generated-files")
        .join(name)
        .display()
        .to_string()
}
