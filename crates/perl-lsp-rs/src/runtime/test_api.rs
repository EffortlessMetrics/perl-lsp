//! Test-only public methods.
//!
//! These methods exist to exercise JSON-RPC routing in tests without
//! needing an external transport. They are compiled only for `cargo test`
//! or when the `expose_lsp_test_api` feature is enabled.
//!
//! They are NOT part of the supported runtime API and should not be used
//! outside of test code.

#[cfg(any(test, feature = "expose_lsp_test_api"))]
use serde_json::Value;

#[cfg(any(test, feature = "expose_lsp_test_api"))]
use super::{JsonRpcError, LspServer};

#[cfg(any(test, feature = "expose_lsp_test_api"))]
impl LspServer {
    /// Test-only entrypoint for LSP `textDocument/didOpen`.
    ///
    /// This method exercises the `didOpen` notification handler without
    /// needing an external transport. Use it in tests to simulate opening
    /// a document in the LSP server.
    ///
    /// # Parameters
    /// - `params`: JSON-RPC params containing `textDocument` with `uri`, `text`, etc.
    ///
    /// # Errors
    /// Returns [`JsonRpcError`] if params are invalid or the handler fails.
    ///
    /// # See also
    /// - [`Self::handle_did_open`] (internal handler)
    pub fn test_handle_did_open(&self, params: Option<Value>) -> Result<(), JsonRpcError> {
        self.handle_did_open(params)
    }

    /// Test-only entrypoint for LSP `textDocument/didChange`.
    ///
    /// This method exercises the real text-sync change handler without needing
    /// an external transport. Use it in tests to prove provider receipts refresh
    /// after document edits.
    ///
    /// # Parameters
    /// - `params`: JSON-RPC params containing `textDocument` and `contentChanges`.
    ///
    /// # Errors
    /// Returns [`JsonRpcError`] if params are invalid or the handler fails.
    pub fn test_handle_did_change(&self, params: Option<Value>) -> Result<(), JsonRpcError> {
        self.handle_did_change(params)
    }

    /// Convenience helper: apply a `didOpen` for plain Perl text.
    ///
    /// Returns the underlying handler error if synchronisation fails so
    /// callers can `?` it from `Result<()>`-returning tests (per the
    /// AGENTS.md test convention).
    pub fn test_apply_did_open(
        &self,
        uri: &str,
        text: &str,
        version: i32,
    ) -> Result<(), JsonRpcError> {
        let params = serde_json::json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": version,
                "text": text,
            }
        });
        self.handle_did_open(Some(params))
    }

    /// Convenience helper: apply a full-text `didChange` for plain Perl text.
    pub fn test_apply_did_change(
        &self,
        uri: &str,
        new_text: &str,
        version: i32,
    ) -> Result<(), JsonRpcError> {
        let params = serde_json::json!({
            "textDocument": { "uri": uri, "version": version },
            "contentChanges": [ { "text": new_text } ],
        });
        self.handle_did_change(Some(params))
    }

    /// Test-only helper that updates an open document snapshot without touching
    /// the workspace index.
    ///
    /// This models the post-edit window where `didChange` has made the document
    /// current but the asynchronous workspace index update has not completed.
    /// Production text sync must continue to use the real didChange handler.
    pub fn test_replace_document_without_index(
        &self,
        uri: &str,
        text: &str,
        version: i32,
    ) -> Result<(), String> {
        let normalized_uri = self.normalize_uri_key(uri);
        let mut parser = perl_parser::Parser::new(text);
        let ast = match parser.parse() {
            Ok(ast) => Some(std::sync::Arc::new(ast)),
            Err(err) => return Err(format!("Parse error: {err}")),
        };
        let errors = parser.errors().to_vec();

        let mut parent_map = perl_parser::declaration::ParentMap::default();
        if let Some(ref ast) = ast {
            crate::declaration::DeclarationProvider::build_parent_map(ast, &mut parent_map, None);
        }

        let rope = ropey::Rope::from_str(text);
        let line_starts = perl_parser::position::LineStartsCache::new_rope(&rope);
        let degradation_tier = crate::state::DegradationTier::from_parse_result(&ast, &errors);

        let mut documents = self.documents.lock();
        let doc = documents
            .get_mut(&normalized_uri)
            .ok_or_else(|| format!("document not open: {uri}"))?;
        doc.rope = rope;
        doc.text = text.to_string();
        doc.version = version;
        doc.ast = ast;
        doc.parse_errors = errors;
        doc.parent_map = parent_map;
        doc.line_starts = line_starts;
        doc.generation.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        doc.degradation_tier = degradation_tier;
        #[cfg(feature = "incremental")]
        {
            doc.incremental_doc = None;
            doc.incremental_state = None;
        }

        Ok(())
    }

    /// Test-only entrypoint for LSP `initialize`.
    pub fn test_handle_initialize_dispatch(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        self.handle_initialize(params)
    }

    /// Test-only entrypoint for the LSP `initialized` notification.
    pub fn test_handle_initialized_dispatch(&self) -> Result<Option<Value>, JsonRpcError> {
        self.handle_initialized_dispatch()
    }

    /// Return the client capabilities captured during `initialize`.
    ///
    /// This is a test-only clone of the server's negotiated capability state so
    /// integration tests can assert capability parsing without reaching through
    /// private state modules.
    pub fn test_client_capabilities(&self) -> crate::state::ClientCapabilities {
        self.client_capabilities.lock().clone()
    }

    /// Test-only entrypoint for LSP `textDocument/definition`.
    ///
    /// Exercises go-to-definition functionality in tests. Returns the
    /// definition location(s) for the symbol at the given position.
    ///
    /// # Parameters
    /// - `params`: JSON-RPC params with `textDocument.uri` and `position`.
    ///
    /// # Returns
    /// - `Ok(Some(locations))`: Definition location(s) found.
    /// - `Ok(None)`: No definition found at position.
    ///
    /// # Errors
    /// Returns [`JsonRpcError`] if params are invalid or document not found.
    pub fn test_handle_definition(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        self.handle_definition(params)
    }

    /// Test-only receipt for definition runtime quality proof.
    ///
    /// Calls the live `textDocument/definition` handler and compares that result
    /// with the compiler-fact cutover receipt from the same runtime workspace
    /// index. This does not change live navigation behavior.
    pub fn test_definition_runtime_quality_receipt(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        self.definition_runtime_quality_receipt(params)
    }

    /// Test-only entrypoint for LSP `textDocument/references`.
    ///
    /// Exercises find-references functionality in tests. Returns all
    /// locations where the symbol at the given position is referenced.
    ///
    /// # Parameters
    /// - `params`: JSON-RPC params with `textDocument.uri`, `position`, and `context`.
    ///
    /// # Returns
    /// - `Ok(Some(locations))`: Reference locations found.
    /// - `Ok(None)`: No references found.
    ///
    /// # Errors
    /// Returns [`JsonRpcError`] if params are invalid or document not found.
    pub fn test_handle_references(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        self.handle_references(params)
    }

    /// Test-only receipt for references runtime quality proof.
    ///
    /// Calls the live `textDocument/references` handler and compares that result
    /// with the compiler-fact cutover receipt from the same runtime workspace
    /// index. This does not change live navigation behavior.
    pub fn test_references_runtime_quality_receipt(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        self.references_runtime_quality_receipt(params)
    }

    /// Test-only entrypoint for LSP `textDocument/rename`.
    ///
    /// Exercises the live rename handler without needing an external transport.
    ///
    /// # Errors
    /// Returns [`JsonRpcError`] if params are invalid or rename is refused.
    pub fn test_handle_rename(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        self.handle_rename_workspace(params)
    }

    /// Test-only receipt for rename runtime blocker UX proof.
    ///
    /// Calls the live rename handler and compares it with the compiler-fact
    /// rename plan from the same runtime workspace index. This does not change
    /// live refactor behavior.
    pub fn test_rename_runtime_blocker_ux_receipt(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        self.rename_runtime_blocker_ux_receipt(params)
    }

    /// Test-only receipt for safe-delete runtime blocker UX proof.
    ///
    /// Records the compiler-fact safe-delete plan from the same runtime
    /// workspace index without introducing a live symbol-level safe-delete
    /// provider.
    pub fn test_safe_delete_runtime_blocker_ux_receipt(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        self.safe_delete_runtime_blocker_ux_receipt(params)
    }

    /// Test-only entrypoint for LSP `textDocument/completion`.
    ///
    /// Exercises completion functionality in tests. Returns completion
    /// items available at the given position.
    ///
    /// # Parameters
    /// - `params`: JSON-RPC params with `textDocument.uri` and `position`.
    ///
    /// # Returns
    /// - `Ok(Some(items))`: Completion items available.
    /// - `Ok(None)`: No completions available.
    ///
    /// # Errors
    /// Returns [`JsonRpcError`] if params are invalid or document not found.
    pub fn test_handle_completion(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        self.handle_completion(params)
    }

    /// Test-only entrypoint for LSP `textDocument/hover`.
    ///
    /// Exercises hover functionality in tests. Returns hover information
    /// (documentation, type info) for the symbol at the given position.
    ///
    /// # Parameters
    /// - `params`: JSON-RPC params with `textDocument.uri` and `position`.
    ///
    /// # Returns
    /// - `Ok(Some(hover))`: Hover information found.
    /// - `Ok(None)`: No hover info available at position.
    ///
    /// # Errors
    /// Returns [`JsonRpcError`] if params are invalid or document not found.
    pub fn test_handle_hover(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        self.handle_hover(params)
    }

    /// Test-only entrypoint for LSP `textDocument/documentSymbol`.
    ///
    /// Exercises document symbol functionality in tests. Returns the
    /// outline of symbols (packages, subs, variables) in the document.
    ///
    /// # Parameters
    /// - `params`: JSON-RPC params with `textDocument.uri`.
    ///
    /// # Returns
    /// - `Ok(Some(symbols))`: Document symbols found.
    /// - `Ok(None)`: No symbols in document.
    ///
    /// # Errors
    /// Returns [`JsonRpcError`] if params are invalid or document not found.
    pub fn test_handle_document_symbols(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        self.handle_document_symbol(params)
    }

    /// Test-only entrypoint for LSP `workspace/symbol`.
    ///
    /// Exercises workspace symbol search in tests. Returns symbols
    /// matching the query across all indexed files.
    ///
    /// # Parameters
    /// - `params`: JSON-RPC params with `query` string.
    ///
    /// # Returns
    /// - `Ok(Some(symbols))`: Matching workspace symbols.
    /// - `Ok(None)`: No matching symbols found.
    ///
    /// # Errors
    /// Returns [`JsonRpcError`] if params are invalid.
    pub fn test_handle_workspace_symbols(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        self.handle_workspace_symbols_v2(params)
    }

    /// Test-only receipt for document symbol runtime quality proof.
    ///
    /// Calls the live `textDocument/documentSymbol` handler and returns a typed
    /// quality receipt. Document symbols is in `shadowed` state — the receipt
    /// captures live provider results without changing live behavior.
    pub fn test_document_symbols_runtime_quality_receipt(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        self.document_symbols_runtime_quality_receipt(params)
    }

    /// Test-only receipt for workspace symbol runtime quality proof.
    ///
    /// Calls the live `workspace/symbol` handler and returns a typed quality
    /// receipt. Workspace symbols is in `shadowed` state — the receipt captures
    /// live provider results without changing live behavior.
    pub fn test_workspace_symbols_runtime_quality_receipt(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        self.workspace_symbols_runtime_quality_receipt(params)
    }

    /// Test-only entrypoint for LSP `textDocument/documentColor`.
    ///
    /// Exercises document color detection functionality in tests.
    ///
    /// # Parameters
    /// - `params`: JSON-RPC params with `textDocument.uri`.
    ///
    /// # Returns
    /// - `Ok(Some(colors))`: Array of ColorInformation objects.
    /// - `Ok(None)`: No colors found.
    ///
    /// # Errors
    /// Returns [`JsonRpcError`] if params are invalid or document not found.
    pub fn test_handle_document_color(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        self.handle_document_color(params)
    }

    /// Test-only entrypoint for LSP `textDocument/colorPresentation`.
    ///
    /// Exercises color presentation generation in tests.
    ///
    /// # Parameters
    /// - `params`: JSON-RPC params with `color` and `range`.
    ///
    /// # Returns
    /// - `Ok(Some(presentations))`: Array of ColorPresentation objects.
    ///
    /// # Errors
    /// Returns [`JsonRpcError`] if params are invalid.
    pub fn test_handle_color_presentation(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        self.handle_color_presentation(params)
    }

    /// Test-only entrypoint for LSP `workspace/textDocumentContent`.
    ///
    /// Exercises virtual document content functionality in tests.
    ///
    /// # Parameters
    /// - `params`: JSON-RPC params with `uri` (e.g., "perldoc://Module::Name").
    ///
    /// # Returns
    /// - `Ok(Some(content))`: Object with `text` field containing document content.
    ///
    /// # Errors
    /// Returns [`JsonRpcError`] if URI scheme is unsupported or content not found.
    pub fn test_handle_text_document_content(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        self.handle_text_document_content(params)
    }

    /// Test-only entrypoint for `textDocument/diagnostic` (pull diagnostics).
    ///
    /// Exercises the pull-diagnostics handler without needing an external transport.
    /// Returns the full diagnostics result object or `None`.
    ///
    /// # Parameters
    /// - `params`: JSON-RPC params with `textDocument.uri`.
    ///
    /// # Errors
    /// Returns [`JsonRpcError`] if params are invalid or the handler fails.
    pub fn test_handle_document_diagnostic(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        self.handle_document_diagnostic(params)
    }

    /// Test-only entrypoint for LSP `workspace/diagnostic`.
    ///
    /// Exercises the pull-style workspace-diagnostics handler without an
    /// external transport.  Used by generation-guard tests that need to drive
    /// both the document and workspace pull paths under controlled conditions.
    ///
    /// # Parameters
    /// - `params`: JSON-RPC params (`previousResultIds` array, optional).
    ///
    /// # Errors
    /// Returns [`JsonRpcError`] if params are invalid or the handler fails.
    pub fn test_handle_workspace_diagnostic(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        self.handle_workspace_diagnostic(params)
    }

    /// Return the current generation counter for an open document.
    ///
    /// Returns `None` when the document is not open.  Used by tests to read
    /// the generation before and after simulated `didChange` events so they can
    /// assert that the staleness guard does not false-positive.
    pub fn test_document_generation(&self, uri: &str) -> Option<u32> {
        self.document_generation(uri)
    }

    /// Test-only entrypoint for `workspace/didChangeConfiguration`.
    ///
    /// Applies the same configuration-update path as a real client notification.
    pub fn test_handle_did_change_configuration(&self, params: Option<Value>) {
        self.handle_did_change_configuration(params);
    }

    /// Install a mock subprocess runtime for the `CriticAnalyzer`.
    ///
    /// When set, the lazy-init path in `collect_external_perlcritic_diagnostics`
    /// constructs a `CriticAnalyzer` using this runtime instead of the OS runtime.
    /// This allows tests to exercise the full pipeline — including config-driven
    /// profile discovery — without spawning a real `perlcritic` process.
    ///
    /// Call [`Self::test_bypass_perlcritic_command_check`] alongside this to
    /// skip the `command_exists` guard.
    ///
    /// Resets the cached analyzer to `None` so the next diagnostic cycle
    /// rebuilds it with the injected runtime.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn test_install_mock_critic_runtime(
        &self,
        runtime: std::sync::Arc<dyn perl_subprocess_runtime::SubprocessRuntime>,
    ) {
        *self.critic_runtime_override.lock() = Some(runtime);
        // Reset any cached analyzer so it is rebuilt with the new runtime.
        *self.critic_analyzer.lock() = None;
    }

    /// Skip the `command_exists("perlcritic")` guard in
    /// `collect_external_perlcritic_diagnostics` for the lifetime of this server.
    ///
    /// This lets tests exercise the full diagnostic pipeline with a mock runtime
    /// without needing perlcritic installed on the test machine.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn test_bypass_perlcritic_command_check(&self) {
        self.skip_perlcritic_command_check.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    /// Set the server root path (used for `.perlcriticrc` walk-up discovery).
    pub fn test_set_root_path(&self, path: std::path::PathBuf) {
        *self.root_path.lock() = Some(path);
    }

    /// Configure perlcritic settings directly for test purposes.
    ///
    /// Avoids direct access to `self.config` (which is `pub(crate)`) from
    /// integration tests.  Equivalent to mutating `config.perlcritic_enabled`,
    /// `config.perlcritic_severity`, and `config.perlcritic_profile` directly.
    pub fn test_configure_perlcritic(&self, enabled: bool, severity: u8, profile: Option<String>) {
        let mut cfg = self.config.lock();
        cfg.perlcritic_enabled = enabled;
        cfg.perlcritic_severity = severity;
        cfg.perlcritic_profile = profile;
    }

    /// Configure the critic engine directly for test purposes.
    pub fn test_configure_critic_engine(&self, engine: perl_lsp_rs_core::config::CriticEngine) {
        self.config.lock().critic_engine = engine;
    }

    /// Configure the native critic profile directly for test purposes.
    pub fn test_configure_native_critic_profile(&self, profile: &str) {
        if let Some(profile) =
            perl_lsp_rs_core::tooling::perl_critic::NativeCriticProfile::parse(profile)
        {
            self.config.lock().native_critic_profile = profile.as_str().to_string();
        }
    }

    /// Configure native critic include/exclude filters directly for test purposes.
    pub fn test_configure_native_critic_filters(&self, include: Vec<String>, exclude: Vec<String>) {
        let mut cfg = self.config.lock();
        cfg.native_critic_include = include;
        cfg.native_critic_exclude = exclude;
    }

    /// Test-only entrypoint for LSP `textDocument/inlineCompletion`.
    ///
    /// Exercises inline completion functionality in tests.
    ///
    /// # Parameters
    /// - `params`: JSON-RPC params with `textDocument.uri` and `position`.
    ///
    /// # Returns
    /// - `Ok(Some(list))`: Inline completion list with items.
    /// - `Ok(None)`: No completions available.
    ///
    /// # Errors
    /// Returns [`JsonRpcError`] if params are invalid or document not found.
    pub fn test_handle_inline_completion(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        self.handle_inline_completion(params)
    }

    /// Install a mock AI inline-completion backend for testing.
    ///
    /// Replaces any previously registered backend with the provided one.
    /// Pass `None` to clear the backend entirely.
    pub fn test_install_ai_backend(
        &self,
        backend: Option<
            std::sync::Arc<
                dyn perl_lsp_rs_core::providers::inline_completion::InlineCompletionBackend,
            >,
        >,
    ) {
        *self.ai_inline_backend.lock() = backend;
    }

    /// Configure AI completion settings directly for test purposes.
    ///
    /// Avoids direct access to `self.config` from integration tests.
    pub fn test_configure_ai_completion(&self, enabled: bool, fallback: bool) {
        let mut cfg = self.config.lock();
        cfg.ai_completion.enabled = enabled;
        cfg.ai_completion.fallback = fallback;
    }

    /// Test-only entrypoint for LSP `textDocument/semanticTokens/full`.
    ///
    /// Exercises semantic token generation in tests. Returns the full semantic
    /// token data for the document as a flat encoded array.
    ///
    /// # Parameters
    /// - `params`: JSON-RPC params with `textDocument.uri`.
    ///
    /// # Returns
    /// - `Ok(Some({"data": [...]}))`: Semantic token data (flat u32 array, 5 values per token).
    ///
    /// # Errors
    /// Returns [`JsonRpcError`] if params are invalid or the document is not open.
    pub fn test_handle_semantic_tokens(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        self.handle_semantic_tokens(params)
    }

    /// Test-only receipt for semantic tokens runtime quality proof.
    ///
    /// Calls the live `textDocument/semanticTokens/full` handler and captures the
    /// result in a typed receipt. The receipt records token count, shadow state,
    /// and a quality proof note. This does not change live semantic token behavior.
    pub fn test_semantic_tokens_runtime_quality_receipt(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        self.semantic_tokens_runtime_quality_receipt(params)
    }

    /// Returns `true` if a document with the given URI is currently in the
    /// document store.
    ///
    /// Used in tests to verify that `workspace/didChangeWatchedFiles` DELETED
    /// events remove files from the in-memory store.
    pub fn test_has_document(&self, uri: &str) -> bool {
        self.documents.lock().contains_key(uri)
    }

    /// Force the workspace coordinator into Building/Indexing state and index a
    /// file directly into the underlying index, simulating the background scan
    /// path without transitioning to Ready.
    ///
    /// This is used in tests for Gap 2 (#4152): workspace/symbol during Building
    /// state should still return results from the partial index.
    ///
    /// # Parameters
    /// - `uri`: File URI string (e.g. `"file:///project/lib/Foo.pm"`)
    /// - `text`: Perl source text to index
    ///
    /// # Returns
    /// `Ok(())` if the file was indexed; `Err` if URI parse failed or indexing failed.
    #[cfg(feature = "workspace")]
    pub fn test_index_file_in_building_state(&self, uri: &str, text: &str) -> Result<(), String> {
        let Some(coordinator) = self.index_coordinator.as_ref() else {
            return Err("No coordinator available".to_string());
        };
        // Move to Building/Indexing phase so the state machine won't auto-transition
        // to Ready when index_file is called (simulates background scan in progress).
        coordinator.transition_to_scanning();
        coordinator.transition_to_indexing(10);

        let url = url::Url::parse(uri).map_err(|e| e.to_string())?;
        coordinator.index().index_file(url, text.to_string())
    }

    /// Register workspace folder URIs on the server for multi-root workspace tests.
    ///
    /// Used by deterministic regression tests (e.g. #1514) that need workspace
    /// folder matching without going through the full `initialize` handshake.
    ///
    /// Each `folder_uri` string becomes a `WorkspaceFolderState` entry and is also
    /// propagated to the underlying workspace index via `set_workspace_folders`.
    pub fn test_set_workspace_folder_uris(&self, folder_uris: &[&str]) {
        use super::workspace_folder::WorkspaceFolderState;
        let mut folders = self.workspace_folders.lock();
        folders.clear();
        for &uri in folder_uris {
            folders.push(WorkspaceFolderState::new(uri.to_string()));
        }
        #[cfg(feature = "workspace")]
        if let Some(coordinator) = self.index_coordinator.as_ref() {
            coordinator
                .index()
                .set_workspace_folders(folder_uris.iter().map(|u| u.to_string()).collect());
        }
    }

    /// Simulate background indexing completion by clearing the `indexing_in_progress`
    /// flag and transitioning the coordinator to Ready.
    ///
    /// In production the background thread does this via RAII `IndexingGuard` drop.
    /// In tests we call this directly after `test_index_file_in_building_state`.
    #[cfg(feature = "workspace")]
    pub fn test_simulate_indexing_complete(&self) {
        use std::sync::atomic::Ordering;
        self.indexing_in_progress.store(false, Ordering::Release);
        if let Some(coordinator) = self.index_coordinator.as_ref() {
            let file_count = coordinator.index().file_count();
            let symbol_count = coordinator.index().symbol_count();
            coordinator.transition_to_ready(file_count, symbol_count);
        }
    }

    /// Set `indexing_in_progress` to `true` without spawning a background thread.
    ///
    /// Used by regression tests that need to simulate the race window where a
    /// `workspace/symbol` request arrives while background indexing is still in
    /// progress (i.e. `indexing_in_progress=true`, coordinator still Building).
    ///
    /// Pair with `test_simulate_indexing_complete` — called from a background thread
    /// or after the LSP handler returns — to release the wait.
    ///
    /// In production this flag is set by `start_workspace_indexing` via
    /// compare-exchange before the background thread is spawned.
    #[cfg(feature = "workspace")]
    pub fn test_simulate_indexing_start(&self) {
        use std::sync::atomic::Ordering;
        self.indexing_in_progress.store(true, Ordering::Release);
    }

    /// Notify a test when `workspace/symbol` enters the bounded index-ready wait.
    ///
    /// This is intentionally test-only instrumentation for deterministic race
    /// regressions. The observer is consumed the first time the wait loop sees
    /// `IndexState::Building`.
    #[cfg(feature = "workspace")]
    pub fn test_notify_index_ready_wait_entered(&self, sender: std::sync::mpsc::Sender<()>) {
        let _ = self;
        super::readiness::set_index_ready_wait_entered_observer(sender);
    }

    /// Enable `callHierarchy` in the server's advertised features.
    ///
    /// Test-only helper used by coverage tests that need to reach the
    /// `handle_prepare_call_hierarchy` workspace wait path.  The feature gate
    /// in that handler returns early (method-not-advertised) unless this flag
    /// is set, so the wait line is unreachable without enabling it.
    pub fn test_enable_call_hierarchy(&self) {
        self.advertised_features.lock().call_hierarchy = true;
    }
}

#[cfg(all(test, feature = "workspace"))]
mod tests {
    use super::LspServer;
    use crate::runtime::readiness::{IndexReadinessOutcome, IndexReadinessPolicy};
    use anyhow::Result;
    use std::time::Duration;

    #[test]
    fn test_notify_index_ready_wait_entered_forwards_to_readiness_observer() -> Result<()> {
        let server = LspServer::new();
        let coordinator = server
            .index_coordinator
            .as_ref()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("workspace coordinator missing"))?;
        coordinator.transition_to_building(1);
        server.test_simulate_indexing_start();

        let (wait_entered_tx, wait_entered_rx) = std::sync::mpsc::channel();
        server.test_notify_index_ready_wait_entered(wait_entered_tx);
        let worker_coordinator = coordinator;
        let worker = std::thread::spawn(move || -> Result<()> {
            wait_entered_rx.recv_timeout(Duration::from_secs(1))?;
            worker_coordinator.transition_to_ready(0, 0);
            Ok(())
        });

        let outcome = server.check_index_readiness(IndexReadinessPolicy::WaitBriefly);

        worker.join().map_err(|_| anyhow::anyhow!("readiness observer thread panicked"))??;
        assert!(matches!(outcome, IndexReadinessOutcome::Ready));
        assert!(outcome.is_ready());
        Ok(())
    }
}
