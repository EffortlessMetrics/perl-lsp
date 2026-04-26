use anyhow::Result;
use assert_cmd::Command;
use std::path::PathBuf;
use tempfile::TempDir;

fn repo_root() -> Result<PathBuf> {
    Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".."))
}

fn fixture(path: &str) -> Result<PathBuf> {
    Ok(repo_root()?.join("xtask/tests/fixtures/agent-leases").join(path))
}

fn run_xtask(args: &[&str]) -> Result<std::process::Output> {
    let mut cmd = Command::cargo_bin("xtask")?;
    cmd.current_dir(repo_root()?);
    Ok(cmd.args(args).output()?)
}

#[test]
fn valid_lease_verifies() -> Result<()> {
    let temp = TempDir::new()?;
    let lease_out = temp.path().join("lease.json");
    let task_path = fixture("task-valid.json")?;
    let current_path = fixture("current-valid.json")?;
    let task = task_path.to_string_lossy().into_owned();
    let lease = lease_out.to_string_lossy().into_owned();
    let current = current_path.to_string_lossy().into_owned();

    let acquire = run_xtask(&["agent", "lease", "acquire", "--task", &task, "--out", &lease])?;
    assert!(acquire.status.success(), "acquire should pass");

    let verify =
        run_xtask(&["agent", "lease", "verify", "--lease", &lease, "--current", &current])?;
    assert!(verify.status.success(), "verify should pass");
    Ok(())
}

#[test]
fn expired_lease_fails() -> Result<()> {
    let temp = TempDir::new()?;
    let lease_out = temp.path().join("lease-expired.json");
    let task_path = fixture("task-expired.json")?;
    let current_path = fixture("current-valid.json")?;
    let task = task_path.to_string_lossy().into_owned();
    let lease = lease_out.to_string_lossy().into_owned();
    let current = current_path.to_string_lossy().into_owned();

    let acquire = run_xtask(&["agent", "lease", "acquire", "--task", &task, "--out", &lease])?;
    assert!(acquire.status.success(), "acquire should pass");

    let verify =
        run_xtask(&["agent", "lease", "verify", "--lease", &lease, "--current", &current])?;
    assert!(!verify.status.success(), "expired lease verify must fail");
    Ok(())
}

#[test]
fn stale_head_fails() -> Result<()> {
    let temp = TempDir::new()?;
    let lease_out = temp.path().join("lease.json");
    let task_path = fixture("task-valid.json")?;
    let current_path = fixture("current-stale-head.json")?;
    let task = task_path.to_string_lossy().into_owned();
    let lease = lease_out.to_string_lossy().into_owned();
    let current = current_path.to_string_lossy().into_owned();

    let acquire = run_xtask(&["agent", "lease", "acquire", "--task", &task, "--out", &lease])?;
    assert!(acquire.status.success(), "acquire should pass");

    let verify =
        run_xtask(&["agent", "lease", "verify", "--lease", &lease, "--current", &current])?;
    assert!(!verify.status.success(), "stale head verify must fail");
    Ok(())
}

#[test]
fn forbidden_mutation_receipt_fails() -> Result<()> {
    let receipt_path = fixture("receipt-forbidden-mutation.json")?;
    let receipt = receipt_path.to_string_lossy().into_owned();
    let validate = run_xtask(&["agent", "receipt", "validate", "--receipt", &receipt])?;
    assert!(!validate.status.success(), "forbidden mutation receipt must fail");
    Ok(())
}
