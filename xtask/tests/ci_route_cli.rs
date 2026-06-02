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
fn coverage_pack_manifest_declares_completion_core_pack() -> Result<()> {
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
            pack.get("id").and_then(toml::Value::as_str) == Some("patch-coverage-completion-core")
        })
        .ok_or_else(|| anyhow!("missing completion core coverage pack"))?;
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
            .any(|value| { value == "crates/perl-lsp-rs-core/src/providers/completion/" })
    );
    assert!(commands.iter().filter_map(toml::Value::as_str).any(|value| {
        value
            == "cargo test -p perl-lsp-rs-core --lib --profile agent --locked completion::completion -- --nocapture"
    }));
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
fn coverage_pack_manifest_declares_learned_estimate_pack_owns_estimate_helper() -> Result<()> {
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
            pack.get("id").and_then(toml::Value::as_str) == Some("patch-coverage-learned-estimate")
        })
        .ok_or_else(|| anyhow!("missing learned estimate coverage pack"))?;
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
            .any(|value| value == "scripts/ci/learned_estimate.py")
    );
    assert!(
        commands
            .iter()
            .filter_map(toml::Value::as_str)
            .any(|value| { value == "python -m unittest scripts/ci/test_learned_estimate.py" })
    );
    Ok(())
}

#[test]
fn coverage_pack_manifest_declares_risk_pack_validator_pack_owns_validator_helper() -> Result<()> {
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
                == Some("patch-coverage-risk-packs-validator")
        })
        .ok_or_else(|| anyhow!("missing risk-packs validator coverage pack"))?;
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
            .any(|value| value == "scripts/ci/validate_risk_packs.py")
    );
    assert!(
        commands
            .iter()
            .filter_map(toml::Value::as_str)
            .any(|value| { value == "python -m unittest scripts/ci/test_validate_risk_packs.py" })
    );
    Ok(())
}

#[test]
fn coverage_pack_manifest_declares_gate_lane_mapping_pack_owns_validator_helper() -> Result<()> {
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
            pack.get("id").and_then(toml::Value::as_str) == Some("patch-coverage-gate-lane-mapping")
        })
        .ok_or_else(|| anyhow!("missing gate-lane mapping coverage pack"))?;
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
            .any(|value| value == "scripts/ci/validate_gate_lane_mapping.py")
    );
    assert!(commands.iter().filter_map(toml::Value::as_str).any(|value| {
        value == "python -m unittest scripts/ci/test_validate_gate_lane_mapping.py"
    }));
    Ok(())
}

#[test]
fn coverage_pack_manifest_declares_trust_lanes_validator_pack_owns_validator_helper() -> Result<()>
{
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
                == Some("patch-coverage-trust-lanes-validator")
        })
        .ok_or_else(|| anyhow!("missing trust-lanes validator coverage pack"))?;
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
            .any(|value| value == "scripts/ci/validate_trust_lanes.py")
    );
    assert!(
        commands
            .iter()
            .filter_map(toml::Value::as_str)
            .any(|value| value == "python -m unittest scripts/ci/test_validate_trust_lanes.py")
    );
    Ok(())
}

#[test]
fn coverage_pack_manifest_declares_receipts_junit_pack_owns_junit_helper() -> Result<()> {
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
            pack.get("id").and_then(toml::Value::as_str) == Some("patch-coverage-receipts-junit")
        })
        .ok_or_else(|| anyhow!("missing receipts-junit coverage pack"))?;
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
            .any(|value| value == "scripts/ci/receipts-to-junit.py")
    );
    assert!(
        commands
            .iter()
            .filter_map(toml::Value::as_str)
            .any(|value| value == "python -m unittest scripts/ci/test_receipts_to_junit.py")
    );
    Ok(())
}

#[test]
fn coverage_pack_manifest_declares_core_package_validator_pack_owns_validator_helper() -> Result<()>
{
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
                == Some("patch-coverage-core-package-validator")
        })
        .ok_or_else(|| anyhow!("missing core-package validator coverage pack"))?;
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
            .any(|value| value == "scripts/ci/check_perl_lsp_rs_core_package.py")
    );
    assert!(commands.iter().filter_map(toml::Value::as_str).any(|value| {
        value == "python -m unittest scripts/ci/test_check_perl_lsp_rs_core_package.py"
    }));
    Ok(())
}

#[test]
fn coverage_pack_manifest_declares_aggregate_lane_history_pack_owns_history_helper() -> Result<()> {
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
                == Some("patch-coverage-aggregate-lane-history")
        })
        .ok_or_else(|| anyhow!("missing aggregate-lane-history coverage pack"))?;
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
            .any(|value| value == "scripts/ci/aggregate_lane_history.py")
    );
    assert!(
        commands
            .iter()
            .filter_map(toml::Value::as_str)
            .any(|value| value == "python -m unittest scripts/ci/test_aggregate_lane_history.py")
    );
    Ok(())
}

#[test]
fn coverage_pack_manifest_declares_pr_plan_pack_owns_plan_helper() -> Result<()> {
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
        .find(|pack| pack.get("id").and_then(toml::Value::as_str) == Some("patch-coverage-pr-plan"))
        .ok_or_else(|| anyhow!("missing pr-plan coverage pack"))?;
    let files = pack
        .get("files")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| anyhow!("coverage pack files must be an array"))?;
    let commands = pack
        .get("commands")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| anyhow!("coverage pack commands must be an array"))?;

    assert!(
        files.iter().filter_map(toml::Value::as_str).any(|value| value == "scripts/ci/pr_plan.py")
    );
    assert!(
        commands
            .iter()
            .filter_map(toml::Value::as_str)
            .any(|value| value == "python -m unittest scripts/ci/test_pr_plan.py")
    );
    Ok(())
}

#[test]
fn coverage_pack_manifest_declares_pr_overlap_pack_owns_overlap_helper() -> Result<()> {
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
            pack.get("id").and_then(toml::Value::as_str) == Some("patch-coverage-pr-overlap")
        })
        .ok_or_else(|| anyhow!("missing pr-overlap coverage pack"))?;
    let files = pack
        .get("files")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| anyhow!("coverage pack files must be an array"))?;
    let commands = pack
        .get("commands")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| anyhow!("coverage pack commands must be an array"))?;

    assert!(
        files.iter().filter_map(toml::Value::as_str).any(|value| value == "scripts/pr_overlap.py")
    );
    assert!(
        commands
            .iter()
            .filter_map(toml::Value::as_str)
            .any(|value| value == "python scripts/tests/test_pr_overlap.py")
    );
    Ok(())
}

#[test]
fn coverage_pack_manifest_declares_control_plane_lock_pack_owns_lock_helper() -> Result<()> {
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
                == Some("patch-coverage-control-plane-lock")
        })
        .ok_or_else(|| anyhow!("missing control-plane-lock coverage pack"))?;
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
            .any(|value| value == "scripts/control-plane-lock.sh")
    );
    assert!(
        commands
            .iter()
            .filter_map(toml::Value::as_str)
            .any(|value| value == "bash scripts/test-control-plane-lock.sh")
    );
    Ok(())
}

#[test]
fn coverage_pack_manifest_declares_agent_preflight_pack_owns_preflight_helper() -> Result<()> {
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
            pack.get("id").and_then(toml::Value::as_str) == Some("patch-coverage-agent-preflight")
        })
        .ok_or_else(|| anyhow!("missing agent-preflight coverage pack"))?;
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
            .any(|value| value == "scripts/agent-preflight.sh")
    );
    assert!(
        commands
            .iter()
            .filter_map(toml::Value::as_str)
            .any(|value| value == "bash scripts/test-agent-preflight.sh")
    );
    Ok(())
}

#[test]
fn coverage_pack_manifest_declares_preflight_pack_owns_preflight_wrapper() -> Result<()> {
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
            pack.get("id").and_then(toml::Value::as_str) == Some("patch-coverage-preflight-wrapper")
        })
        .ok_or_else(|| anyhow!("missing preflight wrapper coverage pack"))?;
    let files = pack
        .get("files")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| anyhow!("coverage pack files must be an array"))?;
    let commands = pack
        .get("commands")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| anyhow!("coverage pack commands must be an array"))?;

    assert!(
        files.iter().filter_map(toml::Value::as_str).any(|value| value == "scripts/preflight.sh")
    );
    assert!(
        commands
            .iter()
            .filter_map(toml::Value::as_str)
            .any(|value| value == "bash scripts/tests/test-preflight-wrapper.sh")
    );
    Ok(())
}

#[test]
fn coverage_pack_manifest_declares_install_githooks_pack_owns_githooks_wrapper() -> Result<()> {
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
                == Some("patch-coverage-install-githooks-wrapper")
        })
        .ok_or_else(|| anyhow!("missing install-githooks wrapper coverage pack"))?;
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
            .any(|value| { value == "scripts/install-githooks.sh" })
    );
    assert!(
        commands
            .iter()
            .filter_map(toml::Value::as_str)
            .any(|value| { value == "bash scripts/tests/test-install-githooks-wrapper.sh" })
    );
    Ok(())
}

#[test]
fn coverage_pack_manifest_declares_e2e_gate_pack_owns_e2e_wrapper() -> Result<()> {
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
            pack.get("id").and_then(toml::Value::as_str) == Some("patch-coverage-e2e-gate-wrapper")
        })
        .ok_or_else(|| anyhow!("missing e2e-gate wrapper coverage pack"))?;
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
            .any(|value| { value == "scripts/e2e-gate.sh" })
    );
    assert!(
        commands
            .iter()
            .filter_map(toml::Value::as_str)
            .any(|value| { value == "bash scripts/tests/test-e2e-gate-wrapper.sh" })
    );
    Ok(())
}

#[test]
fn coverage_pack_manifest_declares_execute_gate_pack_owns_execute_wrapper() -> Result<()> {
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
                == Some("patch-coverage-execute-gate-wrapper")
        })
        .ok_or_else(|| anyhow!("missing execute-gate wrapper coverage pack"))?;
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
            .any(|value| { value == "scripts/execute-gate.sh" })
    );
    assert!(
        commands
            .iter()
            .filter_map(toml::Value::as_str)
            .any(|value| { value == "bash scripts/tests/test-execute-gate-wrapper.sh" })
    );
    Ok(())
}

#[test]
fn coverage_pack_manifest_declares_run_gates_pack_owns_run_gates_wrapper() -> Result<()> {
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
            pack.get("id").and_then(toml::Value::as_str) == Some("patch-coverage-run-gates-wrapper")
        })
        .ok_or_else(|| anyhow!("missing run-gates wrapper coverage pack"))?;
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
            .any(|value| { value == "scripts/run-gates.sh" })
    );
    assert!(
        commands
            .iter()
            .filter_map(toml::Value::as_str)
            .any(|value| { value == "bash scripts/tests/test-run-gates-wrapper.sh" })
    );
    Ok(())
}

#[test]
fn coverage_pack_manifest_declares_gate_local_pack_owns_gate_local_wrapper() -> Result<()> {
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
                == Some("patch-coverage-gate-local-wrapper")
        })
        .ok_or_else(|| anyhow!("missing gate-local wrapper coverage pack"))?;
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
            .any(|value| { value == "scripts/gate-local.sh" })
    );
    assert!(
        commands
            .iter()
            .filter_map(toml::Value::as_str)
            .any(|value| { value == "bash scripts/tests/test-gate-local-wrapper.sh" })
    );
    Ok(())
}

#[test]
fn coverage_pack_manifest_declares_list_gates_pack_owns_list_gates_wrapper() -> Result<()> {
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
                == Some("patch-coverage-list-gates-wrapper")
        })
        .ok_or_else(|| anyhow!("missing list-gates wrapper coverage pack"))?;
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
            .any(|value| { value == "scripts/list-gates.py" })
    );
    assert!(
        commands
            .iter()
            .filter_map(toml::Value::as_str)
            .any(|value| { value == "python scripts/tests/test-list-gates-wrapper.py" })
    );
    Ok(())
}

#[test]
fn coverage_pack_manifest_declares_forbid_fatal_pack_owns_forbid_fatal_wrapper() -> Result<()> {
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
                == Some("patch-coverage-forbid-fatal-constructs-wrapper")
        })
        .ok_or_else(|| anyhow!("missing forbid-fatal-constructs wrapper coverage pack"))?;
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
            .any(|value| { value == "scripts/forbid-fatal-constructs.sh" })
    );
    assert!(
        commands
            .iter()
            .filter_map(toml::Value::as_str)
            .any(|value| { value == "bash scripts/tests/test-forbid-fatal-constructs-wrapper.sh" })
    );
    Ok(())
}

#[test]
fn coverage_pack_manifest_declares_dead_code_pack_owns_dead_code_wrapper() -> Result<()> {
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
            pack.get("id").and_then(toml::Value::as_str) == Some("patch-coverage-dead-code-wrapper")
        })
        .ok_or_else(|| anyhow!("missing dead-code wrapper coverage pack"))?;
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
            .any(|value| { value == "scripts/dead-code-check.sh" })
    );
    assert!(
        commands
            .iter()
            .filter_map(toml::Value::as_str)
            .any(|value| { value == "bash scripts/tests/test-dead-code-wrapper.sh" })
    );
    Ok(())
}

#[test]
fn coverage_pack_manifest_declares_check_toolchain_pack_owns_toolchain_wrapper() -> Result<()> {
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
                == Some("patch-coverage-check-toolchain-wrapper")
        })
        .ok_or_else(|| anyhow!("missing check-toolchain wrapper coverage pack"))?;
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
            .any(|value| value == "scripts/check-rust-toolchain.sh")
    );
    assert!(
        files
            .iter()
            .filter_map(toml::Value::as_str)
            .any(|value| value == "scripts/tests/test-check-rust-toolchain-wrapper.sh")
    );
    assert!(
        commands
            .iter()
            .filter_map(toml::Value::as_str)
            .any(|value| value == "bash scripts/tests/test-check-rust-toolchain-wrapper.sh")
    );
    Ok(())
}

#[test]
fn coverage_pack_manifest_declares_coverage_baseline_pack_owns_baseline_helper() -> Result<()> {
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
            pack.get("id").and_then(toml::Value::as_str) == Some("patch-coverage-baseline-script")
        })
        .ok_or_else(|| anyhow!("missing coverage-baseline script coverage pack"))?;
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
            .any(|value| value == "scripts/check-coverage-baseline.sh")
    );
    assert!(
        commands
            .iter()
            .filter_map(toml::Value::as_str)
            .any(|value| value == "bash scripts/tests/test-check-coverage-baseline.sh")
    );
    Ok(())
}

#[test]
fn coverage_pack_manifest_declares_update_coverage_baseline_pack_owns_baseline_helper() -> Result<()>
{
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
                == Some("patch-coverage-update-baseline-script")
        })
        .ok_or_else(|| anyhow!("missing update-coverage-baseline script coverage pack"))?;
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
            .any(|value| value == "scripts/update-coverage-baseline.sh")
    );
    assert!(
        commands
            .iter()
            .filter_map(toml::Value::as_str)
            .any(|value| value == "bash scripts/tests/test-update-coverage-baseline.sh")
    );
    Ok(())
}

#[test]
fn coverage_pack_manifest_declares_generate_receipt_pack_owns_receipt_helper() -> Result<()> {
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
                == Some("patch-coverage-generate-receipt-script")
        })
        .ok_or_else(|| anyhow!("missing generate-receipt script coverage pack"))?;
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
            .any(|value| value == "scripts/generate-receipt.sh")
    );
    assert!(
        commands
            .iter()
            .filter_map(toml::Value::as_str)
            .any(|value| value == "bash scripts/tests/test-generate-receipt.sh")
    );
    Ok(())
}

#[test]
fn coverage_pack_manifest_declares_quick_receipts_pack_owns_receipt_wrapper() -> Result<()> {
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
                == Some("patch-coverage-quick-receipts-wrapper")
        })
        .ok_or_else(|| anyhow!("missing quick-receipts wrapper coverage pack"))?;
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
            .any(|value| value == "scripts/quick-receipts.sh")
    );
    assert!(
        commands
            .iter()
            .filter_map(toml::Value::as_str)
            .any(|value| value == "bash scripts/tests/test-quick-receipts-wrapper.sh")
    );
    Ok(())
}

#[test]
fn coverage_pack_manifest_declares_generate_badges_pack_owns_badges_wrapper() -> Result<()> {
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
                == Some("patch-coverage-generate-badges-wrapper")
        })
        .ok_or_else(|| anyhow!("missing generate-badges wrapper coverage pack"))?;
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
            .any(|value| value == "scripts/generate-badges.sh")
    );
    assert!(
        commands
            .iter()
            .filter_map(toml::Value::as_str)
            .any(|value| value == "bash scripts/tests/test-generate-badges-wrapper.sh")
    );
    Ok(())
}

#[test]
fn coverage_pack_manifest_declares_ignored_test_count_pack_owns_count_wrapper() -> Result<()> {
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
                == Some("patch-coverage-ignored-test-count-wrapper")
        })
        .ok_or_else(|| anyhow!("missing ignored-test-count wrapper coverage pack"))?;
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
            .any(|value| value == "scripts/ignored-test-count.sh")
    );
    assert!(
        commands
            .iter()
            .filter_map(toml::Value::as_str)
            .any(|value| value == "bash scripts/tests/test-ignored-test-count-wrapper.sh")
    );
    Ok(())
}

#[test]
fn coverage_pack_manifest_declares_clean_tmp_targets_pack_owns_cleanup_helper() -> Result<()> {
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
            pack.get("id").and_then(toml::Value::as_str) == Some("patch-coverage-clean-tmp-targets")
        })
        .ok_or_else(|| anyhow!("missing clean-tmp-targets coverage pack"))?;
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
            .any(|value| value == "scripts/clean-tmp-targets.sh")
    );
    assert!(
        commands
            .iter()
            .filter_map(toml::Value::as_str)
            .any(|value| value == "bash scripts/tests/test-clean-tmp-targets.sh")
    );
    Ok(())
}

#[test]
fn coverage_pack_manifest_declares_swarm_cleanup_pack_owns_cleanup_helpers() -> Result<()> {
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
            pack.get("id").and_then(toml::Value::as_str) == Some("patch-coverage-swarm-cleanup")
        })
        .ok_or_else(|| anyhow!("missing swarm-cleanup coverage pack"))?;
    let files = pack
        .get("files")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| anyhow!("coverage pack files must be an array"))?;
    let commands = pack
        .get("commands")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| anyhow!("coverage pack commands must be an array"))?;

    assert!(
        files.iter().filter_map(toml::Value::as_str).any(|value| value == "scripts/swarm-clean")
    );
    assert!(
        files.iter().filter_map(toml::Value::as_str).any(|value| value == "scripts/swarm-doctor")
    );
    assert!(
        commands
            .iter()
            .filter_map(toml::Value::as_str)
            .any(|value| value == "bash scripts/tests/test_swarm_clean.sh")
    );
    assert!(
        commands
            .iter()
            .filter_map(toml::Value::as_str)
            .any(|value| value == "bash scripts/tests/test_swarm_doctor.sh")
    );
    Ok(())
}

#[test]
fn coverage_pack_manifest_declares_pre_merge_check_pack_owns_pre_merge_helper() -> Result<()> {
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
            pack.get("id").and_then(toml::Value::as_str) == Some("patch-coverage-pre-merge-check")
        })
        .ok_or_else(|| anyhow!("missing pre-merge-check coverage pack"))?;
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
            .any(|value| value == "scripts/pre-merge-check.sh")
    );
    assert!(
        commands
            .iter()
            .filter_map(toml::Value::as_str)
            .any(|value| value == "bash scripts/tests/test-pre-merge-check.sh")
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
fn ci_route_cli_keeps_inline_quality_focused_but_out_of_codecov_lcov() -> Result<()> {
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
            "xtask/src/tasks/inline_completion_quality.rs",
        ])
        .assert()
        .success();

    let route: Value = serde_json::from_str(&std::fs::read_to_string(receipt)?)?;
    assert_eq!(
        route.pointer("/changed_surfaces/0").and_then(Value::as_str),
        Some("xtask-inline-completion-quality")
    );
    assert!(
        route.get("required_proof_packs").and_then(Value::as_array).is_some_and(|packs| packs
            .iter()
            .any(|pack| {
                pack.get("id").and_then(Value::as_str) == Some("xtask-inline-completion-quality")
                    && pack.get("commands").and_then(Value::as_array).is_some_and(|commands| {
                        commands.iter().any(|command| {
                            command.as_str().is_some_and(|command| {
                                command.contains("inline-completion-quality")
                            })
                        })
                    })
            })),
        "inline quality changes must still require their focused receipt command"
    );
    assert!(
        route.get("coverage_pack_selector").and_then(Value::as_array).is_some_and(Vec::is_empty),
        "inline quality receipt harness must not select Codecov LCOV proof"
    );
    assert!(
        route.get("coverage_proof_packs").and_then(Value::as_array).is_some_and(Vec::is_empty),
        "inline quality receipt harness must not appear as a Codecov coverage proof pack"
    );
    assert_eq!(
        route
            .pointer("/skipped_by_policy/patch-coverage-xtask-inline-quality")
            .and_then(Value::as_str),
        Some("non-LCOV CI policy/routing surface; covered by focused CI gates")
    );
    let summary = fs::read_to_string(summary)?;
    assert!(
        summary
            .contains("`patch-coverage-xtask-inline-quality`: non-LCOV CI policy/routing surface")
    );
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
fn ci_route_cli_maps_learned_estimate_script_to_estimate_proof_pack() -> Result<()> {
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
            "scripts/ci/learned_estimate.py",
        ])
        .assert()
        .success();

    let route: Value = serde_json::from_str(&std::fs::read_to_string(receipt)?)?;
    assert_eq!(
        route.pointer("/changed_surfaces/0").and_then(Value::as_str),
        Some("learned-estimate")
    );
    assert!(
        route.get("required_proof_packs").and_then(Value::as_array).is_some_and(|packs| packs
            .iter()
            .any(|pack| {
                pack.get("id").and_then(Value::as_str) == Some("learned-estimate-focused")
                    && pack.get("commands").and_then(Value::as_array).is_some_and(|commands| {
                        commands.iter().any(|command| {
                            command.as_str()
                                == Some("python -m unittest scripts/ci/test_learned_estimate.py")
                        })
                    })
            })),
        "learned LEM helper changes must run the focused estimate proof"
    );
    assert!(
        route.get("coverage_pack_selector").and_then(Value::as_array).is_some_and(Vec::is_empty),
        "learned-estimate proof pack is non-LCOV and must not be uploaded as Codecov coverage"
    );
    assert_eq!(
        route.pointer("/skipped_by_policy/patch-coverage-learned-estimate").and_then(Value::as_str),
        Some("non-LCOV CI policy/routing surface; covered by focused CI gates")
    );
    let summary = fs::read_to_string(summary)?;
    assert!(summary.contains("python -m unittest scripts/ci/test_learned_estimate.py"));
    assert!(
        summary.contains("`patch-coverage-learned-estimate`: non-LCOV CI policy/routing surface")
    );
    Ok(())
}

#[test]
fn ci_route_cli_maps_risk_pack_validator_script_to_validator_proof_pack() -> Result<()> {
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
            "scripts/ci/validate_risk_packs.py",
        ])
        .assert()
        .success();

    let route: Value = serde_json::from_str(&std::fs::read_to_string(receipt)?)?;
    assert_eq!(
        route.pointer("/changed_surfaces/0").and_then(Value::as_str),
        Some("risk-packs-validator")
    );
    assert!(
        route.get("required_proof_packs").and_then(Value::as_array).is_some_and(|packs| packs
            .iter()
            .any(|pack| {
                pack.get("id").and_then(Value::as_str) == Some("risk-packs-validator-focused")
                    && pack.get("commands").and_then(Value::as_array).is_some_and(|commands| {
                        commands.iter().any(|command| {
                            command.as_str()
                                == Some("python -m unittest scripts/ci/test_validate_risk_packs.py")
                        })
                    })
            })),
        "risk-pack validator changes must run the focused validator proof"
    );
    assert!(
        route.get("coverage_pack_selector").and_then(Value::as_array).is_some_and(Vec::is_empty),
        "risk-packs-validator proof pack is non-LCOV and must not be uploaded as Codecov coverage"
    );
    assert_eq!(
        route
            .pointer("/skipped_by_policy/patch-coverage-risk-packs-validator")
            .and_then(Value::as_str),
        Some("non-LCOV CI policy/routing surface; covered by focused CI gates")
    );
    let summary = fs::read_to_string(summary)?;
    assert!(summary.contains("python -m unittest scripts/ci/test_validate_risk_packs.py"));
    assert!(
        summary
            .contains("`patch-coverage-risk-packs-validator`: non-LCOV CI policy/routing surface")
    );
    Ok(())
}

#[test]
fn ci_route_cli_maps_gate_lane_mapping_script_to_mapping_proof_pack() -> Result<()> {
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
            "scripts/ci/validate_gate_lane_mapping.py",
        ])
        .assert()
        .success();

    let route: Value = serde_json::from_str(&std::fs::read_to_string(receipt)?)?;
    assert_eq!(
        route.pointer("/changed_surfaces/0").and_then(Value::as_str),
        Some("gate-lane-mapping")
    );
    assert!(
        route.get("required_proof_packs").and_then(Value::as_array).is_some_and(|packs| packs
            .iter()
            .any(|pack| {
                pack.get("id").and_then(Value::as_str) == Some("gate-lane-mapping-focused")
                    && pack.get("commands").and_then(Value::as_array).is_some_and(|commands| {
                        commands.iter().any(|command| {
                            command.as_str()
                                == Some(
                                    "python -m unittest scripts/ci/test_validate_gate_lane_mapping.py"
                                )
                        })
                    })
            })),
        "gate-lane mapping changes must run the focused validator proof"
    );
    assert!(
        route.get("coverage_pack_selector").and_then(Value::as_array).is_some_and(Vec::is_empty),
        "gate-lane mapping proof pack is non-LCOV and must not be uploaded as Codecov coverage"
    );
    assert_eq!(
        route
            .pointer("/skipped_by_policy/patch-coverage-gate-lane-mapping")
            .and_then(Value::as_str),
        Some("non-LCOV CI policy/routing surface; covered by focused CI gates")
    );
    let summary = fs::read_to_string(summary)?;
    assert!(summary.contains("python -m unittest scripts/ci/test_validate_gate_lane_mapping.py"));
    assert!(
        summary.contains("`patch-coverage-gate-lane-mapping`: non-LCOV CI policy/routing surface")
    );
    Ok(())
}

#[test]
fn ci_route_cli_maps_trust_lanes_script_to_validator_proof_pack() -> Result<()> {
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
            "scripts/ci/validate_trust_lanes.py",
        ])
        .assert()
        .success();

    let route: Value = serde_json::from_str(&std::fs::read_to_string(receipt)?)?;
    assert_eq!(
        route.pointer("/changed_surfaces/0").and_then(Value::as_str),
        Some("trust-lanes-validator")
    );
    assert!(
        route.get("required_proof_packs").and_then(Value::as_array).is_some_and(|packs| packs
            .iter()
            .any(|pack| {
                pack.get("id").and_then(Value::as_str) == Some("trust-lanes-validator-focused")
                    && pack.get("commands").and_then(Value::as_array).is_some_and(|commands| {
                        commands.iter().any(|command| {
                            command.as_str()
                                == Some(
                                    "python -m unittest scripts/ci/test_validate_trust_lanes.py",
                                )
                        })
                    })
            })),
        "trust-lane validator changes must run the focused validator proof"
    );
    assert!(
        route.get("coverage_pack_selector").and_then(Value::as_array).is_some_and(Vec::is_empty),
        "trust-lanes validator proof pack is non-LCOV and must not be uploaded as Codecov coverage"
    );
    assert_eq!(
        route
            .pointer("/skipped_by_policy/patch-coverage-trust-lanes-validator")
            .and_then(Value::as_str),
        Some("non-LCOV CI policy/routing surface; covered by focused CI gates")
    );
    let summary = fs::read_to_string(summary)?;
    assert!(summary.contains("python -m unittest scripts/ci/test_validate_trust_lanes.py"));
    assert!(
        summary
            .contains("`patch-coverage-trust-lanes-validator`: non-LCOV CI policy/routing surface")
    );
    Ok(())
}

#[test]
fn ci_route_cli_maps_receipts_junit_script_to_junit_proof_pack() -> Result<()> {
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
            "scripts/ci/receipts-to-junit.py",
        ])
        .assert()
        .success();

    let route: Value = serde_json::from_str(&std::fs::read_to_string(receipt)?)?;
    assert_eq!(
        route.pointer("/changed_surfaces/0").and_then(Value::as_str),
        Some("receipts-junit")
    );
    assert!(
        route.get("required_proof_packs").and_then(Value::as_array).is_some_and(|packs| packs
            .iter()
            .any(|pack| {
                pack.get("id").and_then(Value::as_str) == Some("receipts-junit-focused")
                    && pack.get("commands").and_then(Value::as_array).is_some_and(|commands| {
                        commands.iter().any(|command| {
                            command.as_str()
                                == Some("python -m unittest scripts/ci/test_receipts_to_junit.py")
                        })
                    })
            })),
        "receipt-to-JUnit changes must run the focused JUnit proof"
    );
    assert!(
        route.get("coverage_pack_selector").and_then(Value::as_array).is_some_and(Vec::is_empty),
        "receipts-junit proof pack is non-LCOV and must not be uploaded as Codecov coverage"
    );
    assert_eq!(
        route.pointer("/skipped_by_policy/patch-coverage-receipts-junit").and_then(Value::as_str),
        Some("non-LCOV CI policy/routing surface; covered by focused CI gates")
    );
    let summary = fs::read_to_string(summary)?;
    assert!(summary.contains("python -m unittest scripts/ci/test_receipts_to_junit.py"));
    assert!(
        summary.contains("`patch-coverage-receipts-junit`: non-LCOV CI policy/routing surface")
    );
    Ok(())
}

#[test]
fn ci_route_cli_maps_core_package_validator_script_to_validator_proof_pack() -> Result<()> {
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
            "scripts/ci/check_perl_lsp_rs_core_package.py",
        ])
        .assert()
        .success();

    let route: Value = serde_json::from_str(&std::fs::read_to_string(receipt)?)?;
    assert_eq!(
        route.pointer("/changed_surfaces/0").and_then(Value::as_str),
        Some("core-package-validator")
    );
    assert!(
        route.get("required_proof_packs").and_then(Value::as_array).is_some_and(|packs| packs
            .iter()
            .any(|pack| {
                pack.get("id").and_then(Value::as_str) == Some("core-package-validator-focused")
                    && pack.get("commands").and_then(Value::as_array).is_some_and(|commands| {
                        commands.iter().any(|command| {
                            command.as_str()
                                == Some(
                                    "python -m unittest scripts/ci/test_check_perl_lsp_rs_core_package.py",
                                )
                        })
                    })
            })),
        "core package validator changes must run the focused package proof"
    );
    assert!(
        route.get("coverage_pack_selector").and_then(Value::as_array).is_some_and(Vec::is_empty),
        "core-package validator proof pack is non-LCOV and must not be uploaded as Codecov coverage"
    );
    assert_eq!(
        route
            .pointer("/skipped_by_policy/patch-coverage-core-package-validator")
            .and_then(Value::as_str),
        Some("non-LCOV CI policy/routing surface; covered by focused CI gates")
    );
    let summary = fs::read_to_string(summary)?;
    assert!(
        summary.contains("python -m unittest scripts/ci/test_check_perl_lsp_rs_core_package.py")
    );
    assert!(
        summary.contains(
            "`patch-coverage-core-package-validator`: non-LCOV CI policy/routing surface"
        )
    );
    Ok(())
}

#[test]
fn ci_route_cli_maps_aggregate_lane_history_script_to_history_proof_pack() -> Result<()> {
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
            "scripts/ci/aggregate_lane_history.py",
        ])
        .assert()
        .success();

    let route: Value = serde_json::from_str(&std::fs::read_to_string(receipt)?)?;
    assert_eq!(
        route.pointer("/changed_surfaces/0").and_then(Value::as_str),
        Some("aggregate-lane-history")
    );
    assert!(
        route.get("required_proof_packs").and_then(Value::as_array).is_some_and(|packs| packs
            .iter()
            .any(|pack| {
                pack.get("id").and_then(Value::as_str) == Some("aggregate-lane-history-focused")
                    && pack.get("commands").and_then(Value::as_array).is_some_and(|commands| {
                        commands.iter().any(|command| {
                            command.as_str()
                                == Some(
                                    "python -m unittest scripts/ci/test_aggregate_lane_history.py",
                                )
                        })
                    })
            })),
        "aggregate lane history changes must run the focused history proof"
    );
    assert!(
        route.get("coverage_pack_selector").and_then(Value::as_array).is_some_and(Vec::is_empty),
        "aggregate-lane-history proof pack is non-LCOV and must not be uploaded as Codecov coverage"
    );
    assert_eq!(
        route
            .pointer("/skipped_by_policy/patch-coverage-aggregate-lane-history")
            .and_then(Value::as_str),
        Some("non-LCOV CI policy/routing surface; covered by focused CI gates")
    );
    let summary = fs::read_to_string(summary)?;
    assert!(summary.contains("python -m unittest scripts/ci/test_aggregate_lane_history.py"));
    assert!(
        summary.contains(
            "`patch-coverage-aggregate-lane-history`: non-LCOV CI policy/routing surface"
        )
    );
    Ok(())
}

#[test]
fn ci_route_cli_maps_pr_plan_script_to_plan_proof_pack() -> Result<()> {
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
            "scripts/ci/pr_plan.py",
        ])
        .assert()
        .success();

    let route: Value = serde_json::from_str(&std::fs::read_to_string(receipt)?)?;
    assert_eq!(route.pointer("/changed_surfaces/0").and_then(Value::as_str), Some("pr-plan"));
    assert!(
        route.get("required_proof_packs").and_then(Value::as_array).is_some_and(|packs| packs
            .iter()
            .any(|pack| {
                pack.get("id").and_then(Value::as_str) == Some("pr-plan-focused")
                    && pack.get("commands").and_then(Value::as_array).is_some_and(|commands| {
                        commands.iter().any(|command| {
                            command.as_str()
                                == Some("python -m unittest scripts/ci/test_pr_plan.py")
                        })
                    })
            })),
        "pr-plan helper changes must run the focused plan proof"
    );
    assert!(
        route.get("coverage_pack_selector").and_then(Value::as_array).is_some_and(Vec::is_empty),
        "pr-plan proof pack is non-LCOV and must not be uploaded as Codecov coverage"
    );
    assert_eq!(
        route.pointer("/skipped_by_policy/patch-coverage-pr-plan").and_then(Value::as_str),
        Some("non-LCOV CI policy/routing surface; covered by focused CI gates")
    );
    let summary = fs::read_to_string(summary)?;
    assert!(summary.contains("python -m unittest scripts/ci/test_pr_plan.py"));
    assert!(summary.contains("`patch-coverage-pr-plan`: non-LCOV CI policy/routing surface"));
    Ok(())
}

#[test]
fn ci_route_cli_maps_clean_tmp_targets_script_to_cleanup_proof_pack() -> Result<()> {
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
            "scripts/clean-tmp-targets.sh",
        ])
        .assert()
        .success();

    let route: Value = serde_json::from_str(&std::fs::read_to_string(receipt)?)?;
    assert_eq!(
        route.pointer("/changed_surfaces/0").and_then(Value::as_str),
        Some("clean-tmp-targets")
    );
    assert!(
        route.get("required_proof_packs").and_then(Value::as_array).is_some_and(|packs| packs
            .iter()
            .any(|pack| {
                pack.get("id").and_then(Value::as_str) == Some("clean-tmp-targets-focused")
                    && pack.get("commands").and_then(Value::as_array).is_some_and(|commands| {
                        commands.iter().any(|command| {
                            command.as_str() == Some("bash scripts/tests/test-clean-tmp-targets.sh")
                        })
                    })
            })),
        "clean-tmp-targets changes must run the focused cleanup proof"
    );
    assert!(
        route.get("coverage_pack_selector").and_then(Value::as_array).is_some_and(Vec::is_empty),
        "clean-tmp-targets proof pack is non-LCOV and must not be uploaded as Codecov coverage"
    );
    assert_eq!(
        route
            .pointer("/skipped_by_policy/patch-coverage-clean-tmp-targets")
            .and_then(Value::as_str),
        Some("non-LCOV CI policy/routing surface; covered by focused CI gates")
    );
    let summary = fs::read_to_string(summary)?;
    assert!(summary.contains("bash scripts/tests/test-clean-tmp-targets.sh"));
    assert!(
        summary.contains("`patch-coverage-clean-tmp-targets`: non-LCOV CI policy/routing surface")
    );
    Ok(())
}

#[test]
fn ci_route_cli_maps_swarm_cleanup_scripts_to_cleanup_proof_pack() -> Result<()> {
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
            "scripts/swarm-clean",
            "--changed-file",
            "scripts/tests/test_swarm_doctor.sh",
        ])
        .assert()
        .success();

    let route: Value = serde_json::from_str(&std::fs::read_to_string(receipt)?)?;
    assert_eq!(route.pointer("/changed_surfaces/0").and_then(Value::as_str), Some("swarm-cleanup"));
    assert!(
        route.get("required_proof_packs").and_then(Value::as_array).is_some_and(|packs| packs
            .iter()
            .any(|pack| {
                pack.get("id").and_then(Value::as_str) == Some("swarm-cleanup-focused")
                    && pack.get("commands").and_then(Value::as_array).is_some_and(|commands| {
                        commands.iter().any(|command| {
                            command.as_str() == Some("bash scripts/tests/test_swarm_clean.sh")
                        }) && commands.iter().any(|command| {
                            command.as_str() == Some("bash scripts/tests/test_swarm_doctor.sh")
                        })
                    })
            })),
        "swarm cleanup changes must run focused cleanup proofs"
    );
    assert!(
        route.get("coverage_pack_selector").and_then(Value::as_array).is_some_and(Vec::is_empty),
        "swarm cleanup proof pack is non-LCOV and must not be uploaded as Codecov coverage"
    );
    assert_eq!(
        route.pointer("/skipped_by_policy/patch-coverage-swarm-cleanup").and_then(Value::as_str),
        Some("non-LCOV CI policy/routing surface; covered by focused CI gates")
    );
    let summary = fs::read_to_string(summary)?;
    assert!(summary.contains("bash scripts/tests/test_swarm_clean.sh"));
    assert!(summary.contains("bash scripts/tests/test_swarm_doctor.sh"));
    assert!(summary.contains("`patch-coverage-swarm-cleanup`: non-LCOV CI policy/routing surface"));
    Ok(())
}

#[test]
fn ci_route_cli_maps_pre_merge_check_script_to_pre_merge_proof_pack() -> Result<()> {
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
            "scripts/pre-merge-check.sh",
        ])
        .assert()
        .success();

    let route: Value = serde_json::from_str(&std::fs::read_to_string(receipt)?)?;
    assert_eq!(
        route.pointer("/changed_surfaces/0").and_then(Value::as_str),
        Some("pre-merge-check")
    );
    assert!(
        route.get("required_proof_packs").and_then(Value::as_array).is_some_and(|packs| packs
            .iter()
            .any(|pack| {
                pack.get("id").and_then(Value::as_str) == Some("pre-merge-check-focused")
                    && pack.get("commands").and_then(Value::as_array).is_some_and(|commands| {
                        commands.iter().any(|command| {
                            command.as_str() == Some("bash scripts/tests/test-pre-merge-check.sh")
                        })
                    })
            })),
        "pre-merge check changes must run the focused pre-merge proof"
    );
    assert!(
        route.get("coverage_pack_selector").and_then(Value::as_array).is_some_and(Vec::is_empty),
        "pre-merge proof pack is non-LCOV and must not be uploaded as Codecov coverage"
    );
    assert_eq!(
        route.pointer("/skipped_by_policy/patch-coverage-pre-merge-check").and_then(Value::as_str),
        Some("non-LCOV CI policy/routing surface; covered by focused CI gates")
    );
    let summary = fs::read_to_string(summary)?;
    assert!(summary.contains("bash scripts/tests/test-pre-merge-check.sh"));
    assert!(
        summary.contains("`patch-coverage-pre-merge-check`: non-LCOV CI policy/routing surface")
    );
    Ok(())
}

#[test]
fn ci_route_cli_maps_pr_overlap_script_to_overlap_proof_pack() -> Result<()> {
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
            "scripts/pr_overlap.py",
        ])
        .assert()
        .success();

    let route: Value = serde_json::from_str(&std::fs::read_to_string(receipt)?)?;
    assert_eq!(route.pointer("/changed_surfaces/0").and_then(Value::as_str), Some("pr-overlap"));
    assert!(
        route.get("required_proof_packs").and_then(Value::as_array).is_some_and(|packs| packs
            .iter()
            .any(|pack| {
                pack.get("id").and_then(Value::as_str) == Some("pr-overlap-focused")
                    && pack.get("commands").and_then(Value::as_array).is_some_and(|commands| {
                        commands.iter().any(|command| {
                            command.as_str() == Some("python scripts/tests/test_pr_overlap.py")
                        })
                    })
            })),
        "PR overlap helper changes must run the focused overlap proof"
    );
    assert!(
        route.get("coverage_pack_selector").and_then(Value::as_array).is_some_and(Vec::is_empty),
        "pr-overlap proof pack is non-LCOV and must not be uploaded as Codecov coverage"
    );
    assert_eq!(
        route.pointer("/skipped_by_policy/patch-coverage-pr-overlap").and_then(Value::as_str),
        Some("non-LCOV CI policy/routing surface; covered by focused CI gates")
    );
    let summary = fs::read_to_string(summary)?;
    assert!(summary.contains("python scripts/tests/test_pr_overlap.py"));
    assert!(summary.contains("`patch-coverage-pr-overlap`: non-LCOV CI policy/routing surface"));
    Ok(())
}

#[test]
fn ci_route_cli_maps_control_plane_lock_script_to_lock_proof_pack() -> Result<()> {
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
            "scripts/control-plane-lock.sh",
        ])
        .assert()
        .success();

    let route: Value = serde_json::from_str(&std::fs::read_to_string(receipt)?)?;
    assert_eq!(
        route.pointer("/changed_surfaces/0").and_then(Value::as_str),
        Some("control-plane-lock")
    );
    assert!(
        route.get("required_proof_packs").and_then(Value::as_array).is_some_and(|packs| packs
            .iter()
            .any(|pack| {
                pack.get("id").and_then(Value::as_str) == Some("control-plane-lock-focused")
                    && pack.get("commands").and_then(Value::as_array).is_some_and(|commands| {
                        commands.iter().any(|command| {
                            command.as_str() == Some("bash scripts/test-control-plane-lock.sh")
                        })
                    })
            })),
        "control-plane lock helper changes must run the focused lock proof"
    );
    assert!(
        route.get("coverage_pack_selector").and_then(Value::as_array).is_some_and(Vec::is_empty),
        "control-plane-lock proof pack is non-LCOV and must not be uploaded as Codecov coverage"
    );
    assert_eq!(
        route
            .pointer("/skipped_by_policy/patch-coverage-control-plane-lock")
            .and_then(Value::as_str),
        Some("non-LCOV CI policy/routing surface; covered by focused CI gates")
    );
    let summary = fs::read_to_string(summary)?;
    assert!(summary.contains("bash scripts/test-control-plane-lock.sh"));
    assert!(
        summary.contains("`patch-coverage-control-plane-lock`: non-LCOV CI policy/routing surface")
    );
    Ok(())
}

#[test]
fn ci_route_cli_maps_agent_preflight_script_to_preflight_proof_pack() -> Result<()> {
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
            "scripts/agent-preflight.sh",
        ])
        .assert()
        .success();

    let route: Value = serde_json::from_str(&std::fs::read_to_string(receipt)?)?;
    assert_eq!(
        route.pointer("/changed_surfaces/0").and_then(Value::as_str),
        Some("agent-preflight")
    );
    assert!(
        route.get("required_proof_packs").and_then(Value::as_array).is_some_and(|packs| packs
            .iter()
            .any(|pack| {
                pack.get("id").and_then(Value::as_str) == Some("agent-preflight-focused")
                    && pack.get("commands").and_then(Value::as_array).is_some_and(|commands| {
                        commands.iter().any(|command| {
                            command.as_str() == Some("bash scripts/test-agent-preflight.sh")
                        })
                    })
            })),
        "agent preflight helper changes must run the focused preflight proof"
    );
    assert!(
        route.get("coverage_pack_selector").and_then(Value::as_array).is_some_and(Vec::is_empty),
        "agent-preflight proof pack is non-LCOV and must not be uploaded as Codecov coverage"
    );
    assert_eq!(
        route.pointer("/skipped_by_policy/patch-coverage-agent-preflight").and_then(Value::as_str),
        Some("non-LCOV CI policy/routing surface; covered by focused CI gates")
    );
    let summary = fs::read_to_string(summary)?;
    assert!(summary.contains("bash scripts/test-agent-preflight.sh"));
    assert!(
        summary.contains("`patch-coverage-agent-preflight`: non-LCOV CI policy/routing surface")
    );
    Ok(())
}

#[test]
fn ci_route_cli_maps_preflight_wrapper_to_preflight_proof_pack() -> Result<()> {
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
            "scripts/preflight.sh",
        ])
        .assert()
        .success();

    let route: Value = serde_json::from_str(&std::fs::read_to_string(receipt)?)?;
    assert_eq!(
        route.pointer("/changed_surfaces/0").and_then(Value::as_str),
        Some("preflight-wrapper")
    );
    assert!(
        route.get("required_proof_packs").and_then(Value::as_array).is_some_and(|packs| packs
            .iter()
            .any(|pack| {
                pack.get("id").and_then(Value::as_str) == Some("preflight-wrapper-focused")
                    && pack.get("commands").and_then(Value::as_array).is_some_and(|commands| {
                        commands.iter().any(|command| {
                            command.as_str() == Some("bash scripts/tests/test-preflight-wrapper.sh")
                        })
                    })
            })),
        "preflight wrapper changes must run the focused preflight proof"
    );
    assert!(
        route.get("coverage_pack_selector").and_then(Value::as_array).is_some_and(Vec::is_empty),
        "preflight wrapper proof pack is non-LCOV and must not be uploaded as Codecov coverage"
    );
    assert_eq!(
        route
            .pointer("/skipped_by_policy/patch-coverage-preflight-wrapper")
            .and_then(Value::as_str),
        Some("non-LCOV CI policy/routing surface; covered by focused CI gates")
    );
    let summary = fs::read_to_string(summary)?;
    assert!(summary.contains("bash scripts/tests/test-preflight-wrapper.sh"));
    assert!(
        summary.contains("`patch-coverage-preflight-wrapper`: non-LCOV CI policy/routing surface")
    );
    Ok(())
}

#[test]
fn ci_route_cli_maps_install_githooks_wrapper_to_githooks_proof_pack() -> Result<()> {
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
            "scripts/install-githooks.sh",
        ])
        .assert()
        .success();

    let route: Value = serde_json::from_str(&std::fs::read_to_string(receipt)?)?;
    assert_eq!(
        route.pointer("/changed_surfaces/0").and_then(Value::as_str),
        Some("install-githooks-wrapper")
    );
    assert!(
        route.get("required_proof_packs").and_then(Value::as_array).is_some_and(|packs| packs
            .iter()
            .any(|pack| {
                pack.get("id").and_then(Value::as_str) == Some("install-githooks-wrapper-focused")
                    && pack.get("commands").and_then(Value::as_array).is_some_and(|commands| {
                        commands.iter().any(|command| {
                            command.as_str()
                                == Some("bash scripts/tests/test-install-githooks-wrapper.sh")
                        })
                    })
            })),
        "install-githooks wrapper changes must run the focused githooks proof"
    );
    assert!(
        route.get("coverage_pack_selector").and_then(Value::as_array).is_some_and(Vec::is_empty),
        "install-githooks wrapper proof pack is non-LCOV and must not be uploaded as Codecov coverage"
    );
    assert_eq!(
        route
            .pointer("/skipped_by_policy/patch-coverage-install-githooks-wrapper")
            .and_then(Value::as_str),
        Some("non-LCOV CI policy/routing surface; covered by focused CI gates")
    );
    let summary = fs::read_to_string(summary)?;
    assert!(summary.contains("bash scripts/tests/test-install-githooks-wrapper.sh"));
    assert!(
        summary.contains(
            "`patch-coverage-install-githooks-wrapper`: non-LCOV CI policy/routing surface"
        )
    );
    Ok(())
}

#[test]
fn ci_route_cli_maps_e2e_gate_wrapper_to_e2e_proof_pack() -> Result<()> {
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
            "scripts/e2e-gate.sh",
        ])
        .assert()
        .success();

    let route: Value = serde_json::from_str(&std::fs::read_to_string(receipt)?)?;
    assert_eq!(
        route.pointer("/changed_surfaces/0").and_then(Value::as_str),
        Some("e2e-gate-wrapper")
    );
    assert!(
        route.get("required_proof_packs").and_then(Value::as_array).is_some_and(|packs| packs
            .iter()
            .any(|pack| {
                pack.get("id").and_then(Value::as_str) == Some("e2e-gate-wrapper-focused")
                    && pack.get("commands").and_then(Value::as_array).is_some_and(|commands| {
                        commands.iter().any(|command| {
                            command.as_str() == Some("bash scripts/tests/test-e2e-gate-wrapper.sh")
                        })
                    })
            })),
        "e2e-gate wrapper changes must run the focused e2e proof"
    );
    assert!(
        route.get("coverage_pack_selector").and_then(Value::as_array).is_some_and(Vec::is_empty),
        "e2e-gate wrapper proof pack is non-LCOV and must not be uploaded as Codecov coverage"
    );
    assert_eq!(
        route.pointer("/skipped_by_policy/patch-coverage-e2e-gate-wrapper").and_then(Value::as_str),
        Some("non-LCOV CI policy/routing surface; covered by focused CI gates")
    );
    let summary = fs::read_to_string(summary)?;
    assert!(summary.contains("bash scripts/tests/test-e2e-gate-wrapper.sh"));
    assert!(
        summary.contains("`patch-coverage-e2e-gate-wrapper`: non-LCOV CI policy/routing surface")
    );
    Ok(())
}

#[test]
fn ci_route_cli_maps_execute_gate_wrapper_to_execute_gate_proof_pack() -> Result<()> {
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
            "scripts/execute-gate.sh",
        ])
        .assert()
        .success();

    let route: Value = serde_json::from_str(&std::fs::read_to_string(receipt)?)?;
    assert_eq!(
        route.pointer("/changed_surfaces/0").and_then(Value::as_str),
        Some("execute-gate-wrapper")
    );
    assert!(
        route.get("required_proof_packs").and_then(Value::as_array).is_some_and(|packs| packs
            .iter()
            .any(|pack| {
                pack.get("id").and_then(Value::as_str) == Some("execute-gate-wrapper-focused")
                    && pack.get("commands").and_then(Value::as_array).is_some_and(|commands| {
                        commands.iter().any(|command| {
                            command.as_str()
                                == Some("bash scripts/tests/test-execute-gate-wrapper.sh")
                        })
                    })
            })),
        "execute-gate wrapper changes must run the focused execute-gate proof"
    );
    assert!(
        route.get("coverage_pack_selector").and_then(Value::as_array).is_some_and(Vec::is_empty),
        "execute-gate wrapper proof pack is non-LCOV and must not be uploaded as Codecov coverage"
    );
    assert_eq!(
        route
            .pointer("/skipped_by_policy/patch-coverage-execute-gate-wrapper")
            .and_then(Value::as_str),
        Some("non-LCOV CI policy/routing surface; covered by focused CI gates")
    );
    let summary = fs::read_to_string(summary)?;
    assert!(summary.contains("bash scripts/tests/test-execute-gate-wrapper.sh"));
    assert!(
        summary
            .contains("`patch-coverage-execute-gate-wrapper`: non-LCOV CI policy/routing surface")
    );
    Ok(())
}

#[test]
fn ci_route_cli_maps_run_gates_wrapper_to_run_gates_proof_pack() -> Result<()> {
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
            "scripts/run-gates.sh",
        ])
        .assert()
        .success();

    let route: Value = serde_json::from_str(&std::fs::read_to_string(receipt)?)?;
    assert_eq!(
        route.pointer("/changed_surfaces/0").and_then(Value::as_str),
        Some("run-gates-wrapper")
    );
    assert!(
        route.get("required_proof_packs").and_then(Value::as_array).is_some_and(|packs| packs
            .iter()
            .any(|pack| {
                pack.get("id").and_then(Value::as_str) == Some("run-gates-wrapper-focused")
                    && pack.get("commands").and_then(Value::as_array).is_some_and(|commands| {
                        commands.iter().any(|command| {
                            command.as_str() == Some("bash scripts/tests/test-run-gates-wrapper.sh")
                        })
                    })
            })),
        "run-gates wrapper changes must run the focused run-gates proof"
    );
    assert!(
        route.get("coverage_pack_selector").and_then(Value::as_array).is_some_and(Vec::is_empty),
        "run-gates wrapper proof pack is non-LCOV and must not be uploaded as Codecov coverage"
    );
    assert_eq!(
        route
            .pointer("/skipped_by_policy/patch-coverage-run-gates-wrapper")
            .and_then(Value::as_str),
        Some("non-LCOV CI policy/routing surface; covered by focused CI gates")
    );
    let summary = fs::read_to_string(summary)?;
    assert!(summary.contains("bash scripts/tests/test-run-gates-wrapper.sh"));
    assert!(
        summary.contains("`patch-coverage-run-gates-wrapper`: non-LCOV CI policy/routing surface")
    );
    Ok(())
}

#[test]
fn ci_route_cli_maps_gate_local_wrapper_to_gate_local_proof_pack() -> Result<()> {
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
            "scripts/gate-local.sh",
        ])
        .assert()
        .success();

    let route: Value = serde_json::from_str(&std::fs::read_to_string(receipt)?)?;
    assert_eq!(
        route.pointer("/changed_surfaces/0").and_then(Value::as_str),
        Some("gate-local-wrapper")
    );
    assert!(
        route.get("required_proof_packs").and_then(Value::as_array).is_some_and(|packs| packs
            .iter()
            .any(|pack| {
                pack.get("id").and_then(Value::as_str) == Some("gate-local-wrapper-focused")
                    && pack.get("commands").and_then(Value::as_array).is_some_and(|commands| {
                        commands.iter().any(|command| {
                            command.as_str()
                                == Some("bash scripts/tests/test-gate-local-wrapper.sh")
                        })
                    })
            })),
        "gate-local wrapper changes must run the focused gate-local proof"
    );
    assert!(
        route.get("coverage_pack_selector").and_then(Value::as_array).is_some_and(Vec::is_empty),
        "gate-local wrapper proof pack is non-LCOV and must not be uploaded as Codecov coverage"
    );
    assert_eq!(
        route
            .pointer("/skipped_by_policy/patch-coverage-gate-local-wrapper")
            .and_then(Value::as_str),
        Some("non-LCOV CI policy/routing surface; covered by focused CI gates")
    );
    let summary = fs::read_to_string(summary)?;
    assert!(summary.contains("bash scripts/tests/test-gate-local-wrapper.sh"));
    assert!(
        summary.contains("`patch-coverage-gate-local-wrapper`: non-LCOV CI policy/routing surface")
    );
    Ok(())
}

#[test]
fn ci_route_cli_maps_list_gates_wrapper_to_list_gates_proof_pack() -> Result<()> {
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
            "scripts/list-gates.py",
        ])
        .assert()
        .success();

    let route: Value = serde_json::from_str(&std::fs::read_to_string(receipt)?)?;
    assert_eq!(
        route.pointer("/changed_surfaces/0").and_then(Value::as_str),
        Some("list-gates-wrapper")
    );
    assert!(
        route.get("required_proof_packs").and_then(Value::as_array).is_some_and(|packs| packs
            .iter()
            .any(|pack| {
                pack.get("id").and_then(Value::as_str) == Some("list-gates-wrapper-focused")
                    && pack.get("commands").and_then(Value::as_array).is_some_and(|commands| {
                        commands.iter().any(|command| {
                            command.as_str()
                                == Some("python scripts/tests/test-list-gates-wrapper.py")
                        })
                    })
            })),
        "list-gates wrapper changes must run the focused list-gates proof"
    );
    assert!(
        route.get("coverage_pack_selector").and_then(Value::as_array).is_some_and(Vec::is_empty),
        "list-gates wrapper proof pack is non-LCOV and must not be uploaded as Codecov coverage"
    );
    assert_eq!(
        route
            .pointer("/skipped_by_policy/patch-coverage-list-gates-wrapper")
            .and_then(Value::as_str),
        Some("non-LCOV CI policy/routing surface; covered by focused CI gates")
    );
    let summary = fs::read_to_string(summary)?;
    assert!(summary.contains("python scripts/tests/test-list-gates-wrapper.py"));
    assert!(
        summary.contains("`patch-coverage-list-gates-wrapper`: non-LCOV CI policy/routing surface")
    );
    Ok(())
}

#[test]
fn ci_route_cli_maps_forbid_fatal_wrapper_to_forbid_fatal_proof_pack() -> Result<()> {
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
            "scripts/forbid-fatal-constructs.sh",
        ])
        .assert()
        .success();

    let route: Value = serde_json::from_str(&std::fs::read_to_string(receipt)?)?;
    assert_eq!(
        route.pointer("/changed_surfaces/0").and_then(Value::as_str),
        Some("forbid-fatal-constructs-wrapper")
    );
    assert!(
        route.get("required_proof_packs").and_then(Value::as_array).is_some_and(|packs| packs
            .iter()
            .any(|pack| {
                pack.get("id").and_then(Value::as_str)
                    == Some("forbid-fatal-constructs-wrapper-focused")
                    && pack.get("commands").and_then(Value::as_array).is_some_and(|commands| {
                        commands.iter().any(|command| {
                            command.as_str()
                                == Some(
                                    "bash scripts/tests/test-forbid-fatal-constructs-wrapper.sh",
                                )
                        })
                    })
            })),
        "forbid-fatal-constructs wrapper changes must run focused proof"
    );
    assert!(
        route.get("coverage_pack_selector").and_then(Value::as_array).is_some_and(Vec::is_empty),
        "forbid-fatal-constructs wrapper proof pack is non-LCOV and must not be uploaded as Codecov coverage"
    );
    assert_eq!(
        route
            .pointer("/skipped_by_policy/patch-coverage-forbid-fatal-constructs-wrapper")
            .and_then(Value::as_str),
        Some("non-LCOV CI policy/routing surface; covered by focused CI gates")
    );
    let summary = fs::read_to_string(summary)?;
    assert!(summary.contains("bash scripts/tests/test-forbid-fatal-constructs-wrapper.sh"));
    assert!(summary.contains(
        "`patch-coverage-forbid-fatal-constructs-wrapper`: non-LCOV CI policy/routing surface"
    ));
    Ok(())
}

#[test]
fn ci_route_cli_maps_dead_code_wrapper_to_dead_code_proof_pack() -> Result<()> {
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
            "scripts/dead-code-check.sh",
        ])
        .assert()
        .success();

    let route: Value = serde_json::from_str(&std::fs::read_to_string(receipt)?)?;
    assert_eq!(
        route.pointer("/changed_surfaces/0").and_then(Value::as_str),
        Some("dead-code-wrapper")
    );
    assert!(
        route.get("required_proof_packs").and_then(Value::as_array).is_some_and(|packs| packs
            .iter()
            .any(|pack| {
                pack.get("id").and_then(Value::as_str) == Some("dead-code-wrapper-focused")
                    && pack.get("commands").and_then(Value::as_array).is_some_and(|commands| {
                        commands.iter().any(|command| {
                            command.as_str() == Some("bash scripts/tests/test-dead-code-wrapper.sh")
                        })
                    })
            })),
        "dead-code wrapper changes must run focused proof"
    );
    assert!(
        route.get("coverage_pack_selector").and_then(Value::as_array).is_some_and(Vec::is_empty),
        "dead-code wrapper proof pack is non-LCOV and must not be uploaded as Codecov coverage"
    );
    assert_eq!(
        route
            .pointer("/skipped_by_policy/patch-coverage-dead-code-wrapper")
            .and_then(Value::as_str),
        Some("non-LCOV CI policy/routing surface; covered by focused CI gates")
    );
    let summary = fs::read_to_string(summary)?;
    assert!(summary.contains("bash scripts/tests/test-dead-code-wrapper.sh"));
    assert!(
        summary.contains("`patch-coverage-dead-code-wrapper`: non-LCOV CI policy/routing surface")
    );
    Ok(())
}

#[test]
fn ci_route_cli_maps_check_toolchain_wrapper_to_toolchain_proof_pack() -> Result<()> {
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
            "scripts/check-rust-toolchain.sh",
        ])
        .assert()
        .success();

    let route: Value = serde_json::from_str(&std::fs::read_to_string(receipt)?)?;
    assert_eq!(
        route.pointer("/changed_surfaces/0").and_then(Value::as_str),
        Some("check-toolchain-wrapper")
    );
    assert!(
        route.get("required_proof_packs").and_then(Value::as_array).is_some_and(|packs| packs
            .iter()
            .any(|pack| {
                pack.get("id").and_then(Value::as_str) == Some("check-toolchain-wrapper-focused")
                    && pack.get("commands").and_then(Value::as_array).is_some_and(|commands| {
                        commands.iter().any(|command| {
                            command.as_str()
                                == Some("bash scripts/tests/test-check-rust-toolchain-wrapper.sh")
                        })
                    })
            })),
        "check-toolchain wrapper changes must run focused proof"
    );
    assert!(
        route.get("coverage_pack_selector").and_then(Value::as_array).is_some_and(Vec::is_empty),
        "check-toolchain wrapper proof pack is non-LCOV and must not be uploaded as Codecov coverage"
    );
    assert_eq!(
        route
            .pointer("/skipped_by_policy/patch-coverage-check-toolchain-wrapper")
            .and_then(Value::as_str),
        Some("non-LCOV CI policy/routing surface; covered by focused CI gates")
    );
    let summary = fs::read_to_string(summary)?;
    assert!(summary.contains("bash scripts/tests/test-check-rust-toolchain-wrapper.sh"));
    assert!(
        summary.contains(
            "`patch-coverage-check-toolchain-wrapper`: non-LCOV CI policy/routing surface"
        )
    );
    Ok(())
}

#[test]
fn ci_route_cli_maps_coverage_baseline_script_to_baseline_proof_pack() -> Result<()> {
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
            "scripts/check-coverage-baseline.sh",
        ])
        .assert()
        .success();

    let route: Value = serde_json::from_str(&std::fs::read_to_string(receipt)?)?;
    assert_eq!(
        route.pointer("/changed_surfaces/0").and_then(Value::as_str),
        Some("coverage-baseline-script")
    );
    assert!(
        route.get("required_proof_packs").and_then(Value::as_array).is_some_and(|packs| packs
            .iter()
            .any(|pack| {
                pack.get("id").and_then(Value::as_str) == Some("coverage-baseline-script-focused")
                    && pack.get("commands").and_then(Value::as_array).is_some_and(|commands| {
                        commands.iter().any(|command| {
                            command.as_str()
                                == Some("bash scripts/tests/test-check-coverage-baseline.sh")
                        })
                    })
            })),
        "coverage baseline helper changes must run the focused baseline proof"
    );
    assert!(
        route.get("coverage_pack_selector").and_then(Value::as_array).is_some_and(Vec::is_empty),
        "coverage-baseline script proof pack is non-LCOV and must not be uploaded as Codecov coverage"
    );
    assert_eq!(
        route.pointer("/skipped_by_policy/patch-coverage-baseline-script").and_then(Value::as_str),
        Some("non-LCOV CI policy/routing surface; covered by focused CI gates")
    );
    let summary = fs::read_to_string(summary)?;
    assert!(summary.contains("bash scripts/tests/test-check-coverage-baseline.sh"));
    assert!(
        summary.contains("`patch-coverage-baseline-script`: non-LCOV CI policy/routing surface")
    );
    Ok(())
}

#[test]
fn ci_route_cli_maps_update_coverage_baseline_script_to_baseline_proof_pack() -> Result<()> {
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
            "scripts/update-coverage-baseline.sh",
        ])
        .assert()
        .success();

    let route: Value = serde_json::from_str(&std::fs::read_to_string(receipt)?)?;
    assert_eq!(
        route.pointer("/changed_surfaces/0").and_then(Value::as_str),
        Some("update-coverage-baseline-script")
    );
    assert!(
        route.get("required_proof_packs").and_then(Value::as_array).is_some_and(|packs| packs
            .iter()
            .any(|pack| {
                pack.get("id").and_then(Value::as_str)
                    == Some("update-coverage-baseline-script-focused")
                    && pack.get("commands").and_then(Value::as_array).is_some_and(|commands| {
                        commands.iter().any(|command| {
                            command.as_str()
                                == Some("bash scripts/tests/test-update-coverage-baseline.sh")
                        })
                    })
            })),
        "update coverage baseline helper changes must run the focused baseline proof"
    );
    assert!(
        route.get("coverage_pack_selector").and_then(Value::as_array).is_some_and(Vec::is_empty),
        "update-coverage-baseline script proof pack is non-LCOV and must not be uploaded as Codecov coverage"
    );
    assert_eq!(
        route
            .pointer("/skipped_by_policy/patch-coverage-update-baseline-script")
            .and_then(Value::as_str),
        Some("non-LCOV CI policy/routing surface; covered by focused CI gates")
    );
    let summary = fs::read_to_string(summary)?;
    assert!(summary.contains("bash scripts/tests/test-update-coverage-baseline.sh"));
    assert!(
        summary.contains(
            "`patch-coverage-update-baseline-script`: non-LCOV CI policy/routing surface"
        )
    );
    Ok(())
}

#[test]
fn ci_route_cli_maps_generate_receipt_script_to_receipt_proof_pack() -> Result<()> {
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
            "scripts/generate-receipt.sh",
        ])
        .assert()
        .success();

    let route: Value = serde_json::from_str(&std::fs::read_to_string(receipt)?)?;
    assert_eq!(
        route.pointer("/changed_surfaces/0").and_then(Value::as_str),
        Some("generate-receipt-script")
    );
    assert!(
        route.get("required_proof_packs").and_then(Value::as_array).is_some_and(|packs| packs
            .iter()
            .any(|pack| {
                pack.get("id").and_then(Value::as_str) == Some("generate-receipt-script-focused")
                    && pack.get("commands").and_then(Value::as_array).is_some_and(|commands| {
                        commands.iter().any(|command| {
                            command.as_str() == Some("bash scripts/tests/test-generate-receipt.sh")
                        })
                    })
            })),
        "generate receipt helper changes must run the focused receipt proof"
    );
    assert!(
        route.get("coverage_pack_selector").and_then(Value::as_array).is_some_and(Vec::is_empty),
        "generate-receipt script proof pack is non-LCOV and must not be uploaded as Codecov coverage"
    );
    assert_eq!(
        route
            .pointer("/skipped_by_policy/patch-coverage-generate-receipt-script")
            .and_then(Value::as_str),
        Some("non-LCOV CI policy/routing surface; covered by focused CI gates")
    );
    let summary = fs::read_to_string(summary)?;
    assert!(summary.contains("bash scripts/tests/test-generate-receipt.sh"));
    assert!(
        summary.contains(
            "`patch-coverage-generate-receipt-script`: non-LCOV CI policy/routing surface"
        )
    );
    Ok(())
}

#[test]
fn ci_route_cli_maps_quick_receipts_wrapper_to_receipt_proof_pack() -> Result<()> {
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
            "scripts/quick-receipts.sh",
        ])
        .assert()
        .success();

    let route: Value = serde_json::from_str(&std::fs::read_to_string(receipt)?)?;
    assert_eq!(
        route.pointer("/changed_surfaces/0").and_then(Value::as_str),
        Some("quick-receipts-wrapper")
    );
    assert!(
        route.get("required_proof_packs").and_then(Value::as_array).is_some_and(|packs| packs
            .iter()
            .any(|pack| {
                pack.get("id").and_then(Value::as_str) == Some("quick-receipts-wrapper-focused")
                    && pack.get("commands").and_then(Value::as_array).is_some_and(|commands| {
                        commands.iter().any(|command| {
                            command.as_str()
                                == Some("bash scripts/tests/test-quick-receipts-wrapper.sh")
                        })
                    })
            })),
        "quick receipt wrapper changes must run the focused receipt proof"
    );
    assert!(
        route.get("coverage_pack_selector").and_then(Value::as_array).is_some_and(Vec::is_empty),
        "quick-receipts wrapper proof pack is non-LCOV and must not be uploaded as Codecov coverage"
    );
    assert_eq!(
        route
            .pointer("/skipped_by_policy/patch-coverage-quick-receipts-wrapper")
            .and_then(Value::as_str),
        Some("non-LCOV CI policy/routing surface; covered by focused CI gates")
    );
    let summary = fs::read_to_string(summary)?;
    assert!(summary.contains("bash scripts/tests/test-quick-receipts-wrapper.sh"));
    assert!(
        summary.contains(
            "`patch-coverage-quick-receipts-wrapper`: non-LCOV CI policy/routing surface"
        )
    );
    Ok(())
}

#[test]
fn ci_route_cli_maps_generate_badges_wrapper_to_badges_proof_pack() -> Result<()> {
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
            "scripts/generate-badges.sh",
        ])
        .assert()
        .success();

    let route: Value = serde_json::from_str(&std::fs::read_to_string(receipt)?)?;
    assert_eq!(
        route.pointer("/changed_surfaces/0").and_then(Value::as_str),
        Some("generate-badges-wrapper")
    );
    assert!(
        route.get("required_proof_packs").and_then(Value::as_array).is_some_and(|packs| packs
            .iter()
            .any(|pack| {
                pack.get("id").and_then(Value::as_str) == Some("generate-badges-wrapper-focused")
                    && pack.get("commands").and_then(Value::as_array).is_some_and(|commands| {
                        commands.iter().any(|command| {
                            command.as_str()
                                == Some("bash scripts/tests/test-generate-badges-wrapper.sh")
                        })
                    })
            })),
        "generate badges wrapper changes must run the focused badge proof"
    );
    assert!(
        route.get("coverage_pack_selector").and_then(Value::as_array).is_some_and(Vec::is_empty),
        "generate-badges wrapper proof pack is non-LCOV and must not be uploaded as Codecov coverage"
    );
    assert_eq!(
        route
            .pointer("/skipped_by_policy/patch-coverage-generate-badges-wrapper")
            .and_then(Value::as_str),
        Some("non-LCOV CI policy/routing surface; covered by focused CI gates")
    );
    let summary = fs::read_to_string(summary)?;
    assert!(summary.contains("bash scripts/tests/test-generate-badges-wrapper.sh"));
    assert!(
        summary.contains(
            "`patch-coverage-generate-badges-wrapper`: non-LCOV CI policy/routing surface"
        )
    );
    Ok(())
}

#[test]
fn ci_route_cli_maps_ignored_test_count_wrapper_to_count_proof_pack() -> Result<()> {
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
            "scripts/ignored-test-count.sh",
        ])
        .assert()
        .success();

    let route: Value = serde_json::from_str(&std::fs::read_to_string(receipt)?)?;
    assert_eq!(
        route.pointer("/changed_surfaces/0").and_then(Value::as_str),
        Some("ignored-test-count-wrapper")
    );
    assert!(
        route.get("required_proof_packs").and_then(Value::as_array).is_some_and(|packs| packs
            .iter()
            .any(|pack| {
                pack.get("id").and_then(Value::as_str) == Some("ignored-test-count-wrapper-focused")
                    && pack.get("commands").and_then(Value::as_array).is_some_and(|commands| {
                        commands.iter().any(|command| {
                            command.as_str()
                                == Some("bash scripts/tests/test-ignored-test-count-wrapper.sh")
                        })
                    })
            })),
        "ignored test count wrapper changes must run the focused count proof"
    );
    assert!(
        route.get("coverage_pack_selector").and_then(Value::as_array).is_some_and(Vec::is_empty),
        "ignored-test-count wrapper proof pack is non-LCOV and must not be uploaded as Codecov coverage"
    );
    assert_eq!(
        route
            .pointer("/skipped_by_policy/patch-coverage-ignored-test-count-wrapper")
            .and_then(Value::as_str),
        Some("non-LCOV CI policy/routing surface; covered by focused CI gates")
    );
    let summary = fs::read_to_string(summary)?;
    assert!(summary.contains("bash scripts/tests/test-ignored-test-count-wrapper.sh"));
    assert!(summary.contains(
        "`patch-coverage-ignored-test-count-wrapper`: non-LCOV CI policy/routing surface"
    ));
    Ok(())
}

#[test]
fn ci_route_cli_maps_completion_provider_to_completion_proof_pack() -> Result<()> {
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
            "crates/perl-lsp-rs-core/src/providers/completion/completion/import_map/used_modules.rs",
        ])
        .assert()
        .success();

    let route: Value = serde_json::from_str(&std::fs::read_to_string(receipt)?)?;
    assert_eq!(
        route.pointer("/changed_surfaces/0").and_then(Value::as_str),
        Some("completion-core")
    );
    assert_eq!(
        route.pointer("/coverage_pack_selector/0").and_then(Value::as_str),
        Some("patch-coverage-completion-core")
    );
    assert!(
        route.get("required_proof_packs").and_then(Value::as_array).is_some_and(|packs| {
            packs.iter().any(|pack| {
                pack.get("id").and_then(Value::as_str) == Some("completion-core")
                    && pack.get("commands").and_then(Value::as_array).is_some_and(|commands| {
                        commands.iter().any(|command| {
                            command
                                .as_str()
                                .is_some_and(|text| text.contains("completion::completion"))
                        })
                    })
            })
        }),
        "completion provider changes must run the focused completion proof"
    );
    let summary = fs::read_to_string(summary)?;
    assert!(summary.contains("patch-coverage-completion-core"));
    assert!(summary.contains("completion::completion"));
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
