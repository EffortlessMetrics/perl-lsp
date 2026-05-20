//! Live provider decision trace shaping.

use serde_json::{Value, json};

use crate::protocol::JsonRpcError;

pub(in crate::runtime::language) const DIAGNOSTIC_EXPLANATION_SCHEMA_VERSION: &str =
    "diagnostic_explanation.v1";
const MAX_DIAGNOSTIC_EXPLANATIONS: usize = 8;
const MAX_REPORTED_INC_PATHS: usize = 8;

pub(super) struct LiveProviderResultShape {
    pub(super) decision: &'static str,
    pub(super) reason: &'static str,
    pub(super) fallback_state: &'static str,
    pub(super) result_kind: &'static str,
    pub(super) result_count: usize,
    pub(super) error: Option<Value>,
}

pub(super) fn live_provider_trace_key(method: &str) -> Option<&'static str> {
    match method {
        "textDocument/hover" => Some("hover"),
        "textDocument/diagnostic" | "workspace/diagnostic" => Some("diagnostics"),
        "textDocument/documentSymbol" => Some("document_symbols"),
        "textDocument/semanticTokens/full" | "textDocument/semanticTokens/range" => {
            Some("semantic_tokens")
        }
        _ => None,
    }
}

pub(super) fn live_provider_result_shape(
    result: &Result<Option<Value>, JsonRpcError>,
) -> LiveProviderResultShape {
    match result {
        Ok(Some(value)) => {
            let (result_kind, result_count) = live_provider_value_shape(value);
            let has_result = result_count > 0;
            LiveProviderResultShape {
                decision: if has_result { "acted" } else { "fallback" },
                reason: if has_result { "live_provider_result" } else { "no_result" },
                fallback_state: if has_result { "live_provider" } else { "no_result" },
                result_kind,
                result_count,
                error: None,
            }
        }
        Ok(None) => LiveProviderResultShape {
            decision: "fallback",
            reason: "no_result",
            fallback_state: "no_result",
            result_kind: "none",
            result_count: 0,
            error: None,
        },
        Err(error) => LiveProviderResultShape {
            decision: "fallback",
            reason: "provider_error",
            fallback_state: "provider_error",
            result_kind: "error",
            result_count: 0,
            error: Some(json!({
                "code": error.code,
                "message": error.message,
            })),
        },
    }
}

fn live_provider_value_shape(value: &Value) -> (&'static str, usize) {
    if value.is_null() {
        return ("null", 0);
    }
    if let Some(items) = value.get("items").and_then(Value::as_array) {
        return ("items", items.len());
    }
    if let Some(data) = value.get("data").and_then(Value::as_array) {
        return ("semantic_token_data", data.len() / 5);
    }
    if let Some(changes) = value.get("changes").and_then(Value::as_object) {
        let edit_count = changes.values().filter_map(Value::as_array).map(Vec::len).sum();
        return ("workspace_edit_changes", edit_count);
    }
    if let Some(array) = value.as_array() {
        return ("array", array.len());
    }
    if value.is_object() {
        return ("object", 1);
    }
    ("scalar", 1)
}

pub(super) fn diagnostic_explanation_payload(
    method: &str,
    result: &Result<Option<Value>, JsonRpcError>,
) -> Option<(Value, String, bool)> {
    if !matches!(method, "textDocument/diagnostic" | "workspace/diagnostic") {
        return None;
    }
    let Ok(Some(value)) = result else {
        return None;
    };

    let diagnostics: Vec<Value> = collect_diagnostic_values(value).into_iter().cloned().collect();

    Some(diagnostic_explanation_payload_from_diagnostics(method, &diagnostics))
}

fn collect_diagnostic_values(value: &Value) -> Vec<&Value> {
    let Some(items) = value.get("items").and_then(Value::as_array) else {
        return Vec::new();
    };

    let mut nested = Vec::new();
    for item in items {
        if let Some(document_items) = item.get("items").and_then(Value::as_array) {
            nested.extend(document_items);
        }
    }

    if nested.is_empty() { items.iter().collect() } else { nested }
}

pub(in crate::runtime::language) fn diagnostic_explanation_payload_from_diagnostics(
    method: &str,
    diagnostics: &[Value],
) -> (Value, String, bool) {
    let diagnostic_count = diagnostics.len();
    let explanations: Vec<Value> =
        diagnostics.iter().take(MAX_DIAGNOSTIC_EXPLANATIONS).map(diagnostic_explanation).collect();

    let truncated = diagnostic_count.saturating_sub(explanations.len());
    let user_message = diagnostic_explanation_user_message(diagnostic_count, &explanations);
    let has_dynamic_boundary = explanations.iter().any(|explanation| {
        explanation.get("trust_boundary").and_then(Value::as_str) == Some("dynamic_boundary")
    });

    (
        json!({
            "schema_version": DIAGNOSTIC_EXPLANATION_SCHEMA_VERSION,
            "surface": "diagnostics",
            "decision": "explanation_only",
            "provider_action": method,
            "fact_source": "provider_runtime",
            "confidence": "low",
            "freshness": "fresh",
            "diagnostic_count": diagnostic_count,
            "diagnostic_explanations": explanations,
            "truncated_diagnostic_explanations": truncated,
            "dynamic_boundary_detected": has_dynamic_boundary,
            "claim_boundary": "explains returned diagnostics only; no new suppression, severity, or support-tier promotion",
        }),
        user_message,
        has_dynamic_boundary,
    )
}

fn diagnostic_explanation(diagnostic: &Value) -> Value {
    let code = diagnostic_code(diagnostic);
    let message = diagnostic.get("message").and_then(Value::as_str).unwrap_or("");
    let trust_boundary = diagnostic_trust_boundary(code.as_deref(), message);
    let mut explanation = serde_json::Map::new();

    if let Some(code) = code {
        explanation.insert("code".to_string(), json!(code));
    }
    explanation.insert("trust_boundary".to_string(), json!(trust_boundary));
    explanation.insert("severity".to_string(), json!(diagnostic_severity_label(diagnostic)));
    explanation.insert("summary".to_string(), json!(diagnostic_summary(message)));
    explanation.insert("reason".to_string(), json!(diagnostic_trust_reason(trust_boundary)));
    explanation
        .insert("why_diagnostic_fired".to_string(), json!(diagnostic_fired_reason(trust_boundary)));
    explanation.insert(
        "why_diagnostic_was_not_suppressed".to_string(),
        json!(diagnostic_not_suppressed_reason(trust_boundary)),
    );

    if trust_boundary == "module_resolution" {
        explanation.insert("module_resolution".to_string(), pl701_module_resolution(message));
    }

    Value::Object(explanation)
}

fn diagnostic_code(diagnostic: &Value) -> Option<String> {
    diagnostic
        .get("code")
        .and_then(|code| {
            code.as_str()
                .map(str::to_string)
                .or_else(|| code.as_i64().map(|value| value.to_string()))
        })
        .or_else(|| diagnostic.pointer("/data/code").and_then(Value::as_str).map(str::to_string))
}

fn diagnostic_trust_boundary(code: Option<&str>, message: &str) -> &'static str {
    if code == Some("PL701") {
        return "module_resolution";
    }

    let lower = message.to_ascii_lowercase();
    if lower.contains("dynamic boundary") || lower.contains("dynamic-boundary") {
        "dynamic_boundary"
    } else if lower.contains("low-confidence") || lower.contains("low confidence") {
        "low_confidence"
    } else if lower.contains("ambiguous") {
        "ambiguous_evidence"
    } else if lower.contains("stale") {
        "stale_fact"
    } else if lower.contains("generated")
        || lower.contains("no-source")
        || lower.contains("not source-backed")
    {
        "generated_or_not_source_backed"
    } else {
        "conservative_diagnostic"
    }
}

fn diagnostic_trust_reason(trust_boundary: &str) -> &'static str {
    match trust_boundary {
        "module_resolution" => {
            "Missing-module diagnostic; inspect the reported @INC search context and include-path policy."
        }
        "dynamic_boundary" => {
            "Dynamic Perl boundary prevents static confirmation, so the diagnostic stays conservative."
        }
        "low_confidence" => {
            "Low-confidence evidence is visible and does not silently suppress the diagnostic."
        }
        "ambiguous_evidence" => {
            "Ambiguous evidence is visible and does not silently suppress the diagnostic."
        }
        "stale_fact" => "Stale facts are not trusted as fresh diagnostic evidence.",
        "generated_or_not_source_backed" => {
            "Generated or non-source-backed evidence is not treated as an exact static fact."
        }
        _ => {
            "Diagnostic was returned by the live provider; no stronger trust boundary was inferred."
        }
    }
}

fn diagnostic_fired_reason(trust_boundary: &str) -> &'static str {
    match trust_boundary {
        "module_resolution" => {
            "The diagnostic provider returned missing-module evidence from the current module-resolution context."
        }
        "dynamic_boundary" => {
            "The diagnostic provider returned a finding at a dynamic Perl boundary."
        }
        "low_confidence" => {
            "The diagnostic provider returned a finding with low-confidence evidence."
        }
        "ambiguous_evidence" => {
            "The diagnostic provider returned a finding where evidence remained ambiguous."
        }
        "stale_fact" => "The diagnostic provider returned a finding with stale fact context.",
        "generated_or_not_source_backed" => {
            "The diagnostic provider returned a finding involving generated or non-source-backed evidence."
        }
        _ => "The diagnostic provider returned the finding for the current request.",
    }
}

fn diagnostic_not_suppressed_reason(trust_boundary: &str) -> &'static str {
    match trust_boundary {
        "module_resolution" => {
            "No trusted source-backed module fact suppressed the missing-module diagnostic."
        }
        "dynamic_boundary" => {
            "Dynamic evidence is labeled and does not suppress conservative diagnostics."
        }
        "low_confidence" => {
            "Low-confidence evidence is visible and does not silently suppress diagnostics."
        }
        "ambiguous_evidence" => {
            "Ambiguous evidence is visible and does not silently suppress diagnostics."
        }
        "stale_fact" => "Stale facts are not trusted to suppress diagnostics.",
        "generated_or_not_source_backed" => {
            "Generated or non-source-backed evidence is not trusted as an exact suppression fact."
        }
        _ => "No stronger trusted fact was available to suppress the returned diagnostic.",
    }
}

fn diagnostic_severity_label(diagnostic: &Value) -> &'static str {
    match diagnostic.get("severity").and_then(Value::as_u64) {
        Some(1) => "error",
        Some(2) => "warning",
        Some(3) => "information",
        Some(4) => "hint",
        _ => "unknown",
    }
}

fn diagnostic_summary(message: &str) -> String {
    message.lines().next().unwrap_or("").chars().take(200).collect()
}

fn pl701_module_resolution(message: &str) -> Value {
    let requested_module = extract_between(message, "Module '", "'");
    let expected_relative_path =
        requested_module.as_ref().map(|module| format!("{}.pm", module.replace("::", "/")));
    let reported_inc_paths = reported_inc_paths(message);
    let effective_include_paths_reported = !reported_inc_paths.is_empty();
    let workspace_include_paths_labeled = message.contains("workspace includePaths");
    let perl5lib_policy = if message.contains("PERL5LIB") {
        "reported_in_effective_search_context"
    } else {
        "not_reported_in_diagnostic_search_context"
    };

    json!({
        "requested_module": requested_module,
        "expected_relative_path": expected_relative_path,
        "reported_inc_paths": reported_inc_paths,
        "effective_include_paths_reported": effective_include_paths_reported,
        "workspace_include_paths_labeled": workspace_include_paths_labeled,
        "perl5lib_policy": perl5lib_policy,
        "searched_inc_reported": message.contains("Searched @INC"),
    })
}

fn extract_between(message: &str, prefix: &str, suffix: &str) -> Option<String> {
    let start = message.find(prefix)? + prefix.len();
    let rest = &message[start..];
    let end = rest.find(suffix)?;
    Some(rest[..end].to_string())
}

fn reported_inc_paths(message: &str) -> Vec<String> {
    let mut paths = Vec::new();
    for line in message.lines() {
        let trimmed = line.trim();
        if let Some(path) = trimmed.strip_prefix("- ") {
            paths.push(path.to_string());
        }
        if paths.len() >= MAX_REPORTED_INC_PATHS {
            return paths;
        }
    }

    if paths.is_empty() {
        if let Some(after_inc) = message.split_once("Searched @INC:").map(|(_, rest)| rest) {
            let before_suggestion =
                after_inc.split_once("Suggestion:").map(|(search, _)| search).unwrap_or(after_inc);
            let before_sentence = before_suggestion
                .split_once(". Add")
                .map(|(search, _)| search)
                .unwrap_or(before_suggestion);
            paths.extend(
                before_sentence
                    .split(',')
                    .map(str::trim)
                    .filter(|path| !path.is_empty())
                    .take(MAX_REPORTED_INC_PATHS)
                    .map(str::to_string),
            );
        }
    }

    paths
}

fn diagnostic_explanation_user_message(count: usize, explanations: &[Value]) -> String {
    if count == 0 {
        return "Diagnostics returned no findings for this request.".to_string();
    }

    let has_pl701 = explanations.iter().any(|explanation| {
        explanation.get("trust_boundary").and_then(Value::as_str) == Some("module_resolution")
    });
    if has_pl701 {
        return format!(
            "Diagnostics returned {count} item(s). PL701 includes missing-module @INC lookup context so users can distinguish code issues from include-path setup."
        );
    }

    let has_dynamic_or_low_confidence = explanations.iter().any(|explanation| {
        matches!(
            explanation.get("trust_boundary").and_then(Value::as_str),
            Some("dynamic_boundary" | "low_confidence" | "ambiguous_evidence")
        )
    });
    if has_dynamic_or_low_confidence {
        return format!(
            "Diagnostics returned {count} item(s). Dynamic, low-confidence, or ambiguous evidence is labeled and kept conservative."
        );
    }

    format!(
        "Diagnostics returned {count} item(s). This receipt explains the returned diagnostics without changing suppression or severity."
    )
}
