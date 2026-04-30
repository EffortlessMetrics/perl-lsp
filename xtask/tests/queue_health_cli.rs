use anyhow::{Result, bail};
use assert_cmd::cargo::cargo_bin_cmd;

#[test]
fn fixture_master_green_maps_to_green() -> Result<()> {
    assert_mode("tests/fixtures/queue-health/master-green.json", "GREEN")
}

#[test]
fn fixture_master_pending_maps_to_pending() -> Result<()> {
    assert_mode("tests/fixtures/queue-health/master-pending.json", "PENDING")
}

#[test]
fn fixture_master_red_maps_to_red() -> Result<()> {
    assert_mode("tests/fixtures/queue-health/master-red.json", "RED")
}

fn assert_mode(fixture: &str, expected: &str) -> Result<()> {
    let mut cmd = cargo_bin_cmd!("xtask");
    let output = cmd.args(["queue", "health", "--fixture", fixture]).output()?;

    if !output.status.success() {
        bail!("xtask queue health failed for fixture {fixture}");
    }

    let stdout = String::from_utf8(output.stdout)?;
    let first_line = stdout.lines().next().unwrap_or_default();
    if first_line != expected {
        bail!("expected mode {expected}, got {first_line}");
    }

    Ok(())
}
