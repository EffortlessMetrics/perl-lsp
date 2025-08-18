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
    positions::LineStartsCache,
    semantic_tokens_provider::{SemanticTokensProvider, encode_semantic_tokens},
    test_runner::{TestKind, TestRunner},
    type_hierarchy::TypeHierarchyProvider,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::collections::HashSet;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU32, Ordering},
};

#[cfg(feature = "workspace")]
use crate::workspace_index::{LspWorkspaceSymbol, WorkspaceIndex, WorkspaceSymbol, uri_to_fs_path};

// JSON-RPC Error Codes
const ERR_METHOD_NOT_FOUND: i32 = -32601;
const ERR_REQUEST_CANCELLED: i32 = -32800;
const ERR_CONTENT_MODIFIED: i32 = -32801;

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
}

/// LSP server that handles JSON-RPC communication
pub struct LspServer {
    /// Document contents indexed by URI
    pub(crate) documents: Arc<Mutex<HashMap<String, DocumentState>>>,
    /// Whether the server is initialized
    initialized: bool,
    /// Workspace-wide index for cross-file features (enabled via PERL_LSP_WORKSPACE=1)
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

impl LspServer {
    /// Create a new LSP server
    pub fn new() -> Self {
        // Check if workspace indexing is enabled
        #[cfg(feature = "workspace")]
        let workspace_index = if std::env::var("PERL_LSP_WORKSPACE").is_ok() {
            Some(Arc::new(WorkspaceIndex::new()))
        } else {
            None
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
        }
    }

    /// Create a new LSP server with custom output (for testing)
    pub fn with_output(output: Arc<Mutex<Box<dyn Write + Send>>>) -> Self {
        // Check if workspace indexing is enabled
        #[cfg(feature = "workspace")]
        let workspace_index = if std::env::var("PERL_LSP_WORKSPACE").is_ok() {
            Some(Arc::new(WorkspaceIndex::new()))
        } else {
            None
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
                reader.read_exact(&mut content)?;

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

    /// Check if a request has been cancelled
    fn is_cancelled(&self, id: &Value) -> bool {
        let mut set = self.cancelled.lock().unwrap();
        set.take(id).is_some()
    }

    /// Handle a JSON-RPC request
    pub fn handle_request(&mut self, request: JsonRpcRequest) -> Option<JsonRpcResponse> {
        let id = request.id.clone();

        // Handle $/cancelRequest notification
        if request.method == "$/cancelRequest" {
            if let Some(params) = request.params {
                if let Some(cancel_id) = params.get("id").cloned() {
                    self.cancelled.lock().unwrap().insert(cancel_id);
                }
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

                // Register file watchers for Perl files
                self.register_file_watchers();

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
            "textDocument/codeAction" => self.handle_code_action(request.params),
            "textDocument/hover" => self.handle_hover(request.params),
            "textDocument/signatureHelp" => self.handle_signature_help(request.params),
            "textDocument/definition" => self.handle_definition(request.params),
            "textDocument/declaration" => self.handle_declaration(request.params),
            "textDocument/references" => self.handle_references(request.params),
            "textDocument/documentHighlight" => self.handle_document_highlight(request.params),
            "textDocument/prepareTypeHierarchy" => {
                self.handle_prepare_type_hierarchy(request.params)
            }
            "typeHierarchy/supertypes" => self.handle_type_hierarchy_supertypes(request.params),
            "typeHierarchy/subtypes" => self.handle_type_hierarchy_subtypes(request.params),
            "textDocument/prepareRename" => self.handle_prepare_rename(request.params),
            "textDocument/rename" => self.handle_rename(request.params),
            "textDocument/documentSymbol" => {
                eprintln!("Processing documentSymbol request");
                let result = self.handle_document_symbol(request.params);
                eprintln!("DocumentSymbol result: {:?}", result.is_ok());
                result
            }
            "textDocument/foldingRange" => self.handle_folding_range(request.params),
            "textDocument/formatting" => self.handle_formatting(request.params),
            "textDocument/rangeFormatting" => self.handle_range_formatting(request.params),
            "workspace/symbol" => self.handle_workspace_symbols(request.params),
            "textDocument/codeLens" => self.handle_code_lens(request.params),
            "codeLens/resolve" => self.handle_code_lens_resolve(request.params),
            "textDocument/semanticTokens/full" => self.handle_semantic_tokens_full(request.params),
            "textDocument/semanticTokens/range" => {
                self.handle_semantic_tokens_range(request.params)
            }
            "textDocument/prepareCallHierarchy" => {
                self.handle_prepare_call_hierarchy(request.params)
            }
            "callHierarchy/incomingCalls" => self.handle_incoming_calls(request.params),
            "callHierarchy/outgoingCalls" => self.handle_outgoing_calls(request.params),
            "textDocument/inlayHint" => self.handle_inlay_hint(request.params),
            "textDocument/documentLink" => self.handle_document_link(request.params),
            "textDocument/selectionRange" => self.handle_selection_range(request.params),
            "textDocument/onTypeFormatting" => self.handle_on_type_formatting(request.params),
            "workspace/executeCommand" => self.handle_execute_command(request.params),
            "experimental/testDiscovery" => self.handle_test_discovery(request.params),
            "workspace/configuration" => self.handle_configuration(request.params),
            "workspace/didChangeWatchedFiles" => {
                self.handle_did_change_watched_files(request.params)
            }
            "workspace/willRenameFiles" => self.handle_will_rename_files(request.params),
            "workspace/didDeleteFiles" => self.handle_did_delete_files(request.params),
            "workspace/applyEdit" => self.handle_apply_edit(request.params),
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
        }

        // Check for available tools by trying to execute them
        let has_perltidy = std::process::Command::new("perltidy")
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        let has_perlcritic = std::process::Command::new("perlcritic")
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        eprintln!("Tool availability: perltidy={}, perlcritic={}", has_perltidy, has_perlcritic);

        // Check if incremental parsing is enabled
        let sync_kind =
            if cfg!(feature = "incremental") && std::env::var("PERL_LSP_INCREMENTAL").is_ok() {
                2 // Incremental sync
            } else {
                1 // Full document sync
            };

        let mut capabilities = json!({
            "positionEncoding": "utf-16",
            "textDocumentSync": {
                "openClose": true,
                "change": sync_kind, // Dynamic based on incremental feature
                "willSave": true,
                "willSaveWaitUntil": false, // Only enable when formatter is available
                "save": {
                    "includeText": true // Include text for robust save handling
                }
            },
            "completionProvider": {
                "triggerCharacters": ["$", "@", "%", ">", "-"], // covers '->'
                "allCommitCharacters": [";", " ", ")", "]", "}"]
            },
            "hoverProvider": true,
            "definitionProvider": true,
            "declarationProvider": true,
            "referencesProvider": true,
            "documentHighlightProvider": true,
            "typeHierarchyProvider": true,
            "signatureHelpProvider": {
                "triggerCharacters": ["(", ","]
            },
            "renameProvider": true,
            "documentSymbolProvider": true,
            "foldingRangeProvider": true,
            "codeActionProvider": true,
            "workspaceSymbolProvider": true,
            "codeLensProvider": {
                "resolveProvider": true
            },
            "semanticTokensProvider": {
                "legend": {
                    "tokenTypes": ["namespace", "class", "function", "method", "variable", "parameter", "property", "keyword", "comment", "string", "number", "regexp", "operator", "macro"],
                    "tokenModifiers": ["declaration", "definition", "reference", "modification", "static", "defaultLibrary", "async", "readonly", "deprecated"]
                },
                "full": true,
                "range": true
            },
            "callHierarchyProvider": true,
            "inlayHintProvider": {
                "resolveProvider": false
            },
            "documentLinkProvider": {
                "resolveProvider": false
            },
            "selectionRangeProvider": true,
            "documentOnTypeFormattingProvider": {
                "firstTriggerCharacter": "}",
                "moreTriggerCharacter": ["{", ";", ")", "\n"]
            },
            "executeCommandProvider": {
                "commands": [
                    "perl.runTest",
                    "perl.runTestFile",
                    "perl.runTests",
                    "perl.runFile",
                    "perl.runTestSub",
                    "perl.debugFile",
                    "perl.debugTests"
                ]
            },
            "experimental": {
                "testProvider": true
            },
            "positionEncoding": "utf-16"
        });

        // Only advertise formatting if perltidy is available
        if has_perltidy {
            capabilities["documentFormattingProvider"] = json!(true);
            capabilities["documentRangeFormattingProvider"] = json!(true);
        }

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
                // Parse the document
                let mut parser = Parser::new(text);
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

            // Store document state
            self.documents.lock().unwrap().insert(
                uri.to_string(),
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

    /// Handle didChange notification
    pub(crate) fn handle_did_change(&self, params: Option<Value>) -> Result<(), JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let version = params["textDocument"]["version"].as_i64().unwrap_or(0) as i32;

            if let Some(changes) = params["contentChanges"].as_array() {
                // Get current document state or create new one
                let mut documents = self.documents.lock().unwrap();
                let mut doc_state = documents.get(uri).cloned().unwrap_or_else(|| DocumentState {
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
                    // Parse the document
                    let mut parser = Parser::new(&text);
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
                if let Some(existing_doc) = documents.get(uri) {
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

                documents.insert(uri.to_string(), doc_state);

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
                // Get diagnostics
                let provider = DiagnosticsProvider::new(ast, doc.content.clone());
                let diagnostics = provider.get_diagnostics(ast, &doc.parse_errors, &doc.content);

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

            // Always send diagnostics notification with version
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
            if let Some(doc) = documents.get(uri) {
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
            if let Some(doc) = documents.get(uri) {
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
            if let Some(doc) = documents.get(uri) {
                let offset = self.pos16_to_offset(doc, line, character);

                // Get completions, with fallback for missing AST
                #[cfg_attr(not(feature = "workspace"), allow(unused_mut))]
                let mut completions = if let Some(ast) = &doc.ast {
                    // Get completions from the local completion provider
                    let provider = CompletionProvider::new(ast);
                    provider.get_completions(&doc.content, offset)
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
            if let Some(doc) = documents.get(uri) {
                if let Some(ast) = &doc.ast {
                    let start_offset = self.pos16_to_offset(doc, start_line, start_char);
                    let end_offset = self.pos16_to_offset(doc, end_line, end_char);

                    // Get diagnostics from the document
                    let diag_provider = DiagnosticsProvider::new(ast, doc.content.clone());
                    let diagnostics =
                        diag_provider.get_diagnostics(ast, &doc.parse_errors, &doc.content);

                    // Get code actions from both providers
                    let mut code_actions: Vec<Value> = Vec::new();

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
            if let Some(doc) = documents.get(uri) {
                if let Some(_ast) = &doc.ast {
                    let offset = self.pos16_to_offset(doc, line, character);

                    // For now, just show the token at position
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
            if let Some(doc) = documents.get(uri) {
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
        if let NodeKind::Subroutine { params: sub_params, body, .. } = &sub_node.kind {
            if !sub_params.is_empty() {
                // Extract parameter names from the params node
                for param in sub_params {
                    self.extract_params(param, &mut params);
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
            if let Some(doc) = documents.get(uri) {
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
            if let Some(doc) = documents.get(uri) {
                if let Some(ref ast) = doc.ast {
                    let offset = self.pos16_to_offset(doc, line, character);

                    // Try workspace index first for cross-file definitions
                    #[cfg(feature = "workspace")]
                    if let Some(ref workspace_index) = self.workspace_index {
                        // Get the symbol at the current position
                        let analyzer = crate::semantic::SemanticAnalyzer::analyze(ast);
                        let source_loc = crate::SourceLocation { start: offset, end: offset + 1 };
                        if let Some(symbol_info) = analyzer.symbol_at(source_loc) {
                            // Look for qualified name (e.g., Module::function)
                            let symbol_name = if symbol_info.name.contains("::") {
                                symbol_info.name.clone()
                            } else {
                                // Check if it's a method call or qualified reference
                                // TODO: Extract package context from analyzer
                                symbol_info.name.clone()
                            };

                            // Find definition in workspace
                            if let Some(def_location) =
                                workspace_index.find_definition(&symbol_name)
                            {
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
            if let Some(doc) = documents.get(uri) {
                if let Some(ref ast) = doc.ast {
                    let offset = self.pos16_to_offset(doc, line, character);

                    // Try workspace index first for cross-file references
                    #[cfg(feature = "workspace")]
                    if let Some(ref workspace_index) = self.workspace_index {
                        // Get the symbol at the current position
                        let analyzer = crate::semantic::SemanticAnalyzer::analyze(ast);
                        let source_loc = crate::SourceLocation { start: offset, end: offset + 1 };
                        if let Some(symbol_info) = analyzer.symbol_at(source_loc) {
                            // Look for qualified name (e.g., Module::function)
                            let symbol_name = if symbol_info.name.contains("::") {
                                symbol_info.name.clone()
                            } else {
                                // Check if it's a method call or qualified reference
                                // TODO: Extract package context from analyzer
                                symbol_info.name.clone()
                            };

                            // Find all references in workspace
                            let refs = workspace_index.find_references(&symbol_name);
                            if !refs.is_empty() {
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
            if let Some(doc) = documents.get(uri) {
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
            if let Some(doc) = documents.get(uri) {
                if let Some(ref ast) = doc.ast {
                    let offset = self.pos16_to_offset(doc, line, character);

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
            }
        }

        Ok(None)
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
            if let Some(doc) = documents.get(uri) {
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
            if let Some(doc) = documents.get(uri) {
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

    /// Handle textDocument/documentSymbol request
    fn handle_document_symbol(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");

            let documents = self.documents.lock().unwrap();
            if let Some(doc) = documents.get(uri) {
                if let Some(ref ast) = doc.ast {
                    // Extract symbols from AST
                    let extractor = crate::symbol::SymbolExtractor::new();
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
            if let Some(doc) = documents.get(uri) {
                if let Some(ref ast) = doc.ast {
                    // Extract folding ranges from AST
                    let mut extractor = crate::folding::FoldingRangeExtractor::new();
                    let ranges = extractor.extract(ast);

                    // Convert to LSP JSON format with proper line offsets
                    let mut lsp_ranges = Vec::new();
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

        for ch in content.chars() {
            if byte_pos >= offset {
                break;
            }

            match ch {
                '\r' => { /* ignore; CRLF will be handled on '\n' */ }
                '\n' => {
                    line += 1;
                    col_utf16 = 0;
                }
                _ => {
                    // Count UTF-16 code units (surrogate pairs count as 2)
                    col_utf16 += if ch.len_utf16() == 2 { 2 } else { 1 };
                }
            }

            byte_pos += ch.len_utf8();
        }

        (line, col_utf16)
    }

    /// Convert line/column position to offset (UTF-16 aware, CRLF safe)
    #[allow(deprecated)]
    pub fn position_to_offset(&self, content: &str, line: u32, character: u32) -> usize {
        let mut cur_line = 0u32;
        let mut col_utf16 = 0u32;
        let mut byte_pos = 0usize;

        for ch in content.chars() {
            if cur_line == line {
                match ch {
                    '\n' => {
                        // End of target line - clamp to EOL
                        return byte_pos.min(content.len());
                    }
                    '\r' => {
                        // ignore CR; CRLF handled on '\n'
                    }
                    _ => {
                        let w = if ch.len_utf16() == 2 { 2 } else { 1 };
                        if col_utf16 + w > character {
                            // Character position is in the middle of this char
                            return byte_pos;
                        }
                        if col_utf16 + w == character {
                            // Caret is after this char
                            return byte_pos + ch.len_utf8();
                        }
                        col_utf16 += w;
                    }
                }
            }

            match ch {
                '\n' => {
                    cur_line += 1;
                    if cur_line > line {
                        // We've gone past the target line
                        return byte_pos;
                    }
                    col_utf16 = 0;
                }
                '\r' => { /* ignore */ }
                _ => {}
            }

            byte_pos += ch.len_utf8();
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
            message: "ContentModified".to_string(),
            data: None,
        }
    }

    /// Ensure the request version matches the current document version
    fn ensure_latest(&self, uri: &str, req_version: Option<i32>) -> Result<(), JsonRpcError> {
        if let Some(v) = req_version {
            let documents = self.documents.lock().unwrap();
            if let Some(doc) = documents.get(uri) {
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
            if let Some(doc) = documents.get(uri) {
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
            if let Some(doc) = documents.get(uri) {
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

    /// Handle workspace/symbol request
    fn handle_workspace_symbols(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        let query =
            params.as_ref().and_then(|p| p.get("query")).and_then(|q| q.as_str()).unwrap_or("");

        eprintln!("Workspace symbol search: '{}'", query);

        // Use workspace index for all symbol searches
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

            // First deduplicate the internal symbols
            let deduped_symbols: Vec<WorkspaceSymbol> = index_symbols
                .into_iter()
                .filter(|sym| {
                    // Deduplicate by (uri, start position, name, kind)
                    seen.insert((
                        sym.uri.clone(),
                        sym.range.start.line,
                        sym.range.start.character,
                        sym.name.clone(),
                        sym.kind,
                    ))
                })
                .collect();

            // Convert to LSP DTOs (no internal fields)
            let lsp_symbols: Vec<LspWorkspaceSymbol> =
                deduped_symbols.iter().map(|sym| sym.into()).collect();

            eprintln!("Found {} symbols (after deduplication)", lsp_symbols.len());

            // Convert to JSON for LSP response
            // Now we're only serializing LSP-compliant fields
            let result = serde_json::to_value(&lsp_symbols).unwrap_or_else(|_| json!([]));

            Ok(Some(result))
        }

        #[cfg(not(feature = "workspace"))]
        {
            // Without workspace feature, return empty results
            Ok(Some(json!([])))
        }
    }

    /// Handle textDocument/codeLens request
    fn handle_code_lens(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");

            eprintln!("Getting code lenses for: {}", uri);

            let documents = self.documents.lock().unwrap();
            if let Some(doc) = documents.get(uri) {
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
            if let Some(doc) = documents.get(uri) {
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
            if let Some(doc) = documents.get(uri) {
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
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let position = &params["position"];
            let line = position["line"].as_u64().unwrap_or(0) as u32;
            let character = position["character"].as_u64().unwrap_or(0) as u32;

            eprintln!("Preparing call hierarchy at: {} ({}:{})", uri, line, character);

            let documents = self.documents.lock().unwrap();
            if let Some(doc) = documents.get(uri) {
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
            if let Some(doc) = documents.get(uri) {
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
            if let Some(doc) = documents.get(uri) {
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

    /// Handle inlay hint request
    fn handle_inlay_hint(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let range = &params["range"];

            eprintln!("Getting inlay hints for: {}", uri);

            let documents = self.documents.lock().unwrap();
            if let Some(doc) = documents.get(uri) {
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
                    let hints = provider.extract(ast);

                    // Filter hints to the requested range if needed
                    let start_line = range["start"]["line"].as_u64().unwrap_or(0) as u32;
                    let end_line = range["end"]["line"].as_u64().unwrap_or(u32::MAX as u64) as u32;

                    let filtered_hints: Vec<_> = hints
                        .into_iter()
                        .filter(|hint| {
                            hint.position.line >= start_line && hint.position.line <= end_line
                        })
                        .map(|hint| hint.to_json())
                        .collect();

                    eprintln!("Found {} inlay hints", filtered_hints.len());

                    return Ok(Some(json!(filtered_hints)));
                }
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
            if let Some(doc) = documents.get(uri) {
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
                        if let Some(doc) = documents.get_mut(&uri) {
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
                        if let Some(doc) = documents.get_mut(uri) {
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
            if let Some(doc) = documents.get(uri) {
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

    /// Handle selectionRange request
    fn handle_selection_range(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let positions = params["positions"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|pos| {
                            let line = pos["line"].as_u64()? as u32;
                            let character = pos["character"].as_u64()? as u32;
                            Some(lsp_types::Position::new(line, character))
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            let documents = self.documents.lock().unwrap();
            if let Some(doc) = documents.get(uri) {
                let ranges = crate::lsp_selection_range::selection_ranges(&doc.content, &positions);
                Ok(Some(serde_json::to_value(ranges).unwrap_or(Value::Null)))
            } else {
                Ok(Some(Value::Null))
            }
        } else {
            Ok(Some(Value::Null))
        }
    }

    /// Handle onTypeFormatting request
    fn handle_on_type_formatting(
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
            if let Some(doc) = documents.get(uri) {
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
    fn register_file_watchers(&self) {
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

        // Send the registration request
        let request = json!({
            "jsonrpc": "2.0",
            "id": serde_json::Value::Number(serde_json::Number::from(9999)),
            "method": "client/registerCapability",
            "params": params
        });

        // Print to stdout to send to client
        let msg = serde_json::to_string(&request).unwrap();
        print!("Content-Length: {}\r\n\r\n{}", msg.len(), msg);
        std::io::stdout().flush().unwrap();

        eprintln!("Registered file watchers for Perl files");
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

impl Default for LspServer {
    fn default() -> Self {
        Self::new()
    }
}
