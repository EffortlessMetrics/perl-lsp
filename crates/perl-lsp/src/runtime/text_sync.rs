//! Text document synchronization
//!
//! Handles didOpen, didChange, didClose, didSave notifications.

use super::*;
use crate::protocol::invalid_params;
#[cfg(feature = "workspace")]
use perl_parser::workspace_index::{IndexPhase, IndexState};

impl LspServer {
    /// Handle textDocument/didOpen notification
    pub(crate) fn handle_did_open(&self, params: Option<Value>) -> Result<(), JsonRpcError> {
        if let Some(params) = params {
            let uri = params
                .pointer("/textDocument/uri")
                .and_then(|v| v.as_str())
                .ok_or_else(|| invalid_params("Missing required parameter: textDocument.uri"))?;
            let text = params
                .pointer("/textDocument/text")
                .and_then(|v| v.as_str())
                .ok_or_else(|| invalid_params("Missing required parameter: textDocument.text"))?;
            let version_i64 =
                params.pointer("/textDocument/version").and_then(|v| v.as_i64()).unwrap_or(0);
            let version = i32::try_from(version_i64).unwrap_or(0);

            eprintln!("Document opened: {}", uri);

            // Notify coordinator of pending change (tracks parse storm)
            #[cfg(feature = "workspace")]
            if let Some(coordinator) = self.coordinator() {
                coordinator.notify_change(uri);
            }

            // Check cache first
            let (ast, errors) = if let Some(cached_ast) = self.ast_cache.get(uri, text) {
                eprintln!("Using cached AST for {}", uri);
                (Some((*cached_ast).clone()), vec![])
            } else {
                // Parse the document up to __DATA__ or __END__ marker
                let code_text = crate::util::code_slice(text);
                let mut parser = Parser::new(code_text);
                match parser.parse() {
                    Ok(ast) => {
                        let errors = parser.errors().to_vec();
                        let arc_ast = Arc::new(ast);
                        self.ast_cache.put(uri.to_string(), text, Arc::clone(&arc_ast));
                        (Some((*arc_ast).clone()), errors)
                    }
                    Err(e) => (None, vec![e]),
                }
            };

            // Convert AST to Arc for stable pointers
            let ast_arc = ast.map(Arc::new);

            // Build parent map from the Arc'd AST so pointers remain stable
            let mut parent_map = ParentMap::default();
            if let Some(ref arc) = ast_arc {
                crate::declaration::DeclarationProvider::build_parent_map(
                    arc,
                    &mut parent_map,
                    None,
                );
            }

            // Build line starts cache for O(log n) position conversion
            let rope = ropey::Rope::from_str(text);
            let line_starts = LineStartsCache::new_rope(&rope);

            // Store document state with normalized URI
            let normalized_uri = self.normalize_uri_key(uri);
            self.documents.lock().insert(
                normalized_uri,
                DocumentState {
                    rope: rope.clone(),
                    text: text.to_string(),
                    version,
                    ast: ast_arc.clone(),
                    parse_errors: errors,
                    parent_map,
                    line_starts,
                    generation: Arc::new(AtomicU32::new(0)),
                },
            );

            // Index symbols for workspace search
            // Note: Indexing is a MUTATION operation - use coordinator.index() directly
            // This must happen BEFORE notify_parse_complete to keep work inside the tracking window
            if let Some(ref _ast) = ast_arc {
                // Update the fast symbol index with symbols from workspace index
                #[cfg(feature = "workspace")]
                if let Some(coordinator) = self.coordinator() {
                    let workspace_index = coordinator.index();
                    let index_symbols = workspace_index.find_symbols("");
                    let symbols = index_symbols
                        .into_iter()
                        .filter(|s| s.uri == uri)
                        .map(|s| s.name.clone())
                        .collect::<Vec<_>>();

                    let mut index = self.symbol_index.lock();
                    for symbol in symbols {
                        index.add_symbol(symbol);
                    }
                }
                #[cfg(not(feature = "workspace"))]
                {
                    let _index = self.symbol_index.lock();
                    // Just ensure the index exists even without workspace feature
                }

                // Update the workspace-wide index for cross-file features
                #[cfg(feature = "workspace")]
                if let Some(coordinator) = self.coordinator() {
                    let workspace_index = coordinator.index();
                    if let Ok(url) = url::Url::parse(uri) {
                        match workspace_index.index_file(url, text.to_string()) {
                            Ok(()) => {
                                // Transition to Ready on first successful index if still Building
                                if matches!(
                                    coordinator.state(),
                                    IndexState::Building { phase: IndexPhase::Idle, .. }
                                ) {
                                    let symbol_count = workspace_index.symbol_count();
                                    let file_count = workspace_index.file_count();
                                    coordinator.transition_to_ready(file_count, symbol_count);
                                    eprintln!(
                                        "Index transitioned to Ready after first file (symbols: {})",
                                        symbol_count
                                    );
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to index file {}: {}", uri, e);
                            }
                        }
                    }
                }
            }

            // Notify coordinator that all work (parse + index) is complete (may trigger recovery)
            // This must come AFTER indexing to keep mutation work inside the tracking window
            #[cfg(feature = "workspace")]
            if let Some(coordinator) = self.coordinator() {
                coordinator.notify_parse_complete(uri);
            }

            // Send diagnostics
            self.publish_diagnostics(uri);
        }

        Ok(())
    }

    /// Convenience wrapper to open a document from tests
    pub fn did_open(&self, params: Value) -> Result<(), JsonRpcError> {
        self.handle_did_open(Some(params))
    }

    /// Handle didChange notification
    pub(crate) fn handle_did_change(&self, params: Option<Value>) -> Result<(), JsonRpcError> {
        if let Some(params) = params {
            let uri = params
                .pointer("/textDocument/uri")
                .and_then(|v| v.as_str())
                .ok_or_else(|| invalid_params("Missing required parameter: textDocument.uri"))?;
            let version_i64 =
                params.pointer("/textDocument/version").and_then(|v| v.as_i64()).unwrap_or(0);
            let version = i32::try_from(version_i64).unwrap_or(0);

            if let Some(changes) = params["contentChanges"].as_array() {
                // Get current document state or create new one
                let mut documents = self.documents.lock();
                let normalized_uri = self.normalize_uri_key(uri);
                let mut doc_state = documents
                    .get(&normalized_uri)
                    .or_else(|| documents.get(uri))
                    .cloned()
                    .unwrap_or_else(|| DocumentState {
                        rope: ropey::Rope::new(),
                        text: String::new(),
                        version,
                        ast: None,
                        parse_errors: vec![],
                        parent_map: ParentMap::default(),
                        line_starts: LineStartsCache::new(""),
                        generation: Arc::new(AtomicU32::new(0)),
                    });

                // Increment generation counter for this change
                let next_gen = doc_state.generation.fetch_add(1, Ordering::SeqCst).wrapping_add(1);
                let target_version = version;

                // Apply incremental changes with UTF-16 aware mapping
                use crate::textdoc::{Doc, PosEnc, apply_changes};
                use lsp_types::TextDocumentContentChangeEvent;

                let mut doc = Doc { rope: doc_state.rope.clone(), version };

                // Convert JSON changes to proper LSP types with error logging
                // (Silent filter_map failures can mask document state corruption)
                let mut lsp_changes = Vec::with_capacity(changes.len());
                for (i, c) in changes.iter().enumerate() {
                    match serde_json::from_value::<TextDocumentContentChangeEvent>(c.clone()) {
                        Ok(change) => lsp_changes.push(change),
                        Err(e) => {
                            eprintln!(
                                "ERROR: Failed to deserialize change {} for {}: {}",
                                i, uri, e
                            );
                            eprintln!("Change JSON: {:?}", c);
                            // Continue processing other changes; LSP has no server-initiated
                            // full sync, so logging is critical for diagnosing state issues.
                        }
                    }
                }

                // Apply changes with UTF-16 encoding (as advertised in initialize)
                apply_changes(&mut doc, &lsp_changes, PosEnc::Utf16);

                let text = doc.rope.to_string();
                eprintln!("Document changed: {} (version {})", uri, version);

                // Notify coordinator of pending change (tracks parse storm)
                #[cfg(feature = "workspace")]
                if let Some(coordinator) = self.coordinator() {
                    coordinator.notify_change(uri);
                }

                // Check cache first
                let (ast, errors) = if let Some(cached_ast) = self.ast_cache.get(uri, &text) {
                    eprintln!("Using cached AST for {}", uri);
                    (Some((*cached_ast).clone()), vec![])
                } else {
                    // Parse the document up to __DATA__ or __END__ marker
                    let code_text = crate::util::code_slice(&text);
                    let mut parser = Parser::new(code_text);
                    match parser.parse() {
                        Ok(ast) => {
                            let errors = parser.errors().to_vec();
                            let arc_ast = Arc::new(ast);
                            self.ast_cache.put(uri.to_string(), &text, Arc::clone(&arc_ast));
                            (Some((*arc_ast).clone()), errors)
                        }
                        Err(e) => (None, vec![e]),
                    }
                };

                // Convert AST to Arc for stable pointers
                let ast_arc = ast.map(Arc::new);

                // Build parent map from the Arc'd AST so pointers remain stable
                let mut parent_map = ParentMap::default();
                if let Some(ref arc) = ast_arc {
                    crate::declaration::DeclarationProvider::build_parent_map(
                        arc,
                        &mut parent_map,
                        None,
                    );
                }

                // Build line starts cache for O(log n) position conversion
                let line_starts = LineStartsCache::new_rope(&doc.rope);

                // Update document state with properly updated content
                doc_state = DocumentState {
                    rope: doc.rope.clone(),
                    text: text.to_string(),
                    version,
                    ast: ast_arc.clone(),
                    parse_errors: errors,
                    parent_map,
                    line_starts,
                    generation: doc_state.generation.clone(), // Preserve the generation counter
                };

                // Check if a newer change arrived while we were parsing
                if let Some(existing_doc) = self.get_document(&documents, uri) {
                    if existing_doc.generation.load(Ordering::SeqCst) != next_gen
                        || existing_doc.version > target_version
                    {
                        eprintln!(
                            "Discarding stale parse result for {} (gen {} != {} or version {} > {})",
                            uri,
                            next_gen,
                            existing_doc.generation.load(Ordering::SeqCst),
                            existing_doc.version,
                            target_version
                        );
                        // Still notify completion even if discarding, to keep coordinator state consistent
                        #[cfg(feature = "workspace")]
                        if let Some(coordinator) = self.coordinator() {
                            coordinator.notify_parse_complete(uri);
                        }
                        return Ok(());
                    }
                }

                documents.insert(normalized_uri.clone(), doc_state);

                // Must drop the lock before calling publish_diagnostics
                drop(documents);

                // Index symbols for workspace search
                // Note: Indexing is a MUTATION operation - use coordinator.index() directly
                // This must happen BEFORE notify_parse_complete to keep work inside the tracking window
                if let Some(ref _ast) = ast_arc {
                    // Update the workspace-wide index for cross-file features
                    // Note: version is maintained by the document state
                    #[cfg(feature = "workspace")]
                    if let Some(coordinator) = self.coordinator() {
                        let workspace_index = coordinator.index();
                        if let Ok(url) = url::Url::parse(uri) {
                            let doc_content = self
                                .documents
                                .lock()
                                .get(uri)
                                .map(|d| d.text.clone())
                                .unwrap_or_default();
                            if let Err(e) = workspace_index.index_file(url, doc_content) {
                                eprintln!("Failed to index file {}: {}", uri, e);
                            }
                        }
                    }
                }

                // Notify coordinator that all work (parse + index) is complete (may trigger recovery)
                // This must come AFTER indexing to keep mutation work inside the tracking window
                #[cfg(feature = "workspace")]
                if let Some(coordinator) = self.coordinator() {
                    coordinator.notify_parse_complete(uri);
                }

                // Send diagnostics
                self.publish_diagnostics(uri);
            }
        }

        Ok(())
    }

    /// Handle didClose notification
    ///
    /// Deterministic state transition: notify coordinator of document close
    /// so it can update pending change tracking if needed.
    pub(crate) fn handle_did_close(&self, params: Option<Value>) -> Result<(), JsonRpcError> {
        if let Some(params) = params {
            let uri = params
                .pointer("/textDocument/uri")
                .and_then(|v| v.as_str())
                .ok_or_else(|| invalid_params("Missing required parameter: textDocument.uri"))?;

            eprintln!("Document closed: {}", uri);

            // Notify coordinator of pending change to track cleanup work
            #[cfg(feature = "workspace")]
            if let Some(coordinator) = self.coordinator() {
                coordinator.notify_change(uri);
            }

            // Remove from documents
            self.documents.lock().remove(uri);

            // Clear from workspace index
            // Note: Mutation operation - use coordinator.index() directly
            #[cfg(feature = "workspace")]
            if let Some(coordinator) = self.coordinator() {
                coordinator.index().clear_file(uri);
            }

            // Notify coordinator that cleanup is complete
            #[cfg(feature = "workspace")]
            if let Some(coordinator) = self.coordinator() {
                coordinator.notify_parse_complete(uri);
            }

            // Clear diagnostics for this file using centralized notify
            let _ = self.notify(
                "textDocument/publishDiagnostics",
                json!({
                    "uri": uri,
                    "diagnostics": []
                }),
            );
        }

        Ok(())
    }

    /// Handle didSave notification
    pub(crate) fn handle_did_save(&self, params: Option<Value>) -> Result<(), JsonRpcError> {
        if let Some(params) = params {
            let uri = params
                .pointer("/textDocument/uri")
                .and_then(|v| v.as_str())
                .ok_or_else(|| invalid_params("Missing required parameter: textDocument.uri"))?;
            let _version = params
                .pointer("/textDocument/version")
                .and_then(|v| v.as_i64())
                .and_then(|v| i32::try_from(v).ok());

            eprintln!("Document saved: {}", uri);

            // Re-run diagnostics on save to catch any changes
            let documents = self.documents.lock();
            if let Some(doc) = self.get_document(&documents, uri) {
                if let Some(ref ast) = doc.ast {
                    // Run diagnostics
                    let provider = DiagnosticsProvider::new(ast, doc.text.clone());
                    let diagnostics = provider.get_diagnostics(ast, &doc.parse_errors, &doc.text);

                    // Convert diagnostics
                    let lsp_diagnostics: Vec<Value> = diagnostics
                        .iter()
                        .map(|diag| {
                            let (start_line, start_char) = self.offset_to_pos16(doc, diag.range.0);
                            let (end_line, end_char) = self.offset_to_pos16(doc, diag.range.1);

                            json!({
                                "range": {
                                    "start": { "line": start_line, "character": start_char },
                                    "end": { "line": end_line, "character": end_char }
                                },
                                "severity": match diag.severity {
                                    InternalDiagnosticSeverity::Error => 1,
                                    InternalDiagnosticSeverity::Warning => 2,
                                    InternalDiagnosticSeverity::Information => 3,
                                    InternalDiagnosticSeverity::Hint => 4,
                                },
                                "message": diag.message,
                                "source": "perl"
                            })
                        })
                        .collect();

                    // Send diagnostics notification
                    let _ = self.notify(
                        "textDocument/publishDiagnostics",
                        json!({
                            "uri": uri,
                            "diagnostics": lsp_diagnostics
                        }),
                    );
                }
            }

            // Optionally, trigger any post-save hooks here
            // For example: format on save, run tests, etc.
        }

        Ok(())
    }

    /// Handle willSave notification
    pub(crate) fn handle_will_save(&self, params: Option<Value>) -> Result<(), JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let reason = params["reason"].as_u64().unwrap_or(1); // 1 = Manual, 2 = AfterDelay, 3 = FocusOut

            eprintln!("Document will save: {} (reason: {})", uri, reason);

            // Pre-save validation or cleanup can be done here
            // For example: remove trailing whitespace, fix imports, etc.
        }

        Ok(())
    }

    /// Handle willSaveWaitUntil request
    pub(crate) fn handle_will_save_wait_until(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let _reason = params["reason"].as_u64().unwrap_or(1);

            eprintln!("Document will save wait until: {}", uri);

            let documents = self.documents.lock();
            if let Some(doc) = self.get_document(&documents, uri) {
                // Return text edits to be applied before saving
                // For example: format document, organize imports, etc.

                // Check if we should format on save
                let config = self.config.lock();
                if config.test_runner_enabled {
                    // Using existing config field as example
                    // Could add format_on_save config option
                    let formatter = CodeFormatter::new();
                    let format_options = FormattingOptions {
                        tab_size: 4,
                        insert_spaces: true,
                        trim_trailing_whitespace: Some(true),
                        insert_final_newline: Some(true),
                        trim_final_newlines: Some(true),
                    };

                    if let Ok(edits) = formatter.format_document(&doc.text, &format_options) {
                        if !edits.is_empty() {
                            // Convert FormatTextEdit to LSP TextEdit
                            // The edits already have line/character positions
                            let lsp_edits: Vec<Value> = edits
                                .iter()
                                .map(|edit| {
                                    json!({
                                        "range": {
                                            "start": {
                                                "line": edit.range.start.line,
                                                "character": edit.range.start.character
                                            },
                                            "end": {
                                                "line": edit.range.end.line,
                                                "character": edit.range.end.character
                                            }
                                        },
                                        "newText": edit.new_text
                                    })
                                })
                                .collect();

                            return Ok(Some(json!(lsp_edits)));
                        }
                    }
                }
            }
        }

        // Return empty array if no edits
        Ok(Some(json!([])))
    }

    /// Get the end position of a document
    pub(crate) fn get_document_end_position(&self, content: &str) -> Value {
        let lines: Vec<&str> = content.lines().collect();
        let last_line = lines.len().saturating_sub(1);
        let last_char = lines.last().map(|l| l.len()).unwrap_or(0);

        json!({
            "line": last_line,
            "character": last_char
        })
    }
}
