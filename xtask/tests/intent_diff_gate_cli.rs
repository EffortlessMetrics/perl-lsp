use anyhow::Result;
use assert_cmd::cargo::cargo_bin_cmd;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from("tests/fixtures/intent-diff").join(name)
}

#[test]
fn doc_only_code_fix_claim_fails() -> Result<()> {
    let mut cmd = cargo_bin_cmd!("xtask");
    let output = cmd
        .arg("intent-diff-gate")
        .arg("--fixture")
        .arg(fixture("doc_only_code_fix_claim.json"))
        .output()?;

    assert!(!output.status.success(), "doc-only fix claim fixture should fail");
    Ok(())
}

#[test]
fn partial_pr_using_refs_passes() -> Result<()> {
    let mut cmd = cargo_bin_cmd!("xtask");
    let output = cmd
        .arg("intent-diff-gate")
        .arg("--fixture")
        .arg(fixture("partial_refs_pass.json"))
        .output()?;

    assert!(output.status.success(), "partial refs fixture should pass");
    Ok(())
}

#[test]
fn valid_closeout_with_target_path_passes() -> Result<()> {
    let mut cmd = cargo_bin_cmd!("xtask");
    let output = cmd
        .arg("intent-diff-gate")
        .arg("--fixture")
        .arg(fixture("valid_closeout_target_path.json"))
        .output()?;

    assert!(output.status.success(), "valid closeout fixture should pass");
    Ok(())
}
