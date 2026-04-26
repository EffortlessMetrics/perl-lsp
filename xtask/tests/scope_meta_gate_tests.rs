use std::fs;
use std::path::PathBuf;

use assert_cmd::Command;
use color_eyre::eyre::Result;

#[test]
fn fixture_parser_rule_removed_fails() -> Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let fixture = root.join("xtask/tests/fixtures/scope-meta-gate/parser-rule-removed.json");
    let receipt = root.join("target/receipts/scope-meta-gate-fixture-fail.json");

    let output = Command::cargo_bin("xtask")?
        .args([
            "scope-meta-gate",
            "--fixture",
            fixture.to_string_lossy().as_ref(),
            "--receipt",
            receipt.to_string_lossy().as_ref(),
        ])
        .output()?;

    assert!(!output.status.success(), "expected fail verdict for narrowed parser lane");

    let raw = fs::read_to_string(&receipt)?;
    let parsed: serde_json::Value = serde_json::from_str(&raw)?;
    assert_eq!(parsed["verdict"], serde_json::json!("fail"));
    Ok(())
}

#[test]
fn fixture_docs_scope_expands_warns() -> Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let fixture = root.join("xtask/tests/fixtures/scope-meta-gate/docs-expands-safely.json");
    let receipt = root.join("target/receipts/scope-meta-gate-fixture-warn.json");

    let output = Command::cargo_bin("xtask")?
        .args([
            "scope-meta-gate",
            "--fixture",
            fixture.to_string_lossy().as_ref(),
            "--receipt",
            receipt.to_string_lossy().as_ref(),
        ])
        .output()?;

    assert!(output.status.success(), "warn verdict should not fail command");

    let raw = fs::read_to_string(&receipt)?;
    let parsed: serde_json::Value = serde_json::from_str(&raw)?;
    assert!(matches!(parsed["verdict"].as_str(), Some("pass" | "warn")));
    Ok(())
}

#[test]
fn receipt_schema_file_exists_for_validation() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let schema = root.join(".ci/receipts/schemas/scope-meta-gate.schema.json");
    assert!(schema.exists(), "receipt schema must exist for registry-based validation");
}
