use anyhow::Result;
use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn repo_root() -> PathBuf {
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    root.pop();
    root
}

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("fix-forward")
        .join(name)
}

fn classify_fixture(name: &str) -> Result<Value> {
    let output_dir = TempDir::new()?;
    let output_path = output_dir.path().join("fix-forward.json");

    let mut cmd = Command::cargo_bin("xtask")?;
    cmd.current_dir(repo_root()).args([
        "fix-forward",
        "classify",
        "--receipt",
        &fixture_path(name).to_string_lossy(),
        "--output",
        &output_path.to_string_lossy(),
    ]);
    cmd.assert().success();

    let raw = fs::read_to_string(output_path)?;
    Ok(serde_json::from_str(&raw)?)
}

#[test]
fn fmt_receipt_classifies_to_fmt_only() -> Result<()> {
    let json = classify_fixture("fmt-failure.json")?;
    assert_eq!(json.get("classification").and_then(Value::as_str), Some("FMT_ONLY"));
    Ok(())
}

#[test]
fn stale_base_receipt_classifies_to_stale_base_cascade() -> Result<()> {
    let json = classify_fixture("stale-base-failure.json")?;
    assert_eq!(json.get("classification").and_then(Value::as_str), Some("STALE_BASE_CASCADE"));
    Ok(())
}

#[test]
fn generated_docs_receipt_classifies_to_generated_doc_regen() -> Result<()> {
    let json = classify_fixture("generated-docs-failure.json")?;
    assert_eq!(json.get("classification").and_then(Value::as_str), Some("GENERATED_DOC_REGEN"));
    Ok(())
}
