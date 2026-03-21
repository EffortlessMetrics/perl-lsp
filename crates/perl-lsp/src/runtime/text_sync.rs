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
                        #[cfg(feature = "incremental")]
                        incremental_doc: None,
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

            // Initialize incremental document from the already-parsed text (didOpen).
            // code_slice is applied here to match what the full parser sees.
            #[cfg(feature = "incremental")]
            let incremental_doc = {
                use perl_incremental_parsing::incremental::incremental_document::IncrementalDocument;
                let code_text = crate::util::code_slice(text);
                IncrementalDocument::new(code_text.to_string()).ok()
            };

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
                    #[cfg(feature = "incremental")]
                    incremental_doc,
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
                        #[cfg(feature = "incremental")]
                        incremental_doc: None,
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

                // Build incremental edits from the OLD source BEFORE mutating the rope.
                // UTF-16 line/char → byte conversion must use the pre-change line index.
                #[cfg(feature = "incremental")]
                let incremental_edits_opt: Option<
                    perl_incremental_parsing::incremental::incremental_edit::IncrementalEditSet,
                > = {
                    use perl_incremental_parsing::incremental::incremental_edit::{
                        IncrementalEdit, IncrementalEditSet,
                    };
                    let mut edit_set = IncrementalEditSet::new();
                    let mut all_ranged = true;
                    for change in &lsp_changes {
                        if let Some(range) = change.range {
                            // Convert UTF-16 line/char to byte offsets using the pre-change
                            // line_starts (populated from the rope before apply_changes runs).
                            let start_byte = doc_state.line_starts.position_to_offset_rope(
                                &doc_state.rope,
                                range.start.line,
                                range.start.character,
                            );
                            let old_end_byte = doc_state.line_starts.position_to_offset_rope(
                                &doc_state.rope,
                                range.end.line,
                                range.end.character,
                            );
                            edit_set.add(IncrementalEdit::new(
                                start_byte,
                                old_end_byte,
                                change.text.clone(),
                            ));
                        } else {
                            // Full-document replace — not a ranged edit; reset below
                            tracing::trace!(
                                "Full-document replace detected for {} — incremental edits not supported",
                                uri
                            );
                            all_ranged = false;
                            break;
                        }
                    }
                    if all_ranged && !edit_set.is_empty() { Some(edit_set) } else { None }
                };

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
                        #[cfg(feature = "incremental")]
                        incremental_doc: None,
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

                // Update or reinitialize IncrementalDocument for the new text.
                // - Ranged edits: apply to existing incremental_doc (fast path).
                // - Full replace or no existing doc: reinitialize from new text (fallback).
                #[cfg(feature = "incremental")]
                let incremental_doc = {
                    use perl_incremental_parsing::incremental::incremental_document::IncrementalDocument;
                    let code_text = crate::util::code_slice(&text);
                    match (doc_state.incremental_doc.take(), incremental_edits_opt) {
                        (Some(mut inc), Some(edits)) => {
                            // Try applying the incremental edits to the existing tree
                            match inc.apply_edits(&edits) {
                                Ok(()) => Some(inc),
                                Err(_) => {
                                    // Fallback: reinitialize from the post-change source
                                    IncrementalDocument::new(code_text.to_string()).ok()
                                }
                            }
                        }
                        // Full-document replace or no prior incremental state: reinitialize
                        _ => IncrementalDocument::new(code_text.to_string()).ok(),
                    }
                };

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
                    #[cfg(feature = "incremental")]
                    incremental_doc,
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

    /// Verify that a ranged didChange initializes and preserves incremental_doc.
    #[cfg(feature = "incremental")]
    #[test]
    fn test_incremental_path_taken_on_ranged_change() -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::new();
        let uri = "file:///test_incremental.pl";
        let text = "my $x = 42;\nmy $y = 99;\n";

        server.did_open(json!({
            "textDocument": { "uri": uri, "languageId": "perl", "version": 1, "text": text }
        }))?;

        // Verify incremental_doc was initialized on didOpen
        {
            let docs = server.documents.lock();
            let doc = docs.get(uri).ok_or("document not stored after didOpen")?;
            assert!(
                doc.incremental_doc.is_some(),
                "incremental_doc must be initialized on didOpen"
            );
        }

        // Apply a ranged change: replace "42" with "43"
        server.handle_did_change(Some(json!({
            "textDocument": { "uri": uri, "version": 2 },
            "contentChanges": [{
                "range": {
                    "start": { "line": 0, "character": 8 },
                    "end":   { "line": 0, "character": 10 }
                },
                "text": "43"
            }]
        })))?;

        // Document must still be stored with updated content and a present AST
        {
            let docs = server.documents.lock();
            let doc = docs.get(uri).ok_or("document not stored after didChange")?;
            assert!(doc.text.contains("43"), "document text must be updated");
            assert!(doc.ast.is_some(), "AST must be present after incremental change");
            // incremental_doc must still be present after a ranged edit
            assert!(doc.incremental_doc.is_some(), "incremental_doc must survive a ranged edit");
            // The incremental doc's internal source must reflect the edit.
            // This catches a silent reinit-instead-of-apply bug: reinit would also hold
            // "43" in the source, but would not have the version counter bumped from 0.
            // Checking the source text is the strongest behavioral assertion available
            // without mocking the apply_edits call itself.
            let inc = doc.incremental_doc.as_ref().unwrap();
            assert!(
                inc.source.contains("43"),
                "incremental_doc.source must contain the edit result; got: {:?}",
                inc.source
            );
            assert!(
                !inc.source.contains("42"),
                "incremental_doc.source must not contain the old value; got: {:?}",
                inc.source
            );
            // version > 0 proves apply_edits was called (increments version), not just reinit
            // (which starts at version 0 after IncrementalDocument::new).
            assert!(
                inc.version > 0,
                "incremental_doc.version must be > 0 after at least one edit; got {}",
                inc.version
            );
        }
        Ok(())
    }

    /// Verify that a full-document replace (no range) re-initializes incremental_doc.
    #[cfg(feature = "incremental")]
    #[test]
    fn test_full_replace_reinitializes_incremental_doc() -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::new();
        let uri = "file:///test_inc_replace.pl";
        let text = "my $x = 1;\n";

        server.did_open(json!({
            "textDocument": { "uri": uri, "languageId": "perl", "version": 1, "text": text }
        }))?;

        // Full-document replace (no range field)
        server.handle_did_change(Some(json!({
            "textDocument": { "uri": uri, "version": 2 },
            "contentChanges": [{ "text": "my $y = 2;\n" }]
        })))?;

        let docs = server.documents.lock();
        let doc = docs.get(uri).ok_or("document not stored after full replace")?;
        assert!(
            doc.incremental_doc.is_some(),
            "incremental_doc must be re-initialized on full replace"
        );
        assert!(doc.text.contains("$y"), "text must be updated to new content");
        Ok(())
    }

    /// Verify that broken syntax does not panic and leaves the document in a valid state.
    #[cfg(feature = "incremental")]
    #[test]
    fn test_incremental_fallback_on_parse_error() -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::new();
        let uri = "file:///test_inc_error.pl";

        server.did_open(json!({
            "textDocument": { "uri": uri, "languageId": "perl", "version": 1,
                              "text": "my $x = 42;\n" }
        }))?;

        // Replace with broken syntax — must not panic; document must survive
        server.handle_did_change(Some(json!({
            "textDocument": { "uri": uri, "version": 2 },
            "contentChanges": [{
                "range": { "start": { "line": 0, "character": 0 },
                           "end":   { "line": 0, "character": 11 } },
                "text": "sub { !!!"
            }]
        })))?;

        assert!(server.documents.lock().contains_key(uri), "document must survive broken syntax");
        Ok(())
    }

    /// Verify that UTF-16 position conversion handles multi-byte characters correctly.
    /// LSP clients send UTF-16 code unit indices; characters like emoji or CJK take 2 units
    /// but 4+ UTF-8 bytes. The byte offset calculation must account for this.
    #[cfg(feature = "incremental")]
    #[test]
    fn test_incremental_utf16_multi_byte_character_positions()
    -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::new();
        let uri = "file:///test_inc_utf16.pl";
        // Line 0: "my $emoji = 😀;\n" (😀 is U+1F600, takes 2 UTF-16 units, 4 UTF-8 bytes)
        // UTF-16 positions: m(0) y(1) space(2) $(3) e(4) m(5) o(6) j(7) i(8) space(9) =(10) space(11) 😀(12-13) ;(14)
        // UTF-8 bytes: "my $emoji = " (12 bytes) + "😀" (4 bytes) + ";\n"
        let text = "my $emoji = 😀;\n";

        server.did_open(json!({
            "textDocument": { "uri": uri, "languageId": "perl", "version": 1, "text": text }
        }))?;

        // Replace the emoji (UTF-16: start=12, end=14) with the ASCII "xx"
        server.handle_did_change(Some(json!({
            "textDocument": { "uri": uri, "version": 2 },
            "contentChanges": [{
                "range": {
                    "start": { "line": 0, "character": 12 },
                    "end":   { "line": 0, "character": 14 }
                },
                "text": "xx"
            }]
        })))?;

        let docs = server.documents.lock();
        let doc = docs.get(uri).ok_or("document not stored after UTF-16 edit")?;
        // Should have replaced emoji with "xx"
        assert!(
            doc.text.contains("xx"),
            "UTF-16 multi-byte replacement failed: expected 'xx' in text"
        );
        // The emoji should no longer be there
        assert!(!doc.text.contains("😀"), "UTF-16 multi-byte removal failed: emoji should be gone");
        Ok(())
    }

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
