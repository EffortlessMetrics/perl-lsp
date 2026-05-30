use color_eyre::eyre::{Result, bail};
use perl_lsp_rs_core::providers::inline_completion::{
    NextEditCandidateFamily, NextEditFeatureGate, NextEditProvider, NextEditRequest,
    NextEditResponse, NextEditSafetyPolicy, NextEditStatus, PreparedInlineCompletionContext,
};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct SemanticInlineNextEditReceipt {
    schema_version: &'static str,
    provider: &'static str,
    provider_action: &'static str,
    claim_boundary: &'static str,
    enabled_by_default: bool,
    runtime_provider_registered: bool,
    ai_candidate_source_enabled: bool,
    planned_candidate_families: &'static [NextEditCandidateFamily],
    safety_policy: NextEditSafetyPolicy,
    default_response: NextEditResponse,
    receipt_only_response: NextEditResponse,
    explicit_gate_response: NextEditResponse,
    future_gated: Vec<&'static str>,
}

pub fn run(receipt: PathBuf) -> Result<()> {
    let provider = NextEditProvider;
    let context = scaffold_context();
    let mut request = NextEditRequest::receipt_only(context);
    let receipt_only_response = provider.suggest(&request);

    request.gate = NextEditFeatureGate::default();
    let default_response = provider.suggest(&request);

    request.gate = NextEditFeatureGate::explicit_enabled();
    let explicit_gate_response = provider.suggest(&request);

    validate_scaffold_responses(
        &default_response,
        &receipt_only_response,
        &explicit_gate_response,
        &request.safety_policy,
    )?;

    let receipt_data = SemanticInlineNextEditReceipt {
        schema_version: "semantic-inline-next-edit.v1",
        provider: "inline_completion",
        provider_action: "next_edit_scaffold",
        claim_boundary: "machine-readable next-edit scaffold receipt only; does not register an LSP method, emit editor-visible next-edit suggestions, mirror to source, release, or enable AI behavior",
        enabled_by_default: false,
        runtime_provider_registered: false,
        ai_candidate_source_enabled: request.safety_policy.ai_source_enabled,
        planned_candidate_families: NextEditCandidateFamily::planned(),
        safety_policy: request.safety_policy,
        default_response,
        receipt_only_response,
        explicit_gate_response,
        future_gated: vec![
            "runtime_next_edit_provider",
            "editor_visible_next_edit_suggestions",
            "missing_import_next_action",
            "optional_ai_candidate_source",
        ],
    };

    write_receipt(&receipt, &receipt_data)?;
    println!("semantic inline next-edit scaffold receipt OK: {}", receipt.display());
    Ok(())
}

fn scaffold_context() -> PreparedInlineCompletionContext {
    PreparedInlineCompletionContext {
        prefix: "use My::".to_string(),
        current_line: "use My::".to_string(),
        previous_non_empty_line: Some("use strict;".to_string()),
        current_function: None,
        current_package: Some("Demo".to_string()),
        variables: vec!["$got".to_string(), "$expected".to_string()],
        imports: vec!["strict".to_string(), "warnings".to_string()],
    }
}

fn validate_scaffold_responses(
    default_response: &NextEditResponse,
    receipt_only_response: &NextEditResponse,
    explicit_gate_response: &NextEditResponse,
    safety_policy: &NextEditSafetyPolicy,
) -> Result<()> {
    if default_response.status != NextEditStatus::Disabled {
        bail!("next-edit scaffold must default to disabled");
    }
    if receipt_only_response.status != NextEditStatus::ReceiptOnly {
        bail!("next-edit scaffold receipt mode must remain receipt-only");
    }
    if explicit_gate_response.status != NextEditStatus::RuntimeProviderNotRegistered {
        bail!("next-edit scaffold must not claim a registered runtime provider");
    }
    if !default_response.suggestions.is_empty()
        || !receipt_only_response.suggestions.is_empty()
        || !explicit_gate_response.suggestions.is_empty()
    {
        bail!("next-edit scaffold emitted editor-visible suggestions");
    }
    if !safety_policy.requires_editor_safe_range
        || !safety_policy.requires_parse_safety
        || !safety_policy.requires_selected_completion_compatibility
        || !safety_policy.deterministic_sources_only
        || safety_policy.ai_source_enabled
    {
        bail!("next-edit scaffold safety policy is weaker than the inline lane contract");
    }
    Ok(())
}

fn write_receipt(path: &Path, receipt: &SemanticInlineNextEditReceipt) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(receipt)?;
    fs::write(path, format!("{json}\n"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_lsp_rs_core::providers::inline_completion::NextEditSuggestion;
    use serde_json::Value;
    use tempfile::TempDir;

    #[test]
    fn receipt_validation_rejects_editor_visible_suggestions() -> Result<()> {
        let response = NextEditResponse::new(NextEditStatus::ReceiptOnly, vec![]);
        let emitted = NextEditResponse::new(
            NextEditStatus::RuntimeProviderNotRegistered,
            vec![NextEditSuggestion::new(NextEditCandidateFamily::MissingImport, "use My::App;\n")],
        );

        assert!(
            validate_scaffold_responses(
                &NextEditResponse::new(NextEditStatus::Disabled, vec![]),
                &response,
                &emitted,
                &NextEditSafetyPolicy::default(),
            )
            .is_err()
        );
        Ok(())
    }

    #[test]
    fn receipt_writer_records_next_edit_boundary() -> Result<()> {
        let temp = TempDir::new()?;
        let receipt_path = temp.path().join("semantic-inline-next-edit.json");

        run(receipt_path.clone())?;

        let value: Value = serde_json::from_str(&fs::read_to_string(receipt_path)?)?;
        assert_eq!(
            value.get("schema_version").and_then(Value::as_str),
            Some("semantic-inline-next-edit.v1")
        );
        assert_eq!(
            value.get("provider_action").and_then(Value::as_str),
            Some("next_edit_scaffold")
        );
        assert_eq!(value.get("enabled_by_default").and_then(Value::as_bool), Some(false));
        assert_eq!(value.get("runtime_provider_registered").and_then(Value::as_bool), Some(false));
        assert_eq!(value.get("ai_candidate_source_enabled").and_then(Value::as_bool), Some(false));
        assert_eq!(
            value.pointer("/default_response/status").and_then(Value::as_str),
            Some("disabled")
        );
        assert_eq!(
            value.pointer("/receipt_only_response/status").and_then(Value::as_str),
            Some("receipt_only")
        );
        assert_eq!(
            value.pointer("/explicit_gate_response/status").and_then(Value::as_str),
            Some("runtime_provider_not_registered")
        );
        assert_eq!(
            value.pointer("/explicit_gate_response/suggestions").and_then(Value::as_array),
            Some(&Vec::new())
        );
        assert_eq!(
            value.get("planned_candidate_families").and_then(Value::as_array).map(Vec::len),
            Some(NextEditCandidateFamily::planned().len())
        );

        Ok(())
    }
}
