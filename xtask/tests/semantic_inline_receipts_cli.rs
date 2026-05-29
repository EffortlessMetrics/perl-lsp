use anyhow::{Result, anyhow};
use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::{Value, json};
use tempfile::TempDir;

#[test]
fn semantic_inline_receipts_cli_writes_dashboard_inventory() -> Result<()> {
    let temp = TempDir::new()?;
    let receipt = temp.path().join("semantic-inline-receipts.json");
    let missing_quality_receipt = temp.path().join("missing-inline-quality.json");
    let receipt_arg =
        receipt.to_str().ok_or_else(|| anyhow!("invalid semantic inline receipt path"))?;
    let quality_arg = missing_quality_receipt
        .to_str()
        .ok_or_else(|| anyhow!("invalid inline quality receipt path"))?;

    cargo_bin_cmd!("xtask")
        .args([
            "semantic-inline-receipts",
            "--receipt",
            receipt_arg,
            "--quality-receipt",
            quality_arg,
        ])
        .assert()
        .success();

    let dashboard: Value = serde_json::from_str(&std::fs::read_to_string(&receipt)?)?;
    assert_eq!(
        dashboard.get("schema_version").and_then(Value::as_str),
        Some("semantic-inline-receipts.v1")
    );
    assert_eq!(dashboard.get("provider").and_then(Value::as_str), Some("inline_completion"));
    assert_eq!(
        dashboard.get("provider_action").and_then(Value::as_str),
        Some("semantic_inline_receipt_dashboard")
    );
    assert_eq!(
        dashboard.get("all_required_capabilities_registered").and_then(Value::as_bool),
        Some(true)
    );

    let semantic_inline = dashboard
        .get("semantic_inline")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("semantic_inline map missing"))?;
    assert_eq!(semantic_inline.len(), 10);
    assert_eq!(
        semantic_inline
            .get("project_module_import")
            .and_then(|entry| entry.get("workflow_id"))
            .and_then(Value::as_str),
        Some("real_workspace_module_import_inline_completion_quality")
    );
    assert_eq!(
        semantic_inline
            .get("guard_condition")
            .and_then(|entry| entry.get("workflow_id"))
            .and_then(Value::as_str),
        Some("guard_condition_inline_completion_quality")
    );

    let future_gated = dashboard
        .get("future_gated")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("future_gated map missing"))?;
    assert_eq!(future_gated.get("next_edit").and_then(Value::as_str), Some("future_gated"));
    assert_eq!(
        future_gated.get("optional_ai_candidate_source").and_then(Value::as_str),
        Some("future_gated")
    );

    let quality_counters = dashboard
        .get("quality_counters")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("quality_counters map missing"))?;
    assert_eq!(quality_counters.get("available").and_then(Value::as_bool), Some(false));

    Ok(())
}

#[test]
fn semantic_inline_receipts_cli_embeds_quality_counters_when_available() -> Result<()> {
    let temp = TempDir::new()?;
    let receipt = temp.path().join("semantic-inline-receipts.json");
    let quality_receipt = temp.path().join("inline-completion-quality.json");

    std::fs::write(
        &quality_receipt,
        serde_json::to_vec_pretty(&json!({
            "fixtures_total": 28,
            "fixtures_passed": 28,
            "checks": {
                "hard_zone_rejected": 14,
                "parse_regressions": 0
            }
        }))?,
    )?;

    cargo_bin_cmd!("xtask")
        .args([
            "semantic-inline-receipts",
            "--receipt",
            &receipt.display().to_string(),
            "--quality-receipt",
            &quality_receipt.display().to_string(),
        ])
        .assert()
        .success();

    let receipt_json: Value = serde_json::from_slice(&std::fs::read(&receipt)?)?;
    let quality_counters = receipt_json
        .get("quality_counters")
        .ok_or_else(|| anyhow!("semantic inline receipt omitted quality_counters"))?;

    assert_eq!(quality_counters.get("available").and_then(Value::as_bool), Some(true));
    assert_eq!(quality_counters.get("fixtures_total").and_then(Value::as_u64), Some(28));
    assert_eq!(quality_counters.get("fixtures_passed").and_then(Value::as_u64), Some(28));
    assert_eq!(quality_counters.get("hard_zone_rejections").and_then(Value::as_u64), Some(14));
    assert_eq!(quality_counters.get("parse_regressions").and_then(Value::as_u64), Some(0));

    Ok(())
}
