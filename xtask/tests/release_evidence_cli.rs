use anyhow::{Context, Result};
use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

const REQUIRED: [&str; 8] = [
    "ci-gate.json",
    "parser-ratchet-release.json",
    "vscode-extension-smoke.json",
    "lsp-scenario.json",
    "real-workspace-baseline.json",
    "ai-completion-e2e.json",
    "advisory-status.json",
    "unresolved-risk-register.json",
];

fn fixture_path(name: &str) -> PathBuf {
    Path::new("tests").join("fixtures").join("release-evidence").join(name)
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn write_bundle(version: &str, advisory_fixture: &str) -> Result<PathBuf> {
    let bundle =
        workspace_root().join("target").join("release-evidence").join(format!("v{version}"));
    if bundle.exists() {
        fs::remove_dir_all(&bundle)?;
    }
    fs::create_dir_all(&bundle)?;

    let pass = fs::read_to_string(fixture_path("pass.json"))?;
    for receipt in REQUIRED {
        let path = bundle.join(receipt);
        if receipt == "advisory-status.json" {
            fs::write(path, fs::read_to_string(fixture_path(advisory_fixture))?)?;
        } else {
            fs::write(path, &pass)?;
        }
    }

    Ok(bundle)
}

fn run_verify(version: &str, receipt: &Path) -> assert_cmd::assert::Assert {
    cargo_bin_cmd!("xtask")
        .args([
            "release",
            "verify-evidence",
            "--version",
            version,
            "--receipt",
            receipt.to_str().unwrap_or("target/receipts/release-evidence.json"),
        ])
        .assert()
}

#[test]
fn fixture_complete_bundle_passes() -> Result<()> {
    let version = "0.13.0-fixture-complete";
    write_bundle(version, "pass.json")?;
    let receipt =
        workspace_root().join("target").join("receipts").join("release-evidence-complete.json");

    run_verify(version, &receipt).success();

    let summary: Value =
        serde_json::from_str(&fs::read_to_string(&receipt).context("reading summary receipt")?)?;
    assert_eq!(summary["status"], "pass");
    Ok(())
}

#[test]
fn fixture_missing_parser_ratchet_release_fails() -> Result<()> {
    let version = "0.13.0-fixture-missing-parser";
    let bundle = write_bundle(version, "pass.json")?;
    fs::remove_file(bundle.join("parser-ratchet-release.json"))?;
    let receipt =
        workspace_root().join("target").join("receipts").join("release-evidence-missing.json");

    run_verify(version, &receipt).failure();

    let summary: Value = serde_json::from_str(&fs::read_to_string(&receipt)?)?;
    assert_eq!(summary["status"], "fail");
    assert!(summary["missing_receipts"].to_string().contains("parser-ratchet-release.json"));
    Ok(())
}

#[test]
fn fixture_advisory_failure_produces_classified_warning() -> Result<()> {
    let version = "0.13.0-fixture-advisory-warning";
    write_bundle(version, "advisory-fail-nonblocking.json")?;
    let receipt =
        workspace_root().join("target").join("receipts").join("release-evidence-advisory.json");

    run_verify(version, &receipt).success();

    let summary: Value = serde_json::from_str(&fs::read_to_string(&receipt)?)?;
    assert_eq!(summary["status"], "pass");
    let warnings = summary["advisory_warnings"].to_string();
    assert!(warnings.contains("failed"));
    Ok(())
}
