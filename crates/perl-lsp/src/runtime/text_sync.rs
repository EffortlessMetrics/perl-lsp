//! Text document synchronization
//!
//! Handles didOpen, didChange, didClose, didSave notifications.
//!
//! We advertise `TextDocumentSyncKind::Incremental` (2): the client sends
//! range-based text edits which are applied to the in-memory Rope via
//! [`apply_changes`].  After applying the edits the *entire* document is
//! reparsed — incremental *parsing* is future work.  The sync kind is about
//! how document text is transferred, not the parsing strategy.

use super::*;
use crate::protocol::invalid_params;
use crate::state::DegradationTier;
#[cfg(feature = "workspace")]
use perl_parser::workspace_index::{IndexPhase, IndexState};

impl LspServer {
    /// Handle textDocument/didOpen notification.
    ///
    /// Delegates to [`Self::handle_did_open_with_cancellation`] with no token.
    pub(crate) fn handle_did_open(&self, params: Option<Value>) -> Result<(), JsonRpcError> {
        self.handle_did_open_with_cancellation(params, None)
    }

    /// Handle textDocument/didOpen with an optional parser cancellation token.
    ///
    /// When a cancellation token is provided the parser is constructed via
    /// `Parser::new_with_cancellation` so that setting the flag to `true` can
    /// cooperatively interrupt the parse.  Pass `None` for the legacy
    /// (non-cancellable) path.
    pub fn handle_did_open_with_cancellation(
        &self,
        params: Option<Value>,
        cancellation_token: Option<Arc<AtomicBool>>,
    ) -> Result<(), JsonRpcError> {
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

            tracing::debug!("Document opened: {}", uri);

            // Large file guard: skip parsing for oversized files
            let file_size = text.len();
            let size_limit = crate::state::max_file_size_bytes();
            if file_size > size_limit {
                tracing::warn!(
                    "Skipping parse for {} ({} bytes exceeds {} byte limit)",
                    uri,
                    file_size,
                    size_limit
                );

                // Store document state without AST
                let rope = ropey::Rope::from_str(text);
                let line_starts = LineStartsCache::new_rope(&rope);
                let normalized_uri = self.normalize_uri_key(uri);
                self.documents.lock().insert(
                    normalized_uri.clone(),
                    DocumentState {
                        rope,
                        text: text.to_string(),
                        version,
                        ast: None,
                        parse_errors: vec![],
                        parent_map: ParentMap::default(),
                        line_starts,
                        generation: Arc::new(AtomicU32::new(0)),
                        degradation_tier: DegradationTier::Minimal,
                    },
                );

                if let Err(e) = self.notify(
                    "textDocument/publishDiagnostics",
                    json!({
                        "uri": uri,
                        "diagnostics": []
                    }),
                ) {
                    tracing::warn!("Failed to publish diagnostics for {}: {}", uri, e);
                }

                return Ok(());
            }

            // Notify coordinator of pending change (tracks parse storm)
            #[cfg(feature = "workspace")]
            if let Some(coordinator) = self.coordinator() {
                coordinator.notify_change(uri);
            }

            // Check cache first
            let (ast, errors) = if let Some(cached_ast) = self.ast_cache.get(uri, text) {
                tracing::debug!("Using cached AST for {}", uri);
                (Some((*cached_ast).clone()), vec![])
            } else {
                // Parse the document up to __DATA__ or __END__ marker
                let code_text = crate::util::code_slice(text);
                let mut parser = match cancellation_token {
                    Some(token) => Parser::new_with_cancellation(code_text, token),
                    None => Parser::new(code_text),
                };
                match parser.parse() {
                    Ok(ast) => {
                        let errors = parser.errors().to_vec();
                        let arc_ast = Arc::new(ast);
                        self.ast_cache.put(uri.to_string(), text, Arc::clone(&arc_ast));
                        (Some((*arc_ast).clone()), errors)
                    }
                    Err(crate::error::ParseError::Cancelled) => {
                        tracing::debug!("Parse cancelled for {} — newer change pending", uri);
                        return Ok(());
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

            // Compute degradation tier before moving errors
            let degradation_tier = DegradationTier::from_parse_result(&ast_arc, &errors);

            // Store document state with normalized URI
            let normalized_uri = self.normalize_uri_key(uri);
            self.documents.lock().insert(
                normalized_uri.clone(),
                DocumentState {
                    rope: rope.clone(),
                    text: text.to_string(),
                    version,
                    ast: ast_arc.clone(),
                    parse_errors: errors,
                    parent_map,
                    line_starts,
                    generation: Arc::new(AtomicU32::new(0)),
                    degradation_tier,
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

                // Update the workspace-wide index for cross-file features.
                // Indexing runs in a background task so the handler returns
                // immediately without blocking on file I/O or symbol extraction.
                // `notify_parse_complete` is called inside the background task.
                #[cfg(feature = "workspace")]
                if let Some(coordinator) = self.coordinator() {
                    if let Ok(url) = url::Url::parse(uri) {
                        let workspace_index = Arc::clone(coordinator.index());
                        let coordinator_clone = Arc::clone(coordinator);
                        let text_owned = text.to_string();
                        let uri_owned = uri.to_string();
                        let task_counter = Arc::clone(&self.pending_index_task_count);
                        task_counter.fetch_add(1, Ordering::SeqCst);

                        let task = move || {
                            match workspace_index.index_file(url, text_owned) {
                                Ok(()) => {
                                    if matches!(
                                        coordinator_clone.state(),
                                        IndexState::Building { phase: IndexPhase::Idle, .. }
                                    ) {
                                        let symbol_count = workspace_index.symbol_count();
                                        let file_count = workspace_index.file_count();
                                        coordinator_clone
                                            .transition_to_ready(file_count, symbol_count);
                                        tracing::info!(
                                            "Index transitioned to Ready after first file \
                                             (symbols: {})",
                                            symbol_count
                                        );
                                    }
                                }
                                Err(e) => {
                                    tracing::warn!("Failed to index file {}: {}", uri_owned, e);
                                }
                            }
                            coordinator_clone.notify_parse_complete(&uri_owned);
                            task_counter.fetch_sub(1, Ordering::SeqCst);
                        };

                        // Spawn on the tokio blocking pool when a runtime is available
                        // (production path via Scheduler).  Fall back to synchronous
                        // execution in unit tests that construct LspServer directly.
                        match tokio::runtime::Handle::try_current() {
                            Ok(handle) => {
                                handle.spawn_blocking(task);
                                // Diagnostics are published below; coordinator completion
                                // happens asynchronously in the background task.
                            }
                            Err(_) => {
                                task();
                            }
                        }
                        // Skip the synchronous notify_parse_complete below — it was
                        // moved into the background task (or run inline on fallback).
                        self.publish_diagnostics(uri);
                        return Ok(());
                    }
                }
            }

            // Notify coordinator that all work (parse + index) is complete (may trigger recovery)
            // Reached only when: no coordinator, URL parse fails, or workspace feature is off.
            #[cfg(feature = "workspace")]
            if let Some(coordinator) = self.coordinator() {
                coordinator.notify_parse_complete(uri);
            }

            // Send diagnostics (use original URI for client notification)
            self.publish_diagnostics(uri);
        }

        Ok(())
    }

    /// Convenience wrapper to open a document from tests
    pub fn did_open(&self, params: Value) -> Result<(), JsonRpcError> {
        self.handle_did_open(Some(params))
    }

    /// Handle didChange notification.
    ///
    /// Delegates to [`Self::handle_did_change_with_cancellation`] with no token.
    pub(crate) fn handle_did_change(&self, params: Option<Value>) -> Result<(), JsonRpcError> {
        self.handle_did_change_with_cancellation(params, None)
    }

    /// Handle didChange with an optional parser cancellation token.
    ///
    /// When a cancellation token is provided the parser is constructed via
    /// `Parser::new_with_cancellation` so that setting the flag to `true` can
    /// cooperatively interrupt the parse.  Pass `None` for the legacy
    /// (non-cancellable) path.
    pub fn handle_did_change_with_cancellation(
        &self,
        params: Option<Value>,
        cancellation_token: Option<Arc<AtomicBool>>,
    ) -> Result<(), JsonRpcError> {
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
                        degradation_tier: DegradationTier::Minimal,
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
                            tracing::error!(
                                "Failed to deserialize change {} for {}: {}",
                                i,
                                uri,
                                e
                            );
                            tracing::error!("Change JSON: {:?}", c);
                            // Continue processing other changes; LSP has no server-initiated
                            // full sync, so logging is critical for diagnosing state issues.
                        }
                    }
                }

                // Apply changes with UTF-16 encoding (as advertised in initialize)
                apply_changes(&mut doc, &lsp_changes, PosEnc::Utf16);

                let text = doc.rope.to_string();
                tracing::debug!("Document changed: {} (version {})", uri, version);

                // Large file guard: skip parsing for oversized files
                let file_size = text.len();
                let size_limit = crate::state::max_file_size_bytes();
                if file_size > size_limit {
                    tracing::warn!(
                        "Skipping parse for {} ({} bytes exceeds {} byte limit)",
                        uri,
                        file_size,
                        size_limit
                    );

                    // Update document state without AST
                    let line_starts = LineStartsCache::new_rope(&doc.rope);
                    let normalized_uri = self.normalize_uri_key(uri);
                    doc_state = DocumentState {
                        rope: doc.rope.clone(),
                        text: text.to_string(),
                        version,
                        ast: None,
                        parse_errors: vec![],
                        parent_map: ParentMap::default(),
                        line_starts,
                        generation: doc_state.generation.clone(),
                        degradation_tier: DegradationTier::Minimal,
                    };
                    documents.insert(normalized_uri.clone(), doc_state);
                    drop(documents);

                    if let Err(e) = self.notify(
                        "textDocument/publishDiagnostics",
                        json!({
                            "uri": uri,
                            "diagnostics": []
                        }),
                    ) {
                        tracing::warn!("Failed to publish diagnostics for {}: {}", uri, e);
                    }

                    return Ok(());
                }

                // Notify coordinator of pending change (tracks parse storm)
                #[cfg(feature = "workspace")]
                if let Some(coordinator) = self.coordinator() {
                    coordinator.notify_change(uri);
                }

                // Check cache first
                let (ast, errors) = if let Some(cached_ast) = self.ast_cache.get(uri, &text) {
                    tracing::debug!("Using cached AST for {}", uri);
                    (Some((*cached_ast).clone()), vec![])
                } else {
                    // Parse the document up to __DATA__ or __END__ marker
                    let code_text = crate::util::code_slice(&text);
                    let mut parser = match cancellation_token {
                        Some(token) => Parser::new_with_cancellation(code_text, token),
                        None => Parser::new(code_text),
                    };
                    match parser.parse() {
                        Ok(ast) => {
                            let errors = parser.errors().to_vec();
                            let arc_ast = Arc::new(ast);
                            self.ast_cache.put(uri.to_string(), &text, Arc::clone(&arc_ast));
                            (Some((*arc_ast).clone()), errors)
                        }
                        Err(crate::error::ParseError::Cancelled) => {
                            tracing::debug!("Parse cancelled for {} — newer change pending", uri);
                            return Ok(());
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

                // Compute degradation tier before moving errors
                let degradation_tier = DegradationTier::from_parse_result(&ast_arc, &errors);

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
                    degradation_tier,
                };

                // Check if a newer change arrived while we were parsing
                if let Some(existing_doc) = self.get_document(&documents, uri) {
                    if existing_doc.generation.load(Ordering::SeqCst) != next_gen
                        || existing_doc.version > target_version
                    {
                        tracing::debug!(
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

                // Index symbols for workspace search.
                // Indexing runs in a background task so the handler returns
                // immediately; `notify_parse_complete` is called inside the task.
                if let Some(ref _ast) = ast_arc {
                    #[cfg(feature = "workspace")]
                    if let Some(coordinator) = self.coordinator() {
                        if let Ok(url) = url::Url::parse(uri) {
                            let workspace_index = Arc::clone(coordinator.index());
                            let coordinator_clone = Arc::clone(coordinator);
                            let doc_content = self
                                .documents
                                .lock()
                                .get(uri)
                                .map(|d| d.text.clone())
                                .unwrap_or_default();
                            let uri_owned = uri.to_string();
                            let task_counter = Arc::clone(&self.pending_index_task_count);
                            task_counter.fetch_add(1, Ordering::SeqCst);

                            let task = move || {
                                if let Err(e) = workspace_index.index_file(url, doc_content) {
                                    tracing::warn!("Failed to index file {}: {}", uri_owned, e);
                                }
                                coordinator_clone.notify_parse_complete(&uri_owned);
                                task_counter.fetch_sub(1, Ordering::SeqCst);
                            };

                            match tokio::runtime::Handle::try_current() {
                                Ok(handle) => {
                                    handle.spawn_blocking(task);
                                }
                                Err(_) => {
                                    task();
                                }
                            }

                            // Send diagnostics (debounced); coordinator completion is async.
                            self.publish_diagnostics_debounced(uri);
                            return Ok(());
                        }
                    }
                }

                // Notify coordinator synchronously when no coordinator/URL/workspace feature.
                #[cfg(feature = "workspace")]
                if let Some(coordinator) = self.coordinator() {
                    coordinator.notify_parse_complete(uri);
                }

                // Send diagnostics (use original URI for client notification)
                // Debounced: coalesces rapid typing into a single publication
                self.publish_diagnostics_debounced(uri);
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
            let normalized_uri = self.normalize_uri_key(uri);

            tracing::debug!("Document closed: {}", uri);

            // Notify coordinator of pending change to track cleanup work
            #[cfg(feature = "workspace")]
            if let Some(coordinator) = self.coordinator() {
                coordinator.notify_change(uri);
            }

            // Remove from documents
            let mut documents = self.documents.lock();
            documents.remove(&normalized_uri).or_else(|| documents.remove(uri));

            // Cancel any in-progress parse and clean up the cancellation flag.
            {
                let mut flags = self.parse_cancel_flags.lock();
                if let Some(flag) = flags.remove(&normalized_uri) {
                    flag.store(true, Ordering::Release);
                }
                // Also try raw URI in case normalize produced a different key.
                if let Some(flag) = flags.remove(uri) {
                    flag.store(true, Ordering::Release);
                }
            }

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
            if let Err(e) = self.notify(
                "textDocument/publishDiagnostics",
                json!({
                    "uri": normalized_uri,
                    "diagnostics": []
                }),
            ) {
                tracing::warn!("Failed to clear diagnostics for {}: {}", normalized_uri, e);
            }
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
            let normalized_uri = self.normalize_uri_key(uri);
            let _version = params
                .pointer("/textDocument/version")
                .and_then(|v| v.as_i64())
                .and_then(|v| i32::try_from(v).ok());

            tracing::debug!("Document saved: {}", uri);

            // Re-run diagnostics on save to catch any changes
            let documents = self.documents.lock();
            if let Some(doc) = self.get_document(&documents, &normalized_uri) {
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
                    if let Err(e) = self.notify(
                        "textDocument/publishDiagnostics",
                        json!({
                            "uri": normalized_uri,
                            "diagnostics": lsp_diagnostics
                        }),
                    ) {
                        tracing::warn!(
                            "Failed to publish diagnostics for {}: {}",
                            normalized_uri,
                            e
                        );
                    }
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

            tracing::debug!("Document will save: {} (reason: {})", uri, reason);

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

            tracing::debug!("Document will save wait until: {}", uri);

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
        let lines: Vec<&str> = content.split('\n').collect();
        let last_line = lines.len().saturating_sub(1);
        let last_char = lines.last().map(|l| l.len()).unwrap_or(0);

        json!({
            "line": last_line,
            "character": last_char
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Verify that `did_open` and `did_change` return with the document stored
    /// and that the `pending_index_tasks()` counter is accessible (issue #2352).
    ///
    /// Without a tokio runtime the sync fallback path runs, so the counter
    /// returns to zero before the assertions.  This test exercises the public
    /// API surface introduced by the async-indexing refactor.
    #[test]
    fn test_indexing_does_not_block_did_change() -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::new();
        let uri = "file:///test_async_index.pl";
        let text = "package Foo;\nsub bar { 1 }\n1;\n";

        // Open document — handler must return Ok even when indexing is async.
        server.did_open(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": text
            }
        }))?;

        // Document must be stored in the in-memory map after did_open returns.
        assert!(server.documents.lock().contains_key(uri));

        // The counter is accessible; in the sync fallback path (no tokio runtime
        // in unit tests) it settles to 0 once the handler returns.
        assert_eq!(server.pending_index_tasks(), 0);

        // A subsequent did_change must also succeed.
        server.handle_did_change(Some(json!({
            "textDocument": { "uri": uri, "version": 2 },
            "contentChanges": [{ "text": "package Foo;\nsub baz { 2 }\n1;\n" }]
        })))?;

        assert!(server.documents.lock().contains_key(uri));
        assert_eq!(server.pending_index_tasks(), 0);

        Ok(())
    }
}
