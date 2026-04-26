use anyhow::Result;
use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;

fn run_fixture(path: &str) -> Result<Value> {
    let mut cmd = cargo_bin_cmd!("xtask");
    let output = cmd.args(["queue", "health", "--fixture", path]).output()?;
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    let receipt: Value = serde_json::from_str(&stdout)?;
    Ok(receipt)
}

fn fixture_path(name: &str) -> String {
    format!("{}/tests/fixtures/queue-health/{name}", env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn fixture_master_green_maps_to_green_mode() -> Result<()> {
    let receipt = run_fixture(&fixture_path("master-green.json"))?;
    assert_eq!(receipt["mode"], "GREEN");
    Ok(())
}

#[test]
fn fixture_master_pending_maps_to_pending_mode() -> Result<()> {
    let receipt = run_fixture(&fixture_path("master-pending.json"))?;
    assert_eq!(receipt["mode"], "PENDING");
    Ok(())
}

#[test]
fn fixture_master_red_maps_to_red_mode() -> Result<()> {
    let receipt = run_fixture(&fixture_path("master-red.json"))?;
    assert_eq!(receipt["mode"], "RED");
    Ok(())
}
