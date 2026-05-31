use color_eyre::eyre::{Result, bail};
use perl_lsp_rs_core::providers::inline_completion::{
    MissingImportNextEditProof, MissingImportNextEditRequest, NextEditCandidateFamily,
    NextEditFeatureGate, NextEditProvider, NextEditRejectionReason, NextEditRequest,
    NextEditResponse, NextEditSafetyPolicy, NextEditStatus, PreparedInlineCompletionContext,
};
use perl_parser::Parser;
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
    missing_import_next_action: MissingImportNextActionReceipt,
    future_gated: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct MissingImportNextActionReceipt {
    claim_boundary: &'static str,
    reachable_candidate: MissingImportNextEditProof,
    duplicate_import: MissingImportNextEditProof,
    unreachable_module: MissingImportNextEditProof,
    default_gate: MissingImportNextEditProof,
    explicit_gate: MissingImportNextEditProof,
    accepted_document_text: String,
    parse_stable: bool,
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
    let missing_import_next_action = missing_import_next_action_receipt(&provider)?;

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
        missing_import_next_action,
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

fn missing_import_next_action_receipt(
    provider: &NextEditProvider,
) -> Result<MissingImportNextActionReceipt> {
    let source = "use strict;\nuse warnings;\nmy $value = My::App->new;\n";
    let expected = "use strict;\nuse warnings;\nuse My::App;\nmy $value = My::App->new;\n";
    let reachable = provider.prove_missing_import(&MissingImportNextEditRequest::receipt_only(
        source,
        "My::App",
        vec!["My::App".to_string()],
        vec!["strict".to_string(), "warnings".to_string()],
    ));
    let duplicate = provider.prove_missing_import(&MissingImportNextEditRequest::receipt_only(
        "use strict;\nuse warnings;\nuse My::App;\nmy $value = My::App->new;\n",
        "My::App",
        vec!["My::App".to_string()],
        vec!["My::App".to_string()],
    ));
    let unreachable = provider.prove_missing_import(&MissingImportNextEditRequest::receipt_only(
        source,
        "My::Missing",
        vec!["My::App".to_string()],
        vec!["strict".to_string(), "warnings".to_string()],
    ));

    let mut default_gate = MissingImportNextEditRequest::receipt_only(
        source,
        "My::App",
        vec!["My::App".to_string()],
        vec![],
    );
    default_gate.gate = NextEditFeatureGate::default();
    let default_gate = provider.prove_missing_import(&default_gate);

    let mut explicit_gate = MissingImportNextEditRequest::receipt_only(
        source,
        "My::App",
        vec!["My::App".to_string()],
        vec![],
    );
    explicit_gate.gate = NextEditFeatureGate::explicit_enabled();
    let explicit_gate = provider.prove_missing_import(&explicit_gate);

    let candidate = reachable.candidate.as_ref().ok_or_else(|| {
        color_eyre::eyre::eyre!("reachable missing-import proof omitted candidate")
    })?;
    let accepted_document_text = candidate
        .edit
        .apply_to(source)
        .ok_or_else(|| color_eyre::eyre::eyre!("reachable missing-import edit did not apply"))?;
    if accepted_document_text != expected {
        bail!("reachable missing-import edit produced unexpected document text");
    }
    let parse_stable = parse_succeeds(source) && parse_succeeds(&accepted_document_text);
    if !parse_stable {
        bail!("reachable missing-import edit did not preserve parse success");
    }

    let receipt = MissingImportNextActionReceipt {
        claim_boundary: "receipt-only missing-import next-action proof; no runtime LSP method, editor-visible next-edit provider, source mirror, release action, or AI behavior",
        reachable_candidate: reachable,
        duplicate_import: duplicate,
        unreachable_module: unreachable,
        default_gate,
        explicit_gate,
        accepted_document_text,
        parse_stable,
    };
    validate_missing_import_next_action(&receipt)?;
    Ok(receipt)
}

fn parse_succeeds(source: &str) -> bool {
    let mut parser = Parser::new(source);
    parser.parse().is_ok()
}

fn validate_missing_import_next_action(receipt: &MissingImportNextActionReceipt) -> Result<()> {
    let Some(candidate) = receipt.reachable_candidate.candidate.as_ref() else {
        bail!("reachable missing-import proof must prepare a receipt-only candidate");
    };
    if receipt.reachable_candidate.status != NextEditStatus::ReceiptOnly
        || candidate.family != NextEditCandidateFamily::MissingImport
        || candidate.module != "My::App"
        || candidate.editor_visible
        || candidate.edit.new_text != "use My::App;\n"
        || !receipt.reachable_candidate.rejection_reasons.is_empty()
    {
        bail!("reachable missing-import proof did not satisfy the receipt-only contract");
    }
    if receipt.duplicate_import.candidate.is_some()
        || !receipt
            .duplicate_import
            .rejection_reasons
            .contains(&NextEditRejectionReason::DuplicateImport)
    {
        bail!("duplicate missing-import proof must reject duplicate imports");
    }
    if receipt.unreachable_module.candidate.is_some()
        || !receipt
            .unreachable_module
            .rejection_reasons
            .contains(&NextEditRejectionReason::UnreachableModule)
    {
        bail!("unreachable missing-import proof must reject unreachable modules");
    }
    if receipt.default_gate.status != NextEditStatus::Disabled
        || receipt.default_gate.candidate.is_some()
        || !receipt.default_gate.rejection_reasons.contains(&NextEditRejectionReason::GateDisabled)
    {
        bail!("missing-import next action must remain disabled by default");
    }
    if receipt.explicit_gate.status != NextEditStatus::RuntimeProviderNotRegistered
        || receipt.explicit_gate.candidate.is_some()
        || !receipt
            .explicit_gate
            .rejection_reasons
            .contains(&NextEditRejectionReason::RuntimeProviderNotRegistered)
    {
        bail!("missing-import next action must not bypass the unregistered runtime provider");
    }
    if !receipt.parse_stable {
        bail!("missing-import next action must keep local parse state stable");
    }
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
        assert_eq!(
            value
                .pointer("/missing_import_next_action/reachable_candidate/status")
                .and_then(Value::as_str),
            Some("receipt_only")
        );
        assert_eq!(
            value
                .pointer("/missing_import_next_action/reachable_candidate/candidate/module")
                .and_then(Value::as_str),
            Some("My::App")
        );
        assert_eq!(
            value
                .pointer("/missing_import_next_action/reachable_candidate/candidate/editorVisible")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            value
                .pointer("/missing_import_next_action/duplicate_import/rejectionReasons/0")
                .and_then(Value::as_str),
            Some("duplicate_import")
        );
        assert_eq!(
            value
                .pointer("/missing_import_next_action/unreachable_module/rejectionReasons/0")
                .and_then(Value::as_str),
            Some("unreachable_module")
        );
        assert_eq!(
            value
                .pointer("/missing_import_next_action/default_gate/status")
                .and_then(Value::as_str),
            Some("disabled")
        );
        assert_eq!(
            value
                .pointer("/missing_import_next_action/explicit_gate/status")
                .and_then(Value::as_str),
            Some("runtime_provider_not_registered")
        );
        assert_eq!(
            value.pointer("/missing_import_next_action/parse_stable").and_then(Value::as_bool),
            Some(true)
        );

        Ok(())
    }

    #[test]
    fn missing_import_next_action_validation_rejects_contract_drift() -> Result<()> {
        let provider = NextEditProvider;
        let mut receipt = missing_import_next_action_receipt(&provider)?;
        receipt.reachable_candidate.candidate = None;
        let error = validate_missing_import_next_action(&receipt)
            .expect_err("missing reachable candidate must fail validation");
        assert!(
            error.to_string().contains("prepare a receipt-only candidate"),
            "error should identify missing reachable candidate, got {error}"
        );

        let mut receipt = missing_import_next_action_receipt(&provider)?;
        let candidate = receipt
            .reachable_candidate
            .candidate
            .as_mut()
            .ok_or_else(|| color_eyre::eyre::eyre!("valid receipt omitted candidate"))?;
        candidate.editor_visible = true;
        let error = validate_missing_import_next_action(&receipt)
            .expect_err("editor-visible reachable candidate must fail validation");
        assert!(
            error.to_string().contains("receipt-only contract"),
            "error should identify reachable candidate contract drift, got {error}"
        );

        let mut receipt = missing_import_next_action_receipt(&provider)?;
        receipt.parse_stable = false;
        let error = validate_missing_import_next_action(&receipt)
            .expect_err("parse-unstable missing-import action must fail validation");
        assert!(
            error.to_string().contains("parse state stable"),
            "error should identify parse-stability drift, got {error}"
        );

        Ok(())
    }

    #[test]
    fn missing_import_next_action_validation_rejects_rejection_drift() -> Result<()> {
        let provider = NextEditProvider;
        let mut receipt = missing_import_next_action_receipt(&provider)?;
        receipt.duplicate_import.rejection_reasons.clear();
        let error = validate_missing_import_next_action(&receipt)
            .expect_err("duplicate import without rejection reason must fail validation");
        assert!(
            error.to_string().contains("reject duplicate imports"),
            "error should identify duplicate-import drift, got {error}"
        );

        let mut receipt = missing_import_next_action_receipt(&provider)?;
        receipt.unreachable_module.rejection_reasons.clear();
        let error = validate_missing_import_next_action(&receipt)
            .expect_err("unreachable module without rejection reason must fail validation");
        assert!(
            error.to_string().contains("reject unreachable modules"),
            "error should identify unreachable-module drift, got {error}"
        );

        let mut receipt = missing_import_next_action_receipt(&provider)?;
        receipt.default_gate.rejection_reasons.clear();
        let error = validate_missing_import_next_action(&receipt)
            .expect_err("default gate without rejection reason must fail validation");
        assert!(
            error.to_string().contains("disabled by default"),
            "error should identify default-gate drift, got {error}"
        );

        let mut receipt = missing_import_next_action_receipt(&provider)?;
        receipt.explicit_gate.rejection_reasons.clear();
        let error = validate_missing_import_next_action(&receipt)
            .expect_err("explicit gate without runtime rejection reason must fail validation");
        assert!(
            error.to_string().contains("unregistered runtime provider"),
            "error should identify explicit-gate drift, got {error}"
        );

        Ok(())
    }

    #[test]
    fn parse_succeeds_accepts_valid_documents() -> Result<()> {
        assert!(parse_succeeds("use strict;\nmy $value = 1;\n"));
        Ok(())
    }
}
