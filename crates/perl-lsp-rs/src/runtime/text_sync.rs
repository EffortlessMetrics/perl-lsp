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
use crate::state::{DegradationTier, ParsedSnapshot};
#[cfg(feature = "workspace")]
use perl_parser::workspace_index::{IndexPhase, IndexState};
use perl_parser_core::source_file::is_binary_content;

mod document_state;
mod lifecycle;
mod srp_helpers;
mod symbols;
use document_state::{empty_state, minimal_state, minimal_state_from_rope};
#[cfg(feature = "incremental")]
use srp_helpers::build_incremental_edit_set;
use srp_helpers::{is_embedded_template_uri, is_perl_language_id};

/// Last path segment of a document URI (bounded to 64 chars), used as the
/// `detail` field of a `PERL_LSP_TIMING` span. Never allocates on the hot path
/// unless timing is enabled (callers guard the call).
fn uri_tail(uri: &str) -> String {
    let tail = uri.rsplit(['/', '\\']).next().unwrap_or(uri);
    // Keep the last 64 chars on a char boundary to bound JSONL line length.
    tail.char_indices()
        .rev()
        .nth(63)
        .map(|(idx, _)| tail[idx..].to_string())
        .unwrap_or_else(|| tail.to_string())
}

impl LspServer {
    /// Whether the dormant eager-incremental-maintenance fast-path
    /// (`incremental_doc`/`incremental_state`) is opted into for this
    /// server. Always `false` when the `incremental` cargo feature is not
    /// compiled in.
    ///
    /// `didChange` uses this to decide whether to take the off-lock async
    /// parse-worker path (#3396 Phase 3) or the synchronous fallback: eager
    /// incremental maintenance needs its own parse to run synchronously
    /// under the same `documents` lock as the text-state update, so it is
    /// incompatible with the async worker path today.
    fn incremental_eager_enabled(&self) -> bool {
        #[cfg(feature = "incremental")]
        {
            self.incremental_eager.load(Ordering::Relaxed)
        }
        #[cfg(not(feature = "incremental"))]
        {
            false
        }
    }

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
            let language_id =
                params.pointer("/textDocument/languageId").and_then(|v| v.as_str()).unwrap_or("");

            tracing::debug!("Document opened: {}", uri);

            // Template guard: Mojolicious/TT template files are frequently opened
            // with an HTML/template language mode. Parsing those as plain Perl
            // creates noisy diagnostics and poor startup UX.
            if is_embedded_template_uri(uri) && !is_perl_language_id(language_id) {
                tracing::debug!(
                    "Skipping parse for template-like document {} (languageId={})",
                    uri,
                    language_id
                );

                let normalized_uri = self.normalize_uri_key(uri);
                self.documents.lock().insert(normalized_uri.clone(), minimal_state(text, version));

                if let Err(e) = self.notify(
                    "textDocument/publishDiagnostics",
                    json!({
                        "uri": uri,
                        "diagnostics": []
                    }),
                ) {
                    tracing::warn!("Failed to publish diagnostics for {}: {}", uri, e);
                }
                self.clear_document_symbols(uri);

                return Ok(());
            }

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
                let normalized_uri = self.normalize_uri_key(uri);
                self.documents.lock().insert(normalized_uri.clone(), minimal_state(text, version));

                if let Err(e) = self.notify(
                    "textDocument/publishDiagnostics",
                    json!({
                        "uri": uri,
                        "diagnostics": []
                    }),
                ) {
                    tracing::warn!("Failed to publish diagnostics for {}: {}", uri, e);
                }
                self.clear_document_symbols(uri);

                return Ok(());
            }

            // Binary content guard: skip parsing for binary files.
            // Detection is centralized in `perl_source_file::is_binary_content`.
            if is_binary_content(text) {
                tracing::warn!(
                    "Skipping parse for {} (binary content detected: null bytes present)",
                    uri
                );

                let normalized_uri = self.normalize_uri_key(uri);
                self.documents.lock().insert(normalized_uri.clone(), minimal_state(text, version));

                if let Err(e) = self.notify(
                    "textDocument/publishDiagnostics",
                    json!({
                        "uri": uri,
                        "diagnostics": [{
                            "range": {
                                "start": {"line": 0, "character": 0},
                                "end": {"line": 0, "character": 0}
                            },
                            "severity": 3,
                            "source": "perl-lsp",
                            "message": "File appears to contain binary content (null bytes detected). Perl diagnostics are disabled."
                        }]
                    }),
                ) {
                    tracing::warn!(
                        "Failed to publish binary-content diagnostic for {}: {}",
                        uri,
                        e
                    );
                }
                self.clear_document_symbols(uri);

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

            let rope = ropey::Rope::from_str(text);

            // Store document state with normalized URI
            let normalized_uri = self.normalize_uri_key(uri);
            let generation = Arc::new(AtomicU32::new(0));

            // Initialize the incremental parsing state from the already-parsed
            // text (didOpen). Off by default (#3396): the committed AST that
            // providers read comes from the full parse above, and nothing on the
            // read path consumes these fields, so they are only maintained when
            // `set_incremental_eager(true)` opts into the dormant fast-path.
            // code_slice is applied here to match what the full parser sees.
            #[cfg(feature = "incremental")]
            let (incremental_doc, incremental_state) = if self
                .incremental_eager
                .load(Ordering::Relaxed)
            {
                use perl_parser::incremental::IncrementalState;
                use perl_parser::incremental::incremental_document::IncrementalDocument;
                let code_text = crate::util::code_slice(text);
                let inc_doc = match IncrementalDocument::new(code_text.to_string()) {
                    Ok(doc) => Some(doc),
                    Err(e) => {
                        tracing::warn!(
                            "Incremental parsing init failed for {}, falling back to full parsing: {}",
                            uri,
                            e
                        );
                        None
                    }
                };
                // IncrementalState tracks lexer checkpoints (Gap A, #2080) so
                // small ranged edits re-lex from the nearest safe boundary.
                let inc_state = Some(IncrementalState::new(code_text.to_string()));
                (inc_doc, inc_state)
            } else {
                (None, None)
            };

            let mut doc_state = DocumentState::from_parts(
                rope.clone(),
                text.to_string(),
                version,
                Arc::clone(&generation),
            );
            #[cfg(feature = "incremental")]
            {
                doc_state.incremental_doc = incremental_doc;
                doc_state.incremental_state = incremental_state;
            }
            // Publish the parse result as a single ParsedSnapshot rather than
            // writing ast/parse_errors/parent_map/degradation_tier
            // separately -- see `state::ParsedSnapshot`. `from_parse_result`
            // derives content_hash/parent_map/degradation_tier internally so
            // they can never disagree with `ast_arc`/`errors`/`text`. didOpen
            // always starts at generation 0 (freshly created above), so this
            // publication always succeeds synchronously.
            let doc_generation = doc_state.current_generation();
            let snapshot = Arc::new(ParsedSnapshot::from_parse_result(
                doc_generation,
                text,
                ast_arc.clone(),
                errors,
            ));
            doc_state.publish_parsed_if_current(doc_generation, snapshot);

            self.documents.lock().insert(normalized_uri.clone(), doc_state);

            if let Some(ref ast) = ast_arc {
                self.reindex_document_symbols(uri, ast, text);
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
                        let generation = Arc::clone(&generation);
                        let task_counter = Arc::clone(&self.pending_index_task_count);
                        task_counter.fetch_add(1, Ordering::SeqCst);

                        let task = move || {
                            if generation.load(Ordering::Acquire) != 0 {
                                tracing::debug!(
                                    uri = %uri_owned,
                                    "Skipping stale background index task after document close/change"
                                );
                                coordinator_clone.notify_parse_complete(&uri_owned);
                                task_counter.fetch_sub(1, Ordering::SeqCst);
                                return;
                            }
                            match workspace_index.index_file_with_generation(url, text_owned, 0) {
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
                        self.publish_parse_errors_fast(uri);
                        self.publish_diagnostics_debounced(uri);
                        return Ok(());
                    }
                }
            } else {
                self.clear_document_symbols(uri);
            }

            // Notify coordinator that all work (parse + index) is complete (may trigger recovery)
            // Reached only when: no coordinator, URL parse fails, or workspace feature is off.
            #[cfg(feature = "workspace")]
            if let Some(coordinator) = self.coordinator() {
                coordinator.notify_parse_complete(uri);
            }

            // Send diagnostics (use original URI for client notification)
            self.publish_parse_errors_fast(uri);
            self.publish_diagnostics_debounced(uri);
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
            let incoming_version_i64 =
                params.pointer("/textDocument/version").and_then(|v| v.as_i64());
            let incoming_version = incoming_version_i64.and_then(|v| i32::try_from(v).ok());

            // Cancel any active streaming inline completion sessions for this URI
            // that are older than the new document version.
            for key in self.uri_key_variants(uri) {
                if let Some(version) = incoming_version_i64 {
                    self.stream_sessions().cancel_for_uri_version(&key, version);
                } else {
                    self.stream_sessions().cancel_for_uri(&key);
                }
            }

            if let Some(changes) = params["contentChanges"].as_array() {
                // Phase-1 latency instrumentation (opt-in via PERL_LSP_TIMING).
                // Instrumentation only — no behavior change to the mutation path.
                let timing_on = crate::runtime::timing::is_enabled();
                let t_did_change_start = std::time::Instant::now();

                // Get current document state or create new one
                let t_lock_start = std::time::Instant::now();
                let mut documents = self.documents.lock();
                let lock_wait_ms = crate::runtime::timing::elapsed_ms(t_lock_start);
                let normalized_uri = self.normalize_uri_key(uri);
                let existing_doc =
                    documents.get(&normalized_uri).or_else(|| documents.get(uri)).cloned();

                // LSP requires didChange only for opened documents. If we don't
                // have a document and receive ranged edits, applying them against
                // an empty buffer can corrupt state. Ignore this notification and
                // wait for didOpen (or a full-document replace change).
                if existing_doc.is_none() && changes.iter().all(|c| c.get("range").is_some()) {
                    tracing::warn!("Ignoring ranged didChange for unopened document {}", uri);
                    return Ok(());
                }

                // Invalidate the perlcritic violation cache for this file so that
                // the next diagnostic cycle re-runs perlcritic on the new content.
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let file_path = url::Url::parse(uri).ok().and_then(|u| u.to_file_path().ok());
                    if let Some(path) = file_path {
                        let path_str = path.to_string_lossy().to_string();
                        if let Some(ref mut analyzer) = *self.critic_analyzer.lock() {
                            analyzer.invalidate_cache(&path_str);
                        }
                        self.pull_diagnostics_orchestrator.invalidate_file_cache(&path);
                    }
                }

                let mut doc_state =
                    existing_doc.unwrap_or_else(|| empty_state(incoming_version.unwrap_or(0)));

                // Ignore stale didChange notifications that arrive out of order.
                // We only gate on explicit client-provided versions; if a client omits
                // the version field we preserve legacy behavior and treat the change as new.
                if let Some(version) = incoming_version {
                    if version <= doc_state.version {
                        tracing::debug!(
                            "Ignoring stale didChange for {} (incoming version {} <= current {})",
                            uri,
                            version,
                            doc_state.version
                        );
                        return Ok(());
                    }
                }

                // didChange version is required by LSP, but keep a fallback for tolerant
                // handling of non-conforming clients in tests/custom integrations.
                let version =
                    incoming_version.unwrap_or_else(|| doc_state.version.saturating_add(1));
                let skip_template_parse = is_embedded_template_uri(uri)
                    && doc_state
                        .current_parsed()
                        .map(|s| s.degradation_tier())
                        .unwrap_or(DegradationTier::Minimal)
                        == DegradationTier::Minimal;

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
                    perl_parser::incremental::incremental_edit::IncrementalEditSet,
                > = build_incremental_edit_set(&doc_state.rope, &lsp_changes);

                // Apply changes with UTF-16 encoding (as advertised in initialize)
                let t_apply_start = std::time::Instant::now();
                apply_changes(&mut doc, &lsp_changes, PosEnc::Utf16);
                let apply_changes_ms = crate::runtime::timing::elapsed_ms(t_apply_start);

                let t_rope_start = std::time::Instant::now();
                let text = doc.rope.to_string();
                let rope_to_string_ms = crate::runtime::timing::elapsed_ms(t_rope_start);
                tracing::debug!("Document changed: {} (version {})", uri, version);

                // Keep template documents that were intentionally skipped on didOpen
                // in no-parse mode across subsequent didChange notifications.
                if skip_template_parse {
                    let normalized_uri = self.normalize_uri_key(uri);
                    doc_state = minimal_state_from_rope(
                        doc.rope.clone(),
                        text.to_string(),
                        version,
                        doc_state.generation.clone(),
                    );
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
                    self.clear_document_symbols(uri);

                    return Ok(());
                }

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
                    let normalized_uri = self.normalize_uri_key(uri);
                    doc_state = minimal_state_from_rope(
                        doc.rope.clone(),
                        text.to_string(),
                        version,
                        doc_state.generation.clone(),
                    );
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
                    self.clear_document_symbols(uri);

                    return Ok(());
                }

                // Binary content guard: skip parsing for binary files.
                // Detection is centralized in `perl_source_file::is_binary_content`.
                if is_binary_content(&text) {
                    tracing::warn!(
                        "Skipping parse for {} (binary content detected via didChange)",
                        uri
                    );

                    let normalized_uri = self.normalize_uri_key(uri);
                    doc_state = minimal_state_from_rope(
                        doc.rope.clone(),
                        text.to_string(),
                        version,
                        doc_state.generation.clone(),
                    );
                    documents.insert(normalized_uri.clone(), doc_state);
                    drop(documents);

                    if let Err(e) = self.notify(
                        "textDocument/publishDiagnostics",
                        json!({
                            "uri": uri,
                            "diagnostics": [{
                                "range": {
                                    "start": {"line": 0, "character": 0},
                                    "end": {"line": 0, "character": 0}
                                },
                                "severity": 3,
                                "source": "perl-lsp",
                                "message": "File appears to contain binary content (null bytes detected). Perl diagnostics are disabled."
                            }]
                        }),
                    ) {
                        tracing::warn!(
                            "Failed to publish binary-content diagnostic for {}: {}",
                            uri,
                            e
                        );
                    }
                    self.clear_document_symbols(uri);

                    return Ok(());
                }

                // ---- Off-lock async parse path (#3396 Phase 3, default) ----
                //
                // Active whenever a parse worker is installed (the
                // production runtime via `Scheduler::new`, or a test that
                // opted in explicitly) AND the dormant `incremental_eager`
                // fast-path is not in play (that flag needs its own parse
                // to happen synchronously under this same lock -- see
                // `DocumentState::incremental_doc`/`incremental_state`
                // docs). Falls through to the synchronous fallback path
                // below otherwise, so a bare `LspServer::new()` (used by
                // hundreds of existing unit tests that assert
                // `current_parsed()` is available immediately after
                // `handle_did_change` returns) is unaffected.
                if !self.incremental_eager_enabled() {
                    if let Some(worker) = self.parse_worker() {
                        // Commit the text-only mutation now; the parse +
                        // parent-map + publish happen off this lock, in the
                        // worker. `current_parsed()` reports `None` for
                        // this document until the worker publishes for
                        // `next_gen` (or forever, if a newer edit
                        // supersedes it first); `latest_parsed()` keeps
                        // answering with the pre-edit snapshot in the
                        // meantime -- see `state::DocumentState` module
                        // docs and the #3589 pending-parse provider
                        // policies.
                        doc_state.replace_text_state(doc.rope.clone(), text.clone(), version);
                        #[cfg(feature = "incremental")]
                        {
                            doc_state.incremental_doc = None;
                            doc_state.incremental_state = None;
                        }
                        let generation_handle = doc_state.generation.clone();
                        documents.insert(normalized_uri.clone(), doc_state);
                        drop(documents);

                        if timing_on {
                            use crate::runtime::timing::{TimingSpan, elapsed_ms, emit};
                            let tail = uri_tail(uri);
                            let bytes = text.len();
                            let edits = changes.len();
                            let ver = i64::from(version);
                            let total_ms = elapsed_ms(t_did_change_start);
                            for (name, ms) in [
                                ("didChange.total", total_ms),
                                ("didChange.lock_wait", lock_wait_ms),
                                ("didChange.apply_changes", apply_changes_ms),
                                ("didChange.rope_to_string", rope_to_string_ms),
                            ] {
                                emit(TimingSpan::document(
                                    name,
                                    ms,
                                    tail.clone(),
                                    ver,
                                    bytes,
                                    edits,
                                ));
                            }
                        }

                        // Coordinator notification for a NEW pending-parse
                        // lifecycle (tracks parse storm) is fired from
                        // INSIDE `enqueue` itself (`Coordinator::on_activated`,
                        // wired in `install_default_parse_worker`), not from
                        // here after `enqueue` returns -- calling it from
                        // this caller left a window where an unusually fast
                        // worker could dequeue, process, and settle (its
                        // decrement) before this call ever ran, permanently
                        // stranding the pending-parse counter (#3618 settle-
                        // before-increment race). `enqueue`'s return value
                        // is no longer needed by this caller.
                        worker.enqueue(
                            uri.to_string(),
                            normalized_uri,
                            next_gen,
                            generation_handle,
                            Arc::from(text.as_str()),
                        );

                        return Ok(());
                    }
                }

                // ---- Synchronous fallback path (unchanged behavior) ----
                // Active when no worker is installed, or `incremental_eager`
                // opted into the dormant fast-path that needs the parse
                // to happen synchronously under this same lock. Every call
                // here fully parses before returning (no coalescing is
                // possible), so `notify_change` below is always followed by
                // exactly one matching `notify_parse_complete` for THIS
                // edit -- unlike the async branch above, this unconditional
                // call is already balanced.
                #[cfg(feature = "workspace")]
                if let Some(coordinator) = self.coordinator() {
                    coordinator.notify_change(uri);
                }

                // Check cache first
                let t_parse_start = std::time::Instant::now();
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
                            // The cooperative cancellation flag fired mid-parse — record
                            // it so a typing storm's discarded parses are observable.
                            if timing_on {
                                crate::runtime::timing::emit(
                                    crate::runtime::timing::TimingSpan::labeled(
                                        "parse.cancel_seen",
                                        crate::runtime::timing::elapsed_ms(t_did_change_start),
                                        uri_tail(uri),
                                    ),
                                );
                            }
                            return Ok(());
                        }
                        Err(e) => (None, vec![e]),
                    }
                };
                let full_parse_ms = crate::runtime::timing::elapsed_ms(t_parse_start);

                // Convert AST to Arc for stable pointers
                let ast_arc = ast.map(Arc::new);

                // Build the ParsedSnapshot now, while `errors` is still
                // available to move -- `from_parse_result` derives
                // content_hash/parent_map/degradation_tier internally from
                // `text`/`ast_arc`/`errors` so they can never disagree (see
                // `state::ParsedSnapshot`). Timed as `parent_map_ms` since
                // parent-map construction (inside `from_parse_result`)
                // dominates this call's cost; hashing and tier derivation
                // are cheap by comparison. Published later, once `doc_state`
                // has been rebuilt below.
                let t_parent_map_start = std::time::Instant::now();
                let snapshot = Arc::new(ParsedSnapshot::from_parse_result(
                    next_gen,
                    &text,
                    ast_arc.clone(),
                    errors,
                ));
                let parent_map_ms = crate::runtime::timing::elapsed_ms(t_parent_map_start);

                let t_incremental_start = std::time::Instant::now();
                // Maintain the per-document incremental parsing state — but only
                // when eagerly opted in (#3396). The committed AST that every
                // provider reads was produced by the full `Parser::new` parse
                // above; `incremental_doc` / `incremental_state` feed nothing on
                // the read path, so on the default keystroke path we skip this
                // work entirely (it measured ~14x the full parse while committing
                // nothing to the AST). The stale prior state, if any, is dropped
                // when `doc_state` is reassigned below. Toggling this changes
                // neither the committed AST, parse errors, parent map, nor the
                // stale-read generation semantics.
                #[cfg(feature = "incremental")]
                let (incremental_doc, incremental_state) = if self
                    .incremental_eager
                    .load(Ordering::Relaxed)
                {
                    // Update or reinitialize IncrementalDocument for the new text.
                    // - Ranged edits: apply to existing incremental_doc (fast path).
                    // - Full replace or no existing doc: reinitialize from new text (fallback).
                    // Clone the edit set so the incremental_state block below can also use it.
                    let incremental_edits_opt_clone = incremental_edits_opt.clone();
                    let incremental_doc = {
                        use perl_parser::incremental::incremental_document::IncrementalDocument;
                        let code_text = crate::util::code_slice(&text);
                        match (doc_state.incremental_doc.take(), incremental_edits_opt) {
                            (Some(mut inc), Some(edits)) => {
                                // Try applying the incremental edits to the existing tree
                                match inc.apply_edits(&edits) {
                                    Ok(()) => Some(inc),
                                    Err(e) => {
                                        // Fallback: reinitialize from the post-change source
                                        tracing::warn!(
                                            "Incremental edit application failed for {}, reinitializing: {}",
                                            uri,
                                            e
                                        );
                                        match IncrementalDocument::new(code_text.to_string()) {
                                            Ok(doc) => Some(doc),
                                            Err(e2) => {
                                                tracing::warn!(
                                                    "Incremental parsing reinit failed for {}, falling back to full parsing: {}",
                                                    uri,
                                                    e2
                                                );
                                                None
                                            }
                                        }
                                    }
                                }
                            }
                            // Full-document replace or no prior incremental state: reinitialize
                            _ => match IncrementalDocument::new(code_text.to_string()) {
                                Ok(doc) => Some(doc),
                                Err(e) => {
                                    tracing::warn!(
                                        "Incremental parsing reinit failed for {}, falling back to full parsing: {}",
                                        uri,
                                        e
                                    );
                                    None
                                }
                            },
                        }
                    };

                    // Apply edits to the checkpoint-based IncrementalState (Gap A, #2080).
                    //
                    // On a ranged edit we try to apply via `perl_parser::incremental::apply_edits`,
                    // which re-lexes from the nearest checkpoint rather than offset 0. This speeds
                    // up the token stream used by downstream passes for large files. On failure
                    // (edit > 64 KB, > 10 changed lines, or no prior state) we reinitialize the
                    // state from the already-parsed `text` so future edits can use checkpoints.
                    //
                    // The AST for this change still comes from the `Parser::new` call above —
                    // `IncrementalState` speeds up the lexer pass only; the parser pass is unchanged.
                    let incremental_state = {
                        use perl_parser::incremental::{
                            Edit as IncEdit, IncrementalState, apply_edits as inc_apply_edits,
                        };
                        let code_text = crate::util::code_slice(&text);
                        match (doc_state.incremental_state.take(), &incremental_edits_opt_clone) {
                            (Some(mut inc_state), Some(edit_set)) => {
                                // Convert IncrementalEditSet -> Vec<IncEdit> for apply_edits
                                let edits: Vec<IncEdit> = edit_set
                                    .edits
                                    .iter()
                                    .map(|e| IncEdit {
                                        start_byte: e.start_byte,
                                        old_end_byte: e.old_end_byte,
                                        new_end_byte: e.start_byte + e.new_text.len(),
                                        new_text: e.new_text.clone(),
                                    })
                                    .collect();
                                match inc_apply_edits(&mut inc_state, &edits) {
                                    Ok(result) => {
                                        tracing::debug!(
                                            "Incremental state fast-path for {}: reparsed {} of {} bytes",
                                            uri,
                                            result.reparsed_bytes,
                                            inc_state.source.len()
                                        );
                                        Some(inc_state)
                                    }
                                    Err(e) => {
                                        // Fast-path failed (e.g. large edit); reinitialize checkpoints
                                        tracing::debug!(
                                            "Incremental state apply_edits failed for {}, reinitializing: {}",
                                            uri,
                                            e
                                        );
                                        Some(IncrementalState::new(code_text.to_string()))
                                    }
                                }
                            }
                            // Full-document replace or no prior state: reinitialize checkpoints
                            _ => Some(IncrementalState::new(code_text.to_string())),
                        }
                    };
                    (incremental_doc, incremental_state)
                } else {
                    // Default path: the incremental edit set and any prior
                    // incremental state are simply dropped — nothing reads them.
                    (None, None)
                };
                let incremental_doc_update_ms =
                    crate::runtime::timing::elapsed_ms(t_incremental_start);

                // Update document state's text fields IN PLACE (not a fresh
                // `DocumentState::from_parts`) so the previously-published
                // snapshot -- whatever was published for the pre-edit
                // generation -- is preserved rather than silently
                // discarded. `generation` was
                // already bumped to `next_gen` above (same Arc<AtomicU32>),
                // before this edit's text was applied, so `current_parsed()`
                // already reports stale for the remainder of this handler
                // (correctly: the just-parsed snapshot below hasn't
                // published yet) while `latest_parsed()` keeps exposing the
                // pre-edit snapshot until the `publish_parsed_if_current`
                // call below lands the new one -- see
                // `state::DocumentState::replace_text_state`.
                doc_state.replace_text_state(doc.rope.clone(), text.to_string(), version);
                #[cfg(feature = "incremental")]
                {
                    doc_state.incremental_doc = incremental_doc;
                    doc_state.incremental_state = incremental_state;
                }
                // Publish the snapshot built above -- `doc_state`'s
                // generation Arc was already bumped to `next_gen` earlier
                // (same atomic, cloned), so this publication always succeeds
                // in today's synchronous parse-under-the-lock world. Clone
                // (not move) so `snapshot` is still available below to
                // build the `PublishedParseTicket` for
                // `run_post_parse_side_effects`.
                doc_state.publish_parsed_if_current(next_gen, Arc::clone(&snapshot));

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

                let generation_for_index_task = doc_state.generation.clone();
                let t_commit_start = std::time::Instant::now();
                documents.insert(normalized_uri.clone(), doc_state);

                // Must drop the lock before calling publish_diagnostics
                drop(documents);
                let commit_ms = crate::runtime::timing::elapsed_ms(t_commit_start);

                // Emit the phase-1 didChange timing spans (opt-in). This is the
                // mutation critical section: every span above ran while the
                // documents lock was held, so a keystroke's read latency includes
                // the full-parse + parent-map cost recorded here.
                if timing_on {
                    use crate::runtime::timing::{TimingSpan, elapsed_ms, emit};
                    let tail = uri_tail(uri);
                    let bytes = text.len();
                    let edits = changes.len();
                    let ver = i64::from(version);
                    let total_ms = elapsed_ms(t_did_change_start);
                    for (name, ms) in [
                        ("didChange.total", total_ms),
                        ("didChange.lock_wait", lock_wait_ms),
                        ("didChange.apply_changes", apply_changes_ms),
                        ("didChange.rope_to_string", rope_to_string_ms),
                        ("didChange.full_parse", full_parse_ms),
                        ("didChange.parent_map", parent_map_ms),
                        ("didChange.incremental_doc_update", incremental_doc_update_ms),
                        ("didChange.commit", commit_ms),
                    ] {
                        emit(TimingSpan::document(name, ms, tail.clone(), ver, bytes, edits));
                    }
                }

                // Symbol reindex, workspace index, diagnostics -- shared
                // with the async parse worker's post-publish callback (see
                // `Self::run_post_parse_side_effects`). Only reached here
                // after a successful synchronous publish above, matching
                // the worker's "only after a successful, freshness-gated
                // publish" invariant. `ast_arc` is intentionally not passed
                // separately -- the ticket carries `snapshot`, and every
                // side effect derives `ast` from `snapshot.ast()` so there
                // is exactly one source of truth for it.
                self.run_post_parse_side_effects(parse_worker::PublishedParseTicket {
                    uri: uri.to_string(),
                    document_instance: generation_for_index_task,
                    generation: next_gen,
                    snapshot,
                    text: Arc::from(text.as_str()),
                    // No worker, no queue, no `finish()` settle hook on
                    // this path -- `run_post_parse_side_effects` must keep
                    // firing `notify_parse_complete` itself. See
                    // `PublishedParseTicket`'s doc comment and #3660.
                    settle_notified_by_worker: false,
                });
            }
        }

        Ok(())
    }

    /// Run post-parse side effects (symbol reindex, workspace index,
    /// diagnostics) for a just-published parse carried by `ticket`.
    ///
    /// Shared by the synchronous fallback path (called inline, after its
    /// own `publish_parsed_if_current`) and the async parse worker's
    /// post-publish callback (`LspServer::install_default_parse_worker`,
    /// invoked from `parse_worker::process_job` only after a successful
    /// freshness-gated publish). Every call site must only reach this
    /// method after a publish it knows to have succeeded -- a rejected
    /// publish must never call this, or a stale generation's diagnostics /
    /// index entry / symbol table would leak past the freshness gate that
    /// exists specifically to prevent that.
    ///
    /// ## Publication validity != side-effect validity
    ///
    /// A successful `publish_parsed_if_current` proves the snapshot was
    /// current *at publish time* -- it does not prove these *deferred*
    /// side effects are still current by the time they actually commit.
    /// Between the async worker's publish-lock release and each side effect
    /// below actually running, a newer edit can land and supersede
    /// `ticket.generation`. Every mutating side effect therefore commits
    /// through [`Self::commit_parse_effect_if_current`] -- the single
    /// sanctioned oracle that re-validates freshness immediately before
    /// commit, not merely once at this method's entry. This makes "a side
    /// effect forgot to re-check freshness at all" structurally impossible
    /// to introduce by accident: there is no other sanctioned way to write
    /// parse-derived state, so a future side effect either goes through the
    /// oracle or has no path to commit at all. It does NOT make the
    /// check-then-commit sequence atomic -- see the TOCTOU note on
    /// [`document_generation_still_current`] for the residual (nanosecond-
    /// scale, `documents.lock()`-released-before-`commit()`-runs) window a
    /// newer edit can still land in.
    pub(crate) fn run_post_parse_side_effects(&self, ticket: parse_worker::PublishedParseTicket) {
        let ast_arc = ticket.snapshot.ast().cloned();

        let symbols_committed = self.commit_parse_effect_if_current(&ticket, || {
            if let Some(ref ast) = ast_arc {
                self.reindex_document_symbols(&ticket.uri, ast, &ticket.text);
            } else {
                self.clear_document_symbols(&ticket.uri);
            }
        });

        if symbols_committed.is_none() {
            // Stale by the time these side effects were about to commit --
            // a newer edit already superseded `ticket.generation`. Skip
            // every remaining mutating effect below; only keep the
            // coordinator's completion bookkeeping consistent (mirrors the
            // pre-existing "still notify completion even if discarding, to
            // keep coordinator state consistent" precedent in the
            // synchronous fallback path's own stale-parse discard branch)
            // -- unless the async worker's settle hook already owns this
            // lifecycle's decrement (see `PublishedParseTicket` and #3660).
            if !ticket.settle_notified_by_worker {
                #[cfg(feature = "workspace")]
                if let Some(coordinator) = self.coordinator() {
                    coordinator.notify_parse_complete(&ticket.uri);
                }
            }
            return;
        }

        // Index symbols for workspace search.
        // Indexing runs in a background task so the handler returns
        // immediately; `notify_parse_complete` is called inside the task.
        //
        // This task is the highest-risk deferred side effect: it can run
        // arbitrarily later than the other side effects above (scheduled on
        // the blocking pool or run inline), so its own commit-time oracle
        // call -- not just the entry-point check above -- is load-bearing,
        // not defense-in-depth.
        if ast_arc.is_some() {
            #[cfg(feature = "workspace")]
            if let Some(coordinator) = self.coordinator() {
                if let Ok(url) = url::Url::parse(&ticket.uri) {
                    let workspace_index = Arc::clone(coordinator.index());
                    let coordinator_clone = Arc::clone(coordinator);
                    let doc_content = ticket.text.to_string();
                    let uri_owned = ticket.uri.clone();
                    let normalized_uri_owned = self.normalize_uri_key(&ticket.uri);
                    let documents_for_task =
                        crate::runtime::parse_worker::DocumentsHandle(Arc::clone(&self.documents));
                    let expected_generation = ticket.generation;
                    let document_instance = Arc::clone(&ticket.document_instance);
                    let task_counter = Arc::clone(&self.pending_index_task_count);
                    let settle_notified_by_worker = ticket.settle_notified_by_worker;
                    task_counter.fetch_add(1, Ordering::SeqCst);

                    let task = move || {
                        // The SAME sanctioned oracle, called at THIS task's
                        // own (much later) commit boundary -- see
                        // `commit_parse_effect_if_current`.
                        let indexed = commit_parse_effect_if_current(
                            &documents_for_task,
                            &normalized_uri_owned,
                            expected_generation,
                            &document_instance,
                            || {
                                if let Err(e) = workspace_index.index_file_with_generation(
                                    url,
                                    doc_content,
                                    expected_generation,
                                ) {
                                    tracing::warn!("Failed to index file {}: {}", uri_owned, e);
                                }
                            },
                        );
                        if indexed.is_none() {
                            tracing::debug!(
                                uri = %uri_owned,
                                expected_generation,
                                "Skipping stale background index task after document close/change"
                            );
                        }
                        // See `PublishedParseTicket` and #3660: the async
                        // worker's settle hook already owns this
                        // lifecycle's decrement when `true`.
                        if !settle_notified_by_worker {
                            coordinator_clone.notify_parse_complete(&uri_owned);
                        }
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

                    // Fast path: immediately publish parse-error diagnostics so
                    // syntax errors appear before the slow debounce fires.
                    // The debounced full publish replaces this notification.
                    self.commit_parse_effect_if_current(&ticket, || {
                        self.publish_parse_errors_fast(&ticket.uri);
                    });
                    // Send full diagnostics (debounced); coordinator completion is async.
                    self.commit_parse_effect_if_current(&ticket, || {
                        self.publish_diagnostics_debounced(&ticket.uri);
                    });
                    return;
                }
            }
        }

        // Notify coordinator synchronously when no coordinator/URL/workspace
        // feature -- unless the async worker's settle hook already owns
        // this lifecycle's decrement (see `PublishedParseTicket` and #3660).
        if !ticket.settle_notified_by_worker {
            #[cfg(feature = "workspace")]
            if let Some(coordinator) = self.coordinator() {
                coordinator.notify_parse_complete(&ticket.uri);
            }
        }

        // Fast path: immediately publish parse-error diagnostics.
        self.commit_parse_effect_if_current(&ticket, || {
            self.publish_parse_errors_fast(&ticket.uri);
        });
        // Send full diagnostics (use original URI for client notification)
        // Debounced: coalesces rapid typing into a single publication
        self.commit_parse_effect_if_current(&ticket, || {
            self.publish_diagnostics_debounced(&ticket.uri);
        });
    }

    /// The ONLY sanctioned way to commit a deferred post-parse side effect.
    ///
    /// Every side effect derived from a [`parse_worker::PublishedParseTicket`]
    /// -- diagnostics, document-symbol reindex, workspace-index replacement,
    /// symbol-cache updates, semantic-fact publication, any
    /// freshness-claiming trace -- routes through this function rather than
    /// hand-rolling its own generation re-check. Re-validates document
    /// instance identity + generation freshness immediately before `commit`
    /// runs (not merely when the ticket was constructed, and not merely once
    /// at some earlier "entry point") via [`document_generation_still_current`].
    /// Runs `commit` and returns `Some` only if `ticket`'s
    /// `(document_instance, generation)` still matched the live document at
    /// the instant the check ran; otherwise the effect is dropped entirely
    /// and this returns `None`.
    ///
    /// NOT atomic with `commit` itself: the check takes `documents.lock()`,
    /// reads, and releases it before `commit` runs (`commit` closures do
    /// I/O -- notifications, index writes -- that must not run while holding
    /// the documents lock, or every side effect would serialize behind it
    /// and defeat the point of moving parse work off that lock). A newer
    /// edit can therefore still land in the gap between the check passing
    /// and `commit` actually writing. In practice this window is
    /// nanoseconds wide (no `.await`, no I/O, no blocking call between the
    /// check returning and `commit()` being invoked) and is not eliminated,
    /// only made vanishingly unlikely; a caller that needs a hard guarantee
    /// must not rely on this function for it.
    pub(crate) fn commit_parse_effect_if_current<T>(
        &self,
        ticket: &parse_worker::PublishedParseTicket,
        commit: impl FnOnce() -> T,
    ) -> Option<T> {
        let normalized_uri = self.normalize_uri_key(&ticket.uri);
        commit_parse_effect_if_current(
            &self.documents,
            &normalized_uri,
            ticket.generation,
            &ticket.document_instance,
            commit,
        )
    }
}

/// Whether `generation` (identified by `generation_handle`) is still the
/// current generation of the document stored at `normalized_uri`.
///
/// A deferred side effect (symbol reindex, workspace-index mutation,
/// diagnostics) must re-validate freshness at its own commit point, not
/// only trust that the `ParsedSnapshot` it was derived from published
/// successfully at some earlier point in time -- a newer edit can supersede
/// `generation` in the window between that publish and this side effect
/// actually committing. Two independent checks are required:
///
/// - **Document-instance identity** (`Arc::ptr_eq`): closes the
///   close/reopen ABA hole -- a didClose+didOpen cycle on the same URI
///   installs a brand-new `DocumentState` with a fresh `Arc<AtomicU32>`
///   generation counter that could coincidentally reach the same numeric
///   value `generation_handle` is still holding.
/// - **Live generation number**: even for the *same* document instance, a
///   later edit bumps the generation past `generation`.
///
/// Both must hold, checked together under one `documents.lock()`
/// acquisition, for the side effect to be considered still valid to commit.
/// A document that has been closed entirely (removed from the map) also
/// fails this check, since `documents.get(normalized_uri)` returns `None`.
pub(crate) fn document_generation_still_current(
    documents: &Mutex<HashMap<String, DocumentState>>,
    normalized_uri: &str,
    generation: u32,
    generation_handle: &Arc<AtomicU32>,
) -> bool {
    let docs = documents.lock();
    docs.get(normalized_uri).is_some_and(|doc| {
        Arc::ptr_eq(&doc.generation, generation_handle) && doc.current_generation() == generation
    })
}

/// Free-function core of the single sanctioned post-parse side-effect
/// oracle -- see [`LspServer::commit_parse_effect_if_current`] for the
/// `&self` convenience wrapper most call sites use. This standalone form
/// exists so a detached background task (which does not carry `&self`,
/// only the individually `Arc`-cloned pieces it needs -- see the
/// background workspace-index task in
/// [`LspServer::run_post_parse_side_effects`]) can still commit through the
/// exact same freshness check rather than a hand-rolled duplicate.
pub(crate) fn commit_parse_effect_if_current<T>(
    documents: &Mutex<HashMap<String, DocumentState>>,
    normalized_uri: &str,
    generation: u32,
    generation_handle: &Arc<AtomicU32>,
    commit: impl FnOnce() -> T,
) -> Option<T> {
    if document_generation_still_current(documents, normalized_uri, generation, generation_handle) {
        Some(commit())
    } else {
        None
    }
}

#[cfg(test)]
mod tests;
