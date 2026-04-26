use anyhow::Result;
use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;
use std::path::Path;
use tempfile::TempDir;

fn fixture_path(name: &str) -> String {
    Path::new("tests")
        .join("fixtures")
        .join("release-evidence")
        .join(name)
        .display()
        .to_string()
}

#[test]
fn fixture_complete_bundle_passes() -> Result<()> {
    let temp = TempDir::new()?;
    let receipt = temp.path().join("release-evidence.json");

    cargo_bin_cmd!("xtask")
        .args([
            "release",
            "verify-evidence",
            "--version",
            "0.13.0",
            "--bundle-dir",
            &fixture_path("complete"),
            "--receipt",
            receipt.to_str().ok_or_else(|| anyhow::anyhow!("invalid receipt path"))?,
        ])
        .assert()
        .success();

    let summary: Value = serde_json::from_str(&std::fs::read_to_string(&receipt)?)?;
    assert_eq!(summary.get("status").and_then(Value::as_str), Some("pass"));
    Ok(())
}

#[test]
fn fixture_missing_parser_ratchet_release_fails() -> Result<()> {
    let temp = TempDir::new()?;
    let receipt = temp.path().join("release-evidence.json");

    cargo_bin_cmd!("xtask")
        .args([
            "release",
            "verify-evidence",
            "--version",
            "0.13.0",
            "--bundle-dir",
            &fixture_path("missing-parser"),
            "--receipt",
            receipt.to_str().ok_or_else(|| anyhow::anyhow!("invalid receipt path"))?,
        ])
        .assert()
        .failure();

    let summary: Value = serde_json::from_str(&std::fs::read_to_string(&receipt)?)?;
    assert_eq!(summary.get("status").and_then(Value::as_str), Some("fail"));
    Ok(())
}

#[test]
fn fixture_advisory_failure_produces_classified_warning() -> Result<()> {
    let temp = TempDir::new()?;
    let receipt = temp.path().join("release-evidence.json");

    cargo_bin_cmd!("xtask")
        .args([
            "release",
            "verify-evidence",
            "--version",
            "0.13.0",
            "--bundle-dir",
            &fixture_path("advisory-warning"),
            "--receipt",
            receipt.to_str().ok_or_else(|| anyhow::anyhow!("invalid receipt path"))?,
        ])
        .assert()
        .success();

    let summary: Value = serde_json::from_str(&std::fs::read_to_string(&receipt)?)?;
    let warnings = summary
        .get("warnings")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("warnings missing"))?;

    assert!(warnings.iter().any(|item| {
        item.as_str().map(|v| v.contains("classified advisory warning")).unwrap_or(false)
    }));

    let entries = summary
        .get("receipts")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("receipts missing"))?;
    assert!(entries.iter().any(|entry| {
        entry.get("name").and_then(Value::as_str) == Some("advisory-status")
            && entry.get("classification").and_then(Value::as_str)
                == Some("failure-advisory-warning")
    }));

    Ok(())
}
