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
}
