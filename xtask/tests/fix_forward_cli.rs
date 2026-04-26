use anyhow::{Context, Result};
use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;

fn fixture_path(name: &str) -> String {
    format!("tests/fixtures/fix-forward/{name}")
}

#[test]
fn classifies_fmt_receipt_as_fmt_only() -> Result<()> {
    let output = tempfile::NamedTempFile::new().context("creating temp output")?;
    let mut cmd = cargo_bin_cmd!("xtask");
    let output_status = cmd
        .args([
            "fix-forward",
            "classify",
            "--receipt",
            &fixture_path("fmt-failure-receipt.json"),
            "--output",
            output.path().to_str().context("temp path is not utf-8")?,
        ])
        .output()
        .context("running xtask fix-forward classify for fmt receipt")?;

    assert!(output_status.status.success(), "classify command failed for fmt fixture");

    let raw = fs::read_to_string(output.path()).context("reading classifier output")?;
    let value: serde_json::Value =
        serde_json::from_str(&raw).context("parsing classifier output")?;
    assert_eq!(value["fix_forward_kind"], "FMT_ONLY");
    assert_eq!(value["safe_auto_fix"], true);

    Ok(())
}

#[test]
fn classifies_stale_base_receipt_as_stale_base_cascade() -> Result<()> {
    let output = tempfile::NamedTempFile::new().context("creating temp output")?;
    let mut cmd = cargo_bin_cmd!("xtask");
    let output_status = cmd
        .args([
            "fix-forward",
            "classify",
            "--receipt",
            &fixture_path("stale-base-receipt.json"),
            "--output",
            output.path().to_str().context("temp path is not utf-8")?,
        ])
        .output()
        .context("running xtask fix-forward classify for stale-base receipt")?;

    assert!(output_status.status.success(), "classify command failed for stale-base fixture");

    let raw = fs::read_to_string(output.path()).context("reading classifier output")?;
    let value: serde_json::Value =
        serde_json::from_str(&raw).context("parsing classifier output")?;
    assert_eq!(value["fix_forward_kind"], "STALE_BASE_CASCADE");
    assert_eq!(value["route"], "cascade-update");

    Ok(())
}

#[test]
fn classifies_generated_docs_receipt_as_generated_doc_regen() -> Result<()> {
    let output = tempfile::NamedTempFile::new().context("creating temp output")?;
    let mut cmd = cargo_bin_cmd!("xtask");
    let output_status = cmd
        .args([
            "fix-forward",
            "classify",
            "--receipt",
            &fixture_path("generated-docs-receipt.json"),
            "--output",
            output.path().to_str().context("temp path is not utf-8")?,
        ])
        .output()
        .context("running xtask fix-forward classify for generated-docs receipt")?;

    assert!(output_status.status.success(), "classify command failed for generated-docs fixture");

    let raw = fs::read_to_string(output.path()).context("reading classifier output")?;
    let value: serde_json::Value =
        serde_json::from_str(&raw).context("parsing classifier output")?;
    assert_eq!(value["fix_forward_kind"], "GENERATED_DOC_REGEN");

    Ok(())
}
