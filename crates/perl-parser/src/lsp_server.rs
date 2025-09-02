//! Full JSON-RPC LSP Server implementation
//!
//! This module provides a complete Language Server Protocol implementation
//! that can be used with any LSP-compatible editor.

use crate::{
    CodeActionKind as InternalCodeActionKind, CodeActionKindV2 as InternalCodeActionKindV2,
    CodeActionsProvider, CodeActionsProviderV2, CompletionItemKind, CompletionProvider,
    DiagnosticSeverity as InternalDiagnosticSeverity, DiagnosticsProvider, Parser,
    ast::{Node, NodeKind},
    call_hierarchy_provider::CallHierarchyProvider,
    code_actions_enhanced::EnhancedCodeActionsProvider,
    code_lens_provider::{CodeLensProvider, get_shebang_lens, resolve_code_lens},
    declaration::ParentMap,
    document_highlight::DocumentHighlightProvider,
    formatting::{CodeFormatter, FormattingOptions},
    inlay_hints_provider::{InlayHintConfig, InlayHintsProvider},
    performance::{AstCache, SymbolIndex},
    perl_critic::BuiltInAnalyzer,
    positions::LineStartsCache,
    semantic_tokens_provider::{SemanticTokensProvider, encode_semantic_tokens},
    tdd_basic::TestGenerator,
    test_runner::{TestKind, TestRunner},
    type_hierarchy::TypeHierarchyProvider,
    type_inference::TypeInferenceEngine,
};
use lsp_types::Location;
use md5;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::borrow::Cow;
use std::collections::HashMap;
use std::collections::HashSet;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU32, Ordering},
};
use url::Url;

use crate::uri::parse_uri;
#[cfg(feature = "workspace")]
use crate::workspace_index::{LspWorkspaceSymbol, WorkspaceIndex, WorkspaceSymbol, uri_to_fs_path};

// JSON-RPC Error Codes
const ERR_METHOD_NOT_FOUND: i32 = -32601;
const ERR_INVALID_REQUEST: i32 = -32600;
const ERR_INVALID_PARAMS: i32 = -32602;
// LSP 3.17 standard error codes:
// -32802 ServerCancelled (preferred for server-side cancellations)
// -32801 ContentModified (document changed; redo request)
// -32800 RequestCancelled (client-side; we use this for $/cancelRequest)
#[allow(dead_code)]
const ERR_SERVER_CANCELLED: i32 = -32802; // Server cancelled the request (LSP 3.17)
const ERR_CONTENT_MODIFIED: i32 = -32801; // Content modified, operation obsolete
const ERR_REQUEST_CANCELLED: i32 = -32800; // Client cancelled via $/cancelRequest

/// Helper to create a cancelled response
fn cancelled_response(id: &serde_json::Value) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id: Some(id.clone()),
        result: None,
        error: Some(JsonRpcError {
            code: ERR_REQUEST_CANCELLED,
            message: "Request cancelled".into(),
            data: None,
        }),
    }
}

/// Macro for early cancellation check in dispatcher arms
macro_rules! early_cancel_or {
    ($self:ident, $id:expr, $handler:expr) => {{
        if let Some(ref _rid) = $id {
            if $self.is_cancelled(_rid) {
                $self.cancel_clear(_rid);
                return Some(cancelled_response(_rid));
            }
        }
        $handler
    }};
}

/// Client capabilities received during initialization
#[derive(Debug, Clone, Default)]
struct ClientCapabilities {
    /// Supports LocationLink for goto declaration
    declaration_link_support: bool,
    /// Supports LocationLink for goto definition
    definition_link_support: bool,
    /// Supports LocationLink for goto type definition
    type_definition_link_support: bool,
    /// Supports LocationLink for goto implementation
    implementation_link_support: bool,
    /// Supports dynamic registration for file watching
    dynamic_registration_support: bool,
}

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
    #[allow(dead_code)]
    advertised_features: std::sync::Mutex<crate::capabilities::AdvertisedFeatures>,
    /// Client supports pull diagnostics
    client_supports_pull_diags: Arc<AtomicBool>,
}

/// State of a document
#[derive(Clone)]
pub(crate) struct DocumentState {
    /// Document content
    /// TODO: Use Rope for O(1) edits in future optimization
    pub(crate) content: String,
    /// Version number
    pub(crate) _version: i32,
    /// Parsed AST (cached)
    pub(crate) ast: Option<std::sync::Arc<crate::ast::Node>>,
    /// Parse errors
    pub(crate) parse_errors: Vec<crate::error::ParseError>,
    /// Parent map for O(1) scope traversal (built once per AST)
    /// Uses FxHashMap for faster pointer hashing
    pub(crate) parent_map: ParentMap,
    /// Line starts cache for O(log n) position conversion
    pub(crate) line_starts: LineStartsCache,
    /// Generation counter for latest-wins race condition prevention
    pub(crate) generation: Arc<AtomicU32>,
}

/// Normalize legacy package separator ' to ::
fn norm_pkg<'a>(s: &'a str) -> Cow<'a, str> {
    if s.contains('\'') { Cow::Owned(s.replace('\'', "::")) } else { Cow::Borrowed(s) }
}

/// Server configuration
#[derive(Debug, Clone)]
struct ServerConfig {
    // Inlay hints configuration
    inlay_hints_enabled: bool,
    inlay_hints_parameter_hints: bool,
    inlay_hints_type_hints: bool,
    inlay_hints_chained_hints: bool,
    inlay_hints_max_length: usize,

    // Test runner configuration
    test_runner_enabled: bool,
    test_runner_command: String,
    test_runner_args: Vec<String>,
    test_runner_timeout: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            inlay_hints_enabled: true,
            inlay_hints_parameter_hints: true,
            inlay_hints_type_hints: true,
            inlay_hints_chained_hints: false,
            inlay_hints_max_length: 30,
            test_runner_enabled: true,
            test_runner_command: "perl".to_string(),
            test_runner_args: vec![],
            test_runner_timeout: 60000,
        }
    }
}

/// JSON-RPC request
#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    #[serde(rename = "jsonrpc")]
    pub _jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

/// JSON-RPC response
#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC error
#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<Value>,
}

#[allow(dead_code)]
impl LspServer {
    /// Create a new LSP server
    pub fn new() -> Self {
        // Initialize workspace indexing (always enabled when workspace feature is on)
        #[cfg(feature = "workspace")]
        let workspace_index = Some(Arc::new(WorkspaceIndex::new()));

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
            advertised_features: std::sync::Mutex::new(
                crate::capabilities::AdvertisedFeatures::default(),
            ),
            client_supports_pull_diags: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Create a new LSP server with custom output (for testing)
    pub fn with_output(output: Arc<Mutex<Box<dyn Write + Send>>>) -> Self {
        // Initialize workspace indexing (always enabled when workspace feature is on)
        #[cfg(feature = "workspace")]
        let workspace_index = Some(Arc::new(WorkspaceIndex::new()));

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
            advertised_features: std::sync::Mutex::new(
                crate::capabilities::AdvertisedFeatures::default(),
            ),
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
        let mut output = self.output.lock().unwrap();
        write!(output, "Content-Length: {}\r\n\r\n{}", notification_str.len(), notification_str)?;
        output.flush()
    }

    /// Run the LSP server
    pub fn run(&mut self) -> io::Result<()> {
        let stdin = io::stdin();
        let stdout = io::stdout();
        let mut reader = BufReader::new(stdin.lock());
        let mut stdout = stdout.lock();

        eprintln!("LSP server started");

        loop {
            // Read LSP message
            match self.read_message_from(&mut reader)? {
                Some(request) => {
                    eprintln!("Received request: {}", request.method);

                    // Handle the request
                    if let Some(response) = self.handle_request(request) {
                        // Send response
                        self.send_message(&mut stdout, &response)?;
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

    /// Send an LSP message to stdout
    fn send_message(
        &self,
        stdout: &mut io::StdoutLock<'_>,
        response: &JsonRpcResponse,
    ) -> io::Result<()> {
        let content = serde_json::to_string(response)?;
        let content_length = content.len();

        // Log outgoing response for debugging
        eprintln!(
            "[perl-lsp:tx] id={:?} has_result={} has_error={} len={}",
            response.id,
            response.result.is_some(),
            response.error.is_some(),
            content_length
        );

        write!(stdout, "Content-Length: {}\r\n\r\n{}", content_length, content)?;
        stdout.flush()?;

        Ok(())
    }

    /// Handle a message from any reader (for testing)
    pub fn handle_message<R: Read>(&mut self, reader: &mut R) -> io::Result<()> {
        let mut buf_reader = BufReader::new(reader);
        if let Some(request) = self.read_message_from(&mut buf_reader)? {
            if let Some(response) = self.handle_request(request) {
                // Write response to the configured output
                if let Ok(mut output) = self.output.lock() {
                    let content = serde_json::to_string(&response)?;
                    let content_length = content.len();
                    write!(output, "Content-Length: {}\r\n\r\n{}", content_length, content)?;
                    output.flush()?;
                }
            }
        }
        Ok(())
    }

    /// Read an LSP message from any BufRead source
    fn read_message_from<R: BufRead>(&self, reader: &mut R) -> io::Result<Option<JsonRpcRequest>> {
        let mut headers = HashMap::new();

        // Read headers
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line)? == 0 {
                return Ok(None); // EOF
            }

            let line = line.trim_end();
            if line.is_empty() {
                break; // End of headers
            }

            if let Some((key, value)) = line.split_once(": ") {
                headers.insert(key.to_string(), value.to_string());
            }
        }

        // Read content
        if let Some(content_length) = headers.get("Content-Length") {
            if let Ok(length) = content_length.parse::<usize>() {
                let mut content = vec![0u8; length];
                let mut bytes_read = 0;

                // Read content in chunks to handle partial reads
                while bytes_read < length {
                    let bytes_to_read = length - bytes_read;
                    let mut chunk = vec![0u8; bytes_to_read];
                    match reader.read(&mut chunk)? {
                        0 => return Ok(None), // Unexpected EOF
                        n => {
                            content[bytes_read..bytes_read + n].copy_from_slice(&chunk[..n]);
                            bytes_read += n;
                        }
                    }
                }

                // Parse JSON-RPC request
                if let Ok(request) = serde_json::from_slice(&content) {
                    return Ok(Some(request));
                }
            }
        }

        Ok(None)
    }

    /// Create a request cancelled error
    fn request_cancelled() -> JsonRpcError {
        JsonRpcError {
            code: ERR_REQUEST_CANCELLED,
            message: "Request cancelled".to_string(),
            data: None,
        }
    }

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

    /// Handle a JSON-RPC request
    pub fn handle_request(&mut self, request: JsonRpcRequest) -> Option<JsonRpcResponse> {
        let id = request.id.clone();

        // Handle $/cancelRequest notification
        if request.method == "$/cancelRequest" {
            if let Some(idv) = request.params.as_ref().and_then(|p| p.get("id")).cloned() {
                self.cancel_mark(&idv);
            }
            return None; // Notifications don't get responses
        }

        // Check if this request has been cancelled
        if let Some(ref id) = id {
            if self.is_cancelled(id) {
                return Some(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: Some(id.clone()),
                    result: None,
                    error: Some(Self::request_cancelled()),
                });
            }
        }

        let result = match request.method.as_str() {
            "initialize" => self.handle_initialize(request.params),
            "initialized" => {
                self.initialized = true;
                eprintln!("Server initialized");

                // Register file watchers for Perl files only if client supports it
                if self.client_capabilities.dynamic_registration_support {
                    self.register_file_watchers_async();
                }

                Ok(None)
            }
            "shutdown" => {
                // Clear any pending cancelled requests on shutdown
                self.cancelled.lock().unwrap().clear();
                Ok(Some(json!(null)))
            }
            "textDocument/didOpen" => match self.handle_did_open(request.params) {
                Ok(_) => Ok(None),
                Err(e) => Err(e),
            },
            "textDocument/didChange" => {
                // Use incremental version if available
                #[cfg(feature = "incremental")]
                let result = if std::env::var("PERL_LSP_INCREMENTAL").is_ok() {
                    self.handle_did_change_incremental(request.params)
                } else {
                    self.handle_did_change(request.params)
                };
                #[cfg(not(feature = "incremental"))]
                let result = self.handle_did_change(request.params);
                match result {
                    Ok(_) => Ok(None),
                    Err(e) => Err(e),
                }
            }
            "textDocument/didClose" => match self.handle_did_close(request.params) {
                Ok(_) => Ok(None),
                Err(e) => Err(e),
            },
            "textDocument/didSave" => match self.handle_did_save(request.params) {
                Ok(_) => Ok(None),
                Err(e) => Err(e),
            },
            "textDocument/willSave" => match self.handle_will_save(request.params) {
                Ok(_) => Ok(None),
                Err(e) => Err(e),
            },
            "textDocument/willSaveWaitUntil" => self.handle_will_save_wait_until(request.params),
            "textDocument/completion" => self.handle_completion(request.params),
            "textDocument/hover" => self.handle_hover(request.params),
            "textDocument/signatureHelp" => self.handle_signature_help(request.params),
            "textDocument/definition" => {
                // Use test fallback in test mode, production handler otherwise
                let use_fallback = std::env::var("LSP_TEST_FALLBACKS").is_ok();
                if use_fallback {
                    match self.on_definition(request.params.clone().unwrap_or(json!({}))) {
                        Ok(res) => Ok(Some(res)),
                        Err(_) => self.handle_definition(request.params),
                    }
                } else {
                    // Production: try real handler first, fall back if needed
                    self.handle_definition(request.params)
                        .or_else(|_| self.on_definition(json!({})).map(Some))
                }
            }
            "textDocument/declaration" => self.handle_declaration(request.params),
            "textDocument/references" => early_cancel_or!(self, id, {
                // Use test fallback in test mode, production handler otherwise
                let use_fallback = std::env::var("LSP_TEST_FALLBACKS").is_ok();
                if use_fallback {
                    match self.on_references(request.params.clone().unwrap_or(json!({}))) {
                        Ok(res) => Ok(Some(res)),
                        Err(_) => self.handle_references(request.params),
                    }
                } else {
                    // Production: try real handler first, fall back if needed
                    self.handle_references(request.params)
                        .or_else(|_| self.on_references(json!({})).map(Some))
                }
            }),
            "textDocument/documentHighlight" => self.handle_document_highlight(request.params),
            "textDocument/prepareTypeHierarchy" => {
                self.handle_prepare_type_hierarchy(request.params)
            }
            "typeHierarchy/prepare" => {
                // Alias for deprecated/alternate method string
                self.handle_prepare_type_hierarchy(request.params)
            }
            "typeHierarchy/supertypes" => self.handle_type_hierarchy_supertypes(request.params),
            "typeHierarchy/subtypes" => self.handle_type_hierarchy_subtypes(request.params),
            "textDocument/diagnostic" => self.handle_document_diagnostic(request.params),
            "workspace/diagnostic" => {
                early_cancel_or!(self, id, self.handle_workspace_diagnostic(request.params))
            }
            "textDocument/prepareRename" => self.handle_prepare_rename(request.params),
            // GA contract: not supported in v0.8.3
            // PR 3: Wire workspace/symbol to use the index
            "workspace/symbol" => {
                #[cfg(feature = "workspace")]
                let result = self.handle_workspace_symbols_v2(request.params);
                #[cfg(not(feature = "workspace"))]
                let result = self.handle_workspace_symbols(request.params);
                early_cancel_or!(self, id, result)
            }
            "workspace/symbol/resolve" => self.handle_workspace_symbol_resolve(request.params),

            "textDocument/rename" => self.handle_rename_workspace(request.params),
            "textDocument/codeAction" => self.handle_code_actions_pragmas(request.params),
            "codeAction/resolve" => self.handle_code_action_resolve(request.params),
            // PR 6: Semantic tokens
            "textDocument/semanticTokens/full" => self.handle_semantic_tokens(request.params),
            // PR 7: Inlay hints
            "textDocument/inlayHint" => {
                early_cancel_or!(self, id, self.handle_inlay_hints(request.params))
            }
            // PR 8: Document links
            "textDocument/documentLink" => self.handle_document_links(request.params),
            // PR 8: Selection ranges
            "textDocument/selectionRange" => self.handle_selection_range(request.params),
            // PR 9: On-type formatting
            "textDocument/onTypeFormatting" => self.handle_on_type_formatting(request.params),
            // Code lens support
            "textDocument/codeLens" => self.handle_code_lens(request.params),
            "codeLens/resolve" => self.handle_code_lens_resolve(request.params),
            // Linked editing ranges
            "textDocument/linkedEditingRange" => self.handle_linked_editing_range(request.params),
            // Inline completion
            "textDocument/inlineCompletion" => self.handle_inline_completion(request.params),
            // Inline values for debugging
            "textDocument/inlineValue" => self.handle_inline_value(request.params),
            // Monikers
            "textDocument/moniker" => self.handle_moniker(request.params),
            // Document colors
            "textDocument/documentColor" => self.handle_document_color(request.params),
            "textDocument/colorPresentation" => self.handle_color_presentation(request.params),
            // Semantic tokens range
            "textDocument/semanticTokens/range" => {
                self.handle_semantic_tokens_range(request.params)
            }
            // GA contract: these methods remain unsupported in v0.8.3
            "workspace/executeCommand" => self.handle_execute_command(request.params),
            "textDocument/typeDefinition" => self.handle_type_definition(request.params),
            "textDocument/implementation" => self.handle_implementation(request.params),
            "textDocument/documentSymbol" => {
                eprintln!("Processing documentSymbol request");
                let result = self.handle_document_symbol(request.params);
                eprintln!("DocumentSymbol result: {:?}", result.is_ok());
                result
            }
            "textDocument/foldingRange" => {
                // Use test fallback in test mode, production handler otherwise
                let use_fallback = std::env::var("LSP_TEST_FALLBACKS").is_ok();
                if use_fallback {
                    match self.on_folding_range(request.params.clone().unwrap_or(json!({}))) {
                        Ok(res) => Ok(Some(res)),
                        Err(_) => self.handle_folding_range(request.params),
                    }
                } else {
                    // Production: try real handler first, fall back if needed
                    self.handle_folding_range(request.params)
                        .or_else(|_| self.on_folding_range(json!({})).map(Some))
                }
            }
            "textDocument/formatting" => self.handle_formatting(request.params),
            "textDocument/rangeFormatting" => self.handle_range_formatting(request.params),
            "textDocument/prepareCallHierarchy" => {
                self.handle_prepare_call_hierarchy(request.params)
            }
            "callHierarchy/incomingCalls" => self.handle_incoming_calls(request.params),
            "callHierarchy/outgoingCalls" => self.handle_outgoing_calls(request.params),
            "experimental/testDiscovery" => self.handle_test_discovery(request.params),
            "workspace/configuration" => self.handle_configuration(request.params),
            "workspace/didChangeWatchedFiles" => {
                self.handle_did_change_watched_files(request.params)
            }
            "workspace/didChangeWorkspaceFolders" => {
                match self.handle_did_change_workspace_folders(request.params) {
                    Ok(_) => Ok(None),
                    Err(e) => Err(e),
                }
            }
            "workspace/willRenameFiles" => self.handle_will_rename_files(request.params),
            "workspace/didDeleteFiles" => self.handle_did_delete_files(request.params),
            "workspace/applyEdit" => self.handle_apply_edit(request.params),
            // Test-specific slow operation for cancellation testing
            // This is available in all builds but only used by tests
            "$/test/slowOperation" => {
                // Check for cancellation periodically during the slow operation
                // Total time: 20 * 50ms = 1 second
                for i in 0..20 {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    if let Some(ref id) = id {
                        if self.is_cancelled(id) {
                            eprintln!("Operation cancelled at iteration {}", i);
                            return Some(JsonRpcResponse {
                                jsonrpc: "2.0".to_string(),
                                id: Some(id.clone()),
                                result: None,
                                error: Some(Self::request_cancelled()),
                            });
                        }
                    }
                }
                eprintln!("Slow operation completed without cancellation");
                Ok(Some(json!({"status": "completed", "iterations": 20})))
            }
            _ => {
                eprintln!("Method not implemented: {}", request.method);
                Err(JsonRpcError {
                    code: ERR_METHOD_NOT_FOUND,
                    message: "Method not found".to_string(),
                    data: None,
                })
            }
        };

        match result {
            Ok(Some(result)) => {
                eprintln!("Sending successful response for request {}", request.method);
                Some(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: Some(result),
                    error: None,
                })
            }
            Ok(None) => {
                eprintln!("Request {} is a notification, no response", request.method);
                None // Notification, no response
            }
            Err(error) => {
                eprintln!("Sending error response for request {}: {:?}", request.method, error);
                Some(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(error),
                })
            }
        }
    }

    /// Handle initialize request
    fn handle_initialize(&mut self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        // Check if already initialized
        if self.initialized {
            return Err(JsonRpcError {
                code: -32600, // InvalidRequest per LSP spec 3.17
                message: "initialize may only be sent once".to_string(),
                data: None,
            });
        }

        // Parse client capabilities
        if let Some(params) = &params {
            self.client_capabilities.declaration_link_support = params
                .get("capabilities")
                .and_then(|c| c.get("textDocument"))
                .and_then(|td| td.get("declaration"))
                .and_then(|d| d.get("linkSupport"))
                .and_then(|b| b.as_bool())
                .unwrap_or(false);

            self.client_capabilities.definition_link_support = params
                .get("capabilities")
                .and_then(|c| c.get("textDocument"))
                .and_then(|td| td.get("definition"))
                .and_then(|d| d.get("linkSupport"))
                .and_then(|b| b.as_bool())
                .unwrap_or(false);

            self.client_capabilities.type_definition_link_support = params
                .get("capabilities")
                .and_then(|c| c.get("textDocument"))
                .and_then(|td| td.get("typeDefinition"))
                .and_then(|d| d.get("linkSupport"))
                .and_then(|b| b.as_bool())
                .unwrap_or(false);

            self.client_capabilities.implementation_link_support = params
                .get("capabilities")
                .and_then(|c| c.get("textDocument"))
                .and_then(|td| td.get("implementation"))
                .and_then(|d| d.get("linkSupport"))
                .and_then(|b| b.as_bool())
                .unwrap_or(false);

            // Check if client supports dynamic registration for file watching
            self.client_capabilities.dynamic_registration_support = params
                .get("capabilities")
                .and_then(|c| c.get("workspace"))
                .and_then(|w| w.get("didChangeWatchedFiles"))
                .and_then(|d| d.get("dynamicRegistration"))
                .and_then(|b| b.as_bool())
                .unwrap_or(false);

            // Check if client supports pull diagnostics
            let supports_pull = params
                .get("capabilities")
                .and_then(|c| c.get("textDocument"))
                .and_then(|td| td.get("diagnostic"))
                .is_some();

            if supports_pull {
                self.client_supports_pull_diags.store(true, Ordering::Relaxed);
                eprintln!("Client supports pull diagnostics - suppressing automatic publishing");
            }

            // Initialize workspace folders
            if let Some(workspace_folders) =
                params.get("workspaceFolders").and_then(|f| f.as_array())
            {
                let mut folders = self.workspace_folders.lock().unwrap();
                for folder in workspace_folders {
                    if let Some(uri) = folder["uri"].as_str() {
                        eprintln!("Initialized with workspace folder: {}", uri);
                        folders.push(uri.to_string());
                    }
                }
            } else if let Some(root_uri) = params.get("rootUri").and_then(|u| u.as_str()) {
                // Fallback to rootUri if workspaceFolders is not provided
                let mut folders = self.workspace_folders.lock().unwrap();
                eprintln!("Initialized with root URI: {}", root_uri);
                folders.push(root_uri.to_string());
                // Also set the root path for module resolution
                self.set_root_uri(root_uri);
            }
        }

        // Check for available tools quickly with a timeout
        // Use which/where command which is much faster than spawning the actual tools
        let has_perltidy = if cfg!(target_os = "windows") {
            std::process::Command::new("where")
                .arg("perltidy")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        } else {
            std::process::Command::new("which")
                .arg("perltidy")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        };

        let has_perlcritic = if cfg!(target_os = "windows") {
            std::process::Command::new("where")
                .arg("perlcritic")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        } else {
            std::process::Command::new("which")
                .arg("perlcritic")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        };

        eprintln!("Tool availability: perltidy={}, perlcritic={}", has_perltidy, has_perlcritic);

        // Check if incremental parsing is enabled
        let sync_kind =
            if cfg!(feature = "incremental") && std::env::var("PERL_LSP_INCREMENTAL").is_ok() {
                2 // Incremental sync
            } else {
                1 // Full document sync
            };

        // Build capabilities using catalog-driven approach
        let mut build_flags = if cfg!(feature = "lsp-ga-lock") {
            crate::capabilities::BuildFlags::ga_lock()
        } else {
            crate::capabilities::BuildFlags::production()
        };

        // Set formatting flags based on perltidy availability
        if has_perltidy {
            build_flags.formatting = true;
            build_flags.range_formatting = true;
        }

        // Generate capabilities from build flags
        let server_caps = crate::capabilities::capabilities_for(build_flags.clone());
        let mut capabilities = serde_json::to_value(&server_caps).unwrap();

        // Add fields not yet in lsp-types 0.97
        capabilities["positionEncoding"] = json!("utf-16");
        capabilities["declarationProvider"] = json!(true);
        capabilities["documentHighlightProvider"] = json!(true);

        // Override text document sync with more detailed options
        capabilities["textDocumentSync"] = json!({
            "openClose": true,
            "change": sync_kind,
            "willSave": true,
            "willSaveWaitUntil": false,
            "save": { "includeText": true }
        });

        // Store advertised features for gating
        *self.advertised_features.lock().unwrap() = build_flags.to_advertised_features();

        Ok(Some(json!({
            "capabilities": capabilities,
            "serverInfo": {
                "name": "perl-lsp",
                "version": env!("CARGO_PKG_VERSION")
            }
        })))
    }

    /// Handle didOpen notification
    pub(crate) fn handle_did_open(&self, params: Option<Value>) -> Result<(), JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let text = params["textDocument"]["text"].as_str().unwrap_or("");
            let version = params["textDocument"]["version"].as_i64().unwrap_or(0) as i32;

            eprintln!("Document opened: {}", uri);

            // Check cache first
            let (ast, errors) = if let Some(cached_ast) = self.ast_cache.get(uri, text) {
                eprintln!("Using cached AST for {}", uri);
                (Some((*cached_ast).clone()), vec![])
            } else {
                // Parse the document up to __DATA__ or __END__ marker
                let code_text = crate::util::code_slice(text);
                let mut parser = Parser::new(code_text);
                match parser.parse() {
                    Ok(ast) => {
                        let arc_ast = Arc::new(ast);
                        self.ast_cache.put(uri.to_string(), text, Arc::clone(&arc_ast));
                        (Some((*arc_ast).clone()), vec![])
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
            let line_starts = LineStartsCache::new(text);

            // Store document state with normalized URI
            let normalized_uri = self.normalize_uri_key(uri);
            self.documents.lock().unwrap().insert(
                normalized_uri,
                DocumentState {
                    content: text.to_string(),
                    _version: version,
                    ast: ast_arc.clone(),
                    parse_errors: errors,
                    parent_map,
                    line_starts,
                    generation: Arc::new(AtomicU32::new(0)),
                },
            );

            // Index symbols for workspace search
            if let Some(ref _ast) = ast_arc {
                // Update the fast symbol index with symbols from workspace index
                #[cfg(feature = "workspace")]
                if let Some(ref workspace_index) = self.workspace_index {
                    let index_symbols = workspace_index.find_symbols("");
                    let symbols = index_symbols
                        .into_iter()
                        .filter(|s| s.uri == uri)
                        .map(|s| s.name.clone())
                        .collect::<Vec<_>>();

                    let mut index = self.symbol_index.lock().unwrap();
                    for symbol in symbols {
                        index.add_symbol(symbol);
                    }
                }
                #[cfg(not(feature = "workspace"))]
                {
                    let _index = self.symbol_index.lock().unwrap();
                    // Just ensure the index exists even without workspace feature
                }

                // Update the workspace-wide index for cross-file features
                #[cfg(feature = "workspace")]
                if let Some(ref workspace_index) = self.workspace_index {
                    if let Ok(url) = url::Url::parse(uri) {
                        if let Err(e) = workspace_index.index_file(url, text.to_string()) {
                            eprintln!("Failed to index file {}: {}", uri, e);
                        }
                    }
                }
            }

            // Send diagnostics
            self.publish_diagnostics(uri);
        }

        Ok(())
    }

    /// Convenience wrapper to open a document from tests
    pub fn did_open(&self, params: Value) -> Result<(), JsonRpcError> {
        self.handle_did_open(Some(params))
    }

    /// Handle didChange notification
    pub(crate) fn handle_did_change(&self, params: Option<Value>) -> Result<(), JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let version = params["textDocument"]["version"].as_i64().unwrap_or(0) as i32;

            if let Some(changes) = params["contentChanges"].as_array() {
                // Get current document state or create new one
                let mut documents = self.documents.lock().unwrap();
                let normalized_uri = self.normalize_uri_key(uri);
                let mut doc_state = documents
                    .get(&normalized_uri)
                    .or_else(|| documents.get(uri))
                    .cloned()
                    .unwrap_or_else(|| DocumentState {
                        content: String::new(),
                        _version: version,
                        ast: None,
                        parse_errors: vec![],
                        parent_map: ParentMap::default(),
                        line_starts: LineStartsCache::new(""),
                        generation: Arc::new(AtomicU32::new(0)),
                    });

                // Increment generation counter for this change
                let next_gen = doc_state.generation.fetch_add(1, Ordering::SeqCst).wrapping_add(1);
                let target_version = version;

                // Apply incremental changes with UTF-16 aware mapping
                use crate::textdoc::{Doc, PosEnc, apply_changes};
                use lsp_types::TextDocumentContentChangeEvent;
                use ropey::Rope;

                let mut doc = Doc { rope: Rope::from_str(&doc_state.content), version };

                // Convert JSON changes to proper LSP types
                let lsp_changes: Vec<TextDocumentContentChangeEvent> =
                    changes.iter().filter_map(|c| serde_json::from_value(c.clone()).ok()).collect();

                // Apply changes with UTF-16 encoding (as advertised in initialize)
                apply_changes(&mut doc, &lsp_changes, PosEnc::Utf16);

                let text = doc.rope.to_string();
                eprintln!("Document changed: {} (version {})", uri, version);

                // Check cache first
                let (ast, errors) = if let Some(cached_ast) = self.ast_cache.get(uri, &text) {
                    eprintln!("Using cached AST for {}", uri);
                    (Some((*cached_ast).clone()), vec![])
                } else {
                    // Parse the document up to __DATA__ or __END__ marker
                    let code_text = crate::util::code_slice(&text);
                    let mut parser = Parser::new(code_text);
                    match parser.parse() {
                        Ok(ast) => {
                            let arc_ast = Arc::new(ast);
                            self.ast_cache.put(uri.to_string(), &text, Arc::clone(&arc_ast));
                            (Some((*arc_ast).clone()), vec![])
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
                let line_starts = LineStartsCache::new(&text);

                // Update document state with properly updated content
                doc_state = DocumentState {
                    content: text.to_string(),
                    _version: version,
                    ast: ast_arc.clone(),
                    parse_errors: errors,
                    parent_map,
                    line_starts,
                    generation: doc_state.generation.clone(), // Preserve the generation counter
                };

                // Check if a newer change arrived while we were parsing
                if let Some(existing_doc) = self.get_document(&documents, uri) {
                    if existing_doc.generation.load(Ordering::SeqCst) != next_gen
                        || existing_doc._version > target_version
                    {
                        eprintln!(
                            "Discarding stale parse result for {} (gen {} != {} or version {} > {})",
                            uri,
                            next_gen,
                            existing_doc.generation.load(Ordering::SeqCst),
                            existing_doc._version,
                            target_version
                        );
                        return Ok(());
                    }
                }

                documents.insert(normalized_uri.clone(), doc_state);

                // Must drop the lock before calling publish_diagnostics
                drop(documents);

                // Index symbols for workspace search
                if let Some(ref _ast) = ast_arc {
                    // Update the workspace-wide index for cross-file features
                    // Note: version is maintained by the document state
                    #[cfg(feature = "workspace")]
                    if let Some(ref workspace_index) = self.workspace_index {
                        if let Ok(url) = url::Url::parse(uri) {
                            let doc_content = self
                                .documents
                                .lock()
                                .unwrap()
                                .get(uri)
                                .map(|d| d.content.clone())
                                .unwrap_or_default();
                            if let Err(e) = workspace_index.index_file(url, doc_content) {
                                eprintln!("Failed to index file {}: {}", uri, e);
                            }
                        }
                    }
                }

                // Send diagnostics
                self.publish_diagnostics(uri);
            }
        }

        Ok(())
    }

    /// Publish diagnostics for a document
    pub(crate) fn publish_diagnostics(&self, uri: &str) {
        let documents = self.documents.lock().unwrap();
        if let Some(doc) = documents.get(uri) {
            let lsp_diagnostics: Vec<Value> = if let Some(ast) = &doc.ast {
                // Get diagnostics (already includes unused variable detection)
                let provider = DiagnosticsProvider::new(ast, doc.content.clone());
                let mut diagnostics =
                    provider.get_diagnostics(ast, &doc.parse_errors, &doc.content);

                // Add Perl::Critic built-in analysis
                let built_in_analyzer = BuiltInAnalyzer::new();
                let violations = built_in_analyzer.analyze(ast, &doc.content);
                for violation in violations {
                    diagnostics.push(crate::Diagnostic {
                        range: (violation.range.start.byte, violation.range.end.byte),
                        severity: violation.severity.to_diagnostic_severity(),
                        code: Some(violation.policy),
                        message: violation.description,
                        related_information: Vec::new(),
                        tags: Vec::new(),
                    });
                }

                // Convert to LSP diagnostics
                diagnostics
                    .into_iter()
                    .map(|d| {
                        let (start_line, start_char) = self.offset_to_pos16(doc, d.range.0);
                        let (end_line, end_char) = self.offset_to_pos16(doc, d.range.1);

                        json!({
                            "range": {
                                "start": {"line": start_line, "character": start_char},
                                "end": {"line": end_line, "character": end_char},
                            },
                            "severity": match d.severity {
                                InternalDiagnosticSeverity::Error => 1,
                                InternalDiagnosticSeverity::Warning => 2,
                                InternalDiagnosticSeverity::Information => 3,
                                InternalDiagnosticSeverity::Hint => 4,
                            },
                            "code": d.code,
                            "source": "perl-parser",
                            "message": d.message,
                        })
                    })
                    .collect()
            } else {
                // No AST available (parse failed completely), just report parse errors
                doc.parse_errors
                    .iter()
                    .map(|e| {
                        // Extract location and message from error enum
                        let (location, message) = match e {
                            crate::error::ParseError::UnexpectedToken {
                                location,
                                expected,
                                found,
                            } => (*location, format!("Expected {}, found {}", expected, found)),
                            crate::error::ParseError::SyntaxError { location, message } => {
                                (*location, message.clone())
                            }
                            crate::error::ParseError::UnexpectedEof => {
                                (doc.content.len(), "Unexpected end of input".to_string())
                            }
                            crate::error::ParseError::LexerError { message } => {
                                (0, message.clone())
                            }
                            _ => (0, e.to_string()),
                        };

                        // Convert byte offset to line/column
                        let (line, character) = self.offset_to_pos16(doc, location);

                        json!({
                            "range": {
                                "start": {"line": line, "character": character},
                                "end": {"line": line, "character": character + 1},
                            },
                            "severity": 1, // Error
                            "code": "parse-error",
                            "source": "perl-parser",
                            "message": message,
                        })
                    })
                    .collect()
            };

            eprintln!(
                "Publishing {} diagnostics for {} (version {})",
                lsp_diagnostics.len(),
                uri,
                doc._version
            );

            // Only publish if client doesn't support pull diagnostics
            // This avoids double-flow for modern clients
            if !self.client_supports_pull_diags.load(Ordering::Relaxed) {
                // Send diagnostics notification with version
                // This ensures diagnostics are cleared when all errors are fixed
                let _ = self.notify(
                    "textDocument/publishDiagnostics",
                    json!({
                        "uri": uri,
                        "version": doc._version,
                        "diagnostics": lsp_diagnostics
                    }),
                );
            }
        }
    }

    /// Handle didClose notification
    fn handle_did_close(&self, params: Option<Value>) -> Result<(), JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");

            eprintln!("Document closed: {}", uri);

            // Remove from documents
            self.documents.lock().unwrap().remove(uri);

            // Clear from workspace index
            #[cfg(feature = "workspace")]
            if let Some(ref workspace_index) = self.workspace_index {
                workspace_index.clear_file(uri);
            }

            // Clear diagnostics for this file using centralized notify
            let _ = self.notify(
                "textDocument/publishDiagnostics",
                json!({
                    "uri": uri,
                    "diagnostics": []
                }),
            );
        }

        Ok(())
    }

    /// Handle didSave notification
    fn handle_did_save(&self, params: Option<Value>) -> Result<(), JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let _version =
                params["textDocument"].get("version").and_then(|v| v.as_i64()).map(|v| v as i32);

            eprintln!("Document saved: {}", uri);

            // Re-run diagnostics on save to catch any changes
            let documents = self.documents.lock().unwrap();
            if let Some(doc) = self.get_document(&documents, uri) {
                if let Some(ref ast) = doc.ast {
                    // Run diagnostics
                    let provider = DiagnosticsProvider::new(ast, doc.content.clone());
                    let diagnostics =
                        provider.get_diagnostics(ast, &doc.parse_errors, &doc.content);

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
                    let _ = self.notify(
                        "textDocument/publishDiagnostics",
                        json!({
                            "uri": uri,
                            "diagnostics": lsp_diagnostics
                        }),
                    );
                }
            }

            // Optionally, trigger any post-save hooks here
            // For example: format on save, run tests, etc.
        }

        Ok(())
    }

    /// Handle willSave notification
    fn handle_will_save(&self, params: Option<Value>) -> Result<(), JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let reason = params["reason"].as_u64().unwrap_or(1); // 1 = Manual, 2 = AfterDelay, 3 = FocusOut

            eprintln!("Document will save: {} (reason: {})", uri, reason);

            // Pre-save validation or cleanup can be done here
            // For example: remove trailing whitespace, fix imports, etc.
        }

        Ok(())
    }

    /// Handle willSaveWaitUntil request
    fn handle_will_save_wait_until(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let _reason = params["reason"].as_u64().unwrap_or(1);

            eprintln!("Document will save wait until: {}", uri);

            let documents = self.documents.lock().unwrap();
            if let Some(doc) = self.get_document(&documents, uri) {
                // Return text edits to be applied before saving
                // For example: format document, organize imports, etc.

                // Check if we should format on save
                let config = self.config.lock().unwrap();
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

                    if let Ok(edits) = formatter.format_document(&doc.content, &format_options) {
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
    #[allow(dead_code)]
    fn get_document_end_position(&self, content: &str) -> Value {
        let lines: Vec<&str> = content.lines().collect();
        let last_line = lines.len().saturating_sub(1);
        let last_char = lines.last().map(|l| l.len()).unwrap_or(0);

        json!({
            "line": last_line,
            "character": last_char
        })
    }

    /// Format type information concisely for completion detail
    fn format_type_for_detail(t: &crate::type_inference::PerlType) -> String {
        use crate::type_inference::PerlType;
        match t {
            PerlType::Scalar(_) => "scalar".to_string(),
            PerlType::Array(_) => "array".to_string(),
            PerlType::Hash { .. } => "hash".to_string(),
            PerlType::Subroutine { .. } => "code".to_string(),
            PerlType::Reference(inner) => format!("ref {}", Self::format_type_for_detail(inner)),
            PerlType::Object(name) => format!("object {}", name),
            PerlType::Glob => "glob".to_string(),
            PerlType::Union(_) => "mixed".to_string(),
            PerlType::Any => "any".to_string(),
            PerlType::Void => "void".to_string(),
        }
    }

    /// Handle completion request
    fn handle_completion(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let line = params["position"]["line"].as_u64().unwrap_or(0) as u32;
            let character = params["position"]["character"].as_u64().unwrap_or(0) as u32;

            // Reject stale requests
            let req_version = params["textDocument"]["version"].as_i64().map(|n| n as i32);
            self.ensure_latest(uri, req_version)?;

            let documents = self.documents.lock().unwrap();
            if let Some(doc) = self.get_document(&documents, uri) {
                let offset = self.pos16_to_offset(doc, line, character);

                // Get completions, with fallback for missing AST
                #[cfg_attr(not(feature = "workspace"), allow(unused_mut))]
                let mut completions = if let Some(ast) = &doc.ast {
                    // Get completions from the local completion provider
                    #[cfg(feature = "workspace")]
                    let provider = CompletionProvider::new_with_index_and_source(
                        ast,
                        &doc.content,
                        self.workspace_index.clone(),
                    );
                    
                    #[cfg(not(feature = "workspace"))]
                    let provider = CompletionProvider::new_with_index_and_source(
                        ast,
                        &doc.content,
                        None,
                    );

                    let mut base_completions =
                        provider.get_completions_with_path(&doc.content, offset, Some(uri));

                    // Enhance completions with type information
                    let mut type_engine = TypeInferenceEngine::new();
                    let _ = type_engine.infer(ast); // Build type environment

                    // Add type information to completion items where possible
                    for completion in &mut base_completions {
                        // Add type detail to variables based on inferred types
                        if completion.kind == CompletionItemKind::Variable {
                            // Try to get the actual inferred type for the variable
                            let var_name =
                                completion.label.trim_start_matches(['$', '@', '%', '&']);
                            if let Some(perl_type) = type_engine.get_type_at(var_name) {
                                completion.detail = Some(Self::format_type_for_detail(&perl_type));
                            } else {
                                // Fallback to sigil-based type hint
                                let type_hint = if completion.label.starts_with('$') {
                                    "scalar"
                                } else if completion.label.starts_with('@') {
                                    "array"
                                } else if completion.label.starts_with('%') {
                                    "hash"
                                } else if completion.label.starts_with('&') {
                                    "code"
                                } else {
                                    "unknown"
                                };
                                completion.detail = Some(type_hint.to_string());
                            }
                        }
                    }

                    base_completions
                } else {
                    // Fallback: provide basic keyword completions when AST is unavailable
                    self.lexical_complete(&doc.content, offset)
                };

                // Add workspace-wide completions (functions and modules from other files)
                #[cfg(feature = "workspace")]
                if let Some(ref workspace_index) = self.workspace_index {
                    // Get the current context to filter relevant completions
                    let text_before = &doc.content[..offset.min(doc.content.len())];
                    let prefix = text_before
                        .chars()
                        .rev()
                        .take_while(|&c| c.is_alphanumeric() || c == '_' || c == ':')
                        .collect::<String>()
                        .chars()
                        .rev()
                        .collect::<String>();

                    // Find matching symbols in the workspace
                    let workspace_symbols = workspace_index.find_symbols(&prefix);

                    // Add unique workspace symbols as completions
                    use std::collections::HashSet;
                    let mut seen = HashSet::new();
                    for completion in &completions {
                        seen.insert(completion.label.clone());
                    }

                    for symbol in workspace_symbols {
                        // Skip if already in local completions
                        if seen.contains(&symbol.name) {
                            continue;
                        }

                        // Add workspace symbol as completion
                        let kind = match symbol.kind {
                            crate::workspace_index::SymbolKind::Package => {
                                CompletionItemKind::Module
                            }
                            crate::workspace_index::SymbolKind::Subroutine => {
                                CompletionItemKind::Function
                            }
                            crate::workspace_index::SymbolKind::Variable => {
                                CompletionItemKind::Variable
                            }
                            crate::workspace_index::SymbolKind::Class => CompletionItemKind::Module,
                            crate::workspace_index::SymbolKind::Method => {
                                CompletionItemKind::Function
                            }
                            crate::workspace_index::SymbolKind::Constant => {
                                CompletionItemKind::Constant
                            }
                            crate::workspace_index::SymbolKind::Role => CompletionItemKind::Module,
                            crate::workspace_index::SymbolKind::Import => {
                                CompletionItemKind::Module
                            }
                            crate::workspace_index::SymbolKind::Export => {
                                CompletionItemKind::Function
                            }
                        };

                        completions.push(crate::completion::CompletionItem {
                            label: symbol.name.clone(),
                            kind,
                            detail: symbol.qualified_name,
                            insert_text: Some(symbol.name),
                            sort_text: None,
                            filter_text: None,
                            documentation: None,
                            additional_edits: Vec::new(),
                            text_edit_range: None, // Workspace completions don't need precise text edit
                        });
                    }
                }

                let items: Vec<Value> = completions
                    .into_iter()
                    .map(|c| {
                        let mut item = json!({
                            "label": c.label,
                            "kind": match c.kind {
                                CompletionItemKind::Variable => 6,
                                CompletionItemKind::Function => 3,
                                CompletionItemKind::Keyword => 14,
                                CompletionItemKind::Module => 9,
                                CompletionItemKind::File => 17,
                                CompletionItemKind::Snippet => 15,
                                CompletionItemKind::Constant => 14,
                                CompletionItemKind::Property => 7,
                            },
                            "detail": c.detail,
                            "insertText": c.insert_text,
                            "insertTextFormat": 1,  // 1=PlainText, 2=Snippet
                        });

                        // Only add commit characters for functions and variables, not keywords
                        let needs_commit_chars = matches!(
                            c.kind,
                            CompletionItemKind::Function
                                | CompletionItemKind::Variable
                                | CompletionItemKind::Module
                                | CompletionItemKind::Constant
                        );
                        if needs_commit_chars {
                            item["commitCharacters"] = json!([";", " ", ")", "]", "}"]);
                        }

                        item
                    })
                    .collect();

                eprintln!("Returning {} completions", items.len());
                return Ok(Some(json!({"isIncomplete": false, "items": items})));
            }
        }

        Ok(Some(json!({"isIncomplete": false, "items": []})))
    }

    /// Handle code action request
    fn handle_code_action(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let start_line = params["range"]["start"]["line"].as_u64().unwrap_or(0) as u32;
            let start_char = params["range"]["start"]["character"].as_u64().unwrap_or(0) as u32;
            let end_line = params["range"]["end"]["line"].as_u64().unwrap_or(0) as u32;
            let end_char = params["range"]["end"]["character"].as_u64().unwrap_or(0) as u32;

            let documents = self.documents.lock().unwrap();
            if let Some(doc) = self.get_document(&documents, uri) {
                if let Some(ast) = &doc.ast {
                    let start_offset = self.pos16_to_offset(doc, start_line, start_char);
                    let end_offset = self.pos16_to_offset(doc, end_line, end_char);

                    // Get diagnostics from the document
                    let diag_provider = DiagnosticsProvider::new(ast, doc.content.clone());
                    let diagnostics =
                        diag_provider.get_diagnostics(ast, &doc.parse_errors, &doc.content);

                    // Get code actions from both providers
                    let mut code_actions: Vec<Value> = Vec::new();

                    // Add Perl::Critic quick fixes
                    let builtin_analyzer = BuiltInAnalyzer::new();
                    let violations = builtin_analyzer.analyze(ast, &doc.content);
                    for violation in &violations {
                        if let Some(quick_fix) =
                            builtin_analyzer.get_quick_fix(violation, &doc.content)
                        {
                            let mut changes = HashMap::new();
                            let (start_line, start_char) =
                                self.offset_to_pos16(doc, violation.range.start.byte);
                            let (end_line, end_char) =
                                self.offset_to_pos16(doc, violation.range.end.byte);

                            changes.insert(
                                uri.to_string(),
                                vec![json!({
                                    "range": {
                                        "start": {"line": start_line, "character": start_char},
                                        "end": {"line": end_line, "character": end_char},
                                    },
                                    "newText": quick_fix.edit.new_text,
                                })],
                            );

                            code_actions.push(json!({
                                "title": quick_fix.title,
                                "kind": "quickfix",
                                "diagnostics": [{
                                    "range": {
                                        "start": {"line": start_line, "character": start_char},
                                        "end": {"line": end_line, "character": end_char},
                                    },
                                    "severity": match violation.severity {
                                        crate::perl_critic::Severity::Brutal |
                                        crate::perl_critic::Severity::Cruel => 1, // Error
                                        crate::perl_critic::Severity::Harsh => 2, // Warning
                                        _ => 3, // Information
                                    },
                                    "code": violation.policy.clone(),
                                    "source": "Perl::Critic",
                                    "message": violation.description.clone()
                                }],
                                "edit": {
                                    "changes": changes,
                                },
                            }));
                        }
                    }

                    // Get quick-fixes from the V2 provider (diagnostic-based)
                    let provider_v2 = CodeActionsProviderV2::new(doc.content.clone());
                    let quick_fixes =
                        provider_v2.get_code_actions((start_offset, end_offset), &diagnostics);

                    for action in quick_fixes {
                        let mut changes = HashMap::new();
                        let (start_line, start_char) =
                            self.offset_to_pos16(doc, action.edit.range.0);
                        let (end_line, end_char) = self.offset_to_pos16(doc, action.edit.range.1);

                        let edits = vec![json!({
                            "range": {
                                "start": {"line": start_line, "character": start_char},
                                "end": {"line": end_line, "character": end_char},
                            },
                            "newText": action.edit.new_text,
                        })];
                        changes.insert(uri.to_string(), edits);

                        code_actions.push(json!({
                            "title": action.title,
                            "kind": match action.kind {
                                InternalCodeActionKindV2::QuickFix => "quickfix",
                                InternalCodeActionKindV2::Refactor => "refactor",
                                InternalCodeActionKindV2::RefactorExtract => "refactor.extract",
                                InternalCodeActionKindV2::RefactorInline => "refactor.inline",
                                InternalCodeActionKindV2::RefactorRewrite => "refactor.rewrite",
                            },
                            "edit": {
                                "changes": changes,
                            },
                        }));
                    }

                    // Get refactorings from the original provider (AST-based)
                    let provider = CodeActionsProvider::new(doc.content.clone());
                    let actions =
                        provider.get_code_actions(ast, (start_offset, end_offset), &diagnostics);

                    for action in actions {
                        let mut changes = HashMap::new();
                        let edits: Vec<Value> = action
                            .edit
                            .changes
                            .into_iter()
                            .map(|edit| {
                                let (start_line, start_char) =
                                    self.offset_to_pos16(doc, edit.location.start);
                                let (end_line, end_char) =
                                    self.offset_to_pos16(doc, edit.location.end);
                                json!({
                                    "range": {
                                        "start": {"line": start_line, "character": start_char},
                                        "end": {"line": end_line, "character": end_char},
                                    },
                                    "newText": edit.new_text,
                                })
                            })
                            .collect();
                        changes.insert(uri.to_string(), edits);

                        code_actions.push(json!({
                            "title": action.title,
                            "kind": match action.kind {
                                InternalCodeActionKind::QuickFix => "quickfix",
                                InternalCodeActionKind::Refactor => "refactor",
                                InternalCodeActionKind::RefactorExtract => "refactor.extract",
                                _ => "quickfix",
                            },
                            "edit": {
                                "changes": changes,
                            },
                        }));
                    }

                    // Get enhanced refactorings (extract variable, convert loops, etc.)
                    let enhanced_provider = EnhancedCodeActionsProvider::new(doc.content.clone());
                    let enhanced_actions = enhanced_provider
                        .get_enhanced_refactoring_actions(ast, (start_offset, end_offset));

                    // Add test generation actions
                    let test_generator = TestGenerator::new("Test::More");
                    let subroutines = test_generator.find_subroutines(ast);

                    for action in enhanced_actions {
                        let mut changes = HashMap::new();
                        let edits: Vec<Value> = action
                            .edit
                            .changes
                            .into_iter()
                            .map(|edit| {
                                let (start_line, start_char) =
                                    self.offset_to_pos16(doc, edit.location.start);
                                let (end_line, end_char) =
                                    self.offset_to_pos16(doc, edit.location.end);
                                json!({
                                    "range": {
                                        "start": {"line": start_line, "character": start_char},
                                        "end": {"line": end_line, "character": end_char},
                                    },
                                    "newText": edit.new_text,
                                })
                            })
                            .collect();
                        changes.insert(uri.to_string(), edits);

                        code_actions.push(json!({
                            "title": action.title,
                            "kind": match action.kind {
                                InternalCodeActionKind::QuickFix => "quickfix",
                                InternalCodeActionKind::Refactor => "refactor",
                                InternalCodeActionKind::RefactorExtract => "refactor.extract",
                                InternalCodeActionKind::RefactorRewrite => "refactor.rewrite",
                                _ => "refactor",
                            },
                            "edit": {
                                "changes": changes,
                            },
                        }));
                    }

                    // Add test generation actions for subroutines in range
                    for sub_info in subroutines {
                        // Check if cursor is near this subroutine
                        let test_code =
                            test_generator.generate_test(&sub_info.name, sub_info.param_count);
                        code_actions.push(json!({
                            "title": format!("Generate test for '{}'", sub_info.name),
                            "kind": "source",
                            "command": {
                                "title": "Generate test",
                                "command": "perl.generateTest",
                                "arguments": [json!({
                                    "uri": uri,
                                    "name": sub_info.name,
                                    "test": test_code
                                })]
                            }
                        }));
                    }

                    // Always offer generic debug actions when there are diagnostics
                    if !diagnostics.is_empty() {
                        // Add debug print action
                        code_actions.push(json!({
                            "title": "Add debug print",
                            "kind": "refactor.rewrite",
                            "command": {
                                "title": "Add debug print",
                                "command": "perl.addDebugPrint",
                                "arguments": [json!({ "uri": uri, "range": {
                                    "start": {"line": start_line, "character": start_char},
                                    "end": {"line": end_line, "character": end_char}
                                }})]
                            }
                        }));

                        // Extract variable action
                        code_actions.push(json!({
                            "title": "Extract variable",
                            "kind": "refactor.extract",
                            "command": {
                                "title": "Extract variable",
                                "command": "perl.extractVariable",
                                "arguments": [json!({ "uri": uri, "range": {
                                    "start": {"line": start_line, "character": start_char},
                                    "end": {"line": end_line, "character": end_char}
                                }})]
                            }
                        }));
                    }

                    return Ok(Some(json!(code_actions)));
                } else {
                    // No AST (parse error), but we can still offer some actions
                    let mut code_actions: Vec<Value> = Vec::new();

                    // Check if source lacks strict/warnings
                    if !doc.content.contains("use strict") || !doc.content.contains("use warnings")
                    {
                        let mut changes = HashMap::new();
                        // Find first non-shebang line
                        let insert_pos = if doc.content.starts_with("#!") {
                            doc.content.find('\n').map(|p| p + 1).unwrap_or(0)
                        } else {
                            0
                        };

                        let new_text = if !doc.content.contains("use strict")
                            && !doc.content.contains("use warnings")
                        {
                            "use strict;\nuse warnings;\n\n"
                        } else if !doc.content.contains("use strict") {
                            "use strict;\n"
                        } else {
                            "use warnings;\n"
                        };

                        let (line, char) = self.offset_to_pos16(doc, insert_pos);
                        changes.insert(
                            uri.to_string(),
                            vec![json!({
                                "range": {
                                    "start": {"line": line, "character": char},
                                    "end": {"line": line, "character": char},
                                },
                                "newText": new_text,
                            })],
                        );

                        code_actions.push(json!({
                            "title": "Add 'use strict' and 'use warnings'",
                            "kind": "quickfix",
                            "edit": {
                                "changes": changes,
                            },
                        }));
                    }

                    // Always offer debug actions for files with issues
                    code_actions.push(json!({
                        "title": "Add debug print",
                        "kind": "refactor.rewrite",
                        "command": {
                            "title": "Add debug print",
                            "command": "perl.addDebugPrint",
                            "arguments": [json!({ "uri": uri })]
                        }
                    }));

                    // Check for global variables that could use 'my' declarations
                    let global_var_pattern =
                        regex::Regex::new(r"(?m)^(\$|\@|\%)[a-zA-Z_]\w*\s*=").ok();
                    if let Some(re) = global_var_pattern {
                        if re.is_match(&doc.content) {
                            code_actions.push(json!({
                                "title": "Convert globals to 'my' declarations",
                                "kind": "refactor.rewrite",
                                "command": {
                                    "title": "Convert to my declarations",
                                    "command": "perl.convertToMyDeclarations",
                                    "arguments": [json!({ "uri": uri })]
                                }
                            }));
                        }
                    }

                    return Ok(Some(json!(code_actions)));
                }
            }
        }

        Ok(Some(json!([])))
    }

    /// Handle hover request
    fn handle_hover(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let line = params["position"]["line"].as_u64().unwrap_or(0) as u32;
            let character = params["position"]["character"].as_u64().unwrap_or(0) as u32;

            // Reject stale requests
            let req_version = params["textDocument"]["version"].as_i64().map(|n| n as i32);
            self.ensure_latest(uri, req_version)?;

            let documents = self.documents.lock().unwrap();
            if let Some(doc) = self.get_document(&documents, uri) {
                if let Some(ast) = &doc.ast {
                    let offset = self.pos16_to_offset(doc, line, character);

                    // Use SemanticAnalyzer for type information
                    let analyzer = crate::semantic::SemanticAnalyzer::analyze(ast);
                    let source_loc = crate::SourceLocation { start: offset, end: offset + 1 };

                    // Try to get symbol information from semantic analyzer
                    if let Some(symbol_info) = analyzer.symbol_at(source_loc) {
                        // Get symbol kind as string
                        let kind_str = match symbol_info.kind {
                            crate::symbol::SymbolKind::ScalarVariable => "Scalar Variable",
                            crate::symbol::SymbolKind::ArrayVariable => "Array Variable",
                            crate::symbol::SymbolKind::HashVariable => "Hash Variable",
                            crate::symbol::SymbolKind::Subroutine => "Subroutine",
                            crate::symbol::SymbolKind::Package => "Package",
                            crate::symbol::SymbolKind::Constant => "Constant",
                            crate::symbol::SymbolKind::Label => "Label",
                            crate::symbol::SymbolKind::Format => "Format",
                        };

                        // Add sigil if applicable
                        let sigil = symbol_info.kind.sigil().unwrap_or("");
                        let full_name = format!("{}{}", sigil, symbol_info.name);

                        // Add declaration type if available
                        let decl_info = symbol_info
                            .declaration
                            .as_ref()
                            .map(|d| format!("\n**Declaration**: `{}`", d))
                            .unwrap_or_default();

                        // Add documentation if available
                        let doc_info = symbol_info
                            .documentation
                            .as_ref()
                            .map(|d| format!("\n\n{}", d))
                            .unwrap_or_default();

                        return Ok(Some(json!({
                            "contents": {
                                "kind": "markdown",
                                "value": format!("**{}**\n\n`{}`{}{}",
                                    kind_str,
                                    full_name,
                                    decl_info,
                                    doc_info
                                ),
                            },
                        })));
                    }

                    // Fall back to simple token display
                    let hover_text = self.get_token_at_position(&doc.content, offset);

                    if !hover_text.is_empty() {
                        return Ok(Some(json!({
                            "contents": {
                                "kind": "markdown",
                                "value": format!("**Perl**: `{}`", hover_text),
                            },
                        })));
                    }
                }
            }
        }

        Ok(Some(json!(null)))
    }

    /// Handle textDocument/signatureHelp request
    fn handle_signature_help(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let line = params["position"]["line"].as_u64().unwrap_or(0) as u32;
            let character = params["position"]["character"].as_u64().unwrap_or(0) as u32;

            let documents = self.documents.lock().unwrap();
            if let Some(doc) = self.get_document(&documents, uri) {
                let offset = self.pos16_to_offset(doc, line, character);

                // Find the function call context at this position
                if let Some((function_name, active_param)) =
                    self.find_function_context(&doc.content, offset)
                {
                    // Try to get signature from user-defined functions first (if AST exists)
                    if let Some(ref ast) = doc.ast {
                        if let Some(signature) =
                            self.get_user_function_signature(ast, &function_name)
                        {
                            return Ok(Some(json!({
                                "signatures": [signature],
                                "activeSignature": 0,
                                "activeParameter": active_param
                            })));
                        }
                    }

                    // Fall back to built-in functions
                    if let Some(signature) = self.get_function_signature(&function_name) {
                        return Ok(Some(json!({
                            "signatures": [signature],
                            "activeSignature": 0,
                            "activeParameter": active_param
                        })));
                    }

                    // If no signature found, return a generic one
                    return Ok(Some(json!({
                        "signatures": [json!({
                            "label": format!("{}(...)", function_name),
                            "documentation": null,
                            "parameters": []
                        })],
                        "activeSignature": 0,
                        "activeParameter": active_param
                    })));
                }
            }
        }

        Ok(None)
    }

    /// Find function context at position (returns function name and active parameter index)
    fn find_function_context(&self, content: &str, offset: usize) -> Option<(String, usize)> {
        let chars: Vec<char> = content.chars().collect();
        if offset > chars.len() {
            return None;
        }

        // Find the opening parenthesis, tracking all bracket types
        let mut paren_pos = None;
        let mut depth = 0;
        let mut i = if offset > 0 { offset - 1 } else { return None };

        loop {
            match chars[i] {
                ')' => depth += 1,
                ']' => depth += 1,
                '}' => depth += 1,
                '(' => {
                    if depth == 0 {
                        paren_pos = Some(i);
                        break;
                    }
                    depth -= 1;
                }
                '[' | '{' => {
                    if depth > 0 {
                        depth -= 1;
                    }
                }
                _ => {}
            }

            if i == 0 {
                break;
            }
            i -= 1;
        }

        let paren_pos = paren_pos?;

        // Now extract the function name before the parenthesis
        // Handle: func(), $obj->func(), Package::func()
        let mut j = if paren_pos > 0 {
            paren_pos - 1
        } else {
            return None;
        };

        // Skip whitespace before '('
        while j > 0 && chars[j].is_whitespace() {
            j -= 1;
        }

        if j == 0 && !chars[0].is_alphanumeric() && chars[0] != '_' {
            return None;
        }

        let mut end = j + 1;
        let mut start = j;

        // Check for method call pattern (->)
        if j >= 1 && chars[j] == '>' && chars[j - 1] == '-' {
            // This is a method call, extract method name after ->
            // First find where -> starts
            let arrow_end = j - 1; // Position of '-'

            // Now find method name after ->
            j = paren_pos - 1;
            while j > arrow_end + 1 && chars[j].is_whitespace() {
                j -= 1;
            }
            end = j + 1;

            j = arrow_end + 2; // Start after ->
            while j < end && chars[j].is_whitespace() {
                j += 1;
            }
            start = j;
        } else {
            // Regular function or Package::function
            while start > 0 {
                let ch = chars[start];
                if ch.is_alphanumeric() || ch == '_' {
                    start -= 1;
                } else if start >= 2 && ch == ':' && chars[start - 1] == ':' {
                    // Package separator
                    start -= 2;
                } else {
                    // Adjust if we overshot
                    if !ch.is_alphanumeric() && ch != '_' && ch != ':' {
                        start += 1;
                    }
                    break;
                }
            }

            // Handle case where we're at the beginning
            if start == 0 && (chars[0].is_alphanumeric() || chars[0] == '_') {
                // Include first character
            } else if start == 0 {
                start = 1;
            }
        }

        if start >= end {
            return None;
        }

        let full_name: String = chars[start..end].iter().collect();

        // Extract just the function name (strip package prefix if present)
        let func_name =
            if let Some(pos) = full_name.rfind("::") { &full_name[pos + 2..] } else { &full_name };

        // Count commas at depth 0 to determine active parameter
        let mut comma_count = 0;
        let mut depth = 0;
        for k in (paren_pos + 1)..offset.min(chars.len()) {
            match chars[k] {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => depth -= 1,
                ',' if depth == 0 => comma_count += 1,
                _ => {}
            }
        }

        Some((func_name.trim().to_string(), comma_count))
    }

    /// Get function signature information
    /// Get signature for user-defined functions from AST
    fn get_user_function_signature(&self, ast: &Node, function_name: &str) -> Option<Value> {
        // Walk the AST to find the subroutine definition
        let sub_node = self.find_subroutine_definition(ast, function_name)?;

        // Extract parameters from the subroutine
        let mut params = Vec::new();
        if let NodeKind::Subroutine { signature: sub_signature, body, .. } = &sub_node.kind {
            if let Some(sig) = sub_signature {
                if let NodeKind::Signature { parameters } = &sig.kind {
                    for param in parameters {
                        self.extract_params(param, &mut params);
                    }
                }
            } else {
                // Look for my (...) = @_; pattern in the body
                self.extract_params_from_body(body, &mut params);
            }
        }

        // Build signature
        let label = if params.is_empty() {
            format!("sub {}", function_name)
        } else {
            format!("sub {}({})", function_name, params.join(", "))
        };

        let parameters: Vec<Value> = params
            .iter()
            .map(|p| {
                json!({
                    "label": p,
                    "documentation": null
                })
            })
            .collect();

        Some(json!({
            "label": label,
            "documentation": format!("User-defined function '{}'", function_name),
            "parameters": parameters
        }))
    }

    /// Find a subroutine definition by name in the AST
    fn find_subroutine_definition<'a>(&self, node: &'a Node, name: &str) -> Option<&'a Node> {
        match &node.kind {
            NodeKind::Subroutine { name: sub_name, .. } => {
                if let Some(sub_name) = sub_name {
                    if sub_name == name {
                        return Some(node);
                    }
                }
            }
            NodeKind::Program { statements } | NodeKind::Block { statements } => {
                for stmt in statements {
                    if let Some(found) = self.find_subroutine_definition(stmt, name) {
                        return Some(found);
                    }
                }
            }
            _ => {}
        }
        None
    }

    /// Extract parameter names from a params node
    fn extract_params(&self, params_node: &Node, params: &mut Vec<String>) {
        if let NodeKind::Variable { sigil, name } = &params_node.kind {
            params.push(format!("{}{}", sigil, name));
        }
    }

    /// Extract parameters from my (...) = @_; pattern in the body
    fn extract_params_from_body(&self, body: &Node, params: &mut Vec<String>) {
        if let NodeKind::Block { statements } = &body.kind {
            if let Some(first_stmt) = statements.first() {
                // Look for my (...) = @_ pattern
                if let NodeKind::VariableListDeclaration { variables, initializer, .. } =
                    &first_stmt.kind
                {
                    // Check if initializer is @_
                    if let Some(init) = initializer {
                        if let NodeKind::Variable { sigil, name } = &init.kind {
                            if sigil == "@" && name == "_" {
                                // Extract params from variables
                                for var in variables {
                                    if let NodeKind::Variable { sigil: var_sigil, name: var_name } =
                                        &var.kind
                                    {
                                        params.push(format!("{}{}", var_sigil, var_name));
                                    }
                                }
                            }
                        }
                    }
                } else if let NodeKind::Assignment { lhs, rhs, .. } = &first_stmt.kind {
                    // Alternative pattern: ($x, $y) = @_
                    if let NodeKind::Variable { sigil, name } = &rhs.kind {
                        if sigil == "@" && name == "_" {
                            // Extract params from lhs
                            self.extract_params_from_lhs(lhs, params);
                        }
                    }
                }
            }
        }
    }

    /// Helper to extract params from left-hand side of assignment
    fn extract_params_from_lhs(&self, lhs: &Node, params: &mut Vec<String>) {
        match &lhs.kind {
            NodeKind::Variable { sigil, name } => {
                params.push(format!("{}{}", sigil, name));
            }
            NodeKind::VariableListDeclaration { variables, .. } => {
                for var in variables {
                    if let NodeKind::Variable { sigil, name } = &var.kind {
                        params.push(format!("{}{}", sigil, name));
                    }
                }
            }
            _ => {}
        }
    }

    fn get_function_signature(&self, function_name: &str) -> Option<Value> {
        // Define signatures for common Perl built-in functions
        let signature = match function_name {
            "print" => Some(("print LIST", vec!["LIST"])),
            "printf" => Some(("printf FORMAT, LIST", vec!["FORMAT", "LIST"])),
            "open" => Some(("open FILEHANDLE, MODE, EXPR", vec!["FILEHANDLE", "MODE", "EXPR"])),
            "close" => Some(("close FILEHANDLE", vec!["FILEHANDLE"])),
            "read" => Some((
                "read FILEHANDLE, SCALAR, LENGTH, OFFSET",
                vec!["FILEHANDLE", "SCALAR", "LENGTH", "OFFSET"],
            )),
            "write" => Some(("write FILEHANDLE", vec!["FILEHANDLE"])),
            "die" => Some(("die LIST", vec!["LIST"])),
            "warn" => Some(("warn LIST", vec!["LIST"])),
            "substr" => Some((
                "substr EXPR, OFFSET, LENGTH, REPLACEMENT",
                vec!["EXPR", "OFFSET", "LENGTH", "REPLACEMENT"],
            )),
            "length" => Some(("length EXPR", vec!["EXPR"])),
            "index" => Some(("index STR, SUBSTR, POSITION", vec!["STR", "SUBSTR", "POSITION"])),
            "rindex" => Some(("rindex STR, SUBSTR, POSITION", vec!["STR", "SUBSTR", "POSITION"])),
            "sprintf" => Some(("sprintf FORMAT, LIST", vec!["FORMAT", "LIST"])),
            "join" => Some(("join EXPR, LIST", vec!["EXPR", "LIST"])),
            "split" => Some(("split /PATTERN/, EXPR, LIMIT", vec!["/PATTERN/", "EXPR", "LIMIT"])),
            "push" => Some(("push ARRAY, LIST", vec!["ARRAY", "LIST"])),
            "pop" => Some(("pop ARRAY", vec!["ARRAY"])),
            "shift" => Some(("shift ARRAY", vec!["ARRAY"])),
            "unshift" => Some(("unshift ARRAY, LIST", vec!["ARRAY", "LIST"])),
            "splice" => Some((
                "splice ARRAY, OFFSET, LENGTH, LIST",
                vec!["ARRAY", "OFFSET", "LENGTH", "LIST"],
            )),
            "grep" => Some(("grep BLOCK LIST", vec!["BLOCK", "LIST"])),
            "map" => Some(("map BLOCK LIST", vec!["BLOCK", "LIST"])),
            "sort" => Some(("sort BLOCK LIST", vec!["BLOCK", "LIST"])),
            "reverse" => Some(("reverse LIST", vec!["LIST"])),
            "keys" => Some(("keys HASH", vec!["HASH"])),
            "values" => Some(("values HASH", vec!["HASH"])),
            "each" => Some(("each HASH", vec!["HASH"])),
            "exists" => Some(("exists EXPR", vec!["EXPR"])),
            "delete" => Some(("delete EXPR", vec!["EXPR"])),
            "defined" => Some(("defined EXPR", vec!["EXPR"])),
            "undef" => Some(("undef EXPR", vec!["EXPR"])),
            "ref" => Some(("ref EXPR", vec!["EXPR"])),
            "bless" => Some(("bless REF, CLASSNAME", vec!["REF", "CLASSNAME"])),
            "chomp" => Some(("chomp VARIABLE", vec!["VARIABLE"])),
            "chop" => Some(("chop VARIABLE", vec!["VARIABLE"])),
            "chr" => Some(("chr NUMBER", vec!["NUMBER"])),
            "ord" => Some(("ord EXPR", vec!["EXPR"])),
            "lc" => Some(("lc EXPR", vec!["EXPR"])),
            "uc" => Some(("uc EXPR", vec!["EXPR"])),
            "lcfirst" => Some(("lcfirst EXPR", vec!["EXPR"])),
            "ucfirst" => Some(("ucfirst EXPR", vec!["EXPR"])),

            // File operations
            "seek" => Some((
                "seek FILEHANDLE, POSITION, WHENCE",
                vec!["FILEHANDLE", "POSITION", "WHENCE"],
            )),
            "tell" => Some(("tell FILEHANDLE", vec!["FILEHANDLE"])),
            "stat" => Some(("stat EXPR", vec!["EXPR"])),
            "lstat" => Some(("lstat EXPR", vec!["EXPR"])),
            "chmod" => Some(("chmod MODE, LIST", vec!["MODE", "LIST"])),
            "chown" => Some(("chown UID, GID, LIST", vec!["UID", "GID", "LIST"])),
            "unlink" => Some(("unlink LIST", vec!["LIST"])),
            "rename" => Some(("rename OLDNAME, NEWNAME", vec!["OLDNAME", "NEWNAME"])),
            "mkdir" => Some(("mkdir FILENAME, MODE", vec!["FILENAME", "MODE"])),
            "rmdir" => Some(("rmdir FILENAME", vec!["FILENAME"])),
            "opendir" => Some(("opendir DIRHANDLE, EXPR", vec!["DIRHANDLE", "EXPR"])),
            "readdir" => Some(("readdir DIRHANDLE", vec!["DIRHANDLE"])),
            "closedir" => Some(("closedir DIRHANDLE", vec!["DIRHANDLE"])),
            "link" => Some(("link OLDFILE, NEWFILE", vec!["OLDFILE", "NEWFILE"])),
            "symlink" => Some(("symlink OLDFILE, NEWFILE", vec!["OLDFILE", "NEWFILE"])),
            "readlink" => Some(("readlink EXPR", vec!["EXPR"])),
            "truncate" => Some(("truncate FILEHANDLE, LENGTH", vec!["FILEHANDLE", "LENGTH"])),

            // String/Data functions
            "pack" => Some(("pack TEMPLATE, LIST", vec!["TEMPLATE", "LIST"])),
            "unpack" => Some(("unpack TEMPLATE, EXPR", vec!["TEMPLATE", "EXPR"])),
            "quotemeta" => Some(("quotemeta EXPR", vec!["EXPR"])),
            "hex" => Some(("hex EXPR", vec!["EXPR"])),
            "oct" => Some(("oct EXPR", vec!["EXPR"])),
            "vec" => Some(("vec EXPR, OFFSET, BITS", vec!["EXPR", "OFFSET", "BITS"])),
            "crypt" => Some(("crypt PLAINTEXT, SALT", vec!["PLAINTEXT", "SALT"])),

            // Array/List functions
            "scalar" => Some(("scalar EXPR", vec!["EXPR"])),
            "wantarray" => Some(("wantarray", vec![])),

            // Math functions
            "abs" => Some(("abs VALUE", vec!["VALUE"])),
            "int" => Some(("int EXPR", vec!["EXPR"])),
            "sqrt" => Some(("sqrt EXPR", vec!["EXPR"])),
            "exp" => Some(("exp EXPR", vec!["EXPR"])),
            "log" => Some(("log EXPR", vec!["EXPR"])),
            "sin" => Some(("sin EXPR", vec!["EXPR"])),
            "cos" => Some(("cos EXPR", vec!["EXPR"])),
            "tan" => Some(("tan EXPR", vec!["EXPR"])),
            "atan2" => Some(("atan2 Y, X", vec!["Y", "X"])),
            "rand" => Some(("rand EXPR", vec!["EXPR"])),
            "srand" => Some(("srand EXPR", vec!["EXPR"])),

            // System/Process functions
            "system" => Some(("system LIST", vec!["LIST"])),
            "exec" => Some(("exec LIST", vec!["LIST"])),
            "fork" => Some(("fork", vec![])),
            "wait" => Some(("wait", vec![])),
            "waitpid" => Some(("waitpid PID, FLAGS", vec!["PID", "FLAGS"])),
            "kill" => Some(("kill SIGNAL, LIST", vec!["SIGNAL", "LIST"])),
            "sleep" => Some(("sleep EXPR", vec!["EXPR"])),
            "alarm" => Some(("alarm SECONDS", vec!["SECONDS"])),
            "exit" => Some(("exit EXPR", vec!["EXPR"])),
            "getpgrp" => Some(("getpgrp PID", vec!["PID"])),
            "setpgrp" => Some(("setpgrp PID, PGRP", vec!["PID", "PGRP"])),
            "getppid" => Some(("getppid", vec![])),
            "getpriority" => Some(("getpriority WHICH, WHO", vec!["WHICH", "WHO"])),
            "setpriority" => {
                Some(("setpriority WHICH, WHO, PRIORITY", vec!["WHICH", "WHO", "PRIORITY"]))
            }

            // Time functions
            "time" => Some(("time", vec![])),
            "localtime" => Some(("localtime EXPR", vec!["EXPR"])),
            "gmtime" => Some(("gmtime EXPR", vec!["EXPR"])),
            "times" => Some(("times", vec![])),

            // User/Group functions
            "getpwuid" => Some(("getpwuid UID", vec!["UID"])),
            "getpwnam" => Some(("getpwnam NAME", vec!["NAME"])),
            "getgrgid" => Some(("getgrgid GID", vec!["GID"])),
            "getgrnam" => Some(("getgrnam NAME", vec!["NAME"])),
            "getlogin" => Some(("getlogin", vec![])),

            // Network functions
            "socket" => Some((
                "socket SOCKET, DOMAIN, TYPE, PROTOCOL",
                vec!["SOCKET", "DOMAIN", "TYPE", "PROTOCOL"],
            )),
            "bind" => Some(("bind SOCKET, NAME", vec!["SOCKET", "NAME"])),
            "listen" => Some(("listen SOCKET, QUEUESIZE", vec!["SOCKET", "QUEUESIZE"])),
            "accept" => {
                Some(("accept NEWSOCKET, GENERICSOCKET", vec!["NEWSOCKET", "GENERICSOCKET"]))
            }
            "connect" => Some(("connect SOCKET, NAME", vec!["SOCKET", "NAME"])),
            "send" => Some(("send SOCKET, MSG, FLAGS, TO", vec!["SOCKET", "MSG", "FLAGS", "TO"])),
            "recv" => Some((
                "recv SOCKET, SCALAR, LENGTH, FLAGS",
                vec!["SOCKET", "SCALAR", "LENGTH", "FLAGS"],
            )),
            "shutdown" => Some(("shutdown SOCKET, HOW", vec!["SOCKET", "HOW"])),
            "getsockname" => Some(("getsockname SOCKET", vec!["SOCKET"])),
            "getpeername" => Some(("getpeername SOCKET", vec!["SOCKET"])),

            // Control flow
            "eval" => Some(("eval EXPR", vec!["EXPR"])),
            "require" => Some(("require EXPR", vec!["EXPR"])),
            "do" => Some(("do EXPR", vec!["EXPR"])),
            "caller" => Some(("caller EXPR", vec!["EXPR"])),
            "return" => Some(("return LIST", vec!["LIST"])),
            "goto" => Some(("goto LABEL", vec!["LABEL"])),
            "last" => Some(("last LABEL", vec!["LABEL"])),
            "next" => Some(("next LABEL", vec!["LABEL"])),
            "redo" => Some(("redo LABEL", vec!["LABEL"])),

            // Misc functions
            "tie" => Some(("tie VARIABLE, CLASSNAME, LIST", vec!["VARIABLE", "CLASSNAME", "LIST"])),
            "untie" => Some(("untie VARIABLE", vec!["VARIABLE"])),
            "tied" => Some(("tied VARIABLE", vec!["VARIABLE"])),
            "dbmopen" => Some(("dbmopen HASH, DBNAME, MODE", vec!["HASH", "DBNAME", "MODE"])),
            "dbmclose" => Some(("dbmclose HASH", vec!["HASH"])),
            "select" => Some(("select FILEHANDLE", vec!["FILEHANDLE"])),
            "syscall" => Some(("syscall NUMBER, LIST", vec!["NUMBER", "LIST"])),
            "dump" => Some(("dump LABEL", vec!["LABEL"])),
            "prototype" => Some(("prototype FUNCTION", vec!["FUNCTION"])),
            "lock" => Some(("lock THING", vec!["THING"])),

            _ => None,
        };

        if let Some((label, params)) = signature {
            let parameters: Vec<Value> = params
                .iter()
                .map(|p| {
                    json!({
                        "label": p.to_string()
                    })
                })
                .collect();

            Some(json!({
                "label": label,
                "parameters": parameters
            }))
        } else {
            None
        }
    }

    /// Handle textDocument/definition request
    fn handle_declaration(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        let t0 = std::time::Instant::now();

        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let line = params["position"]["line"].as_u64().unwrap_or(0) as u32;
            let character = params["position"]["character"].as_u64().unwrap_or(0) as u32;

            let documents = self.documents.lock().unwrap();
            if let Some(doc) = self.get_document(&documents, uri) {
                if let Some(ref ast) = doc.ast {
                    let offset = self.pos16_to_offset(doc, line, character);

                    // Use the Declaration provider - ast is already an Arc
                    let provider = crate::declaration::DeclarationProvider::new(
                        Arc::clone(ast),
                        doc.content.clone(),
                        uri.to_string(),
                    )
                    .with_parent_map(&doc.parent_map)
                    .with_doc_version(doc._version);

                    // Find declaration at the position
                    if let Some(location_links) = provider.find_declaration(offset, doc._version) {
                        // Check client capability and return appropriate format
                        if self.client_capabilities.declaration_link_support {
                            // Return LocationLink format
                            let result: Vec<Value> = location_links
                                .iter()
                                .map(|link| {
                                    let (orig_start_line, orig_start_char) =
                                        self.offset_to_pos16(doc, link.origin_selection_range.0);
                                    let (orig_end_line, orig_end_char) =
                                        self.offset_to_pos16(doc, link.origin_selection_range.1);

                                    let (target_start_line, target_start_char) =
                                        self.offset_to_pos16(doc, link.target_range.0);
                                    let (target_end_line, target_end_char) =
                                        self.offset_to_pos16(doc, link.target_range.1);

                                    let (sel_start_line, sel_start_char) =
                                        self.offset_to_pos16(doc, link.target_selection_range.0);
                                    let (sel_end_line, sel_end_char) =
                                        self.offset_to_pos16(doc, link.target_selection_range.1);

                                    json!({
                                            "originSelectionRange": {
                                                "start": {
                                                    "line": orig_start_line,
                                                    "character": orig_start_char,
                                                },
                                                "end": {
                                                    "line": orig_end_line,
                                                    "character": orig_end_char,
                                                },
                                            },
                                            "targetUri": link.target_uri,
                                            "targetRange": {
                                            "start": {
                                                "line": target_start_line,
                                                "character": target_start_char,
                                            },
                                            "end": {
                                                "line": target_end_line,
                                                "character": target_end_char,
                                            },
                                        },
                                        "targetSelectionRange": {
                                            "start": {
                                                "line": sel_start_line,
                                                "character": sel_start_char,
                                            },
                                            "end": {
                                                "line": sel_end_line,
                                                "character": sel_end_char,
                                            },
                                        },
                                    })
                                })
                                .collect();

                            return Ok(Some(json!(result)));
                        } else {
                            // Down-convert to Location format for clients that don't support LocationLink
                            let result: Vec<Value> = location_links
                                .iter()
                                .map(|link| {
                                    let (sel_start_line, sel_start_char) =
                                        self.offset_to_pos16(doc, link.target_selection_range.0);
                                    let (sel_end_line, sel_end_char) =
                                        self.offset_to_pos16(doc, link.target_selection_range.1);

                                    json!({
                                        "uri": link.target_uri,
                                        "range": {
                                            "start": {
                                                "line": sel_start_line,
                                                "character": sel_start_char,
                                            },
                                            "end": {
                                                "line": sel_end_line,
                                                "character": sel_end_char,
                                            },
                                        },
                                    })
                                })
                                .collect();

                            return Ok(Some(json!(result)));
                        }
                    }
                }

                // Performance monitoring
                let dt = t0.elapsed();
                if doc.content.len() < 50_000 && dt > std::time::Duration::from_millis(50) {
                    eprintln!("[warn] slow declaration: {:?} (uri={})", dt, uri);
                }
            }
        }
        Ok(Some(json!([])))
    }

    fn handle_definition(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let line = params["position"]["line"].as_u64().unwrap_or(0) as u32;
            let character = params["position"]["character"].as_u64().unwrap_or(0) as u32;

            let documents = self.documents.lock().unwrap();
            if let Some(doc) = self.get_document(&documents, uri) {
                // First check if we're on a module name in use/require statement
                let offset = self.pos16_to_offset(doc, line, character);

                // Extract text around cursor to check for module references
                let radius = 50;
                let text_start = offset.saturating_sub(radius);
                let text_around = self.get_text_around_offset(&doc.content, offset, radius);
                let cursor_in_text = offset - text_start;

                // Check for patterns like "use Module::Name", "require Module::Name", or "Module::Name->method"
                if let Some(module_name) =
                    self.extract_module_reference(&text_around, cursor_in_text)
                {
                    // Try to resolve module to file path
                    if let Some(module_path) = self.resolve_module_to_path(&module_name) {
                        return Ok(Some(json!([{
                            "uri": module_path,
                            "range": {
                                "start": {
                                    "line": 0,
                                    "character": 0,
                                },
                                "end": {
                                    "line": 0,
                                    "character": 0,
                                },
                            },
                        }])));
                    }
                }

                // Also check if we're on a package name followed by ->
                let package_pattern = regex::Regex::new(
                    r"([A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*)\s*->",
                )
                .ok();
                if let Some(re) = package_pattern {
                    for cap in re.captures_iter(&text_around) {
                        if let Some(package_match) = cap.get(1) {
                            let match_start = package_match.start();
                            let match_end = package_match.end();

                            // Check if cursor is within the package name
                            if cursor_in_text >= match_start && cursor_in_text <= match_end {
                                let package_name = package_match.as_str();
                                if let Some(module_path) = self.resolve_module_to_path(package_name)
                                {
                                    return Ok(Some(json!([{
                                        "uri": module_path,
                                        "range": {
                                            "start": {
                                                "line": 0,
                                                "character": 0,
                                            },
                                            "end": {
                                                "line": 0,
                                                "character": 0,
                                            },
                                        },
                                    }])));
                                }
                            }
                        }
                    }
                }

                if let Some(ref ast) = doc.ast {
                    let offset = self.pos16_to_offset(doc, line, character);

                    // Try DeclarationProvider first (it handles function calls properly)
                    let provider = crate::declaration::DeclarationProvider::new(
                        Arc::clone(ast),
                        doc.content.clone(),
                        uri.to_string(),
                    )
                    .with_parent_map(&doc.parent_map)
                    .with_doc_version(doc._version);

                    if let Some(location_links) = provider.find_declaration(offset, doc._version) {
                        // Convert to Location format for definition
                        let result: Vec<Value> = location_links
                            .iter()
                            .map(|link| {
                                let (sel_start_line, sel_start_char) =
                                    self.offset_to_pos16(doc, link.target_selection_range.0);
                                let (sel_end_line, sel_end_char) =
                                    self.offset_to_pos16(doc, link.target_selection_range.1);

                                json!({
                                    "uri": link.target_uri,
                                    "range": {
                                        "start": {
                                            "line": sel_start_line,
                                            "character": sel_start_char,
                                        },
                                        "end": {
                                            "line": sel_end_line,
                                            "character": sel_end_char,
                                        },
                                    },
                                })
                            })
                            .collect();

                        if !result.is_empty() {
                            return Ok(Some(json!(result)));
                        }
                    }

                    // Try workspace index for cross-file definitions
                    #[cfg(feature = "workspace")]
                    if let Some(ref workspace_index) = self.workspace_index {
                        // Use symbol_at_cursor to get the symbol key
                        let current_package = crate::declaration::current_package_at(ast, offset);
                        if let Some(symbol_key) =
                            crate::declaration::symbol_at_cursor(ast, offset, current_package)
                        {
                            eprintln!("Looking for definition of {:?}", symbol_key);

                            // Try to find definition using the symbol key
                            if let Some(def_location) = workspace_index.find_def(&symbol_key) {
                                eprintln!("Found definition at {:?}", def_location);
                                // Convert internal Location to LSP Location
                                if let Some(lsp_location) =
                                    crate::workspace_index::lsp_adapter::to_lsp_location(
                                        &def_location,
                                    )
                                {
                                    return Ok(Some(json!([lsp_location])));
                                }
                            }

                            // Also try with find_definition for backward compatibility
                            let symbol_name =
                                if symbol_key.kind == crate::workspace_index::SymKind::Sub {
                                    format!("{}::{}", symbol_key.pkg, symbol_key.name)
                                } else {
                                    symbol_key.name.to_string()
                                };

                            if let Some(def_location) =
                                workspace_index.find_definition(&symbol_name)
                            {
                                eprintln!(
                                    "Found definition via find_definition for {}",
                                    symbol_name
                                );
                                // Convert internal Location to LSP Location
                                if let Some(lsp_location) =
                                    crate::workspace_index::lsp_adapter::to_lsp_location(
                                        &def_location,
                                    )
                                {
                                    return Ok(Some(json!([lsp_location])));
                                }
                            }
                        }
                    }

                    // Fall back to same-file definition
                    let analyzer = crate::semantic::SemanticAnalyzer::analyze(ast);

                    // Find definition at the position
                    if let Some(definition) = analyzer.find_definition(offset) {
                        let (def_line, def_char) =
                            self.offset_to_pos16(doc, definition.location.start);

                        return Ok(Some(json!([{
                            "uri": uri,
                            "range": {
                                "start": {
                                    "line": def_line,
                                    "character": def_char,
                                },
                                "end": {
                                    "line": def_line,
                                    "character": def_char + definition.name.len() as u32,
                                },
                            },
                        }])));
                    }
                }
            }
        }

        Ok(Some(json!([])))
    }

    /// Handle textDocument/typeDefinition request
    fn handle_type_definition(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        use crate::type_definition::TypeDefinitionProvider;

        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let line = params["position"]["line"].as_u64().unwrap_or(0) as u32;
            let character = params["position"]["character"].as_u64().unwrap_or(0) as u32;

            let documents = self.documents.lock().unwrap();
            if let Some(doc) = self.get_document(&documents, uri) {
                if let Some(ref ast) = doc.ast {
                    let provider = TypeDefinitionProvider::new();

                    // Convert documents to HashMap<String, String> for provider
                    let doc_map: HashMap<String, String> =
                        documents.iter().map(|(k, v)| (k.clone(), v.content.clone())).collect();

                    if let Some(locations) =
                        provider.find_type_definition(ast, line, character, uri, &doc_map)
                    {
                        return Ok(Some(json!(locations)));
                    }
                }
            }
        }

        Ok(Some(json!([])))
    }

    /// Handle textDocument/implementation request
    fn handle_implementation(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        use crate::implementation_provider::ImplementationProvider;

        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let line = params["position"]["line"].as_u64().unwrap_or(0) as u32;
            let character = params["position"]["character"].as_u64().unwrap_or(0) as u32;

            let documents = self.documents.lock().unwrap();
            if let Some(doc) = self.get_document(&documents, uri) {
                if let Some(ref ast) = doc.ast {
                    #[cfg(feature = "workspace")]
                    let provider = ImplementationProvider::new(self.workspace_index.clone());
                    
                    #[cfg(not(feature = "workspace"))]
                    let provider = ImplementationProvider::new(None);

                    // Convert documents to HashMap<String, String> for provider
                    let doc_map: HashMap<String, String> =
                        documents.iter().map(|(k, v)| (k.clone(), v.content.clone())).collect();

                    let locations =
                        provider.find_implementations(ast, line, character, uri, &doc_map);
                    return Ok(Some(json!(locations)));
                }
            }
        }

        Ok(Some(json!([])))
    }

    /// Find all implementations (simplified version)
    fn find_all_implementations(
        &self,
        ast: &Node,
        documents: &HashMap<String, DocumentState>,
    ) -> Vec<Location> {
        let mut results = Vec::new();

        // Find packages in current file and look for their implementations
        let mut packages = Vec::new();
        self.find_packages_in_ast(ast, &mut packages);

        for package_name in packages {
            let impls = self.find_subclasses(&package_name, documents);
            results.extend(impls);
        }

        results
    }

    /// Find all packages in an AST
    fn find_packages_in_ast(&self, node: &Node, packages: &mut Vec<String>) {
        if let NodeKind::Package { name, .. } = &node.kind {
            packages.push(name.clone());
        }

        // Traverse based on node type
        match &node.kind {
            NodeKind::Program { statements } => {
                for stmt in statements {
                    self.find_packages_in_ast(stmt, packages);
                }
            }
            NodeKind::Block { statements } => {
                for stmt in statements {
                    self.find_packages_in_ast(stmt, packages);
                }
            }
            NodeKind::Package { block, .. } => {
                if let Some(b) = block {
                    self.find_packages_in_ast(b, packages);
                }
            }
            _ => {}
        }
    }

    /// Find classes that extend a given base class
    fn find_subclasses(
        &self,
        base_class: &str,
        documents: &HashMap<String, DocumentState>,
    ) -> Vec<Location> {
        let mut results = Vec::new();

        for (uri, doc) in documents.iter() {
            if let Some(ref ast) = doc.ast {
                self.find_subclasses_in_ast(ast, base_class, uri, &mut results);
            }
        }

        results
    }

    /// Find subclasses in an AST
    fn find_subclasses_in_ast(
        &self,
        node: &Node,
        base_class: &str,
        uri: &str,
        results: &mut Vec<Location>,
    ) {
        if let NodeKind::Package { name: _name, .. } = &node.kind {
            // Check if this package extends the base class
            // Look for @ISA assignment or 'use base' or 'use parent'
            // This would need proper traversal - simplified for now
            if self.check_inheritance_in_package(node, base_class) {
                // Get source text for position conversion
                let documents = self.documents.lock().unwrap();
                if let Some(doc) = documents.get(uri) {
                    let source_text = &doc.content;
                    // Convert byte offsets to UTF-16 line/column
                    let (start_line, start_col) =
                        crate::position::offset_to_utf16_line_col(source_text, node.location.start);
                    let (end_line, end_col) =
                        crate::position::offset_to_utf16_line_col(source_text, node.location.end);

                    // Create typed Location
                    results.push(Location {
                        uri: parse_uri(uri),
                        range: lsp_types::Range::new(
                            lsp_types::Position::new(start_line, start_col),
                            lsp_types::Position::new(end_line, end_col),
                        ),
                    });
                }
            }
        }

        // Recurse into children based on node type
        match &node.kind {
            NodeKind::Program { statements } | NodeKind::Block { statements } => {
                for stmt in statements {
                    self.find_subclasses_in_ast(stmt, base_class, uri, results);
                }
            }
            NodeKind::Package { block, .. } => {
                if let Some(b) = block {
                    self.find_subclasses_in_ast(b, base_class, uri, results);
                }
            }
            _ => {}
        }
    }

    /// Check if a package inherits from base class (simplified)
    fn check_inheritance_in_package(&self, _node: &Node, _base_class: &str) -> bool {
        // This is a simplified check - would need proper AST traversal
        // to find @ISA assignments and use base/parent statements
        false
    }

    /// Find method implementations in subclasses
    fn find_method_implementations(
        &self,
        base_package: &str,
        method_name: &str,
        documents: &HashMap<String, DocumentState>,
    ) -> Vec<Value> {
        let mut results = Vec::new();

        // First find all subclasses
        let subclasses = self.find_subclasses(base_package, documents);

        // Then find the method in each subclass
        for subclass_loc in subclasses {
            let uri_str = subclass_loc.uri.as_str();
            if let Some(doc) = documents.get(uri_str) {
                if let Some(ref ast) = doc.ast {
                    self.find_method_in_ast(ast, method_name, uri_str, &mut results);
                }
            }
        }

        results
    }

    /// Find a specific method in an AST
    fn find_method_in_ast(
        &self,
        node: &Node,
        method_name: &str,
        uri: &str,
        results: &mut Vec<Value>,
    ) {
        // Check for function declarations (simplified - actual AST uses Subroutine)
        if let NodeKind::Subroutine { name: Some(name), .. } = &node.kind {
            if name == method_name {
                results.push(json!({
                    "uri": uri,
                    "range": {
                        "start": {
                            "line": 0,
                            "character": 0,
                        },
                        "end": {
                            "line": 0,
                            "character": 0,
                        }
                    }
                }));
            }
        }

        // Recurse into children based on node type
        match &node.kind {
            NodeKind::Program { statements } | NodeKind::Block { statements } => {
                for stmt in statements {
                    self.find_method_in_ast(stmt, method_name, uri, results);
                }
            }
            NodeKind::Package { block, .. } => {
                if let Some(b) = block {
                    self.find_method_in_ast(b, method_name, uri, results);
                }
            }
            _ => {}
        }
    }

    /// Handle textDocument/references request
    fn handle_references(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let line = params["position"]["line"].as_u64().unwrap_or(0) as u32;
            let character = params["position"]["character"].as_u64().unwrap_or(0) as u32;
            let include_declaration = if let Some(context) = params.get("context") {
                context["includeDeclaration"].as_bool().unwrap_or(true)
            } else {
                true
            };

            let documents = self.documents.lock().unwrap();
            if let Some(doc) = self.get_document(&documents, uri) {
                if let Some(ref ast) = doc.ast {
                    let offset = self.pos16_to_offset(doc, line, character);

                    // Try workspace index first for cross-file references
                    #[cfg(feature = "workspace")]
                    if let Some(ref workspace_index) = self.workspace_index {
                        // Use symbol_at_cursor to get the symbol key
                        let current_package = crate::declaration::current_package_at(ast, offset);
                        if let Some(symbol_key) =
                            crate::declaration::symbol_at_cursor(ast, offset, current_package)
                        {
                            eprintln!("Looking for references of {:?}", symbol_key);

                            // Try to find references using the symbol key
                            let mut all_refs = workspace_index.find_refs(&symbol_key);

                            // Add the definition if includeDeclaration is true
                            if include_declaration {
                                if let Some(def) = workspace_index.find_def(&symbol_key) {
                                    all_refs.push(def);
                                }
                            }

                            if !all_refs.is_empty() {
                                eprintln!("Found {} references via find_refs", all_refs.len());
                                // Convert internal Locations to LSP Locations
                                let lsp_locations =
                                    crate::workspace_index::lsp_adapter::to_lsp_locations(all_refs);
                                if !lsp_locations.is_empty() {
                                    return Ok(Some(json!(lsp_locations)));
                                }
                            }

                            // Also try with find_references for backward compatibility
                            let symbol_name =
                                if symbol_key.kind == crate::workspace_index::SymKind::Sub {
                                    format!("{}::{}", symbol_key.pkg, symbol_key.name)
                                } else {
                                    symbol_key.name.to_string()
                                };

                            let refs = workspace_index.find_references(&symbol_name);
                            if !refs.is_empty() {
                                eprintln!(
                                    "Found {} references via find_references for {}",
                                    refs.len(),
                                    symbol_name
                                );
                                // Convert internal Locations to LSP Locations
                                let lsp_locations =
                                    crate::workspace_index::lsp_adapter::to_lsp_locations(refs);
                                if !lsp_locations.is_empty() {
                                    return Ok(Some(json!(lsp_locations)));
                                }
                            }
                        }
                    }

                    // Fall back to same-file references
                    let analyzer = crate::semantic::SemanticAnalyzer::analyze(ast);

                    // Find all references at the position
                    let references = analyzer.find_all_references(offset, include_declaration);

                    if !references.is_empty() {
                        let locations: Vec<Value> = references
                            .iter()
                            .map(|loc| {
                                let (start_line, start_char) = self.offset_to_pos16(doc, loc.start);
                                let (end_line, end_char) = self.offset_to_pos16(doc, loc.end);

                                json!({
                                    "uri": uri,
                                    "range": {
                                        "start": {
                                            "line": start_line,
                                            "character": start_char,
                                        },
                                        "end": {
                                            "line": end_line,
                                            "character": end_char,
                                        },
                                    },
                                })
                            })
                            .collect();

                        return Ok(Some(json!(locations)));
                    }
                }
            }
        }

        Ok(Some(json!([])))
    }

    /// Handle textDocument/documentHighlight request
    fn handle_document_highlight(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let line = params["position"]["line"].as_u64().unwrap_or(0) as u32;
            let character = params["position"]["character"].as_u64().unwrap_or(0) as u32;

            let documents = self.documents.lock().unwrap();
            if let Some(doc) = self.get_document(&documents, uri) {
                if let Some(ref ast) = doc.ast {
                    let offset = self.pos16_to_offset(doc, line, character);

                    // Create document highlight provider
                    let provider = DocumentHighlightProvider::new();

                    // Find all highlights at the position
                    let highlights = provider.find_highlights(ast, &doc.content, offset);

                    if !highlights.is_empty() {
                        let lsp_highlights: Vec<Value> = highlights
                            .iter()
                            .map(|highlight| {
                                let (start_line, start_char) =
                                    self.offset_to_pos16(doc, highlight.location.start);
                                let (end_line, end_char) =
                                    self.offset_to_pos16(doc, highlight.location.end);

                                json!({
                                    "range": {
                                        "start": {
                                            "line": start_line,
                                            "character": start_char,
                                        },
                                        "end": {
                                            "line": end_line,
                                            "character": end_char,
                                        },
                                    },
                                    "kind": highlight.kind as u32,
                                })
                            })
                            .collect();

                        return Ok(Some(json!(lsp_highlights)));
                    }
                }
            }
        }

        Ok(Some(json!([])))
    }

    /// Handle textDocument/prepareTypeHierarchy request
    fn handle_prepare_type_hierarchy(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let line = params["position"]["line"].as_u64().unwrap_or(0) as u32;
            let character = params["position"]["character"].as_u64().unwrap_or(0) as u32;

            let documents = self.documents.lock().unwrap();
            if let Some(doc) = self.get_document(&documents, uri) {
                let offset = self.pos16_to_offset(doc, line, character);

                // Try AST-based approach first
                if let Some(ref ast) = doc.ast {
                    // Create type hierarchy provider
                    let provider = TypeHierarchyProvider::new();

                    // Prepare type hierarchy at the position
                    if let Some(items) = provider.prepare(ast, &doc.content, offset) {
                        let lsp_items: Vec<Value> = items
                            .iter()
                            .map(|item| {
                                json!({
                                    "name": item.name,
                                    "kind": item.kind as u32,
                                    "uri": uri,
                                    "range": {
                                        "start": {
                                            "line": item.range.start.line,
                                            "character": item.range.start.character,
                                        },
                                        "end": {
                                            "line": item.range.end.line,
                                            "character": item.range.end.character,
                                        },
                                    },
                                    "selectionRange": {
                                        "start": {
                                            "line": item.selection_range.start.line,
                                            "character": item.selection_range.start.character,
                                        },
                                        "end": {
                                            "line": item.selection_range.end.line,
                                            "character": item.selection_range.end.character,
                                        },
                                    },
                                    "detail": item.detail,
                                    "data": {
                                        "uri": uri,
                                        "name": item.name,
                                    },
                                })
                            })
                            .collect();

                        return Ok(Some(json!(lsp_items)));
                    }
                }

                // Fallback to regex-based approach
                let sub_regex = regex::Regex::new(r"\bsub\s+([a-zA-Z_]\w*)\b").unwrap();
                let package_regex = regex::Regex::new(r"\bpackage\s+([a-zA-Z_][\w:]*)\b").unwrap();

                // Find all subs and packages with their positions
                let mut exact_sub: Option<(String, usize, usize)> = None;
                for cap in sub_regex.captures_iter(&doc.content) {
                    if let (Some(m), Some(name)) = (cap.get(0), cap.get(1)) {
                        if offset >= m.start() && offset <= m.end() {
                            // Exact match - cursor is on this sub
                            exact_sub = Some((name.as_str().to_string(), m.start(), m.end()));
                            break;
                        }
                    }
                }

                if let Some((name, start, end)) = exact_sub {
                    let start_pos = doc.line_starts.offset_to_position(&doc.content, start);
                    let end_pos = doc.line_starts.offset_to_position(&doc.content, end);
                    return Ok(Some(json!([{
                        "name": name,
                        "kind": 12, // Function
                        "uri": uri,
                        "range": {
                            "start": { "line": start_pos.0, "character": start_pos.1 },
                            "end": { "line": end_pos.0, "character": end_pos.1 },
                        },
                        "selectionRange": {
                            "start": { "line": start_pos.0, "character": start_pos.1 },
                            "end": { "line": end_pos.0, "character": end_pos.1 },
                        },
                        "detail": "sub",
                        "data": { "uri": uri, "name": name },
                    }])));
                }

                // Check packages
                let mut exact_pkg: Option<(String, usize, usize)> = None;
                for cap in package_regex.captures_iter(&doc.content) {
                    if let (Some(m), Some(name)) = (cap.get(0), cap.get(1)) {
                        if offset >= m.start() && offset <= m.end() {
                            // Exact match - cursor is on this package
                            exact_pkg = Some((name.as_str().to_string(), m.start(), m.end()));
                            break;
                        }
                    }
                }

                if let Some((name, start, end)) = exact_pkg {
                    let start_pos = doc.line_starts.offset_to_position(&doc.content, start);
                    let end_pos = doc.line_starts.offset_to_position(&doc.content, end);
                    return Ok(Some(json!([{
                        "name": name,
                        "kind": 5, // Class
                        "uri": uri,
                        "range": {
                            "start": { "line": start_pos.0, "character": start_pos.1 },
                            "end": { "line": end_pos.0, "character": end_pos.1 },
                        },
                        "selectionRange": {
                            "start": { "line": start_pos.0, "character": start_pos.1 },
                            "end": { "line": end_pos.0, "character": end_pos.1 },
                        },
                        "detail": "package",
                        "data": { "uri": uri, "name": name },
                    }])));
                }
            }
        }

        Ok(Some(json!([])))
    }

    /// Handle typeHierarchy/supertypes request
    fn handle_type_hierarchy_supertypes(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            if let Some(item) = params.get("item") {
                let uri = item["data"]["uri"].as_str().unwrap_or("");
                let name = item["data"]["name"].as_str().unwrap_or("");

                let documents = self.documents.lock().unwrap();
                if let Some(doc) = documents.get(uri) {
                    if let Some(ref ast) = doc.ast {
                        // Create type hierarchy provider
                        let provider = TypeHierarchyProvider::new();

                        // Create item from request
                        let type_item = crate::type_hierarchy::TypeHierarchyItem {
                            name: name.to_string(),
                            kind: crate::type_hierarchy::SymbolKind::Class,
                            uri: uri.to_string(),
                            range: crate::type_hierarchy::Range {
                                start: crate::type_hierarchy::Position { line: 0, character: 0 },
                                end: crate::type_hierarchy::Position { line: 0, character: 0 },
                            },
                            selection_range: crate::type_hierarchy::Range {
                                start: crate::type_hierarchy::Position { line: 0, character: 0 },
                                end: crate::type_hierarchy::Position { line: 0, character: 0 },
                            },
                            detail: None,
                            data: None,
                        };

                        // Find supertypes
                        let supertypes = provider.find_supertypes(ast, &type_item);

                        let lsp_items: Vec<Value> = supertypes
                            .iter()
                            .map(|item| {
                                json!({
                                    "name": item.name,
                                    "kind": item.kind as u32,
                                    "uri": uri,
                                    "range": {
                                        "start": {
                                            "line": item.range.start.line,
                                            "character": item.range.start.character,
                                        },
                                        "end": {
                                            "line": item.range.end.line,
                                            "character": item.range.end.character,
                                        },
                                    },
                                    "selectionRange": {
                                        "start": {
                                            "line": item.selection_range.start.line,
                                            "character": item.selection_range.start.character,
                                        },
                                        "end": {
                                            "line": item.selection_range.end.line,
                                            "character": item.selection_range.end.character,
                                        },
                                    },
                                    "detail": item.detail,
                                    "data": {
                                        "uri": uri,
                                        "name": item.name,
                                    },
                                })
                            })
                            .collect();

                        return Ok(Some(json!(lsp_items)));
                    }
                }
            }
        }

        Ok(Some(json!([])))
    }

    /// Handle typeHierarchy/subtypes request
    fn handle_type_hierarchy_subtypes(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            if let Some(item) = params.get("item") {
                let uri = item["data"]["uri"].as_str().unwrap_or("");
                let name = item["data"]["name"].as_str().unwrap_or("");

                let documents = self.documents.lock().unwrap();
                if let Some(doc) = documents.get(uri) {
                    if let Some(ref ast) = doc.ast {
                        // Create type hierarchy provider
                        let provider = TypeHierarchyProvider::new();

                        // Create item from request
                        let type_item = crate::type_hierarchy::TypeHierarchyItem {
                            name: name.to_string(),
                            kind: crate::type_hierarchy::SymbolKind::Class,
                            uri: uri.to_string(),
                            range: crate::type_hierarchy::Range {
                                start: crate::type_hierarchy::Position { line: 0, character: 0 },
                                end: crate::type_hierarchy::Position { line: 0, character: 0 },
                            },
                            selection_range: crate::type_hierarchy::Range {
                                start: crate::type_hierarchy::Position { line: 0, character: 0 },
                                end: crate::type_hierarchy::Position { line: 0, character: 0 },
                            },
                            detail: None,
                            data: None,
                        };

                        // Find subtypes
                        let subtypes = provider.find_subtypes(ast, &type_item);

                        let lsp_items: Vec<Value> = subtypes
                            .iter()
                            .map(|item| {
                                json!({
                                    "name": item.name,
                                    "kind": item.kind as u32,
                                    "uri": uri,
                                    "range": {
                                        "start": {
                                            "line": item.range.start.line,
                                            "character": item.range.start.character,
                                        },
                                        "end": {
                                            "line": item.range.end.line,
                                            "character": item.range.end.character,
                                        },
                                    },
                                    "selectionRange": {
                                        "start": {
                                            "line": item.selection_range.start.line,
                                            "character": item.selection_range.start.character,
                                        },
                                        "end": {
                                            "line": item.selection_range.end.line,
                                            "character": item.selection_range.end.character,
                                        },
                                    },
                                    "detail": item.detail,
                                    "data": {
                                        "uri": uri,
                                        "name": item.name,
                                    },
                                })
                            })
                            .collect();

                        return Ok(Some(json!(lsp_items)));
                    }
                }
            }
        }

        Ok(Some(json!([])))
    }

    /// Handle textDocument/rename request
    fn handle_prepare_rename(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let line = params["position"]["line"].as_u64().unwrap_or(0) as u32;
            let character = params["position"]["character"].as_u64().unwrap_or(0) as u32;

            let documents = self.documents.lock().unwrap();
            if let Some(doc) = self.get_document(&documents, uri) {
                if let Some(_ast) = &doc.ast {
                    let offset = self.pos16_to_offset(doc, line, character);

                    // Get the token at the current position
                    let token = self.get_token_at_position(&doc.content, offset);
                    if !token.is_empty()
                        && (token.starts_with('$')
                            || token.starts_with('@')
                            || token.starts_with('%')
                            || token.chars().next().is_some_and(|c| c.is_alphabetic() || c == '_'))
                    {
                        // Find the token bounds
                        let (start_offset, end_offset) =
                            self.get_token_bounds(&doc.content, offset);
                        let (start_line, start_char) = self.offset_to_pos16(doc, start_offset);
                        let (end_line, end_char) = self.offset_to_pos16(doc, end_offset);

                        // Return the range and placeholder text
                        return Ok(Some(json!({
                            "range": {
                                "start": {
                                    "line": start_line,
                                    "character": start_char
                                },
                                "end": {
                                    "line": end_line,
                                    "character": end_char
                                }
                            },
                            "placeholder": token
                        })));
                    }
                }
            }
        }

        // Return null if rename is not possible at this position
        Ok(Some(json!(null)))
    }

    fn handle_rename(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let line = params["position"]["line"].as_u64().unwrap_or(0) as u32;
            let character = params["position"]["character"].as_u64().unwrap_or(0) as u32;
            let new_name = params["newName"].as_str().unwrap_or("");

            // Validate the new name
            if !self.is_valid_identifier(new_name) {
                return Err(JsonRpcError {
                    code: -32602,
                    message: format!("Invalid identifier: {}", new_name),
                    data: None,
                });
            }

            let documents = self.documents.lock().unwrap();
            if let Some(doc) = self.get_document(&documents, uri) {
                if let Some(ref ast) = doc.ast {
                    let offset = self.pos16_to_offset(doc, line, character);

                    // Create semantic analyzer
                    let analyzer = crate::semantic::SemanticAnalyzer::analyze(ast);

                    // Find all references (including definition)
                    let references = analyzer.find_all_references(offset, true);

                    if !references.is_empty() {
                        // Create text edits for all references
                        let mut edits = Vec::new();
                        for location in references {
                            let (start_line, start_char) =
                                self.offset_to_pos16(doc, location.start);
                            let (end_line, end_char) = self.offset_to_pos16(doc, location.end);

                            edits.push(json!({
                                "range": {
                                    "start": { "line": start_line, "character": start_char },
                                    "end": { "line": end_line, "character": end_char }
                                },
                                "newText": new_name
                            }));
                        }

                        // Return WorkspaceEdit
                        return Ok(Some(json!({
                            "changes": {
                                uri: edits
                            }
                        })));
                    }
                }
            }
        }

        // Return empty workspace edit if nothing to rename
        Ok(Some(json!({
            "changes": {}
        })))
    }

    /// Handle textDocument/rename request with workspace support
    fn handle_rename_workspace(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(p) = params {
            if let (Some(uri), Some(line), Some(ch), Some(new_name)) = (
                p.get("textDocument").and_then(|t| t.get("uri")).and_then(|s| s.as_str()),
                p.get("position").and_then(|p| p.get("line")).and_then(|n| n.as_u64()),
                p.get("position").and_then(|p| p.get("character")).and_then(|n| n.as_u64()),
                p.get("newName").and_then(|s| s.as_str()),
            ) {
                let documents = self.documents.lock().unwrap();
                if let Some(doc) = documents.get(uri) {
                    if let Some(ref ast) = doc.ast {
                        let offset = self.pos16_to_offset(doc, line as u32, ch as u32);
                        let current_pkg = crate::declaration::current_package_at(ast, offset);
                        if let Some(key) =
                            crate::declaration::symbol_at_cursor(ast, offset, current_pkg)
                        {
                            #[cfg(feature = "workspace")]
                            if let Some(ref idx) = self.workspace_index {
                                let edits =
                                    crate::workspace_rename::build_rename_edit(idx, &key, new_name);
                                let ws_edit = crate::workspace_rename::to_workspace_edit(edits);
                                return Ok(Some(ws_edit));
                            }
                        }
                    }
                }
            }
        }
        // Return empty edit if we can't resolve
        Ok(Some(json!({"changes": {}})))
    }

    /// Handle textDocument/codeAction request for pragmas
    fn handle_code_actions_pragmas(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(p) = params {
            if let Some(uri) = p["textDocument"]["uri"].as_str() {
                let documents = self.documents.lock().unwrap();
                if let Some(doc) = documents.get(uri) {
                    let mut actions =
                        crate::code_actions_pragmas::missing_pragmas_actions(uri, &doc.content);

                    // Fill in edits with proper ranges
                    for a in &mut actions {
                        let data_info = (
                            a.get("data")
                                .and_then(|d| d.get("uri"))
                                .and_then(|s| s.as_str())
                                .map(|s| s.to_string()),
                            a.get("data").and_then(|d| d.get("insertAt")).and_then(|n| n.as_u64()),
                            a.get("data")
                                .and_then(|d| d.get("text"))
                                .and_then(|s| s.as_str())
                                .map(|s| s.to_string()),
                        );

                        if let (Some(u), Some(off), Some(txt)) = data_info {
                            let (line, col) = self.offset_to_pos16(doc, off as usize);
                            if let Some(obj) = a.as_object_mut() {
                                obj.insert("edit".into(), json!({
                                    "changes": {
                                        u: [{
                                            "range": { "start": {"line": line, "character": col},
                                                       "end":   {"line": line, "character": col} },
                                            "newText": txt
                                        }]
                                    }
                                }));
                                obj.remove("data");
                            }
                        }
                    }
                    return Ok(Some(json!(actions)));
                }
            }
        }
        Ok(Some(json!([])))
    }

    /// Handle codeAction/resolve request
    fn handle_code_action_resolve(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(mut action) = params {
            // The action should already have minimal information
            // We now need to compute the actual edits

            if let Some(kind) = action.get("kind").and_then(|k| k.as_str()) {
                if kind == "quickfix" {
                    // For quickfix actions, compute the workspace edit now
                    if let Some(data) = action.get("data") {
                        if let Some(uri) = data.get("uri").and_then(|u| u.as_str()) {
                            let documents = self.documents.lock().unwrap();
                            if self.get_document(&documents, uri).is_some() {
                                // Example: Add "use strict;" at the beginning
                                if let Some(pragma) = data.get("pragma").and_then(|p| p.as_str()) {
                                    let text = format!("{}\n", pragma);
                                    let edit = json!({
                                        "changes": {
                                            uri: [{
                                                "range": {
                                                    "start": {"line": 0, "character": 0},
                                                    "end": {"line": 0, "character": 0}
                                                },
                                                "newText": text
                                            }]
                                        }
                                    });

                                    if let Some(obj) = action.as_object_mut() {
                                        obj.insert("edit".to_string(), edit);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            Ok(Some(action))
        } else {
            Ok(None)
        }
    }

    /// Handle textDocument/semanticTokens/full request  
    fn handle_semantic_tokens(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        if let Some(p) = params {
            let uri = p["textDocument"]["uri"].as_str().ok_or_else(|| JsonRpcError {
                code: ERR_INVALID_PARAMS,
                message: "Missing textDocument.uri".into(),
                data: None,
            })?;
            let documents = self.documents.lock().unwrap();
            let doc = self.get_document(&documents, uri).ok_or_else(|| JsonRpcError {
                code: ERR_INVALID_REQUEST,
                message: format!("Document not open: {}", uri),
                data: None,
            })?;
            if let Some(ref ast) = doc.ast {
                let data =
                    crate::semantic_tokens::collect_semantic_tokens(ast, &doc.content, &|off| {
                        self.offset_to_pos16(doc, off)
                    });
                return Ok(Some(json!({ "data": data.into_iter().flatten().collect::<Vec<_>>() })));
            }
        }
        Ok(Some(json!({ "data": [] })))
    }

    /// Handle textDocument/inlayHint request
    fn handle_inlay_hints(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        if let Some(p) = params {
            let uri = p["textDocument"]["uri"].as_str().ok_or_else(|| JsonRpcError {
                code: ERR_INVALID_PARAMS,
                message: "Missing textDocument.uri".into(),
                data: None,
            })?;

            // Extract the range parameter (required by LSP spec)
            let range = if let Some(range_val) = p.get("range") {
                let start_line = range_val["start"]["line"].as_u64().unwrap_or(0) as u32;
                let start_char = range_val["start"]["character"].as_u64().unwrap_or(0) as u32;
                let end_line = range_val["end"]["line"].as_u64().unwrap_or(u32::MAX as u64) as u32;
                let end_char =
                    range_val["end"]["character"].as_u64().unwrap_or(u32::MAX as u64) as u32;
                Some(crate::positions::Range::new(start_line, start_char, end_line, end_char))
            } else {
                None
            };

            let documents = self.documents.lock().unwrap();
            let doc = self.get_document(&documents, uri).ok_or_else(|| JsonRpcError {
                code: ERR_INVALID_REQUEST,
                message: format!("Document not open: {}", uri),
                data: None,
            })?;
            if let Some(ref ast) = doc.ast {
                let mut hints = Vec::new();
                hints.extend(crate::inlay_hints::parameter_hints(
                    ast,
                    &|off| self.offset_to_pos16(doc, off),
                    range,
                ));
                hints.extend(crate::inlay_hints::trivial_type_hints(
                    ast,
                    &|off| self.offset_to_pos16(doc, off),
                    range,
                ));
                return Ok(Some(json!(hints)));
            }
        }
        Ok(Some(json!([])))
    }

    /// Handle textDocument/documentLink request
    fn handle_document_links(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        if let Some(p) = params {
            let uri = p["textDocument"]["uri"].as_str().ok_or_else(|| JsonRpcError {
                code: ERR_INVALID_PARAMS,
                message: "Missing textDocument.uri".into(),
                data: None,
            })?;
            let documents = self.documents.lock().unwrap();
            let doc = self.get_document(&documents, uri).ok_or_else(|| JsonRpcError {
                code: ERR_INVALID_REQUEST,
                message: format!("Document not open: {}", uri),
                data: None,
            })?;

            // Get workspace roots from initialization params
            let roots = self.workspace_roots();
            let links = crate::document_links::compute_links(uri, &doc.content, &roots);
            Ok(Some(json!(links)))
        } else {
            Ok(Some(json!([])))
        }
    }

    /// Handle textDocument/selectionRange request
    fn handle_selection_range(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        if let Some(p) = params {
            let uri = p["textDocument"]["uri"].as_str().ok_or_else(|| JsonRpcError {
                code: ERR_INVALID_PARAMS,
                message: "Missing textDocument.uri".into(),
                data: None,
            })?;
            let positions = p["positions"].as_array().ok_or_else(|| JsonRpcError {
                code: ERR_INVALID_PARAMS,
                message: "Missing positions array".into(),
                data: None,
            })?;

            let documents = self.documents.lock().unwrap();
            let doc = self.get_document(&documents, uri).ok_or_else(|| JsonRpcError {
                code: ERR_INVALID_REQUEST,
                message: format!("Document not open: {}", uri),
                data: None,
            })?;

            let mut out = Vec::new();
            if let Some(ref ast) = doc.ast {
                // Build parent map if not cached
                let parent_map = crate::selection_range::build_parent_map(ast);

                for pos in positions {
                    let line = pos["line"].as_u64().unwrap_or(0) as u32;
                    let col = pos["character"].as_u64().unwrap_or(0) as u32;
                    let off = self.pos16_to_offset(doc, line, col);
                    let chain =
                        crate::selection_range::selection_chain(ast, &parent_map, off, &|o| {
                            self.offset_to_pos16(doc, o)
                        });
                    out.push(chain);
                }
            }
            Ok(Some(json!(out)))
        } else {
            Ok(Some(json!([])))
        }
    }

    /// Handle textDocument/onTypeFormatting request
    fn handle_on_type_formatting(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(p) = params {
            let uri = p["textDocument"]["uri"].as_str().ok_or_else(|| JsonRpcError {
                code: ERR_INVALID_PARAMS,
                message: "Missing textDocument.uri".into(),
                data: None,
            })?;
            let ch = p["ch"].as_str().and_then(|s| s.chars().next()).unwrap_or('\n');
            let pos = &p["position"];
            let line = pos["line"].as_u64().unwrap_or(0) as u32;
            let col = pos["character"].as_u64().unwrap_or(0) as u32;

            let documents = self.documents.lock().unwrap();
            let doc = self.get_document(&documents, uri).ok_or_else(|| JsonRpcError {
                code: ERR_INVALID_REQUEST,
                message: format!("Document not open: {}", uri),
                data: None,
            })?;

            if let Some(edits) =
                crate::on_type_formatting::compute_on_type_edit(&doc.content, line, col, ch)
            {
                return Ok(Some(json!(edits)));
            }
        }
        Ok(Some(json!([])))
    }

    /// Get workspace roots from initialization
    fn workspace_roots(&self) -> Vec<url::Url> {
        // In a real implementation, store these from initialize params
        // For now, return empty vec
        vec![]
    }

    /// Handle textDocument/documentSymbol request
    fn handle_document_symbol(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");

            let documents = self.documents.lock().unwrap();
            if let Some(doc) = self.get_document(&documents, uri) {
                if let Some(ref ast) = doc.ast {
                    // Extract symbols from AST
                    let extractor = crate::symbol::SymbolExtractor::new_with_source(&doc.content);
                    let symbol_table = extractor.extract(ast);

                    // Convert to DocumentSymbol format
                    let mut document_symbols = Vec::new();

                    // Group symbols by scope and kind
                    let mut symbols_by_scope: std::collections::HashMap<
                        crate::symbol::ScopeId,
                        Vec<crate::symbol::Symbol>,
                    > = std::collections::HashMap::new();
                    for symbols in symbol_table.symbols.values() {
                        for symbol in symbols {
                            symbols_by_scope
                                .entry(symbol.scope_id)
                                .or_default()
                                .push(symbol.clone());
                        }
                    }

                    // Build hierarchical structure starting from global scope
                    let empty_vec = Vec::new();
                    let global_symbols = symbols_by_scope.get(&0).unwrap_or(&empty_vec);

                    for symbol in global_symbols {
                        let (start_line, start_char) =
                            self.offset_to_pos16(doc, symbol.location.start);
                        let (end_line, end_char) = self.offset_to_pos16(doc, symbol.location.end);

                        // Map symbol kind to LSP SymbolKind
                        let symbol_kind = match symbol.kind {
                            crate::symbol::SymbolKind::Package => 4,         // Module
                            crate::symbol::SymbolKind::Subroutine => 12,     // Function
                            crate::symbol::SymbolKind::ScalarVariable => 13, // Variable
                            crate::symbol::SymbolKind::ArrayVariable => 18,  // Array
                            crate::symbol::SymbolKind::HashVariable => 19, // Object (closest match)
                            crate::symbol::SymbolKind::Constant => 14,     // Constant
                            crate::symbol::SymbolKind::Label => 16,        // String (closest match)
                            crate::symbol::SymbolKind::Format => 12,       // Function
                        };

                        // Create display name with sigil if applicable
                        let display_name = if let Some(sigil) = symbol.kind.sigil() {
                            format!("{}{}", sigil, symbol.name)
                        } else {
                            symbol.name.clone()
                        };

                        // Find child symbols for this scope (if it's a package or subroutine)
                        let mut children = Vec::new();
                        if symbol.kind == crate::symbol::SymbolKind::Package
                            || symbol.kind == crate::symbol::SymbolKind::Subroutine
                        {
                            // Find scope ID for this symbol
                            for (scope_id, scope) in &symbol_table.scopes {
                                if scope.location.start == symbol.location.start {
                                    // Get symbols in this scope
                                    if let Some(child_symbols) = symbols_by_scope.get(scope_id) {
                                        for child in child_symbols {
                                            let (child_start_line, child_start_char) =
                                                self.offset_to_pos16(doc, child.location.start);
                                            let (child_end_line, child_end_char) =
                                                self.offset_to_pos16(doc, child.location.end);

                                            let child_kind = match child.kind {
                                                crate::symbol::SymbolKind::Package => 4,
                                                crate::symbol::SymbolKind::Subroutine => 12,
                                                crate::symbol::SymbolKind::ScalarVariable => 13,
                                                crate::symbol::SymbolKind::ArrayVariable => 18,
                                                crate::symbol::SymbolKind::HashVariable => 19,
                                                crate::symbol::SymbolKind::Constant => 14,
                                                crate::symbol::SymbolKind::Label => 16,
                                                crate::symbol::SymbolKind::Format => 12,
                                            };

                                            let child_display_name =
                                                if let Some(sigil) = child.kind.sigil() {
                                                    format!("{}{}", sigil, child.name)
                                                } else {
                                                    child.name.clone()
                                                };

                                            children.push(json!({
                                                "name": child_display_name,
                                                "detail": child.declaration.as_deref().unwrap_or(""),
                                                "kind": child_kind,
                                                "range": {
                                                    "start": { "line": child_start_line, "character": child_start_char },
                                                    "end": { "line": child_end_line, "character": child_end_char }
                                                },
                                                "selectionRange": {
                                                    "start": { "line": child_start_line, "character": child_start_char },
                                                    "end": { "line": child_end_line, "character": child_end_char }
                                                },
                                                "children": []
                                            }));
                                        }
                                    }
                                    break;
                                }
                            }
                        }

                        let symbol_info = json!({
                            "name": display_name,
                            "detail": symbol.declaration.as_deref().unwrap_or(""),
                            "kind": symbol_kind,
                            "range": {
                                "start": { "line": start_line, "character": start_char },
                                "end": { "line": end_line, "character": end_char }
                            },
                            "selectionRange": {
                                "start": { "line": start_line, "character": start_char },
                                "end": { "line": end_line, "character": end_char }
                            },
                            "children": children
                        });

                        document_symbols.push(symbol_info);
                    }

                    return Ok(Some(json!(document_symbols)));
                } else {
                    // Fallback: Extract symbols via regex when parse fails
                    eprintln!("Using fallback symbol extraction for {}", uri);
                    let symbols = self.extract_symbols_fallback(&doc.content);
                    eprintln!("Returning {} fallback symbols", symbols.len());
                    return Ok(Some(json!(symbols)));
                }
            }
        }

        Ok(Some(json!([])))
    }

    /// Handle textDocument/foldingRange request
    fn handle_folding_range(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");

            let documents = self.documents.lock().unwrap();
            if let Some(doc) = self.get_document(&documents, uri) {
                let mut lsp_ranges = Vec::new();

                // Add text-based data section folding
                if let Some(marker_offset) = crate::util::find_data_marker_byte_lexed(&doc.content)
                {
                    let marker_line = self.offset_to_line(&doc.content, marker_offset);
                    let total_lines = doc.content.lines().count();

                    // Add fold for data section body if it exists
                    if marker_line + 1 < total_lines {
                        lsp_ranges.push(json!({
                            "startLine": marker_line + 1,
                            "endLine": total_lines - 1,
                            "kind": "comment"
                        }));
                    }
                }

                // Add heredoc folding ranges from lexer
                let heredoc_ranges =
                    crate::folding::FoldingRangeExtractor::extract_heredoc_ranges(&doc.content);
                for range in heredoc_ranges {
                    // Use saturating_sub to ensure we're inside the body
                    let (start_line, _) = self.offset_to_pos16(doc, range.start_offset);
                    let (end_line, _) =
                        self.offset_to_pos16(doc, range.end_offset.saturating_sub(1));

                    if start_line <= end_line {
                        lsp_ranges.push(json!({
                            "startLine": start_line,
                            "endLine": end_line,
                            "kind": "region"
                        }));
                    }
                }

                if let Some(ref ast) = doc.ast {
                    // Extract folding ranges from AST
                    let mut extractor = crate::folding::FoldingRangeExtractor::new();
                    let ranges = extractor.extract(ast);

                    // Convert to LSP JSON format with proper line offsets
                    for range in ranges {
                        // Calculate actual line numbers from document content
                        let start_line = self.offset_to_line(&doc.content, range.start_offset);
                        let end_line = self.offset_to_line(&doc.content, range.end_offset);

                        if end_line > start_line {
                            let mut lsp_range = json!({
                                "startLine": start_line,
                                "endLine": end_line - 1,  // LSP folding ranges are inclusive
                            });

                            if let Some(ref kind) = range.kind {
                                lsp_range["kind"] = match kind {
                                    crate::folding::FoldingRangeKind::Comment => json!("comment"),
                                    crate::folding::FoldingRangeKind::Imports => json!("imports"),
                                    crate::folding::FoldingRangeKind::Region => json!("region"),
                                };
                            }

                            lsp_ranges.push(lsp_range);
                        }
                    }

                    // If no ranges from AST, try fallback
                    if lsp_ranges.is_empty() {
                        return Ok(Some(json!(self.extract_folding_fallback(&doc.content))));
                    }

                    return Ok(Some(json!(lsp_ranges)));
                } else {
                    // No AST, use fallback
                    return Ok(Some(json!(self.extract_folding_fallback(&doc.content))));
                }
            }
        }

        Ok(Some(json!([])))
    }

    /// Helper function to convert offset to line number
    fn offset_to_line(&self, content: &str, offset: usize) -> usize {
        content[..offset.min(content.len())].chars().filter(|&c| c == '\n').count()
    }

    /// Fallback folding extraction using text-based analysis
    fn extract_folding_fallback(&self, content: &str) -> Vec<Value> {
        let mut ranges = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut brace_stack: Vec<usize> = Vec::new();
        let mut sub_start: Option<usize> = None;
        let mut pod_start: Option<usize> = None;

        for (line_num, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Handle POD sections
            if trimmed.starts_with("=") {
                if trimmed == "=cut" {
                    if let Some(start) = pod_start {
                        if line_num > start {
                            ranges.push(json!({
                                "startLine": start,
                                "endLine": line_num,
                                "kind": "comment"
                            }));
                        }
                        pod_start = None;
                    }
                } else if pod_start.is_none() {
                    pod_start = Some(line_num);
                }
                continue;
            }

            // Skip if we're in POD
            if pod_start.is_some() {
                continue;
            }

            // Handle subroutines
            if trimmed.starts_with("sub ") {
                // If we had a previous sub, close it
                if let Some(start) = sub_start {
                    if line_num > start + 1 {
                        ranges.push(json!({
                            "startLine": start,
                            "endLine": line_num - 1
                        }));
                    }
                }
                sub_start = Some(line_num);
            }

            // Count braces (simple approach, doesn't handle strings/comments perfectly)
            let mut in_string = false;
            let mut escape_next = false;
            let mut prev_char = ' ';

            for ch in line.chars() {
                if escape_next {
                    escape_next = false;
                    prev_char = ch;
                    continue;
                }

                if ch == '\\' {
                    escape_next = true;
                    prev_char = ch;
                    continue;
                }

                // Simple string detection (not perfect but good enough)
                if (ch == '"' || ch == '\'') && (!in_string || prev_char != '\\') {
                    in_string = !in_string;
                }

                if !in_string {
                    if ch == '{' {
                        brace_stack.push(line_num);
                    } else if ch == '}' {
                        if let Some(start_line) = brace_stack.pop() {
                            // Only create fold if it spans multiple lines
                            if line_num > start_line {
                                ranges.push(json!({
                                    "startLine": start_line,
                                    "endLine": line_num
                                }));
                            }
                        }
                    }
                }

                prev_char = ch;
            }
        }

        // Close any remaining sub
        if let Some(start) = sub_start {
            if lines.len() > start + 1 {
                ranges.push(json!({
                    "startLine": start,
                    "endLine": lines.len() - 1
                }));
            }
        }

        ranges
    }

    /// Fallback symbol extraction using regex when parser fails
    fn extract_symbols_fallback(&self, content: &str) -> Vec<Value> {
        let mut symbols = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        // Regex for subroutine declarations
        let sub_regex = regex::Regex::new(r"^\s*sub\s+([a-zA-Z_]\w*)\b").unwrap();
        let package_regex = regex::Regex::new(r"^\s*package\s+([a-zA-Z_][\w:]*)\b").unwrap();

        for (line_num, line) in lines.iter().enumerate() {
            // Check for subroutines
            if let Some(captures) = sub_regex.captures(line) {
                if let Some(name_match) = captures.get(1) {
                    let name = name_match.as_str().to_string();
                    let start_char = name_match.start();
                    let end_char = name_match.end();

                    symbols.push(json!({
                        "name": name,
                        "kind": 12, // Function
                        "range": {
                            "start": { "line": line_num, "character": 0 },
                            "end": { "line": line_num, "character": line.len() }
                        },
                        "selectionRange": {
                            "start": { "line": line_num, "character": start_char },
                            "end": { "line": line_num, "character": end_char }
                        }
                    }));
                }
            }

            // Check for packages
            if let Some(captures) = package_regex.captures(line) {
                if let Some(name_match) = captures.get(1) {
                    let name = name_match.as_str().to_string();
                    let start_char = name_match.start();
                    let end_char = name_match.end();

                    symbols.push(json!({
                        "name": name,
                        "kind": 4, // Module
                        "range": {
                            "start": { "line": line_num, "character": 0 },
                            "end": { "line": line_num, "character": line.len() }
                        },
                        "selectionRange": {
                            "start": { "line": line_num, "character": start_char },
                            "end": { "line": line_num, "character": end_char }
                        }
                    }));
                }
            }
        }

        symbols
    }

    /// Validate if a string is a valid Perl identifier
    fn is_valid_identifier(&self, name: &str) -> bool {
        if name.is_empty() {
            return false;
        }

        let chars: Vec<char> = name.chars().collect();

        // First character must be letter or underscore
        if !chars[0].is_alphabetic() && chars[0] != '_' {
            return false;
        }

        // Rest must be alphanumeric or underscore
        for ch in &chars[1..] {
            if !ch.is_alphanumeric() && *ch != '_' {
                return false;
            }
        }

        true
    }

    /// Get token at position (simple implementation)
    fn get_token_at_position(&self, content: &str, offset: usize) -> String {
        let chars: Vec<char> = content.chars().collect();
        if offset >= chars.len() {
            return String::new();
        }

        // Find word boundaries
        let mut start = offset;
        while start > 0
            && (chars[start - 1].is_alphanumeric()
                || chars[start - 1] == '_'
                || chars[start - 1] == '$'
                || chars[start - 1] == '@'
                || chars[start - 1] == '%')
        {
            start -= 1;
        }

        let mut end = offset;
        while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
            end += 1;
        }

        chars[start..end].iter().collect()
    }

    /// Get the bounds of the token at the given position
    fn get_token_bounds(&self, content: &str, offset: usize) -> (usize, usize) {
        let chars: Vec<char> = content.chars().collect();
        if offset >= chars.len() {
            return (offset, offset);
        }

        // Find word boundaries
        let mut start = offset;
        while start > 0
            && (chars[start - 1].is_alphanumeric()
                || chars[start - 1] == '_'
                || chars[start - 1] == '$'
                || chars[start - 1] == '@'
                || chars[start - 1] == '%')
        {
            start -= 1;
        }

        let mut end = offset;
        while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
            end += 1;
        }

        (start, end)
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
        doc.line_starts.position_to_offset(&doc.content, line, ch)
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

    /// Lexical completion fallback for when AST is unavailable
    fn lexical_complete(
        &self,
        content: &str,
        offset: usize,
    ) -> Vec<crate::completion::CompletionItem> {
        let mut completions = Vec::new();

        // Get the prefix we're completing
        let text_before = &content[..offset.min(content.len())];
        let prefix = text_before
            .chars()
            .rev()
            .take_while(|&c| c.is_alphanumeric() || c == '_')
            .collect::<String>()
            .chars()
            .rev()
            .collect::<String>();

        // Check what sigil we're after (if any)
        let sigil = text_before.chars().rev().find(|&c| !(c.is_alphanumeric() || c == '_'));

        // Basic keywords that match the prefix
        let keywords = [
            "my", "our", "local", "state", "sub", "package", "use", "require", "if", "elsif",
            "else", "unless", "while", "until", "for", "foreach", "given", "when", "default",
            "return", "last", "next", "redo", "goto", "die", "warn", "print", "say", "open",
            "close", "read", "write", "push", "pop", "shift", "unshift", "splice", "grep", "map",
            "sort",
        ];

        match sigil {
            Some('$') => {
                // Scalar variables - suggest common ones
                if "_".starts_with(&prefix) || prefix.is_empty() {
                    completions.push(crate::completion::CompletionItem {
                        label: "_".to_string(),
                        kind: CompletionItemKind::Variable,
                        detail: Some("Default variable".to_string()),
                        documentation: None,
                        insert_text: Some("_".to_string()),
                        additional_edits: vec![],
                        sort_text: None,
                        filter_text: None,
                        text_edit_range: None,
                    });
                }
            }
            Some('@') => {
                // Array variables - suggest common ones
                if "ARGV".starts_with(&prefix) || prefix.is_empty() {
                    completions.push(crate::completion::CompletionItem {
                        label: "ARGV".to_string(),
                        kind: CompletionItemKind::Variable,
                        detail: Some("Command line arguments".to_string()),
                        documentation: None,
                        insert_text: Some("ARGV".to_string()),
                        additional_edits: vec![],
                        sort_text: None,
                        filter_text: None,
                        text_edit_range: None,
                    });
                }
                if "_".starts_with(&prefix) || prefix.is_empty() {
                    completions.push(crate::completion::CompletionItem {
                        label: "_".to_string(),
                        kind: CompletionItemKind::Variable,
                        detail: Some("Function arguments".to_string()),
                        documentation: None,
                        insert_text: Some("_".to_string()),
                        additional_edits: vec![],
                        sort_text: None,
                        filter_text: None,
                        text_edit_range: None,
                    });
                }
            }
            Some('%') => {
                // Hash variables - suggest common ones
                if "ENV".starts_with(&prefix) || prefix.is_empty() {
                    completions.push(crate::completion::CompletionItem {
                        label: "ENV".to_string(),
                        kind: CompletionItemKind::Variable,
                        detail: Some("Environment variables".to_string()),
                        documentation: None,
                        insert_text: Some("ENV".to_string()),
                        additional_edits: vec![],
                        sort_text: None,
                        filter_text: None,
                        text_edit_range: None,
                    });
                }
            }
            _ => {
                // No sigil - suggest keywords
                for kw in &keywords {
                    if kw.starts_with(&prefix) {
                        completions.push(crate::completion::CompletionItem {
                            label: kw.to_string(),
                            kind: CompletionItemKind::Keyword,
                            detail: None,
                            documentation: None,
                            insert_text: Some(kw.to_string()),
                            additional_edits: vec![],
                            sort_text: None,
                            filter_text: None,
                            text_edit_range: None,
                        });
                    }
                }
            }
        }

        completions
    }

    /// Helper to create a ContentModified error response
    fn content_modified() -> JsonRpcError {
        JsonRpcError {
            code: ERR_CONTENT_MODIFIED,
            message: "Document changed before request executed".to_string(),
            data: None,
        }
    }

    /// Ensure the request version matches the current document version
    fn ensure_latest(&self, uri: &str, req_version: Option<i32>) -> Result<(), JsonRpcError> {
        if let Some(v) = req_version {
            let documents = self.documents.lock().unwrap();
            if let Some(doc) = self.get_document(&documents, uri) {
                if v < doc._version {
                    return Err(Self::content_modified());
                }
            }
        }
        Ok(())
    }

    /// Offset to position conversion using cached line starts for O(log n) performance
    #[inline]
    fn offset_to_pos16(&self, doc: &DocumentState, offset: usize) -> (u32, u32) {
        doc.line_starts.offset_to_position(&doc.content, offset)
    }

    /// Handle textDocument/formatting request
    fn handle_formatting(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");

            // Reject stale requests
            let req_version = params["textDocument"]["version"].as_i64().map(|n| n as i32);
            self.ensure_latest(uri, req_version)?;

            let options: FormattingOptions = serde_json::from_value(params["options"].clone())
                .unwrap_or(FormattingOptions {
                    tab_size: 4,
                    insert_spaces: true,
                    trim_trailing_whitespace: None,
                    insert_final_newline: None,
                    trim_final_newlines: None,
                });

            eprintln!("Formatting document: {}", uri);

            let documents = self.documents.lock().unwrap();
            if let Some(doc) = self.get_document(&documents, uri) {
                let formatter = CodeFormatter::new();
                match formatter.format_document(&doc.content, &options) {
                    Ok(edits) => {
                        let lsp_edits: Vec<Value> = edits
                            .into_iter()
                            .map(|edit| {
                                json!({
                                    "range": {
                                        "start": {
                                            "line": edit.range.start.line,
                                            "character": edit.range.start.character,
                                        },
                                        "end": {
                                            "line": edit.range.end.line,
                                            "character": edit.range.end.character,
                                        },
                                    },
                                    "newText": edit.new_text,
                                })
                            })
                            .collect();

                        return Ok(Some(json!(lsp_edits)));
                    }
                    Err(e) => {
                        eprintln!("Formatting error: {}", e);
                        return Err(JsonRpcError {
                            code: -32603,
                            message: format!("Formatting failed: {}", e),
                            data: None,
                        });
                    }
                }
            }
        }

        Ok(Some(json!([])))
    }

    /// Handle textDocument/rangeFormatting request
    fn handle_range_formatting(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let options: FormattingOptions = serde_json::from_value(params["options"].clone())
                .unwrap_or(FormattingOptions {
                    tab_size: 4,
                    insert_spaces: true,
                    trim_trailing_whitespace: None,
                    insert_final_newline: None,
                    trim_final_newlines: None,
                });

            let range = if let Some(range_value) = params.get("range") {
                crate::formatting::Range {
                    start: crate::formatting::Position {
                        line: range_value["start"]["line"].as_u64().unwrap_or(0) as u32,
                        character: range_value["start"]["character"].as_u64().unwrap_or(0) as u32,
                    },
                    end: crate::formatting::Position {
                        line: range_value["end"]["line"].as_u64().unwrap_or(0) as u32,
                        character: range_value["end"]["character"].as_u64().unwrap_or(0) as u32,
                    },
                }
            } else {
                return Ok(Some(json!([])));
            };

            eprintln!("Formatting range in document: {}", uri);

            let documents = self.documents.lock().unwrap();
            if let Some(doc) = self.get_document(&documents, uri) {
                let formatter = CodeFormatter::new();
                match formatter.format_range(&doc.content, &range, &options) {
                    Ok(edits) => {
                        let lsp_edits: Vec<Value> = edits
                            .into_iter()
                            .map(|edit| {
                                json!({
                                    "range": {
                                        "start": {
                                            "line": edit.range.start.line,
                                            "character": edit.range.start.character,
                                        },
                                        "end": {
                                            "line": edit.range.end.line,
                                            "character": edit.range.end.character,
                                        },
                                    },
                                    "newText": edit.new_text,
                                })
                            })
                            .collect();

                        return Ok(Some(json!(lsp_edits)));
                    }
                    Err(e) => {
                        eprintln!("Range formatting error: {}", e);
                        return Err(JsonRpcError {
                            code: -32603,
                            message: format!("Range formatting failed: {}", e),
                            data: None,
                        });
                    }
                }
            }
        }

        Ok(Some(json!([])))
    }

    /// Handle workspace/symbol request v2 - uses workspace index
    #[cfg(feature = "workspace")]
    fn handle_workspace_symbols_v2(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        let query =
            params.as_ref().and_then(|p| p.get("query")).and_then(|q| q.as_str()).unwrap_or("");

        eprintln!("Workspace symbol search v2: '{}'", query);

        // Use workspace index if available
        if let Some(ref workspace_index) = self.workspace_index {
            let symbols = workspace_index.search_symbols(query);

            // Convert to LSP format with yielding
            let lsp_symbols: Vec<LspWorkspaceSymbol> = symbols
                .iter()
                .enumerate()
                .map(|(i, sym)| {
                    // Cooperative yield every 64 symbols
                    if i & 0x3f == 0 {
                        std::thread::yield_now();
                    }
                    sym.into()
                })
                .collect();

            eprintln!("Found {} symbols from index", lsp_symbols.len());
            return Ok(Some(json!(lsp_symbols)));
        }

        // Fallback to document-based search
        let mut all_symbols = Vec::new();

        // Collect document snapshots without holding lock
        let docs_snapshot: Vec<(String, DocumentState)> = {
            let documents = self.documents.lock().unwrap();
            documents.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        };

        for (i, (uri, doc)) in docs_snapshot.iter().enumerate() {
            // Cooperative yield every 8 documents
            if i & 0x7 == 0 {
                std::thread::yield_now();
            }

            if let Some(ref ast) = doc.ast {
                let doc_symbols = self.extract_document_symbols(ast, &doc.content, uri);
                let query_lower = query.to_lowercase();

                for sym in doc_symbols {
                    if sym.name.to_lowercase().contains(&query_lower) {
                        all_symbols.push(sym);
                    }
                }
            }
        }

        eprintln!("Found {} symbols from documents", all_symbols.len());
        Ok(Some(json!(all_symbols)))
    }

    /// Handle workspace/symbol request (legacy implementation)
    fn handle_workspace_symbols(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        let query =
            params.as_ref().and_then(|p| p.get("query")).and_then(|q| q.as_str()).unwrap_or("");

        eprintln!("Workspace symbol search: '{}'", query);

        // First, get symbols from currently open documents (synchronous)
        #[cfg(feature = "workspace")]
        let mut all_symbols = Vec::new();

        #[cfg(feature = "workspace")]
        {
            let documents = self.documents.lock().unwrap();
            for (uri, doc) in documents.iter() {
                if let Some(ref ast) = doc.ast {
                    // Extract symbols from this document
                    let doc_symbols = self.extract_document_symbols(ast, &doc.content, uri);

                    // Filter by query
                    let query_lower = query.to_lowercase();
                    for sym in doc_symbols {
                        if sym.name.to_lowercase().contains(&query_lower) {
                            all_symbols.push(sym);
                        }
                    }
                }
            }
        }

        #[cfg(not(feature = "workspace"))]
        let all_symbols = {
            // Simple synchronous extraction without workspace feature
            let mut symbols = Vec::new();
            let documents = self.documents.lock().unwrap();
            for (uri, doc) in documents.iter() {
                if let Some(ref ast) = doc.ast {
                    // Extract symbols using document symbol provider
                    self.extract_simple_symbols(ast, &doc.content, uri, query, &mut symbols);
                }
            }
            symbols
        };

        // Also use workspace index if available
        #[cfg(feature = "workspace")]
        {
            let index_symbols = if let Some(ref workspace_index) = self.workspace_index {
                workspace_index.find_symbols(query)
            } else {
                Vec::new()
            };

            // Convert workspace index symbols to typed LSP WorkspaceSymbol structs
            use std::collections::HashSet;
            let mut seen = HashSet::new();

            // Track what we already have from open docs
            for sym in &all_symbols {
                seen.insert((
                    sym.location.uri.clone(),
                    sym.location.range.start.line,
                    sym.location.range.start.character,
                    sym.name.clone(),
                    sym.kind,
                ));
            }

            // Add index symbols that aren't already in the results
            let additional_symbols: Vec<WorkspaceSymbol> = index_symbols
                .into_iter()
                .filter(|sym| {
                    // Deduplicate by (uri, start position, name, kind)
                    seen.insert((
                        sym.uri.clone(),
                        sym.range.start.line,
                        sym.range.start.character,
                        sym.name.clone(),
                        sym.kind.to_lsp_kind(),
                    ))
                })
                .collect();

            // Convert to LSP DTOs and add to results
            for sym in additional_symbols {
                all_symbols.push(LspWorkspaceSymbol {
                    name: sym.name,
                    kind: sym.kind.to_lsp_kind(),
                    location: crate::workspace_index::LspLocation {
                        uri: sym.uri,
                        range: crate::workspace_index::LspRange {
                            start: crate::workspace_index::LspPosition {
                                line: sym.range.start.line,
                                character: sym.range.start.character,
                            },
                            end: crate::workspace_index::LspPosition {
                                line: sym.range.end.line,
                                character: sym.range.end.character,
                            },
                        },
                    },
                    container_name: sym.container_name.map(|s| norm_pkg(&s).into_owned()),
                });
            }
        }

        eprintln!("Found {} symbols total", all_symbols.len());

        // Convert to JSON for LSP response
        let result = serde_json::to_value(&all_symbols).unwrap_or_else(|_| json!([]));

        Ok(Some(result))
    }

    /// Handle textDocument/codeLens request
    fn handle_code_lens(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        // Gate unadvertised feature
        if !self.advertised_features.lock().unwrap().code_lens {
            return Err(crate::lsp_errors::method_not_advertised());
        }

        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");

            eprintln!("Getting code lenses for: {}", uri);

            let documents = self.documents.lock().unwrap();
            if let Some(doc) = self.get_document(&documents, uri) {
                if let Some(ref ast) = doc.ast {
                    let provider = CodeLensProvider::new(doc.content.clone());
                    let mut lenses = provider.extract(ast);

                    // Add shebang lens if applicable
                    if let Some(shebang_lens) = get_shebang_lens(&doc.content) {
                        lenses.insert(0, shebang_lens);
                    }

                    eprintln!("Found {} code lenses", lenses.len());

                    return Ok(Some(json!(lenses)));
                }
            }
        }

        Ok(Some(json!([])))
    }

    /// Handle codeLens/resolve request
    fn handle_code_lens_resolve(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            // Parse the code lens
            if let Ok(lens) =
                serde_json::from_value::<crate::code_lens_provider::CodeLens>(params.clone())
            {
                // Extract the symbol name and kind from the lens data
                let symbol_name = lens
                    .data
                    .as_ref()
                    .and_then(|d| d.get("name"))
                    .and_then(|n| n.as_str())
                    .unwrap_or("");

                let symbol_kind = lens
                    .data
                    .as_ref()
                    .and_then(|d| d.get("kind"))
                    .and_then(|k| k.as_str())
                    .unwrap_or("unknown");

                // Get the document URI from the range (we need to track this better in the future)
                // For now, count references across all documents
                let mut total_references = 0;

                let documents = self.documents.lock().unwrap();
                for (_uri, doc) in documents.iter() {
                    if let Some(ref ast) = doc.ast {
                        total_references += self.count_references(ast, symbol_name, symbol_kind);
                    }
                }

                let resolved = resolve_code_lens(lens, total_references);
                return Ok(Some(json!(resolved)));
            }
        }

        Err(JsonRpcError { code: -32602, message: "Invalid parameters".to_string(), data: None })
    }

    /// Handle textDocument/inlineCompletion request
    fn handle_inline_completion(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        use crate::inline_completions::InlineCompletionProvider;

        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let position = &params["position"];
            let line = position["line"].as_u64().unwrap_or(0) as u32;
            let character = position["character"].as_u64().unwrap_or(0) as u32;

            let documents = self.documents.lock().unwrap();
            if let Some(doc) = self.get_document(&documents, uri) {
                let provider = InlineCompletionProvider::new();
                let completions = provider.get_inline_completions(&doc.content, line, character);
                return Ok(Some(serde_json::to_value(completions).unwrap_or(Value::Null)));
            }
        }

        Ok(Some(json!({
            "items": []
        })))
    }

    /// Handle textDocument/inlineValue request
    fn handle_inline_value(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let range = &params["range"];
            let _context = &params["context"]; // Debug context (stopped at breakpoint, etc)

            let documents = self.documents.lock().unwrap();
            if let Some(doc) = self.get_document(&documents, uri) {
                // Extract visible scalar variables in the range
                let start_line = range["start"]["line"].as_u64().unwrap_or(0) as u32;
                let end_line = range["end"]["line"].as_u64().unwrap_or(0) as u32;

                let mut inline_values = Vec::new();

                // Simple implementation: find scalar variables in the visible range
                let lines: Vec<&str> = doc.content.lines().collect();
                // Move regex construction outside loop
                let re = regex::Regex::new(r"\$([a-zA-Z_][a-zA-Z0-9_]*)").unwrap();
                for line_num in start_line..=end_line.min((lines.len() - 1) as u32) {
                    let line_text = lines[line_num as usize];

                    // Find scalar variables using regex
                    for cap in re.captures_iter(line_text) {
                        if let Some(m) = cap.get(0) {
                            let var_text = m.as_str();
                            let col = m.start();

                            // Create inline value text hint (showing the variable name as placeholder)
                            inline_values.push(json!({
                                "range": {
                                    "start": { "line": line_num, "character": col as u32 },
                                    "end": { "line": line_num, "character": (col + var_text.len()) as u32 }
                                },
                                "text": format!("{} = ?", var_text)
                            }));
                        }
                    }
                }

                return Ok(Some(json!(inline_values)));
            }
        }

        Ok(Some(json!([])))
    }

    /// Handle textDocument/moniker request
    fn handle_moniker(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let position = &params["position"];
            let line = position["line"].as_u64().unwrap_or(0) as u32;
            let character = position["character"].as_u64().unwrap_or(0) as u32;

            let documents = self.documents.lock().unwrap();
            if let Some(doc) = self.get_document(&documents, uri) {
                if let Some(ref ast) = doc.ast {
                    let offset = self.pos16_to_offset(doc, line, character);

                    // Find the symbol at the cursor position
                    let current_pkg = crate::declaration::current_package_at(ast, offset);
                    if let Some(key) =
                        crate::declaration::symbol_at_cursor(ast, offset, current_pkg)
                    {
                        // Generate a stable moniker for the symbol
                        let identifier = format!("{}::{}", key.pkg, key.name).replace("::", ".");
                        let moniker = json!({
                            "scheme": "perl",
                            "identifier": identifier,
                            "unique": "project",
                            "kind": "export"
                        });

                        return Ok(Some(json!([moniker])));
                    }
                }
            }
        }

        Ok(Some(json!([])))
    }

    /// Handle textDocument/documentColor request
    fn handle_document_color(&self, _params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        Err(JsonRpcError { code: -32601, message: "Method not found".into(), data: None })
    }

    /// Handle textDocument/colorPresentation request
    fn handle_color_presentation(
        &self,
        _params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        Err(JsonRpcError { code: -32601, message: "Method not found".into(), data: None })
    }

    /// Handle textDocument/linkedEditingRange request
    fn handle_linked_editing_range(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        // Gate unadvertised feature
        if !self.advertised_features.lock().unwrap().linked_editing {
            return Err(crate::lsp_errors::method_not_advertised());
        }

        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let position = &params["position"];
            let line = position["line"].as_u64().unwrap_or(0) as u32;
            let character = position["character"].as_u64().unwrap_or(0) as u32;

            let documents = self.documents.lock().unwrap();
            if let Some(doc) = self.get_document(&documents, uri) {
                let result =
                    crate::linked_editing::handle_linked_editing(&doc.content, line, character);
                return Ok(Some(serde_json::to_value(result).unwrap_or(Value::Null)));
            }
        }

        Ok(Some(Value::Null))
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
                        container_name: container.map(|s| norm_pkg(s).into_owned()),
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
                    container_name: container.map(|s| norm_pkg(s).into_owned()),
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

    /// Handle semantic tokens full request
    fn handle_semantic_tokens_full(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");

            eprintln!("Getting semantic tokens for: {}", uri);

            let documents = self.documents.lock().unwrap();
            if let Some(doc) = self.get_document(&documents, uri) {
                if let Some(ref ast) = doc.ast {
                    let mut provider = SemanticTokensProvider::new(doc.content.clone());
                    let tokens = provider.extract(ast);
                    let encoded = encode_semantic_tokens(&tokens);

                    eprintln!("Found {} semantic tokens", tokens.len());

                    return Ok(Some(json!({
                        "data": encoded
                    })));
                }
            }
        }

        Ok(Some(json!({
            "data": []
        })))
    }

    /// Handle semantic tokens range request
    fn handle_semantic_tokens_range(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let range = &params["range"];
            let start_line = range["start"]["line"].as_u64().unwrap_or(0) as u32;
            let end_line = range["end"]["line"].as_u64().unwrap_or(u32::MAX as u64) as u32;

            eprintln!(
                "Getting semantic tokens for range: {} (lines {}-{})",
                uri, start_line, end_line
            );

            let documents = self.documents.lock().unwrap();
            if let Some(doc) = self.get_document(&documents, uri) {
                if let Some(ref ast) = doc.ast {
                    let mut provider = SemanticTokensProvider::new(doc.content.clone());
                    let all_tokens = provider.extract(ast);

                    // Filter tokens to the requested range
                    let range_tokens: Vec<_> = all_tokens
                        .into_iter()
                        .filter(|token| token.line >= start_line && token.line <= end_line)
                        .collect();

                    let encoded = encode_semantic_tokens(&range_tokens);

                    eprintln!("Found {} semantic tokens in range", range_tokens.len());

                    return Ok(Some(json!({
                        "data": encoded
                    })));
                }
            }
        }

        Ok(Some(json!({
            "data": []
        })))
    }

    /// Handle prepare call hierarchy request
    fn handle_prepare_call_hierarchy(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        // Gate unadvertised feature
        if !self.advertised_features.lock().unwrap().call_hierarchy {
            return Err(crate::lsp_errors::method_not_advertised());
        }

        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let position = &params["position"];
            let line = position["line"].as_u64().unwrap_or(0) as u32;
            let character = position["character"].as_u64().unwrap_or(0) as u32;

            eprintln!("Preparing call hierarchy at: {} ({}:{})", uri, line, character);

            let documents = self.documents.lock().unwrap();
            if let Some(doc) = self.get_document(&documents, uri) {
                if let Some(ref ast) = doc.ast {
                    let provider = CallHierarchyProvider::new(doc.content.clone(), uri.to_string());
                    if let Some(items) = provider.prepare(ast, line, character) {
                        let json_items: Vec<_> = items.iter().map(|item| item.to_json()).collect();
                        return Ok(Some(json!(json_items)));
                    }
                }
            }
        }

        Ok(Some(json!(null)))
    }

    /// Handle incoming calls request
    fn handle_incoming_calls(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let item = &params["item"];
            let uri = item["uri"].as_str().unwrap_or("");

            eprintln!("Getting incoming calls for: {}", item["name"].as_str().unwrap_or(""));

            let documents = self.documents.lock().unwrap();
            if let Some(doc) = self.get_document(&documents, uri) {
                if let Some(ref ast) = doc.ast {
                    // Reconstruct the CallHierarchyItem from JSON
                    let ch_item = self.json_to_call_hierarchy_item(item)?;

                    let provider = CallHierarchyProvider::new(doc.content.clone(), uri.to_string());
                    let calls = provider.incoming_calls(ast, &ch_item);

                    let json_calls: Vec<_> = calls.iter().map(|call| call.to_json()).collect();
                    return Ok(Some(json!(json_calls)));
                }
            }
        }

        Ok(Some(json!([])))
    }

    /// Handle outgoing calls request
    fn handle_outgoing_calls(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let item = &params["item"];
            let uri = item["uri"].as_str().unwrap_or("");

            eprintln!("Getting outgoing calls for: {}", item["name"].as_str().unwrap_or(""));

            let documents = self.documents.lock().unwrap();
            if let Some(doc) = self.get_document(&documents, uri) {
                if let Some(ref ast) = doc.ast {
                    // Reconstruct the CallHierarchyItem from JSON
                    let ch_item = self.json_to_call_hierarchy_item(item)?;

                    let provider = CallHierarchyProvider::new(doc.content.clone(), uri.to_string());
                    let calls = provider.outgoing_calls(ast, &ch_item);

                    let json_calls: Vec<_> = calls.iter().map(|call| call.to_json()).collect();
                    return Ok(Some(json!(json_calls)));
                }
            }
        }

        Ok(Some(json!([])))
    }

    /// Convert JSON to CallHierarchyItem
    fn json_to_call_hierarchy_item(
        &self,
        json: &Value,
    ) -> Result<crate::call_hierarchy_provider::CallHierarchyItem, JsonRpcError> {
        use crate::call_hierarchy_provider::{CallHierarchyItem, Position, Range};

        let name = json["name"].as_str().unwrap_or("").to_string();
        let kind = match json["kind"].as_u64().unwrap_or(12) {
            6 => "method",
            _ => "function",
        }
        .to_string();
        let uri = json["uri"].as_str().unwrap_or("").to_string();

        let range = Range {
            start: Position {
                line: json["range"]["start"]["line"].as_u64().unwrap_or(0) as u32,
                character: json["range"]["start"]["character"].as_u64().unwrap_or(0) as u32,
            },
            end: Position {
                line: json["range"]["end"]["line"].as_u64().unwrap_or(0) as u32,
                character: json["range"]["end"]["character"].as_u64().unwrap_or(0) as u32,
            },
        };

        let selection_range = Range {
            start: Position {
                line: json["selectionRange"]["start"]["line"].as_u64().unwrap_or(0) as u32,
                character: json["selectionRange"]["start"]["character"].as_u64().unwrap_or(0)
                    as u32,
            },
            end: Position {
                line: json["selectionRange"]["end"]["line"].as_u64().unwrap_or(0) as u32,
                character: json["selectionRange"]["end"]["character"].as_u64().unwrap_or(0) as u32,
            },
        };

        let detail = json["detail"].as_str().map(|s| s.to_string());

        Ok(CallHierarchyItem { name, kind, uri, range, selection_range, detail })
    }

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

    /// Handle inlay hint request
    fn handle_inlay_hint(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let range = &params["range"];

            eprintln!("Getting inlay hints for: {}", uri);

            let documents = self.documents.lock().unwrap();
            if let Some(doc) = self.get_document(&documents, uri) {
                let mut hints = Vec::new();

                if let Some(ref ast) = doc.ast {
                    // Configure inlay hints based on server settings
                    let server_config = self.config.lock().unwrap();
                    let config = InlayHintConfig {
                        parameter_hints: server_config.inlay_hints_parameter_hints,
                        type_hints: server_config.inlay_hints_type_hints,
                        chained_hints: server_config.inlay_hints_chained_hints,
                        max_length: server_config.inlay_hints_max_length,
                    };

                    let provider = InlayHintsProvider::with_config(doc.content.clone(), config);

                    // Extract range if provided
                    let lsp_range = if params.get("range").is_some() {
                        let start_line = range["start"]["line"].as_u64().unwrap_or(0) as u32;
                        let start_char = range["start"]["character"].as_u64().unwrap_or(0) as u32;
                        let end_line =
                            range["end"]["line"].as_u64().unwrap_or(u32::MAX as u64) as u32;
                        let end_char =
                            range["end"]["character"].as_u64().unwrap_or(u32::MAX as u64) as u32;
                        Some(crate::positions::Range::new(
                            start_line, start_char, end_line, end_char,
                        ))
                    } else {
                        None
                    };

                    // Use the extract_range method if range is provided
                    let ast_hints = if let Some(r) = lsp_range {
                        provider.extract_range(ast, r)
                    } else {
                        provider.extract(ast)
                    };

                    for hint in ast_hints {
                        hints.push(hint.to_json());
                    }
                }

                // Named-arg fallback: foo(bar => $x, 'baz' => 2)
                // Respect client-supplied range if present
                let has_range = params.get("range").and_then(|r| r.as_object()).is_some();
                let (_win_s, _win_e, window) = if has_range {
                    let s_line = range["start"]["line"].as_u64().unwrap_or(0) as u32;
                    let s_ch = range["start"]["character"].as_u64().unwrap_or(0) as u32;
                    let e_line = range["end"]["line"].as_u64().unwrap_or(0) as u32;
                    let e_ch = range["end"]["character"].as_u64().unwrap_or(0) as u32;
                    self.slice_in_range(&doc.content, (s_line, s_ch), (e_line, e_ch))
                } else {
                    (0, doc.content.len(), doc.content.as_str())
                };

                // Add named argument hints for => pairs
                use crate::builtin_signatures_phf::{BUILTIN_SIGS, get_param_names};
                use regex::Regex;
                use std::collections::HashSet;
                lazy_static::lazy_static! {
                    static ref RE_CALL: Regex = Regex::new(r"(?m)([A-Za-z_]\w*)\s*\(").unwrap();
                    static ref RE_PAIR: Regex = Regex::new(
                        r#"(?x)
                        (?P<key>
                            [A-Za-z_]\w*                # bareword
                            | '([^'\\]|\\.)*'           # single-quoted
                            | "([^"\\]|\\.)*"           # double-quoted
                        )
                        \s*=>\s*
                        "#
                    ).unwrap();
                }

                for m in RE_PAIR.find_iter(window) {
                    if let Some(caps) = RE_PAIR.captures(m.as_str()) {
                        let mut key =
                            caps.name("key").map(|k| k.as_str()).unwrap_or("").to_string();

                        // Strip quotes if present
                        if (key.starts_with('"') && key.ends_with('"'))
                            || (key.starts_with('\'') && key.ends_with('\''))
                        {
                            key = key[1..key.len().saturating_sub(1)].to_string();
                        }

                        // Calculate position for the hint (at the value start)
                        let val_offset = m.end();
                        let (l, c) = doc.line_starts.offset_to_position(&doc.content, val_offset);

                        // Only add hint if within requested range
                        let start_line = range["start"]["line"].as_u64().unwrap_or(0) as u32;
                        let start_char = range["start"]["character"].as_u64().unwrap_or(0) as u32;
                        let end_line =
                            range["end"]["line"].as_u64().unwrap_or(u32::MAX as u64) as u32;
                        let end_char =
                            range["end"]["character"].as_u64().unwrap_or(u32::MAX as u64) as u32;

                        let pos = crate::positions::Position::new(l, c);
                        let range = crate::positions::Range::new(
                            start_line, start_char, end_line, end_char,
                        );
                        if crate::positions::pos_in_range(pos, range) {
                            hints.push(json!({
                                "position": { "line": l, "character": c },
                                "label": format!("{}:", key),
                                "kind": 2, // Parameter hint
                            }));
                        }
                    }
                }

                // ---- Parameter hints for common built-ins with comma-separated args ----
                // Dedup across AST/named-arg hints
                let mut seen: HashSet<(u32, u32, String)> = HashSet::new();
                for h in &hints {
                    if let (Some(l), Some(c), Some(lbl)) = (
                        h.get("position")
                            .and_then(|p| p.get("line"))
                            .and_then(|v| v.as_u64())
                            .map(|v| v as u32),
                        h.get("position")
                            .and_then(|p| p.get("character"))
                            .and_then(|v| v.as_u64())
                            .map(|v| v as u32),
                        h.get("label").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    ) {
                        seen.insert((l, c, lbl));
                    }
                }

                // Range window (reuse earlier computed range/window start)
                let has_range_2 = params.get("range").and_then(|r| r.as_object()).is_some();
                let (win_s, _win_e2, window2) = if has_range_2 {
                    let s_line = range["start"]["line"].as_u64().unwrap_or(0) as u32;
                    let s_ch = range["start"]["character"].as_u64().unwrap_or(0) as u32;
                    let e_line = range["end"]["line"].as_u64().unwrap_or(0) as u32;
                    let e_ch = range["end"]["character"].as_u64().unwrap_or(0) as u32;
                    self.slice_in_range(&doc.content, (s_line, s_ch), (e_line, e_ch))
                } else {
                    (0, doc.content.len(), doc.content.as_str())
                };
                let _start_line = range["start"]["line"].as_u64().unwrap_or(0) as u32;
                let _end_line = range["end"]["line"].as_u64().unwrap_or(u32::MAX as u64) as u32;

                for m in RE_CALL.find_iter(window2) {
                    // function name
                    let caps = RE_CALL.captures(&window2[m.start()..m.end()]).unwrap();
                    let name = caps.get(1).unwrap().as_str();
                    let sig = get_param_names(name);
                    if sig.is_empty() {
                        continue;
                    }
                    // find the matching ')' in the **whole window** after '('
                    // compute open and close offsets relative to window
                    let open_global = win_s + m.end() - 1; // m ends right after '(' due to regex; step back to '('
                    let open_in_window = m.end() - 1;
                    let close_in_window = match self.find_matching_paren(window2, open_in_window) {
                        Some(p) => p,
                        None => continue,
                    };
                    let body = &window2[open_in_window + 1..close_in_window];
                    let arg_starts = self.arg_starts_in_call_body(body);
                    for (i, rel) in arg_starts.iter().enumerate() {
                        if i >= sig.len() {
                            break;
                        }
                        let local_anchor = self.smart_arg_anchor(body, *rel);
                        let global_off = open_global + 1 + local_anchor;
                        let (l, c) = doc.line_starts.offset_to_position(&doc.content, global_off);
                        // Get range bounds for position checking
                        let start_line = range["start"]["line"].as_u64().unwrap_or(0) as u32;
                        let start_char = range["start"]["character"].as_u64().unwrap_or(0) as u32;
                        let end_line =
                            range["end"]["line"].as_u64().unwrap_or(u32::MAX as u64) as u32;
                        let end_char =
                            range["end"]["character"].as_u64().unwrap_or(u32::MAX as u64) as u32;
                        let pos = crate::positions::Position::new(l, c);
                        let hint_range = crate::positions::Range::new(
                            start_line, start_char, end_line, end_char,
                        );
                        if !crate::positions::pos_in_range(pos, hint_range) {
                            continue;
                        }
                        let label = format!("{}:", sig[i]);
                        if seen.insert((l, c, label.clone())) {
                            hints.push(json!({
                                "position": { "line": l, "character": c },
                                "label": label,
                                "kind": 2, // Parameter
                            }));
                        }
                    }
                }

                // Non-parenthesized built-ins: `push @arr, ...;`, `open FH, ...;`, `split /re/, s;`
                for (name, sig) in BUILTIN_SIGS.entries() {
                    let re = Regex::new(&format!(r#"(?m)\b{}\b(?!\s*\()"#, regex::escape(name)))
                        .unwrap();
                    for m in re.find_iter(window2) {
                        // body = from end of name to end-of-statement `;` (window-limited)
                        let body_start = m.end();
                        let stmt_end_in_win = self.slice_until_stmt_end(window2, body_start);
                        let body = &window2[body_start..stmt_end_in_win];
                        let arg_starts = self.arg_starts_top_level(body);
                        for (i, rel) in arg_starts.iter().enumerate() {
                            if i >= sig.len() {
                                break;
                            }
                            // Re-anchor on the variable/filehandle token (skip `my`/`our`, whitespace)
                            let local_anchor = self.smart_arg_anchor(body, *rel);
                            let global_off = win_s + body_start + local_anchor;
                            let (l, c) =
                                doc.line_starts.offset_to_position(&doc.content, global_off);
                            // Get range bounds for position checking
                            let start_line = range["start"]["line"].as_u64().unwrap_or(0) as u32;
                            let start_char =
                                range["start"]["character"].as_u64().unwrap_or(0) as u32;
                            let end_line =
                                range["end"]["line"].as_u64().unwrap_or(u32::MAX as u64) as u32;
                            let end_char =
                                range["end"]["character"].as_u64().unwrap_or(u32::MAX as u64)
                                    as u32;
                            let pos = crate::positions::Position::new(l, c);
                            let hint_range = crate::positions::Range::new(
                                start_line, start_char, end_line, end_char,
                            );
                            if !crate::positions::pos_in_range(pos, hint_range) {
                                continue;
                            }
                            let label = format!("{}:", sig[i]);
                            if seen.insert((l, c, label.clone())) {
                                hints.push(json!({
                                    "position": { "line": l, "character": c },
                                    "label": label,
                                    "kind": 2, // Parameter
                                }));
                            }
                        }
                    }
                }

                // ---- Type hints (simple heuristics) ----
                // Patterns:
                //  my %hash;              -> "hash"
                //  my @arr;               -> "array"
                //  my $ref = {};          -> "HashRef"
                //  my $ref = [];          -> "ArrayRef"
                //  my $s   = "text";      -> "Str"
                let type_patterns: &[(&str, &str, u8)] = &[
                    // allow optional initializer: `my %h;` or `my %h = ();`
                    (r"(?m)my\s+%([A-Za-z_]\w*)\b(?:\s*=\s*\(\s*\))?", "hash", 1),
                    (r"(?m)my\s+@([A-Za-z_]\w*)\b(?:\s*=\s*\(\s*\))?", "array", 1),
                    (r"(?m)my\s+\$([A-Za-z_]\w*)\s*=\s*\{", "HashRef", 1),
                    (r"(?m)my\s+\$([A-Za-z_]\w*)\s*=\s*\[", "ArrayRef", 1),
                    (r#"(?m)my\s+\$([A-Za-z_]\w*)\s*=\s*"(?:[^"\\]|\\.)*""#, "Str", 1),
                ];
                for (pat, label_txt, kind_code) in type_patterns {
                    let re = Regex::new(pat).unwrap();
                    for m in re.find_iter(window2) {
                        // Position hint at the variable's start (first capture)
                        if let Some(caps) = re.captures(&window2[m.start()..m.end()]) {
                            if let Some(var) = caps.get(1) {
                                let var_global = win_s + m.start() + var.start();
                                let (l, c) =
                                    doc.line_starts.offset_to_position(&doc.content, var_global);
                                // Get range bounds for position checking
                                let start_line =
                                    range["start"]["line"].as_u64().unwrap_or(0) as u32;
                                let start_char =
                                    range["start"]["character"].as_u64().unwrap_or(0) as u32;
                                let end_line =
                                    range["end"]["line"].as_u64().unwrap_or(u32::MAX as u64) as u32;
                                let end_char =
                                    range["end"]["character"].as_u64().unwrap_or(u32::MAX as u64)
                                        as u32;
                                let pos = crate::positions::Position::new(l, c);
                                let hint_range = crate::positions::Range::new(
                                    start_line, start_char, end_line, end_char,
                                );
                                if !crate::positions::pos_in_range(pos, hint_range) {
                                    continue;
                                }
                                let label = label_txt.to_string();
                                if seen.insert((l, c, label.clone())) {
                                    hints.push(json!({
                                        "position": { "line": l, "character": c },
                                        "label": label,
                                        "kind": *kind_code, // 1 = Type
                                    }));
                                }
                            }
                        }
                    }
                }

                return Ok(Some(json!(hints)));
            }
        }

        Ok(Some(json!([])))
    }

    /// Handle test discovery request
    fn handle_test_discovery(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");

            eprintln!("Discovering tests for: {}", uri);

            let documents = self.documents.lock().unwrap();
            if let Some(doc) = self.get_document(&documents, uri) {
                if let Some(ref ast) = doc.ast {
                    let runner = TestRunner::new(doc.content.clone(), uri.to_string());
                    let tests = runner.discover_tests(ast);

                    // Convert test items to JSON
                    let test_items: Vec<Value> = tests
                        .into_iter()
                        .map(|test| {
                            json!({
                                "id": test.id,
                                "label": test.label,
                                "uri": test.uri,
                                "range": {
                                    "start": {
                                        "line": test.range.start_line,
                                        "character": test.range.start_character
                                    },
                                    "end": {
                                        "line": test.range.end_line,
                                        "character": test.range.end_character
                                    }
                                },
                                "kind": match test.kind {
                                    TestKind::File => "file",
                                    TestKind::Suite => "suite",
                                    TestKind::Test => "test"
                                },
                                "children": test.children.into_iter()
                                    .map(|child| json!({
                                        "id": child.id,
                                        "label": child.label,
                                        "uri": child.uri,
                                        "range": {
                                            "start": {
                                                "line": child.range.start_line,
                                                "character": child.range.start_character
                                            },
                                            "end": {
                                                "line": child.range.end_line,
                                                "character": child.range.end_character
                                            }
                                        },
                                        "kind": match child.kind {
                                            TestKind::File => "file",
                                            TestKind::Suite => "suite",
                                            TestKind::Test => "test"
                                        },
                                        "children": []
                                    }))
                                    .collect::<Vec<_>>()
                            })
                        })
                        .collect();

                    eprintln!("Found {} test items", test_items.len());

                    return Ok(Some(json!(test_items)));
                }
            }
        }

        Ok(Some(json!([])))
    }

    /// Handle execute command request
    fn handle_execute_command(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        use crate::execute_command::ExecuteCommandProvider;

        if let Some(params) = params {
            let command = params["command"].as_str().unwrap_or("");
            let arguments = params["arguments"].as_array().cloned().unwrap_or_default();

            eprintln!("Executing command: {}", command);

            // Use the new execute command provider for new commands
            let provider = ExecuteCommandProvider::new();

            match command {
                // Keep existing test commands for backward compatibility
                "perl.runTest" => {
                    if let Some(test_id) = arguments.first().and_then(|v| v.as_str()) {
                        return self.run_test(test_id);
                    }
                }
                "perl.runTestFile" => {
                    if let Some(file_uri) = arguments.first().and_then(|v| v.as_str()) {
                        return self.run_test_file(file_uri);
                    }
                }
                // New commands handled by ExecuteCommandProvider
                "perl.runTests" | "perl.runFile" | "perl.runTestSub" => {
                    match provider.execute_command(command, arguments) {
                        Ok(result) => return Ok(Some(result)),
                        Err(e) => {
                            return Err(JsonRpcError { code: -32603, message: e, data: None });
                        }
                    }
                }
                // Debug commands (stub implementation for now)
                "perl.debugFile" | "perl.debugTests" => {
                    eprintln!("Debug command requested: {}", command);
                    // Return a success status - actual DAP integration can be added later
                    return Ok(Some(
                        json!({"status": "started", "message": format!("Debug session {} initiated", command)}),
                    ));
                }
                // Perl::Critic command
                "perl.runCritic" => {
                    if let Some(file_uri) = arguments.first().and_then(|v| v.as_str()) {
                        return self.run_perl_critic(file_uri);
                    } else {
                        return Err(JsonRpcError {
                            code: -32602,
                            message: "Missing file URI argument".to_string(),
                            data: None,
                        });
                    }
                }
                _ => {
                    return Err(JsonRpcError {
                        code: ERR_METHOD_NOT_FOUND,
                        message: format!("Unknown command: {}", command),
                        data: None,
                    });
                }
            }
        }

        Ok(Some(json!({"status": "error", "message": "Invalid parameters"})))
    }

    /// Run a specific test
    fn run_test(&self, test_id: &str) -> Result<Option<Value>, JsonRpcError> {
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
            let runner = TestRunner::new(doc.content.clone(), uri.to_string());
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

        Ok(Some(json!({"status": "error", "message": "Document not found"})))
    }

    /// Run all tests in a file
    fn run_test_file(&self, uri: &str) -> Result<Option<Value>, JsonRpcError> {
        eprintln!("Running test file: {}", uri);

        let documents = self.documents.lock().unwrap();
        if let Some(doc) = documents.get(uri) {
            let runner = TestRunner::new(doc.content.clone(), uri.to_string());
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

        Ok(Some(json!({"status": "error", "message": "Document not found"})))
    }

    /// Run Perl::Critic on a file
    fn run_perl_critic(&self, uri: &str) -> Result<Option<Value>, JsonRpcError> {
        use crate::perl_critic::{CriticAnalyzer, CriticConfig};
        use std::path::Path;

        eprintln!("Running Perl::Critic on: {}", uri);

        // Get the document content
        let documents = self.documents.lock().unwrap();
        if let Some(doc) = documents.get(uri) {
            // Try to get file path from URI
            let file_path = if uri.starts_with("file://") {
                uri.strip_prefix("file://").unwrap_or(uri)
            } else {
                uri
            };

            // First, try external perlcritic if available
            let violations = if Path::new("/usr/bin/perlcritic").exists()
                || Path::new("/usr/local/bin/perlcritic").exists()
                || std::process::Command::new("which")
                    .arg("perlcritic")
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false)
            {
                // Use external perlcritic
                let mut analyzer = CriticAnalyzer::new(CriticConfig::default());
                match analyzer.analyze_file(Path::new(file_path)) {
                    Ok(violations) => violations,
                    Err(e) => {
                        eprintln!("External perlcritic failed: {}, using built-in analyzer", e);
                        // Fall back to built-in analyzer
                        let builtin = BuiltInAnalyzer::new();
                        let code_text = crate::util::code_slice(&doc.content);
                        let mut parser = Parser::new(code_text);
                        if let Ok(ast) = parser.parse() {
                            builtin.analyze(&ast, &doc.content)
                        } else {
                            builtin.analyze(
                                &Node::new(
                                    NodeKind::Error { message: "Parse error".to_string() },
                                    crate::ast::SourceLocation { start: 0, end: 0 },
                                ),
                                &doc.content,
                            )
                        }
                    }
                }
            } else {
                // Use built-in analyzer
                eprintln!("Using built-in Perl::Critic analyzer");
                let builtin = BuiltInAnalyzer::new();
                let code_text = crate::util::code_slice(&doc.content);
                let mut parser = Parser::new(code_text);
                if let Ok(ast) = parser.parse() {
                    builtin.analyze(&ast, &doc.content)
                } else {
                    builtin.analyze(
                        &Node::new(
                            NodeKind::Error { message: "Parse error".to_string() },
                            crate::ast::SourceLocation { start: 0, end: 0 },
                        ),
                        &doc.content,
                    )
                }
            };

            // Convert violations to diagnostics and publish them
            let diagnostics: Vec<Value> = violations.iter().map(|v| {
                json!({
                    "range": {
                        "start": {"line": v.range.start.line, "character": v.range.start.column},
                        "end": {"line": v.range.end.line, "character": v.range.end.column}
                    },
                    "severity": match v.severity {
                        crate::perl_critic::Severity::Brutal |
                        crate::perl_critic::Severity::Cruel => 1, // Error
                        crate::perl_critic::Severity::Harsh => 2, // Warning
                        _ => 3, // Information
                    },
                    "code": v.policy,
                    "source": "Perl::Critic",
                    "message": v.description
                })
            }).collect();

            // Send publishDiagnostics notification
            let _ = self.notify(
                "textDocument/publishDiagnostics",
                json!({
                    "uri": uri,
                    "diagnostics": diagnostics
                }),
            );

            return Ok(Some(json!({
                "status": "success",
                "violationCount": violations.len(),
                "violations": violations.iter().map(|v| json!({
                    "policy": v.policy,
                    "severity": format!("{:?}", v.severity),
                    "message": v.description,
                    "line": v.range.start.line + 1
                })).collect::<Vec<_>>()
            })));
        }

        Ok(Some(json!({"status": "error", "message": "Document not found"})))
    }

    /// Handle workspace/configuration request
    fn handle_configuration(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            if let Some(items) = params.as_array() {
                let mut results = Vec::new();

                for item in items {
                    if let Some(section) = item.get("section").and_then(|s| s.as_str()) {
                        eprintln!("Configuration requested for section: {}", section);

                        let config = self.config.lock().unwrap();
                        let value = match section {
                            "perl.inlayHints.enabled" => json!(config.inlay_hints_enabled),
                            "perl.inlayHints.parameterHints" => {
                                json!(config.inlay_hints_parameter_hints)
                            }
                            "perl.inlayHints.typeHints" => json!(config.inlay_hints_type_hints),
                            "perl.inlayHints.chainedHints" => {
                                json!(config.inlay_hints_chained_hints)
                            }
                            "perl.inlayHints.maxLength" => json!(config.inlay_hints_max_length),
                            "perl.testRunner.enabled" => json!(config.test_runner_enabled),
                            "perl.testRunner.testCommand" => json!(config.test_runner_command),
                            "perl.testRunner.testArgs" => json!(config.test_runner_args),
                            "perl.testRunner.testTimeout" => json!(config.test_runner_timeout),
                            _ => json!(null),
                        };

                        results.push(value);
                    }
                }

                return Ok(Some(json!(results)));
            }
        }

        Ok(Some(json!([])))
    }

    /// Handle workspace/didChangeWatchedFiles notification
    fn handle_did_change_watched_files(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        use lsp_types::{DidChangeWatchedFilesParams, FileChangeType};

        let Some(params) = params else {
            return Ok(None);
        };

        let Ok(params) = serde_json::from_value::<DidChangeWatchedFilesParams>(params) else {
            eprintln!("Failed to parse didChangeWatchedFiles params");
            return Ok(None);
        };

        for change in params.changes {
            let uri = change.uri.to_string();
            let change_type = change.typ;

            eprintln!("File change detected: {} (type: {:?})", uri, change_type);

            match change_type {
                FileChangeType::CREATED => {
                    // Created
                    // Re-index the file if it's a Perl file
                    #[cfg(feature = "workspace")]
                    if let Some(ref workspace_index) = self.workspace_index {
                        if uri.ends_with(".pl") || uri.ends_with(".pm") || uri.ends_with(".t") {
                            if let Some(path) = uri_to_fs_path(&uri) {
                                if let Ok(content) = std::fs::read_to_string(&path) {
                                    if let Ok(url) = url::Url::parse(&uri) {
                                        let _ = workspace_index.index_file(url, content);
                                        eprintln!("Indexed new file: {}", uri);
                                    }
                                }
                            }
                        }
                    }
                }
                FileChangeType::CHANGED => {
                    // Changed
                    // Re-index the file
                    #[cfg(feature = "workspace")]
                    if let Some(ref workspace_index) = self.workspace_index {
                        if let Some(path) = uri_to_fs_path(&uri) {
                            if let Ok(content) = std::fs::read_to_string(&path) {
                                if let Ok(url) = url::Url::parse(&uri) {
                                    // Clear old index data
                                    workspace_index.clear_file(&uri);
                                    // Re-index with new content
                                    let _ = workspace_index.index_file(url, content.clone());
                                }
                            }
                        }
                    }

                    // Also update our internal document store if it exists
                    #[cfg(feature = "workspace")]
                    if let Ok(mut documents) = self.documents.lock() {
                        if let Some(doc) = self.get_document_mut(&mut documents, &uri) {
                            // Note: content variable is only available inside the cfg block above
                            // We'll need to re-read the file or restructure this
                            if let Some(path) = uri_to_fs_path(&uri) {
                                if let Ok(content) = std::fs::read_to_string(&path) {
                                    doc.content = content;
                                    doc._version += 1;
                                    // Clear cached AST
                                    doc.ast = None;
                                }
                            }
                        }
                    }

                    eprintln!("Re-indexed changed file: {}", uri);
                }
                FileChangeType::DELETED => {
                    // Deleted
                    // Remove from index
                    #[cfg(feature = "workspace")]
                    if let Some(ref workspace_index) = self.workspace_index {
                        workspace_index.remove_file(&uri);
                    }

                    // Remove from document store
                    if let Ok(mut documents) = self.documents.lock() {
                        documents.remove(&uri);
                    }

                    eprintln!("Removed deleted file from index: {}", uri);
                }
                _ => {}
            }
        }

        // This is a notification, no response needed
        Ok(None)
    }

    /// Handle workspace/willRenameFiles request
    fn handle_will_rename_files(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            if let Some(files) = params["files"].as_array() {
                let mut workspace_edit = json!({
                    "changes": {}
                });

                for file in files {
                    let Some(old_uri) = file["oldUri"].as_str() else {
                        continue;
                    };
                    let Some(new_uri) = file["newUri"].as_str() else {
                        continue;
                    };

                    eprintln!("File rename: {} -> {}", old_uri, new_uri);

                    // Extract module names from file paths
                    let old_module = path_to_module_name(old_uri);
                    let new_module = path_to_module_name(new_uri);

                    if !old_module.is_empty() && !new_module.is_empty() {
                        // Find all files that reference the old module
                        #[cfg(feature = "workspace")]
                        let dependents = if let Some(ref workspace_index) = self.workspace_index {
                            workspace_index.find_dependents(&old_module)
                        } else {
                            Vec::new()
                        };

                        #[cfg(not(feature = "workspace"))]
                        let dependents = Vec::<String>::new();

                        for dependent_uri in dependents {
                            // Get the document content
                            let Ok(documents) = self.documents.lock() else {
                                continue;
                            };
                            if let Some(doc) = documents.get(&dependent_uri) {
                                let mut edits = Vec::new();

                                // Find and replace use statements
                                for (line_num, line) in doc.content.lines().enumerate() {
                                    if line.contains(&format!("use {}", old_module))
                                        || line.contains(&format!("require {}", old_module))
                                        || line.contains(&format!("use parent '{}'", old_module))
                                        || line.contains(&format!("use base '{}'", old_module))
                                    {
                                        let new_line = line.replace(&old_module, &new_module);
                                        edits.push(json!({
                                            "range": {
                                                "start": {"line": line_num, "character": 0},
                                                "end": {"line": line_num, "character": line.len()}
                                            },
                                            "newText": new_line
                                        }));
                                    }
                                }

                                if !edits.is_empty() {
                                    workspace_edit["changes"][dependent_uri] = json!(edits);
                                }
                            }
                        }
                    }

                    // Update the index for the renamed file
                    #[cfg(feature = "workspace")]
                    if let Some(ref workspace_index) = self.workspace_index {
                        workspace_index.remove_file(old_uri);
                        if let Some(path) = uri_to_fs_path(new_uri) {
                            if let Ok(content) = std::fs::read_to_string(&path) {
                                if let Ok(url) = url::Url::parse(new_uri) {
                                    let _ = workspace_index.index_file(url, content.clone());
                                }
                            }
                        }
                    }
                }

                return Ok(Some(workspace_edit));
            }
        }

        // Return empty edit if no changes needed
        Ok(Some(json!({"changes": {}})))
    }

    /// Handle workspace/didDeleteFiles notification
    fn handle_did_delete_files(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            if let Some(files) = params["files"].as_array() {
                for file in files {
                    let Some(uri) = file["uri"].as_str() else {
                        continue;
                    };

                    eprintln!("File deleted: {}", uri);

                    // Remove from workspace index
                    #[cfg(feature = "workspace")]
                    if let Some(ref workspace_index) = self.workspace_index {
                        workspace_index.remove_file(uri);
                    }

                    // Remove from document store
                    if let Ok(mut documents) = self.documents.lock() {
                        documents.remove(uri);
                    }
                }
            }
        }

        // This is a notification, no response needed
        Ok(None)
    }

    /// Handle workspace/didChangeWorkspaceFolders notification
    fn handle_did_change_workspace_folders(
        &self,
        params: Option<Value>,
    ) -> Result<(), JsonRpcError> {
        if let Some(params) = params {
            if let Some(event) = params.get("event") {
                // Handle added folders
                if let Some(added) = event["added"].as_array() {
                    let mut workspace_folders = self.workspace_folders.lock().unwrap();
                    for folder in added {
                        if let Some(uri) = folder["uri"].as_str() {
                            eprintln!("Added workspace folder: {}", uri);
                            workspace_folders.push(uri.to_string());
                        }
                    }
                }

                // Handle removed folders
                if let Some(removed) = event["removed"].as_array() {
                    let mut workspace_folders = self.workspace_folders.lock().unwrap();
                    for folder in removed {
                        if let Some(uri) = folder["uri"].as_str() {
                            eprintln!("Removed workspace folder: {}", uri);
                            workspace_folders.retain(|f| f.as_str() != uri);

                            // Also remove documents from the removed workspace
                            let mut documents = self.documents.lock().unwrap();
                            let docs_to_remove: Vec<String> = documents
                                .keys()
                                .filter(|doc_uri| doc_uri.starts_with(uri))
                                .cloned()
                                .collect();

                            for doc_uri in docs_to_remove {
                                eprintln!("Removing document from removed workspace: {}", doc_uri);
                                documents.remove(&doc_uri);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Handle workspace/applyEdit request
    fn handle_apply_edit(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let Some(edit) = params.get("edit") else {
                return Ok(Some(
                    json!({"applied": false, "failureReason": "Missing 'edit' field"}),
                ));
            };

            eprintln!("Applying workspace edit");

            // Apply changes to each document
            if let Some(changes) = edit["changes"].as_object() {
                for (uri, edits) in changes {
                    if let Some(edits) = edits.as_array() {
                        let Ok(mut documents) = self.documents.lock() else {
                            continue;
                        };
                        if let Some(doc) = self.get_document_mut(&mut documents, uri) {
                            // Apply edits in reverse order to maintain positions
                            let mut sorted_edits = edits.clone();
                            sorted_edits.sort_by(|a, b| {
                                let a_line = a["range"]["start"]["line"].as_u64().unwrap_or(0);
                                let b_line = b["range"]["start"]["line"].as_u64().unwrap_or(0);
                                b_line.cmp(&a_line)
                            });

                            for edit in sorted_edits {
                                if let Some(new_text) = edit["newText"].as_str() {
                                    let start_line =
                                        edit["range"]["start"]["line"].as_u64().unwrap_or(0)
                                            as usize;
                                    let start_char =
                                        edit["range"]["start"]["character"].as_u64().unwrap_or(0)
                                            as usize;
                                    let end_line =
                                        edit["range"]["end"]["line"].as_u64().unwrap_or(0) as usize;
                                    let end_char =
                                        edit["range"]["end"]["character"].as_u64().unwrap_or(0)
                                            as usize;

                                    // Apply the edit to the document content
                                    let lines: Vec<String> =
                                        doc.content.lines().map(String::from).collect();
                                    let mut new_lines = Vec::new();

                                    // Copy lines before the edit
                                    for i in 0..start_line {
                                        new_lines.push(lines[i].clone());
                                    }

                                    // Apply the edit
                                    if start_line == end_line {
                                        let line = &lines[start_line];
                                        let new_line = format!(
                                            "{}{}{}",
                                            &line[..start_char.min(line.len())],
                                            new_text,
                                            &line[end_char.min(line.len())..]
                                        );
                                        new_lines.push(new_line);
                                    } else {
                                        // Multi-line edit
                                        let first_line = &lines[start_line];
                                        let last_line = &lines[end_line];
                                        let new_line = format!(
                                            "{}{}{}",
                                            &first_line[..start_char.min(first_line.len())],
                                            new_text,
                                            &last_line[end_char.min(last_line.len())..]
                                        );
                                        new_lines.push(new_line);
                                    }

                                    // Copy lines after the edit
                                    for i in (end_line + 1)..lines.len() {
                                        new_lines.push(lines[i].clone());
                                    }

                                    doc.content = new_lines.join("\n");
                                    doc._version += 1;
                                }
                            }

                            // Re-index the file after changes
                            #[cfg(feature = "workspace")]
                            if let Some(ref workspace_index) = self.workspace_index {
                                if let Ok(url) = url::Url::parse(uri) {
                                    let _ = workspace_index.index_file(url, doc.content.clone());
                                }
                            }

                            // Clear cached AST
                            doc.ast = None;
                        }
                    }
                }
            }

            // Return success
            return Ok(Some(json!({"applied": true})));
        }

        Ok(Some(json!({"applied": false, "failureReason": "Invalid parameters"})))
    }

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

    /// Handle documentLink request
    fn handle_document_link(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");

            let documents = self.documents.lock().unwrap();
            if let Some(doc) = self.get_document(&documents, uri) {
                let uri_parsed = url::Url::parse(uri).map_err(|_| JsonRpcError {
                    code: -32602,
                    message: "Invalid URI".to_string(),
                    data: None,
                })?;
                match crate::lsp_document_link::collect_document_links(&doc.content, &uri_parsed) {
                    Ok(links) => Ok(Some(serde_json::to_value(links).unwrap_or(Value::Null))),
                    Err(_) => Ok(Some(Value::Null)),
                }
            } else {
                Ok(Some(Value::Null))
            }
        } else {
            Ok(Some(Value::Null))
        }
    }

    // Handle selectionRange request - removed duplicate (see new implementation above)
    // The old stub was commented out since the real implementation is above

    // Handle onTypeFormatting request - OLD STUB (replaced above)
    #[allow(dead_code)]
    fn handle_on_type_formatting_old(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let ch = params["ch"].as_str().unwrap_or("");
            let line = params["position"]["line"].as_u64().unwrap_or(0) as u32;
            let character = params["position"]["character"].as_u64().unwrap_or(0) as u32;
            let tab_size = params["options"]["tabSize"].as_u64().unwrap_or(4) as usize;
            let insert_spaces = params["options"]["insertSpaces"].as_bool().unwrap_or(true);

            let documents = self.documents.lock().unwrap();
            if let Some(doc) = self.get_document(&documents, uri) {
                let uri_obj = uri
                    .parse::<lsp_types::Uri>()
                    .unwrap_or_else(|_| "file:///tmp".parse::<lsp_types::Uri>().unwrap());
                let edits = crate::lsp_on_type_formatting::format_on_type(
                    &doc.content,
                    uri_obj,
                    ch.to_string(),
                    lsp_types::Position::new(line, character),
                    tab_size,
                    insert_spaces,
                );
                Ok(Some(serde_json::to_value(edits).unwrap_or(Value::Null)))
            } else {
                Ok(Some(Value::Null))
            }
        } else {
            Ok(Some(Value::Null))
        }
    }

    /// Register file watchers for Perl files
    fn register_file_watchers_async(&self) {
        use lsp_types::{
            DidChangeWatchedFilesRegistrationOptions, FileSystemWatcher, GlobPattern, Registration,
            RegistrationParams, WatchKind,
            notification::{DidChangeWatchedFiles, Notification},
        };

        let watchers = vec![
            FileSystemWatcher {
                glob_pattern: GlobPattern::String("**/*.pl".into()),
                kind: Some(WatchKind::Create | WatchKind::Change | WatchKind::Delete),
            },
            FileSystemWatcher {
                glob_pattern: GlobPattern::String("**/*.pm".into()),
                kind: Some(WatchKind::Create | WatchKind::Change | WatchKind::Delete),
            },
            FileSystemWatcher {
                glob_pattern: GlobPattern::String("**/*.t".into()),
                kind: Some(WatchKind::Create | WatchKind::Change | WatchKind::Delete),
            },
        ];

        let opts = DidChangeWatchedFilesRegistrationOptions { watchers };
        let reg = Registration {
            id: "perl-didChangeWatchedFiles".into(),
            method: <DidChangeWatchedFiles as Notification>::METHOD.to_string(),
            register_options: Some(serde_json::to_value(opts).unwrap_or(Value::Null)),
        };

        let params = RegistrationParams { registrations: vec![reg] };

        // Send the registration request without waiting for a response
        // Use a random ID since we're not tracking the response
        let request_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let request = json!({
            "jsonrpc": "2.0",
            "id": serde_json::Value::Number(serde_json::Number::from(request_id)),
            "method": "client/registerCapability",
            "params": params
        });

        // Send using the proper output mechanism (fire and forget)
        if let Ok(mut output) = self.output.lock() {
            let msg = serde_json::to_string(&request).unwrap();
            write!(output, "Content-Length: {}\r\n\r\n{}", msg.len(), msg).ok();
            output.flush().ok();
        }

        eprintln!("Sent file watcher registration request (async)");
    }

    /// Handle textDocument/diagnostic request (LSP 3.17 pull diagnostics)
    fn handle_document_diagnostic(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let previous_result_id = params["previousResultId"].as_str().map(|s| s.to_string());

            let documents = self.documents.lock().unwrap();
            if let Some(doc) = self.get_document(&documents, uri) {
                // Get diagnostics from the existing provider
                if let Some(ast) = &doc.ast {
                    let provider = DiagnosticsProvider::new(ast, doc.content.clone());
                    let diagnostics =
                        provider.get_diagnostics(ast, &doc.parse_errors, &doc.content);

                    // Generate a result ID based on content
                    let result_id = format!("{:x}", md5::compute(&doc.content));

                    // If the result ID matches the previous one, return unchanged
                    if let Some(prev_id) = previous_result_id {
                        if prev_id == result_id {
                            return Ok(Some(json!({
                                "kind": "unchanged",
                                "resultId": prev_id
                            })));
                        }
                    }

                    // Convert to LSP diagnostics
                    let lsp_diagnostics: Vec<Value> = diagnostics
                        .into_iter()
                        .enumerate()
                        .map(|(j, d)| {
                            // Cooperative yield every 32 items
                            if j & 0x1f == 0 {
                                std::thread::yield_now();
                            }
                            let start_pos =
                                doc.line_starts.offset_to_position(&doc.content, d.range.0);
                            let end_pos =
                                doc.line_starts.offset_to_position(&doc.content, d.range.1);
                            json!({
                                "range": {
                                    "start": {
                                        "line": start_pos.0,
                                        "character": start_pos.1,
                                    },
                                    "end": {
                                        "line": end_pos.0,
                                        "character": end_pos.1,
                                    },
                                },
                                "severity": match d.severity {
                                    InternalDiagnosticSeverity::Error => 1,
                                    InternalDiagnosticSeverity::Warning => 2,
                                    InternalDiagnosticSeverity::Information => 3,
                                    InternalDiagnosticSeverity::Hint => 4,
                                },
                                "source": "perl-lsp",
                                "message": d.message,
                            })
                        })
                        .collect();

                    return Ok(Some(json!({
                        "kind": "full",
                        "resultId": result_id,
                        "items": lsp_diagnostics
                    })));
                }
            }
        }

        // Return empty diagnostics if document not found
        Ok(Some(json!({
            "kind": "full",
            "items": []
        })))
    }

    /// Handle workspace/diagnostic request (LSP 3.17 pull diagnostics)
    fn handle_workspace_diagnostic(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        let previous_result_ids = if let Some(params) = &params {
            if let Some(ids) = params["previousResultIds"].as_array() {
                ids.iter()
                    .filter_map(|item| {
                        let uri = item["uri"].as_str()?;
                        let id = item["value"].as_str()?;
                        Some((uri.to_string(), id.to_string()))
                    })
                    .collect()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        let mut items = Vec::new();

        // Collect document snapshots without holding lock
        let docs_snapshot: Vec<(String, DocumentState)> = {
            let documents = self.documents.lock().unwrap();
            documents.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        };

        for (i, (uri_str, doc)) in docs_snapshot.iter().enumerate() {
            // Cooperative yield every 8 documents
            if i & 0x7 == 0 {
                std::thread::yield_now();
            }

            // Check if we have a previous result ID for this document
            let prev_id =
                previous_result_ids.iter().find(|(u, _)| u == uri_str).map(|(_, id)| id.clone());

            if let Some(ast) = &doc.ast {
                let provider = DiagnosticsProvider::new(ast, doc.content.clone());
                let diagnostics = provider.get_diagnostics(ast, &doc.parse_errors, &doc.content);

                // Generate result ID
                let result_id = format!("{:x}", md5::compute(&doc.content));

                // Check if unchanged
                let report = if let Some(prev) = prev_id {
                    if prev == result_id {
                        json!({
                            "uri": uri_str,
                            "version": doc._version,
                            "kind": "unchanged",
                            "resultId": prev
                        })
                    } else {
                        // Convert diagnostics
                        let lsp_diagnostics: Vec<Value> = diagnostics
                            .into_iter()
                            .enumerate()
                            .map(|(j, d)| {
                                // Cooperative yield every 32 items
                                if j & 0x1f == 0 {
                                    std::thread::yield_now();
                                }
                                let start_pos =
                                    doc.line_starts.offset_to_position(&doc.content, d.range.0);
                                let end_pos =
                                    doc.line_starts.offset_to_position(&doc.content, d.range.1);
                                json!({
                                    "range": {
                                        "start": {
                                            "line": start_pos.0,
                                            "character": start_pos.1,
                                        },
                                        "end": {
                                            "line": end_pos.0,
                                            "character": end_pos.1,
                                        },
                                    },
                                    "severity": match d.severity {
                                        InternalDiagnosticSeverity::Error => 1,
                                        InternalDiagnosticSeverity::Warning => 2,
                                        InternalDiagnosticSeverity::Information => 3,
                                        InternalDiagnosticSeverity::Hint => 4,
                                    },
                                    "source": "perl-lsp",
                                    "message": d.message,
                                })
                            })
                            .collect();

                        json!({
                            "uri": uri_str,
                            "version": doc._version,
                            "kind": "full",
                            "resultId": result_id,
                            "items": lsp_diagnostics
                        })
                    }
                } else {
                    // No previous result, return full
                    let lsp_diagnostics: Vec<Value> = diagnostics
                        .into_iter()
                        .enumerate()
                        .map(|(j, d)| {
                            // Cooperative yield every 32 items
                            if j & 0x1f == 0 {
                                std::thread::yield_now();
                            }
                            let start_pos =
                                doc.line_starts.offset_to_position(&doc.content, d.range.0);
                            let end_pos =
                                doc.line_starts.offset_to_position(&doc.content, d.range.1);
                            json!({
                                "range": {
                                    "start": {
                                        "line": start_pos.0,
                                        "character": start_pos.1,
                                    },
                                    "end": {
                                        "line": end_pos.0,
                                        "character": end_pos.1,
                                    },
                                },
                                "severity": match d.severity {
                                    InternalDiagnosticSeverity::Error => 1,
                                    InternalDiagnosticSeverity::Warning => 2,
                                    InternalDiagnosticSeverity::Information => 3,
                                    InternalDiagnosticSeverity::Hint => 4,
                                },
                                "source": "perl-lsp",
                                "message": d.message,
                            })
                        })
                        .collect();

                    json!({
                        "uri": uri_str,
                        "version": doc._version,
                        "kind": "full",
                        "resultId": result_id,
                        "items": lsp_diagnostics
                    })
                };

                items.push(report);
            }
        }

        Ok(Some(json!({ "items": items })))
    }

    /// Handle workspace/symbol/resolve request (LSP 3.17)
    fn handle_workspace_symbol_resolve(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            // Extract the symbol to resolve
            let symbol = params.as_object().ok_or_else(|| JsonRpcError {
                code: -32602,
                message: "Invalid params".to_string(),
                data: None,
            })?;

            // Get the URI and name from the symbol
            let uri = symbol
                .get("location")
                .and_then(|l| l.get("uri"))
                .and_then(|u| u.as_str())
                .unwrap_or("");

            let name = symbol.get("name").and_then(|n| n.as_str()).unwrap_or("");

            // Normalize the URI for lookup
            let uri_key = self.normalize_uri_key(uri);

            // Look up the symbol in our index to get more details
            let documents = self.documents.lock().unwrap();
            let doc_opt = documents.get(&uri_key).or_else(|| documents.get(uri)); // try raw as a fallback

            if let Some(doc) = doc_opt {
                if let Some(ast) = &doc.ast {
                    // Find the symbol in the AST to get more accurate information
                    let extractor = crate::symbol::SymbolExtractor::new_with_source(&doc.content);
                    let symbol_table = extractor.extract(ast);

                    // Find matching symbol
                    for symbols in symbol_table.symbols.values() {
                        for sym in symbols {
                            if sym.name == name {
                                // Return enhanced symbol with detail and accurate range
                                let start_pos = doc
                                    .line_starts
                                    .offset_to_position(&doc.content, sym.location.start);
                                let end_pos = doc
                                    .line_starts
                                    .offset_to_position(&doc.content, sym.location.end);

                                // Start with the provided symbol JSON so we can add
                                // additional details without panicking if fields are missing
                                let mut resolved = json!(symbol);

                                // Add detail based on symbol kind
                                let detail = match sym.kind {
                                    crate::symbol::SymbolKind::Subroutine => {
                                        format!("sub {}", name)
                                    }
                                    crate::symbol::SymbolKind::ScalarVariable => {
                                        format!("${}", name)
                                    }
                                    crate::symbol::SymbolKind::ArrayVariable => {
                                        format!("@{}", name)
                                    }
                                    crate::symbol::SymbolKind::HashVariable => format!("%{}", name),
                                    crate::symbol::SymbolKind::Package => {
                                        format!("package {}", name)
                                    }
                                    crate::symbol::SymbolKind::Constant => {
                                        format!("constant {}", name)
                                    }
                                    _ => name.to_string(),
                                };
                                resolved["detail"] = json!(detail);

                                // Update location with accurate range
                                resolved["location"]["range"] = json!({
                                    "start": {
                                        "line": start_pos.0,
                                        "character": start_pos.1,
                                    },
                                    "end": {
                                        "line": end_pos.0,
                                        "character": end_pos.1,
                                    }
                                });

                                // Add scope information if available
                                if let Some(scope) = symbol_table.scopes.get(&sym.scope_id) {
                                    if scope.parent.is_some() {
                                        // Find parent scope's package name
                                        for parent_symbols in symbol_table.symbols.values() {
                                            for parent_sym in parent_symbols {
                                                if parent_sym.scope_id == scope.parent.unwrap_or(0)
                                                    && parent_sym.kind
                                                        == crate::symbol::SymbolKind::Package
                                                {
                                                    resolved["containerName"] =
                                                        json!(parent_sym.name);
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }

                                return Ok(Some(json!(resolved)));
                            }
                        }
                    }
                }
            }

            // Return the original symbol if we couldn't enhance it
            Ok(Some(json!(symbol)))
        } else {
            Err(JsonRpcError { code: -32602, message: "Missing params".to_string(), data: None })
        }
    }
}

/// Convert a file path to a Perl module name
fn path_to_module_name(uri: &str) -> String {
    #[cfg(feature = "workspace")]
    let path =
        uri_to_fs_path(uri).and_then(|p| p.to_str().map(|s| s.to_string())).unwrap_or_else(|| {
            // Fallback to trim_start_matches for backward compatibility
            uri.trim_start_matches("file://").to_string()
        });
    #[cfg(not(feature = "workspace"))]
    let path = uri.trim_start_matches("file://").to_string();
    let path = path.as_str();
    let path = path.trim_end_matches(".pm").trim_end_matches(".pl");

    // Find the lib directory and extract module path
    if let Some(lib_index) = path.rfind("/lib/") {
        let module_path = &path[lib_index + 5..];
        return module_path.replace('/', "::");
    }

    // Fallback: use filename as module name
    if let Some(last_slash) = path.rfind('/') {
        return path[last_slash + 1..].to_string();
    }

    path.to_string()
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

    /// Resolve a module name to a file path URI
    fn resolve_module_to_path(&self, module_name: &str) -> Option<String> {
        use std::time::{Duration, Instant};

        // Convert Module::Name to Module/Name.pm
        let relative_path = format!("{}.pm", module_name.replace("::", "/"));

        // First check if we have the document already opened (fast path)
        let documents = self.documents.lock().unwrap();
        for (uri, _doc) in documents.iter() {
            if uri.ends_with(&relative_path) {
                return Some(uri.clone());
            }
        }
        drop(documents);

        // Set a timeout for file system operations
        let start_time = Instant::now();
        let timeout = Duration::from_millis(50); // Reduced timeout for faster response

        // Get workspace folders from initialization
        let workspace_folders = self.workspace_folders.lock().unwrap().clone();

        // Only check workspace-local directories to avoid blocking
        let search_dirs = ["lib", ".", "local/lib/perl5"];

        for workspace_folder in workspace_folders.iter() {
            // Early timeout check
            if start_time.elapsed() > timeout {
                eprintln!(
                    "Module resolution timeout for: {} (elapsed: {:?})",
                    module_name,
                    start_time.elapsed()
                );
                return None;
            }

            // Parse the workspace folder URI to get the file path
            let workspace_path = if workspace_folder.starts_with("file://") {
                workspace_folder.strip_prefix("file://").unwrap_or(workspace_folder)
            } else {
                workspace_folder
            };

            for dir in &search_dirs {
                let full_path = if *dir == "." {
                    format!("{}/{}", workspace_path, relative_path)
                } else {
                    format!("{}/{}/{}", workspace_path, dir, relative_path)
                };

                // Check timeout before each FS operation
                if start_time.elapsed() > timeout {
                    return None;
                }

                // Use metadata() instead of exists() as it's slightly more predictable
                // and we can potentially wrap this in a timeout later
                match std::fs::metadata(&full_path) {
                    Ok(meta) if meta.is_file() => {
                        return Some(format!("file://{}", full_path));
                    }
                    _ => {
                        // File doesn't exist or isn't a regular file, continue
                    }
                }

                // Final timeout check
                if start_time.elapsed() > timeout {
                    return None;
                }
            }
        }

        // Don't check system paths (@INC) to avoid blocking on network filesystems
        None
    }

    /// Set the root path from the root URI during initialization
    fn set_root_uri(&self, root_uri: &str) {
        let root_path = Url::parse(root_uri).ok().and_then(|u| u.to_file_path().ok());
        *self.root_path.lock().unwrap() = root_path;
    }

    /// Enhanced module path resolver using root_path
    fn resolve_module_path(&self, module: &str) -> Option<PathBuf> {
        let root = self.root_path.lock().unwrap().clone()?;
        let rel = module.replace("::", "/") + ".pm";
        for base in ["lib", "inc", "local/lib/perl5"] {
            let p = root.join(base).join(&rel);
            if p.exists() {
                return Some(p);
            }
        }
        // Best-effort even if not present (for test workspaces)
        Some(root.join("lib").join(rel))
    }

    /// Get buffer text for a URI
    fn buffer_text(&self, uri: &str) -> Option<String> {
        let docs = self.documents.lock().unwrap();
        docs.get(uri).map(|d| d.content.clone())
    }

    /// Iterate over all open buffers (for reference search)
    fn iter_open_buffers(&self) -> Vec<(String, String)> {
        let docs = self.documents.lock().unwrap();
        docs.iter().map(|(uri, doc)| (uri.clone(), doc.content.clone())).collect()
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
fn byte_to_utf16_col(line_text: &str, byte_pos: usize) -> usize {
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
