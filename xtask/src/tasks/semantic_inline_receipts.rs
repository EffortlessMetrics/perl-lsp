use color_eyre::eyre::{Context, Result, bail, eyre};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const MATRIX_PATH: &str = "crates/perl-lsp-ux-tests/fixtures/editor_ux_fixture_matrix.json";

const REQUIRED_SEMANTIC_INLINE_RECEIPTS: &[SemanticInlineRequirement] = &[
    SemanticInlineRequirement {
        capability: "mojolicious_project_smoke",
        workflow_id: "mojolicious_inline_completion_quality",
    },
    SemanticInlineRequirement {
        capability: "selected_completion",
        workflow_id: "mojolicious_inline_completion_quality",
    },
    SemanticInlineRequirement {
        capability: "test_assertion",
        workflow_id: "test_inline_completion_quality",
    },
    SemanticInlineRequirement {
        capability: "constructor_style",
        workflow_id: "constructor_inline_completion_quality",
    },
    SemanticInlineRequirement {
        capability: "self_receiver",
        workflow_id: "self_receiver_inline_completion_quality",
    },
    SemanticInlineRequirement {
        capability: "dbi_receiver",
        workflow_id: "dbi_receiver_inline_completion_quality",
    },
    SemanticInlineRequirement {
        capability: "lexical_return",
        workflow_id: "lexical_return_inline_completion_quality",
    },
    SemanticInlineRequirement {
        capability: "loop_binding",
        workflow_id: "loop_binding_inline_completion_quality",
    },
    SemanticInlineRequirement {
        capability: "guard_condition",
        workflow_id: "guard_condition_inline_completion_quality",
    },
    SemanticInlineRequirement {
        capability: "project_module_import",
        workflow_id: "real_workspace_module_import_inline_completion_quality",
    },
    SemanticInlineRequirement {
        capability: "package_boundary_receiver",
        workflow_id: "package_boundary_receiver_inline_completion_quality",
    },
    SemanticInlineRequirement {
        capability: "gated_multiline_constructor",
        workflow_id: "gated_multiline_constructor_inline_completion_quality",
    },
];

#[derive(Debug, Clone, Copy)]
struct SemanticInlineRequirement {
    capability: &'static str,
    workflow_id: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct SemanticInlineReceipt {
    schema_version: &'static str,
    provider: &'static str,
    provider_action: &'static str,
    claim_boundary: &'static str,
    matrix_path: &'static str,
    workflow_count: usize,
    required_capability_count: usize,
    all_required_capabilities_registered: bool,
    semantic_inline: BTreeMap<&'static str, SemanticInlineCapabilityReceipt>,
    quality_counters: InlineQualityCounterSummary,
    next_edit_scaffold: NextEditScaffoldSummary,
    future_gated: BTreeMap<&'static str, &'static str>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct SemanticInlineCapabilityReceipt {
    status: &'static str,
    workflow_id: &'static str,
    scenario_file: String,
    user_journey: String,
    expected_outcomes: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct InlineQualityCounterSummary {
    source: String,
    available: bool,
    all_checks_green: Option<bool>,
    fixtures_total: Option<u64>,
    fixtures_passed: Option<u64>,
    edit_application: Option<QualityCountSummary>,
    hard_zone_rejections: Option<u64>,
    suppression_reasons: Option<BTreeMap<String, u64>>,
    parse_regressions: Option<u64>,
    sources: Option<BTreeMap<String, SourceQualityCounterSummary>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct SourceQualityCounterSummary {
    expected: u64,
    passed: u64,
    failed: u64,
    returned_items: u64,
    edit_application: QualityCountSummary,
    parse_regressions: u64,
    suppression_reasons: BTreeMap<String, u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct QualityCountSummary {
    total: u64,
    passed: u64,
    failed: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct NextEditScaffoldSummary {
    source: String,
    available: bool,
    schema_version: Option<String>,
    provider_action: Option<String>,
    enabled_by_default: Option<bool>,
    runtime_provider_registered: Option<bool>,
    ai_candidate_source_enabled: Option<bool>,
    default_status: Option<String>,
    receipt_only_status: Option<String>,
    explicit_gate_status: Option<String>,
    planned_candidate_families: Option<Vec<String>>,
    future_gated: Option<Vec<String>>,
    missing_import_next_action: Option<MissingImportNextActionSummary>,
    test_assertion_next_action: Option<TestAssertionNextActionSummary>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct MissingImportNextActionSummary {
    reachable_candidate_prepared: bool,
    reachable_candidate_editor_visible: bool,
    duplicate_import_rejected: bool,
    unreachable_module_rejected: bool,
    default_gate_disabled: bool,
    explicit_gate_runtime_unregistered: bool,
    parse_stable: bool,
    line_endings_preserved: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct TestAssertionNextActionSummary {
    test_more_candidate_prepared: bool,
    test_more_candidate_editor_visible: bool,
    test2_candidate_prepared: bool,
    test2_candidate_editor_visible: bool,
    non_test_file_rejected: bool,
    unsupported_framework_rejected: bool,
    missing_variables_rejected: bool,
    default_gate_disabled: bool,
    explicit_gate_runtime_unregistered: bool,
    parse_stable: bool,
}

pub fn run(receipt: PathBuf, quality_receipt: PathBuf, next_edit_receipt: PathBuf) -> Result<()> {
    let root = crate::utils::project_root()?;
    let matrix_path = root.join(MATRIX_PATH);
    let matrix = read_json(&matrix_path)?;
    let quality = read_optional_quality_counter_summary(&root.join(&quality_receipt))?;
    let next_edit_scaffold =
        read_optional_next_edit_scaffold_summary(&root.join(&next_edit_receipt))?;
    let receipt_data = summarize_matrix(&matrix, MATRIX_PATH, quality, next_edit_scaffold)?;

    write_receipt(&receipt, &receipt_data)?;
    println!(
        "semantic inline receipt dashboard OK: {} capabilities, {} workflows, {}",
        receipt_data.required_capability_count,
        receipt_data.workflow_count,
        receipt.display()
    );
    Ok(())
}

fn read_json(path: &Path) -> Result<Value> {
    let raw = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("parsing {}", path.display()))
}

fn read_optional_quality_counter_summary(path: &Path) -> Result<InlineQualityCounterSummary> {
    let source = path.display().to_string();
    if !path.exists() {
        return Ok(InlineQualityCounterSummary {
            source,
            available: false,
            all_checks_green: None,
            fixtures_total: None,
            fixtures_passed: None,
            edit_application: None,
            hard_zone_rejections: None,
            suppression_reasons: None,
            parse_regressions: None,
            sources: None,
        });
    }

    let quality = read_json(path)?;
    let suppression_reasons = quality_counter_map(&quality, "/checks/suppression_reasons")?;
    let edit_application = quality_count_summary(&quality, "/checks/edit_application")?;
    let sources = quality_source_summaries(&quality)?;
    let summary = InlineQualityCounterSummary {
        source,
        available: true,
        all_checks_green: Some(true),
        fixtures_total: quality.get("fixtures_total").and_then(Value::as_u64),
        fixtures_passed: quality.get("fixtures_passed").and_then(Value::as_u64),
        edit_application,
        hard_zone_rejections: quality.pointer("/checks/hard_zone_rejected").and_then(Value::as_u64),
        suppression_reasons,
        parse_regressions: quality.pointer("/checks/parse_regressions").and_then(Value::as_u64),
        sources,
    };
    validate_quality_counter_summary(&summary)?;
    Ok(summary)
}

fn read_optional_next_edit_scaffold_summary(path: &Path) -> Result<NextEditScaffoldSummary> {
    let source = path.display().to_string();
    if !path.exists() {
        return Ok(NextEditScaffoldSummary {
            source,
            available: false,
            schema_version: None,
            provider_action: None,
            enabled_by_default: None,
            runtime_provider_registered: None,
            ai_candidate_source_enabled: None,
            default_status: None,
            receipt_only_status: None,
            explicit_gate_status: None,
            planned_candidate_families: None,
            future_gated: None,
            missing_import_next_action: None,
            test_assertion_next_action: None,
        });
    }

    let scaffold = read_json(path)?;
    let summary = NextEditScaffoldSummary {
        source,
        available: true,
        schema_version: scaffold
            .get("schema_version")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        provider_action: scaffold
            .get("provider_action")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        enabled_by_default: scaffold.get("enabled_by_default").and_then(Value::as_bool),
        runtime_provider_registered: scaffold
            .get("runtime_provider_registered")
            .and_then(Value::as_bool),
        ai_candidate_source_enabled: scaffold
            .get("ai_candidate_source_enabled")
            .and_then(Value::as_bool),
        default_status: scaffold
            .pointer("/default_response/status")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        receipt_only_status: scaffold
            .pointer("/receipt_only_response/status")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        explicit_gate_status: scaffold
            .pointer("/explicit_gate_response/status")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        planned_candidate_families: string_array_field(&scaffold, "planned_candidate_families")?,
        future_gated: string_array_field(&scaffold, "future_gated")?,
        missing_import_next_action: Some(missing_import_next_action_summary(&scaffold)?),
        test_assertion_next_action: Some(test_assertion_next_action_summary(&scaffold)?),
    };
    validate_next_edit_scaffold_summary(&summary, &scaffold)?;
    Ok(summary)
}

fn string_array_field(value: &Value, field: &str) -> Result<Option<Vec<String>>> {
    let Some(items) = value.get(field) else {
        return Ok(None);
    };
    let items = items
        .as_array()
        .ok_or_else(|| eyre!("next-edit scaffold receipt `{field}` must be an array"))?;
    let mut result = Vec::with_capacity(items.len());
    for item in items {
        result.push(
            item.as_str()
                .ok_or_else(|| {
                    eyre!("next-edit scaffold receipt `{field}` entries must be strings")
                })?
                .to_string(),
        );
    }
    Ok(Some(result))
}

fn validate_next_edit_scaffold_summary(
    summary: &NextEditScaffoldSummary,
    scaffold: &Value,
) -> Result<()> {
    if !summary.available {
        return Ok(());
    }

    require_next_edit_value(
        summary.schema_version.as_deref(),
        "schema_version",
        "semantic-inline-next-edit.v1",
    )?;
    require_next_edit_value(
        summary.provider_action.as_deref(),
        "provider_action",
        "next_edit_scaffold",
    )?;
    require_next_edit_bool(summary.enabled_by_default, "enabled_by_default", false)?;
    require_next_edit_bool(
        summary.runtime_provider_registered,
        "runtime_provider_registered",
        false,
    )?;
    require_next_edit_bool(
        summary.ai_candidate_source_enabled,
        "ai_candidate_source_enabled",
        false,
    )?;
    require_next_edit_value(
        summary.default_status.as_deref(),
        "default_response/status",
        "disabled",
    )?;
    require_next_edit_value(
        summary.receipt_only_status.as_deref(),
        "receipt_only_response/status",
        "receipt_only",
    )?;
    require_next_edit_value(
        summary.explicit_gate_status.as_deref(),
        "explicit_gate_response/status",
        "runtime_provider_not_registered",
    )?;
    require_empty_suggestions(scaffold, "/default_response/suggestions")?;
    require_empty_suggestions(scaffold, "/receipt_only_response/suggestions")?;
    require_empty_suggestions(scaffold, "/explicit_gate_response/suggestions")?;

    let planned = summary
        .planned_candidate_families
        .as_ref()
        .ok_or_else(|| eyre!("next-edit scaffold receipt missing planned_candidate_families"))?;
    for required in
        ["missing_import", "test_assertion_body", "call_site_update", "rename_occurrence"]
    {
        if !planned.iter().any(|family| family == required) {
            bail!("next-edit scaffold receipt missing planned family `{required}`");
        }
    }

    let future_gated = summary
        .future_gated
        .as_ref()
        .ok_or_else(|| eyre!("next-edit scaffold receipt missing future_gated list"))?;
    for required in [
        "runtime_next_edit_provider",
        "editor_visible_next_edit_suggestions",
        "missing_import_next_action",
        "test_assertion_next_action",
        "optional_ai_candidate_source",
    ] {
        if !future_gated.iter().any(|entry| entry == required) {
            bail!("next-edit scaffold receipt missing future-gated item `{required}`");
        }
    }
    require_missing_import_next_action(scaffold)?;
    require_test_assertion_next_action(scaffold)?;

    Ok(())
}

fn missing_import_next_action_summary(scaffold: &Value) -> Result<MissingImportNextActionSummary> {
    let action = scaffold
        .get("missing_import_next_action")
        .ok_or_else(|| eyre!("next-edit scaffold receipt missing missing_import_next_action"))?;
    let reachable_candidate = action.pointer("/reachable_candidate/candidate");
    Ok(MissingImportNextActionSummary {
        reachable_candidate_prepared: reachable_candidate.is_some(),
        reachable_candidate_editor_visible: reachable_candidate
            .and_then(|candidate| candidate.get("editorVisible"))
            .and_then(Value::as_bool)
            .unwrap_or(true),
        duplicate_import_rejected: rejection_reason_present(
            action,
            "/duplicate_import/rejectionReasons",
            "duplicate_import",
        )?,
        unreachable_module_rejected: rejection_reason_present(
            action,
            "/unreachable_module/rejectionReasons",
            "unreachable_module",
        )?,
        default_gate_disabled: action.pointer("/default_gate/status").and_then(Value::as_str)
            == Some("disabled"),
        explicit_gate_runtime_unregistered: action
            .pointer("/explicit_gate/status")
            .and_then(Value::as_str)
            == Some("runtime_provider_not_registered"),
        parse_stable: action.get("parse_stable").and_then(Value::as_bool).unwrap_or(false),
        line_endings_preserved: action
            .get("line_endings_preserved")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })
}

fn test_assertion_next_action_summary(scaffold: &Value) -> Result<TestAssertionNextActionSummary> {
    let action = scaffold
        .get("test_assertion_next_action")
        .ok_or_else(|| eyre!("next-edit scaffold receipt missing test_assertion_next_action"))?;
    let test_more_candidate = action.pointer("/test_more_candidate/candidate");
    let test2_candidate = action.pointer("/test2_candidate/candidate");
    Ok(TestAssertionNextActionSummary {
        test_more_candidate_prepared: test_more_candidate.is_some(),
        test_more_candidate_editor_visible: test_more_candidate
            .and_then(|candidate| candidate.get("editorVisible"))
            .and_then(Value::as_bool)
            .unwrap_or(true),
        test2_candidate_prepared: test2_candidate.is_some(),
        test2_candidate_editor_visible: test2_candidate
            .and_then(|candidate| candidate.get("editorVisible"))
            .and_then(Value::as_bool)
            .unwrap_or(true),
        non_test_file_rejected: rejection_reason_present(
            action,
            "/non_test_file/rejectionReasons",
            "test_file_required",
        )?,
        unsupported_framework_rejected: rejection_reason_present(
            action,
            "/unsupported_framework/rejectionReasons",
            "unsupported_test_framework",
        )?,
        missing_variables_rejected: rejection_reason_present(
            action,
            "/missing_variables/rejectionReasons",
            "missing_assertion_variables",
        )?,
        default_gate_disabled: action.pointer("/default_gate/status").and_then(Value::as_str)
            == Some("disabled"),
        explicit_gate_runtime_unregistered: action
            .pointer("/explicit_gate/status")
            .and_then(Value::as_str)
            == Some("runtime_provider_not_registered"),
        parse_stable: action.get("parse_stable").and_then(Value::as_bool).unwrap_or(false),
    })
}

fn require_missing_import_next_action(scaffold: &Value) -> Result<()> {
    let action = scaffold
        .get("missing_import_next_action")
        .ok_or_else(|| eyre!("next-edit scaffold receipt missing missing_import_next_action"))?;
    require_next_edit_value(
        action.pointer("/reachable_candidate/status").and_then(Value::as_str),
        "missing_import_next_action/reachable_candidate/status",
        "receipt_only",
    )?;
    require_next_edit_value(
        action.pointer("/reachable_candidate/candidate/family").and_then(Value::as_str),
        "missing_import_next_action/reachable_candidate/candidate/family",
        "missing_import",
    )?;
    require_next_edit_value(
        action.pointer("/reachable_candidate/candidate/module").and_then(Value::as_str),
        "missing_import_next_action/reachable_candidate/candidate/module",
        "My::App",
    )?;
    require_next_edit_bool(
        action.pointer("/reachable_candidate/candidate/editorVisible").and_then(Value::as_bool),
        "missing_import_next_action/reachable_candidate/candidate/editorVisible",
        false,
    )?;
    require_next_edit_value(
        action.pointer("/reachable_candidate/candidate/edit/newText").and_then(Value::as_str),
        "missing_import_next_action/reachable_candidate/candidate/edit/newText",
        "use My::App;\n",
    )?;
    let accepted_text =
        action.get("accepted_document_text").and_then(Value::as_str).ok_or_else(|| {
            eyre!("next-edit scaffold receipt missing missing_import accepted_document_text")
        })?;
    if !accepted_text.contains("use My::App;\nmy $value = My::App->new;") {
        bail!("missing-import next action accepted document text did not contain import");
    }
    if !rejection_reason_present(action, "/duplicate_import/rejectionReasons", "duplicate_import")?
    {
        bail!("missing-import next action did not reject duplicate import");
    }
    if !rejection_reason_present(
        action,
        "/unreachable_module/rejectionReasons",
        "unreachable_module",
    )? {
        bail!("missing-import next action did not reject unreachable module");
    }
    require_next_edit_value(
        action.pointer("/default_gate/status").and_then(Value::as_str),
        "missing_import_next_action/default_gate/status",
        "disabled",
    )?;
    require_next_edit_value(
        action.pointer("/explicit_gate/status").and_then(Value::as_str),
        "missing_import_next_action/explicit_gate/status",
        "runtime_provider_not_registered",
    )?;
    require_next_edit_bool(
        action.get("parse_stable").and_then(Value::as_bool),
        "missing_import_next_action/parse_stable",
        true,
    )?;
    require_next_edit_bool(
        action.get("line_endings_preserved").and_then(Value::as_bool),
        "missing_import_next_action/line_endings_preserved",
        true,
    )?;
    let crlf_accepted_text =
        action.get("crlf_accepted_document_text").and_then(Value::as_str).ok_or_else(|| {
            eyre!("next-edit scaffold receipt missing missing_import crlf_accepted_document_text")
        })?;
    if !crlf_accepted_text.contains("use My::App;\r\nmy $value = My::App->new;") {
        bail!(
            "missing-import next action CRLF accepted document text did not preserve line endings"
        );
    }
    if crlf_accepted_text.contains("use My::App;\nmy $value = My::App->new;") {
        bail!("missing-import next action CRLF accepted document text mixed LF line endings");
    }
    Ok(())
}

fn require_test_assertion_next_action(scaffold: &Value) -> Result<()> {
    let action = scaffold
        .get("test_assertion_next_action")
        .ok_or_else(|| eyre!("next-edit scaffold receipt missing test_assertion_next_action"))?;
    require_next_edit_value(
        action.pointer("/test_more_candidate/status").and_then(Value::as_str),
        "test_assertion_next_action/test_more_candidate/status",
        "receipt_only",
    )?;
    require_next_edit_value(
        action.pointer("/test_more_candidate/candidate/family").and_then(Value::as_str),
        "test_assertion_next_action/test_more_candidate/candidate/family",
        "test_assertion_body",
    )?;
    require_next_edit_value(
        action.pointer("/test_more_candidate/candidate/edit/newText").and_then(Value::as_str),
        "test_assertion_next_action/test_more_candidate/candidate/edit/newText",
        "is($got, $expected, 'test description');\n",
    )?;
    require_next_edit_bool(
        action.pointer("/test_more_candidate/candidate/editorVisible").and_then(Value::as_bool),
        "test_assertion_next_action/test_more_candidate/candidate/editorVisible",
        false,
    )?;
    require_next_edit_value(
        action.pointer("/test2_candidate/status").and_then(Value::as_str),
        "test_assertion_next_action/test2_candidate/status",
        "receipt_only",
    )?;
    require_next_edit_value(
        action.pointer("/test2_candidate/candidate/framework").and_then(Value::as_str),
        "test_assertion_next_action/test2_candidate/candidate/framework",
        "test2_v0",
    )?;
    require_next_edit_bool(
        action.pointer("/test2_candidate/candidate/editorVisible").and_then(Value::as_bool),
        "test_assertion_next_action/test2_candidate/candidate/editorVisible",
        false,
    )?;
    let accepted_text =
        action.get("accepted_document_text").and_then(Value::as_str).ok_or_else(|| {
            eyre!("next-edit scaffold receipt missing test assertion accepted_document_text")
        })?;
    if !accepted_text.contains("is($got, $expected, 'test description');") {
        bail!("test assertion next action accepted document text did not contain assertion");
    }
    if accepted_text.contains("done_testing") {
        bail!("test assertion next action should not emit done_testing");
    }
    if !rejection_reason_present(action, "/non_test_file/rejectionReasons", "test_file_required")? {
        bail!("test assertion next action did not reject non-test file context");
    }
    if !rejection_reason_present(
        action,
        "/unsupported_framework/rejectionReasons",
        "unsupported_test_framework",
    )? {
        bail!("test assertion next action did not reject unsupported framework");
    }
    if !rejection_reason_present(
        action,
        "/missing_variables/rejectionReasons",
        "missing_assertion_variables",
    )? {
        bail!("test assertion next action did not reject missing variables");
    }
    require_next_edit_value(
        action.pointer("/default_gate/status").and_then(Value::as_str),
        "test_assertion_next_action/default_gate/status",
        "disabled",
    )?;
    require_next_edit_value(
        action.pointer("/explicit_gate/status").and_then(Value::as_str),
        "test_assertion_next_action/explicit_gate/status",
        "runtime_provider_not_registered",
    )?;
    require_next_edit_bool(
        action.get("parse_stable").and_then(Value::as_bool),
        "test_assertion_next_action/parse_stable",
        true,
    )?;
    Ok(())
}

fn rejection_reason_present(action: &Value, pointer: &str, expected: &str) -> Result<bool> {
    let reasons = action
        .pointer(pointer)
        .and_then(Value::as_array)
        .ok_or_else(|| eyre!("next-edit scaffold receipt `{pointer}` must be an array"))?;
    Ok(reasons.iter().any(|reason| reason.as_str() == Some(expected)))
}

fn require_next_edit_value(actual: Option<&str>, field: &str, expected: &str) -> Result<()> {
    if actual != Some(expected) {
        bail!(
            "next-edit scaffold receipt `{field}` must be `{expected}`, got `{}`",
            actual.unwrap_or("<missing>")
        );
    }
    Ok(())
}

fn require_next_edit_bool(actual: Option<bool>, field: &str, expected: bool) -> Result<()> {
    if actual != Some(expected) {
        bail!(
            "next-edit scaffold receipt `{field}` must be `{expected}`, got `{}`",
            actual.map_or("<missing>".to_string(), |value| value.to_string())
        );
    }
    Ok(())
}

fn require_empty_suggestions(scaffold: &Value, pointer: &str) -> Result<()> {
    let suggestions = scaffold
        .pointer(pointer)
        .and_then(Value::as_array)
        .ok_or_else(|| eyre!("next-edit scaffold receipt `{pointer}` must be an array"))?;
    if !suggestions.is_empty() {
        bail!("next-edit scaffold receipt `{pointer}` must remain empty");
    }
    Ok(())
}

fn quality_count_summary(quality: &Value, pointer: &str) -> Result<Option<QualityCountSummary>> {
    let Some(value) = quality.pointer(pointer) else {
        return Ok(None);
    };
    let object =
        value.as_object().ok_or_else(|| eyre!("quality receipt `{pointer}` must be an object"))?;

    Ok(Some(QualityCountSummary {
        total: quality_count_field(object, pointer, "total")?,
        passed: quality_count_field(object, pointer, "passed")?,
        failed: quality_count_field(object, pointer, "failed")?,
    }))
}

fn quality_count_field(
    object: &serde_json::Map<String, Value>,
    pointer: &str,
    field: &str,
) -> Result<u64> {
    object
        .get(field)
        .and_then(Value::as_u64)
        .ok_or_else(|| eyre!("quality receipt `{pointer}/{field}` must be an unsigned count"))
}

fn quality_counter_map(quality: &Value, pointer: &str) -> Result<Option<BTreeMap<String, u64>>> {
    let Some(counters) = quality.pointer(pointer) else {
        return Ok(None);
    };
    let counters = counters
        .as_object()
        .ok_or_else(|| eyre!("quality receipt `{pointer}` must be an object"))?;

    let mut result = BTreeMap::new();
    for (name, count) in counters {
        let count = count
            .as_u64()
            .ok_or_else(|| eyre!("quality receipt `{pointer}/{name}` must be an unsigned count"))?;
        result.insert(name.clone(), count);
    }

    Ok(Some(result))
}

fn quality_source_summaries(
    quality: &Value,
) -> Result<Option<BTreeMap<String, SourceQualityCounterSummary>>> {
    let Some(sources) = quality.get("sources") else {
        return Ok(None);
    };
    let sources =
        sources.as_object().ok_or_else(|| eyre!("quality receipt `/sources` must be an object"))?;

    let mut result = BTreeMap::new();
    for (source_name, source) in sources {
        let source = source
            .as_object()
            .ok_or_else(|| eyre!("quality receipt `/sources/{source_name}` must be an object"))?;
        let edit_application = quality_count_summary_object(
            source,
            &format!("/sources/{source_name}/edit_application"),
        )?;
        let suppression_reasons = required_quality_counter_map(
            source,
            &format!("/sources/{source_name}/suppression_reasons"),
        )?;

        result.insert(
            source_name.clone(),
            SourceQualityCounterSummary {
                expected: quality_count_field(
                    source,
                    &format!("/sources/{source_name}"),
                    "expected",
                )?,
                passed: quality_count_field(source, &format!("/sources/{source_name}"), "passed")?,
                failed: quality_count_field(source, &format!("/sources/{source_name}"), "failed")?,
                returned_items: quality_count_field(
                    source,
                    &format!("/sources/{source_name}"),
                    "returned_items",
                )?,
                edit_application,
                parse_regressions: quality_count_field(
                    source,
                    &format!("/sources/{source_name}"),
                    "parse_regressions",
                )?,
                suppression_reasons,
            },
        );
    }

    Ok(Some(result))
}

fn quality_count_summary_object(
    object: &serde_json::Map<String, Value>,
    pointer: &str,
) -> Result<QualityCountSummary> {
    let value = object
        .get("edit_application")
        .ok_or_else(|| eyre!("quality receipt `{pointer}` must be an object"))?;
    let object =
        value.as_object().ok_or_else(|| eyre!("quality receipt `{pointer}` must be an object"))?;

    Ok(QualityCountSummary {
        total: quality_count_field(object, pointer, "total")?,
        passed: quality_count_field(object, pointer, "passed")?,
        failed: quality_count_field(object, pointer, "failed")?,
    })
}

fn required_quality_counter_map(
    object: &serde_json::Map<String, Value>,
    pointer: &str,
) -> Result<BTreeMap<String, u64>> {
    let counters = object
        .get("suppression_reasons")
        .ok_or_else(|| eyre!("quality receipt `{pointer}` must be an object"))?;
    let counters = counters
        .as_object()
        .ok_or_else(|| eyre!("quality receipt `{pointer}` must be an object"))?;

    let mut result = BTreeMap::new();
    for (name, count) in counters {
        let count = count
            .as_u64()
            .ok_or_else(|| eyre!("quality receipt `{pointer}/{name}` must be an unsigned count"))?;
        result.insert(name.clone(), count);
    }

    Ok(result)
}

fn validate_quality_counter_summary(summary: &InlineQualityCounterSummary) -> Result<()> {
    if !summary.available {
        return Ok(());
    }

    match (summary.fixtures_total, summary.fixtures_passed) {
        (Some(total), Some(passed)) if total != passed => bail!(
            "quality receipt `{}` did not pass all fixtures: {passed}/{total}",
            summary.source
        ),
        _ => {}
    }

    if let Some(edit_application) = &summary.edit_application {
        validate_count_summary(
            edit_application,
            &format!("{}/checks/edit_application", summary.source),
        )?;
    }

    if summary.parse_regressions.unwrap_or(0) != 0 {
        bail!(
            "quality receipt `{}` reported {} parse regression(s)",
            summary.source,
            summary.parse_regressions.unwrap_or(0)
        );
    }

    if let Some(sources) = &summary.sources {
        for (source_name, source) in sources {
            let pointer = format!("{}/sources/{source_name}", summary.source);
            validate_source_quality_counter_summary(source, &pointer)?;
        }
    }

    Ok(())
}

fn validate_source_quality_counter_summary(
    source: &SourceQualityCounterSummary,
    pointer: &str,
) -> Result<()> {
    if source.expected != source.passed + source.failed {
        bail!(
            "quality receipt `{pointer}` expected count must equal passed plus failed, got expected={}, passed={}, failed={}",
            source.expected,
            source.passed,
            source.failed
        );
    }
    if source.failed != 0 {
        bail!("quality receipt `{pointer}` reported {} failed source fixture(s)", source.failed);
    }
    if source.parse_regressions != 0 {
        bail!(
            "quality receipt `{pointer}` reported {} source parse regression(s)",
            source.parse_regressions
        );
    }
    validate_count_summary(&source.edit_application, &format!("{pointer}/edit_application"))
}

fn validate_count_summary(summary: &QualityCountSummary, pointer: &str) -> Result<()> {
    if summary.total != summary.passed + summary.failed {
        bail!(
            "quality receipt `{pointer}` total must equal passed plus failed, got total={}, passed={}, failed={}",
            summary.total,
            summary.passed,
            summary.failed
        );
    }
    if summary.failed != 0 {
        bail!("quality receipt `{pointer}` reported {} failed check(s)", summary.failed);
    }
    Ok(())
}

fn summarize_matrix(
    matrix: &Value,
    matrix_path: &'static str,
    quality_counters: InlineQualityCounterSummary,
    next_edit_scaffold: NextEditScaffoldSummary,
) -> Result<SemanticInlineReceipt> {
    validate_quality_counter_summary(&quality_counters)?;

    let workflows = matrix
        .get("workflows")
        .and_then(Value::as_array)
        .ok_or_else(|| eyre!("fixture matrix missing workflows array"))?;

    let mut by_id = BTreeMap::new();
    for workflow in workflows {
        let id = workflow
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| eyre!("workflow missing string id"))?;
        by_id.insert(id, workflow);
    }

    let mut semantic_inline = BTreeMap::new();
    let mut missing = Vec::new();
    for requirement in REQUIRED_SEMANTIC_INLINE_RECEIPTS {
        let Some(workflow) = by_id.get(requirement.workflow_id) else {
            missing.push(requirement.workflow_id);
            continue;
        };
        validate_semantic_workflow(requirement.workflow_id, workflow)?;
        semantic_inline.insert(
            requirement.capability,
            SemanticInlineCapabilityReceipt {
                status: "registered",
                workflow_id: requirement.workflow_id,
                scenario_file: workflow_string(workflow, "scenario_file")?,
                user_journey: workflow_string(workflow, "user_journey")?,
                expected_outcomes: workflow_string_array(workflow, "expected_outcomes")?,
            },
        );
    }

    if !missing.is_empty() {
        bail!("semantic inline receipt dashboard missing workflows: {}", missing.join(", "));
    }

    let future_gated = BTreeMap::from([
        ("next_edit", "future_gated"),
        ("optional_ai_candidate_source", "future_gated"),
    ]);

    Ok(SemanticInlineReceipt {
        schema_version: "semantic-inline-receipts.v1",
        provider: "inline_completion",
        provider_action: "semantic_inline_receipt_dashboard",
        claim_boundary: "machine-readable semantic inline UX receipt inventory only; does not run UX scenarios, promote support tier, mirror to source, release, or enable AI/next-edit behavior",
        matrix_path,
        workflow_count: workflows.len(),
        required_capability_count: REQUIRED_SEMANTIC_INLINE_RECEIPTS.len(),
        all_required_capabilities_registered: true,
        semantic_inline,
        quality_counters,
        next_edit_scaffold,
        future_gated,
    })
}

fn validate_semantic_workflow(id: &str, workflow: &Value) -> Result<()> {
    let component = workflow_string(workflow, "component")?;
    if component != "completion" {
        bail!("semantic inline workflow `{id}` must be in completion component, got {component}");
    }

    let scenario_file = workflow_string(workflow, "scenario_file")?;
    if !scenario_file.contains("inline_completion_quality") {
        bail!(
            "semantic inline workflow `{id}` must point at inline-completion quality scenario, got {scenario_file}"
        );
    }

    let measures = workflow_string_array(workflow, "measures")?;
    let measures = measures.into_iter().collect::<BTreeSet<_>>();
    if !measures.contains("workflow_pass_rate") || !measures.contains("workflow_stability_rate") {
        bail!("semantic inline workflow `{id}` must keep pass and stability measures");
    }

    let confidence_signals = workflow_string_array(workflow, "confidence_signals")?;
    let confidence_signals = confidence_signals.into_iter().collect::<BTreeSet<_>>();
    for required_signal in
        ["first_five_minutes_harness", "manual_editor_smoke", "issue_burndown_regression_guard"]
    {
        if !confidence_signals.contains(required_signal) {
            bail!("semantic inline workflow `{id}` missing confidence signal {required_signal}");
        }
    }

    let expected_outcomes = workflow_string_array(workflow, "expected_outcomes")?;
    if !expected_outcomes.iter().any(|outcome| {
        outcome.contains("inline completion") || outcome.contains("inline-completion")
    }) {
        bail!("semantic inline workflow `{id}` must state an inline completion outcome");
    }

    Ok(())
}

fn workflow_string(workflow: &Value, key: &str) -> Result<String> {
    workflow
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| eyre!("workflow missing string `{key}`"))
}

fn workflow_string_array(workflow: &Value, key: &str) -> Result<Vec<String>> {
    let values = workflow
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| eyre!("workflow missing array `{key}`"))?;
    values
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| eyre!("workflow `{key}` entries must be strings"))
        })
        .collect()
}

fn write_receipt(path: &Path, receipt: &SemanticInlineReceipt) -> Result<()> {
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
    use serde_json::json;

    fn workflow(id: &'static str) -> Value {
        json!({
            "id": id,
            "scenario_file": format!("ux_scenario_{id}_inline_completion_quality.rs"),
            "component": "completion",
            "user_journey": format!("exercise {id} inline completion"),
            "measures": ["workflow_pass_rate", "workflow_stability_rate"],
            "expected_outcomes": ["inline completion behavior remains covered"],
            "confidence_signals": [
                "first_five_minutes_harness",
                "manual_editor_smoke",
                "issue_burndown_regression_guard"
            ]
        })
    }

    fn complete_matrix() -> Value {
        let workflows = REQUIRED_SEMANTIC_INLINE_RECEIPTS
            .iter()
            .map(|requirement| workflow(requirement.workflow_id))
            .collect::<Vec<_>>();
        json!({ "workflows": workflows })
    }

    fn next_edit_scaffold_summary_error(
        path: &Path,
        expectation: &'static str,
    ) -> color_eyre::Report {
        let result = read_optional_next_edit_scaffold_summary(path);
        assert!(result.is_err(), "{expectation}");
        result.err().unwrap_or_else(|| eyre!("{expectation}"))
    }

    fn unavailable_quality() -> InlineQualityCounterSummary {
        InlineQualityCounterSummary {
            source: "target/receipts/inline-completion-quality.json".to_string(),
            available: false,
            all_checks_green: None,
            fixtures_total: None,
            fixtures_passed: None,
            edit_application: None,
            hard_zone_rejections: None,
            suppression_reasons: None,
            parse_regressions: None,
            sources: None,
        }
    }

    fn unavailable_next_edit_scaffold() -> NextEditScaffoldSummary {
        NextEditScaffoldSummary {
            source: "target/receipts/semantic-inline-next-edit.json".to_string(),
            available: false,
            schema_version: None,
            provider_action: None,
            enabled_by_default: None,
            runtime_provider_registered: None,
            ai_candidate_source_enabled: None,
            default_status: None,
            receipt_only_status: None,
            explicit_gate_status: None,
            planned_candidate_families: None,
            future_gated: None,
            missing_import_next_action: None,
            test_assertion_next_action: None,
        }
    }

    fn valid_next_edit_scaffold_json() -> Value {
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
            "test_assertion_next_action": valid_test_assertion_next_action_json()
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
            "parse_stable": true,
            "line_endings_preserved": true
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

    fn green_quality() -> InlineQualityCounterSummary {
        let mut sources = BTreeMap::new();
        sources.insert(
            "module".to_string(),
            SourceQualityCounterSummary {
                expected: 2,
                passed: 2,
                failed: 0,
                returned_items: 3,
                edit_application: QualityCountSummary { total: 2, passed: 2, failed: 0 },
                parse_regressions: 0,
                suppression_reasons: BTreeMap::new(),
            },
        );

        InlineQualityCounterSummary {
            source: "target/receipts/inline-completion-quality.json".to_string(),
            available: true,
            all_checks_green: Some(true),
            fixtures_total: Some(2),
            fixtures_passed: Some(2),
            edit_application: Some(QualityCountSummary { total: 2, passed: 2, failed: 0 }),
            hard_zone_rejections: Some(0),
            suppression_reasons: Some(BTreeMap::new()),
            parse_regressions: Some(0),
            sources: Some(sources),
        }
    }

    #[test]
    fn dashboard_summarizes_required_semantic_inline_capabilities() -> Result<()> {
        let receipt = summarize_matrix(
            &complete_matrix(),
            MATRIX_PATH,
            unavailable_quality(),
            unavailable_next_edit_scaffold(),
        )?;

        assert!(receipt.all_required_capabilities_registered);
        assert_eq!(receipt.required_capability_count, REQUIRED_SEMANTIC_INLINE_RECEIPTS.len());
        for requirement in REQUIRED_SEMANTIC_INLINE_RECEIPTS {
            let entry = receipt.semantic_inline.get(requirement.capability).ok_or_else(|| {
                eyre!("missing semantic inline capability {}", requirement.capability)
            })?;
            assert_eq!(entry.status, "registered");
            assert_eq!(entry.workflow_id, requirement.workflow_id);
            assert_eq!(
                entry.scenario_file,
                format!("ux_scenario_{}_inline_completion_quality.rs", requirement.workflow_id)
            );
            assert_eq!(
                entry.user_journey,
                format!("exercise {} inline completion", requirement.workflow_id)
            );
            assert_eq!(
                entry.expected_outcomes,
                vec!["inline completion behavior remains covered".to_string()]
            );
        }
        assert_eq!(receipt.future_gated.get("next_edit"), Some(&"future_gated"));
        assert_eq!(receipt.future_gated.get("optional_ai_candidate_source"), Some(&"future_gated"));
        assert!(!receipt.next_edit_scaffold.available);
        Ok(())
    }

    #[test]
    fn dashboard_rejects_missing_required_semantic_inline_workflow() -> Result<()> {
        let mut matrix = complete_matrix();
        let workflows = matrix
            .get_mut("workflows")
            .and_then(Value::as_array_mut)
            .ok_or_else(|| eyre!("test matrix missing workflows"))?;
        workflows.retain(|workflow| {
            workflow.get("id").and_then(Value::as_str)
                != Some("real_workspace_module_import_inline_completion_quality")
        });

        let Err(error) = summarize_matrix(
            &matrix,
            MATRIX_PATH,
            unavailable_quality(),
            unavailable_next_edit_scaffold(),
        ) else {
            bail!("missing workflow must fail dashboard generation");
        };
        assert!(
            error.to_string().contains("real_workspace_module_import_inline_completion_quality"),
            "error should identify missing workflow, got {error}"
        );
        Ok(())
    }

    #[test]
    fn next_edit_scaffold_summary_accepts_disabled_receipt() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("semantic-inline-next-edit.json");
        fs::write(&path, serde_json::to_vec_pretty(&valid_next_edit_scaffold_json())?)?;

        let summary = read_optional_next_edit_scaffold_summary(&path)?;

        assert!(summary.available);
        assert_eq!(summary.schema_version.as_deref(), Some("semantic-inline-next-edit.v1"));
        assert_eq!(summary.enabled_by_default, Some(false));
        assert_eq!(summary.runtime_provider_registered, Some(false));
        assert_eq!(summary.ai_candidate_source_enabled, Some(false));
        assert_eq!(summary.default_status.as_deref(), Some("disabled"));
        assert_eq!(
            summary.explicit_gate_status.as_deref(),
            Some("runtime_provider_not_registered")
        );
        Ok(())
    }

    #[test]
    fn next_edit_scaffold_summary_treats_missing_receipt_as_unavailable() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("semantic-inline-next-edit.json");

        let summary = read_optional_next_edit_scaffold_summary(&path)?;

        assert!(!summary.available);
        assert_eq!(summary.schema_version, None);
        assert_eq!(summary.planned_candidate_families, None);
        validate_next_edit_scaffold_summary(&summary, &json!({}))?;
        Ok(())
    }

    #[test]
    fn next_edit_scaffold_summary_rejects_malformed_candidate_lists() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("semantic-inline-next-edit.json");
        let mut scaffold = valid_next_edit_scaffold_json();
        scaffold["planned_candidate_families"] = json!("missing_import");
        fs::write(&path, serde_json::to_vec_pretty(&scaffold)?)?;

        let Err(error) = read_optional_next_edit_scaffold_summary(&path) else {
            bail!("next-edit scaffold with scalar planned families must fail");
        };
        assert!(
            error.to_string().contains("planned_candidate_families"),
            "error should identify scalar planned families, got {error}"
        );

        scaffold = valid_next_edit_scaffold_json();
        scaffold["future_gated"] = json!(["runtime_next_edit_provider", false]);
        fs::write(&path, serde_json::to_vec_pretty(&scaffold)?)?;

        let Err(error) = read_optional_next_edit_scaffold_summary(&path) else {
            bail!("next-edit scaffold with non-string future gate must fail");
        };
        assert!(
            error.to_string().contains("future_gated"),
            "error should identify non-string future gate, got {error}"
        );
        Ok(())
    }

    #[test]
    fn next_edit_scaffold_summary_rejects_missing_required_lists() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("semantic-inline-next-edit.json");
        let mut scaffold = valid_next_edit_scaffold_json();
        scaffold
            .as_object_mut()
            .ok_or_else(|| eyre!("test scaffold must be an object"))?
            .remove("planned_candidate_families");
        fs::write(&path, serde_json::to_vec_pretty(&scaffold)?)?;

        let Err(error) = read_optional_next_edit_scaffold_summary(&path) else {
            bail!("next-edit scaffold missing planned families must fail");
        };
        assert!(
            error.to_string().contains("planned_candidate_families"),
            "error should identify missing planned families, got {error}"
        );

        scaffold = valid_next_edit_scaffold_json();
        scaffold
            .as_object_mut()
            .ok_or_else(|| eyre!("test scaffold must be an object"))?
            .remove("future_gated");
        fs::write(&path, serde_json::to_vec_pretty(&scaffold)?)?;

        let Err(error) = read_optional_next_edit_scaffold_summary(&path) else {
            bail!("next-edit scaffold missing future gates must fail");
        };
        assert!(
            error.to_string().contains("future_gated"),
            "error should identify missing future gates, got {error}"
        );
        Ok(())
    }

    #[test]
    fn next_edit_scaffold_summary_rejects_editor_visible_suggestions() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("semantic-inline-next-edit.json");
        let mut scaffold = valid_next_edit_scaffold_json();
        scaffold["explicit_gate_response"]["suggestions"] = json!([
            {
                "family": "missing_import",
                "newText": "use My::App;\n"
            }
        ]);
        fs::write(&path, serde_json::to_vec_pretty(&scaffold)?)?;

        let Err(error) = read_optional_next_edit_scaffold_summary(&path) else {
            bail!("next-edit scaffold with suggestions must fail");
        };
        assert!(
            error.to_string().contains("explicit_gate_response/suggestions"),
            "error should identify emitted suggestions, got {error}"
        );
        Ok(())
    }

    #[test]
    fn next_edit_scaffold_summary_rejects_ai_enabled() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("semantic-inline-next-edit.json");
        let mut scaffold = valid_next_edit_scaffold_json();
        scaffold["ai_candidate_source_enabled"] = json!(true);
        fs::write(&path, serde_json::to_vec_pretty(&scaffold)?)?;

        let Err(error) = read_optional_next_edit_scaffold_summary(&path) else {
            bail!("next-edit scaffold with AI enabled must fail");
        };
        assert!(
            error.to_string().contains("ai_candidate_source_enabled"),
            "error should identify AI gate drift, got {error}"
        );
        Ok(())
    }

    #[test]
    fn next_edit_scaffold_summary_rejects_missing_import_drift() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("semantic-inline-next-edit.json");
        let mut scaffold = valid_next_edit_scaffold_json();
        scaffold
            .as_object_mut()
            .ok_or_else(|| eyre!("test scaffold must be an object"))?
            .remove("missing_import_next_action");
        fs::write(&path, serde_json::to_vec_pretty(&scaffold)?)?;

        let Err(error) = read_optional_next_edit_scaffold_summary(&path) else {
            bail!("next-edit scaffold without missing-import proof must fail");
        };
        assert!(
            error.to_string().contains("missing_import_next_action"),
            "error should identify missing missing-import proof, got {error}"
        );

        scaffold = valid_next_edit_scaffold_json();
        scaffold["missing_import_next_action"]["reachable_candidate"]["candidate"]["editorVisible"] =
            json!(true);
        fs::write(&path, serde_json::to_vec_pretty(&scaffold)?)?;

        let Err(error) = read_optional_next_edit_scaffold_summary(&path) else {
            bail!("editor-visible missing-import proof must fail");
        };
        assert!(
            error.to_string().contains("editorVisible"),
            "error should identify editor-visible missing-import drift, got {error}"
        );
        Ok(())
    }

    #[test]
    fn next_edit_scaffold_summary_rejects_missing_import_field_drift() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("semantic-inline-next-edit.json");

        let mut scaffold = valid_next_edit_scaffold_json();
        scaffold["missing_import_next_action"]["reachable_candidate"]["candidate"]["edit"]["newText"] =
            json!("use Other::Module;\n");
        fs::write(&path, serde_json::to_vec_pretty(&scaffold)?)?;
        let error =
            next_edit_scaffold_summary_error(&path, "wrong missing-import edit text must fail");
        assert!(
            error.to_string().contains("candidate/edit/newText"),
            "error should identify missing-import edit text drift, got {error}"
        );

        let mut scaffold = valid_next_edit_scaffold_json();
        scaffold["missing_import_next_action"]["accepted_document_text"] =
            json!("use strict;\nuse warnings;\nmy $value = My::App->new;\n");
        fs::write(&path, serde_json::to_vec_pretty(&scaffold)?)?;
        let error = next_edit_scaffold_summary_error(
            &path,
            "accepted text without inserted import must fail",
        );
        assert!(
            error.to_string().contains("accepted document text"),
            "error should identify missing accepted import drift, got {error}"
        );

        let mut scaffold = valid_next_edit_scaffold_json();
        scaffold["missing_import_next_action"]["parse_stable"] = json!(false);
        fs::write(&path, serde_json::to_vec_pretty(&scaffold)?)?;
        let error = next_edit_scaffold_summary_error(
            &path,
            "parse-unstable missing-import proof must fail",
        );
        assert!(
            error.to_string().contains("parse_stable"),
            "error should identify parse-stability drift, got {error}"
        );

        let mut scaffold = valid_next_edit_scaffold_json();
        scaffold["missing_import_next_action"]["line_endings_preserved"] = json!(false);
        fs::write(&path, serde_json::to_vec_pretty(&scaffold)?)?;
        let error = next_edit_scaffold_summary_error(&path, "line-ending drift must fail");
        assert!(
            error.to_string().contains("line_endings_preserved")
                || error.to_string().contains("line endings"),
            "error should identify line-ending drift, got {error}"
        );

        let mut scaffold = valid_next_edit_scaffold_json();
        scaffold["missing_import_next_action"]["crlf_accepted_document_text"] = Value::Null;
        fs::write(&path, serde_json::to_vec_pretty(&scaffold)?)?;
        let error = next_edit_scaffold_summary_error(&path, "missing CRLF accepted text must fail");
        assert!(
            error.to_string().contains("crlf_accepted_document_text"),
            "error should identify missing CRLF accepted text, got {error}"
        );

        let mut scaffold = valid_next_edit_scaffold_json();
        scaffold["missing_import_next_action"]["crlf_accepted_document_text"] =
            json!("package Demo;\r\nuse strict;\r\nuse My::App;\nmy $value = My::App->new;\r\n");
        fs::write(&path, serde_json::to_vec_pretty(&scaffold)?)?;
        let error = next_edit_scaffold_summary_error(&path, "LF-only CRLF accepted text must fail");
        assert!(
            error.to_string().contains("preserve line endings"),
            "error should identify CRLF preservation drift, got {error}"
        );

        let mut scaffold = valid_next_edit_scaffold_json();
        scaffold["missing_import_next_action"]["crlf_accepted_document_text"] = json!(
            "package Demo;\r\nuse strict;\r\nuse My::App;\r\nmy $value = My::App->new;\r\nuse My::App;\nmy $value = My::App->new;\r\n"
        );
        fs::write(&path, serde_json::to_vec_pretty(&scaffold)?)?;
        let error =
            next_edit_scaffold_summary_error(&path, "mixed-line-ending accepted text must fail");
        assert!(
            error.to_string().contains("mixed LF line endings"),
            "error should identify mixed LF line endings, got {error}"
        );

        Ok(())
    }

    #[test]
    fn next_edit_scaffold_summary_rejects_missing_import_rejection_drift() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("semantic-inline-next-edit.json");

        let mut scaffold = valid_next_edit_scaffold_json();
        scaffold["missing_import_next_action"]["duplicate_import"]["rejectionReasons"] = json!([]);
        fs::write(&path, serde_json::to_vec_pretty(&scaffold)?)?;
        let Err(error) = read_optional_next_edit_scaffold_summary(&path) else {
            bail!("duplicate import without rejection reason must fail");
        };
        assert!(
            error.to_string().contains("duplicate import"),
            "error should identify duplicate import rejection drift, got {error}"
        );

        let mut scaffold = valid_next_edit_scaffold_json();
        scaffold["missing_import_next_action"]["unreachable_module"]["rejectionReasons"] =
            json!([]);
        fs::write(&path, serde_json::to_vec_pretty(&scaffold)?)?;
        let Err(error) = read_optional_next_edit_scaffold_summary(&path) else {
            bail!("unreachable module without rejection reason must fail");
        };
        assert!(
            error.to_string().contains("unreachable module"),
            "error should identify unreachable module rejection drift, got {error}"
        );

        let mut scaffold = valid_next_edit_scaffold_json();
        scaffold["missing_import_next_action"]["default_gate"]["status"] = json!("receipt_only");
        fs::write(&path, serde_json::to_vec_pretty(&scaffold)?)?;
        let Err(error) = read_optional_next_edit_scaffold_summary(&path) else {
            bail!("default gate status drift must fail");
        };
        assert!(
            error.to_string().contains("default_gate/status"),
            "error should identify default gate status drift, got {error}"
        );

        let mut scaffold = valid_next_edit_scaffold_json();
        scaffold["missing_import_next_action"]["explicit_gate"]["status"] = json!("receipt_only");
        fs::write(&path, serde_json::to_vec_pretty(&scaffold)?)?;
        let Err(error) = read_optional_next_edit_scaffold_summary(&path) else {
            bail!("explicit gate status drift must fail");
        };
        assert!(
            error.to_string().contains("explicit_gate/status"),
            "error should identify explicit gate status drift, got {error}"
        );

        Ok(())
    }

    #[test]
    fn next_edit_scaffold_summary_rejects_test_assertion_field_drift() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("semantic-inline-next-edit.json");

        let mut scaffold = valid_next_edit_scaffold_json();
        scaffold["test_assertion_next_action"]["test_more_candidate"]["candidate"]["edit"]["newText"] =
            json!("done_testing();\n");
        fs::write(&path, serde_json::to_vec_pretty(&scaffold)?)?;
        let error =
            next_edit_scaffold_summary_error(&path, "wrong test assertion edit text must fail");
        assert!(
            error.to_string().contains("candidate/edit/newText"),
            "error should identify test assertion edit text drift, got {error}"
        );

        let mut scaffold = valid_next_edit_scaffold_json();
        scaffold["test_assertion_next_action"]["accepted_document_text"] =
            json!("use Test::More;\nmy $got = compute();\nmy $expected = 42;\n");
        fs::write(&path, serde_json::to_vec_pretty(&scaffold)?)?;
        let error = next_edit_scaffold_summary_error(
            &path,
            "accepted text without inserted assertion must fail",
        );
        assert!(
            error.to_string().contains("accepted document text"),
            "error should identify missing accepted assertion drift, got {error}"
        );

        let mut scaffold = valid_next_edit_scaffold_json();
        scaffold["test_assertion_next_action"]["accepted_document_text"] = json!(
            "use Test::More;\nmy $got = compute();\nmy $expected = 42;\nis($got, $expected, 'test description');\ndone_testing();\n"
        );
        fs::write(&path, serde_json::to_vec_pretty(&scaffold)?)?;
        let error = next_edit_scaffold_summary_error(&path, "premature done_testing must fail");
        assert!(
            error.to_string().contains("done_testing"),
            "error should identify done_testing drift, got {error}"
        );

        let mut scaffold = valid_next_edit_scaffold_json();
        scaffold["test_assertion_next_action"]["parse_stable"] = json!(false);
        fs::write(&path, serde_json::to_vec_pretty(&scaffold)?)?;
        let error = next_edit_scaffold_summary_error(
            &path,
            "parse-unstable test assertion proof must fail",
        );
        assert!(
            error.to_string().contains("parse_stable"),
            "error should identify parse-stability drift, got {error}"
        );

        Ok(())
    }

    #[test]
    fn next_edit_scaffold_summary_rejects_test_assertion_rejection_drift() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("semantic-inline-next-edit.json");

        let mut scaffold = valid_next_edit_scaffold_json();
        scaffold["test_assertion_next_action"]["non_test_file"]["rejectionReasons"] = json!([]);
        fs::write(&path, serde_json::to_vec_pretty(&scaffold)?)?;
        let error = next_edit_scaffold_summary_error(&path, "non-test rejection drift must fail");
        assert!(
            error.to_string().contains("non-test file"),
            "error should identify non-test rejection drift, got {error}"
        );

        let mut scaffold = valid_next_edit_scaffold_json();
        scaffold["test_assertion_next_action"]["unsupported_framework"]["rejectionReasons"] =
            json!([]);
        fs::write(&path, serde_json::to_vec_pretty(&scaffold)?)?;
        let error = next_edit_scaffold_summary_error(&path, "framework rejection drift must fail");
        assert!(
            error.to_string().contains("unsupported framework"),
            "error should identify framework rejection drift, got {error}"
        );

        let mut scaffold = valid_next_edit_scaffold_json();
        scaffold["test_assertion_next_action"]["missing_variables"]["rejectionReasons"] = json!([]);
        fs::write(&path, serde_json::to_vec_pretty(&scaffold)?)?;
        let error =
            next_edit_scaffold_summary_error(&path, "missing-variable rejection drift must fail");
        assert!(
            error.to_string().contains("missing variables"),
            "error should identify missing-variable rejection drift, got {error}"
        );

        let mut scaffold = valid_next_edit_scaffold_json();
        scaffold["test_assertion_next_action"]["default_gate"]["status"] = json!("receipt_only");
        fs::write(&path, serde_json::to_vec_pretty(&scaffold)?)?;
        let error = next_edit_scaffold_summary_error(&path, "default gate drift must fail");
        assert!(
            error.to_string().contains("default_gate/status"),
            "error should identify default gate drift, got {error}"
        );

        let mut scaffold = valid_next_edit_scaffold_json();
        scaffold["test_assertion_next_action"]["explicit_gate"]["status"] = json!("receipt_only");
        fs::write(&path, serde_json::to_vec_pretty(&scaffold)?)?;
        let error = next_edit_scaffold_summary_error(&path, "explicit gate drift must fail");
        assert!(
            error.to_string().contains("explicit_gate/status"),
            "error should identify explicit gate drift, got {error}"
        );

        Ok(())
    }

    #[test]
    fn quality_counter_map_returns_none_when_counter_is_absent() -> Result<()> {
        let quality = json!({
            "fixtures_total": 2,
            "checks": {
                "hard_zone_rejected": 1,
                "parse_regressions": 0
            }
        });

        assert!(quality_counter_map(&quality, "/checks/suppression_reasons")?.is_none());

        Ok(())
    }

    #[test]
    fn quality_count_summary_requires_structured_counts() -> Result<()> {
        let quality = json!({
            "checks": {
                "edit_application": {
                    "total": 3,
                    "passed": 2,
                    "failed": 1
                }
            }
        });

        let summary = quality_count_summary(&quality, "/checks/edit_application")?
            .ok_or_else(|| eyre!("missing edit_application summary"))?;
        assert_eq!(summary.total, 3);
        assert_eq!(summary.passed, 2);
        assert_eq!(summary.failed, 1);

        Ok(())
    }

    #[test]
    fn quality_source_summaries_embed_source_outcomes() -> Result<()> {
        let quality = json!({
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
                    "expected": 2,
                    "passed": 2,
                    "failed": 0,
                    "returned_items": 0,
                    "edit_application": {
                        "total": 0,
                        "passed": 0,
                        "failed": 0
                    },
                    "parse_regressions": 0,
                    "suppression_reasons": {
                        "hard_zone": 2
                    }
                }
            }
        });

        let sources = quality_source_summaries(&quality)?
            .ok_or_else(|| eyre!("missing quality source summaries"))?;

        let module = sources.get("module").ok_or_else(|| eyre!("missing module source"))?;
        assert_eq!(module.expected, 4);
        assert_eq!(module.returned_items, 6);
        assert_eq!(module.edit_application.passed, 4);

        let hard_zone =
            sources.get("hard_zone").ok_or_else(|| eyre!("missing hard_zone source"))?;
        assert_eq!(hard_zone.returned_items, 0);
        assert_eq!(hard_zone.suppression_reasons.get("hard_zone").copied(), Some(2));
        Ok(())
    }

    #[test]
    fn quality_source_summaries_require_unsigned_fields() -> Result<()> {
        let quality = json!({
            "sources": {
                "module": {
                    "expected": "bad",
                    "passed": 0,
                    "failed": 0,
                    "returned_items": 0,
                    "edit_application": {
                        "total": 0,
                        "passed": 0,
                        "failed": 0
                    },
                    "parse_regressions": 0,
                    "suppression_reasons": {}
                }
            }
        });

        let Err(error) = quality_source_summaries(&quality) else {
            bail!("invalid source summary must fail");
        };
        assert!(
            error.to_string().contains("/sources/module/expected"),
            "error should identify invalid source field, got {error}"
        );
        Ok(())
    }

    #[test]
    fn dashboard_accepts_green_quality_counters() -> Result<()> {
        let receipt = summarize_matrix(
            &complete_matrix(),
            MATRIX_PATH,
            green_quality(),
            unavailable_next_edit_scaffold(),
        )?;

        assert!(receipt.quality_counters.available);
        assert_eq!(receipt.quality_counters.all_checks_green, Some(true));
        assert_eq!(
            receipt.quality_counters.edit_application.as_ref().map(|summary| summary.passed),
            Some(2)
        );
        assert_eq!(
            receipt
                .quality_counters
                .sources
                .as_ref()
                .and_then(|sources| sources.get("module"))
                .map(|source| source.edit_application.passed),
            Some(2)
        );
        Ok(())
    }

    #[test]
    fn dashboard_rejects_failing_quality_counters() -> Result<()> {
        let quality = InlineQualityCounterSummary {
            source: "target/receipts/inline-completion-quality.json".to_string(),
            available: true,
            all_checks_green: Some(true),
            fixtures_total: Some(2),
            fixtures_passed: Some(1),
            edit_application: Some(QualityCountSummary { total: 1, passed: 1, failed: 0 }),
            hard_zone_rejections: Some(0),
            suppression_reasons: Some(BTreeMap::new()),
            parse_regressions: Some(0),
            sources: Some(BTreeMap::new()),
        };

        let Err(error) = summarize_matrix(
            &complete_matrix(),
            MATRIX_PATH,
            quality,
            unavailable_next_edit_scaffold(),
        ) else {
            bail!("failing quality counters must fail dashboard generation");
        };
        assert!(
            error.to_string().contains("did not pass all fixtures"),
            "error should identify failing quality counters, got {error}"
        );
        Ok(())
    }

    #[test]
    fn dashboard_rejects_parse_regressions() -> Result<()> {
        let mut quality = green_quality();
        quality.parse_regressions = Some(1);

        let Err(error) = summarize_matrix(
            &complete_matrix(),
            MATRIX_PATH,
            quality,
            unavailable_next_edit_scaffold(),
        ) else {
            bail!("parse regressions must fail dashboard generation");
        };
        assert!(
            error.to_string().contains("parse regression"),
            "error should identify parse regressions, got {error}"
        );
        Ok(())
    }

    #[test]
    fn dashboard_rejects_failing_edit_application_count() -> Result<()> {
        let mut quality = green_quality();
        quality.edit_application = Some(QualityCountSummary { total: 2, passed: 1, failed: 0 });

        let Err(error) = summarize_matrix(
            &complete_matrix(),
            MATRIX_PATH,
            quality,
            unavailable_next_edit_scaffold(),
        ) else {
            bail!("invalid edit application total must fail dashboard generation");
        };
        assert!(
            error.to_string().contains("/checks/edit_application"),
            "error should identify edit application counters, got {error}"
        );
        Ok(())
    }

    #[test]
    fn dashboard_rejects_failed_edit_application_count() -> Result<()> {
        let mut quality = green_quality();
        quality.edit_application = Some(QualityCountSummary { total: 1, passed: 0, failed: 1 });

        let Err(error) = summarize_matrix(
            &complete_matrix(),
            MATRIX_PATH,
            quality,
            unavailable_next_edit_scaffold(),
        ) else {
            bail!("failed edit application count must fail dashboard generation");
        };
        assert!(
            error.to_string().contains("failed check"),
            "error should identify failed edit application counters, got {error}"
        );
        Ok(())
    }

    #[test]
    fn dashboard_rejects_failing_source_quality_counters() -> Result<()> {
        let mut sources = BTreeMap::new();
        sources.insert(
            "module".to_string(),
            SourceQualityCounterSummary {
                expected: 1,
                passed: 1,
                failed: 0,
                returned_items: 2,
                edit_application: QualityCountSummary { total: 1, passed: 1, failed: 0 },
                parse_regressions: 1,
                suppression_reasons: BTreeMap::new(),
            },
        );
        let quality = InlineQualityCounterSummary {
            source: "target/receipts/inline-completion-quality.json".to_string(),
            available: true,
            all_checks_green: Some(true),
            fixtures_total: Some(1),
            fixtures_passed: Some(1),
            edit_application: Some(QualityCountSummary { total: 1, passed: 1, failed: 0 }),
            hard_zone_rejections: Some(0),
            suppression_reasons: Some(BTreeMap::new()),
            parse_regressions: Some(0),
            sources: Some(sources),
        };

        let Err(error) = summarize_matrix(
            &complete_matrix(),
            MATRIX_PATH,
            quality,
            unavailable_next_edit_scaffold(),
        ) else {
            bail!("failing source quality counters must fail dashboard generation");
        };
        assert!(
            error.to_string().contains("/sources/module"),
            "error should identify failing source quality counters, got {error}"
        );
        Ok(())
    }

    #[test]
    fn dashboard_rejects_source_expected_count_mismatch() -> Result<()> {
        let mut quality = green_quality();
        let sources = quality.sources.as_mut().ok_or_else(|| eyre!("missing source summaries"))?;
        let source =
            sources.get_mut("module").ok_or_else(|| eyre!("missing module source summary"))?;
        source.expected = 3;

        let Err(error) = summarize_matrix(
            &complete_matrix(),
            MATRIX_PATH,
            quality,
            unavailable_next_edit_scaffold(),
        ) else {
            bail!("source count mismatch must fail dashboard generation");
        };
        assert!(
            error.to_string().contains("expected count"),
            "error should identify source count mismatch, got {error}"
        );
        Ok(())
    }

    #[test]
    fn dashboard_rejects_failed_source_count() -> Result<()> {
        let mut quality = green_quality();
        let sources = quality.sources.as_mut().ok_or_else(|| eyre!("missing source summaries"))?;
        let source =
            sources.get_mut("module").ok_or_else(|| eyre!("missing module source summary"))?;
        source.expected = 3;
        source.failed = 1;

        let Err(error) = summarize_matrix(
            &complete_matrix(),
            MATRIX_PATH,
            quality,
            unavailable_next_edit_scaffold(),
        ) else {
            bail!("failed source count must fail dashboard generation");
        };
        assert!(
            error.to_string().contains("failed source fixture"),
            "error should identify failed source counters, got {error}"
        );
        Ok(())
    }
}
