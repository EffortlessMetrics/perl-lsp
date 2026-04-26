use anyhow::Result;
use assert_cmd::cargo::cargo_bin_cmd;
use std::path::PathBuf;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("intent-diff")
        .join(name)
}

#[test]
fn fixture_6780_like_doc_only_claim_fails() -> Result<()> {
    let mut cmd = cargo_bin_cmd!("xtask");
    let output = cmd
        .args([
            "intent-diff-gate",
            "--fixture",
            &fixture_path("6780-doc-only-fails.json").display().to_string(),
        ])
        .output()?;

    assert!(!output.status.success());
    Ok(())
}

#[test]
fn fixture_partial_refs_passes() -> Result<()> {
    let mut cmd = cargo_bin_cmd!("xtask");
    let output = cmd
        .args([
            "intent-diff-gate",
            "--fixture",
            &fixture_path("partial-refs-passes.json").display().to_string(),
        ])
        .output()?;

    assert!(output.status.success());
    Ok(())
}

#[test]
fn fixture_valid_closeout_target_path_passes() -> Result<()> {
    let mut cmd = cargo_bin_cmd!("xtask");
    let output = cmd
        .args([
            "intent-diff-gate",
            "--fixture",
            &fixture_path("valid-closeout-target-path-passes.json").display().to_string(),
        ])
        .output()?;

    assert!(output.status.success());
    Ok(())
}
