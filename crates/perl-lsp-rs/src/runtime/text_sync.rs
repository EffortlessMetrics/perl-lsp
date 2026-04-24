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
use std::path::Path;

const TEMPLATE_EXTENSIONS: [&str; 4] = ["ep", "tt", "tt2", "mason"];

fn is_embedded_template_uri(uri: &str) -> bool {
    let extension = url::Url::parse(uri)
        .ok()
        .and_then(|url| url.to_file_path().ok())
        .and_then(|path| path.extension().and_then(|ext| ext.to_str()).map(str::to_owned))
        .or_else(|| Path::new(uri).extension().and_then(|ext| ext.to_str()).map(str::to_owned));

    extension.is_some_and(|ext| {
        TEMPLATE_EXTENSIONS.iter().any(|candidate| candidate.eq_ignore_ascii_case(&ext))
    })
}

fn is_perl_language_id(language_id: &str) -> bool {
    matches!(
        language_id.to_ascii_lowercase().as_str(),
        "perl" | "perl5" | "perl-cpanfile" | "embedded-perl" | "mojolicious"
    )
}

#[cfg(feature = "incremental")]
fn build_incremental_edit_set(
    original_rope: &ropey::Rope,
    lsp_changes: &[lsp_types::TextDocumentContentChangeEvent],
) -> Option<perl_parser::incremental::incremental_edit::IncrementalEditSet> {
    use crate::textdoc::{PosEnc, range_to_bytes, range_to_chars};
    use perl_parser::incremental::incremental_edit::{IncrementalEdit, IncrementalEditSet};

    let mut working_rope = original_rope.clone();
    let mut edit_set = IncrementalEditSet::new();
    // Track the cumulative byte shift introduced by all prior edits so we can
    // map evolving-document byte offsets back to original-document space.
    //
    // `apply_edits` / `apply_to_string` sort edits in *reverse* `start_byte`
    // order and apply them against the *original* source string.  All byte
    // offsets stored in `IncrementalEditSet` must therefore be in
    // original-document space, not in the space of the progressively-mutated
    // working rope.
    let mut cumulative_shift: isize = 0;

    for change in lsp_changes {
        let range = change.range.as_ref()?;
        // Measure byte positions against the evolving rope so that the
        // character-to-byte mapping for this edit is correct (the working rope
        // already reflects all preceding edits in this notification batch).
        let (evolving_start, evolving_end) = range_to_bytes(&working_rope, range, PosEnc::Utf16);

        // Map back to original-document space by undoing the byte shift that
        // prior edits introduced into the working rope.
        let orig_start = (evolving_start as isize - cumulative_shift) as usize;
        let orig_end = (evolving_end as isize - cumulative_shift) as usize;
        edit_set.add(IncrementalEdit::new(orig_start, orig_end, change.text.clone()));

        // Apply this edit to the working rope so the next iteration's
        // `range_to_bytes` / `range_to_chars` calls see the correct document.
        let (start_char, end_char) = range_to_chars(&working_rope, range, PosEnc::Utf16);
        if start_char <= end_char {
            working_rope.remove(start_char..end_char);
            working_rope.insert(start_char, &change.text);
        }

        // Accumulate the byte delta: positive for insertions, negative for deletions.
        cumulative_shift +=
            change.text.len() as isize - (evolving_end as isize - evolving_start as isize);
    }

    if edit_set.is_empty() { None } else { Some(edit_set) }
}

impl LspServer {
    fn extract_symbol_names_for_index(
        &self,
        ast: &perl_parser::ast::Node,
        text: &str,
    ) -> Vec<String> {
        let extractor = crate::symbol::SymbolExtractor::new_with_source(text);
        let symbol_table = extractor.extract(ast);
        symbol_table
            .symbols
            .values()
            .flat_map(|symbols| symbols.iter().map(|symbol| symbol.name.clone()))
            .collect()
    }

    fn replace_document_symbol_index(&self, uri: &str, ast: &perl_parser::ast::Node, text: &str) {
        let symbols = self.extract_symbol_names_for_index(ast, text);
        self.symbol_index.lock().set_document_symbols(uri, symbols);
    }

    fn clear_document_symbol_index(&self, uri: &str) {
        self.symbol_index.lock().remove_document(uri);
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
                        #[cfg(feature = "incremental")]
                        incremental_state: None,
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

                self.clear_document_symbol_index(uri);
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
                        #[cfg(feature = "incremental")]
                        incremental_state: None,
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

                self.clear_document_symbol_index(uri);
                return Ok(());
            }

            // Binary content guard: skip parsing for binary files.
            // Detection is centralized in `perl_source_file::is_binary_content`.
            if is_binary_content(text) {
                tracing::warn!(
                    "Skipping parse for {} (binary content detected: null bytes present)",
                    uri
                );

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
                        #[cfg(feature = "incremental")]
                        incremental_state: None,
                    },
                );

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

                self.clear_document_symbol_index(uri);
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
                    generation: Arc::new(AtomicU32::new(0)),
                    degradation_tier,
                    #[cfg(feature = "incremental")]
                    incremental_doc,
                    #[cfg(feature = "incremental")]
                    incremental_state,
                },
            );

            // Index symbols for workspace search
            // Note: Indexing is a MUTATION operation - use coordinator.index() directly
            // This must happen BEFORE notify_parse_complete to keep work inside the tracking window
            if let Some(ref ast) = ast_arc {
                self.replace_document_symbol_index(uri, ast, text);

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
                } else {
                    self.clear_document_symbol_index(uri);
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
            let incoming_version_i64 =
                params.pointer("/textDocument/version").and_then(|v| v.as_i64());
            let incoming_version = incoming_version_i64.and_then(|v| i32::try_from(v).ok());

            // Cancel any active streaming inline completion sessions for this URI
            // that are older than the new document version.
            if let Some(version) = incoming_version_i64 {
                self.stream_sessions().cancel_for_uri_version(uri, version);
            } else {
                self.stream_sessions().cancel_for_uri(uri);
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

                let mut doc_state = existing_doc.unwrap_or_else(|| DocumentState {
                    rope: ropey::Rope::new(),
                    text: String::new(),
                    version: incoming_version.unwrap_or(0),
                    ast: None,
                    parse_errors: vec![],
                    parent_map: ParentMap::default(),
                    line_starts: LineStartsCache::new(""),
                    generation: Arc::new(AtomicU32::new(0)),
                    degradation_tier: DegradationTier::Minimal,
                    #[cfg(feature = "incremental")]
                    incremental_doc: None,
                    #[cfg(feature = "incremental")]
                    incremental_state: None,
                });

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
                        #[cfg(feature = "incremental")]
                        incremental_state: None,
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

                    self.clear_document_symbol_index(uri);
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
                        #[cfg(feature = "incremental")]
                        incremental_state: None,
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

                // Binary content guard: skip parsing for binary files.
                // Detection is centralized in `perl_source_file::is_binary_content`.
                if is_binary_content(&text) {
                    tracing::warn!(
                        "Skipping parse for {} (binary content detected via didChange)",
                        uri
                    );

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
                        #[cfg(feature = "incremental")]
                        incremental_state: None,
                    };
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

                documents.insert(normalized_uri.clone(), doc_state);

                // Must drop the lock before calling publish_diagnostics
                drop(documents);

                // Index symbols for workspace search.
                // Indexing runs in a background task so the handler returns
                // immediately; `notify_parse_complete` is called inside the task.
                if let Some(ref ast) = ast_arc {
                    self.replace_document_symbol_index(uri, ast, &text);
                    #[cfg(feature = "workspace")]
                    if let Some(coordinator) = self.coordinator() {
                        if let Ok(url) = url::Url::parse(uri) {
                            let workspace_index = Arc::clone(coordinator.index());
                            let coordinator_clone = Arc::clone(coordinator);
                            let doc_content = text.clone();
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

                            // Fast path: immediately publish parse-error diagnostics so
                            // syntax errors appear before the slow debounce fires.
                            // The debounced full publish replaces this notification.
                            self.publish_parse_errors_fast(uri);
                            // Send full diagnostics (debounced); coordinator completion is async.
                            self.publish_diagnostics_debounced(uri);
                            return Ok(());
                        }
                    }
                } else {
                    self.clear_document_symbol_index(uri);
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

            // Cancel any active streaming inline completion sessions for this URI.
            self.stream_sessions().cancel_for_uri(uri);

            // Invalidate the SemanticAnalyzer cache for this URI on close.
            {
                let mut cache = self.semantic_analyzer_cache.lock();
                cache.retain(|(cached_uri, _), _| cached_uri != &normalized_uri);
            }

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

            self.clear_document_symbol_index(uri);

            // Notify coordinator that cleanup is complete
            #[cfg(feature = "workspace")]
            if let Some(coordinator) = self.coordinator() {
                coordinator.notify_parse_complete(uri);
            }

            // Clear diagnostics for this file using centralized notify
            if let Err(e) = self.notify(
                "textDocument/publishDiagnostics",
                json!({
                    "uri": uri,
                    "diagnostics": []
                }),
            ) {
                tracing::warn!("Failed to clear diagnostics for {}: {}", uri, e);
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
                    let source_path = source_path_from_uri(uri);
                    let diagnostics = provider.get_diagnostics_with_path(
                        ast,
                        &doc.parse_errors,
                        &doc.text,
                        None,
                        &[],
                        source_path.as_deref(),
                    );

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
                            "uri": uri,
                            "diagnostics": lsp_diagnostics
                        }),
                    ) {
                        tracing::warn!("Failed to publish diagnostics for {}: {}", uri, e);
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
    use std::io::{self, Write};
    use std::sync::Arc as StdArc;
    use std::time::Duration;

    /// Shared-buffer writer for capturing outbound notifications in tests.
    struct SharedVecWriter {
        inner: StdArc<parking_lot::Mutex<Vec<u8>>>,
    }

    impl Write for SharedVecWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.inner.lock().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn make_server_with_capture() -> (LspServer, StdArc<parking_lot::Mutex<Vec<u8>>>) {
        let buf = StdArc::new(parking_lot::Mutex::new(Vec::<u8>::new()));
        let writer = SharedVecWriter { inner: StdArc::clone(&buf) };
        let server =
            LspServer::with_io(Box::new(std::io::Cursor::new(Vec::<u8>::new())), Box::new(writer));
        (server, buf)
    }

    #[cfg(feature = "incremental")]
    #[test]
    fn test_build_incremental_edits_uses_evolving_document_ranges() {
        use lsp_types::{Position, Range, TextDocumentContentChangeEvent};

        // Original text: "abcde" (all ASCII — one byte per character)
        let original_str = "abcde";
        let original = ropey::Rope::from_str(original_str);
        let changes = vec![
            // Edit 0: insert "X" at char 1 (between 'a' and 'b').
            // After this edit the working document becomes "aXbcde".
            TextDocumentContentChangeEvent {
                range: Some(Range {
                    start: Position { line: 0, character: 1 },
                    end: Position { line: 0, character: 1 },
                }),
                range_length: None,
                text: "X".to_string(),
            },
            // Edit 1: replace chars 4..6 on the *post-insert* document "aXbcde".
            // Characters 4..6 of "aXbcde" are "de".  In original-doc space that
            // maps to bytes 3..5 (we subtract the +1 shift from the prior insert).
            TextDocumentContentChangeEvent {
                range: Some(Range {
                    start: Position { line: 0, character: 4 },
                    end: Position { line: 0, character: 6 },
                }),
                range_length: None,
                text: "YZ".to_string(),
            },
        ];

        let edit_set =
            build_incremental_edit_set(&original, &changes).expect("expected ranged edit set");
        assert_eq!(edit_set.edits.len(), 2);

        // Edit 0 is a pure insertion — original-space offsets are 1..1.
        assert_eq!(edit_set.edits[0].start_byte, 1, "edit[0] start_byte must be in original space");
        assert_eq!(
            edit_set.edits[0].old_end_byte, 1,
            "edit[0] old_end_byte must be in original space (insertion)"
        );

        // Edit 1 in evolving space was 4..6, but the prior insert added 1 byte,
        // so in original-document space the range is 3..5 (the "de" suffix).
        assert_eq!(
            edit_set.edits[1].start_byte, 3,
            "edit[1] start_byte must be mapped back to original-doc space"
        );
        assert_eq!(
            edit_set.edits[1].old_end_byte, 5,
            "edit[1] old_end_byte must be mapped back to original-doc space"
        );

        // Crucially, applying the edit set to the original source must produce
        // the same document that the LSP client intended.  `apply_to_string`
        // sorts edits in reverse start_byte order and applies them against the
        // original string — this only works when all offsets are in
        // original-document space.
        //
        // Expected sequence:
        //   1. apply edit[1] (highest start_byte=3): "abcde"[3..5] → "YZ"  ⟹ "abcYZ"
        //   2. apply edit[0] (start_byte=1):         "abcYZ"[1..1] ← "X"   ⟹ "aXbcYZ"
        let result = edit_set.apply_to_string(original_str);
        assert_eq!(result, "aXbcYZ", "apply_to_string must reproduce the client-intended document");
    }

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

    /// Verify that an empty contentChanges array does not crash and leaves the document intact.
    /// The server must handle no-op change notifications gracefully.
    #[cfg(feature = "incremental")]
    #[test]
    fn test_incremental_empty_content_changes() -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::new();
        let uri = "file:///test_inc_empty_changes.pl";
        let text = "my $x = 1;\n";

        server.did_open(json!({
            "textDocument": { "uri": uri, "languageId": "perl", "version": 1, "text": text }
        }))?;

        // Send a didChange with an empty contentChanges array (no-op notification)
        server.handle_did_change(Some(json!({
            "textDocument": { "uri": uri, "version": 2 },
            "contentChanges": []
        })))?;

        let docs = server.documents.lock();
        let doc = docs.get(uri).ok_or("document not stored after empty change")?;
        // Text must be unchanged
        assert_eq!(doc.text, text, "empty contentChanges must not modify document text");
        // incremental_doc must still be present (reinit from same text is fine)
        assert!(
            doc.incremental_doc.is_some(),
            "incremental_doc must be present after no-op change"
        );
        Ok(())
    }

    #[test]
    fn test_did_change_ranged_edit_ignored_for_unopened_document()
    -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::new();
        let uri = "file:///not-opened.pl";

        server.handle_did_change(Some(json!({
            "textDocument": { "uri": uri, "version": 1 },
            "contentChanges": [{
                "range": {
                    "start": { "line": 0, "character": 0 },
                    "end":   { "line": 0, "character": 0 }
                },
                "text": "my $x = 1;\n"
            }]
        })))?;

        let docs = server.documents.lock();
        assert!(docs.get(uri).is_none(), "ranged didChange for unopened docs must be ignored");
        Ok(())
    }

    /// Verify that an edit at the very end of the document (zero-length insertion) is handled.
    /// This is the most common case for autocompletion triggers.
    #[cfg(feature = "incremental")]
    #[test]
    fn test_incremental_insert_at_end_of_document() -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::new();
        let uri = "file:///test_inc_insert_end.pl";
        let text = "my $x = 1;\n";

        server.did_open(json!({
            "textDocument": { "uri": uri, "languageId": "perl", "version": 1, "text": text }
        }))?;

        // Insert a new line at the end (line 1, char 0 — past the only line)
        server.handle_did_change(Some(json!({
            "textDocument": { "uri": uri, "version": 2 },
            "contentChanges": [{
                "range": {
                    "start": { "line": 1, "character": 0 },
                    "end":   { "line": 1, "character": 0 }
                },
                "text": "my $y = 2;\n"
            }]
        })))?;

        let docs = server.documents.lock();
        let doc = docs.get(uri).ok_or("document not stored after end-of-doc insert")?;
        assert!(doc.text.contains("$y"), "new line must appear in document text");
        assert!(
            doc.incremental_doc.is_some(),
            "incremental_doc must survive end-of-document insert"
        );
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

    /// Verify that the `incremental_state` fast-path field is initialized on
    /// `didOpen` and survives a ranged `didChange` (Gap A wiring, issue #2080).
    ///
    /// This test fails before the `IncrementalState` field is wired into
    /// `DocumentState` and confirmed after it is. It also verifies that the
    /// incremental fast path produces a `reparsed_bytes` count less than the
    /// full document size, proving checkpoint recovery ran.
    #[cfg(feature = "incremental")]
    #[test]
    fn test_incremental_state_wired_into_did_change() -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::new();
        let uri = "file:///test_inc_state_gap_a.pl";

        // Build a document large enough to have checkpoints before the edit site.
        let mut lines: Vec<String> = (0..30).map(|i| format!("my $var_{i} = {i};")).collect();
        let text = lines.join("\n") + "\n";

        server.did_open(json!({
            "textDocument": { "uri": uri, "languageId": "perl", "version": 1, "text": text }
        }))?;

        // After didOpen, incremental_state must be initialized.
        {
            let docs = server.documents.lock();
            let doc = docs.get(uri).ok_or("document not stored after didOpen")?;
            assert!(
                doc.incremental_state.is_some(),
                "incremental_state must be initialized on didOpen (Gap A wiring absent)"
            );
            let state = doc.incremental_state.as_ref().unwrap();
            assert!(
                state.lex_checkpoints.len() > 1,
                "IncrementalState must have lex checkpoints after initial parse, got {}",
                state.lex_checkpoints.len()
            );
        }

        // Edit the last line: change `my $var_29 = 29;` -> `my $var_29 = 999;`
        // A checkpoint before the edit site means we should reparse < full doc.
        let edit_line = lines.len() as u64 - 1;
        lines[29] = "my $var_29 = 999;".to_string();

        server.handle_did_change(Some(json!({
            "textDocument": { "uri": uri, "version": 2 },
            "contentChanges": [{
                "range": {
                    "start": { "line": edit_line, "character": 13 },
                    "end":   { "line": edit_line, "character": 15 }
                },
                "text": "999"
            }]
        })))?;

        // After didChange, incremental_state must survive and source must be updated.
        {
            let docs = server.documents.lock();
            let doc = docs.get(uri).ok_or("document not stored after didChange")?;
            assert!(
                doc.incremental_state.is_some(),
                "incremental_state must survive a ranged edit (Gap A wiring absent)"
            );
            let state = doc.incremental_state.as_ref().unwrap();
            assert!(
                state.source.contains("999"),
                "incremental_state.source must reflect edit; got: {:?}",
                &state.source[state.source.len().saturating_sub(50)..]
            );
        }

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

    /// `new_parse_token` must cancel the previous flag when called a second time
    /// for the same URI and return a fresh `false` flag.
    #[test]
    fn test_new_parse_token_cancels_previous_flag() {
        let server = LspServer::new();
        let uri = "file:///test_cancel_token.pl";

        let first = server.new_parse_token(uri);
        assert!(!first.load(Ordering::Relaxed), "first token must start false");

        // Second call for same URI must set the first flag to true.
        let second = server.new_parse_token(uri);
        assert!(first.load(Ordering::Relaxed), "first token must be cancelled after second call");
        assert!(!second.load(Ordering::Relaxed), "second token must start false");

        // Third call cancels second, returns fresh third.
        let third = server.new_parse_token(uri);
        assert!(second.load(Ordering::Relaxed), "second token must be cancelled after third call");
        assert!(!third.load(Ordering::Relaxed), "third token must start false");
    }

    /// Different URIs must not interfere with each other's cancellation tokens.
    #[test]
    fn test_new_parse_token_is_per_uri() {
        let server = LspServer::new();
        let uri_a = "file:///a.pl";
        let uri_b = "file:///b.pl";

        let token_a = server.new_parse_token(uri_a);
        let token_b = server.new_parse_token(uri_b);

        // Issuing a second token for uri_b must not affect uri_a's token.
        let _token_b2 = server.new_parse_token(uri_b);
        assert!(
            !token_a.load(Ordering::Relaxed),
            "uri_a token must not be cancelled by uri_b activity"
        );
        assert!(
            token_b.load(Ordering::Relaxed),
            "uri_b first token must be cancelled by uri_b second token"
        );
    }

    /// `handle_did_close` must cancel the in-flight parse flag and remove it from
    /// the map so that the entry does not leak after the document is closed.
    #[test]
    fn test_did_close_cancels_and_removes_flag() -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::new();
        let uri = "file:///test_close_cancel.pl";

        // Simulate a parse token being registered for this URI.
        let token = server.new_parse_token(uri);
        assert!(!token.load(Ordering::Relaxed), "token must start false");

        // Open document so did_close has something to clean up.
        server.handle_did_open_with_cancellation(
            Some(json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": "my $x = 1;"
                }
            })),
            None,
        )?;

        // Now issue a new token (as dispatch would do) — replaces the previous one.
        let in_flight_token = server.new_parse_token(uri);

        // Close the document.
        server.handle_did_close(Some(json!({"textDocument": {"uri": uri}})))?;

        // The in-flight token must have been cancelled by did_close.
        assert!(
            in_flight_token.load(Ordering::Relaxed),
            "did_close must set the in-flight parse flag to true"
        );

        // The flags map must be empty for this URI — no leak.
        assert!(
            !server.parse_cancel_flags.lock().contains_key(uri),
            "did_close must remove the URI entry from parse_cancel_flags"
        );

        Ok(())
    }

    /// didClose must clear diagnostics using the client-provided URI string.
    ///
    /// This preserves exact URI identity for clients that key diagnostics by
    /// the original URI representation rather than normalized equivalents.
    #[test]
    fn test_did_close_clears_diagnostics_with_original_uri()
    -> Result<(), Box<dyn std::error::Error>> {
        let (server, buf) = make_server_with_capture();
        let uri = "FILE:///test_close_uri_identity.pl";

        server.handle_did_close(Some(json!({"textDocument": {"uri": uri}})))?;
        drop(server);
        std::thread::sleep(Duration::from_millis(50));

        let text = String::from_utf8(buf.lock().clone()).unwrap_or_default();
        assert!(
            text.contains(r#""method":"textDocument/publishDiagnostics""#),
            "didClose must publish diagnostics clear notification; got: {text:?}"
        );
        assert!(
            text.contains(&format!(r#""uri":"{}""#, uri)),
            "didClose must publish diagnostics using original URI; got: {text:?}"
        );
        Ok(())
    }

    /// didSave must publish diagnostics using the original URI string.
    #[test]
    fn test_did_save_publishes_diagnostics_with_original_uri()
    -> Result<(), Box<dyn std::error::Error>> {
        let (server, buf) = make_server_with_capture();
        let uri = "FILE:///test_save_uri_identity.pl";

        server.did_open(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": "my $x = 1;\n"
            }
        }))?;

        // Ignore notifications produced by didOpen; assert only didSave payload.
        buf.lock().clear();

        server.handle_did_save(Some(json!({
            "textDocument": {"uri": uri, "version": 1}
        })))?;
        drop(server);
        std::thread::sleep(Duration::from_millis(50));

        let text = String::from_utf8(buf.lock().clone()).unwrap_or_default();
        assert!(
            text.contains(r#""method":"textDocument/publishDiagnostics""#),
            "didSave must publish diagnostics notification; got: {text:?}"
        );
        assert!(
            text.contains(&format!(r#""uri":"{}""#, uri)),
            "didSave must publish diagnostics using original URI; got: {text:?}"
        );
        Ok(())
    }

    /// A parse cancelled via a pre-set flag must return Ok(()) and not store
    /// a document, so the caller behaves as if the parse simply didn't happen.
    #[test]
    fn test_cancelled_open_returns_ok_without_storing_document()
    -> Result<(), Box<dyn std::error::Error>> {
        use std::sync::Arc;
        use std::sync::atomic::AtomicBool;

        let server = LspServer::new();
        let uri = "file:///test_cancelled_open.pl";

        // Pre-set the cancellation flag — the parse must be skipped immediately.
        let flag = Arc::new(AtomicBool::new(true));

        // Build a source large enough that parse() wouldn't return instantly
        // on its own — we rely on the pre-parse check in parse().
        let text: String = (0..200).map(|i| format!("my $x{} = {};\n", i, i)).collect();

        let result = server.handle_did_open_with_cancellation(
            Some(json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": text
                }
            })),
            Some(flag),
        );

        // The handler must return Ok (not propagate Cancelled as a JsonRpcError).
        assert!(result.is_ok(), "cancelled open must return Ok(()): {:?}", result);

        // The document must NOT have been stored (cancelled parse = no result).
        let normalized = server.normalize_uri_key(uri);
        assert!(
            !server.documents.lock().contains_key(&normalized),
            "cancelled parse must not store document state"
        );

        Ok(())
    }

    /// Binary content guard — didOpen with null bytes must skip the parser and
    /// store the document with DegradationTier::Minimal and no AST.
    #[test]
    fn test_binary_file_guard_did_open_skips_parse() -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::new();
        let uri = "file:///test_binary.pl";
        // Simulate a binary file that arrived as a valid UTF-8 string containing null bytes
        let binary_content = "PK\x00\x03some binary content\x00\x00\x00";

        server.did_open(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": binary_content
            }
        }))?;

        let docs = server.documents.lock();
        let doc = docs.get(uri).ok_or("document not stored after binary didOpen")?;
        assert_eq!(
            doc.degradation_tier,
            DegradationTier::Minimal,
            "binary content should result in Minimal degradation tier"
        );
        assert!(doc.ast.is_none(), "parser must not be called on binary content");
        Ok(())
    }

    /// Binary content guard — a single null byte is sufficient to trigger the guard.
    #[test]
    fn test_binary_file_guard_single_null_byte_triggers_guard()
    -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::new();
        let uri = "file:///test_null.pl";
        let content_with_null = "#!/usr/bin/perl\nmy $x = 1;\x00\n";

        server.did_open(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": content_with_null
            }
        }))?;

        let docs = server.documents.lock();
        let doc = docs.get(uri).ok_or("document not stored after single-null didOpen")?;
        assert_eq!(
            doc.degradation_tier,
            DegradationTier::Minimal,
            "a single null byte must trigger the binary guard"
        );
        assert!(doc.ast.is_none(), "parser must not be called when null byte is present");
        Ok(())
    }

    /// Binary content guard — normal Perl source (no null bytes) must still parse normally.
    #[test]
    fn test_binary_file_guard_normal_perl_still_parses() -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::new();
        let uri = "file:///normal.pl";

        server.did_open(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": "#!/usr/bin/perl\nuse strict;\nmy $x = 42;\n"
            }
        }))?;

        let docs = server.documents.lock();
        let doc = docs.get(uri).ok_or("document not stored after normal didOpen")?;
        assert_ne!(
            doc.degradation_tier,
            DegradationTier::Minimal,
            "normal Perl should not be treated as binary content"
        );
        Ok(())
    }

    /// Binary content guard — didChange with null bytes must skip parse and keep DegradationTier::Minimal.
    #[test]
    fn test_binary_file_guard_did_change_skips_parse() -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::new();
        let uri = "file:///test_binary_change.pl";

        // Open with valid Perl first
        server.did_open(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": "my $x = 1;\n"
            }
        }))?;

        // Full-document replace with binary content (null bytes)
        server.handle_did_change(Some(json!({
            "textDocument": { "uri": uri, "version": 2 },
            "contentChanges": [{ "text": "PK\x00\x03binary\x00data" }]
        })))?;

        let docs = server.documents.lock();
        let doc = docs.get(uri).ok_or("document not stored after binary didChange")?;
        assert_eq!(
            doc.degradation_tier,
            DegradationTier::Minimal,
            "binary content via didChange should result in Minimal degradation tier"
        );
        assert!(doc.ast.is_none(), "parser must not be called on binary content via didChange");
        Ok(())
    }

    #[test]
    fn test_template_file_guard_skips_parse_for_non_perl_language_id()
    -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::new();
        let uri = "file:///app/templates/welcome.html.ep";

        server.did_open(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "html",
                "version": 1,
                "text": "<div><%= $name %></div>"
            }
        }))?;

        let docs = server.documents.lock();
        let doc = docs.get(uri).ok_or("template document not stored after didOpen")?;
        assert_eq!(
            doc.degradation_tier,
            DegradationTier::Minimal,
            "template with non-Perl language mode should stay in no-parse mode"
        );
        assert!(doc.ast.is_none(), "template with non-Perl languageId must skip parse");
        Ok(())
    }

    #[test]
    fn test_template_file_guard_persists_across_did_change()
    -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::new();
        let uri = "file:///app/templates/welcome.html.ep";

        server.did_open(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "html",
                "version": 1,
                "text": "<div><%= $name %></div>"
            }
        }))?;

        server.handle_did_change(Some(json!({
            "textDocument": { "uri": uri, "version": 2 },
            "contentChanges": [{ "text": "<div><%= $title %></div>" }]
        })))?;

        let docs = server.documents.lock();
        let doc = docs.get(uri).ok_or("template document not stored after didChange")?;
        assert_eq!(
            doc.degradation_tier,
            DegradationTier::Minimal,
            "template should remain in no-parse mode after didChange"
        );
        assert!(doc.ast.is_none(), "template should continue skipping parse on didChange");
        Ok(())
    }

    #[test]
    fn test_template_file_guard_parses_embedded_perl_language_id()
    -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::new();
        let uri = "file:///app/templates/welcome.html.ep";

        server.did_open(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "embedded-perl",
                "version": 1,
                "text": "<%= my $name = 'world'; %>"
            }
        }))?;

        let docs = server.documents.lock();
        let doc = docs.get(uri).ok_or("template document not stored after didOpen")?;
        assert!(
            doc.ast.is_some(),
            "template with embedded-perl languageId should be parsed as Perl"
        );
        Ok(())
    }

    #[test]
    fn test_template_file_guard_parses_mojolicious_language_id()
    -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::new();
        let uri = "file:///app/templates/index.html.ep";

        server.did_open(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "mojolicious",
                "version": 1,
                "text": "% my $title = 'Hello';"
            }
        }))?;

        let docs = server.documents.lock();
        let doc = docs.get(uri).ok_or("template document not stored after didOpen")?;
        assert!(doc.ast.is_some(), "template with mojolicious languageId should be parsed as Perl");
        Ok(())
    }

    /// Semantic analyzer cache must accumulate at most one entry per document
    /// version across multiple hover calls at different offsets.
    ///
    /// This verifies the (uri, content_hash) key strategy: two hovers on the
    /// same document text must reuse the cached SemanticAnalyzer rather than
    /// constructing a fresh one.
    #[test]
    fn test_semantic_analyzer_cache_reuses_entry_on_same_version()
    -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::new();
        let uri = "file:///test_cache_hover.pl";
        let text = "my $x = 1;\nmy $y = 2;\n";

        server.did_open(json!({
            "textDocument": { "uri": uri, "languageId": "perl", "version": 1, "text": text }
        }))?;

        // Two hover calls at different positions on the same document version.
        let _ = server.handle_hover(Some(json!({
            "textDocument": { "uri": uri },
            "position": { "line": 0, "character": 3 }
        })));

        let _ = server.handle_hover(Some(json!({
            "textDocument": { "uri": uri },
            "position": { "line": 1, "character": 3 }
        })));

        // Cache must have exactly 1 entry: one per (uri, content_hash).
        let cache = server.semantic_analyzer_cache.lock();
        assert_eq!(
            cache.len(),
            1,
            "should cache exactly one analyzer entry per document version (got {})",
            cache.len()
        );

        Ok(())
    }

    /// The semantic analyzer cache must be cleared for a URI when the document
    /// changes (textDocument/didChange), so stale analysis is never served.
    #[test]
    fn test_semantic_analyzer_cache_invalidated_on_did_change()
    -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::new();
        let uri = "file:///test_cache_invalidate_change.pl";
        let text = "my $x = 1;\n";

        server.did_open(json!({
            "textDocument": { "uri": uri, "languageId": "perl", "version": 1, "text": text }
        }))?;

        // Prime the cache with a hover call.
        let _ = server.handle_hover(Some(json!({
            "textDocument": { "uri": uri },
            "position": { "line": 0, "character": 3 }
        })));

        // Verify the cache has an entry before the change.
        {
            let cache = server.semantic_analyzer_cache.lock();
            assert!(!cache.is_empty(), "cache must be populated before didChange");
        }

        // Apply a document change.
        server.handle_did_change(Some(json!({
            "textDocument": { "uri": uri, "version": 2 },
            "contentChanges": [{ "text": "my $x = 99;\n" }]
        })))?;

        // Cache must be cleared for this URI after didChange.
        let cache = server.semantic_analyzer_cache.lock();
        let uri_key = server.normalize_uri_key(uri);
        let still_has_stale = cache.keys().any(|(k, _)| k == &uri_key);
        assert!(!still_has_stale, "semantic_analyzer_cache must evict entries for changed URI");

        Ok(())
    }

    /// The semantic analyzer cache must be cleared for a URI when the document
    /// is closed (textDocument/didClose), preventing stale memory retention.
    #[test]
    fn test_semantic_analyzer_cache_invalidated_on_did_close()
    -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::new();
        let uri = "file:///test_cache_invalidate_close.pl";
        let text = "my $x = 1;\n";

        server.did_open(json!({
            "textDocument": { "uri": uri, "languageId": "perl", "version": 1, "text": text }
        }))?;

        // Prime the cache with a hover call.
        let _ = server.handle_hover(Some(json!({
            "textDocument": { "uri": uri },
            "position": { "line": 0, "character": 3 }
        })));

        // Verify the cache has an entry before the close.
        {
            let cache = server.semantic_analyzer_cache.lock();
            assert!(!cache.is_empty(), "cache must be populated before didClose");
        }

        // Close the document.
        server.handle_did_close(Some(json!({ "textDocument": { "uri": uri } })))?;

        // Cache must be cleared for this URI after didClose.
        let cache = server.semantic_analyzer_cache.lock();
        let uri_key = server.normalize_uri_key(uri);
        let still_has_stale = cache.keys().any(|(k, _)| k == &uri_key);
        assert!(!still_has_stale, "semantic_analyzer_cache must evict entries for closed URI");

        Ok(())
    }

    /// A new document version must produce a distinct cache entry (different
    /// content hash) while the old version's entry is evicted on didChange.
    #[test]
    fn test_semantic_analyzer_cache_separates_document_versions()
    -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::new();
        let uri = "file:///test_cache_versions.pl";
        let text_v1 = "my $x = 1;\n";
        let text_v2 = "my $x = 999;\n";

        // Open v1 and prime the cache.
        server.did_open(json!({
            "textDocument": { "uri": uri, "languageId": "perl", "version": 1, "text": text_v1 }
        }))?;

        let _ = server.handle_hover(Some(json!({
            "textDocument": { "uri": uri },
            "position": { "line": 0, "character": 3 }
        })));

        // Change to v2 (invalidates v1 entry) then hover again.
        server.handle_did_change(Some(json!({
            "textDocument": { "uri": uri, "version": 2 },
            "contentChanges": [{ "text": text_v2 }]
        })))?;

        let _ = server.handle_hover(Some(json!({
            "textDocument": { "uri": uri },
            "position": { "line": 0, "character": 3 }
        })));

        // Cache must have at most 1 entry (v2 only; v1 was evicted on didChange).
        let cache = server.semantic_analyzer_cache.lock();
        assert!(
            cache.len() <= 1,
            "cache must hold at most one entry after version change (got {})",
            cache.len()
        );

        Ok(())
    }

    // =========================================================================
    // Error-path tests — closes #3039
    //
    // These tests verify that each handler correctly propagates INVALID_PARAMS
    // errors when required LSP parameters are missing.  They use Result<()>
    // returns and explicit Err-branch assertions rather than #[should_panic].
    // =========================================================================

    /// handle_did_close with no textDocument.uri must return INVALID_PARAMS.
    #[test]
    fn handle_did_close_missing_uri_returns_invalid_params() {
        let server = LspServer::new();
        let result = server.handle_did_close(Some(json!({ "textDocument": {} })));
        assert!(result.is_err(), "handle_did_close must error on missing URI");
        if let Err(err) = result {
            assert_eq!(
                err.code,
                crate::protocol::INVALID_PARAMS,
                "error code must be INVALID_PARAMS; got {}",
                err.code
            );
            assert!(
                err.message.contains("textDocument.uri"),
                "error message must name the missing field; got: {}",
                err.message
            );
        }
    }

    /// handle_did_close with None params must succeed silently (no-op).
    #[test]
    fn handle_did_close_none_params_is_ok() {
        let server = LspServer::new();
        let result = server.handle_did_close(None);
        assert!(result.is_ok(), "handle_did_close with None params must not error");
    }

    /// handle_did_close for a non-existent URI must succeed silently.
    #[test]
    fn handle_did_close_unknown_uri_is_ok() {
        let server = LspServer::new();
        let result = server.handle_did_close(Some(
            json!({ "textDocument": { "uri": "file:///never_opened.pl" } }),
        ));
        assert!(result.is_ok(), "closing a document that was never opened must not error");
    }

    /// handle_did_save with no textDocument.uri must return INVALID_PARAMS.
    #[test]
    fn handle_did_save_missing_uri_returns_invalid_params() {
        let server = LspServer::new();
        let result = server.handle_did_save(Some(json!({ "textDocument": {} })));
        assert!(result.is_err(), "handle_did_save must error on missing URI");
        if let Err(err) = result {
            assert_eq!(
                err.code,
                crate::protocol::INVALID_PARAMS,
                "error code must be INVALID_PARAMS; got {}",
                err.code
            );
        }
    }

    /// handle_did_save with None params must succeed silently (no-op).
    #[test]
    fn handle_did_save_none_params_is_ok() {
        let server = LspServer::new();
        let result = server.handle_did_save(None);
        assert!(result.is_ok(), "handle_did_save with None params must not error");
    }

    /// did_open with a missing textDocument.text field must return INVALID_PARAMS.
    #[test]
    fn did_open_missing_text_returns_invalid_params() {
        let server = LspServer::new();
        let result = server.did_open(json!({
            "textDocument": {
                "uri": "file:///missing_text.pl",
                "languageId": "perl",
                "version": 1
            }
        }));
        assert!(result.is_err(), "did_open must error when textDocument.text is absent");
        if let Err(err) = result {
            assert_eq!(
                err.code,
                crate::protocol::INVALID_PARAMS,
                "error code must be INVALID_PARAMS; got {}",
                err.code
            );
        }
    }

    /// did_open with a missing textDocument.uri field must return INVALID_PARAMS.
    #[test]
    fn did_open_missing_uri_returns_invalid_params() {
        let server = LspServer::new();
        let result = server.did_open(json!({
            "textDocument": {
                "languageId": "perl",
                "version": 1,
                "text": "my $x = 1;\n"
            }
        }));
        assert!(result.is_err(), "did_open must error when textDocument.uri is absent");
        if let Err(err) = result {
            assert_eq!(
                err.code,
                crate::protocol::INVALID_PARAMS,
                "error code must be INVALID_PARAMS; got {}",
                err.code
            );
        }
    }

    /// handle_did_change with missing URI must return INVALID_PARAMS.
    #[test]
    fn handle_did_change_missing_uri_returns_invalid_params() {
        let server = LspServer::new();
        let result = server.handle_did_change(Some(json!({
            "textDocument": {},
            "contentChanges": []
        })));
        assert!(result.is_err(), "handle_did_change with missing URI must error");
        if let Err(err) = result {
            assert_eq!(
                err.code,
                crate::protocol::INVALID_PARAMS,
                "error code must be INVALID_PARAMS; got {}",
                err.code
            );
        }
    }

    /// didChange with an out-of-order version must be ignored to avoid document rollback.
    #[test]
    fn handle_did_change_ignores_stale_versions() -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::new();
        let uri = "file:///stale_version.pl";

        server.did_open(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 5,
                "text": "my $x = 1;\n"
            }
        }))?;

        // Incoming didChange version is older than current (4 < 5): ignore.
        server.handle_did_change(Some(json!({
            "textDocument": { "uri": uri, "version": 4 },
            "contentChanges": [{ "text": "my $x = 999;\n" }]
        })))?;

        let docs = server.documents.lock();
        let doc = docs.get(uri).ok_or("document missing after stale didChange")?;
        assert_eq!(doc.version, 5, "stale didChange must not update document version");
        assert_eq!(doc.text, "my $x = 1;\n", "stale didChange must not modify document text");
        Ok(())
    }

    /// didChange without a version field should still be applied for compatibility.
    #[test]
    fn handle_did_change_without_version_uses_next_version()
    -> Result<(), Box<dyn std::error::Error>> {
        let server = LspServer::new();
        let uri = "file:///missing_version.pl";

        server.did_open(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": "my $x = 1;\n"
            }
        }))?;

        server.handle_did_change(Some(json!({
            "textDocument": { "uri": uri },
            "contentChanges": [{ "text": "my $x = 2;\n" }]
        })))?;

        let docs = server.documents.lock();
        let doc = docs.get(uri).ok_or("document missing after didChange without version")?;
        assert_eq!(doc.version, 2, "missing-version didChange should advance version by one");
        assert_eq!(doc.text, "my $x = 2;\n", "didChange without version should apply content");
        Ok(())
    }
}
