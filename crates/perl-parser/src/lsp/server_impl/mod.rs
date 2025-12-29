//! Full JSON-RPC LSP Server implementation
//!
//! This module provides a complete Language Server Protocol implementation
//! that can be used with any LSP-compatible editor.

mod diagnostics;
mod dispatch;
mod language;
mod lifecycle;
mod text_sync;
mod workspace;

pub(crate) use dispatch::early_cancel_or;

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
    // Note: InlayHintConfig and InlayHintsProvider are used in language/misc.rs
    // Import from new modular lsp structure
    // Note: JsonRpcError, JsonRpcRequest, JsonRpcResponse are pub use'd above
    lsp::protocol::{
        CONTENT_MODIFIED, INVALID_PARAMS, INVALID_REQUEST, METHOD_NOT_FOUND, REQUEST_CANCELLED,
        cancelled_response, document_not_found_error, enhanced_error, request_cancelled_error,
        server_cancelled_error,
    },
    lsp::state::{ClientCapabilities, DocumentState, ServerConfig, normalize_package_separator},
    lsp::transport::{log_response, read_message, write_message},
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
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU32, Ordering},
};
use std::time::{Duration, Instant};
use url::Url;

use crate::uri::parse_uri;
#[cfg(feature = "workspace")]
use crate::workspace_index::{
    LspLocation, LspPosition, LspRange, LspWorkspaceSymbol, WorkspaceIndex, uri_to_fs_path,
};

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
// Note: enhanced_cancelled_response and early_cancel_or! macro are in dispatch.rs

// Note: ClientCapabilities imported from crate::lsp::state::document

/// LSP server that handles JSON-RPC communication
pub struct LspServer {
    /// Document contents indexed by URI
    pub(crate) documents: Arc<Mutex<HashMap<String, DocumentState>>>,
    /// Whether the server is initialized
    initialized: bool,
    /// Workspace-wide index for cross-file features
    #[cfg(feature = "workspace")]
    pub(crate) workspace_index: Option<Arc<WorkspaceIndex>>,
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
    advertised_features: std::sync::Mutex<crate::capabilities::AdvertisedFeatures>,
    /// Client supports pull diagnostics
    client_supports_pull_diags: Arc<AtomicBool>,
}

// Note: DocumentState, ServerConfig, and normalize_package_separator are
// imported from crate::lsp::state::{document, config}

#[allow(dead_code)]
impl LspServer {
    /// Create a new LSP server
    pub fn new() -> Self {
        // Initialize workspace indexing (always enabled when workspace feature is on)
        #[cfg(feature = "workspace")]
        let workspace_index = Some(Arc::new(WorkspaceIndex::new()));

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
            workspace_index,
            // Cache up to 100 ASTs with 5 minute TTL
            ast_cache: Arc::new(AstCache::new(100, 300)),
            symbol_index: Arc::new(Mutex::new(SymbolIndex::new())),
            config: Arc::new(Mutex::new(ServerConfig::default())),
            output: Arc::new(Mutex::new(Box::new(io::stdout()))),
            client_capabilities: ClientCapabilities::default(),
            cancelled: Arc::new(Mutex::new(HashSet::new())),
            workspace_folders: Arc::new(Mutex::new(Vec::new())),
            root_path: Arc::new(Mutex::new(None)),
            advertised_features: std::sync::Mutex::new(default_features),
            client_supports_pull_diags: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Create a new LSP server with custom output (for testing)
    pub fn with_output(output: Arc<Mutex<Box<dyn Write + Send>>>) -> Self {
        // Initialize workspace indexing (always enabled when workspace feature is on)
        #[cfg(feature = "workspace")]
        let workspace_index = Some(Arc::new(WorkspaceIndex::new()));

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
            workspace_index,
            ast_cache: Arc::new(AstCache::new(100, 300)),
            symbol_index: Arc::new(Mutex::new(SymbolIndex::new())),
            config: Arc::new(Mutex::new(ServerConfig::default())),
            output,
            client_capabilities: ClientCapabilities::default(),
            cancelled: Arc::new(Mutex::new(HashSet::new())),
            workspace_folders: Arc::new(Mutex::new(Vec::new())),
            root_path: Arc::new(Mutex::new(None)),
            advertised_features: std::sync::Mutex::new(default_features),
            client_supports_pull_diags: Arc::new(AtomicBool::new(false)),
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
        // Handle lock poisoning gracefully instead of panicking
        let mut output = self
            .output
            .lock()
            .map_err(|e| io::Error::other(format!("Failed to acquire output lock: {}", e)))?;
        write!(output, "Content-Length: {}\r\n\r\n{}", notification_str.len(), notification_str)?;
        output.flush()
    }

    /// Acquire a lock on the documents map with poison recovery
    ///
    /// This helper centralizes lock acquisition behavior, recovering from
    /// panicked threads by unwrapping the poisoned mutex. This prevents
    /// server crashes when another thread has panicked while holding the lock.
    #[inline]
    pub(crate) fn documents_guard(
        &self,
    ) -> std::sync::MutexGuard<'_, HashMap<String, DocumentState>> {
        self.documents.lock().unwrap_or_else(|e| e.into_inner())
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
    #[allow(dead_code)]
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
                if let Ok(mut output) = self.output.lock() {
                    write_message(&mut *output, &response)?;
                }
            }
        }
        Ok(())
    }

    // Note: request_cancelled_error, server_cancelled_error, enhanced_error, and
    // document_not_found_error are imported from crate::lsp::protocol

    /// Mark a request as cancelled
    fn cancel_mark(&self, id: &Value) {
        if let Ok(mut c) = self.cancelled.lock() {
            c.insert(id.clone());
        }
    }

    /// Clear a cancelled request
    fn cancel_clear(&self, id: &Value) {
        if let Ok(mut c) = self.cancelled.lock() {
            c.remove(id);
        }
    }

    /// Check if a request has been cancelled
    fn is_cancelled(&self, id: &Value) -> bool {
        if let Ok(set) = self.cancelled.lock() { set.contains(id) } else { false }
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

        let documents = self.documents.lock().unwrap();
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

        let documents = self.documents.lock().unwrap();
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
        let mut line = 0u32;
        let mut col_utf16 = 0u32;
        let mut byte_pos = 0usize;
        let mut chars = content.chars().peekable();

        while let Some(ch) = chars.next() {
            if byte_pos >= offset {
                break;
            }

            match ch {
                '\r' => {
                    // Peek ahead to see if this is CRLF
                    if chars.peek() == Some(&'\n') {
                        // This is CRLF - treat as single line ending
                        if byte_pos + 1 >= offset {
                            // Offset is at the \r - treat as end of current line
                            break;
                        }
                        // Skip both \r and \n
                        chars.next(); // consume the \n
                        line += 1;
                        col_utf16 = 0;
                        byte_pos += 2; // \r + \n
                    } else {
                        // Solo \r - treat as line ending
                        line += 1;
                        col_utf16 = 0;
                        byte_pos += ch.len_utf8();
                    }
                }
                '\n' => {
                    // LF (could be standalone or part of CRLF, but CRLF is handled above)
                    line += 1;
                    col_utf16 = 0;
                    byte_pos += ch.len_utf8();
                }
                _ => {
                    // Regular character
                    col_utf16 += if ch.len_utf16() == 2 { 2 } else { 1 };
                    byte_pos += ch.len_utf8();
                }
            }
        }

        (line, col_utf16)
    }

    /// Convert line/column position to offset (UTF-16 aware, CRLF safe)
    #[allow(deprecated)]
    pub fn position_to_offset(&self, content: &str, line: u32, character: u32) -> usize {
        let mut cur_line = 0u32;
        let mut col_utf16 = 0u32;
        let mut prev_was_cr = false;

        for (byte_idx, ch) in content.char_indices() {
            // Check if we've reached the target position
            if cur_line == line && col_utf16 == character {
                return byte_idx;
            }

            // Handle line endings and character counting
            match ch {
                '\n' => {
                    if !prev_was_cr {
                        // Standalone \n
                        cur_line += 1;
                        col_utf16 = 0;
                    }
                    // If prev_was_cr, this \n is part of CRLF and we already incremented the line
                }
                '\r' => {
                    // Always increment line on \r (whether solo or part of CRLF)
                    cur_line += 1;
                    col_utf16 = 0;
                }
                _ => {
                    // Regular character - only count UTF-16 units on target line
                    if cur_line == line {
                        col_utf16 += if ch.len_utf16() == 2 { 2 } else { 1 };
                    }
                }
            }

            prev_was_cr = ch == '\r';
        }

        // Handle end of file position
        if cur_line == line && col_utf16 == character {
            return content.len();
        }

        // Clamp to end of buffer
        content.len()
    }
    // === END_TEST_ONLY_POSITION_HELPERS ===

    /// Position conversion using cached line starts for O(log n) performance
    #[inline]
    fn pos16_to_offset(&self, doc: &DocumentState, line: u32, ch: u32) -> usize {
        // Uses the cached, CRLF/UTF-16 aware converter
        doc.line_starts.position_to_offset_rope(&doc.rope, line, ch)
    }

    /// Normalize URI key for consistent document lookup
    fn normalize_uri_key(&self, raw: &str) -> String {
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
    fn get_document<'a>(
        &self,
        documents: &'a std::sync::MutexGuard<'_, HashMap<String, DocumentState>>,
        uri: &str,
    ) -> Option<&'a DocumentState> {
        let normalized = self.normalize_uri_key(uri);
        documents.get(&normalized).or_else(|| documents.get(uri))
    }

    /// Get mutable document by URI with normalization fallback
    fn get_document_mut<'a>(
        &self,
        documents: &'a mut std::sync::MutexGuard<'_, HashMap<String, DocumentState>>,
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
            let documents = self.documents.lock().unwrap();
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
        _uri: &str,
    ) -> Vec<crate::code_lens_provider::CodeLens> {
        let mut lenses = Vec::new();

        // Use simple regex patterns to find common Perl constructs that should have code lenses
        use regex::Regex;

        // Find package declarations
        if let Ok(pkg_regex) = Regex::new(r"^\s*package\s+([\w:]+)") {
            for (line_num, line) in text.lines().enumerate() {
                if let Some(captures) = pkg_regex.captures(line) {
                    if let Some(pkg_name) = captures.get(1) {
                        let name = pkg_name.as_str().to_string();

                        lenses.push(crate::code_lens_provider::CodeLens {
                            range: crate::code_lens_provider::Range {
                                start: crate::code_lens_provider::Position {
                                    line: line_num as u32,
                                    character: pkg_name.start() as u32,
                                },
                                end: crate::code_lens_provider::Position {
                                    line: line_num as u32,
                                    character: pkg_name.end() as u32,
                                },
                            },
                            command: None, // Will be resolved later
                            data: Some(json!({
                                "name": name,
                                "kind": "package"
                            })),
                        });
                    }
                }
            }
        }

        // Find subroutine declarations
        if let Ok(sub_regex) = Regex::new(r"^\s*sub\s+(\w+)") {
            for (line_num, line) in text.lines().enumerate() {
                if let Some(captures) = sub_regex.captures(line) {
                    if let Some(sub_name) = captures.get(1) {
                        let name = sub_name.as_str().to_string();

                        lenses.push(crate::code_lens_provider::CodeLens {
                            range: crate::code_lens_provider::Range {
                                start: crate::code_lens_provider::Position {
                                    line: line_num as u32,
                                    character: sub_name.start() as u32,
                                },
                                end: crate::code_lens_provider::Position {
                                    line: line_num as u32,
                                    character: sub_name.end() as u32,
                                },
                            },
                            command: None, // Will be resolved later
                            data: Some(json!({
                                "name": name,
                                "kind": "subroutine"
                            })),
                        });
                    }
                }
            }
        }

        lenses
    }

    /// Extract symbols from text when AST parsing fails
    fn extract_text_based_symbols(
        &self,
        text: &str,
        uri: &str,
        query: &str,
    ) -> Vec<LspWorkspaceSymbol> {
        let mut symbols = Vec::new();
        let query_lower = query.to_lowercase();

        // Use simple regex patterns to find common Perl symbols
        use regex::Regex;

        // Find subroutine definitions
        if let Ok(sub_regex) = Regex::new(r"^\s*sub\s+(\w+)") {
            for (line_num, line) in text.lines().enumerate() {
                if let Some(captures) = sub_regex.captures(line) {
                    if let Some(sub_name) = captures.get(1) {
                        let name = sub_name.as_str().to_string();
                        if name.to_lowercase().contains(&query_lower) {
                            symbols.push(LspWorkspaceSymbol {
                                name,
                                kind: 12, // Function
                                location: LspLocation {
                                    uri: uri.to_string(),
                                    range: LspRange {
                                        start: LspPosition {
                                            line: line_num as u32,
                                            character: sub_name.start() as u32,
                                        },
                                        end: LspPosition {
                                            line: line_num as u32,
                                            character: sub_name.end() as u32,
                                        },
                                    },
                                },
                                container_name: None,
                            });
                        }
                    }
                }
            }
        }

        // Find package declarations
        if let Ok(pkg_regex) = Regex::new(r"^\s*package\s+([\w:]+)") {
            for (line_num, line) in text.lines().enumerate() {
                if let Some(captures) = pkg_regex.captures(line) {
                    if let Some(pkg_name) = captures.get(1) {
                        let name = pkg_name.as_str().to_string();
                        if name.to_lowercase().contains(&query_lower) {
                            symbols.push(LspWorkspaceSymbol {
                                name,
                                kind: 4, // Namespace
                                location: LspLocation {
                                    uri: uri.to_string(),
                                    range: LspRange {
                                        start: LspPosition {
                                            line: line_num as u32,
                                            character: pkg_name.start() as u32,
                                        },
                                        end: LspPosition {
                                            line: line_num as u32,
                                            character: pkg_name.end() as u32,
                                        },
                                    },
                                },
                                container_name: None,
                            });
                        }
                    }
                }
            }
        }

        symbols
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
                    let (start_line, start_char) =
                        self.byte_to_line_col(source, node.location.start);
                    let (end_line, end_char) = self.byte_to_line_col(source, node.location.end);

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
                let (start_line, start_char) = self.byte_to_line_col(source, node.location.start);
                let (end_line, end_char) = self.byte_to_line_col(source, node.location.end);

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
                            self.byte_to_line_col(source, node.location.start);
                        let (end_line, end_char) = self.byte_to_line_col(source, node.location.end);

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
                    let (start_line, start_char) =
                        self.byte_to_line_col(source, node.location.start);
                    let (end_line, end_char) = self.byte_to_line_col(source, node.location.end);

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

    /// Convert byte offset to line and column
    fn byte_to_line_col(&self, source: &str, offset: usize) -> (u32, u32) {
        let mut line = 0;
        let mut col = 0;

        for (i, ch) in source.char_indices() {
            if i >= offset {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
        }

        (line, col)
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

    /// Find matching closing parenthesis
    fn find_matching_paren(&self, s: &str, open_at: usize) -> Option<usize> {
        // s[open_at] must be '('; walk forwards tracking (), [], {} and quotes.
        let mut i = open_at;
        let mut depth_par = 0i32;
        let mut _depth_brk = 0i32;
        let mut _depth_brc = 0i32;
        let mut in_s = false;
        let mut in_d = false;
        while i < s.len() {
            let b = s.as_bytes()[i];
            let prev_backslash = i > 0 && s.as_bytes()[i - 1] == b'\\';
            if in_s {
                if b == b'\'' && !prev_backslash {
                    in_s = false;
                }
            } else if in_d {
                if b == b'"' && !prev_backslash {
                    in_d = false;
                }
            } else {
                match b {
                    b'\'' => in_s = true,
                    b'"' => in_d = true,
                    b'(' => depth_par += 1,
                    b')' => {
                        depth_par -= 1;
                        if depth_par == 0 {
                            return Some(i);
                        }
                    }
                    b'[' => _depth_brk += 1,
                    b']' => _depth_brk -= 1,
                    b'{' => _depth_brc += 1,
                    b'}' => _depth_brc -= 1,
                    _ => {}
                }
            }
            i += 1;
        }
        None
    }

    /// Scan forward until end of statement (top-level `;`) honoring quotes/brackets.
    fn slice_until_stmt_end(&self, src: &str, from: usize) -> usize {
        let mut i = from;
        let mut depth_par = 0i32;
        let mut depth_brk = 0i32;
        let mut depth_brc = 0i32;
        let mut in_s = false;
        let mut in_d = false;
        while i < src.len() {
            let b = src.as_bytes()[i];
            let esc = i > 0 && src.as_bytes()[i - 1] == b'\\';
            if in_s {
                if b == b'\'' && !esc {
                    in_s = false;
                }
            } else if in_d {
                if b == b'"' && !esc {
                    in_d = false;
                }
            } else {
                match b {
                    b'\'' => in_s = true,
                    b'"' => in_d = true,
                    b'(' => depth_par += 1,
                    b')' => depth_par -= 1,
                    b'[' => depth_brk += 1,
                    b']' => depth_brk -= 1,
                    b'{' => depth_brc += 1,
                    b'}' => depth_brc -= 1,
                    b';' if depth_par == 0 && depth_brk == 0 && depth_brc == 0 => return i,
                    _ => {}
                }
            }
            i += 1;
        }
        src.len()
    }

    /// Top-level argument starts for a comma-separated list without surrounding parens.
    fn arg_starts_top_level(&self, src: &str) -> Vec<usize> {
        let mut v = Vec::new();
        let mut i = 0usize;
        while i < src.len() && src.as_bytes()[i].is_ascii_whitespace() {
            i += 1;
        }
        if i < src.len() {
            v.push(i);
        }
        let mut j = i;
        let mut depth_par = 0i32;
        let mut depth_brk = 0i32;
        let mut depth_brc = 0i32;
        let mut in_s = false;
        let mut in_d = false;
        while j < src.len() {
            let b = src.as_bytes()[j];
            let esc = j > 0 && src.as_bytes()[j - 1] == b'\\';
            if in_s {
                if b == b'\'' && !esc {
                    in_s = false;
                }
            } else if in_d {
                if b == b'"' && !esc {
                    in_d = false;
                }
            } else {
                match b {
                    b'\'' => in_s = true,
                    b'"' => in_d = true,
                    b'(' => depth_par += 1,
                    b')' => depth_par -= 1,
                    b'[' => depth_brk += 1,
                    b']' => depth_brk -= 1,
                    b'{' => depth_brc += 1,
                    b'}' => depth_brc -= 1,
                    b',' if depth_par == 0 && depth_brk == 0 && depth_brc == 0 => {
                        let mut k = j + 1;
                        while k < src.len() && src.as_bytes()[k].is_ascii_whitespace() {
                            k += 1;
                        }
                        if k < src.len() {
                            v.push(k);
                        }
                    }
                    _ => {}
                }
            }
            j += 1;
        }
        v
    }

    /// Move the anchor inside an argument to the "interesting" token:
    ///  - skip leading whitespace
    ///  - for `my|our` args, jump to the first sigiled var (`$foo`/`@a`/`%h`)
    ///  - for bareword filehandles (e.g., `FH`), jump to the bareword
    fn anchor_arg_start(&self, body: &str, rel: usize) -> usize {
        let s = &body[rel..];
        let mut i = 0usize;
        while i < s.len() && s.as_bytes()[i].is_ascii_whitespace() {
            i += 1;
        }
        // my/our <sigiled-var>
        if s[i..].starts_with("my ") || s[i..].starts_with("our ") {
            let mut j = i + 3; // skip "my " / "our "
            while j < s.len() && s.as_bytes()[j].is_ascii_whitespace() {
                j += 1;
            }
            return rel + j;
        }
        // If next is sigiled variable, keep; else keep bareword start
        rel + i
    }

    /// If argument starts at `my $fh`, retarget anchor to the `$fh` (or bareword FH).
    fn smart_arg_anchor(&self, body: &str, rel: usize) -> usize {
        let s = &body[rel..];
        let mut i = 0usize;
        while i < s.len() && s.as_bytes()[i].is_ascii_whitespace() {
            i += 1;
        }

        // handle my/our
        for kw in ["my ", "our "] {
            if s[i..].starts_with(kw) {
                i += kw.len();
                while i < s.len() && s.as_bytes()[i].is_ascii_whitespace() {
                    i += 1;
                }
                break;
            }
        }

        // valid anchors: sigils, barewords, deref braces and array/hash derefs
        // $, @, %, &, { (for @{ ... }, %{ ... }), [ (rare, but safe), or A-Za-z_ bareword
        let b = s.as_bytes().get(i).copied().unwrap_or(b' ');
        if matches!(b, b'$' | b'@' | b'%' | b'&' | b'{' | b'[')
            || b.is_ascii_alphabetic()
            || b == b'_'
        {
            return rel + i;
        }
        rel + i
    }

    /// Find argument starts in function call body
    fn arg_starts_in_call_body(&self, body: &str) -> Vec<usize> {
        // Return byte offsets (within body) where each top-level argument starts.
        let mut starts = Vec::new();
        let mut i = 0usize;
        let mut depth_par = 0i32;
        let mut _depth_brk = 0i32;
        let mut _depth_brc = 0i32;
        let mut in_s = false;
        let mut in_d = false;
        let mut _at_token = false;
        // First arg always starts at the first non-space
        while i < body.len() && body.as_bytes()[i].is_ascii_whitespace() {
            i += 1;
        }
        if i < body.len() {
            starts.push(i);
            _at_token = true;
        }
        let mut j = i;
        while j < body.len() {
            let b = body.as_bytes()[j];
            let prev_backslash = j > 0 && body.as_bytes()[j - 1] == b'\\';
            if in_s {
                if b == b'\'' && !prev_backslash {
                    in_s = false;
                }
            } else if in_d {
                if b == b'"' && !prev_backslash {
                    in_d = false;
                }
            } else {
                match b {
                    b'\'' => in_s = true,
                    b'"' => in_d = true,
                    b'(' => depth_par += 1,
                    b')' => depth_par -= 1,
                    b'[' => _depth_brk += 1,
                    b']' => _depth_brk -= 1,
                    b'{' => _depth_brc += 1,
                    b'}' => _depth_brc -= 1,
                    b',' if depth_par == 0 && _depth_brk == 0 && _depth_brc == 0 => {
                        // next arg start = first non-space after comma
                        let mut k = j + 1;
                        while k < body.len() && body.as_bytes()[k].is_ascii_whitespace() {
                            k += 1;
                        }
                        if k < body.len() {
                            starts.push(k);
                        }
                    }
                    _ => {}
                }
            }
            j += 1;
        }
        starts
    }

    /// Convert position to byte offset
    fn pos_to_offset_bytes(&self, text: &str, line: u32, ch: u32) -> usize {
        let mut byte = 0usize;
        for (cur, l) in text.split_inclusive('\n').enumerate() {
            if cur as u32 == line {
                return byte + (ch as usize).min(l.len());
            }
            byte += l.len();
        }
        text.len()
    }

    /// Slice text within range
    fn slice_in_range<'a>(
        &self,
        text: &'a str,
        start: (u32, u32),
        end: (u32, u32),
    ) -> (usize, usize, &'a str) {
        let s = self.pos_to_offset_bytes(text, start.0, start.1);
        let e = self.pos_to_offset_bytes(text, end.0, end.1);
        (s, e, &text[s.min(text.len())..e.min(text.len())])
    }

    // Note: handle_inlay_hint, handle_test_discovery, handle_execute_command, run_test,
    // run_test_file, run_perl_critic are implemented in language/misc.rs

    // Test-only public methods (enabled for unit tests or integration tests with expose_lsp_test_api)
    #[cfg(any(test, feature = "expose_lsp_test_api"))]
    pub fn test_handle_did_open(&self, params: Option<Value>) -> Result<(), JsonRpcError> {
        self.handle_did_open(params)
    }

    #[cfg(any(test, feature = "expose_lsp_test_api"))]
    pub fn test_handle_definition(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        self.handle_definition(params)
    }

    #[cfg(any(test, feature = "expose_lsp_test_api"))]
    pub fn test_handle_references(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        self.handle_references(params)
    }

    #[cfg(any(test, feature = "expose_lsp_test_api"))]
    pub fn test_handle_completion(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        self.handle_completion(params)
    }

    // Note: handle_document_link is implemented in language/misc.rs
}

impl LspServer {
    /// Get text around an offset position
    fn get_text_around_offset(&self, content: &str, offset: usize, radius: usize) -> String {
        let start = offset.saturating_sub(radius);
        let end = (offset + radius).min(content.len());
        content[start..end].to_string()
    }

    /// Extract module reference from text (e.g., from "use Module::Name" or "require Module::Name")
    fn extract_module_reference(&self, text: &str, cursor_pos: usize) -> Option<String> {
        // Look for patterns like "use Module::Name" or "require Module::Name"
        let patterns = [
            r"use\s+([A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*)",
            r"require\s+([A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*)",
        ];

        for pattern in patterns {
            let re = regex::Regex::new(pattern).ok()?;
            for cap in re.captures_iter(text) {
                if let Some(module_match) = cap.get(1) {
                    let match_start = module_match.start();
                    let match_end = module_match.end();

                    // Check if cursor is within the module name
                    if cursor_pos >= match_start && cursor_pos <= match_end {
                        return Some(module_match.as_str().to_string());
                    }
                }
            }
        }

        None
    }

    /// Get buffer text for a URI
    fn buffer_text(&self, uri: &str) -> Option<String> {
        let docs = self.documents.lock().unwrap();
        docs.get(uri).map(|d| d.text.clone())
    }

    /// Iterate over all open buffers (for reference search)
    fn iter_open_buffers(&self) -> Vec<(String, String)> {
        let docs = self.documents.lock().unwrap();
        docs.iter().map(|(uri, doc)| (uri.clone(), doc.text.clone())).collect()
    }

    /// Non-blocking definition handler with fallback
    fn on_definition(&self, params: serde_json::Value) -> Result<serde_json::Value, JsonRpcError> {
        let uri = params.pointer("/textDocument/uri").and_then(|v| v.as_str()).unwrap_or("");
        let line = params.pointer("/position/line").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let ch =
            params.pointer("/position/character").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

        let text = self.buffer_text(uri).unwrap_or_default();
        let module = token_under_cursor(&text, line, ch).filter(|s| s.contains("::"));

        if let Some(m) = module {
            if let Some(path) = self.resolve_module_path(&m) {
                let loc = location_from_path(&path);
                return Ok(serde_json::json!([loc]));
            }
        }

        // Fallback: try existing analysis
        // For now, just return empty array
        Ok(serde_json::json!([]))
    }

    /// Non-blocking references handler with fallback
    fn on_references(&self, params: serde_json::Value) -> Result<serde_json::Value, JsonRpcError> {
        let uri = params.pointer("/textDocument/uri").and_then(|v| v.as_str()).unwrap_or("");
        let line = params.pointer("/position/line").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let ch =
            params.pointer("/position/character").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

        let text = self.buffer_text(uri).unwrap_or_default();
        let needle = token_under_cursor(&text, line, ch).unwrap_or_default();
        if needle.is_empty() {
            return Ok(serde_json::json!([]));
        }

        // Fallback: search all open docs with word boundary checking
        let mut out = Vec::new();
        for (doc_uri, doc_text) in self.iter_open_buffers() {
            for (ln, l) in doc_text.lines().enumerate() {
                let line_bytes = l.as_bytes();
                let mut start = 0usize;
                while let Some(idx) = l[start..].find(&needle) {
                    let col = start + idx;
                    // Only include if it's a word boundary match
                    if is_word_boundary(line_bytes, col, needle.len()) {
                        // Convert byte position to UTF-16 for LSP
                        let col_utf16 = byte_to_utf16_col(l, col);
                        let end_utf16 = byte_to_utf16_col(l, col + needle.len());
                        out.push(serde_json::json!({
                            "uri": doc_uri,
                            "range": {
                                "start": {"line": ln as u32, "character": col_utf16 as u32},
                                "end":   {"line": ln as u32, "character": end_utf16 as u32}
                            }
                        }));
                    }
                    start = col + needle.len();
                }
            }
        }

        // Sort for deterministic output and deduplicate
        out.sort_by_key(|loc| {
            (
                loc["uri"].as_str().unwrap_or("").to_string(),
                loc["range"]["start"]["line"].as_u64().unwrap_or(0),
                loc["range"]["start"]["character"].as_u64().unwrap_or(0),
            )
        });
        out.dedup();

        Ok(serde_json::Value::Array(out))
    }

    /// Non-blocking folding range handler with text-based fallback
    fn on_folding_range(
        &self,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let uri = params.pointer("/textDocument/uri").and_then(|v| v.as_str()).unwrap_or("");
        let text = self.buffer_text(uri).unwrap_or_default();
        let ranges = folding_ranges_from_text(&text, 128);
        Ok(serde_json::to_value(ranges).unwrap_or(serde_json::json!([])))
    }
}

// Helper functions for non-blocking handlers

/// Convert UTF-16 column position to byte offset
fn byte_offset_utf16(line_text: &str, col_utf16: usize) -> usize {
    let mut units = 0;
    for (i, ch) in line_text.char_indices() {
        if units == col_utf16 {
            return i;
        }
        // UTF-16 encoding: chars >= U+10000 use 2 units (surrogate pairs)
        let add = if ch as u32 >= 0x10000 { 2 } else { 1 };
        units += add;
    }
    line_text.len()
}

/// Extract token at cursor position (UTF-16 aware)
fn token_under_cursor(text: &str, line: usize, col_utf16: usize) -> Option<String> {
    let l = text.lines().nth(line)?;
    let byte_pos = byte_offset_utf16(l, col_utf16);
    let bytes = l.as_bytes();

    if byte_pos >= bytes.len() {
        return None;
    }

    // Expand to a "word" containing :: and \w
    // Also include sigils if we're on or after one
    let mut s = byte_pos;
    let mut e = byte_pos;

    // Expand left - if we hit a sigil, include it
    while s > 0 && is_modchar(bytes[s - 1]) {
        s -= 1;
    }
    if s > 0
        && (bytes[s - 1] == b'$'
            || bytes[s - 1] == b'@'
            || bytes[s - 1] == b'%'
            || bytes[s - 1] == b'&'
            || bytes[s - 1] == b'*')
    {
        s -= 1;
    }

    // Expand right
    while e < bytes.len() && is_modchar(bytes[e]) {
        e += 1;
    }

    Some(l[s..e].to_string())
}

fn is_modchar(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b':' || b == b'_'
}

/// Check if position is at word boundary (for accurate reference matching)
fn is_word_boundary(text: &[u8], pos: usize, word_len: usize) -> bool {
    // For Perl variables with sigils, we need special handling
    // If the match starts with a sigil ($, @, %), check the character after the variable name
    let _sigil_offset =
        if pos < text.len() && (text[pos] == b'$' || text[pos] == b'@' || text[pos] == b'%') {
            1
        } else {
            0
        };

    // Check left boundary (before the sigil if present)
    if pos > 0 && is_modchar(text[pos - 1]) {
        return false;
    }

    // Check right boundary (after the identifier part)
    let end_pos = pos + word_len;
    if end_pos < text.len() && is_modchar(text[end_pos]) {
        return false;
    }

    true
}

fn location_from_path(p: &Path) -> serde_json::Value {
    let uri = Url::from_file_path(p).unwrap().to_string();
    // Jump to start of file or try to find 'package' later if you prefer
    serde_json::json!({
        "uri": uri,
        "range": { "start": { "line": 0, "character": 0}, "end": { "line": 0, "character": 0} }
    })
}

/// Convert byte offset to UTF-16 column position
///
/// LSP uses UTF-16 code units for character positions, but Rust strings use
/// UTF-8 byte offsets. This function converts a byte position within a line
/// to the corresponding UTF-16 column position.
pub(crate) fn byte_to_utf16_col(line_text: &str, byte_pos: usize) -> usize {
    let mut units = 0;
    for (i, ch) in line_text.char_indices() {
        if i >= byte_pos {
            break;
        }
        // UTF-16 encoding: chars >= U+10000 use 2 units
        units += if ch as u32 >= 0x10000 { 2 } else { 1 };
    }
    units
}

fn folding_ranges_from_text(src: &str, limit: usize) -> Vec<serde_json::Value> {
    let mut out = Vec::new();
    let lines: Vec<&str> = src.lines().collect();

    // Track different types of blocks
    let mut sub_stack: Vec<usize> = Vec::new();
    let mut pod_start: Option<usize> = None;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();

        // Skip lines that look like strings (basic heuristic)
        if trimmed.starts_with('"') || trimmed.starts_with('\'') || trimmed.starts_with('`') {
            continue;
        }

        // POD documentation blocks
        if trimmed.starts_with("=pod") || trimmed.starts_with("=head") {
            pod_start = Some(i);
        } else if trimmed.starts_with("=cut") {
            if let Some(start) = pod_start.take() {
                if i > start {
                    out.push(serde_json::json!({
                        "startLine": start as u32,
                        "endLine": i as u32,
                        "kind": "comment"
                    }));
                }
            }
        }

        // Subroutine blocks
        if trimmed.starts_with("sub ") && trimmed.contains('{') {
            sub_stack.push(i);
        } else if trimmed.starts_with('}') && pod_start.is_none() {
            if let Some(start) = sub_stack.pop() {
                if i > start {
                    out.push(serde_json::json!({
                        "startLine": start as u32,
                        "endLine": i as u32,
                        "kind": "region"
                    }));
                }
            }
        }
    }

    if out.len() > limit {
        out.truncate(limit);
    }
    out
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
        use std::sync::Arc;

        let server = LspServer::new();
        let uri = "file:///test.pl";
        let text = "package Foo;"; // No trailing newline
        let rope = ropey::Rope::from_str(text);
        let line_starts = LineStartsCache::new_rope(&rope);
        server.documents.lock().unwrap().insert(
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
