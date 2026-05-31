use anyhow::{Result, anyhow};
use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn ci_route_cli_writes_supported_editor_proof_pack_receipt() -> Result<()> {
    let temp = TempDir::new()?;
    let receipt = temp.path().join("ci-route.json");

    cargo_bin_cmd!("xtask")
        .args([
            "ci",
            "route",
            "--base",
            "origin/main",
            "--head",
            "HEAD",
            "--receipt",
            receipt.to_str().ok_or_else(|| anyhow!("invalid ci route receipt path"))?,
            "--changed-file",
            "xtask/src/tasks/supported_editor_inline_smoke.rs",
        ])
        .assert()
        .success();

    let route: Value = serde_json::from_str(&std::fs::read_to_string(receipt)?)?;
    assert_eq!(route.get("schema_version").and_then(Value::as_str), Some("ci-route.v1"));
    assert_eq!(
        route.pointer("/changed_surfaces/0").and_then(Value::as_str),
        Some("xtask-supported-editor-inline-smoke")
    );
    assert_eq!(
        route.pointer("/coverage_pack_selector/0").and_then(Value::as_str),
        Some("patch-coverage-xtask-supported-editor-inline-smoke")
    );
    assert!(route.get("required_proof_packs").and_then(Value::as_array).is_some_and(|packs| {
        packs.iter().any(|pack| {
            pack.get("id").and_then(Value::as_str) == Some("xtask-supported-editor-inline-smoke")
        })
    }));
    assert_eq!(
        route.pointer("/skipped_by_policy/full-ux-regression").and_then(Value::as_str),
        Some("supported-editor smoke receipt change")
    );
    Ok(())
}
