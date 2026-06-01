use anyhow::{Result, anyhow};
use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;
use std::{fs, path::PathBuf};
use tempfile::TempDir;

#[test]
fn ci_route_cli_writes_supported_editor_proof_pack_receipt() -> Result<()> {
    let temp = TempDir::new()?;
    let receipt = temp.path().join("ci-route.json");
    let summary = temp.path().join("ci-route.md");

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
            "--summary",
            summary.to_str().ok_or_else(|| anyhow!("invalid ci route summary path"))?,
            "--changed-file",
            "xtask/src/tasks/supported_editor_inline_smoke.rs",
        ])
        .assert()
        .success();

    let route: Value = serde_json::from_str(&std::fs::read_to_string(receipt)?)?;
    assert_eq!(route.get("schema_version").and_then(Value::as_str), Some("ci-route.v1"));
    let claim_boundary = route
        .get("claim_boundary")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("missing claim_boundary"))?;
    assert!(claim_boundary.contains("CI-enforced changed-file proof routing"));
    assert!(claim_boundary.contains("Codecov / Patch 95"));
    assert!(
        !claim_boundary.contains("not enforced by CI yet"),
        "ci route receipt must not describe live coverage packs as advisory"
    );
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
    let summary = fs::read_to_string(summary)?;
    assert!(summary.contains("## Coverage Proof Packs"));
    assert!(summary.contains("patch-coverage-xtask-supported-editor-inline-smoke"));
    assert!(summary.contains("cargo test -p xtask --test supported_editor_inline_smoke_cli"));
    Ok(())
}

#[test]
fn coverage_pack_manifest_declares_supported_editor_pack() -> Result<()> {
    let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("xtask manifest path has no parent"))?
        .join(".ci/coverage-packs.toml");
    let manifest_text = fs::read_to_string(manifest_path)?;
    assert!(manifest_text.contains("CI-enforced coverage proof-pack manifest"));
    assert!(
        !manifest_text.contains("advisory until CI consumes route-selected packs directly"),
        "coverage pack manifest must not describe routed patch proof as future advisory work"
    );
    let manifest: toml::Value = toml::from_str(&manifest_text)?;
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

#[test]
fn coverage_pack_manifest_declares_ci_policy_pack_owns_classifier() -> Result<()> {
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
            pack.get("id").and_then(toml::Value::as_str) == Some("patch-coverage-ci-policy")
        })
        .ok_or_else(|| anyhow!("missing ci policy coverage pack"))?;
    let files = pack
        .get("files")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| anyhow!("coverage pack files must be an array"))?;
    let commands = pack
        .get("commands")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| anyhow!("coverage pack commands must be an array"))?;

    assert!(
        files
            .iter()
            .filter_map(toml::Value::as_str)
            .any(|value| value == "scripts/ci/ci_classify.py")
    );
    assert!(
        commands
            .iter()
            .filter_map(toml::Value::as_str)
            .any(|value| value == "python -m unittest scripts/ci/test_ci_classify.py")
    );
    Ok(())
}

#[test]
fn coverage_pack_manifest_declares_ci_route_pack_owns_python_router() -> Result<()> {
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
            pack.get("id").and_then(toml::Value::as_str) == Some("patch-coverage-ci-route")
        })
        .ok_or_else(|| anyhow!("missing ci route coverage pack"))?;
    let files = pack
        .get("files")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| anyhow!("coverage pack files must be an array"))?;
    let commands = pack
        .get("commands")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| anyhow!("coverage pack commands must be an array"))?;

    assert!(
        files
            .iter()
            .filter_map(toml::Value::as_str)
            .any(|value| value == "scripts/ci/route-codecov-packs.py")
    );
    assert!(
        commands
            .iter()
            .filter_map(toml::Value::as_str)
            .any(|value| value == "python -m unittest scripts/ci/test_route_codecov_packs.py")
    );
    Ok(())
}

#[test]
fn coverage_pack_manifest_declares_ci_actuals_pack_owns_actuals_helper() -> Result<()> {
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
            pack.get("id").and_then(toml::Value::as_str) == Some("patch-coverage-ci-actuals")
        })
        .ok_or_else(|| anyhow!("missing ci actuals coverage pack"))?;
    let files = pack
        .get("files")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| anyhow!("coverage pack files must be an array"))?;
    let commands = pack
        .get("commands")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| anyhow!("coverage pack commands must be an array"))?;

    assert!(
        files
            .iter()
            .filter_map(toml::Value::as_str)
            .any(|value| value == "scripts/ci/emit_ci_actuals.py")
    );
    assert!(
        commands
            .iter()
            .filter_map(toml::Value::as_str)
            .any(|value| value == "python -m unittest scripts/ci/test_emit_ci_actuals.py")
    );
    Ok(())
}

#[test]
fn coverage_pack_manifest_declares_ripr_summary_pack_owns_summary_helper() -> Result<()> {
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
            pack.get("id").and_then(toml::Value::as_str) == Some("patch-coverage-ripr-summary")
        })
        .ok_or_else(|| anyhow!("missing ripr summary coverage pack"))?;
    let files = pack
        .get("files")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| anyhow!("coverage pack files must be an array"))?;
    let commands = pack
        .get("commands")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| anyhow!("coverage pack commands must be an array"))?;

    assert!(
        files
            .iter()
            .filter_map(toml::Value::as_str)
            .any(|value| value == "scripts/ci/ripr_summary.py")
    );
    assert!(
        commands
            .iter()
            .filter_map(toml::Value::as_str)
            .any(|value| value == "python -m unittest scripts/ci/test_ripr_summary.py")
    );
    Ok(())
}

#[test]
fn ci_route_cli_skips_non_lcov_policy_packs_from_codecov_coverage_receipt() -> Result<()> {
    let temp = TempDir::new()?;
    let receipt = temp.path().join("ci-route.json");
    let summary = temp.path().join("ci-route.md");

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
            "--summary",
            summary.to_str().ok_or_else(|| anyhow!("invalid ci route summary path"))?,
            "--changed-file",
            "xtask/src/tasks/ci_route.rs",
        ])
        .assert()
        .success();

    let route: Value = serde_json::from_str(&std::fs::read_to_string(receipt)?)?;
    assert_eq!(route.pointer("/changed_surfaces/0").and_then(Value::as_str), Some("ci-routing"));
    assert!(
        route.get("required_proof_packs").and_then(Value::as_array).is_some_and(|packs| packs
            .iter()
            .any(|pack| { pack.get("id").and_then(Value::as_str) == Some("ci-route-receipt") })),
        "non-LCOV route surfaces still need focused proof packs"
    );
    assert!(
        route.get("coverage_pack_selector").and_then(Value::as_array).is_some_and(Vec::is_empty),
        "non-LCOV packs must not be selected for Codecov LCOV upload"
    );
    assert!(
        route.get("coverage_proof_packs").and_then(Value::as_array).is_some_and(Vec::is_empty),
        "non-LCOV packs must not appear as Codecov coverage proof packs"
    );
    assert_eq!(
        route.pointer("/skipped_by_policy/patch-coverage-ci-route").and_then(Value::as_str),
        Some("non-LCOV CI policy/routing surface; covered by focused CI gates")
    );
    let summary = fs::read_to_string(summary)?;
    assert!(summary.contains("`patch-coverage-ci-route`: non-LCOV CI policy/routing surface"));
    assert!(summary.contains("## Coverage Proof Packs"));
    assert!(summary.contains("- none"));
    Ok(())
}

#[test]
fn ci_route_cli_maps_codecov_router_script_to_route_proof_pack() -> Result<()> {
    let temp = TempDir::new()?;
    let receipt = temp.path().join("ci-route.json");
    let summary = temp.path().join("ci-route.md");

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
            "--summary",
            summary.to_str().ok_or_else(|| anyhow!("invalid ci route summary path"))?,
            "--changed-file",
            "scripts/ci/route-codecov-packs.py",
        ])
        .assert()
        .success();

    let route: Value = serde_json::from_str(&std::fs::read_to_string(receipt)?)?;
    assert_eq!(route.pointer("/changed_surfaces/0").and_then(Value::as_str), Some("ci-routing"));
    assert!(
        route.get("required_proof_packs").and_then(Value::as_array).is_some_and(|packs| packs
            .iter()
            .any(|pack| {
                pack.get("id").and_then(Value::as_str) == Some("ci-route-receipt")
                    && pack.get("commands").and_then(Value::as_array).is_some_and(|commands| {
                        commands.iter().any(|command| {
                            command.as_str()
                                == Some("python -m unittest scripts/ci/test_route_codecov_packs.py")
                        })
                    })
            })),
        "Python Codecov router changes must run the focused ci-route proof pack"
    );
    assert!(
        route.get("coverage_pack_selector").and_then(Value::as_array).is_some_and(Vec::is_empty),
        "ci-route proof pack is non-LCOV and must not be uploaded as Codecov coverage"
    );
    assert_eq!(
        route.pointer("/skipped_by_policy/patch-coverage-ci-route").and_then(Value::as_str),
        Some("non-LCOV CI policy/routing surface; covered by focused CI gates")
    );
    let summary = fs::read_to_string(summary)?;
    assert!(summary.contains("python -m unittest scripts/ci/test_route_codecov_packs.py"));
    assert!(summary.contains("`patch-coverage-ci-route`: non-LCOV CI policy/routing surface"));
    Ok(())
}

#[test]
fn ci_route_cli_maps_ci_classifier_script_to_policy_proof_pack() -> Result<()> {
    let temp = TempDir::new()?;
    let receipt = temp.path().join("ci-route.json");
    let summary = temp.path().join("ci-route.md");

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
            "--summary",
            summary.to_str().ok_or_else(|| anyhow!("invalid ci route summary path"))?,
            "--changed-file",
            "scripts/ci/ci_classify.py",
        ])
        .assert()
        .success();

    let route: Value = serde_json::from_str(&std::fs::read_to_string(receipt)?)?;
    assert_eq!(route.pointer("/changed_surfaces/0").and_then(Value::as_str), Some("ci-policy"));
    assert!(
        route.get("required_proof_packs").and_then(Value::as_array).is_some_and(|packs| packs
            .iter()
            .any(|pack| {
                pack.get("id").and_then(Value::as_str) == Some("ci-policy-focused")
                    && pack.get("commands").and_then(Value::as_array).is_some_and(|commands| {
                        commands.iter().any(|command| {
                            command.as_str()
                                == Some("python -m unittest scripts/ci/test_ci_classify.py")
                        })
                    })
            })),
        "CI classifier changes must run the focused classifier proof"
    );
    assert!(
        route.get("coverage_pack_selector").and_then(Value::as_array).is_some_and(Vec::is_empty),
        "ci-policy proof pack is non-LCOV and must not be uploaded as Codecov coverage"
    );
    assert_eq!(
        route.pointer("/skipped_by_policy/patch-coverage-ci-policy").and_then(Value::as_str),
        Some("non-LCOV CI policy/routing surface; covered by focused CI gates")
    );
    let summary = fs::read_to_string(summary)?;
    assert!(summary.contains("python -m unittest scripts/ci/test_ci_classify.py"));
    assert!(summary.contains("`patch-coverage-ci-policy`: non-LCOV CI policy/routing surface"));
    Ok(())
}

#[test]
fn ci_route_cli_maps_ci_actuals_script_to_actuals_proof_pack() -> Result<()> {
    let temp = TempDir::new()?;
    let receipt = temp.path().join("ci-route.json");
    let summary = temp.path().join("ci-route.md");

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
            "--summary",
            summary.to_str().ok_or_else(|| anyhow!("invalid ci route summary path"))?,
            "--changed-file",
            "scripts/ci/emit_ci_actuals.py",
        ])
        .assert()
        .success();

    let route: Value = serde_json::from_str(&std::fs::read_to_string(receipt)?)?;
    assert_eq!(route.pointer("/changed_surfaces/0").and_then(Value::as_str), Some("ci-actuals"));
    assert!(
        route.get("required_proof_packs").and_then(Value::as_array).is_some_and(|packs| packs
            .iter()
            .any(|pack| {
                pack.get("id").and_then(Value::as_str) == Some("ci-actuals-focused")
                    && pack.get("commands").and_then(Value::as_array).is_some_and(|commands| {
                        commands.iter().any(|command| {
                            command.as_str()
                                == Some("python -m unittest scripts/ci/test_emit_ci_actuals.py")
                        })
                    })
            })),
        "CI actuals helper changes must run the focused actuals proof"
    );
    assert!(
        route.get("coverage_pack_selector").and_then(Value::as_array).is_some_and(Vec::is_empty),
        "ci-actuals proof pack is non-LCOV and must not be uploaded as Codecov coverage"
    );
    assert_eq!(
        route.pointer("/skipped_by_policy/patch-coverage-ci-actuals").and_then(Value::as_str),
        Some("non-LCOV CI policy/routing surface; covered by focused CI gates")
    );
    let summary = fs::read_to_string(summary)?;
    assert!(summary.contains("python -m unittest scripts/ci/test_emit_ci_actuals.py"));
    assert!(summary.contains("`patch-coverage-ci-actuals`: non-LCOV CI policy/routing surface"));
    Ok(())
}

#[test]
fn ci_route_cli_maps_ripr_summary_script_to_summary_proof_pack() -> Result<()> {
    let temp = TempDir::new()?;
    let receipt = temp.path().join("ci-route.json");
    let summary = temp.path().join("ci-route.md");

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
            "--summary",
            summary.to_str().ok_or_else(|| anyhow!("invalid ci route summary path"))?,
            "--changed-file",
            "scripts/ci/ripr_summary.py",
        ])
        .assert()
        .success();

    let route: Value = serde_json::from_str(&std::fs::read_to_string(receipt)?)?;
    assert_eq!(route.pointer("/changed_surfaces/0").and_then(Value::as_str), Some("ripr-summary"));
    assert!(
        route.get("required_proof_packs").and_then(Value::as_array).is_some_and(|packs| packs
            .iter()
            .any(|pack| {
                pack.get("id").and_then(Value::as_str) == Some("ripr-summary-focused")
                    && pack.get("commands").and_then(Value::as_array).is_some_and(|commands| {
                        commands.iter().any(|command| {
                            command.as_str()
                                == Some("python -m unittest scripts/ci/test_ripr_summary.py")
                        })
                    })
            })),
        "RIPR summary helper changes must run the focused summary proof"
    );
    assert!(
        route.get("coverage_pack_selector").and_then(Value::as_array).is_some_and(Vec::is_empty),
        "ripr-summary proof pack is non-LCOV and must not be uploaded as Codecov coverage"
    );
    assert_eq!(
        route.pointer("/skipped_by_policy/patch-coverage-ripr-summary").and_then(Value::as_str),
        Some("non-LCOV CI policy/routing surface; covered by focused CI gates")
    );
    let summary = fs::read_to_string(summary)?;
    assert!(summary.contains("python -m unittest scripts/ci/test_ripr_summary.py"));
    assert!(summary.contains("`patch-coverage-ripr-summary`: non-LCOV CI policy/routing surface"));
    Ok(())
}

#[test]
fn ci_route_cli_reports_lcov_pack_that_only_matched_test_files() -> Result<()> {
    let temp = TempDir::new()?;
    let receipt = temp.path().join("ci-route.json");
    let summary = temp.path().join("ci-route.md");

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
            "--summary",
            summary.to_str().ok_or_else(|| anyhow!("invalid ci route summary path"))?,
            "--changed-file",
            "xtask/tests/semantic_inline_receipts_cli.rs",
        ])
        .assert()
        .success();

    let route: Value = serde_json::from_str(&std::fs::read_to_string(receipt)?)?;
    assert_eq!(
        route.pointer("/changed_surfaces/0").and_then(Value::as_str),
        Some("xtask-semantic-inline-receipts")
    );
    assert!(
        route.get("required_proof_packs").and_then(Value::as_array).is_some_and(|packs| packs
            .iter()
            .any(|pack| {
                pack.get("id").and_then(Value::as_str) == Some("xtask-semantic-inline-receipts")
            })),
        "test-only LCOV matches still need focused proof packs"
    );
    assert!(
        route.get("coverage_pack_selector").and_then(Value::as_array).is_some_and(Vec::is_empty),
        "test-only LCOV matches must not be selected for Codecov LCOV upload"
    );
    assert_eq!(
        route
            .pointer("/skipped_by_policy/patch-coverage-xtask-semantic-inline")
            .and_then(Value::as_str),
        Some("LCOV coverage pack matched only non-source files; covered by focused CI gates")
    );
    let summary = fs::read_to_string(summary)?;
    assert!(summary.contains(
        "`patch-coverage-xtask-semantic-inline`: LCOV coverage pack matched only non-source files"
    ));
    assert!(summary.contains("## Coverage Proof Packs"));
    assert!(summary.contains("- none"));
    Ok(())
}
