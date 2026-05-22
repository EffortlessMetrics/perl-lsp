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
use perl_parser_core::source_file::is_binary_content;

mod document_state;
mod lifecycle;
mod srp_helpers;
mod symbols;
use document_state::{empty_state, minimal_state, minimal_state_from_rope};
#[cfg(feature = "incremental")]
use srp_helpers::build_incremental_edit_set;
use srp_helpers::{is_embedded_template_uri, is_perl_language_id};

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
            let generation = Arc::new(AtomicU32::new(0));

            // Initialize incremental document from the already-parsed text (didOpen).
            // code_slice is applied here to match what the full parser sees.
            #[cfg(feature = "incremental")]
            let incremental_doc = {
                use perl_parser::incremental::incremental_document::IncrementalDocument;
                let code_text = crate::util::code_slice(text);
                match IncrementalDocument::new(code_text.to_string()) {
                    Ok(doc) => Some(doc),
                    Err(e) => {
                        tracing::warn!(
                            "Incremental parsing init failed for {}, falling back to full parsing: {}",
                            uri,
                            e
                        );
                        None
                    }
                }
            };

            // Initialize IncrementalState for the didChange checkpoint fast-path (Gap A, #2080).
            // This state tracks lexer checkpoints so that small ranged edits re-lex from the
            // nearest safe boundary rather than offset 0.
            #[cfg(feature = "incremental")]
            let incremental_state = {
                use perl_parser::incremental::IncrementalState;
                let code_text = crate::util::code_slice(text);
                Some(IncrementalState::new(code_text.to_string()))
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
                    generation: Arc::clone(&generation),
                    degradation_tier,
                    #[cfg(feature = "incremental")]
                    incremental_doc,
                    #[cfg(feature = "incremental")]
                    incremental_state,
                },
            );

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
                // Get current document state or create new one
                let mut documents = self.documents.lock();
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

                // Invalidate the SemanticAnalyzer cache for this URI — content is changing.
                {
                    let mut cache = self.semantic_analyzer_cache.lock();
                    cache.retain(|(cached_uri, _), _| cached_uri != &normalized_uri);
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
                    && doc_state.degradation_tier == DegradationTier::Minimal;

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
                apply_changes(&mut doc, &lsp_changes, PosEnc::Utf16);

                let text = doc.rope.to_string();
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

                // Update or reinitialize IncrementalDocument for the new text.
                // - Ranged edits: apply to existing incremental_doc (fast path).
                // - Full replace or no existing doc: reinitialize from new text (fallback).
                // Clone the edit set so that the incremental_state block below can also use it.
                #[cfg(feature = "incremental")]
                let incremental_edits_opt_clone = incremental_edits_opt.clone();
                #[cfg(feature = "incremental")]
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
                #[cfg(feature = "incremental")]
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
                    #[cfg(feature = "incremental")]
                    incremental_state,
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

                let generation_for_index_task = doc_state.generation.clone();
                documents.insert(normalized_uri.clone(), doc_state);

                // Must drop the lock before calling publish_diagnostics
                drop(documents);

                if let Some(ref ast) = ast_arc {
                    self.reindex_document_symbols(uri, ast, &text);
                } else {
                    self.clear_document_symbols(uri);
                }

                // Index symbols for workspace search.
                // Indexing runs in a background task so the handler returns
                // immediately; `notify_parse_complete` is called inside the task.
                if let Some(ref _ast) = ast_arc {
                    #[cfg(feature = "workspace")]
                    if let Some(coordinator) = self.coordinator() {
                        if let Ok(url) = url::Url::parse(uri) {
                            let workspace_index = Arc::clone(coordinator.index());
                            let coordinator_clone = Arc::clone(coordinator);
                            let doc_content = text.clone();
                            let uri_owned = uri.to_string();
                            let expected_generation = next_gen;
                            let generation = Arc::clone(&generation_for_index_task);
                            let task_counter = Arc::clone(&self.pending_index_task_count);
                            task_counter.fetch_add(1, Ordering::SeqCst);

                            let task = move || {
                                if generation.load(Ordering::Acquire) != expected_generation {
                                    tracing::debug!(
                                        uri = %uri_owned,
                                        expected_generation,
                                        "Skipping stale background index task after document close/change"
                                    );
                                    coordinator_clone.notify_parse_complete(&uri_owned);
                                    task_counter.fetch_sub(1, Ordering::SeqCst);
                                    return;
                                }
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

                            // Fast path: immediately publish parse-error diagnostics so
                            // syntax errors appear before the slow debounce fires.
                            // The debounced full publish replaces this notification.
                            self.publish_parse_errors_fast(uri);
                            // Send full diagnostics (debounced); coordinator completion is async.
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

                // Fast path: immediately publish parse-error diagnostics.
                self.publish_parse_errors_fast(uri);
                // Send full diagnostics (use original URI for client notification)
                // Debounced: coalesces rapid typing into a single publication
                self.publish_diagnostics_debounced(uri);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests;
