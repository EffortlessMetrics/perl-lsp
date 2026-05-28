//! Miscellaneous language feature handlers
//!
//! Handles various LSP features including:
//! - Inlay hints
//! - Document links
//! - Selection ranges
//! - Code lens
//! - Inline completion and values
//! - Linked editing ranges
//! - Test discovery
//! - Execute command

use super::super::*;
use crate::protocol::{invalid_params, req_position, req_uri};
use crate::state::{code_lens_cap, code_lens_resolve_deadline, inlay_hints_cap};
use perl_lsp_rs_core::providers::normalize_provider_decision_receipt;
use perl_parser_core::source_file::is_perl_source_uri;
use std::time::{Duration, Instant};

mod debug_launch;
mod inline_values;
mod live_provider_trace;
#[cfg(not(target_arch = "wasm32"))]
use debug_launch::debug_command_from_oracle;
use inline_values::inline_value_regex;
pub(super) use live_provider_trace::{
    DIAGNOSTIC_EXPLANATION_SCHEMA_VERSION, diagnostic_explanation_payload_from_diagnostics,
};
use live_provider_trace::{
    diagnostic_explanation_payload, live_provider_result_shape, live_provider_trace_key,
};

fn truncate_inlay_hint_label(hint: &mut Value, max_chars: usize) {
    let Some(label) = hint.get_mut("label") else {
        return;
    };
    let Some(text) = label.as_str() else {
        return;
    };
    if text.chars().count() <= max_chars {
        return;
    }

    *label = Value::String(text.chars().take(max_chars).collect());
}

#[derive(Debug, Clone)]
struct SelectedInlineCompletionInfo {
    range: lsp_types::Range,
    text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InlineCompletionTriggerKind {
    Invoked,
    Automatic,
    LegacyNoContext,
}

fn inline_completion_trigger_kind(
    params: &Value,
) -> Result<InlineCompletionTriggerKind, JsonRpcError> {
    match params.pointer("/context/triggerKind").and_then(Value::as_u64) {
        Some(1) => Ok(InlineCompletionTriggerKind::Invoked),
        Some(2) => Ok(InlineCompletionTriggerKind::Automatic),
        Some(_) => Err(invalid_params("Invalid inlineCompletion.context.triggerKind")),
        None => Ok(InlineCompletionTriggerKind::LegacyNoContext),
    }
}

fn selected_inline_completion_info(
    params: &Value,
) -> Result<Option<SelectedInlineCompletionInfo>, JsonRpcError> {
    let Some(selected) = params.pointer("/context/selectedCompletionInfo") else {
        return Ok(None);
    };

    let text = selected
        .get("text")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_params("Missing selectedCompletionInfo.text"))?
        .to_string();
    let range = selected
        .get("range")
        .ok_or_else(|| invalid_params("Missing selectedCompletionInfo.range"))
        .and_then(|range| {
            serde_json::from_value(range.clone())
                .map_err(|_| invalid_params("Invalid selectedCompletionInfo.range"))
        })?;

    Ok(Some(SelectedInlineCompletionInfo { range, text }))
}

fn constrain_inline_completions_to_selected_info(
    mut list: perl_lsp_rs_core::providers::inline_completion::InlineCompletionList,
    selected: Option<&SelectedInlineCompletionInfo>,
    line: u32,
    character: u32,
) -> perl_lsp_rs_core::providers::inline_completion::InlineCompletionList {
    let Some(selected) = selected else {
        return list;
    };

    if selected.range.start.line != selected.range.end.line {
        list.items.clear();
        return list;
    }

    let implicit_range = lsp_types::Range {
        start: lsp_types::Position::new(line, character),
        end: lsp_types::Position::new(line, character),
    };

    list.items = list
        .items
        .into_iter()
        .filter_map(|mut item| {
            if !item.insert_text.starts_with(&selected.text) {
                return None;
            }

            match &item.range {
                Some(range) if range == &selected.range => Some(item),
                Some(_) => None,
                None if selected.range == implicit_range => {
                    item.range = Some(selected.range);
                    Some(item)
                }
                None => None,
            }
        })
        .collect();
    list
}

fn apply_inline_completion_trigger_policy(
    mut list: perl_lsp_rs_core::providers::inline_completion::InlineCompletionList,
    trigger_kind: InlineCompletionTriggerKind,
) -> perl_lsp_rs_core::providers::inline_completion::InlineCompletionList {
    if trigger_kind == InlineCompletionTriggerKind::Automatic {
        list.items.truncate(1);
    }

    list
}

impl LspServer {
    pub(crate) fn record_provider_decision_trace(&self, provider: &str, receipt: &Value) {
        if receipt.is_object() {
            self.provider_decision_traces
                .lock()
                .insert(provider.to_string(), normalize_provider_decision_receipt(receipt.clone()));
        }
    }

    pub(crate) fn record_live_provider_decision_trace(
        &self,
        method: &str,
        result: &Result<Option<Value>, JsonRpcError>,
    ) {
        let Some(provider) = live_provider_trace_key(method) else {
            return;
        };
        if method == "textDocument/semanticTokens/full" && result.is_ok() {
            // The semantic-token full handler records a provider-specific trace that
            // distinguishes the source-backed compiler-token live slice from the
            // parser/HIR fallback. Do not replace it with the generic dispatcher
            // shape after the handler returns.
            return;
        }

        let shape = live_provider_result_shape(result);
        let mut receipt = serde_json::Map::new();
        receipt.insert("provider".to_string(), json!(provider));
        receipt.insert("provider_action".to_string(), json!(method));
        receipt.insert("decision".to_string(), json!(shape.decision));
        receipt.insert("reason".to_string(), json!(shape.reason));
        receipt.insert("fact_source".to_string(), json!("provider_runtime"));
        receipt.insert("confidence".to_string(), json!("low"));
        receipt.insert("freshness".to_string(), json!("fresh"));
        receipt.insert("source_backed".to_string(), json!(false));
        receipt.insert("source_backed_state".to_string(), json!("not_proven_by_dispatch_trace"));
        receipt.insert("dynamic_boundary".to_string(), json!(false));
        receipt.insert("fallback_state".to_string(), json!(shape.fallback_state));
        receipt.insert("live_provider_result_kind".to_string(), json!(shape.result_kind));
        receipt.insert(
            "live_provider_result_count".to_string(),
            json!(u64::try_from(shape.result_count).unwrap_or(u64::MAX)),
        );
        receipt.insert("trace_only_no_live_behavior_change".to_string(), json!(true));
        receipt.insert(
            "claim_boundary".to_string(),
            json!(
                "records live provider request shape only; compiler-fact trust remains gated by surface receipts"
            ),
        );
        if let Some((diagnostic_payload, user_message, has_dynamic_boundary)) =
            diagnostic_explanation_payload(method, result)
        {
            receipt.insert(
                "diagnostic_explanation_schema".to_string(),
                json!(DIAGNOSTIC_EXPLANATION_SCHEMA_VERSION),
            );
            receipt.insert("diagnostic_explanation".to_string(), diagnostic_payload);
            receipt.insert("user_message".to_string(), json!(user_message));
            if has_dynamic_boundary {
                receipt.insert("dynamic_boundary".to_string(), json!(true));
            }
            receipt.insert(
                "claim_boundary".to_string(),
                json!(
                    "records live diagnostic explanation payload only; no new suppression, severity, or support-tier promotion"
                ),
            );
        }
        if let Some(error) = shape.error {
            receipt.insert("provider_error".to_string(), error);
        }

        self.record_provider_decision_trace(provider, &Value::Object(receipt));
    }

    fn provider_decision_trace(&self, provider: &str) -> Option<Value> {
        self.provider_decision_traces.lock().get(provider).cloned()
    }

    fn attach_provider_decision_trace(&self, arguments: &mut [Value]) {
        let Some(request) = arguments.first_mut().and_then(Value::as_object_mut) else {
            return;
        };
        if request.contains_key("request_receipt") {
            return;
        }
        let Some(provider) = request.get("provider").and_then(Value::as_str).map(str::to_string)
        else {
            return;
        };
        if let Some(trace) = self.provider_decision_trace(&provider) {
            request.insert("request_receipt".to_string(), trace);
        }
    }

    /// Handle textDocument/inlayHint request
    pub(crate) fn handle_inlay_hints(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        use crate::protocol::req_range;

        // Return empty if client does not support inlay hints.
        if !self.client_capabilities.lock().inlay_hint_support {
            return Ok(Some(json!([])));
        }

        // Snapshot config once to avoid holding the lock across the hint generation.
        let (hints_enabled, param_hints, type_hints, max_label_length) = {
            let cfg = self.config.lock();
            (
                cfg.inlay_hints_enabled,
                cfg.inlay_hints_parameter_hints,
                cfg.inlay_hints_type_hints,
                cfg.inlay_hints_max_length,
            )
        };

        if !hints_enabled {
            return Ok(Some(json!([])));
        }

        let cap = inlay_hints_cap();

        if let Some(p) = params {
            let uri = req_uri(&p)?;

            // Extract the range parameter (required by LSP spec)
            // InlayHint range is required per spec, but we allow graceful degradation to full doc
            let range = if let Ok(((sl, sc), (el, ec))) = req_range(&p) {
                Some(perl_position_tracking::WireRange::new(
                    perl_position_tracking::WirePosition::new(sl, sc),
                    perl_position_tracking::WirePosition::new(el, ec),
                ))
            } else {
                None
            };

            let documents = self.documents_guard();
            let doc = self.get_document(&documents, uri).ok_or_else(|| JsonRpcError {
                code: INVALID_REQUEST,
                message: format!("Document not open: {}", uri),
                data: None,
            })?;
            if let Some(ref ast) = doc.ast {
                let mut hints = Vec::new();
                if param_hints {
                    hints.extend(crate::inlay_hints::parameter_hints(
                        ast,
                        &|off| self.offset_to_pos16(doc, off),
                        range,
                    ));
                }
                if type_hints {
                    hints.extend(crate::inlay_hints::trivial_type_hints(
                        ast,
                        &|off| self.offset_to_pos16(doc, off),
                        range,
                    ));
                }

                // Add URI to hint data for later resolution.
                // Merge with any existing data (e.g. functionName/paramIndex from
                // the hints provider) rather than overwriting it.
                let enriched_hints: Vec<Value> = hints
                    .iter()
                    .map(|hint| {
                        let mut h = hint.clone();
                        if let Some(obj) = h.as_object_mut() {
                            let data = obj.entry("data".to_string()).or_insert_with(|| json!({}));
                            if let Some(data_obj) = data.as_object_mut() {
                                data_obj.insert("uri".to_string(), json!(uri));
                            }
                        }
                        h
                    })
                    .collect();

                let mut result = enriched_hints;
                for hint in &mut result {
                    truncate_inlay_hint_label(hint, max_label_length);
                }

                // Apply cap to inlay hints.
                if result.len() > cap {
                    tracing::debug!(from = result.len(), to = cap, "InlayHints: capping");
                    result.truncate(cap);
                }
                return Ok(Some(json!(result)));
            }
        }
        Ok(Some(json!([])))
    }

    /// Handle inlayHint/resolve request
    ///
    /// Resolves deferred properties of an inlay hint, such as:
    /// - tooltip: detailed explanation of the hint
    /// - label.location: source location for the hint label
    /// - command: executable command associated with the hint
    ///
    /// This allows the initial inlayHint response to be fast and defer
    /// expensive computations until the user actually views the hint.
    pub(crate) fn handle_inlay_hint_resolve(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(mut hint) = params {
            // If hint already has both tooltip and labelDetails, return as-is
            if hint.get("tooltip").is_some() && hint.get("labelDetails").is_some() {
                return Ok(Some(hint));
            }

            // Extract hint properties for tooltip and label location generation
            let label = hint.get("label").and_then(|l| l.as_str()).unwrap_or("").to_string();
            let kind = hint.get("kind").and_then(|k| k.as_u64()).unwrap_or(0);

            // Add tooltip if not already present.
            // Prefer documentation summary from hint data (Phase 3);
            // fall back to generic tooltip generation.
            if hint.get("tooltip").is_none() {
                let tooltip = hint
                    .pointer("/data/docSummary")
                    .and_then(|v| v.as_str())
                    .map(String::from)
                    .or_else(|| {
                        // Check for deferred tooltip embedded in data
                        hint.pointer("/data/tooltip").and_then(|v| v.as_str()).map(String::from)
                    })
                    .unwrap_or_else(|| match kind {
                        1 => {
                            // Type hint
                            if label.contains("Str") {
                                "String value".to_string()
                            } else if label.contains("Num") {
                                "Numeric value".to_string()
                            } else if label.contains("Array") || label.contains("ARRAY") {
                                "Array reference".to_string()
                            } else if label.contains("Hash") || label.contains("HASH") {
                                "Hash reference".to_string()
                            } else if label.contains("Regex") {
                                "Regular expression".to_string()
                            } else if label.contains("CodeRef") {
                                "Code reference (anonymous subroutine)".to_string()
                            } else {
                                "Type annotation".to_string()
                            }
                        }
                        2 => {
                            let param_name = label.trim_end_matches(':').trim();
                            // Include the function name in the tooltip when available
                            let func = hint
                                .pointer("/data/functionName")
                                .and_then(|v| v.as_str())
                                .or_else(|| hint.pointer("/data/function").and_then(|v| v.as_str()))
                                .unwrap_or("unknown");
                            format!("{}() — parameter: {}", func, param_name)
                        }
                        _ => "Inlay hint".to_string(),
                    });
                if let Some(obj) = hint.as_object_mut() {
                    obj.insert("tooltip".to_string(), json!(tooltip));
                }
            }

            // Add labelDetails.location for parameter hints (kind=2) if not already present,
            // but only when the client declared "label.location" in resolveSupport.properties.
            let client_supports_label_location = self
                .client_capabilities
                .lock()
                .inlay_hint_resolve_support
                .as_ref()
                .map(|props| props.contains("label.location"))
                .unwrap_or(false);

            if hint.get("labelDetails").is_none() && kind == 2 && client_supports_label_location {
                if let Some(label_location) = self.resolve_hint_label_location(&hint) {
                    if let Some(obj) = hint.as_object_mut() {
                        obj.insert(
                            "labelDetails".to_string(),
                            json!({ "location": label_location }),
                        );
                    }
                }
            }

            Ok(Some(hint))
        } else {
            Err(invalid_params("Missing inlay hint parameter"))
        }
    }

    /// Resolve the LSP Location for an inlay hint label, enabling click-to-definition.
    ///
    /// Extracts the document URI and function name from the hint's `data` field,
    /// looks up the open document, walks the AST to find the subroutine definition,
    /// and converts its byte-offset location to an LSP `{ uri, range }` object.
    ///
    /// Returns `None` when the document is not open, the function is not found,
    /// or the hint data is missing required fields.
    fn resolve_hint_label_location(&self, hint: &Value) -> Option<Value> {
        let data = hint.get("data")?;
        let uri = data.get("uri").and_then(|u| u.as_str())?;
        let function_name = data
            .get("functionName")
            .and_then(|f| f.as_str())
            .or_else(|| data.get("function").and_then(|f| f.as_str()))?;
        let short_name = function_name.rsplit("::").next().unwrap_or(function_name);

        let documents = self.documents_guard();
        let doc = self.get_document(&documents, uri)?;
        let ast = doc.ast.as_ref()?;

        let sub_node = Self::find_subroutine_node(ast, function_name).or_else(|| {
            (short_name != function_name)
                .then(|| Self::find_subroutine_node(ast, short_name))
                .flatten()
        })?;
        let (start_line, start_char) = self.offset_to_pos16(doc, sub_node.location.start);
        let (end_line, end_char) = self.offset_to_pos16(doc, sub_node.location.end);

        Some(json!({
            "uri": uri,
            "range": {
                "start": { "line": start_line, "character": start_char },
                "end":   { "line": end_line,   "character": end_char   }
            }
        }))
    }

    /// Walk the AST to find a top-level subroutine node with the given name.
    fn find_subroutine_node<'a>(node: &'a Node, name: &str) -> Option<&'a Node> {
        if matches!(&node.kind, NodeKind::Subroutine { name: Some(sub_name), .. } if sub_name == name)
        {
            return Some(node);
        }

        let mut found = None;
        node.for_each_child(|child| {
            if found.is_none() {
                found = Self::find_subroutine_node(child, name);
            }
        });
        found
    }

    /// Handle textDocument/selectionRange request
    pub(crate) fn handle_selection_range(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(p) = params {
            let uri = req_uri(&p)?;
            let positions = p["positions"]
                .as_array()
                .ok_or_else(|| invalid_params("Missing required parameter: positions"))?;

            let documents = self.documents_guard();
            let doc = self.get_document(&documents, uri).ok_or_else(|| JsonRpcError {
                code: INVALID_REQUEST,
                message: format!("Document not open: {}", uri),
                data: None,
            })?;

            // Use the text-based provider so selection expansion still works for
            // hash access, strings, and function signatures even when the AST
            // hierarchy does not expose those intermediate ranges directly.
            let requested_positions: Vec<lsp_types::Position> = positions
                .iter()
                .map(|pos| {
                    let line =
                        pos["line"].as_u64().and_then(|v| u32::try_from(v).ok()).unwrap_or(0);
                    let col =
                        pos["character"].as_u64().and_then(|v| u32::try_from(v).ok()).unwrap_or(0);
                    lsp_types::Position::new(line, col)
                })
                .collect();

            let out = crate::features::lsp_selection_range::selection_ranges(
                &doc.text,
                &requested_positions,
            );
            Ok(Some(json!(out)))
        } else {
            Ok(Some(json!([])))
        }
    }

    /// Handle textDocument/codeLens request
    pub(crate) fn handle_code_lens(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        // Gate unadvertised feature
        if !self.advertised_features.lock().code_lens {
            return Err(crate::protocol::method_not_advertised());
        }

        if let Some(params) = params {
            let uri = req_uri(&params)?;
            let cap = code_lens_cap();
            let doc_snapshot = {
                let documents = self.documents_guard();
                self.get_document(&documents, uri).cloned()
            };

            if let Some(doc) = doc_snapshot {
                let start = Instant::now();
                let deadline = code_lens_resolve_deadline();
                if let Some(ref ast) = doc.ast {
                    let provider = CodeLensProvider::with_source(doc.text.clone())
                        .with_file_path(uri.to_string());
                    let mut lenses = provider.extract(ast);

                    // Add shebang lens if applicable
                    if let Some(shebang_lens) = get_shebang_lens(&doc.text) {
                        lenses.insert(0, shebang_lens);
                    }

                    // Apply cap to code lenses
                    if lenses.len() > cap {
                        tracing::debug!(from = lenses.len(), to = cap, "CodeLens: capping");
                        lenses.truncate(cap);
                    }

                    let lenses = self.prepare_code_lenses_for_client(lenses, start, deadline);
                    return Ok(Some(json!(lenses)));
                } else {
                    // Text-based fallback when AST is not available
                    let mut text_lenses = self.extract_text_based_code_lenses(&doc.text, uri);
                    // Add subtest lenses via text scanning (AST not available)
                    text_lenses.extend(CodeLensProvider::extract_subtest_lenses(&doc.text));
                    // Apply cap to text-based lenses
                    if text_lenses.len() > cap {
                        tracing::debug!(
                            from = text_lenses.len(),
                            to = cap,
                            "CodeLens (text): capping"
                        );
                        text_lenses.truncate(cap);
                    }
                    let text_lenses =
                        self.prepare_code_lenses_for_client(text_lenses, start, deadline);
                    return Ok(Some(json!(text_lenses)));
                }
            }
        }

        Ok(Some(json!([])))
    }

    fn client_supports_code_lens_command_resolve(&self) -> bool {
        self.client_capabilities
            .lock()
            .code_lens_resolve_support
            .as_ref()
            .is_some_and(|properties| properties.contains("command"))
    }

    fn prepare_code_lenses_for_client(
        &self,
        lenses: Vec<crate::code_lens_provider::CodeLens>,
        start: Instant,
        deadline: Duration,
    ) -> Vec<crate::code_lens_provider::CodeLens> {
        if self.client_supports_code_lens_command_resolve() {
            return lenses;
        }

        lenses
            .into_iter()
            .map(|lens| self.resolve_code_lens_for_client(lens, start, deadline))
            .collect()
    }

    fn resolve_code_lens_for_client(
        &self,
        lens: crate::code_lens_provider::CodeLens,
        start: Instant,
        deadline: Duration,
    ) -> crate::code_lens_provider::CodeLens {
        if lens.command.is_some() || lens.data.is_none() {
            return lens;
        }

        let symbol_name =
            lens.data.as_ref().and_then(|d| d.get("name")).and_then(|n| n.as_str()).unwrap_or("");
        let symbol_kind = lens
            .data
            .as_ref()
            .and_then(|d| d.get("kind"))
            .and_then(|k| k.as_str())
            .unwrap_or("unknown");
        let reference_count =
            self.count_code_lens_references(symbol_name, symbol_kind, start, deadline);
        resolve_code_lens(lens, reference_count)
    }

    fn count_code_lens_references(
        &self,
        symbol_name: &str,
        symbol_kind: &str,
        start: Instant,
        deadline: Duration,
    ) -> usize {
        #[cfg(feature = "workspace")]
        let index_count = self.coordinator().map(|coord| coord.index().count_usages(symbol_name));
        #[cfg(not(feature = "workspace"))]
        let index_count: Option<usize> = None;

        if let Some(count) = index_count {
            return count;
        }

        let snapshot = self.documents_scan_snapshot();
        let mut count = 0;
        for (scanned_docs, view) in snapshot.iter().enumerate() {
            if scanned_docs % 10 == 0 && start.elapsed() >= deadline {
                tracing::debug!(
                    scanned = scanned_docs,
                    count,
                    "CodeLensResolve: deadline exceeded, returning partial"
                );
                break;
            }

            if let Some(ref ast) = view.ast {
                count += self.count_references(ast, symbol_name, symbol_kind);
            } else {
                count += self.count_references_text_based(&view.text, symbol_name, symbol_kind);
            }
        }
        count
    }

    /// Handle codeLens/resolve request
    ///
    /// This implementation uses the snapshot pattern to minimize lock hold time.
    /// The documents lock is held only during the snapshot creation, then released
    /// before the CPU-intensive reference counting work begins.
    ///
    /// Includes deadline enforcement to prevent blocking on large workspaces.
    pub(crate) fn handle_code_lens_resolve(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        let start = Instant::now();
        let deadline = code_lens_resolve_deadline();

        if let Some(params) = params {
            // Parse the code lens
            if let Ok(lens) =
                serde_json::from_value::<crate::code_lens_provider::CodeLens>(params.clone())
            {
                // Extract the symbol name and kind from the lens data
                let symbol_name = lens
                    .data
                    .as_ref()
                    .and_then(|d| d.get("name"))
                    .and_then(|n| n.as_str())
                    .unwrap_or("");

                let symbol_kind = lens
                    .data
                    .as_ref()
                    .and_then(|d| d.get("kind"))
                    .and_then(|k| k.as_str())
                    .unwrap_or("unknown");

                let total_references =
                    self.count_code_lens_references(symbol_name, symbol_kind, start, deadline);

                let resolved = resolve_code_lens(lens, total_references);
                return Ok(Some(json!(resolved)));
            }
        }

        Err(JsonRpcError { code: -32602, message: "Invalid parameters".to_string(), data: None })
    }

    /// Handle textDocument/inlineCompletion request.
    ///
    /// When an AI backend is registered and AI completion is enabled in config,
    /// the handler tries the backend first. On failure or empty results, it
    /// falls back to deterministic completions (controlled by `ai_config.fallback`).
    pub(crate) fn handle_inline_completion(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        use crate::inline_completions::InlineCompletionProvider;

        if !self.advertised_features.lock().inline_completion {
            return Err(crate::protocol::method_not_advertised());
        }

        if let Some(params) = params {
            let uri = req_uri(&params)?;
            let (line, character) = req_position(&params)?;
            let trigger_kind = inline_completion_trigger_kind(&params)?;
            let selected_completion = selected_inline_completion_info(&params)?;

            // Snapshot text under document lock, then release before any slow work
            let text = {
                let documents = self.documents_guard();
                match self.get_document(&documents, uri) {
                    Some(doc) => doc.text.clone(),
                    None => {
                        return Ok(Some(json!({ "items": [] })));
                    }
                }
            };

            let provider = InlineCompletionProvider::new();

            // Try AI backend if enabled
            let ai_config = self.config.lock().ai_completion.clone();
            if ai_config.enabled {
                if let Some(context) = provider.prepare_context(&text, line, character) {
                    let backend_result = self.try_ai_inline_completion(&context, &ai_config);
                    match backend_result {
                        Ok(ref items) if !items.is_empty() => {
                            let list = perl_lsp_rs_core::providers::inline_completion::InlineCompletionList {
                                items: items.clone(),
                            };
                            let list = provider.apply_replacement_ranges_for_context(
                                list, &context, line, character,
                            );
                            let list = constrain_inline_completions_to_selected_info(
                                list,
                                selected_completion.as_ref(),
                                line,
                                character,
                            );
                            let list = apply_inline_completion_trigger_policy(list, trigger_kind);
                            if !list.items.is_empty() || !ai_config.fallback {
                                return Ok(Some(serde_json::to_value(list).map_err(|e| {
                                    crate::protocol::internal_error(&format!(
                                        "Failed to serialize inline completions: {}",
                                        e
                                    ))
                                })?));
                            }
                        }
                        Err(ref e) => {
                            tracing::debug!("AI inline completion failed: {}", e);
                            if !ai_config.fallback {
                                return Ok(Some(json!({ "items": [] })));
                            }
                            // Fall through to deterministic
                        }
                        _ => {
                            // Ok(empty) — fall through to deterministic if fallback enabled
                            if !ai_config.fallback {
                                return Ok(Some(json!({ "items": [] })));
                            }
                        }
                    }
                }
            }

            // Deterministic fallback
            let completions = constrain_inline_completions_to_selected_info(
                provider.get_inline_completions(&text, line, character),
                selected_completion.as_ref(),
                line,
                character,
            );
            let completions = apply_inline_completion_trigger_policy(completions, trigger_kind);
            return Ok(Some(serde_json::to_value(completions).map_err(|e| {
                crate::protocol::internal_error(&format!(
                    "Failed to serialize inline completions: {}",
                    e
                ))
            })?));
        }

        Ok(Some(json!({ "items": [] })))
    }

    /// Attempt AI-backed inline completion.
    ///
    /// Returns `Ok(items)` on success, `Err` on any failure.
    fn try_ai_inline_completion(
        &self,
        context: &perl_lsp_rs_core::providers::inline_completion::PreparedInlineCompletionContext,
        ai_config: &perl_lsp_rs_core::config::AiCompletionConfig,
    ) -> Result<
        Vec<perl_lsp_rs_core::providers::inline_completion::InlineCompletionItem>,
        perl_lsp_rs_core::providers::inline_completion::BackendError,
    > {
        // Get the backend from server state (if registered)
        let backend = self.ai_backend();
        let backend = match backend.as_ref() {
            Some(b) => b,
            None => return Ok(vec![]),
        };

        let req = perl_lsp_rs_core::providers::inline_completion::BackendRequest {
            context: context.clone(),
            max_output_tokens: ai_config.max_output_tokens,
            timeout_ms: ai_config.timeout_ms,
        };

        let texts = backend.complete(&req)?;
        let items = texts
            .into_iter()
            .map(|text| perl_lsp_rs_core::providers::inline_completion::InlineCompletionItem {
                insert_text: text,
                filter_text: None,
                range: None,
                command: None,
            })
            .collect();

        Ok(items)
    }

    /// Handle textDocument/inlineValue request
    ///
    /// Returns `InlineValueVariableLookup` items so the debug client resolves
    /// actual variable values via DAP, rather than displaying placeholder text.
    /// Supports scalar ($), array (@), and hash (%) variables.
    pub(crate) fn handle_inline_value(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        use crate::protocol::req_range;
        if let Some(params) = params {
            let uri = req_uri(&params)?;
            let ((start_line, _start_char), (end_line, _end_char)) = req_range(&params)?;

            // Use stoppedLocation from debug context to limit scope when available
            let context = &params["context"];
            let effective_end = context
                .get("stoppedLocation")
                .and_then(|loc| loc.get("end"))
                .and_then(|end| end.get("line"))
                .and_then(|l| l.as_u64())
                .and_then(|v| u32::try_from(v).ok())
                .map(|stopped_line| stopped_line.min(end_line))
                .unwrap_or(end_line);

            let documents = self.documents_guard();
            if let Some(doc) = self.get_document(&documents, uri) {
                use super::super::byte_to_utf16_col;

                let mut inline_values = Vec::new();

                let lines: Vec<&str> = doc.text.lines().collect();
                let Some(re) = inline_value_regex() else {
                    return Ok(Some(json!([])));
                };

                for line_num in start_line..=effective_end.min((lines.len() - 1) as u32) {
                    let line_text = lines[line_num as usize];

                    // Find $scalar, @array, and %hash variables
                    for cap in re.captures_iter(line_text) {
                        if let Some(m) = cap.get(0) {
                            let var_text = m.as_str();
                            // Convert byte positions to UTF-16 code units for LSP compliance
                            let start_utf16 = byte_to_utf16_col(line_text, m.start());
                            let end_utf16 = byte_to_utf16_col(line_text, m.end());

                            // Use InlineValueVariableLookup so the debug client resolves
                            // actual values via DAP rather than showing placeholder text
                            inline_values.push(json!({
                                "range": {
                                    "start": { "line": line_num, "character": start_utf16 as u32 },
                                    "end": { "line": line_num, "character": end_utf16 as u32 }
                                },
                                "variableName": var_text,
                                "caseSensitiveLookup": true
                            }));
                        }
                    }
                }

                return Ok(Some(json!(inline_values)));
            }
        }

        Ok(Some(json!([])))
    }

    /// Handle textDocument/documentColor request
    pub(crate) fn handle_document_color(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        // Gate unadvertised feature
        if !self.advertised_features.lock().document_color {
            return Err(crate::protocol::method_not_advertised());
        }

        let params = params.ok_or_else(|| invalid_params("Missing params"))?;
        let uri = req_uri(&params)?;

        let documents = self.documents_guard();
        let doc = self.get_document(&documents, uri).ok_or_else(|| JsonRpcError {
            code: -32602,
            message: format!("Document not found: {}", uri),
            data: None,
        })?;

        // Detect colors in the document text
        let color_infos = super::colors::detect_colors(&doc.text);

        // Convert to LSP format
        let lsp_colors: Vec<Value> = color_infos
            .iter()
            .map(|info| {
                json!({
                    "range": {
                        "start": {
                            "line": info.range.start.line,
                            "character": info.range.start.character
                        },
                        "end": {
                            "line": info.range.end.line,
                            "character": info.range.end.character
                        }
                    },
                    "color": {
                        "red": info.color.red,
                        "green": info.color.green,
                        "blue": info.color.blue,
                        "alpha": info.color.alpha
                    }
                })
            })
            .collect();

        Ok(Some(json!(lsp_colors)))
    }

    /// Handle textDocument/colorPresentation request
    pub(crate) fn handle_color_presentation(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        // Gate unadvertised feature
        if !self.advertised_features.lock().document_color {
            return Err(crate::protocol::method_not_advertised());
        }

        let params = params.ok_or_else(|| invalid_params("Missing params"))?;

        // Extract color from params
        let color_obj = params.get("color").ok_or_else(|| invalid_params("Missing color field"))?;

        let red = color_obj
            .get("red")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| invalid_params("Invalid red value"))?;
        let green = color_obj
            .get("green")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| invalid_params("Invalid green value"))?;
        let blue = color_obj
            .get("blue")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| invalid_params("Invalid blue value"))?;
        let alpha = color_obj.get("alpha").and_then(|v| v.as_f64()).unwrap_or(1.0);

        let color = super::colors::Color { red, green, blue, alpha };

        // Generate color presentations
        let presentations = super::colors::color_to_presentations(&color);

        Ok(Some(json!(presentations)))
    }

    /// Handle textDocument/linkedEditingRange request
    pub(crate) fn handle_linked_editing_range(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        // Gate unadvertised feature
        if !self.advertised_features.lock().linked_editing {
            return Err(crate::protocol::method_not_advertised());
        }

        if let Some(params) = params {
            let uri = req_uri(&params)?;
            let (line, character) = req_position(&params)?;

            let documents = self.documents_guard();
            if let Some(doc) = self.get_document(&documents, uri) {
                let result =
                    crate::linked_editing::handle_linked_editing(&doc.text, line, character);
                return Ok(Some(serde_json::to_value(result).map_err(|e| {
                    crate::protocol::internal_error(&format!(
                        "Failed to serialize linked editing ranges: {}",
                        e
                    ))
                })?));
            }
        }

        Ok(Some(Value::Null))
    }

    /// Handle test discovery request
    pub(crate) fn handle_test_discovery(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = req_uri(&params)?;

            tracing::debug!(uri, "Discovering tests");

            let documents = self.documents_guard();
            if let Some(doc) = self.get_document(&documents, uri) {
                if let Some(ref ast) = doc.ast {
                    let runner = TestRunner::new(doc.text.clone(), uri.to_string());
                    let tests = runner.discover_tests(ast);

                    // Convert test items to JSON
                    let test_items: Vec<Value> = tests
                        .into_iter()
                        .map(|test| {
                            json!({
                                "id": test.id,
                                "label": test.label,
                                "uri": test.uri,
                                "range": {
                                    "start": {
                                        "line": test.range.start_line,
                                        "character": test.range.start_character
                                    },
                                    "end": {
                                        "line": test.range.end_line,
                                        "character": test.range.end_character
                                    }
                                },
                                "kind": match test.kind {
                                    TestKind::File => "file",
                                    TestKind::Suite => "suite",
                                    TestKind::Test => "test"
                                },
                                "children": test.children.into_iter()
                                    .map(|child| json!({
                                        "id": child.id,
                                        "label": child.label,
                                        "uri": child.uri,
                                        "range": {
                                            "start": {
                                                "line": child.range.start_line,
                                                "character": child.range.start_character
                                            },
                                            "end": {
                                                "line": child.range.end_line,
                                                "character": child.range.end_character
                                            }
                                        },
                                        "kind": match child.kind {
                                            TestKind::File => "file",
                                            TestKind::Suite => "suite",
                                            TestKind::Test => "test"
                                        },
                                        "children": []
                                    }))
                                    .collect::<Vec<_>>()
                            })
                        })
                        .collect();

                    tracing::debug!(count = test_items.len(), "Found test items");

                    return Ok(Some(json!(test_items)));
                }
            }
        }

        Ok(Some(json!([])))
    }

    /// Handle execute command request
    pub(crate) fn handle_execute_command(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        use crate::execute_command::ExecuteCommandProvider;

        if let Some(params) = params {
            let command = params["command"]
                .as_str()
                .ok_or_else(|| invalid_params("Missing required parameter: command"))?;

            // LSP 3.17 compliance: arguments field is required even if empty
            if !params.as_object().unwrap_or(&serde_json::Map::new()).contains_key("arguments") {
                return Err(JsonRpcError {
                    code: -32602, // InvalidParams
                    message: "Missing required 'arguments' field in executeCommand request"
                        .to_string(),
                    data: Some(json!({
                        "command": command,
                        "errorType": "executeCommand",
                        "originalError": "Missing 'arguments' field"
                    })),
                });
            }

            let mut arguments = params["arguments"].as_array().cloned().unwrap_or_default();
            if command == "perl.explainProviderDecision" {
                self.attach_provider_decision_trace(&mut arguments);
            }

            tracing::debug!(command, "Executing command");

            // Use the new execute command provider for new commands
            // Collect workspace roots, deduplicating to avoid redundant security checks
            let mut workspace_roots = Vec::new();

            // Add legacy root path if available
            if let Some(root_path) = self.root_path.lock().clone() {
                workspace_roots.push(root_path);
            }

            // Add workspace folders (deduplicate against already added paths)
            {
                let folders = self.workspace_folders.lock();
                for folder in folders.iter() {
                    if let Ok(parsed) = url::Url::parse(&folder.uri) {
                        if let Ok(path) = parsed.to_file_path() {
                            if !workspace_roots.contains(&path) {
                                workspace_roots.push(path);
                            }
                        }
                    }
                }
            }

            let provider = ExecuteCommandProvider::with_workspace_roots(workspace_roots)
                .with_workspace_config(self.workspace_config.lock().clone());

            match command {
                // Keep existing test commands for backward compatibility
                "perl.runTest" => {
                    if let Some(test_id) = arguments.first().and_then(|v| v.as_str()) {
                        return self.run_test(test_id);
                    }
                }
                "perl.runTestFile" => {
                    if let Some(file_uri) = arguments.first().and_then(|v| v.as_str()) {
                        return self.run_test_file(file_uri);
                    }
                }
                "perl.runSubtest" => {
                    let subtest_name = arguments
                        .first()
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| invalid_params("Missing subtest name argument"))?;
                    return self.run_subtest(subtest_name);
                }
                "perl.workspaceTrustReport" => {
                    return self.workspace_trust_report(arguments.first());
                }
                "perl.previewSafeDelete" => {
                    let request = arguments
                        .first()
                        .cloned()
                        .ok_or_else(|| invalid_params("Missing safe-delete preview argument"))?;
                    return self.safe_delete_symbol_preview(Some(request));
                }
                "perl.safeDeleteSymbol" => {
                    let request = arguments
                        .first()
                        .cloned()
                        .ok_or_else(|| invalid_params("Missing safe-delete symbol argument"))?;
                    return self.safe_delete_symbol_live_pilot(Some(request));
                }
                "perl.previewPackageRename" => {
                    let request = arguments
                        .first()
                        .cloned()
                        .ok_or_else(|| invalid_params("Missing package rename preview argument"))?;
                    return self.package_rename_preview(Some(request));
                }
                "perl.explainMissingModuleLookup" => {
                    let request = arguments
                        .first()
                        .cloned()
                        .ok_or_else(|| invalid_params("Missing missing-module lookup argument"))?;
                    return self.explain_missing_module_lookup(Some(request));
                }
                "perl.debugTest" => {
                    let test_id = arguments
                        .first()
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| invalid_params("Missing test ID argument"))?;
                    return self.debug_test(test_id);
                }
                // Commands handled by ExecuteCommandProvider
                "perl.runTests"
                | "perl.runFile"
                | "perl.runTestSub"
                | "perl.runCritic"
                | "perl.goToTest"
                | "perl.goToImplementation"
                | "perl.debugTests"
                | "perl.explainProviderDecision" => {
                    match provider.execute_command(command, arguments) {
                        Ok(result) => return Ok(Some(result)),
                        Err(e) => {
                            // Return proper JSON-RPC error according to LSP 3.17 specification
                            let error_code = if e.contains("Missing") || e.contains("argument") {
                                -32602 // InvalidParams
                            } else if e.contains("Unknown command") {
                                -32601 // MethodNotFound
                            } else if e.contains("Path traversal") || e.contains("security") {
                                -32603 // InternalError (security)
                            } else {
                                -32603 // InternalError (general)
                            };

                            return Err(JsonRpcError {
                                code: error_code,
                                message: format!("Execute command failed: {}", e),
                                data: Some(json!({
                                    "command": command,
                                    "errorType": "executeCommand",
                                    "originalError": e
                                })),
                            });
                        }
                    }
                }
                // Debug file: validate path and launch perl -d
                "perl.debugFile" => {
                    let file_path =
                        arguments.first().and_then(|v| v.as_str()).ok_or_else(|| {
                            invalid_params("Missing file path argument for perl.debugFile")
                        })?;

                    // Validate file extension
                    if !is_perl_source_uri(file_path) {
                        return Err(JsonRpcError {
                            code: -32602,
                            message: "File must have a Perl extension (.pl, .pm, .t, .psgi)"
                                .to_string(),
                            data: Some(json!({"file": file_path})),
                        });
                    }

                    // Security: use the same workspace-rooted path resolution
                    let resolved =
                        provider.resolve_debug_file_path(file_path).map_err(|e| JsonRpcError {
                            code: -32603,
                            message: format!("Path validation failed: {}", e),
                            data: Some(json!({"file": file_path})),
                        })?;

                    // Strip \\?\ extended-length prefix so perl.exe can accept the path.
                    // resolve_debug_file_path calls canonicalize() which on Windows returns
                    // paths with the \\?\ prefix that external programs cannot handle.
                    let ext_resolved =
                        crate::execute_command::normalize_path_for_external_command(&resolved);

                    // Launch perl -d as a detached child process.
                    // PerlOracleEnv enforces a deny-all-ambient policy: PERL5OPT and
                    // local::lib are stripped; PERL5LIB passes through only when the
                    // user has explicitly opted in via `usePerl5lib` config.
                    #[cfg(not(target_arch = "wasm32"))]
                    let mut debug_cmd = {
                        use perl_lsp_rs_core::config::PerlOracleEnv;
                        let debug_cwd = resolved
                            .parent()
                            .map(std::path::Path::to_path_buf)
                            .unwrap_or_else(|| {
                                std::env::current_dir()
                                    .unwrap_or_else(|_| std::path::PathBuf::from("."))
                            });
                        let debug_config = self.workspace_config.lock().clone();
                        debug_command_from_oracle(
                            PerlOracleEnv::for_language_probe(&debug_config, debug_cwd),
                            &resolved,
                        )?
                    };
                    #[cfg(target_arch = "wasm32")]
                    return Err(debug_launch::unresolved_debug_perl_error(&resolved));
                    match debug_cmd
                        .arg("-d")
                        .arg("--")
                        .arg(&ext_resolved)
                        .stdin(std::process::Stdio::null())
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .spawn()
                    {
                        Ok(child) => {
                            let pid = child.id();
                            tracing::info!(file = %resolved.display(), pid, "Debug session started");
                            return Ok(Some(json!({
                                "status": "started",
                                "pid": pid,
                                "file": resolved.display().to_string()
                            })));
                        }
                        Err(e) => {
                            return Err(JsonRpcError {
                                code: -32603,
                                message: format!(
                                    "Cannot start Perl debugger for '{}': {}. \
                                     Check that 'perl' is on your PATH and that the file exists.",
                                    resolved.display(),
                                    e
                                ),
                                data: Some(json!({"file": resolved.display().to_string()})),
                            });
                        }
                    }
                }
                _ => {
                    return Err(JsonRpcError {
                        code: METHOD_NOT_FOUND,
                        message: format!("Unknown command: {}", command),
                        data: None,
                    });
                }
            }
        }

        // Missing params entirely
        Err(JsonRpcError {
            code: -32602, // InvalidParams
            message: "Missing parameters for executeCommand request".to_string(),
            data: Some(json!({
                "errorType": "executeCommand",
                "originalError": "Missing params"
            })),
        })
    }

    /// Count references to a symbol using text-based search
    pub(crate) fn count_references_text_based(
        &self,
        text: &str,
        symbol_name: &str,
        symbol_kind: &str,
    ) -> usize {
        let mut count = 0;

        match symbol_kind {
            "package" => {
                // Count package usage (use statements, new() calls, etc.)
                use regex::Regex;

                // Count "use PackageName" statements
                if let Ok(use_regex) =
                    Regex::new(&format!(r"\buse\s+{}\b", regex::escape(symbol_name)))
                {
                    count += use_regex.find_iter(text).count();
                }

                // Count "PackageName->new()" or "PackageName->method()" calls
                if let Ok(call_regex) = Regex::new(&format!(r"\b{}->", regex::escape(symbol_name)))
                {
                    count += call_regex.find_iter(text).count();
                }

                // Count "bless ... PackageName" statements
                if let Ok(bless_regex) =
                    Regex::new(&format!(r"bless\s+.*?,\s*{}", regex::escape(symbol_name)))
                {
                    count += bless_regex.find_iter(text).count();
                }
            }
            "subroutine" => {
                // Count function calls
                use regex::Regex;

                // Count "function_name(" calls
                if let Ok(call_regex) =
                    Regex::new(&format!(r"\b{}\s*\(", regex::escape(symbol_name)))
                {
                    count += call_regex.find_iter(text).count();
                }

                // Count "&function_name" references
                if let Ok(ref_regex) = Regex::new(&format!(r"&{}\b", regex::escape(symbol_name))) {
                    count += ref_regex.find_iter(text).count();
                }
            }
            _ => {
                // Generic search
                use regex::Regex;
                if let Ok(re) = Regex::new(&format!(r"\b{}\b", regex::escape(symbol_name))) {
                    count += re.find_iter(text).count();
                }
            }
        }

        count
    }

    /// Get workspace roots from initialization
    pub(crate) fn workspace_roots(&self) -> Vec<url::Url> {
        let mut results = Vec::new();

        if let Some(ref path) = *self.root_path.lock() {
            if let Ok(url) = url::Url::from_file_path(path) {
                results.push(url);
            }
        }

        {
            let folders = self.workspace_folders.lock();
            for folder in folders.iter() {
                if let Ok(parsed) = url::Url::parse(&folder.uri) {
                    if !results.contains(&parsed) {
                        results.push(parsed);
                    }
                }
            }
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use crate::LspServer;
    use crate::state::ClientCapabilities;
    use serde_json::json;
    use std::collections::HashSet;
    use std::io::Cursor;

    /// Build a minimal test server with custom capabilities applied.
    fn make_server_with_caps(caps: ClientCapabilities) -> LspServer {
        let server =
            LspServer::with_io(Box::new(Cursor::new(Vec::<u8>::new())), Box::new(Vec::<u8>::new()));
        *server.client_capabilities.lock() = caps;
        server
    }

    /// When the client declares "label.location" in resolveSupport.properties,
    /// handle_inlay_hint_resolve must include labelDetails in the response for
    /// a parameter hint (kind=2) that has no function data to resolve.
    ///
    /// In this test the hint has no `data.functionName` so `resolve_hint_label_location`
    /// returns None — but the important thing is that the code path is entered
    /// (i.e. no labelDetails are injected when there is nothing to look up, and
    /// no panic occurs).
    #[test]
    fn inlay_hint_resolve_label_location_requires_client_capability() {
        // Hint without client capability: labelDetails must NOT be added
        let server_no_cap = make_server_with_caps(ClientCapabilities {
            inlay_hint_resolve_support: None,
            ..ClientCapabilities::default()
        });
        let hint = json!({
            "label": "$self:",
            "kind": 2,
            "position": { "line": 0, "character": 0 },
            "data": { "uri": "file:///fake.pl" }
        });
        let result = server_no_cap
            .handle_inlay_hint_resolve(Some(hint.clone()))
            .expect("resolve must not error");
        let resolved = result.expect("must return Some");
        assert!(
            resolved.get("labelDetails").is_none(),
            "labelDetails must be absent when client did not declare resolve support"
        );

        // Hint with client capability for a different property: labelDetails must NOT be added
        let mut other_props = HashSet::new();
        other_props.insert("tooltip".to_string());
        let server_other_prop = make_server_with_caps(ClientCapabilities {
            inlay_hint_resolve_support: Some(other_props),
            ..ClientCapabilities::default()
        });
        let result2 = server_other_prop
            .handle_inlay_hint_resolve(Some(hint.clone()))
            .expect("resolve must not error");
        let resolved2 = result2.expect("must return Some");
        assert!(
            resolved2.get("labelDetails").is_none(),
            "labelDetails must be absent when client only declared 'tooltip' resolve support"
        );

        // Hint with client capability declaring "label.location": the resolver attempts
        // label location lookup.  With no open document the lookup returns None so
        // labelDetails is still absent — but no panic or error must occur.
        let mut location_props = HashSet::new();
        location_props.insert("label.location".to_string());
        let server_with_cap = make_server_with_caps(ClientCapabilities {
            inlay_hint_resolve_support: Some(location_props),
            ..ClientCapabilities::default()
        });
        let result3 = server_with_cap
            .handle_inlay_hint_resolve(Some(hint))
            .expect("resolve must not error when client declares label.location");
        let resolved3 = result3.expect("must return Some");
        // Document is not open so resolve_hint_label_location returns None — labelDetails absent
        assert!(
            resolved3.get("labelDetails").is_none(),
            "labelDetails must be absent when document is not open (no sub found)"
        );
        // Tooltip must still be filled in regardless of label.location capability
        assert!(
            resolved3.get("tooltip").is_some(),
            "tooltip must be resolved regardless of label.location capability"
        );
    }

    /// Verify that the initialize handler parses resolveSupport.properties correctly.
    #[test]
    fn initialize_parses_inlay_hint_resolve_support_properties() {
        let server =
            LspServer::with_io(Box::new(Cursor::new(Vec::<u8>::new())), Box::new(Vec::<u8>::new()));

        let params = json!({
            "capabilities": {
                "textDocument": {
                    "inlayHint": {
                        "resolveSupport": {
                            "properties": ["label.location", "tooltip"]
                        }
                    }
                }
            }
        });

        server.handle_initialize(Some(params)).expect("initialize must not error");

        let caps = server.client_capabilities.lock();
        let props = caps
            .inlay_hint_resolve_support
            .as_ref()
            .expect("inlay_hint_resolve_support must be Some after initialize with resolveSupport");
        assert!(props.contains("label.location"), "must contain 'label.location'");
        assert!(props.contains("tooltip"), "must contain 'tooltip'");
    }

    /// When the client sends no resolveSupport entry, inlay_hint_resolve_support is None.
    #[test]
    fn initialize_no_resolve_support_leaves_field_none() {
        let server =
            LspServer::with_io(Box::new(Cursor::new(Vec::<u8>::new())), Box::new(Vec::<u8>::new()));

        let params = json!({
            "capabilities": {
                "textDocument": {
                    "inlayHint": {}
                }
            }
        });

        server.handle_initialize(Some(params)).expect("initialize must not error");

        let caps = server.client_capabilities.lock();
        assert!(
            caps.inlay_hint_resolve_support.is_none(),
            "inlay_hint_resolve_support must remain None when client sends no resolveSupport"
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn debug_command_from_oracle_rejects_missing_oracle() -> Result<(), Box<dyn std::error::Error>>
    {
        let resolved = std::path::PathBuf::from("script.pl");
        let err = super::debug_command_from_oracle(None, &resolved).err().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "expected missing Perl oracle to reject debug launch",
            )
        })?;

        assert_eq!(err.code, -32603);
        assert!(
            err.message.contains("refusing ambient fallback"),
            "debugFile should fail closed instead of falling back to ambient perl: {}",
            err.message
        );
        Ok(())
    }
}
