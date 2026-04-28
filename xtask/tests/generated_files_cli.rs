//! Integration tests for `cargo xtask generated-files`.

use std::path::PathBuf;

use assert_cmd::Command;
use color_eyre::eyre::Result;

fn fixture_path(name: &str) -> Result<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    Ok(manifest_dir.join("tests/fixtures/generated-files").join(name))
}

#[test]
fn generated_file_changed_without_receipt_fails() -> Result<()> {
    let fixture = fixture_path("changed-without-receipt.json")?;
    let receipt = tempfile::NamedTempFile::new()?;

    Command::cargo_bin("xtask")?
        .args([
            "generated-files",
            "check",
            "--fixture",
            fixture.to_string_lossy().as_ref(),
            "--receipt",
            receipt.path().to_string_lossy().as_ref(),
        ])
        .assert()
        .failure();

    Ok(())
}

#[test]
fn generated_file_changed_with_receipt_passes() -> Result<()> {
    let fixture = fixture_path("changed-with-receipt.json")?;
    let receipt = tempfile::NamedTempFile::new()?;

    Command::cargo_bin("xtask")?
        .args([
            "generated-files",
            "check",
            "--fixture",
            fixture.to_string_lossy().as_ref(),
            "--receipt",
            receipt.path().to_string_lossy().as_ref(),
        ])
        .assert()
        .success();

    Ok(())
}
