use color_eyre::eyre::{Result, bail};
use perl_lsp_rs_core::providers::inline_completion::{
    CallSiteUpdateNextEditProof, CallSiteUpdateNextEditRequest, MissingImportNextEditProof,
    MissingImportNextEditRequest, NextEditCandidateFamily, NextEditFeatureGate, NextEditProvider,
    NextEditRejectionReason, NextEditRequest, NextEditResponse, NextEditSafetyPolicy,
    NextEditStatus, PreparedInlineCompletionContext, RenameOccurrenceNextEditProof,
    RenameOccurrenceNextEditRequest, TestAssertionNextEditProof, TestAssertionNextEditRequest,
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
    test_assertion_next_action: TestAssertionNextActionReceipt,
    call_site_update_next_action: CallSiteUpdateNextActionReceipt,
    rename_occurrence_next_action: RenameOccurrenceNextActionReceipt,
    optional_ai_candidate_boundary: OptionalAiCandidateBoundaryReceipt,
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
    crlf_accepted_document_text: String,
    parse_stable: bool,
    line_endings_preserved: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct TestAssertionNextActionReceipt {
    claim_boundary: &'static str,
    test_more_candidate: TestAssertionNextEditProof,
    test2_candidate: TestAssertionNextEditProof,
    non_test_file: TestAssertionNextEditProof,
    unsupported_framework: TestAssertionNextEditProof,
    missing_variables: TestAssertionNextEditProof,
    default_gate: TestAssertionNextEditProof,
    explicit_gate: TestAssertionNextEditProof,
    accepted_document_text: String,
    parse_stable: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct CallSiteUpdateNextActionReceipt {
    claim_boundary: &'static str,
    next_call_site_candidate: CallSiteUpdateNextEditProof,
    duplicate_argument: CallSiteUpdateNextEditProof,
    missing_call_site: CallSiteUpdateNextEditProof,
    unsafe_call_site: CallSiteUpdateNextEditProof,
    invalid_target: CallSiteUpdateNextEditProof,
    missing_argument: CallSiteUpdateNextEditProof,
    default_gate: CallSiteUpdateNextEditProof,
    explicit_gate: CallSiteUpdateNextEditProof,
    accepted_document_text: String,
    parse_stable: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct RenameOccurrenceNextActionReceipt {
    claim_boundary: &'static str,
    next_occurrence_candidate: RenameOccurrenceNextEditProof,
    unsafe_occurrence: RenameOccurrenceNextEditProof,
    missing_occurrence: RenameOccurrenceNextEditProof,
    invalid_symbol: RenameOccurrenceNextEditProof,
    default_gate: RenameOccurrenceNextEditProof,
    explicit_gate: RenameOccurrenceNextEditProof,
    accepted_document_text: String,
    parse_stable: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct OptionalAiCandidateBoundaryReceipt {
    claim_boundary: &'static str,
    enabled_by_default: bool,
    ai_candidate_source_enabled: bool,
    default_response_suggestions_empty: bool,
    receipt_only_response_suggestions_empty: bool,
    explicit_gate_response_suggestions_empty: bool,
    rejects_ai_enabled_policy: bool,
    rejects_missing_editor_safe_range: bool,
    rejects_missing_parse_safety: bool,
    rejects_missing_selected_completion_compatibility: bool,
    rejects_nondeterministic_sources: bool,
    deterministic_sources_only: bool,
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
    let test_assertion_next_action = test_assertion_next_action_receipt(&provider)?;
    let call_site_update_next_action = call_site_update_next_action_receipt(&provider)?;
    let rename_occurrence_next_action = rename_occurrence_next_action_receipt(&provider)?;
    let optional_ai_candidate_boundary = optional_ai_candidate_boundary_receipt(
        &default_response,
        &receipt_only_response,
        &explicit_gate_response,
        request.safety_policy,
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
        missing_import_next_action,
        test_assertion_next_action,
        call_site_update_next_action,
        rename_occurrence_next_action,
        optional_ai_candidate_boundary,
        future_gated: vec![
            "runtime_next_edit_provider",
            "editor_visible_next_edit_suggestions",
            "missing_import_next_action",
            "test_assertion_next_action",
            "call_site_update_next_action",
            "rename_occurrence_next_action",
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

    let crlf_source = "package Demo;\r\nuse strict;\r\nmy $value = My::App->new;\r\n";
    let crlf_expected =
        "package Demo;\r\nuse strict;\r\nuse My::App;\r\nmy $value = My::App->new;\r\n";
    let crlf_reachable =
        provider.prove_missing_import(&MissingImportNextEditRequest::receipt_only(
            crlf_source,
            "My::App",
            vec!["My::App".to_string()],
            vec!["strict".to_string()],
        ));

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
    let crlf_candidate = crlf_reachable
        .candidate
        .as_ref()
        .ok_or_else(|| color_eyre::eyre::eyre!("CRLF missing-import proof omitted candidate"))?;
    let crlf_accepted_document_text = crlf_candidate
        .edit
        .apply_to(crlf_source)
        .ok_or_else(|| color_eyre::eyre::eyre!("CRLF missing-import edit did not apply"))?;
    if crlf_accepted_document_text != crlf_expected {
        bail!("CRLF missing-import edit produced unexpected document text");
    }
    let line_endings_preserved = crlf_candidate.edit.new_text.ends_with("\r\n")
        && crlf_accepted_document_text.contains("use My::App;\r\nmy $value")
        && !crlf_accepted_document_text.contains("use My::App;\nmy $value");
    if !line_endings_preserved {
        bail!("missing-import next action did not preserve CRLF line endings");
    }

    let receipt = MissingImportNextActionReceipt {
        claim_boundary: "receipt-only missing-import next-action proof; no runtime LSP method, editor-visible next-edit provider, source mirror, release action, or AI behavior",
        reachable_candidate: reachable,
        duplicate_import: duplicate,
        unreachable_module: unreachable,
        default_gate,
        explicit_gate,
        accepted_document_text,
        crlf_accepted_document_text,
        parse_stable,
        line_endings_preserved,
    };
    validate_missing_import_next_action(&receipt)?;
    Ok(receipt)
}

fn test_assertion_next_action_receipt(
    provider: &NextEditProvider,
) -> Result<TestAssertionNextActionReceipt> {
    let source = "use Test::More;\nmy $got = compute();\nmy $expected = 42;\n";
    let expected = "use Test::More;\nmy $got = compute();\nmy $expected = 42;\nis($got, $expected, 'test description');\n";
    let test_more = provider.prove_test_assertion(&TestAssertionNextEditRequest::receipt_only(
        source,
        source.len(),
        vec!["Test::More".to_string()],
        vec!["$got".to_string(), "$expected".to_string()],
    ));
    let test2_source = "use Test2::V0;\nmy $result = compute();\nmy $want = 42;\n";
    let test2 = provider.prove_test_assertion(&TestAssertionNextEditRequest::receipt_only(
        test2_source,
        test2_source.len(),
        vec!["Test2::V0".to_string()],
        vec!["$result".to_string(), "$want".to_string()],
    ));

    let mut non_test_request = TestAssertionNextEditRequest::receipt_only(
        source,
        source.len(),
        vec!["Test::More".to_string()],
        vec!["$got".to_string(), "$expected".to_string()],
    );
    non_test_request.file_role_is_test = false;
    let non_test_file = provider.prove_test_assertion(&non_test_request);

    let unsupported_source = "my $got = compute();\nmy $expected = 42;\n";
    let unsupported_framework =
        provider.prove_test_assertion(&TestAssertionNextEditRequest::receipt_only(
            unsupported_source,
            unsupported_source.len(),
            vec![],
            vec!["$got".to_string(), "$expected".to_string()],
        ));
    let missing_variables =
        provider.prove_test_assertion(&TestAssertionNextEditRequest::receipt_only(
            source,
            source.len(),
            vec!["Test::More".to_string()],
            vec!["$got".to_string()],
        ));

    let mut default_gate = TestAssertionNextEditRequest::receipt_only(
        source,
        source.len(),
        vec!["Test::More".to_string()],
        vec!["$got".to_string(), "$expected".to_string()],
    );
    default_gate.gate = NextEditFeatureGate::default();
    let default_gate = provider.prove_test_assertion(&default_gate);

    let mut explicit_gate = TestAssertionNextEditRequest::receipt_only(
        source,
        source.len(),
        vec!["Test::More".to_string()],
        vec!["$got".to_string(), "$expected".to_string()],
    );
    explicit_gate.gate = NextEditFeatureGate::explicit_enabled();
    let explicit_gate = provider.prove_test_assertion(&explicit_gate);

    let candidate = test_more
        .candidate
        .as_ref()
        .ok_or_else(|| color_eyre::eyre::eyre!("Test::More proof omitted candidate"))?;
    let accepted_document_text = candidate
        .edit
        .apply_to(source)
        .ok_or_else(|| color_eyre::eyre::eyre!("Test::More edit did not apply"))?;
    if accepted_document_text != expected {
        bail!("Test::More assertion edit produced unexpected document text");
    }
    let parse_stable = parse_succeeds(source) && parse_succeeds(&accepted_document_text);
    if !parse_stable {
        bail!("Test::More assertion edit did not preserve parse success");
    }

    let receipt = TestAssertionNextActionReceipt {
        claim_boundary: "receipt-only test assertion next-action proof; no runtime LSP method, editor-visible next-edit provider, source mirror, release action, or AI behavior",
        test_more_candidate: test_more,
        test2_candidate: test2,
        non_test_file,
        unsupported_framework,
        missing_variables,
        default_gate,
        explicit_gate,
        accepted_document_text,
        parse_stable,
    };
    validate_test_assertion_next_action(&receipt)?;
    Ok(receipt)
}

fn call_site_update_next_action_receipt(
    provider: &NextEditProvider,
) -> Result<CallSiteUpdateNextActionReceipt> {
    let source = "sub build_user ($name, $age) { }\nmy $user = build_user($name);\n";
    let expected = "sub build_user ($name, $age) { }\nmy $user = build_user($name, $age);\n";
    let cursor = source
        .find("my $user")
        .ok_or_else(|| color_eyre::eyre::eyre!("call-site fixture omitted target call"))?;
    let next_call_site_candidate = provider.prove_call_site_update(
        &CallSiteUpdateNextEditRequest::receipt_only(source, "build_user", "$age", cursor),
    );
    let duplicate_argument =
        provider.prove_call_site_update(&CallSiteUpdateNextEditRequest::receipt_only(
            "my $user = build_user($name, $age);\n",
            "build_user",
            "$age",
            0,
        ));
    let missing_call_site =
        provider.prove_call_site_update(&CallSiteUpdateNextEditRequest::receipt_only(
            "my $user = other_builder($name);\n",
            "build_user",
            "$age",
            0,
        ));
    let unsafe_call_site =
        provider.prove_call_site_update(&CallSiteUpdateNextEditRequest::receipt_only(
            "# build_user($name)\n",
            "build_user",
            "$age",
            0,
        ));
    let invalid_target =
        provider.prove_call_site_update(&CallSiteUpdateNextEditRequest::receipt_only(
            "my $user = build_user($name);\n",
            "build-user",
            "$age",
            0,
        ));
    let missing_argument =
        provider.prove_call_site_update(&CallSiteUpdateNextEditRequest::receipt_only(
            "my $user = build_user($name);\n",
            "build_user",
            "system($age)",
            0,
        ));

    let mut default_gate =
        CallSiteUpdateNextEditRequest::receipt_only(source, "build_user", "$age", cursor);
    default_gate.gate = NextEditFeatureGate::default();
    let default_gate = provider.prove_call_site_update(&default_gate);

    let mut explicit_gate =
        CallSiteUpdateNextEditRequest::receipt_only(source, "build_user", "$age", cursor);
    explicit_gate.gate = NextEditFeatureGate::explicit_enabled();
    let explicit_gate = provider.prove_call_site_update(&explicit_gate);

    let candidate = next_call_site_candidate
        .candidate
        .as_ref()
        .ok_or_else(|| color_eyre::eyre::eyre!("call-site update proof omitted candidate"))?;
    let accepted_document_text = candidate
        .edit
        .apply_to(source)
        .ok_or_else(|| color_eyre::eyre::eyre!("call-site update edit did not apply"))?;
    if accepted_document_text != expected {
        bail!("call-site update edit produced unexpected document text");
    }
    let parse_stable = parse_succeeds(source) && parse_succeeds(&accepted_document_text);
    if !parse_stable {
        bail!("call-site update edit did not preserve parse success");
    }

    let receipt = CallSiteUpdateNextActionReceipt {
        claim_boundary: "receipt-only call-site update next-action proof; no runtime LSP method, editor-visible next-edit provider, source mirror, release action, or AI behavior",
        next_call_site_candidate,
        duplicate_argument,
        missing_call_site,
        unsafe_call_site,
        invalid_target,
        missing_argument,
        default_gate,
        explicit_gate,
        accepted_document_text,
        parse_stable,
    };
    validate_call_site_update_next_action(&receipt)?;
    Ok(receipt)
}

fn rename_occurrence_next_action_receipt(
    provider: &NextEditProvider,
) -> Result<RenameOccurrenceNextActionReceipt> {
    let source = "use strict;\nmy $new = compute();\nreturn $old + $old;\n";
    let expected = "use strict;\nmy $new = compute();\nreturn $new + $old;\n";
    let cursor = source
        .find("return")
        .ok_or_else(|| color_eyre::eyre::eyre!("rename occurrence fixture omitted return"))?;
    let next_occurrence_candidate = provider.prove_rename_occurrence(
        &RenameOccurrenceNextEditRequest::receipt_only(source, "$old", "$new", cursor),
    );

    let unsafe_occurrence = provider.prove_rename_occurrence(
        &RenameOccurrenceNextEditRequest::receipt_only("# rename $old here\n", "$old", "$new", 0),
    );
    let missing_occurrence = provider.prove_rename_occurrence(
        &RenameOccurrenceNextEditRequest::receipt_only("my $new = compute();\n", "$old", "$new", 0),
    );
    let invalid_symbol = provider.prove_rename_occurrence(
        &RenameOccurrenceNextEditRequest::receipt_only("return $old;\n", "$old; system", "$new", 0),
    );

    let mut default_gate =
        RenameOccurrenceNextEditRequest::receipt_only(source, "$old", "$new", cursor);
    default_gate.gate = NextEditFeatureGate::default();
    let default_gate = provider.prove_rename_occurrence(&default_gate);

    let mut explicit_gate =
        RenameOccurrenceNextEditRequest::receipt_only(source, "$old", "$new", cursor);
    explicit_gate.gate = NextEditFeatureGate::explicit_enabled();
    let explicit_gate = provider.prove_rename_occurrence(&explicit_gate);

    let candidate = next_occurrence_candidate
        .candidate
        .as_ref()
        .ok_or_else(|| color_eyre::eyre::eyre!("rename occurrence proof omitted candidate"))?;
    let accepted_document_text = candidate
        .edit
        .apply_to(source)
        .ok_or_else(|| color_eyre::eyre::eyre!("rename occurrence edit did not apply"))?;
    if accepted_document_text != expected {
        bail!("rename occurrence edit produced unexpected document text");
    }
    let parse_stable = parse_succeeds(source) && parse_succeeds(&accepted_document_text);
    if !parse_stable {
        bail!("rename occurrence edit did not preserve parse success");
    }

    let receipt = RenameOccurrenceNextActionReceipt {
        claim_boundary: "receipt-only rename-occurrence next-action proof; no runtime LSP method, editor-visible next-edit provider, source mirror, release action, or AI behavior",
        next_occurrence_candidate,
        unsafe_occurrence,
        missing_occurrence,
        invalid_symbol,
        default_gate,
        explicit_gate,
        accepted_document_text,
        parse_stable,
    };
    validate_rename_occurrence_next_action(&receipt)?;
    Ok(receipt)
}

fn optional_ai_candidate_boundary_receipt(
    default_response: &NextEditResponse,
    receipt_only_response: &NextEditResponse,
    explicit_gate_response: &NextEditResponse,
    base_policy: NextEditSafetyPolicy,
) -> Result<OptionalAiCandidateBoundaryReceipt> {
    let mut ai_enabled_policy = base_policy;
    ai_enabled_policy.ai_source_enabled = true;

    let mut missing_range_policy = base_policy;
    missing_range_policy.requires_editor_safe_range = false;

    let mut missing_parse_policy = base_policy;
    missing_parse_policy.requires_parse_safety = false;

    let mut missing_selected_completion_policy = base_policy;
    missing_selected_completion_policy.requires_selected_completion_compatibility = false;

    let mut nondeterministic_policy = base_policy;
    nondeterministic_policy.deterministic_sources_only = false;

    let receipt = OptionalAiCandidateBoundaryReceipt {
        claim_boundary: "optional AI candidate boundary proof only; no AI provider, prompt path, network call, editor-visible next-edit suggestion, source mirror, or release action",
        enabled_by_default: false,
        ai_candidate_source_enabled: base_policy.ai_source_enabled,
        default_response_suggestions_empty: default_response.suggestions.is_empty(),
        receipt_only_response_suggestions_empty: receipt_only_response.suggestions.is_empty(),
        explicit_gate_response_suggestions_empty: explicit_gate_response.suggestions.is_empty(),
        rejects_ai_enabled_policy: validate_scaffold_responses(
            default_response,
            receipt_only_response,
            explicit_gate_response,
            &ai_enabled_policy,
        )
        .is_err(),
        rejects_missing_editor_safe_range: validate_scaffold_responses(
            default_response,
            receipt_only_response,
            explicit_gate_response,
            &missing_range_policy,
        )
        .is_err(),
        rejects_missing_parse_safety: validate_scaffold_responses(
            default_response,
            receipt_only_response,
            explicit_gate_response,
            &missing_parse_policy,
        )
        .is_err(),
        rejects_missing_selected_completion_compatibility: validate_scaffold_responses(
            default_response,
            receipt_only_response,
            explicit_gate_response,
            &missing_selected_completion_policy,
        )
        .is_err(),
        rejects_nondeterministic_sources: validate_scaffold_responses(
            default_response,
            receipt_only_response,
            explicit_gate_response,
            &nondeterministic_policy,
        )
        .is_err(),
        deterministic_sources_only: base_policy.deterministic_sources_only,
    };
    validate_optional_ai_candidate_boundary(&receipt)?;
    Ok(receipt)
}

fn validate_optional_ai_candidate_boundary(
    receipt: &OptionalAiCandidateBoundaryReceipt,
) -> Result<()> {
    if receipt.enabled_by_default || receipt.ai_candidate_source_enabled {
        bail!("optional AI candidate source must remain disabled by default");
    }
    if !receipt.default_response_suggestions_empty
        || !receipt.receipt_only_response_suggestions_empty
        || !receipt.explicit_gate_response_suggestions_empty
    {
        bail!("optional AI boundary must not emit next-edit suggestions");
    }
    if !receipt.rejects_ai_enabled_policy {
        bail!("optional AI boundary must reject AI-enabled policy drift");
    }
    if !receipt.rejects_missing_editor_safe_range {
        bail!("optional AI boundary must reject missing editor-safe range policy");
    }
    if !receipt.rejects_missing_parse_safety {
        bail!("optional AI boundary must reject missing parse-safety policy");
    }
    if !receipt.rejects_missing_selected_completion_compatibility {
        bail!("optional AI boundary must reject missing selected-completion policy");
    }
    if !receipt.rejects_nondeterministic_sources || !receipt.deterministic_sources_only {
        bail!("optional AI boundary must keep deterministic sources first");
    }
    Ok(())
}

fn validate_rename_occurrence_next_action(
    receipt: &RenameOccurrenceNextActionReceipt,
) -> Result<()> {
    let Some(candidate) = receipt.next_occurrence_candidate.candidate.as_ref() else {
        bail!("rename occurrence proof must prepare a receipt-only candidate");
    };
    if receipt.next_occurrence_candidate.status != NextEditStatus::ReceiptOnly
        || candidate.family != NextEditCandidateFamily::RenameOccurrence
        || candidate.original_symbol != "$old"
        || candidate.replacement_symbol != "$new"
        || candidate.editor_visible
        || candidate.edit.new_text != "$new"
        || !receipt.next_occurrence_candidate.rejection_reasons.is_empty()
    {
        bail!("rename occurrence proof did not satisfy the receipt-only contract");
    }
    if receipt.unsafe_occurrence.candidate.is_some()
        || !receipt
            .unsafe_occurrence
            .rejection_reasons
            .contains(&NextEditRejectionReason::UnsafeInsertionPoint)
    {
        bail!("rename occurrence proof must reject unsafe next occurrences");
    }
    if receipt.missing_occurrence.candidate.is_some()
        || !receipt
            .missing_occurrence
            .rejection_reasons
            .contains(&NextEditRejectionReason::MissingRenameOccurrence)
    {
        bail!("rename occurrence proof must reject missing next occurrences");
    }
    if receipt.invalid_symbol.candidate.is_some()
        || !receipt
            .invalid_symbol
            .rejection_reasons
            .contains(&NextEditRejectionReason::InvalidRenameSymbol)
    {
        bail!("rename occurrence proof must reject invalid symbols");
    }
    if receipt.default_gate.status != NextEditStatus::Disabled
        || receipt.default_gate.candidate.is_some()
        || !receipt.default_gate.rejection_reasons.contains(&NextEditRejectionReason::GateDisabled)
    {
        bail!("rename occurrence next action must remain disabled by default");
    }
    if receipt.explicit_gate.status != NextEditStatus::RuntimeProviderNotRegistered
        || receipt.explicit_gate.candidate.is_some()
        || !receipt
            .explicit_gate
            .rejection_reasons
            .contains(&NextEditRejectionReason::RuntimeProviderNotRegistered)
    {
        bail!("rename occurrence next action must not bypass the unregistered runtime provider");
    }
    if !receipt.parse_stable {
        bail!("rename occurrence next action must keep local parse state stable");
    }
    Ok(())
}

fn validate_call_site_update_next_action(receipt: &CallSiteUpdateNextActionReceipt) -> Result<()> {
    let Some(candidate) = receipt.next_call_site_candidate.candidate.as_ref() else {
        bail!("call-site update proof must prepare a receipt-only candidate");
    };
    if receipt.next_call_site_candidate.status != NextEditStatus::ReceiptOnly
        || candidate.family != NextEditCandidateFamily::CallSiteUpdate
        || candidate.callee_name != "build_user"
        || candidate.argument != "$age"
        || candidate.editor_visible
        || candidate.edit.new_text != ", $age"
        || !receipt.next_call_site_candidate.rejection_reasons.is_empty()
    {
        bail!("call-site update proof did not satisfy the receipt-only contract");
    }
    if receipt.duplicate_argument.candidate.is_some()
        || !receipt
            .duplicate_argument
            .rejection_reasons
            .contains(&NextEditRejectionReason::DuplicateCallArgument)
    {
        bail!("call-site update proof must reject duplicate arguments");
    }
    if receipt.missing_call_site.candidate.is_some()
        || !receipt
            .missing_call_site
            .rejection_reasons
            .contains(&NextEditRejectionReason::MissingCallSite)
    {
        bail!("call-site update proof must reject missing call sites");
    }
    if receipt.unsafe_call_site.candidate.is_some()
        || !receipt
            .unsafe_call_site
            .rejection_reasons
            .contains(&NextEditRejectionReason::UnsafeInsertionPoint)
    {
        bail!("call-site update proof must reject unsafe call sites");
    }
    if receipt.invalid_target.candidate.is_some()
        || !receipt
            .invalid_target
            .rejection_reasons
            .contains(&NextEditRejectionReason::InvalidCallTarget)
    {
        bail!("call-site update proof must reject invalid call targets");
    }
    if receipt.missing_argument.candidate.is_some()
        || !receipt
            .missing_argument
            .rejection_reasons
            .contains(&NextEditRejectionReason::MissingCallArgument)
    {
        bail!("call-site update proof must reject missing call arguments");
    }
    if receipt.default_gate.status != NextEditStatus::Disabled
        || receipt.default_gate.candidate.is_some()
        || !receipt.default_gate.rejection_reasons.contains(&NextEditRejectionReason::GateDisabled)
    {
        bail!("call-site update next action must remain disabled by default");
    }
    if receipt.explicit_gate.status != NextEditStatus::RuntimeProviderNotRegistered
        || receipt.explicit_gate.candidate.is_some()
        || !receipt
            .explicit_gate
            .rejection_reasons
            .contains(&NextEditRejectionReason::RuntimeProviderNotRegistered)
    {
        bail!("call-site update next action must not bypass the unregistered runtime provider");
    }
    if !receipt.parse_stable {
        bail!("call-site update next action must keep local parse state stable");
    }
    Ok(())
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
    if !receipt.line_endings_preserved {
        bail!("missing-import next action must preserve document line endings");
    }
    Ok(())
}

fn validate_test_assertion_next_action(receipt: &TestAssertionNextActionReceipt) -> Result<()> {
    let Some(test_more_candidate) = receipt.test_more_candidate.candidate.as_ref() else {
        bail!("Test::More assertion proof must prepare a receipt-only candidate");
    };
    if receipt.test_more_candidate.status != NextEditStatus::ReceiptOnly
        || test_more_candidate.family != NextEditCandidateFamily::TestAssertionBody
        || test_more_candidate.editor_visible
        || test_more_candidate.edit.new_text != "is($got, $expected, 'test description');\n"
        || !receipt.test_more_candidate.rejection_reasons.is_empty()
    {
        bail!("Test::More assertion proof did not satisfy the receipt-only contract");
    }

    let Some(test2_candidate) = receipt.test2_candidate.candidate.as_ref() else {
        bail!("Test2 assertion proof must prepare a receipt-only candidate");
    };
    if receipt.test2_candidate.status != NextEditStatus::ReceiptOnly
        || test2_candidate.family != NextEditCandidateFamily::TestAssertionBody
        || test2_candidate.editor_visible
        || test2_candidate.edit.new_text != "is($result, $want, 'test description');\n"
        || !receipt.test2_candidate.rejection_reasons.is_empty()
    {
        bail!("Test2 assertion proof did not satisfy the receipt-only contract");
    }

    if receipt.non_test_file.candidate.is_some()
        || !receipt
            .non_test_file
            .rejection_reasons
            .contains(&NextEditRejectionReason::TestFileRequired)
    {
        bail!("test assertion proof must reject non-test files");
    }
    if receipt.unsupported_framework.candidate.is_some()
        || !receipt
            .unsupported_framework
            .rejection_reasons
            .contains(&NextEditRejectionReason::UnsupportedTestFramework)
    {
        bail!("test assertion proof must reject unsupported test frameworks");
    }
    if receipt.missing_variables.candidate.is_some()
        || !receipt
            .missing_variables
            .rejection_reasons
            .contains(&NextEditRejectionReason::MissingAssertionVariables)
    {
        bail!("test assertion proof must reject missing assertion variables");
    }
    if receipt.default_gate.status != NextEditStatus::Disabled
        || receipt.default_gate.candidate.is_some()
        || !receipt.default_gate.rejection_reasons.contains(&NextEditRejectionReason::GateDisabled)
    {
        bail!("test assertion next action must remain disabled by default");
    }
    if receipt.explicit_gate.status != NextEditStatus::RuntimeProviderNotRegistered
        || receipt.explicit_gate.candidate.is_some()
        || !receipt
            .explicit_gate
            .rejection_reasons
            .contains(&NextEditRejectionReason::RuntimeProviderNotRegistered)
    {
        bail!("test assertion next action must not bypass the unregistered runtime provider");
    }
    if !receipt.parse_stable {
        bail!("test assertion next action must keep local parse state stable");
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
    use perl_tdd_support::{must_err, must_some};
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
        assert_eq!(
            value
                .pointer("/missing_import_next_action/line_endings_preserved")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert!(
            value
                .pointer("/missing_import_next_action/crlf_accepted_document_text")
                .and_then(Value::as_str)
                .is_some_and(|text| text.contains("use My::App;\r\nmy $value"))
        );
        assert_eq!(
            value
                .pointer("/test_assertion_next_action/test_more_candidate/status")
                .and_then(Value::as_str),
            Some("receipt_only")
        );
        assert_eq!(
            value
                .pointer("/test_assertion_next_action/test_more_candidate/candidate/family")
                .and_then(Value::as_str),
            Some("test_assertion_body")
        );
        assert_eq!(
            value
                .pointer("/test_assertion_next_action/test_more_candidate/candidate/editorVisible")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            value
                .pointer("/test_assertion_next_action/test2_candidate/candidate/framework")
                .and_then(Value::as_str),
            Some("test2_v0")
        );
        assert_eq!(
            value
                .pointer("/test_assertion_next_action/non_test_file/rejectionReasons/0")
                .and_then(Value::as_str),
            Some("test_file_required")
        );
        assert_eq!(
            value
                .pointer("/test_assertion_next_action/unsupported_framework/rejectionReasons/0")
                .and_then(Value::as_str),
            Some("unsupported_test_framework")
        );
        assert_eq!(
            value
                .pointer("/test_assertion_next_action/missing_variables/rejectionReasons/0")
                .and_then(Value::as_str),
            Some("missing_assertion_variables")
        );
        assert_eq!(
            value.pointer("/test_assertion_next_action/parse_stable").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            value
                .pointer("/optional_ai_candidate_boundary/enabled_by_default")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            value
                .pointer("/optional_ai_candidate_boundary/ai_candidate_source_enabled")
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            value
                .pointer("/optional_ai_candidate_boundary/rejects_ai_enabled_policy")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            value
                .pointer("/optional_ai_candidate_boundary/rejects_missing_parse_safety")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert!(
            !value
                .pointer("/test_assertion_next_action/accepted_document_text")
                .and_then(Value::as_str)
                .is_some_and(|text| text.contains("done_testing"))
        );

        Ok(())
    }

    fn assert_optional_ai_boundary_rejects(
        mut receipt: OptionalAiCandidateBoundaryReceipt,
        mutate: impl FnOnce(&mut OptionalAiCandidateBoundaryReceipt),
        expected: &str,
    ) {
        mutate(&mut receipt);
        let error = must_err(validate_optional_ai_candidate_boundary(&receipt));
        assert!(
            error.to_string().contains(expected),
            "error should contain `{expected}`, got {error}"
        );
    }

    #[test]
    fn semantic_inline_next_edit_optional_ai_candidate_boundary_rejects_safety_drift() -> Result<()>
    {
        let response = NextEditResponse::new(NextEditStatus::Disabled, vec![]);
        let receipt_only = NextEditResponse::new(NextEditStatus::ReceiptOnly, vec![]);
        let explicit_gate =
            NextEditResponse::new(NextEditStatus::RuntimeProviderNotRegistered, vec![]);

        let receipt = optional_ai_candidate_boundary_receipt(
            &response,
            &receipt_only,
            &explicit_gate,
            NextEditSafetyPolicy::default(),
        )?;
        assert!(!receipt.enabled_by_default);
        assert!(!receipt.ai_candidate_source_enabled);
        assert!(receipt.rejects_ai_enabled_policy);
        assert!(receipt.rejects_missing_editor_safe_range);
        assert!(receipt.rejects_missing_parse_safety);
        assert!(receipt.rejects_missing_selected_completion_compatibility);
        assert!(receipt.rejects_nondeterministic_sources);
        assert!(receipt.deterministic_sources_only);

        assert_optional_ai_boundary_rejects(
            receipt.clone(),
            |drift| drift.enabled_by_default = true,
            "disabled by default",
        );
        assert_optional_ai_boundary_rejects(
            receipt.clone(),
            |drift| drift.ai_candidate_source_enabled = true,
            "disabled by default",
        );
        assert_optional_ai_boundary_rejects(
            receipt.clone(),
            |drift| drift.default_response_suggestions_empty = false,
            "must not emit",
        );
        assert_optional_ai_boundary_rejects(
            receipt.clone(),
            |drift| drift.receipt_only_response_suggestions_empty = false,
            "must not emit",
        );
        assert_optional_ai_boundary_rejects(
            receipt.clone(),
            |drift| drift.explicit_gate_response_suggestions_empty = false,
            "must not emit",
        );
        assert_optional_ai_boundary_rejects(
            receipt.clone(),
            |drift| drift.rejects_ai_enabled_policy = false,
            "AI-enabled policy drift",
        );
        assert_optional_ai_boundary_rejects(
            receipt.clone(),
            |drift| drift.rejects_missing_editor_safe_range = false,
            "editor-safe range",
        );
        assert_optional_ai_boundary_rejects(
            receipt.clone(),
            |drift| drift.rejects_missing_parse_safety = false,
            "parse-safety",
        );
        assert_optional_ai_boundary_rejects(
            receipt.clone(),
            |drift| drift.rejects_missing_selected_completion_compatibility = false,
            "selected-completion",
        );
        assert_optional_ai_boundary_rejects(
            receipt.clone(),
            |drift| drift.rejects_nondeterministic_sources = false,
            "deterministic sources",
        );
        assert_optional_ai_boundary_rejects(
            receipt,
            |drift| drift.deterministic_sources_only = false,
            "deterministic sources",
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

        let mut receipt = missing_import_next_action_receipt(&provider)?;
        receipt.line_endings_preserved = false;
        let error = validate_missing_import_next_action(&receipt)
            .expect_err("line-ending drift must fail validation");
        assert!(
            error.to_string().contains("line endings"),
            "error should identify line-ending drift, got {error}"
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
    fn test_assertion_next_action_validation_rejects_contract_drift() -> Result<()> {
        let provider = NextEditProvider;
        let mut receipt = test_assertion_next_action_receipt(&provider)?;
        receipt.test_more_candidate.candidate = None;
        let error = must_err(validate_test_assertion_next_action(&receipt));
        assert!(
            error.to_string().contains("Test::More assertion proof"),
            "error should identify missing Test::More candidate, got {error}"
        );

        let mut receipt = test_assertion_next_action_receipt(&provider)?;
        receipt.test2_candidate.candidate = None;
        let error = must_err(validate_test_assertion_next_action(&receipt));
        assert!(
            error.to_string().contains("Test2 assertion proof"),
            "error should identify missing Test2 candidate, got {error}"
        );

        let mut receipt = test_assertion_next_action_receipt(&provider)?;
        let candidate = must_some(receipt.test_more_candidate.candidate.as_mut());
        candidate.editor_visible = true;
        let error = must_err(validate_test_assertion_next_action(&receipt));
        assert!(
            error.to_string().contains("receipt-only contract"),
            "error should identify Test::More candidate contract drift, got {error}"
        );

        let mut receipt = test_assertion_next_action_receipt(&provider)?;
        let candidate = must_some(receipt.test2_candidate.candidate.as_mut());
        candidate.editor_visible = true;
        let error = must_err(validate_test_assertion_next_action(&receipt));
        assert!(
            error.to_string().contains("receipt-only contract"),
            "error should identify Test2 candidate contract drift, got {error}"
        );

        let mut receipt = test_assertion_next_action_receipt(&provider)?;
        receipt.parse_stable = false;
        let error = must_err(validate_test_assertion_next_action(&receipt));
        assert!(
            error.to_string().contains("parse state stable"),
            "error should identify parse-stability drift, got {error}"
        );

        Ok(())
    }

    #[test]
    fn test_assertion_next_action_validation_rejects_rejection_drift() -> Result<()> {
        let provider = NextEditProvider;
        let mut receipt = test_assertion_next_action_receipt(&provider)?;
        receipt.non_test_file.rejection_reasons.clear();
        let error = must_err(validate_test_assertion_next_action(&receipt));
        assert!(
            error.to_string().contains("reject non-test files"),
            "error should identify non-test rejection drift, got {error}"
        );

        let mut receipt = test_assertion_next_action_receipt(&provider)?;
        receipt.unsupported_framework.rejection_reasons.clear();
        let error = must_err(validate_test_assertion_next_action(&receipt));
        assert!(
            error.to_string().contains("reject unsupported test frameworks"),
            "error should identify framework rejection drift, got {error}"
        );

        let mut receipt = test_assertion_next_action_receipt(&provider)?;
        receipt.missing_variables.rejection_reasons.clear();
        let error = must_err(validate_test_assertion_next_action(&receipt));
        assert!(
            error.to_string().contains("reject missing assertion variables"),
            "error should identify missing-variable rejection drift, got {error}"
        );

        let mut receipt = test_assertion_next_action_receipt(&provider)?;
        receipt.default_gate.rejection_reasons.clear();
        let error = must_err(validate_test_assertion_next_action(&receipt));
        assert!(
            error.to_string().contains("disabled by default"),
            "error should identify default-gate drift, got {error}"
        );

        let mut receipt = test_assertion_next_action_receipt(&provider)?;
        receipt.explicit_gate.rejection_reasons.clear();
        let error = must_err(validate_test_assertion_next_action(&receipt));
        assert!(
            error.to_string().contains("unregistered runtime provider"),
            "error should identify explicit-gate drift, got {error}"
        );

        Ok(())
    }

    #[test]
    fn rename_occurrence_next_action_validation_rejects_contract_drift() -> Result<()> {
        let provider = NextEditProvider;
        let mut receipt = rename_occurrence_next_action_receipt(&provider)?;
        receipt.next_occurrence_candidate.candidate = None;
        let error = must_err(validate_rename_occurrence_next_action(&receipt));
        assert!(
            error.to_string().contains("receipt-only candidate"),
            "error should identify missing rename candidate, got {error}"
        );

        let mut receipt = rename_occurrence_next_action_receipt(&provider)?;
        let candidate = must_some(receipt.next_occurrence_candidate.candidate.as_mut());
        candidate.editor_visible = true;
        let error = must_err(validate_rename_occurrence_next_action(&receipt));
        assert!(
            error.to_string().contains("receipt-only contract"),
            "error should identify rename candidate contract drift, got {error}"
        );

        let mut receipt = rename_occurrence_next_action_receipt(&provider)?;
        receipt.parse_stable = false;
        let error = must_err(validate_rename_occurrence_next_action(&receipt));
        assert!(
            error.to_string().contains("parse state stable"),
            "error should identify parse-stability drift, got {error}"
        );

        Ok(())
    }

    #[test]
    fn rename_occurrence_next_action_validation_rejects_rejection_drift() -> Result<()> {
        let provider = NextEditProvider;
        let mut receipt = rename_occurrence_next_action_receipt(&provider)?;
        receipt.unsafe_occurrence.rejection_reasons.clear();
        let error = must_err(validate_rename_occurrence_next_action(&receipt));
        assert!(
            error.to_string().contains("reject unsafe next occurrences"),
            "error should identify unsafe-occurrence rejection drift, got {error}"
        );

        let mut receipt = rename_occurrence_next_action_receipt(&provider)?;
        receipt.missing_occurrence.rejection_reasons.clear();
        let error = must_err(validate_rename_occurrence_next_action(&receipt));
        assert!(
            error.to_string().contains("reject missing next occurrences"),
            "error should identify missing-occurrence rejection drift, got {error}"
        );

        let mut receipt = rename_occurrence_next_action_receipt(&provider)?;
        receipt.invalid_symbol.rejection_reasons.clear();
        let error = must_err(validate_rename_occurrence_next_action(&receipt));
        assert!(
            error.to_string().contains("reject invalid symbols"),
            "error should identify invalid-symbol rejection drift, got {error}"
        );

        let mut receipt = rename_occurrence_next_action_receipt(&provider)?;
        receipt.default_gate.rejection_reasons.clear();
        let error = must_err(validate_rename_occurrence_next_action(&receipt));
        assert!(
            error.to_string().contains("disabled by default"),
            "error should identify default-gate drift, got {error}"
        );

        let mut receipt = rename_occurrence_next_action_receipt(&provider)?;
        receipt.explicit_gate.rejection_reasons.clear();
        let error = must_err(validate_rename_occurrence_next_action(&receipt));
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
