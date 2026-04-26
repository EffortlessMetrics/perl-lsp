use anyhow::{Context, Result};
use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;
use std::fs;
use tempfile::TempDir;

fn classify_fixture(fixture: &str) -> Result<Value> {
    let temp_dir = TempDir::new()?;
    let output = temp_dir.path().join("fix-forward.json");

    let mut cmd = cargo_bin_cmd!("xtask");
    cmd.args(["fix-forward", "classify", "--receipt", fixture, "--output"])
        .arg(&output)
        .assert()
        .success();

    let text =
        fs::read_to_string(&output).with_context(|| format!("reading {}", output.display()))?;
    let value =
        serde_json::from_str(&text).with_context(|| format!("parsing {}", output.display()))?;
    Ok(value)
}

#[test]
fn fmt_failure_receipt_classifies_as_fmt_only() -> Result<()> {
    let result = classify_fixture("tests/fixtures/fix-forward/fmt-failure.json")?;
    assert_eq!(result.get("fix_forward_kind").and_then(Value::as_str), Some("FMT_ONLY"));
    Ok(())
}

#[test]
fn stale_base_receipt_classifies_as_stale_base_cascade() -> Result<()> {
    let result = classify_fixture("tests/fixtures/fix-forward/stale-base-failure.json")?;
    assert_eq!(result.get("fix_forward_kind").and_then(Value::as_str), Some("STALE_BASE_CASCADE"));
    Ok(())
}

#[test]
fn generated_docs_receipt_classifies_as_generated_doc_regen() -> Result<()> {
    let result = classify_fixture("tests/fixtures/fix-forward/generated-docs-failure.json")?;
    assert_eq!(result.get("fix_forward_kind").and_then(Value::as_str), Some("GENERATED_DOC_REGEN"));
    Ok(())
}
