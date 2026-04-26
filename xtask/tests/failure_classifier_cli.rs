use anyhow::{Context, Result};
use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;

fn run_fixture(name: &str) -> Result<Value> {
    let fixture = format!("tests/fixtures/failure-classifier/{name}.json");
    let mut cmd = cargo_bin_cmd!("xtask");
    let output = cmd
        .arg("failure-classifier")
        .arg("--fixture")
        .arg(&fixture)
        .output()
        .with_context(|| format!("running fixture {fixture}"))?;

    assert!(output.status.success(), "fixture command should succeed");

    let stdout = String::from_utf8(output.stdout).context("classifier stdout must be UTF-8")?;
    serde_json::from_str::<Value>(&stdout).context("classifier output must be JSON")
}

#[test]
fn fixture_master_red_classifies_as_master_red() -> Result<()> {
    let json = run_fixture("master-red")?;
    assert_eq!(json["classification"], "MASTER_RED");
    Ok(())
}

#[test]
fn fixture_stale_base_classifies_as_stale_base() -> Result<()> {
    let json = run_fixture("stale-base")?;
    assert_eq!(json["classification"], "STALE_BASE");
    Ok(())
}

#[test]
fn fixture_pr_owned_classifies_as_pr_owned() -> Result<()> {
    let json = run_fixture("pr-owned")?;
    assert_eq!(json["classification"], "PR_OWNED");
    Ok(())
}

#[test]
fn fixture_missing_data_classifies_as_unknown() -> Result<()> {
    let json = run_fixture("unknown")?;
    assert_eq!(json["classification"], "UNKNOWN");
    Ok(())
}
