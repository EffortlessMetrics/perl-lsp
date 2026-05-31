use anyhow::{Result, anyhow};
use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;
use std::{fs, path::PathBuf};
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
    assert_eq!(
        route.pointer("/coverage_proof_packs/0/id").and_then(Value::as_str),
        Some("patch-coverage-xtask-supported-editor-inline-smoke")
    );
    assert!(
        route.pointer("/coverage_proof_packs/0/commands").and_then(Value::as_array).is_some_and(
            |commands| commands.iter().any(|command| {
                command.as_str().is_some_and(|text| text.contains("supported_editor_inline_smoke"))
            })
        )
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

#[test]
fn coverage_pack_manifest_declares_supported_editor_pack() -> Result<()> {
    let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("xtask manifest path has no parent"))?
        .join(".ci/coverage-packs.toml");
    let manifest: toml::Value = toml::from_str(&fs::read_to_string(manifest_path)?)?;
    let packs = manifest
        .get("pack")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| anyhow!("coverage pack manifest must contain pack array"))?;
    let pack = packs
        .iter()
        .find(|pack| {
            pack.get("id").and_then(toml::Value::as_str)
                == Some("patch-coverage-xtask-supported-editor-inline-smoke")
        })
        .ok_or_else(|| anyhow!("missing supported-editor coverage pack"))?;
    let files = pack
        .get("files")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| anyhow!("coverage pack files must be an array"))?;
    let commands = pack
        .get("commands")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| anyhow!("coverage pack commands must be an array"))?;
    let coverage_filters = pack
        .get("coverage_filters")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| anyhow!("coverage pack filters must be an array"))?;

    assert!(
        files
            .iter()
            .filter_map(toml::Value::as_str)
            .any(|value| { value == "xtask/src/tasks/supported_editor_inline_smoke.rs" })
    );
    assert!(commands.iter().filter_map(toml::Value::as_str).any(|value| {
        value
            == "cargo test -p xtask --bin xtask --profile agent --locked supported_editor_inline_smoke -- --nocapture"
    }));
    assert!(
        coverage_filters
            .iter()
            .filter_map(toml::Value::as_str)
            .any(|value| { value == "supported_editor_inline_smoke" })
    );
    Ok(())
}
