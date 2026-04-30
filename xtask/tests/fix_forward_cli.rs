use assert_cmd::Command;
use color_eyre::eyre::{Result, eyre};
use std::path::{Path, PathBuf};

fn fixture_path(name: &str) -> PathBuf {
    Path::new("tests").join("fixtures").join("fix-forward").join(name)
}

fn run_classify_and_read(receipt_name: &str, output_name: &str) -> Result<serde_json::Value> {
    let output_path = Path::new("target").join("tmp").join("fix-forward-tests").join(output_name);

    if output_path.exists() {
        std::fs::remove_file(&output_path)?;
    }

    let output = Command::cargo_bin("xtask")?
        .args([
            "fix-forward",
            "classify",
            "--receipt",
            &fixture_path(receipt_name).to_string_lossy(),
            "--output",
            &output_path.to_string_lossy(),
        ])
        .output()?;

    if !output.status.success() {
        return Err(eyre!(
            "fix-forward classify failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let raw = std::fs::read_to_string(&output_path)?;
    let parsed = serde_json::from_str(&raw)?;
    Ok(parsed)
}

#[test]
fn classify_fmt_failure_receipt() -> Result<()> {
    let parsed = run_classify_and_read("fmt-failure-receipt.json", "fmt-output.json")?;
    assert_eq!(parsed["fix_forward_kind"], "FMT_ONLY");
    assert_eq!(parsed["safe_auto_fix"], true);
    Ok(())
}

#[test]
fn classify_stale_base_receipt() -> Result<()> {
    let parsed = run_classify_and_read("stale-base-receipt.json", "stale-output.json")?;
    assert_eq!(parsed["fix_forward_kind"], "STALE_BASE_CASCADE");
    assert_eq!(parsed["safe_auto_fix"], false);
    Ok(())
}

#[test]
fn classify_generated_docs_receipt() -> Result<()> {
    let parsed = run_classify_and_read("generated-docs-receipt.json", "docs-output.json")?;
    assert_eq!(parsed["fix_forward_kind"], "GENERATED_DOC_REGEN");
    assert_eq!(parsed["safe_auto_fix"], false);
    Ok(())
}

#[test]
fn list_playbooks_includes_known_kinds() -> Result<()> {
    let output = Command::cargo_bin("xtask")?.args(["fix-forward", "list-playbooks"]).output()?;
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("FMT_ONLY"));
    assert!(stdout.contains("STALE_BASE_CASCADE"));
    assert!(stdout.contains("GENERATED_DOC_REGEN"));
    Ok(())
}
