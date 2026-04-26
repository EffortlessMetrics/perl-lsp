use anyhow::Result;
use assert_cmd::Command;

#[test]
fn aggregate_and_finalize_pass_fixture() -> Result<()> {
    let output = Command::cargo_bin("xtask")?
        .args([
            "aggregate-receipts",
            "--check",
            "Test Gate",
            "--inputs",
            "tests/fixtures/aggregator/pass",
            "--output",
            "target/receipts/test-gate.json",
            "--advisory-mode",
            "pass",
        ])
        .output()?;
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));

    let output = Command::cargo_bin("xtask")?
        .args(["finalize-check", "--receipt", "target/receipts/test-gate.json"])
        .output()?;
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));

    Ok(())
}

#[test]
fn finalize_fails_on_required_failure_fixture() -> Result<()> {
    let output = Command::cargo_bin("xtask")?
        .args([
            "aggregate-receipts",
            "--check",
            "Test Gate",
            "--inputs",
            "tests/fixtures/aggregator/fail",
            "--output",
            "target/receipts/test-gate-fail.json",
        ])
        .output()?;
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));

    let output = Command::cargo_bin("xtask")?
        .args(["finalize-check", "--receipt", "target/receipts/test-gate-fail.json"])
        .output()?;
    assert!(!output.status.success(), "stdout: {}", String::from_utf8_lossy(&output.stdout));

    Ok(())
}

#[test]
fn finalize_fails_on_missing_required_fixture() -> Result<()> {
    let output = Command::cargo_bin("xtask")?
        .args([
            "aggregate-receipts",
            "--check",
            "Test Gate",
            "--inputs",
            "tests/fixtures/aggregator/missing-required",
            "--output",
            "target/receipts/test-gate-missing.json",
        ])
        .output()?;
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));

    let output = Command::cargo_bin("xtask")?
        .args(["finalize-check", "--receipt", "target/receipts/test-gate-missing.json"])
        .output()?;
    assert!(!output.status.success(), "stdout: {}", String::from_utf8_lossy(&output.stdout));

    Ok(())
}
