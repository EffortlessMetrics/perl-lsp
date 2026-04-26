use anyhow::Result;
use assert_cmd::cargo::cargo_bin_cmd;
use std::path::{Path, PathBuf};

fn fixture(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/agent-leases").join(path)
}

#[test]
fn valid_lease_verifies() -> Result<()> {
    let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/agent/test-lease-valid.json");

    let mut acquire = cargo_bin_cmd!("xtask");
    let acquire_out = acquire
        .args(["agent", "lease", "acquire", "--task"])
        .arg(fixture("task-valid.json"))
        .args(["--out"])
        .arg(&out)
        .output()?;
    assert!(acquire_out.status.success(), "acquire failed: {:?}", acquire_out);

    let mut verify = cargo_bin_cmd!("xtask");
    let verify_out = verify
        .args(["agent", "lease", "verify", "--lease"])
        .arg(&out)
        .args(["--current"])
        .arg(fixture("snapshot-valid.json"))
        .output()?;

    assert!(verify_out.status.success(), "verify failed: {:?}", verify_out);
    Ok(())
}

#[test]
fn expired_lease_fails() -> Result<()> {
    let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/agent/test-lease-expired.json");

    let mut acquire = cargo_bin_cmd!("xtask");
    let acquire_out = acquire
        .args(["agent", "lease", "acquire", "--task"])
        .arg(fixture("task-expired.json"))
        .args(["--out"])
        .arg(&out)
        .output()?;
    assert!(acquire_out.status.success(), "acquire failed: {:?}", acquire_out);

    let mut verify = cargo_bin_cmd!("xtask");
    let verify_out = verify
        .args(["agent", "lease", "verify", "--lease"])
        .arg(&out)
        .args(["--current"])
        .arg(fixture("snapshot-valid.json"))
        .output()?;

    assert!(!verify_out.status.success(), "expired lease unexpectedly verified");
    Ok(())
}

#[test]
fn stale_head_fails() -> Result<()> {
    let out =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/agent/test-lease-stale-head.json");

    let mut acquire = cargo_bin_cmd!("xtask");
    let acquire_out = acquire
        .args(["agent", "lease", "acquire", "--task"])
        .arg(fixture("task-valid.json"))
        .args(["--out"])
        .arg(&out)
        .output()?;
    assert!(acquire_out.status.success(), "acquire failed: {:?}", acquire_out);

    let mut verify = cargo_bin_cmd!("xtask");
    let verify_out = verify
        .args(["agent", "lease", "verify", "--lease"])
        .arg(&out)
        .args(["--current"])
        .arg(fixture("snapshot-stale-head.json"))
        .output()?;

    assert!(!verify_out.status.success(), "stale head unexpectedly verified");
    Ok(())
}

#[test]
fn forbidden_mutation_receipt_fails() -> Result<()> {
    let mut validate = cargo_bin_cmd!("xtask");
    let validate_out = validate
        .args(["agent", "receipt", "validate", "--receipt"])
        .arg(fixture("receipt-forbidden-mutation.json"))
        .output()?;

    assert!(!validate_out.status.success(), "forbidden mutation unexpectedly validated");
    Ok(())
}
