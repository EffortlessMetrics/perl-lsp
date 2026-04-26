use assert_cmd::Command;
use color_eyre::eyre::Result;

fn fixture_path(name: &str) -> String {
    format!("{}/tests/fixtures/scope-meta-gate/{name}", env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn fixture_parser_lane_removed_fails() -> Result<()> {
    let fixture = fixture_path("parser-lane-dropped.json");
    let output = Command::cargo_bin("xtask")?
        .args(["scope-meta-gate", "--fixture", fixture.as_str()])
        .output()?;

    assert!(
        !output.status.success(),
        "expected failure when parser lane is dropped; stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn fixture_docs_scope_expands_passes_or_warns() -> Result<()> {
    let fixture = fixture_path("docs-scope-expands.json");
    let output = Command::cargo_bin("xtask")?
        .args(["scope-meta-gate", "--fixture", fixture.as_str()])
        .output()?;

    assert!(
        output.status.success(),
        "docs expansion should pass/warn without failing; stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout)?;
    let parsed: serde_json::Value = serde_json::from_str(&stdout)?;
    let status =
        parsed.get("verdict").and_then(|v| v.get("status")).and_then(|v| v.as_str()).unwrap_or("");
    assert!(status == "warn" || status == "pass", "expected pass/warn, got {status}");
    Ok(())
}

#[test]
fn receipt_contains_required_top_level_fields() -> Result<()> {
    let fixture = fixture_path("docs-scope-expands.json");
    let output = Command::cargo_bin("xtask")?
        .args(["scope-meta-gate", "--fixture", fixture.as_str()])
        .output()?;

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    let parsed: serde_json::Value = serde_json::from_str(&stdout)?;

    for field in ["old_decision", "new_decision", "changed_lanes", "verdict"] {
        assert!(parsed.get(field).is_some(), "missing required field: {field}");
    }
    Ok(())
}
