use anyhow::Result;
use assert_cmd::cargo::cargo_bin_cmd;
use tempfile::tempdir;

fn fixture(name: &str) -> String {
    format!("tests/fixtures/agent-leases/{name}")
}

#[test]
fn valid_lease_verifies() -> Result<()> {
    let dir = tempdir()?;
    let lease_path = dir.path().join("lease.json");

    let mut acquire = cargo_bin_cmd!("xtask");
    acquire
        .args([
            "agent",
            "lease",
            "acquire",
            "--task",
            &fixture("task-valid.json"),
            "--out",
            &lease_path.to_string_lossy(),
        ])
        .assert()
        .success();

    let mut verify = cargo_bin_cmd!("xtask");
    verify
        .args([
            "agent",
            "lease",
            "verify",
            "--lease",
            &lease_path.to_string_lossy(),
            "--current",
            &fixture("snapshot-valid.json"),
        ])
        .assert()
        .success();

    Ok(())
}

#[test]
fn expired_lease_fails() -> Result<()> {
    let dir = tempdir()?;
    let lease_path = dir.path().join("lease-expired.json");

    let mut acquire = cargo_bin_cmd!("xtask");
    acquire
        .args([
            "agent",
            "lease",
            "acquire",
            "--task",
            &fixture("task-expired.json"),
            "--out",
            &lease_path.to_string_lossy(),
        ])
        .assert()
        .success();

    let mut verify = cargo_bin_cmd!("xtask");
    verify
        .args([
            "agent",
            "lease",
            "verify",
            "--lease",
            &lease_path.to_string_lossy(),
            "--current",
            &fixture("snapshot-valid.json"),
        ])
        .assert()
        .failure();

    Ok(())
}

#[test]
fn stale_head_fails() -> Result<()> {
    let dir = tempdir()?;
    let lease_path = dir.path().join("lease-stale.json");

    let mut acquire = cargo_bin_cmd!("xtask");
    acquire
        .args([
            "agent",
            "lease",
            "acquire",
            "--task",
            &fixture("task-valid.json"),
            "--out",
            &lease_path.to_string_lossy(),
        ])
        .assert()
        .success();

    let mut verify = cargo_bin_cmd!("xtask");
    verify
        .args([
            "agent",
            "lease",
            "verify",
            "--lease",
            &lease_path.to_string_lossy(),
            "--current",
            &fixture("snapshot-stale-head.json"),
        ])
        .assert()
        .failure();

    Ok(())
}

#[test]
fn forbidden_mutation_receipt_fails() -> Result<()> {
    let mut validate = cargo_bin_cmd!("xtask");
    validate
        .args([
            "agent",
            "receipt",
            "validate",
            "--receipt",
            &fixture("receipt-forbidden-mutation.json"),
        ])
        .assert()
        .failure();

    Ok(())
}
