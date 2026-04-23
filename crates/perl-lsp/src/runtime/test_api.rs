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
        // Reset any cached analyzer state so it is rebuilt with the new runtime.
        self.reset_perlcritic_state();
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
}
