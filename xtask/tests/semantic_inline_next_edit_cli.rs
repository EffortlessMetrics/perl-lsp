use anyhow::{Result, anyhow};
use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn semantic_inline_next_edit_cli_writes_scaffold_receipt() -> Result<()> {
    let temp = TempDir::new()?;
    let receipt = temp.path().join("semantic-inline-next-edit.json");
    let receipt_arg = receipt.to_str().ok_or_else(|| anyhow!("invalid next-edit receipt path"))?;

    cargo_bin_cmd!("xtask")
        .args(["semantic-inline-next-edit", "--receipt", receipt_arg])
        .assert()
        .success();

    let scaffold: Value = serde_json::from_str(&std::fs::read_to_string(&receipt)?)?;
    assert_eq!(
        scaffold.get("schema_version").and_then(Value::as_str),
        Some("semantic-inline-next-edit.v1")
    );
    assert_eq!(scaffold.get("provider").and_then(Value::as_str), Some("inline_completion"));
    assert_eq!(scaffold.get("provider_action").and_then(Value::as_str), Some("next_edit_scaffold"));
    assert_eq!(scaffold.get("enabled_by_default").and_then(Value::as_bool), Some(false));
    assert_eq!(scaffold.get("runtime_provider_registered").and_then(Value::as_bool), Some(false));
    assert_eq!(scaffold.get("ai_candidate_source_enabled").and_then(Value::as_bool), Some(false));
    assert_eq!(
        scaffold.pointer("/default_response/status").and_then(Value::as_str),
        Some("disabled")
    );
    assert_eq!(
        scaffold.pointer("/receipt_only_response/status").and_then(Value::as_str),
        Some("receipt_only")
    );
    assert_eq!(
        scaffold.pointer("/explicit_gate_response/status").and_then(Value::as_str),
        Some("runtime_provider_not_registered")
    );
    assert!(
        scaffold
            .pointer("/explicit_gate_response/suggestions")
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty)
    );

    let planned = scaffold
        .get("planned_candidate_families")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("planned_candidate_families missing"))?;
    for family in ["missing_import", "test_assertion_body", "call_site_update", "rename_occurrence"]
    {
        assert!(planned.iter().any(|entry| entry.as_str() == Some(family)));
    }

    let boundary = scaffold
        .get("claim_boundary")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("claim_boundary missing"))?;
    assert!(boundary.contains("does not register an LSP method"));
    assert!(boundary.contains("emit editor-visible next-edit suggestions"));
    assert!(boundary.contains("enable AI behavior"));

    Ok(())
}
