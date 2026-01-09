//! Full JSON-RPC LSP Server implementation
//!
//! This module provides a complete Language Server Protocol implementation
//! that can be used with any LSP-compatible editor.

mod diagnostics;
mod dispatch;
mod language;
mod lifecycle;
/// Routing module for lifecycle-aware index access
pub mod routing;
mod text_sync;
mod workspace;

// Re-export protocol types for backward compatibility
// Tests and external code import these from perl_parser::lsp_server::
pub use crate::lsp::protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};

use crate::{
    CodeActionKind as InternalCodeActionKind,
    CodeActionKindV2 as InternalCodeActionKindV2,
    CodeActionsProvider,
    CodeActionsProviderV2,
    DiagnosticSeverity as InternalDiagnosticSeverity,
    DiagnosticsProvider,
    Parser,
    ast::{Node, NodeKind},
    call_hierarchy_provider::CallHierarchyProvider,
    cancellation::{
        GLOBAL_CANCELLATION_REGISTRY, PerlLspCancellationToken, ProviderCleanupContext,
    },
    code_actions_enhanced::EnhancedCodeActionsProvider,
    code_lens_provider::{CodeLensProvider, get_shebang_lens, resolve_code_lens},
    declaration::ParentMap,
    document_highlight::DocumentHighlightProvider,
    formatting::{CodeFormatter, FormattingOptions},
    implementation_provider::ImplementationProvider,
    // Import fallback implementations
    lsp::fallback::text::extract_text_based_code_lenses,
    // Note: InlayHintConfig and InlayHintsProvider are used in language/misc.rs
    // Import from new modular lsp structure
    // Note: JsonRpcError, JsonRpcRequest, JsonRpcResponse are pub use'd above
    lsp::protocol::{
        CONTENT_MODIFIED, INVALID_PARAMS, INVALID_REQUEST, METHOD_NOT_FOUND, REQUEST_CANCELLED,
        cancelled_response_with_method, document_not_found_error, enhanced_error,
        request_cancelled_error, server_cancelled_error,
    },
    lsp::state::{
        ClientCapabilities, DocumentState, ServerConfig, WorkspaceConfig,
        normalize_package_separator,
    },
    lsp::transport::{log_response, read_message, write_message},
    // Import text processing helpers
    lsp::utils::{
        byte_to_line_col, byte_to_utf16_col, extract_module_reference, get_text_around_offset,
        offset_to_position, position_to_offset,
    },
    performance::{AstCache, SymbolIndex},
    perl_critic::BuiltInAnalyzer,
    positions::LineStartsCache,
    semantic_tokens_provider::{SemanticTokensProvider, encode_semantic_tokens},
    tdd_basic::TestGenerator,
    test_runner::{TestKind, TestRunner},
    type_hierarchy::TypeHierarchyProvider,
};
use lsp_types::Location;
use md5;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::collections::HashSet;
use std::io::{self, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use parking_lot::Mutex;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU32, Ordering},
};
use std::time::{Duration, Instant};
use url::Url;

use crate::uri::parse_uri;
#[cfg(feature = "workspace")]
use crate::workspace_index::{
    IndexCoordinator, LspWorkspaceSymbol, WorkspaceIndex, uri_to_fs_path,
};

#[cfg(feature = "workspace")]
use crate::lsp::fallback::text::extract_text_based_symbols;

/// Lightweight view of a document for scan-heavy operations
///
/// This struct provides the minimal data needed for workspace-wide scans
/// (code lens resolve, reference counting) without requiring the full
/// DocumentState. Using this snapshot pattern allows the documents lock
/// to be released before CPU-intensive work begins.
///
/// ## Design Rationale
/// - `uri`: Needed to construct LSP Location responses
/// - `text`: Needed for text-based fallback searches (regex, line iteration)
/// - `ast`: Arc clone allows AST traversal without deep copying the tree
///
/// The rope, line_starts cache, parent_map, and other fields are omitted
/// as they're not typically needed for bulk scan operations.
pub(crate) struct DocumentScanView {
    /// Document URI for constructing Location responses
    #[allow(dead_code)] // Preserved for future scan operations that build Location responses
    pub uri: String,
    /// Document text content for text-based searches
    pub text: String,
    /// Optional AST reference (Arc clone) for AST-based operations
    pub ast: Option<Arc<crate::ast::Node>>,
}

// Note: FQN_RE regex moved to language/navigation.rs

// Note: Error codes and cancelled_response imported from crate::lsp::protocol

// Note: ClientCapabilities imported from crate::lsp::state::document

/// LSP server that handles JSON-RPC communication
pub struct LspServer {
    /// Document contents indexed by URI
    pub(crate) documents: Arc<Mutex<HashMap<String, DocumentState>>>,
    /// Whether the server is initialized
    initialized: bool,
    /// Index coordinator for workspace-wide features with lifecycle management
    #[cfg(feature = "workspace")]
    pub(crate) index_coordinator: Option<Arc<IndexCoordinator>>,
    /// AST cache for performance
    ast_cache: Arc<AstCache>,
    /// Symbol index for fast lookups
    symbol_index: Arc<Mutex<SymbolIndex>>,
    /// Server configuration
    config: Arc<Mutex<ServerConfig>>,
    /// Synchronized output writer for notifications
    output: Arc<Mutex<Box<dyn Write + Send>>>,
    /// Client capabilities
    client_capabilities: ClientCapabilities,
    /// Cancelled request IDs
    cancelled: Arc<Mutex<HashSet<Value>>>,
    /// Workspace folders
    workspace_folders: Arc<Mutex<Vec<String>>>,
    /// Root path for module resolution
    root_path: Arc<Mutex<Option<PathBuf>>>,
    /// Advertised server capabilities
    advertised_features: Mutex<crate::capabilities::AdvertisedFeatures>,
    /// Client supports pull diagnostics
    client_supports_pull_diags: Arc<AtomicBool>,
    /// Workspace configuration for module resolution
    workspace_config: Arc<Mutex<WorkspaceConfig>>,
}

// Note: DocumentState, ServerConfig, and normalize_package_separator are
// imported from crate::lsp::state::{document, config}

#[allow(dead_code)]
impl LspServer {
    /// Create a new LSP server
    pub fn new() -> Self {
        // Initialize workspace indexing with coordinator lifecycle management
        #[cfg(feature = "workspace")]
        let index_coordinator = Some(Arc::new(IndexCoordinator::new()));

        let default_features = {
            let flags = if cfg!(feature = "lsp-ga-lock") {
                crate::capabilities::BuildFlags::ga_lock()
            } else {
                crate::capabilities::BuildFlags::production()
            };
            flags.to_advertised_features()
        };

        Self {
            documents: Arc::new(Mutex::new(HashMap::new())),
            initialized: false,
            #[cfg(feature = "workspace")]
            index_coordinator,
            // Cache up to 100 ASTs with 5 minute TTL
            ast_cache: Arc::new(AstCache::new(100, 300)),
            symbol_index: Arc::new(Mutex::new(SymbolIndex::new())),
            config: Arc::new(Mutex::new(ServerConfig::default())),
            output: Arc::new(Mutex::new(Box::new(io::stdout()))),
            client_capabilities: ClientCapabilities::default(),
            cancelled: Arc::new(Mutex::new(HashSet::new())),
            workspace_folders: Arc::new(Mutex::new(Vec::new())),
            root_path: Arc::new(Mutex::new(None)),
            advertised_features: Mutex::new(default_features),
            client_supports_pull_diags: Arc::new(AtomicBool::new(false)),
            workspace_config: Arc::new(Mutex::new(WorkspaceConfig::default())),
        }
    }

    /// Create a new LSP server with custom output (for testing)
    pub fn with_output(output: Arc<Mutex<Box<dyn Write + Send>>>) -> Self {
        // Initialize workspace indexing with coordinator lifecycle management
        #[cfg(feature = "workspace")]
        let index_coordinator = Some(Arc::new(IndexCoordinator::new()));

        let default_features = {
            let flags = if cfg!(feature = "lsp-ga-lock") {
                crate::capabilities::BuildFlags::ga_lock()
            } else {
                crate::capabilities::BuildFlags::production()
            };
            flags.to_advertised_features()
        };

        Self {
            documents: Arc::new(Mutex::new(HashMap::new())),
            initialized: false,
            #[cfg(feature = "workspace")]
            index_coordinator,
            ast_cache: Arc::new(AstCache::new(100, 300)),
            symbol_index: Arc::new(Mutex::new(SymbolIndex::new())),
            config: Arc::new(Mutex::new(ServerConfig::default())),
            output,
            client_capabilities: ClientCapabilities::default(),
            cancelled: Arc::new(Mutex::new(HashSet::new())),
            workspace_folders: Arc::new(Mutex::new(Vec::new())),
            root_path: Arc::new(Mutex::new(None)),
            advertised_features: Mutex::new(default_features),
            client_supports_pull_diags: Arc::new(AtomicBool::new(false)),
            workspace_config: Arc::new(Mutex::new(WorkspaceConfig::default())),
        }
    }

    /// Send a notification to the client with proper framing
    fn notify(&self, method: &str, params: Value) -> io::Result<()> {
        let notification = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });

        let notification_str = serde_json::to_string(&notification)?;
        // parking_lot locks cannot be poisoned
        let mut output = self.output.lock();
        write!(output, "Content-Length: {}\r\n\r\n{}", notification_str.len(), notification_str)?;
        output.flush()
    }

    /// Acquire a lock on the documents map
    ///
    /// This helper centralizes lock acquisition behavior. parking_lot locks
    /// cannot be poisoned, so this always succeeds (or blocks until available).
    #[inline]
    pub(crate) fn documents_guard(
        &self,
    ) -> parking_lot::MutexGuard<'_, HashMap<String, DocumentState>> {
        self.documents.lock()
    }

    /// Create a lightweight snapshot of all document URIs and text content
    ///
    /// This method minimizes lock hold time by copying only the URI and text
    /// fields needed for scan-heavy operations (regex searches, text-based
    /// fallbacks). The lock is released immediately after the snapshot is
    /// created, allowing other operations to proceed while scanning.
    ///
    /// ## Performance Characteristics
    /// - Lock hold time: O(n) where n is the number of documents (just cloning strings)
    /// - Memory usage: ~1x total text size (only text is cloned, not AST/rope)
    /// - Use case: Text-based reference searches, regex scans across workspace
    #[inline]
    pub(crate) fn documents_text_snapshot(&self) -> Vec<(String, String)> {
        let docs = self.documents_guard();
        docs.iter().map(|(k, v)| (k.clone(), v.text.clone())).collect()
    }

    /// Create a snapshot for scan operations that may need AST access
    ///
    /// This method provides a more comprehensive snapshot that includes the
    /// AST reference (as Arc clone) in addition to URI and text. This allows
    /// scan-heavy operations to work with both text and AST without holding
    /// the documents lock during CPU-intensive work.
    ///
    /// ## Performance Characteristics
    /// - Lock hold time: O(n) where n is the number of documents
    /// - Memory usage: ~1x text size + Arc refs (AST is shared, not cloned)
    /// - Use case: Code lens resolve, reference counting across workspace
    #[inline]
    pub(crate) fn documents_scan_snapshot(&self) -> Vec<DocumentScanView> {
        let docs = self.documents_guard();
        docs.iter()
            .map(|(k, v)| DocumentScanView {
                uri: k.clone(),
                text: v.text.clone(),
                ast: v.ast.clone(),
            })
            .collect()
    }

    /// Get the index coordinator for lifecycle-aware index access
    ///
    /// Returns a reference to the IndexCoordinator, which provides:
    /// - `state()`: Lock-free check of current index state (Building/Ready/Degraded)
    /// - `index()`: Access to underlying WorkspaceIndex for queries
    /// - `notify_change(uri)`: Notify of file change (tracks parse storm)
    /// - `notify_parse_complete(uri)`: Notify parse done (may trigger recovery)
    /// - `query(full, partial)`: Automatic dispatch based on state
    ///
    /// ## Usage Pattern
    /// ```rust,ignore
    /// if let Some(coordinator) = self.coordinator() {
    ///     coordinator.notify_change(&uri);
    ///     // ... do parsing work ...
    ///     coordinator.notify_parse_complete(&uri);
    /// }
    /// ```
    #[cfg(feature = "workspace")]
    #[inline]
    pub(crate) fn coordinator(&self) -> Option<&Arc<IndexCoordinator>> {
        self.index_coordinator.as_ref()
    }

    /// Coordinator stub when workspace feature is disabled
    ///
    /// Returns None since no coordinator is available without workspace indexing.
    #[cfg(not(feature = "workspace"))]
    #[inline]
    pub(crate) fn coordinator(&self) -> Option<&()> {
        None
    }

    /// Get the workspace index through the coordinator (DEPRECATED for handler use)
    ///
    /// **WARNING**: Do NOT use this method in LSP handlers. Use one of:
    /// - `route_index_access(self.coordinator())` for query operations
    /// - `coordinator.index()` directly for mutation operations
    ///
    /// This method exists for backwards compatibility and diagnostic purposes only.
    /// The grep guard in `scripts/gate-local.sh` enforces this restriction.
    ///
    /// # Usage in handlers
    ///
    /// Query operations (completion, references, navigation):
    /// ```rust,ignore
    /// let mode = route_index_access(self.coordinator());
    /// match mode {
    ///     IndexAccessMode::Full(coord) => { coord.index() }
    ///     IndexAccessMode::Partial(_) | IndexAccessMode::None => { /* fallback */ }
    /// }
    /// ```
    ///
    /// Mutation operations (text sync, file watcher):
    /// ```rust,ignore
    /// if let Some(coordinator) = self.coordinator() {
    ///     coordinator.notify_change(uri);
    ///     let _ = coordinator.index().index_file(url, content);
    ///     coordinator.notify_parse_complete(uri);
    /// }
    /// ```
    #[cfg(feature = "workspace")]
    #[inline]
    #[allow(dead_code)] // Kept for diagnostics/compatibility, not used in handlers
    pub(crate) fn workspace_index(&self) -> Option<Arc<WorkspaceIndex>> {
        self.coordinator().map(|c| Arc::clone(c.index()))
    }

    /// Run the LSP server
    pub fn run(&mut self) -> io::Result<()> {
        let stdin = io::stdin();
        let stdout = io::stdout();
        let mut reader = BufReader::new(stdin.lock());
        let mut stdout = stdout.lock();

        eprintln!("LSP server started");

        loop {
            // Read LSP message using transport module
            match read_message(&mut reader)? {
                Some(request) => {
                    eprintln!("Received request: {}", request.method);

                    // Handle the request
                    if let Some(response) = self.handle_request(request) {
                        // Log and send response using transport module
                        log_response(&response);
                        write_message(&mut stdout, &response)?;
                    }
                }
                None => {
                    // EOF reached, exit cleanly
                    eprintln!("LSP server: EOF on stdin, shutting down");
                    break;
                }
            }
        }

        Ok(())
    }

    /// Handle a message from any reader (for testing)
    pub fn handle_message<R: Read>(&mut self, reader: &mut R) -> io::Result<()> {
        let mut buf_reader = BufReader::new(reader);
        if let Some(request) = read_message(&mut buf_reader)? {
            if let Some(response) = self.handle_request(request) {
                // Write response to the configured output using transport module
                let mut output = self.output.lock();
                write_message(&mut *output, &response)?;
            }
        }
        Ok(())
    }

    // Note: request_cancelled_error, server_cancelled_error, enhanced_error, and
    // document_not_found_error are imported from crate::lsp::protocol

    /// Mark a request as cancelled
    fn cancel_mark(&self, id: &Value) {
        let mut c = self.cancelled.lock();
        c.insert(id.clone());
    }

    /// Clear a cancelled request
    fn cancel_clear(&self, id: &Value) {
        let mut c = self.cancelled.lock();
        c.remove(id);
    }

    /// Check if a request has been cancelled
    fn is_cancelled(&self, id: &Value) -> bool {
        let set = self.cancelled.lock();
        set.contains(id)
    }

    // Note: handle_request is implemented in dispatch.rs

    // Note: completion handlers are implemented in language/completion.rs

    // Note: handle_code_action is implemented in language/code_actions.rs

    // Note: handle_prepare_type_hierarchy, handle_type_hierarchy_supertypes,
    // handle_type_hierarchy_subtypes are implemented in language/hierarchy.rs

    // Note: handle_prepare_rename, handle_rename, handle_rename_workspace are
    // implemented in language/rename.rs

    // Note: handle_code_actions_pragmas, handle_code_action_resolve are implemented
    // in language/code_actions.rs

    // Note: handle_semantic_tokens is implemented in language/semantic_tokens.rs

    // Note: handle_inlay_hints, handle_document_links, handle_selection_range, workspace_roots
    // are implemented in language/misc.rs

    // Note: is_valid_identifier, get_token_at_position, get_token_bounds are
    // implemented in language/rename.rs

    /// Run a specific test
    pub(crate) fn run_test(&self, test_id: &str) -> Result<Option<Value>, JsonRpcError> {
        eprintln!("Running test: {}", test_id);

        // Parse test ID to get URI and test name
        let parts: Vec<&str> = test_id.split("::").collect();
        if parts.len() < 2 {
            return Ok(Some(json!({"status": "error", "message": "Invalid test ID"})));
        }

        let uri = parts[0];
        let test_name = parts[1..].join("::");

        let documents = self.documents.lock();
        if let Some(doc) = documents.get(uri) {
            let runner = TestRunner::new(doc.text.clone(), uri.to_string());
            let results = runner.run_test(&test_name);

            // Convert results to JSON
            let json_results: Vec<Value> = results
                .into_iter()
                .map(|result| {
                    json!({
                        "testId": result.test_id,
                        "status": result.status.as_str(),
                        "message": result.message,
                        "duration": result.duration
                    })
                })
                .collect();

            return Ok(Some(json!({
                "status": "success",
                "results": json_results
            })));
        }

        Ok(Some(document_not_found_error()))
    }

    /// Run all tests in a file
    pub(crate) fn run_test_file(&self, uri: &str) -> Result<Option<Value>, JsonRpcError> {
        eprintln!("Running test file: {}", uri);

        let documents = self.documents.lock();
        if let Some(doc) = documents.get(uri) {
            let runner = TestRunner::new(doc.text.clone(), uri.to_string());
            let results = runner.run_test(uri);

            // Convert results to JSON
            let json_results: Vec<Value> = results
                .into_iter()
                .map(|result| {
                    json!({
                        "testId": result.test_id,
                        "status": result.status.as_str(),
                        "message": result.message,
                        "duration": result.duration
                    })
                })
                .collect();

            return Ok(Some(json!({
                "status": "success",
                "results": json_results
            })));
        }

        Ok(Some(document_not_found_error()))
    }

    // === BEGIN_TEST_ONLY_POSITION_HELPERS ===
    /// Convert offset to line/column position (UTF-16 aware, CRLF safe)
    #[allow(deprecated)]
    pub fn offset_to_position(&self, content: &str, offset: usize) -> (u32, u32) {
        // Implementation moved to lsp/utils.rs
        let p = offset_to_position(content, offset);
        (p.line, p.character)
    }

    /// Convert line/column position to offset (UTF-16 aware, CRLF safe)
    #[allow(deprecated)]
    pub fn position_to_offset(&self, content: &str, line: u32, character: u32) -> usize {
        // Implementation moved to lsp/utils.rs
        position_to_offset(content, line, character).unwrap_or(content.len())
    }
    // === END_TEST_ONLY_POSITION_HELPERS ===

    /// Position conversion using cached line starts for O(log n) performance
    #[inline]
    fn pos16_to_offset(&self, doc: &DocumentState, line: u32, ch: u32) -> usize {
        // Uses the cached, CRLF/UTF-16 aware converter
        doc.line_starts.position_to_offset_rope(&doc.rope, line, ch)
    }

    /// Normalize URI key for consistent document lookup
    pub(crate) fn normalize_uri_key(&self, raw: &str) -> String {
        // Parse to Url to canonicalize, then stringify the way we store it.
        // If parsing fails, return the raw key so we at least try the given string.
        if let Ok(u) = url::Url::parse(raw) {
            // On Windows, lower-case the drive letter to match how many editors send it.
            #[cfg(windows)]
            {
                let s = u.as_str().to_string();
                if let Some(rest) = s.strip_prefix("file:///") {
                    if rest.len() > 1
                        && rest.as_bytes()[1] == b':'
                        && rest.as_bytes()[0].is_ascii_alphabetic()
                    {
                        return format!(
                            "file:///{}{}",
                            rest[0..1].to_ascii_lowercase(),
                            &rest[1..]
                        );
                    }
                }
                return s;
            }
            #[cfg(not(windows))]
            return u.as_str().to_string();
        }
        raw.to_string()
    }

    /// Get document by URI with normalization fallback
    pub(crate) fn get_document<'a>(
        &self,
        documents: &'a parking_lot::MutexGuard<'_, HashMap<String, DocumentState>>,
        uri: &str,
    ) -> Option<&'a DocumentState> {
        let normalized = self.normalize_uri_key(uri);
        documents.get(&normalized).or_else(|| documents.get(uri))
    }

    /// Get mutable document by URI with normalization fallback
    pub(crate) fn get_document_mut<'a>(
        &self,
        documents: &'a mut parking_lot::MutexGuard<'_, HashMap<String, DocumentState>>,
        uri: &str,
    ) -> Option<&'a mut DocumentState> {
        let normalized = self.normalize_uri_key(uri);
        if documents.contains_key(&normalized) {
            documents.get_mut(&normalized)
        } else {
            documents.get_mut(uri)
        }
    }

    /// Helper to create a ContentModified error response
    fn content_modified() -> JsonRpcError {
        JsonRpcError {
            code: CONTENT_MODIFIED,
            message: "Document changed before request executed".to_string(),
            data: None,
        }
    }

    /// Ensure the request version matches the current document version
    fn ensure_latest(&self, uri: &str, req_version: Option<i32>) -> Result<(), JsonRpcError> {
        if let Some(v) = req_version {
            let documents = self.documents.lock();
            if let Some(doc) = self.get_document(&documents, uri) {
                if v < doc.version {
                    return Err(Self::content_modified());
                }
            }
        }
        Ok(())
    }

    /// Offset to position conversion using cached line starts for O(log n) performance
    #[inline]
    fn offset_to_pos16(&self, doc: &DocumentState, offset: usize) -> (u32, u32) {
        doc.line_starts.offset_to_position_rope(&doc.rope, offset)
    }

    // Note: handle_code_lens, handle_code_lens_resolve, handle_inline_completion,
    // handle_inline_value, handle_moniker, handle_document_color, handle_color_presentation,
    // handle_linked_editing_range, count_references_text_based are implemented in language/misc.rs

    /// Extract code lenses from text when AST parsing fails
    fn extract_text_based_code_lenses(
        &self,
        text: &str,
        uri: &str,
    ) -> Vec<crate::code_lens_provider::CodeLens> {
        extract_text_based_code_lenses(text, uri)
    }

    /// Extract symbols from text when AST parsing fails
    #[cfg(feature = "workspace")]
    fn extract_text_based_symbols(
        &self,
        text: &str,
        uri: &str,
        query: &str,
    ) -> Vec<LspWorkspaceSymbol> {
        extract_text_based_symbols(text, uri, query)
    }

    /// Extract symbols stub when workspace feature is disabled
    #[cfg(not(feature = "workspace"))]
    fn extract_text_based_symbols(
        &self,
        _text: &str,
        _uri: &str,
        _query: &str,
    ) -> Vec<serde_json::Value> {
        Vec::new()
    }

    /// Extract workspace symbols from a document's AST
    #[cfg(feature = "workspace")]
    fn extract_document_symbols(
        &self,
        ast: &crate::ast::Node,
        source: &str,
        uri: &str,
    ) -> Vec<LspWorkspaceSymbol> {
        let mut symbols = Vec::new();
        self.extract_symbols_recursive(ast, source, uri, None, &mut symbols);
        symbols
    }

    #[cfg(not(feature = "workspace"))]
    fn extract_document_symbols(
        &self,
        _ast: &crate::ast::Node,
        _source: &str,
        _uri: &str,
    ) -> Vec<serde_json::Value> {
        Vec::new()
    }

    /// Recursively extract symbols from an AST node
    #[cfg(feature = "workspace")]
    fn extract_symbols_recursive(
        &self,
        node: &crate::ast::Node,
        source: &str,
        uri: &str,
        container: Option<&str>,
        symbols: &mut Vec<LspWorkspaceSymbol>,
    ) {
        use crate::ast::NodeKind;

        match &node.kind {
            NodeKind::Subroutine { name, body, .. } => {
                // Add the subroutine as a symbol if it has a name
                if let Some(sub_name) = name {
                    let (start_line, start_char) = byte_to_line_col(source, node.location.start);
                    let (end_line, end_char) = byte_to_line_col(source, node.location.end);

                    symbols.push(LspWorkspaceSymbol {
                        name: sub_name.clone(),
                        kind: 12, // Function
                        location: crate::workspace_index::LspLocation {
                            uri: uri.to_string(),
                            range: crate::workspace_index::LspRange {
                                start: crate::workspace_index::LspPosition {
                                    line: start_line,
                                    character: start_char,
                                },
                                end: crate::workspace_index::LspPosition {
                                    line: end_line,
                                    character: end_char,
                                },
                            },
                        },
                        container_name: container
                            .map(|s| normalize_package_separator(s).into_owned()),
                    });

                    // Recurse into body with this subroutine as container
                    self.extract_symbols_recursive(
                        body,
                        source,
                        uri,
                        Some(sub_name.as_str()),
                        symbols,
                    );
                }
            }

            NodeKind::Package { name, block, .. } => {
                // Add the package as a symbol
                let (start_line, start_char) = byte_to_line_col(source, node.location.start);
                let (end_line, end_char) = byte_to_line_col(source, node.location.end);

                symbols.push(LspWorkspaceSymbol {
                    name: name.clone(),
                    kind: 2, // Module
                    location: crate::workspace_index::LspLocation {
                        uri: uri.to_string(),
                        range: crate::workspace_index::LspRange {
                            start: crate::workspace_index::LspPosition {
                                line: start_line,
                                character: start_char,
                            },
                            end: crate::workspace_index::LspPosition {
                                line: end_line,
                                character: end_char,
                            },
                        },
                    },
                    container_name: container.map(|s| normalize_package_separator(s).into_owned()),
                });

                // Recurse into block with this package as container
                if let Some(block) = block {
                    self.extract_symbols_recursive(
                        block,
                        source,
                        uri,
                        Some(name.as_str()),
                        symbols,
                    );
                }
            }

            NodeKind::Program { statements } => {
                for stmt in statements {
                    self.extract_symbols_recursive(stmt, source, uri, container, symbols);
                }
            }

            NodeKind::Block { statements } => {
                for stmt in statements {
                    self.extract_symbols_recursive(stmt, source, uri, container, symbols);
                }
            }

            _ => {
                // For other node types, recurse into children if they might contain symbols
                // This is a simplified version - you might want to handle more node types
            }
        }
    }

    /// Extract simple symbols without workspace feature
    #[cfg(not(feature = "workspace"))]
    fn extract_simple_symbols(
        &self,
        node: &crate::ast::Node,
        source: &str,
        uri: &str,
        query: &str,
        symbols: &mut Vec<serde_json::Value>,
    ) {
        use crate::ast::NodeKind;

        let query_lower = query.to_lowercase();

        match &node.kind {
            NodeKind::Subroutine { name, body, .. } => {
                if let Some(sub_name) = name {
                    if sub_name.to_lowercase().contains(&query_lower) {
                        let (start_line, start_char) =
                            byte_to_line_col(source, node.location.start);
                        let (end_line, end_char) = byte_to_line_col(source, node.location.end);

                        symbols.push(json!({
                            "name": sub_name,
                            "kind": 12, // Function
                            "location": {
                                "uri": uri,
                                "range": {
                                    "start": {"line": start_line, "character": start_char},
                                    "end": {"line": end_line, "character": end_char}
                                }
                            }
                        }));
                    }
                }
                // Recurse into body
                self.extract_simple_symbols(body, source, uri, query, symbols);
            }

            NodeKind::Package { name, block, .. } => {
                if name.to_lowercase().contains(&query_lower) {
                    let (start_line, start_char) = byte_to_line_col(source, node.location.start);
                    let (end_line, end_char) = byte_to_line_col(source, node.location.end);

                    symbols.push(json!({
                        "name": name,
                        "kind": 2, // Module
                        "location": {
                            "uri": uri,
                            "range": {
                                "start": {"line": start_line, "character": start_char},
                                "end": {"line": end_line, "character": end_char}
                            }
                        }
                    }));
                }
                // Recurse into block
                if let Some(block) = block {
                    self.extract_simple_symbols(block, source, uri, query, symbols);
                }
            }

            NodeKind::Program { statements } => {
                for stmt in statements {
                    self.extract_simple_symbols(stmt, source, uri, query, symbols);
                }
            }

            NodeKind::Block { statements } => {
                for stmt in statements {
                    self.extract_simple_symbols(stmt, source, uri, query, symbols);
                }
            }

            _ => {}
        }
    }

    /// Count references to a symbol in an AST
    #[allow(clippy::only_used_in_recursion)]
    fn count_references(
        &self,
        node: &crate::ast::Node,
        symbol_name: &str,
        symbol_kind: &str,
    ) -> usize {
        use crate::ast::NodeKind;

        let mut count = 0;

        match &node.kind {
            NodeKind::Program { statements } => {
                for stmt in statements {
                    count += self.count_references(stmt, symbol_name, symbol_kind);
                }
            }

            NodeKind::FunctionCall { name, args } => {
                if symbol_kind == "subroutine" && name == symbol_name {
                    count += 1;
                }
                for arg in args {
                    count += self.count_references(arg, symbol_name, symbol_kind);
                }
            }

            NodeKind::MethodCall { object, method, args } => {
                if symbol_kind == "subroutine" && method == symbol_name {
                    count += 1;
                }
                count += self.count_references(object, symbol_name, symbol_kind);
                for arg in args {
                    count += self.count_references(arg, symbol_name, symbol_kind);
                }
            }

            NodeKind::Use { module, args: _ } => {
                if symbol_kind == "package" && module == symbol_name {
                    count += 1;
                }
            }

            NodeKind::Identifier { name } => {
                if symbol_kind == "package" && name == symbol_name {
                    count += 1;
                }
            }

            NodeKind::Block { statements } => {
                for stmt in statements {
                    count += self.count_references(stmt, symbol_name, symbol_kind);
                }
            }

            NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
                count += self.count_references(condition, symbol_name, symbol_kind);
                count += self.count_references(then_branch, symbol_name, symbol_kind);
                for (cond, branch) in elsif_branches {
                    count += self.count_references(cond, symbol_name, symbol_kind);
                    count += self.count_references(branch, symbol_name, symbol_kind);
                }
                if let Some(else_b) = else_branch {
                    count += self.count_references(else_b, symbol_name, symbol_kind);
                }
            }

            NodeKind::While { condition, body, continue_block }
            | NodeKind::For { condition: Some(condition), body, continue_block, .. } => {
                count += self.count_references(condition, symbol_name, symbol_kind);
                count += self.count_references(body, symbol_name, symbol_kind);
                if let Some(cont) = continue_block {
                    count += self.count_references(cont, symbol_name, symbol_kind);
                }
            }

            NodeKind::Foreach { variable: _, list, body } => {
                count += self.count_references(list, symbol_name, symbol_kind);
                count += self.count_references(body, symbol_name, symbol_kind);
            }

            NodeKind::Binary { left, right, .. } => {
                count += self.count_references(left, symbol_name, symbol_kind);
                count += self.count_references(right, symbol_name, symbol_kind);
            }

            NodeKind::Unary { op, operand } => {
                // Check if this is a reference to a subroutine (\&function)
                if op == "\\" && symbol_kind == "subroutine" {
                    if let NodeKind::Identifier { name } = &operand.kind {
                        if name == symbol_name {
                            count += 1;
                        }
                    }
                }
                count += self.count_references(operand, symbol_name, symbol_kind);
            }

            NodeKind::Ternary { condition, then_expr, else_expr } => {
                count += self.count_references(condition, symbol_name, symbol_kind);
                count += self.count_references(then_expr, symbol_name, symbol_kind);
                count += self.count_references(else_expr, symbol_name, symbol_kind);
            }

            NodeKind::Assignment { lhs, rhs, op: _ } => {
                count += self.count_references(lhs, symbol_name, symbol_kind);
                count += self.count_references(rhs, symbol_name, symbol_kind);
            }

            NodeKind::Return { value } => {
                if let Some(val) = value {
                    count += self.count_references(val, symbol_name, symbol_kind);
                }
            }

            NodeKind::ArrayLiteral { elements } => {
                for elem in elements {
                    count += self.count_references(elem, symbol_name, symbol_kind);
                }
            }

            NodeKind::HashLiteral { pairs } => {
                for (key, val) in pairs {
                    count += self.count_references(key, symbol_name, symbol_kind);
                    count += self.count_references(val, symbol_name, symbol_kind);
                }
            }

            NodeKind::Subroutine { body, .. } => {
                count += self.count_references(body, symbol_name, symbol_kind);
            }

            NodeKind::Package { block, .. } => {
                if let Some(block) = block {
                    count += self.count_references(block, symbol_name, symbol_kind);
                }
            }

            NodeKind::Try { body, catch_blocks, finally_block } => {
                count += self.count_references(body, symbol_name, symbol_kind);
                for (_var, block) in catch_blocks {
                    count += self.count_references(block, symbol_name, symbol_kind);
                }
                if let Some(finally) = finally_block {
                    count += self.count_references(finally, symbol_name, symbol_kind);
                }
            }

            // Recursively handle other node types that might contain references
            _ => {
                // Default: no references in other node types
            }
        }

        count
    }

    // Note: handle_semantic_tokens_full, handle_semantic_tokens_range are implemented
    // in language/semantic_tokens.rs

    // Note: handle_prepare_call_hierarchy, handle_incoming_calls, handle_outgoing_calls,
    // json_to_call_hierarchy_item are implemented in language/hierarchy.rs

    // Note: handle_inlay_hint, handle_test_discovery, handle_execute_command, run_test,
    // run_test_file, run_perl_critic are implemented in language/misc.rs

    // =========================================================================
    // Test-only public methods
    // =========================================================================
    //
    // These methods exist to exercise JSON-RPC routing in tests without
    // needing an external transport. They are compiled only for `cargo test`
    // or when the `expose_lsp_test_api` feature is enabled.
    //
    // They are NOT part of the supported runtime API and should not be used
    // outside of test code.

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
    #[cfg(any(test, feature = "expose_lsp_test_api"))]
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
    #[cfg(any(test, feature = "expose_lsp_test_api"))]
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
    #[cfg(any(test, feature = "expose_lsp_test_api"))]
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
    #[cfg(any(test, feature = "expose_lsp_test_api"))]
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
    #[cfg(any(test, feature = "expose_lsp_test_api"))]
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
    #[cfg(any(test, feature = "expose_lsp_test_api"))]
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
    #[cfg(any(test, feature = "expose_lsp_test_api"))]
    pub fn test_handle_workspace_symbols(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        self.handle_workspace_symbols_v2(params)
    }

    // Note: handle_document_link is implemented in language/misc.rs

    /// Get text around an offset position
    pub(crate) fn get_text_around_offset(
        &self,
        content: &str,
        offset: usize,
        radius: usize,
    ) -> String {
        get_text_around_offset(content, offset, radius)
    }

    /// Extract module reference from text (e.g., from "use Module::Name" or "require Module::Name")
    pub(crate) fn extract_module_reference(&self, text: &str, cursor_pos: usize) -> Option<String> {
        extract_module_reference(text, cursor_pos)
    }

    /// Get buffer text for a URI
    pub(crate) fn buffer_text(&self, uri: &str) -> Option<String> {
        let docs = self.documents.lock();
        docs.get(uri).map(|d| d.text.clone())
    }

    /// Iterate over all open buffers (for reference search)
    pub(crate) fn iter_open_buffers(&self) -> Vec<(String, String)> {
        let docs = self.documents.lock();
        docs.iter().map(|(uri, doc)| (uri.clone(), doc.text.clone())).collect()
    }
}

// Helper functions for non-blocking handlers

pub(crate) fn location_from_path(p: &Path) -> serde_json::Value {
    let uri = Url::from_file_path(p).unwrap().to_string();
    // Jump to start of file or try to find 'package' later if you prefer
    serde_json::json!({
        "uri": uri,
        "range": { "start": { "line": 0, "character": 0}, "end": { "line": 0, "character": 0} }
    })
}

impl Default for LspServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn end_position_handles_missing_final_newline() {
        let server = LspServer::new();
        let content = "package Foo;";
        let pos = server.get_document_end_position(content);
        assert_eq!(pos, json!({"line": 0, "character": content.len()}));
    }

    #[test]
    fn code_action_append_uses_document_end() {
        use ropey::Rope;
        use std::sync::Arc;

        let server = LspServer::new();
        let uri = "file:///test.pl";
        let text = "package Foo;"; // No trailing newline
        let rope = Rope::from_str(text);
        let line_starts = LineStartsCache::new_rope(&rope);
        server.documents.lock().insert(
            uri.to_string(),
            DocumentState {
                rope,
                text: text.to_string(),
                version: 1,
                ast: None,
                parse_errors: Vec::new(),
                parent_map: ParentMap::default(),
                line_starts,
                generation: Arc::new(AtomicU32::new(0)),
            },
        );

        let result = server
            .handle_code_actions_pragmas(Some(json!({"textDocument": {"uri": uri}})))
            .unwrap()
            .unwrap();
        let actions = result.as_array().unwrap();
        assert!(!actions.is_empty());
        let edit = &actions[0]["edit"]["changes"][uri][0]["range"];
        let end = server.get_document_end_position(text);
        assert_eq!(edit["start"], end);
        assert_eq!(edit["end"], end);
    }

    #[test]
    fn formatting_edit_has_correct_end_position() {
        let formatter = CodeFormatter::new();
        let options = FormattingOptions {
            tab_size: 4,
            insert_spaces: true,
            trim_trailing_whitespace: None,
            insert_final_newline: None,
            trim_final_newlines: None,
        };

        let code = "sub test{my$x=1;return$x;}";
        match formatter.format_document(code, &options) {
            Ok(edits) => {
                if edits.is_empty() {
                    return;
                }
                let server = LspServer::new();
                let end = server.get_document_end_position(code);
                assert_eq!(edits[0].range.end.line, end["line"].as_u64().unwrap() as u32);
                assert_eq!(edits[0].range.end.character, end["character"].as_u64().unwrap() as u32);
            }
            Err(e) => {
                if e.to_string().contains("not found") {
                    eprintln!("Skipping test: perltidy not installed");
                } else {
                    panic!("Formatting failed: {}", e);
                }
            }
        }
    }
}
