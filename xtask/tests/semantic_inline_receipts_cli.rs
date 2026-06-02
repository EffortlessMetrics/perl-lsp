use anyhow::{Result, anyhow};
use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::{Value, json};
use tempfile::TempDir;

#[test]
fn semantic_inline_receipts_cli_writes_dashboard_inventory() -> Result<()> {
    let temp = TempDir::new()?;
    let receipt = temp.path().join("semantic-inline-receipts.json");
    let missing_quality_receipt = temp.path().join("missing-inline-quality.json");
    let missing_next_edit_receipt = temp.path().join("missing-next-edit.json");
    let receipt_arg =
        receipt.to_str().ok_or_else(|| anyhow!("invalid semantic inline receipt path"))?;
    let quality_arg = missing_quality_receipt
        .to_str()
        .ok_or_else(|| anyhow!("invalid inline quality receipt path"))?;
    let next_edit_arg = missing_next_edit_receipt
        .to_str()
        .ok_or_else(|| anyhow!("invalid next-edit receipt path"))?;

    cargo_bin_cmd!("xtask")
        .args([
            "semantic-inline-receipts",
            "--receipt",
            receipt_arg,
            "--quality-receipt",
            quality_arg,
            "--next-edit-receipt",
            next_edit_arg,
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
    assert_eq!(semantic_inline.len(), 15);
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
    assert_eq!(
        semantic_inline
            .get("gated_multiline_constructor")
            .and_then(|entry| entry.get("workflow_id"))
            .and_then(Value::as_str),
        Some("gated_multiline_constructor_inline_completion_quality")
    );
    assert_eq!(
        semantic_inline
            .get("package_boundary_receiver")
            .and_then(|entry| entry.get("workflow_id"))
            .and_then(Value::as_str),
        Some("package_boundary_receiver_inline_completion_quality")
    );
    assert_eq!(
        semantic_inline
            .get("project_test_assertion")
            .and_then(|entry| entry.get("workflow_id"))
            .and_then(Value::as_str),
        Some("project_test_assertion_inline_completion_quality")
    );
    assert_eq!(
        semantic_inline
            .get("project_control_flow")
            .and_then(|entry| entry.get("workflow_id"))
            .and_then(Value::as_str),
        Some("project_control_flow_inline_completion_quality")
    );
    assert_eq!(
        semantic_inline
            .get("project_constructor_style")
            .and_then(|entry| entry.get("workflow_id"))
            .and_then(Value::as_str),
        Some("project_constructor_inline_completion_quality")
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
    let next_edit_scaffold = dashboard
        .get("next_edit_scaffold")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("next_edit_scaffold map missing"))?;
    assert_eq!(next_edit_scaffold.get("available").and_then(Value::as_bool), Some(false));

    Ok(())
}

#[test]
fn semantic_inline_receipts_cli_embeds_quality_counters_when_available() -> Result<()> {
    let temp = TempDir::new()?;
    let receipt = temp.path().join("semantic-inline-receipts.json");
    let quality_receipt = temp.path().join("inline-completion-quality.json");
    let next_edit_receipt = temp.path().join("semantic-inline-next-edit.json");

    std::fs::write(
        &quality_receipt,
        serde_json::to_vec_pretty(&json!({
            "fixtures_total": 28,
            "fixtures_passed": 28,
            "checks": {
                "edit_application": {
                    "total": 15,
                    "passed": 15,
                    "failed": 0
                },
                "hard_zone_rejected": 14,
                "suppression_reasons": {
                    "hard_zone": 14,
                    "no_visible_context": 1
                },
                "parse_regressions": 0
            },
            "sources": {
                "module": {
                    "expected": 4,
                    "passed": 4,
                    "failed": 0,
                    "returned_items": 6,
                    "edit_application": {
                        "total": 4,
                        "passed": 4,
                        "failed": 0
                    },
                    "parse_regressions": 0,
                    "suppression_reasons": {}
                },
                "hard_zone": {
                    "expected": 14,
                    "passed": 14,
                    "failed": 0,
                    "returned_items": 0,
                    "edit_application": {
                        "total": 0,
                        "passed": 0,
                        "failed": 0
                    },
                    "parse_regressions": 0,
                    "suppression_reasons": {
                        "hard_zone": 14
                    }
                }
            }
        }))?,
    )?;

    cargo_bin_cmd!("xtask")
        .args(["semantic-inline-next-edit", "--receipt", &next_edit_receipt.display().to_string()])
        .assert()
        .success();

    cargo_bin_cmd!("xtask")
        .args([
            "semantic-inline-receipts",
            "--receipt",
            &receipt.display().to_string(),
            "--quality-receipt",
            &quality_receipt.display().to_string(),
            "--next-edit-receipt",
            &next_edit_receipt.display().to_string(),
        ])
        .assert()
        .success();

    let receipt_json: Value = serde_json::from_slice(&std::fs::read(&receipt)?)?;
    let quality_counters = receipt_json
        .get("quality_counters")
        .ok_or_else(|| anyhow!("semantic inline receipt omitted quality_counters"))?;

    assert_eq!(quality_counters.get("available").and_then(Value::as_bool), Some(true));
    assert_eq!(quality_counters.get("all_checks_green").and_then(Value::as_bool), Some(true));
    assert_eq!(quality_counters.get("fixtures_total").and_then(Value::as_u64), Some(28));
    assert_eq!(quality_counters.get("fixtures_passed").and_then(Value::as_u64), Some(28));
    assert_eq!(
        quality_counters
            .get("edit_application")
            .and_then(|edit_application| edit_application.get("total"))
            .and_then(Value::as_u64),
        Some(15)
    );
    assert_eq!(
        quality_counters
            .get("edit_application")
            .and_then(|edit_application| edit_application.get("passed"))
            .and_then(Value::as_u64),
        Some(15)
    );
    assert_eq!(
        quality_counters
            .get("edit_application")
            .and_then(|edit_application| edit_application.get("failed"))
            .and_then(Value::as_u64),
        Some(0)
    );
    assert_eq!(quality_counters.get("hard_zone_rejections").and_then(Value::as_u64), Some(14));
    assert_eq!(
        quality_counters
            .get("suppression_reasons")
            .and_then(|reasons| reasons.get("hard_zone"))
            .and_then(Value::as_u64),
        Some(14)
    );
    assert_eq!(
        quality_counters
            .get("suppression_reasons")
            .and_then(|reasons| reasons.get("no_visible_context"))
            .and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(quality_counters.get("parse_regressions").and_then(Value::as_u64), Some(0));
    assert_eq!(
        quality_counters
            .get("sources")
            .and_then(|sources| sources.get("module"))
            .and_then(|source| source.get("returned_items"))
            .and_then(Value::as_u64),
        Some(6)
    );
    assert_eq!(
        quality_counters
            .get("sources")
            .and_then(|sources| sources.get("module"))
            .and_then(|source| source.get("edit_application"))
            .and_then(|edit_application| edit_application.get("passed"))
            .and_then(Value::as_u64),
        Some(4)
    );
    assert_eq!(
        quality_counters
            .get("sources")
            .and_then(|sources| sources.get("hard_zone"))
            .and_then(|source| source.get("suppression_reasons"))
            .and_then(|suppression_reasons| suppression_reasons.get("hard_zone"))
            .and_then(Value::as_u64),
        Some(14)
    );
    let next_edit_scaffold = receipt_json
        .get("next_edit_scaffold")
        .ok_or_else(|| anyhow!("semantic inline receipt omitted next_edit_scaffold"))?;
    assert_eq!(next_edit_scaffold.get("available").and_then(Value::as_bool), Some(true));
    assert_eq!(
        next_edit_scaffold.get("schema_version").and_then(Value::as_str),
        Some("semantic-inline-next-edit.v1")
    );
    assert_eq!(next_edit_scaffold.get("enabled_by_default").and_then(Value::as_bool), Some(false));
    assert_eq!(
        next_edit_scaffold.get("runtime_provider_registered").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        next_edit_scaffold.get("ai_candidate_source_enabled").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        next_edit_scaffold
            .get("optional_ai_candidate_boundary")
            .and_then(|boundary| boundary.get("enabled_by_default"))
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        next_edit_scaffold
            .get("optional_ai_candidate_boundary")
            .and_then(|boundary| boundary.get("rejects_ai_enabled_policy"))
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        next_edit_scaffold
            .get("optional_ai_candidate_boundary")
            .and_then(|boundary| boundary.get("rejects_missing_parse_safety"))
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        next_edit_scaffold
            .get("optional_ai_candidate_boundary")
            .and_then(|boundary| boundary.get("deterministic_sources_only"))
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        next_edit_scaffold.get("explicit_gate_status").and_then(Value::as_str),
        Some("runtime_provider_not_registered")
    );
    assert_eq!(
        next_edit_scaffold
            .get("missing_import_next_action")
            .and_then(|action| action.get("reachable_candidate_prepared"))
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        next_edit_scaffold
            .get("missing_import_next_action")
            .and_then(|action| action.get("reachable_candidate_editor_visible"))
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        next_edit_scaffold
            .get("missing_import_next_action")
            .and_then(|action| action.get("reachable_candidate_reason"))
            .and_then(Value::as_str),
        Some("reachable_module_from_effective_inc")
    );
    assert_eq!(
        next_edit_scaffold
            .get("missing_import_next_action")
            .and_then(|action| action.get("duplicate_import_rejected"))
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        next_edit_scaffold
            .get("missing_import_next_action")
            .and_then(|action| action.get("unreachable_module_rejected"))
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        next_edit_scaffold
            .get("missing_import_next_action")
            .and_then(|action| action.get("parse_stable"))
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        next_edit_scaffold
            .get("missing_import_next_action")
            .and_then(|action| action.get("line_endings_preserved"))
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        next_edit_scaffold
            .get("missing_import_next_action")
            .and_then(|action| action.get("rejection_reasons"))
            .and_then(|reasons| reasons.get("duplicate_import"))
            .and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        next_edit_scaffold
            .get("missing_import_next_action")
            .and_then(|action| action.get("rejection_reasons"))
            .and_then(|reasons| reasons.get("unreachable_module"))
            .and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        next_edit_scaffold
            .get("missing_import_next_action")
            .and_then(|action| action.get("rejection_reasons"))
            .and_then(|reasons| reasons.get("gate_disabled"))
            .and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        next_edit_scaffold
            .get("missing_import_next_action")
            .and_then(|action| action.get("rejection_reasons"))
            .and_then(|reasons| reasons.get("runtime_provider_not_registered"))
            .and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        next_edit_scaffold
            .get("test_assertion_next_action")
            .and_then(|action| action.get("test_more_candidate_prepared"))
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        next_edit_scaffold
            .get("test_assertion_next_action")
            .and_then(|action| action.get("test_more_candidate_editor_visible"))
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        next_edit_scaffold
            .get("test_assertion_next_action")
            .and_then(|action| action.get("test_more_candidate_reason"))
            .and_then(Value::as_str),
        Some("visible_lexical_assertion")
    );
    assert_eq!(
        next_edit_scaffold
            .get("test_assertion_next_action")
            .and_then(|action| action.get("test2_candidate_prepared"))
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        next_edit_scaffold
            .get("test_assertion_next_action")
            .and_then(|action| action.get("test2_candidate_reason"))
            .and_then(Value::as_str),
        Some("visible_lexical_assertion")
    );
    assert_eq!(
        next_edit_scaffold
            .get("test_assertion_next_action")
            .and_then(|action| action.get("non_test_file_rejected"))
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        next_edit_scaffold
            .get("test_assertion_next_action")
            .and_then(|action| action.get("parse_stable"))
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        next_edit_scaffold
            .get("test_assertion_next_action")
            .and_then(|action| action.get("rejection_reasons"))
            .and_then(|reasons| reasons.get("test_file_required"))
            .and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        next_edit_scaffold
            .get("test_assertion_next_action")
            .and_then(|action| action.get("rejection_reasons"))
            .and_then(|reasons| reasons.get("unsupported_test_framework"))
            .and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        next_edit_scaffold
            .get("test_assertion_next_action")
            .and_then(|action| action.get("rejection_reasons"))
            .and_then(|reasons| reasons.get("missing_assertion_variables"))
            .and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        next_edit_scaffold
            .get("test_assertion_next_action")
            .and_then(|action| action.get("rejection_reasons"))
            .and_then(|reasons| reasons.get("gate_disabled"))
            .and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        next_edit_scaffold
            .get("test_assertion_next_action")
            .and_then(|action| action.get("rejection_reasons"))
            .and_then(|reasons| reasons.get("runtime_provider_not_registered"))
            .and_then(Value::as_u64),
        Some(1)
    );

    Ok(())
}

#[test]
fn semantic_inline_receipts_cli_rejects_missing_next_edit_candidate_families() -> Result<()> {
    let temp = TempDir::new()?;
    let receipt = temp.path().join("semantic-inline-receipts.json");
    let missing_quality_receipt = temp.path().join("missing-inline-quality.json");
    let next_edit_receipt = temp.path().join("semantic-inline-next-edit.json");
    std::fs::write(
        &next_edit_receipt,
        serde_json::to_vec_pretty(&next_edit_receipt_without_candidate_families_json())?,
    )?;

    let output = cargo_bin_cmd!("xtask")
        .args([
            "semantic-inline-receipts",
            "--receipt",
            &receipt.display().to_string(),
            "--quality-receipt",
            &missing_quality_receipt.display().to_string(),
            "--next-edit-receipt",
            &next_edit_receipt.display().to_string(),
        ])
        .output()?;

    assert!(
        !output.status.success(),
        "dashboard generation should reject incomplete next-edit receipts"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("planned_candidate_families"),
        "error should identify the missing candidate-family list, got {stderr}"
    );

    Ok(())
}

#[test]
fn semantic_inline_receipts_cli_rejects_non_string_next_edit_candidate_family() -> Result<()> {
    let temp = TempDir::new()?;
    let receipt = temp.path().join("semantic-inline-receipts.json");
    let missing_quality_receipt = temp.path().join("missing-inline-quality.json");
    let next_edit_receipt = temp.path().join("semantic-inline-next-edit.json");
    let mut next_edit = valid_next_edit_receipt_json();
    next_edit["planned_candidate_families"] = json!(["missing_import", 42]);
    std::fs::write(&next_edit_receipt, serde_json::to_vec_pretty(&next_edit)?)?;

    let output = cargo_bin_cmd!("xtask")
        .args([
            "semantic-inline-receipts",
            "--receipt",
            &receipt.display().to_string(),
            "--quality-receipt",
            &missing_quality_receipt.display().to_string(),
            "--next-edit-receipt",
            &next_edit_receipt.display().to_string(),
        ])
        .output()?;

    assert!(
        !output.status.success(),
        "dashboard generation should reject malformed next-edit lists"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("planned_candidate_families") && stderr.contains("entries must be strings"),
        "error should identify the malformed candidate-family entry, got {stderr}"
    );

    Ok(())
}

#[test]
fn semantic_inline_receipts_cli_rejects_enabled_next_edit_runtime_provider() -> Result<()> {
    let temp = TempDir::new()?;
    let receipt = temp.path().join("semantic-inline-receipts.json");
    let missing_quality_receipt = temp.path().join("missing-inline-quality.json");
    let next_edit_receipt = temp.path().join("semantic-inline-next-edit.json");
    let mut next_edit = valid_next_edit_receipt_json();
    next_edit["runtime_provider_registered"] = json!(true);
    std::fs::write(&next_edit_receipt, serde_json::to_vec_pretty(&next_edit)?)?;

    let output = cargo_bin_cmd!("xtask")
        .args([
            "semantic-inline-receipts",
            "--receipt",
            &receipt.display().to_string(),
            "--quality-receipt",
            &missing_quality_receipt.display().to_string(),
            "--next-edit-receipt",
            &next_edit_receipt.display().to_string(),
        ])
        .output()?;

    assert!(!output.status.success(), "dashboard generation should reject runtime next-edit drift");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("runtime_provider_registered"),
        "error should identify runtime next-edit drift, got {stderr}"
    );

    Ok(())
}

#[test]
fn semantic_inline_receipts_cli_rejects_missing_import_line_ending_drift() -> Result<()> {
    let temp = TempDir::new()?;
    let receipt = temp.path().join("semantic-inline-receipts.json");
    let missing_quality_receipt = temp.path().join("missing-inline-quality.json");
    let next_edit_receipt = temp.path().join("semantic-inline-next-edit.json");
    let mut next_edit = valid_next_edit_receipt_json();
    next_edit["missing_import_next_action"]["line_endings_preserved"] = json!(false);
    std::fs::write(&next_edit_receipt, serde_json::to_vec_pretty(&next_edit)?)?;

    let output = cargo_bin_cmd!("xtask")
        .args([
            "semantic-inline-receipts",
            "--receipt",
            &receipt.display().to_string(),
            "--quality-receipt",
            &missing_quality_receipt.display().to_string(),
            "--next-edit-receipt",
            &next_edit_receipt.display().to_string(),
        ])
        .output()?;

    assert!(
        !output.status.success(),
        "dashboard generation should reject missing-import line-ending drift"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("line_endings_preserved") || stderr.contains("line endings"),
        "error should identify line-ending drift, got {stderr}"
    );

    Ok(())
}

#[test]
fn semantic_inline_receipts_cli_rejects_test_assertion_runtime_drift() -> Result<()> {
    let temp = TempDir::new()?;
    let receipt = temp.path().join("semantic-inline-receipts.json");
    let missing_quality_receipt = temp.path().join("missing-inline-quality.json");
    let next_edit_receipt = temp.path().join("semantic-inline-next-edit.json");
    let mut next_edit = valid_next_edit_receipt_json();
    next_edit["test_assertion_next_action"]["test_more_candidate"]["candidate"]["editorVisible"] =
        json!(true);
    std::fs::write(&next_edit_receipt, serde_json::to_vec_pretty(&next_edit)?)?;

    let output = cargo_bin_cmd!("xtask")
        .args([
            "semantic-inline-receipts",
            "--receipt",
            &receipt.display().to_string(),
            "--quality-receipt",
            &missing_quality_receipt.display().to_string(),
            "--next-edit-receipt",
            &next_edit_receipt.display().to_string(),
        ])
        .output()?;

    assert!(
        !output.status.success(),
        "dashboard generation should reject editor-visible test assertion next actions"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("test_assertion_next_action") || stderr.contains("editorVisible"),
        "error should identify test assertion next-action drift, got {stderr}"
    );

    Ok(())
}

fn valid_next_edit_receipt_json() -> Value {
    json!({
        "schema_version": "semantic-inline-next-edit.v1",
        "provider_action": "next_edit_scaffold",
        "enabled_by_default": false,
        "runtime_provider_registered": false,
        "ai_candidate_source_enabled": false,
        "default_response": {
            "status": "disabled",
            "suggestions": []
        },
        "receipt_only_response": {
            "status": "receipt_only",
            "suggestions": []
        },
        "explicit_gate_response": {
            "status": "runtime_provider_not_registered",
            "suggestions": []
        },
        "planned_candidate_families": [
            "missing_import",
            "test_assertion_body",
            "call_site_update",
            "rename_occurrence"
        ],
        "future_gated": [
            "runtime_next_edit_provider",
            "editor_visible_next_edit_suggestions",
            "missing_import_next_action",
            "test_assertion_next_action",
            "optional_ai_candidate_source"
        ],
        "missing_import_next_action": valid_missing_import_next_action_json(),
        "test_assertion_next_action": valid_test_assertion_next_action_json(),
        "optional_ai_candidate_boundary": valid_optional_ai_candidate_boundary_json()
    })
}

fn next_edit_receipt_without_candidate_families_json() -> Value {
    json!({
        "schema_version": "semantic-inline-next-edit.v1",
        "provider_action": "next_edit_scaffold",
        "enabled_by_default": false,
        "runtime_provider_registered": false,
        "ai_candidate_source_enabled": false,
        "default_response": {
            "status": "disabled",
            "suggestions": []
        },
        "receipt_only_response": {
            "status": "receipt_only",
            "suggestions": []
        },
        "explicit_gate_response": {
            "status": "runtime_provider_not_registered",
            "suggestions": []
        },
        "future_gated": [
            "runtime_next_edit_provider",
            "editor_visible_next_edit_suggestions",
            "missing_import_next_action",
            "test_assertion_next_action",
            "optional_ai_candidate_source"
        ],
        "missing_import_next_action": valid_missing_import_next_action_json(),
        "test_assertion_next_action": valid_test_assertion_next_action_json(),
        "optional_ai_candidate_boundary": valid_optional_ai_candidate_boundary_json()
    })
}

fn valid_optional_ai_candidate_boundary_json() -> Value {
    json!({
        "claim_boundary": "optional AI candidate boundary proof only",
        "enabled_by_default": false,
        "ai_candidate_source_enabled": false,
        "default_response_suggestions_empty": true,
        "receipt_only_response_suggestions_empty": true,
        "explicit_gate_response_suggestions_empty": true,
        "rejects_ai_enabled_policy": true,
        "rejects_missing_editor_safe_range": true,
        "rejects_missing_parse_safety": true,
        "rejects_missing_selected_completion_compatibility": true,
        "rejects_nondeterministic_sources": true,
        "deterministic_sources_only": true
    })
}

fn valid_missing_import_next_action_json() -> Value {
    json!({
        "claim_boundary": "receipt-only missing-import next-action proof",
        "reachable_candidate": {
            "status": "receipt_only",
            "candidate": {
                "family": "missing_import",
                "module": "My::App",
                "reason": "reachable_module_from_effective_inc",
                "edit": {
                    "startByte": 26,
                    "endByte": 26,
                    "newText": "use My::App;\n"
                },
                "editorVisible": false
            },
            "rejectionReasons": []
        },
        "duplicate_import": {
            "status": "receipt_only",
            "rejectionReasons": ["duplicate_import"]
        },
        "unreachable_module": {
            "status": "receipt_only",
            "rejectionReasons": ["unreachable_module"]
        },
        "default_gate": {
            "status": "disabled",
            "rejectionReasons": ["gate_disabled"]
        },
        "explicit_gate": {
            "status": "runtime_provider_not_registered",
            "rejectionReasons": ["runtime_provider_not_registered"]
        },
        "accepted_document_text": "use strict;\nuse warnings;\nuse My::App;\nmy $value = My::App->new;\n",
        "crlf_accepted_document_text": "package Demo;\r\nuse strict;\r\nuse My::App;\r\nmy $value = My::App->new;\r\n",
        "line_endings_preserved": true,
        "parse_stable": true
    })
}

fn valid_test_assertion_next_action_json() -> Value {
    json!({
        "claim_boundary": "receipt-only test assertion next-action proof",
        "test_more_candidate": {
            "status": "receipt_only",
            "candidate": {
                "family": "test_assertion_body",
                "framework": "test_more",
                "reason": "visible_lexical_assertion",
                "edit": {
                    "startByte": 56,
                    "endByte": 56,
                    "newText": "is($got, $expected, 'test description');\n"
                },
                "editorVisible": false
            },
            "rejectionReasons": []
        },
        "test2_candidate": {
            "status": "receipt_only",
            "candidate": {
                "family": "test_assertion_body",
                "framework": "test2_v0",
                "reason": "visible_lexical_assertion",
                "edit": {
                    "startByte": 54,
                    "endByte": 54,
                    "newText": "is($result, $want, 'test description');\n"
                },
                "editorVisible": false
            },
            "rejectionReasons": []
        },
        "non_test_file": {
            "status": "receipt_only",
            "rejectionReasons": ["test_file_required"]
        },
        "unsupported_framework": {
            "status": "receipt_only",
            "rejectionReasons": ["unsupported_test_framework"]
        },
        "missing_variables": {
            "status": "receipt_only",
            "rejectionReasons": ["missing_assertion_variables"]
        },
        "default_gate": {
            "status": "disabled",
            "rejectionReasons": ["gate_disabled"]
        },
        "explicit_gate": {
            "status": "runtime_provider_not_registered",
            "rejectionReasons": ["runtime_provider_not_registered"]
        },
        "accepted_document_text": "use Test::More;\nmy $got = compute();\nmy $expected = 42;\nis($got, $expected, 'test description');\n",
        "parse_stable": true
    })
}
