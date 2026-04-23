//! Miscellaneous language feature handlers
//!
//! Handles various LSP features including:
//! - Inlay hints
//! - Selection ranges
//! - Code lens
//! - Inline completion and values
//! - Monikers
//! - Linked editing ranges
//! - Test discovery
//! - Execute command

use super::super::*;
use crate::protocol::{invalid_params, req_position, req_uri};
use crate::state::{code_lens_cap, code_lens_resolve_deadline, inlay_hints_cap};
use perl_module::import::resolve_known_export_tag;
use perl_parser_core::source_file::is_perl_source_uri;
use std::sync::OnceLock;
use std::time::Instant;

static INLINE_VALUE_REGEX: OnceLock<Result<regex::Regex, regex::Error>> = OnceLock::new();

fn get_inline_value_regex() -> Option<&'static regex::Regex> {
    INLINE_VALUE_REGEX
        .get_or_init(|| regex::Regex::new(r"([$@%])([a-zA-Z_][a-zA-Z0-9_]*)"))
        .as_ref()
        .ok()
}

impl LspServer {
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
                hints.extend(crate::inlay_hints::parameter_hints(
                    ast,
                    &|off| self.offset_to_pos16(doc, off),
                    range,
                ));
                hints.extend(crate::inlay_hints::trivial_type_hints(
                    ast,
                    &|off| self.offset_to_pos16(doc, off),
                    range,
                ));

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

                // Apply cap to inlay hints
                let mut result = enriched_hints;
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

        let cap = code_lens_cap();

        if let Some(params) = params {
            let uri = req_uri(&params)?;

            let documents = self.documents_guard();
            if let Some(doc) = self.get_document(&documents, uri) {
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
                    return Ok(Some(json!(text_lenses)));
                }
            }
        }

        Ok(Some(json!([])))
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

                // Fast path: use workspace index if available (more accurate,
                // excludes references in comments/strings)
                #[cfg(feature = "workspace")]
                let index_count =
                    self.coordinator().map(|coord| coord.index().count_usages(symbol_name));
                #[cfg(not(feature = "workspace"))]
                let index_count: Option<usize> = None;

                let total_references = if let Some(count) = index_count {
                    count
                } else {
                    // Slow path: scan all documents with AST/text fallback
                    let snapshot = self.documents_scan_snapshot();
                    let mut count = 0;
                    for (scanned_docs, view) in snapshot.iter().enumerate() {
                        // Check deadline periodically (every 10 documents)
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
                            count += self.count_references_text_based(
                                &view.text,
                                symbol_name,
                                symbol_kind,
                            );
                        }
                    }
                    count
                };

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

        if let Some(params) = params {
            let uri = req_uri(&params)?;
            let (line, character) = req_position(&params)?;

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
                            return Ok(Some(serde_json::to_value(list).map_err(|e| {
                                crate::protocol::internal_error(&format!(
                                    "Failed to serialize inline completions: {}",
                                    e
                                ))
                            })?));
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
            let completions = provider.get_inline_completions(&text, line, character);
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
                let Some(re) = get_inline_value_regex() else {
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

    /// Handle textDocument/moniker request
    ///
    /// Generates stable symbol identifiers for cross-project symbol linking.
    /// Supports:
    /// - Exported symbols (kind="export") for symbols in @EXPORT or @EXPORT_OK
    /// - Imported symbols (kind="import") for symbols from use statements
    /// - Local symbols with appropriate uniqueness classification
    /// - Multiple monikers for aliased symbols
    pub(crate) fn handle_moniker(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = req_uri(&params)?;
            let (line, character) = req_position(&params)?;

            let documents = self.documents_guard();
            if let Some(doc) = self.get_document(&documents, uri) {
                if let Some(ref ast) = doc.ast {
                    let offset = self.pos16_to_offset(doc, line, character);

                    // Find the symbol at the cursor position
                    let current_pkg = crate::declaration::current_package_at(ast, offset);
                    if let Some(key) =
                        crate::declaration::symbol_at_cursor(ast, offset, current_pkg)
                    {
                        let mut monikers = Vec::new();

                        // Determine moniker properties based on symbol context
                        let (kind, unique) = self.classify_moniker(ast, &doc.text, &key);

                        // Generate fully qualified identifier
                        let qualified_id = format!("{}::{}", key.pkg, key.name).replace("::", ".");

                        // Primary moniker with full qualification
                        monikers.push(json!({
                            "scheme": "perl",
                            "identifier": qualified_id,
                            "unique": unique,
                            "kind": kind
                        }));

                        // For imported symbols, also add a moniker pointing to the source
                        if kind == "import" {
                            if let Some(source_pkg) = self.find_import_source(ast, &key.name) {
                                let source_id =
                                    format!("{}.{}", source_pkg.replace("::", "."), key.name);
                                monikers.push(json!({
                                    "scheme": "perl",
                                    "identifier": source_id,
                                    "unique": "global",
                                    "kind": "export"
                                }));
                            }
                        }

                        // For package-scoped variables (our), add a bare name alias
                        if key.sigil.is_some() && unique != "document" {
                            let sigil = key.sigil.unwrap_or('$');
                            let bare_id = format!("{}{}", sigil, key.name);
                            monikers.push(json!({
                                "scheme": "perl",
                                "identifier": bare_id,
                                "unique": "document",
                                "kind": "local"
                            }));
                        }

                        // For subroutines in packages with base/parent,
                        // add monikers pointing to potential parent definitions
                        if key.kind == crate::workspace_index::SymKind::Sub {
                            for parent_pkg in Self::find_base_parents(ast) {
                                let parent_id =
                                    format!("{}.{}", parent_pkg.replace("::", "."), key.name);
                                monikers.push(json!({
                                    "scheme": "perl",
                                    "identifier": parent_id,
                                    "unique": "global",
                                    "kind": "local"
                                }));
                            }
                        }

                        return Ok(Some(json!(monikers)));
                    }
                }
            }
        }

        Ok(Some(json!([])))
    }

    /// Classify a symbol's moniker kind and uniqueness
    fn classify_moniker(
        &self,
        ast: &crate::ast::Node,
        text: &str,
        key: &crate::workspace_index::SymbolKey,
    ) -> (&'static str, &'static str) {
        // Check if symbol is exported via @EXPORT or @EXPORT_OK (AST-first, regex fallback)
        let uses_exporter = Self::has_use_exporter(ast);
        let is_exported =
            self.is_symbol_exported_ast(ast, &key.name) || self.is_symbol_exported(text, &key.name);

        // Check if symbol is imported from another module
        let is_imported = self.is_symbol_imported(ast, &key.name);

        // Determine kind
        let kind = if is_exported {
            "export"
        } else if is_imported {
            "import"
        } else {
            "local"
        };

        // Determine uniqueness
        let unique = match key.kind {
            crate::workspace_index::SymKind::Pack => "global",
            crate::workspace_index::SymKind::Sub => {
                if is_exported {
                    "global"
                } else if uses_exporter && key.pkg.as_ref() != "main" {
                    // Module uses Exporter — subs are at least project-visible
                    "project"
                } else if key.pkg.as_ref() != "main" {
                    "project"
                } else {
                    "document"
                }
            }
            crate::workspace_index::SymKind::Var => {
                if self.is_our_variable(ast, &key.name, key.sigil) { "project" } else { "document" }
            }
        };

        (kind, unique)
    }

    /// Check if the AST contains `use Exporter` (or `use parent 'Exporter'`)
    fn has_use_exporter(ast: &crate::ast::Node) -> bool {
        use perl_parser::ast::NodeKind;

        fn check(node: &crate::ast::Node) -> bool {
            match &node.kind {
                NodeKind::Use { module, .. } if module == "Exporter" => true,
                NodeKind::Program { statements } | NodeKind::Block { statements } => {
                    statements.iter().any(check)
                }
                _ => false,
            }
        }
        check(ast)
    }

    /// AST-based export detection: walk Assignment nodes to find
    /// `@EXPORT = (...)` or `@EXPORT_OK = (...)` containing the symbol.
    fn is_symbol_exported_ast(&self, ast: &crate::ast::Node, symbol_name: &str) -> bool {
        use perl_parser::ast::NodeKind;

        fn check(node: &crate::ast::Node, name: &str) -> bool {
            match &node.kind {
                NodeKind::Assignment { lhs, rhs, .. } => {
                    // Check if lhs is @EXPORT or @EXPORT_OK
                    let is_export_var = match &lhs.kind {
                        NodeKind::Variable { name: var_name, sigil } => {
                            sigil.starts_with('@')
                                && (var_name == "EXPORT" || var_name == "EXPORT_OK")
                        }
                        _ => false,
                    };
                    if is_export_var {
                        // Search rhs for the symbol name in string/identifier nodes
                        return contains_symbol_name(rhs, name);
                    }
                    // Recurse into lhs/rhs for nested assignments
                    check(lhs, name) || check(rhs, name)
                }
                NodeKind::Program { statements } | NodeKind::Block { statements } => {
                    statements.iter().any(|s| check(s, name))
                }
                NodeKind::Subroutine { body, .. } => check(body, name),
                NodeKind::ExpressionStatement { expression } => check(expression, name),
                _ => false,
            }
        }

        fn contains_symbol_name(node: &crate::ast::Node, name: &str) -> bool {
            match &node.kind {
                NodeKind::String { value, .. } => {
                    // Check if the string contains the symbol name as a word
                    value.split_whitespace().any(|w| w == name)
                }
                NodeKind::Identifier { name: id } => id == name,
                NodeKind::ArrayLiteral { elements } => {
                    elements.iter().any(|e| contains_symbol_name(e, name))
                }
                _ => {
                    let mut found = false;
                    node.for_each_child(|child| {
                        if !found && contains_symbol_name(child, name) {
                            found = true;
                        }
                    });
                    found
                }
            }
        }

        check(ast, symbol_name)
    }

    /// Detect `use base 'Foo'` or `use parent 'Foo'` and return parent packages
    fn find_base_parents(ast: &crate::ast::Node) -> Vec<String> {
        use perl_parser::ast::NodeKind;

        fn collect(node: &crate::ast::Node, out: &mut Vec<String>) {
            match &node.kind {
                NodeKind::Use { module, args, .. } if module == "base" || module == "parent" => {
                    for arg in args {
                        // Handle qw(...) style: "qw(Foo::Bar Baz::Qux)"
                        if arg.starts_with("qw") {
                            let content = arg
                                .trim_start_matches("qw")
                                .trim_start_matches(|c: char| "([{/<|!".contains(c))
                                .trim_end_matches(|c: char| ")]}/|!>".contains(c));
                            for parent in content.split_whitespace() {
                                if !parent.is_empty() {
                                    out.push(parent.to_string());
                                }
                            }
                        } else if !arg.starts_with('-') && !arg.starts_with("qw") {
                            // Bare string arg: use base 'Foo::Bar'
                            let cleaned = arg.trim_matches(|c: char| c == '\'' || c == '"');
                            if !cleaned.is_empty() {
                                out.push(cleaned.to_string());
                            }
                        }
                    }
                }
                NodeKind::Program { statements } | NodeKind::Block { statements } => {
                    for stmt in statements {
                        collect(stmt, out);
                    }
                }
                _ => {}
            }
        }

        let mut parents = Vec::new();
        collect(ast, &mut parents);
        parents
    }

    /// Check if a symbol name appears in @EXPORT or @EXPORT_OK (regex fallback)
    fn is_symbol_exported(&self, text: &str, symbol_name: &str) -> bool {
        use std::sync::OnceLock;

        static EXPORT_QW_RE: OnceLock<Option<regex::Regex>> = OnceLock::new();
        static EXPORT_ARRAY_RE: OnceLock<Option<regex::Regex>> = OnceLock::new();

        let export_re = EXPORT_QW_RE.get_or_init(|| {
            regex::Regex::new(r"@EXPORT(?:_OK)?\s*=\s*qw[(\[{/<|!]([^)\]}/|!>]+)[)\]}/|!>]").ok()
        });

        if let Some(re) = export_re {
            for cap in re.captures_iter(text) {
                if let Some(content) = cap.get(1) {
                    if content.as_str().split_whitespace().any(|w| w == symbol_name) {
                        return true;
                    }
                }
            }
        }

        let array_re = EXPORT_ARRAY_RE
            .get_or_init(|| regex::Regex::new(r"@EXPORT(?:_OK)?\s*=\s*\(([^)]+)\)").ok());
        if let Some(re) = array_re {
            for cap in re.captures_iter(text) {
                if let Some(content) = cap.get(1) {
                    let c = content.as_str();
                    if c.contains(&format!("'{}'", symbol_name))
                        || c.contains(&format!("\"{}\"", symbol_name))
                    {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Check if a symbol is imported from another module
    fn is_symbol_imported(&self, ast: &crate::ast::Node, symbol_name: &str) -> bool {
        self.find_import_source(ast, symbol_name).is_some()
    }

    /// Find the source module for an imported symbol
    ///
    /// Searches `use` statements for the symbol name, handling both bare imports
    /// and `qw<...>` style import lists with all delimiter types.
    fn find_import_source(&self, ast: &crate::ast::Node, symbol_name: &str) -> Option<String> {
        use perl_parser::ast::NodeKind;

        fn require_module_name(node: &crate::ast::Node) -> Option<String> {
            let args = match &node.kind {
                NodeKind::FunctionCall { name, args } if name == "require" => args,
                _ => return None,
            };
            let arg = args.first()?;
            match &arg.kind {
                NodeKind::Identifier { name } => Some(name.clone()),
                NodeKind::String { value, .. } => {
                    let cleaned = value.trim_matches('\'').trim_matches('"').trim();
                    Some(cleaned.trim_end_matches(".pm").replace('/', "::"))
                }
                _ => None,
            }
        }

        fn module_runtime_alias(expr: &crate::ast::Node) -> Option<(String, String)> {
            let (alias_name, call_node) = match &expr.kind {
                NodeKind::Assignment { lhs, rhs, op } if op == "=" => {
                    let NodeKind::Variable { name, .. } = &lhs.kind else {
                        return None;
                    };
                    (name.as_str(), rhs.as_ref())
                }
                NodeKind::VariableDeclaration { variable, initializer: Some(rhs), .. } => {
                    let NodeKind::Variable { name, .. } = &variable.kind else {
                        return None;
                    };
                    (name.as_str(), rhs.as_ref())
                }
                _ => return None,
            };
            let NodeKind::FunctionCall { name, args } = &call_node.kind else {
                return None;
            };
            if !matches!(
                name.as_str(),
                "use_module"
                    | "require_module"
                    | "Module::Runtime::use_module"
                    | "Module::Runtime::require_module"
            ) {
                return None;
            }
            let first = args.first()?;
            let NodeKind::String { value, .. } = &first.kind else {
                return None;
            };
            let module = value.trim_matches('\'').trim_matches('"').trim();
            if module.is_empty() {
                return None;
            }
            Some((alias_name.to_string(), module.to_string()))
        }

        fn arg_matches_symbol(module: &str, arg: &crate::ast::Node, symbol: &str) -> bool {
            match &arg.kind {
                NodeKind::String { value, .. } => {
                    let bare = value.trim_matches('\'').trim_matches('"').trim();
                    bare == symbol
                        || (bare.starts_with(':')
                            && resolve_known_export_tag(module, bare)
                                .is_some_and(|expanded| expanded.contains(&symbol)))
                }
                NodeKind::Identifier { name } => {
                    if name == symbol {
                        return true;
                    }
                    if name.starts_with("qw") {
                        let content = name
                            .trim_start_matches("qw")
                            .trim_start_matches(|c: char| "([{/<|!".contains(c))
                            .trim_end_matches(|c: char| ")]}/|!>".contains(c));
                        return content.split_whitespace().any(|word| {
                            word == symbol
                                || (word.starts_with(':')
                                    && resolve_known_export_tag(module, word)
                                        .is_some_and(|expanded| expanded.contains(&symbol)))
                        });
                    }
                    false
                }
                NodeKind::ArrayLiteral { elements } => {
                    elements.iter().any(|el| arg_matches_symbol(module, el, symbol))
                }
                _ => false,
            }
        }

        fn import_call_exports(
            expr: &crate::ast::Node,
            module: &str,
            symbol: &str,
            aliases: &std::collections::HashMap<String, String>,
        ) -> bool {
            let NodeKind::MethodCall { object, method, args } = &expr.kind else {
                return false;
            };
            if method != "import" {
                return false;
            }
            let object_name = match &object.kind {
                NodeKind::Identifier { name } => Some(name.as_str()),
                NodeKind::Variable { name, .. } => aliases.get(name).map(String::as_str),
                _ => return false,
            };
            let Some(object_name) = object_name else {
                return false;
            };
            if object_name != module {
                return false;
            }
            if args.is_empty() {
                return true;
            }
            args.iter().any(|arg| arg_matches_symbol(module, arg, symbol))
        }

        fn inner_expr(node: &crate::ast::Node) -> &crate::ast::Node {
            if let NodeKind::ExpressionStatement { expression } = &node.kind {
                expression.as_ref()
            } else {
                node
            }
        }

        fn find(node: &crate::ast::Node, name: &str) -> Option<String> {
            match &node.kind {
                NodeKind::Use { module, args, .. } => {
                    for arg in args {
                        if arg == name {
                            return Some(module.clone());
                        }
                        if arg.starts_with("qw") {
                            // Support all qw delimiters: (), [], {}, <>, //, ||, !!
                            let content = arg
                                .trim_start_matches("qw")
                                .trim_start_matches(|c: char| "([{/<|!".contains(c))
                                .trim_end_matches(|c: char| ")]}/|!>".contains(c));
                            for word in content.split_whitespace() {
                                if word == name {
                                    return Some(module.clone());
                                }
                                if word.starts_with(':')
                                    && let Some(expanded) = resolve_known_export_tag(module, word)
                                    && expanded.contains(&name)
                                {
                                    return Some(module.clone());
                                }
                            }
                        } else if arg.starts_with(':')
                            && let Some(expanded) = resolve_known_export_tag(module, arg)
                            && expanded.contains(&name)
                        {
                            return Some(module.clone());
                        }
                    }
                }
                NodeKind::Program { statements } | NodeKind::Block { statements } => {
                    let mut required_modules: Vec<String> = statements
                        .iter()
                        .filter_map(|stmt| require_module_name(inner_expr(stmt)))
                        .collect();
                    let mut aliases: std::collections::HashMap<String, String> =
                        std::collections::HashMap::new();
                    for stmt in statements {
                        if let Some((alias, module)) = module_runtime_alias(inner_expr(stmt)) {
                            aliases.insert(alias, module.clone());
                            if !required_modules.contains(&module) {
                                required_modules.push(module);
                            }
                        }
                    }
                    for stmt in statements {
                        let expr = inner_expr(stmt);
                        for module in &required_modules {
                            if import_call_exports(expr, module, name, &aliases) {
                                return Some(module.clone());
                            }
                        }
                    }
                    for stmt in statements {
                        if let Some(src) = find(stmt, name) {
                            return Some(src);
                        }
                    }
                }
                _ => {}
            }
            None
        }

        find(ast, symbol_name)
    }

    /// Check if a variable is declared with 'our' (package-scoped)
    fn is_our_variable(&self, ast: &crate::ast::Node, var_name: &str, sigil: Option<char>) -> bool {
        use perl_parser::ast::NodeKind;

        fn check(node: &crate::ast::Node, name: &str, sigil: Option<char>) -> bool {
            match &node.kind {
                NodeKind::VariableDeclaration { declarator, variable, .. }
                    if declarator == "our" =>
                {
                    if let NodeKind::Variable { name: n, sigil: s } = &variable.kind {
                        if n == name {
                            return match sigil {
                                None => true,
                                Some(sig) => s.starts_with(sig),
                            };
                        }
                    }
                }
                NodeKind::VariableListDeclaration { declarator, variables, .. }
                    if declarator == "our" =>
                {
                    for var in variables {
                        if let NodeKind::Variable { name: n, sigil: s } = &var.kind {
                            if n == name {
                                return match sigil {
                                    None => true,
                                    Some(sig) => s.starts_with(sig),
                                };
                            }
                        }
                    }
                }
                NodeKind::Program { statements } | NodeKind::Block { statements } => {
                    for stmt in statements {
                        if check(stmt, name, sigil) {
                            return true;
                        }
                    }
                }
                NodeKind::Subroutine { body, .. } => {
                    if check(body, name, sigil) {
                        return true;
                    }
                }
                _ => {}
            }
            false
        }

        check(ast, var_name, sigil)
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

            let arguments = params["arguments"].as_array().cloned().unwrap_or_default();

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

            let provider = ExecuteCommandProvider::with_workspace_roots(workspace_roots);

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
                | "perl.debugTests" => {
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

                    // Launch perl -d as a detached child process
                    match std::process::Command::new("perl")
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

    #[test]
    fn find_import_source_supports_require_manual_import() -> Result<(), Box<dyn std::error::Error>>
    {
        use crate::Parser;
        let server =
            LspServer::with_io(Box::new(Cursor::new(Vec::<u8>::new())), Box::new(Vec::<u8>::new()));
        let source = "require List::Util;\nList::Util->import('sum');\nmy $x = sum();\n";
        let mut parser = Parser::new(source);
        let ast = parser.parse()?;

        let source_module = server.find_import_source(&ast, "sum");
        assert_eq!(
            source_module.as_deref(),
            Some("List::Util"),
            "sum should resolve through require+manual import"
        );
        Ok(())
    }

    #[test]
    fn find_import_source_supports_require_default_import() -> Result<(), Box<dyn std::error::Error>>
    {
        use crate::Parser;
        let server =
            LspServer::with_io(Box::new(Cursor::new(Vec::<u8>::new())), Box::new(Vec::<u8>::new()));
        let source = "require List::Util;\nList::Util->import();\nmy $x = sum();\n";
        let mut parser = Parser::new(source);
        let ast = parser.parse()?;

        let source_module = server.find_import_source(&ast, "sum");
        assert_eq!(
            source_module.as_deref(),
            Some("List::Util"),
            "sum should resolve through require+default import (best-effort)"
        );
        Ok(())
    }

    #[test]
    fn find_import_source_supports_module_runtime_alias() -> Result<(), Box<dyn std::error::Error>>
    {
        use crate::Parser;
        let server =
            LspServer::with_io(Box::new(Cursor::new(Vec::<u8>::new())), Box::new(Vec::<u8>::new()));
        let source = "my $mod = use_module('Foo::Bar');\n$mod->import('baz');\nbaz();\n";
        let mut parser = Parser::new(source);
        let ast = parser.parse()?;

        let source_module = server.find_import_source(&ast, "baz");
        assert_eq!(
            source_module.as_deref(),
            Some("Foo::Bar"),
            "baz should resolve through use_module+import alias"
        );
        Ok(())
    }
}
