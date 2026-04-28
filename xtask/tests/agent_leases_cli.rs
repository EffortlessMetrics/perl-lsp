use anyhow::Result;
use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;

fn fixture(path: &str) -> String {
    format!("{}/tests/fixtures/agent-leases/{path}", env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn valid_lease_verifies() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let lease = tmp.path().join("lease.json");

    let mut acquire = cargo_bin_cmd!("xtask");
    let output = acquire
        .args([
            "agent",
            "lease",
            "acquire",
            "--task",
            &fixture("task-valid.json"),
            "--out",
            lease.to_string_lossy().as_ref(),
        ])
        .output()?;
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));

    let mut verify = cargo_bin_cmd!("xtask");
    let output = verify
        .args([
            "agent",
            "lease",
            "verify",
            "--lease",
            lease.to_string_lossy().as_ref(),
            "--current",
            &fixture("current-valid.json"),
        ])
        .output()?;
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));

    Ok(())
}

#[test]
fn expired_lease_fails() -> Result<()> {
    let mut cmd = cargo_bin_cmd!("xtask");
    let output = cmd
        .args([
            "agent",
            "lease",
            "verify",
            "--lease",
            &fixture("lease-expired.json"),
            "--current",
            &fixture("current-valid.json"),
        ])
        .output()?;

    assert!(!output.status.success(), "expired lease must fail verification");
    Ok(())
}

#[test]
fn stale_head_fails() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let lease = tmp.path().join("lease.json");

    let mut acquire = cargo_bin_cmd!("xtask");
    let output = acquire
        .args([
            "agent",
            "lease",
            "acquire",
            "--task",
            &fixture("task-valid.json"),
            "--out",
            lease.to_string_lossy().as_ref(),
        ])
        .output()?;
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));

    let mut verify = cargo_bin_cmd!("xtask");
    let output = verify
        .args([
            "agent",
            "lease",
            "verify",
            "--lease",
            lease.to_string_lossy().as_ref(),
            "--current",
            &fixture("current-stale-head.json"),
        ])
        .output()?;

    assert!(!output.status.success(), "stale head must fail verification");
    Ok(())
}

#[test]
fn forbidden_mutation_receipt_fails() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let lease_path = tmp.path().join("lease.json");
    let receipt_path = tmp.path().join("receipt.json");

    let mut acquire = cargo_bin_cmd!("xtask");
    let output = acquire
        .args([
            "agent",
            "lease",
            "acquire",
            "--task",
            &fixture("task-valid.json"),
            "--out",
            lease_path.to_string_lossy().as_ref(),
        ])
        .output()?;
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));

    let raw = fs::read_to_string(fixture("receipt-forbidden-mutation.json"))?;
    let mut receipt_json: serde_json::Value = serde_json::from_str(&raw)?;
    receipt_json["lease_path"] =
        serde_json::Value::String(lease_path.to_string_lossy().to_string());
    fs::write(&receipt_path, format!("{}\n", serde_json::to_string_pretty(&receipt_json)?))?;

    let mut validate = cargo_bin_cmd!("xtask");
    let output = validate
        .args([
            "agent",
            "receipt",
            "validate",
            "--receipt",
            receipt_path.to_string_lossy().as_ref(),
        ])
        .output()?;

    assert!(!output.status.success(), "forbidden mutation receipt must fail validation");
    Ok(())
}
