//! LSP Server with integrated incremental parsing for <1ms updates
//!
//! This enhanced version of the LSP server uses IncrementalDocument
//! to achieve blazing-fast incremental updates through subtree reuse.

use crate::{
    ast::Node,
    builtin_signatures::BUILTIN_SIGNATURES,
    code_actions::CodeActionProvider,
    code_actions_enhanced::EnhancedCodeActionProvider,
    completion::CompletionProvider,
    diagnostics::DiagnosticsProvider,
    edit::{Edit, EditSet},
    error::{JsonRpcError, ParseError},
    execute_command::CommandExecutor,
    formatting::FormattingProvider,
    incremental_document::{IncrementalDocument, ParseMetrics},
    parser::Parser,
    performance::AstCache,
    position::Range,
    semantic_tokens::SemanticTokensProvider,
    symbols::{DocumentSymbolProvider, SymbolIndex, WorkspaceSymbolProvider},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Write};
use std::sync::{Arc, Mutex};

/// Enhanced LSP server with incremental parsing
pub struct IncrementalLspServer {
    /// Documents managed with incremental parsing
    documents: Arc<Mutex<HashMap<String, IncrementalDocumentState>>>,
    /// Symbol index for workspace-wide operations
    symbol_index: Arc<Mutex<SymbolIndex>>,
    /// Configuration
    config: ServerConfig,
    /// Performance metrics
    metrics: Arc<Mutex<ServerMetrics>>,
    /// Initialization state
    initialized: bool,
}

/// State of a document with incremental parsing
struct IncrementalDocumentState {
    /// Incremental document with subtree reuse
    document: IncrementalDocument,
    /// URI of the document
    uri: String,
    /// Language ID
    language_id: String,
}

/// Server configuration
#[derive(Clone)]
struct ServerConfig {
    /// Enable incremental parsing
    incremental_parsing: bool,
    /// Maximum subtree cache size
    max_cache_size: usize,
    /// Enable performance logging
    log_performance: bool,
    /// Target parse time in milliseconds
    target_parse_time_ms: f64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            incremental_parsing: true,
            max_cache_size: 10000,
            log_performance: true,
            target_parse_time_ms: 1.0, // Target <1ms updates
        }
    }
}

/// Server-wide performance metrics
#[derive(Default)]
struct ServerMetrics {
    /// Total incremental parses
    total_incremental_parses: u64,
    /// Total full parses
    total_full_parses: u64,
    /// Average parse time (ms)
    avg_parse_time_ms: f64,
    /// Best parse time (ms)
    best_parse_time_ms: f64,
    /// Worst parse time (ms)
    worst_parse_time_ms: f64,
    /// Total nodes reused
    total_nodes_reused: usize,
    /// Total nodes reparsed
    total_nodes_reparsed: usize,
}

impl IncrementalLspServer {
    /// Create a new incremental LSP server
    pub fn new() -> Self {
        IncrementalLspServer {
            documents: Arc::new(Mutex::new(HashMap::new())),
            symbol_index: Arc::new(Mutex::new(SymbolIndex::new())),
            config: ServerConfig::default(),
            metrics: Arc::new(Mutex::new(ServerMetrics::default())),
            initialized: false,
        }
    }
    
    /// Run the LSP server
    pub fn run(&mut self) -> io::Result<()> {
        let stdin = io::stdin();
        let stdout = io::stdout();
        let mut reader = BufReader::new(stdin);
        
        eprintln!("Incremental Perl LSP server starting...");
        
        loop {
            // Read LSP message
            let mut headers = HashMap::new();
            loop {
                let mut line = String::new();
                reader.read_line(&mut line)?;
                
                if line == "\r\n" || line == "\n" {
                    break;
                }
                
                if let Some(colon_pos) = line.find(':') {
                    let name = line[..colon_pos].trim().to_lowercase();
                    let value = line[colon_pos + 1..].trim();
                    headers.insert(name, value.to_string());
                }
            }
            
            // Parse content length
            let content_length = headers.get("content-length")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0);
            
            if content_length == 0 {
                continue;
            }
            
            // Read message body
            let mut body = vec![0; content_length];
            reader.read_exact(&mut body)?;
            
            // Parse JSON-RPC request
            let request_str = String::from_utf8_lossy(&body);
            if let Ok(request) = serde_json::from_str::<JsonRpcRequest>(&request_str) {
                // Handle the request
                if let Some(response) = self.handle_request(request) {
                    let response_str = serde_json::to_string(&response)?;
                    let mut stdout = stdout.lock();
                    write!(stdout, "Content-Length: {}\r\n\r\n{}", 
                           response_str.len(), response_str)?;
                    stdout.flush()?;
                }
            }
        }
    }
    
    /// Handle a JSON-RPC request
    fn handle_request(&mut self, request: JsonRpcRequest) -> Option<JsonRpcResponse> {
        let id = request.id.clone();
        
        let result = match request.method.as_str() {
            "initialize" => self.handle_initialize(request.params),
            "initialized" => {
                self.initialized = true;
                eprintln!("Incremental server initialized");
                Ok(None)
            }
            "shutdown" => Ok(Some(json!(null))),
            "textDocument/didOpen" => {
                match self.handle_did_open(request.params) {
                    Ok(_) => Ok(None),
                    Err(e) => Err(e),
                }
            }
            "textDocument/didChange" => {
                match self.handle_did_change(request.params) {
                    Ok(_) => Ok(None),
                    Err(e) => Err(e),
                }
            }
            "textDocument/completion" => self.handle_completion(request.params),
            "textDocument/hover" => self.handle_hover(request.params),
            "textDocument/definition" => self.handle_definition(request.params),
            "textDocument/references" => self.handle_references(request.params),
            "textDocument/documentSymbol" => self.handle_document_symbol(request.params),
            "textDocument/codeAction" => self.handle_code_action(request.params),
            "textDocument/rename" => self.handle_rename(request.params),
            "workspace/executeCommand" => self.handle_execute_command(request.params),
            _ => {
                eprintln!("Method not implemented: {}", request.method);
                Err(JsonRpcError {
                    code: -32601,
                    message: "Method not found".to_string(),
                    data: None,
                })
            }
        };
        
        match result {
            Ok(Some(result)) => Some(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(result),
                error: None,
            }),
            Ok(None) => None, // Notification, no response
            Err(error) => Some(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(error),
            }),
        }
    }
    
    /// Handle initialize request
    fn handle_initialize(&self, _params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        Ok(Some(json!({
            "capabilities": {
                "textDocumentSync": {
                    "openClose": true,
                    "change": 2, // Incremental sync
                    "save": true
                },
                "completionProvider": {
                    "triggerCharacters": ["$", "@", "%", "->"]
                },
                "hoverProvider": true,
                "definitionProvider": true,
                "referencesProvider": true,
                "documentSymbolProvider": true,
                "codeActionProvider": true,
                "renameProvider": {
                    "prepareProvider": true
                },
                "executeCommandProvider": {
                    "commands": ["perl.extractVariable", "perl.extractSubroutine"]
                },
                "experimental": {
                    "incrementalParsing": true,
                    "subtreeReuse": true,
                    "targetParseTime": "<1ms"
                }
            },
            "serverInfo": {
                "name": "perl-incremental-lsp",
                "version": "0.8.0"
            }
        })))
    }
    
    /// Handle document open
    fn handle_did_open(&mut self, params: Option<Value>) -> Result<(), JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let text = params["textDocument"]["text"].as_str().unwrap_or("");
            let language_id = params["textDocument"]["languageId"].as_str().unwrap_or("perl");
            
            eprintln!("Opening document with incremental parsing: {}", uri);
            
            // Create incremental document
            match IncrementalDocument::new(text.to_string()) {
                Ok(document) => {
                    // Log initial parse metrics
                    if self.config.log_performance {
                        let metrics = document.metrics();
                        eprintln!("Initial parse: {:.2}ms, {} nodes", 
                                metrics.last_parse_time_ms,
                                metrics.nodes_reparsed);
                    }
                    
                    // Update server metrics
                    self.update_metrics(&document.metrics(), true);
                    
                    // Store document
                    let mut documents = self.documents.lock().unwrap();
                    documents.insert(uri.to_string(), IncrementalDocumentState {
                        document,
                        uri: uri.to_string(),
                        language_id: language_id.to_string(),
                    });
                    
                    // Send diagnostics
                    self.send_diagnostics(uri, &[]);
                }
                Err(e) => {
                    eprintln!("Parse error: {}", e);
                    self.send_diagnostics(uri, &[e]);
                }
            }
        }
        
        Ok(())
    }
    
    /// Handle document change with incremental parsing
    fn handle_did_change(&mut self, params: Option<Value>) -> Result<(), JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let changes = params["contentChanges"].as_array();
            
            if let Some(changes) = changes {
                let mut documents = self.documents.lock().unwrap();
                
                if let Some(doc_state) = documents.get_mut(uri) {
                    // Convert LSP changes to edits
                    let mut edit_set = EditSet::new();
                    
                    for change in changes {
                        if let Some(range) = change["range"].as_object() {
                            // Incremental change
                            let start = self.position_to_offset(&doc_state.document.source, range["start"].as_object());
                            let end = self.position_to_offset(&doc_state.document.source, range["end"].as_object());
                            let text = change["text"].as_str().unwrap_or("");
                            
                            edit_set.add(Edit {
                                start_byte: start,
                                old_end_byte: end,
                                new_text: text.to_string(),
                            });
                        } else {
                            // Full document update (fallback)
                            let text = change["text"].as_str().unwrap_or("");
                            match IncrementalDocument::new(text.to_string()) {
                                Ok(new_doc) => {
                                    doc_state.document = new_doc;
                                    self.update_metrics(&doc_state.document.metrics(), true);
                                }
                                Err(e) => {
                                    eprintln!("Parse error: {}", e);
                                    self.send_diagnostics(uri, &[e]);
                                    return Ok(());
                                }
                            }
                        }
                    }
                    
                    // Apply incremental edits
                    if !edit_set.edits.is_empty() {
                        let start = std::time::Instant::now();
                        
                        if let Err(e) = doc_state.document.apply_edits(&edit_set) {
                            eprintln!("Incremental parse error: {}", e);
                            self.send_diagnostics(uri, &[e]);
                            return Ok(());
                        }
                        
                        let parse_time = start.elapsed().as_secs_f64() * 1000.0;
                        
                        // Log performance
                        if self.config.log_performance {
                            let metrics = doc_state.document.metrics();
                            eprintln!("Incremental update: {:.2}ms ({} reused, {} reparsed)", 
                                    parse_time,
                                    metrics.nodes_reused,
                                    metrics.nodes_reparsed);
                            
                            if parse_time > self.config.target_parse_time_ms {
                                eprintln!("⚠️ Parse time exceeded target ({:.2}ms > {:.2}ms)",
                                        parse_time, self.config.target_parse_time_ms);
                            } else {
                                eprintln!("✅ Parse time within target ({:.2}ms <= {:.2}ms)",
                                        parse_time, self.config.target_parse_time_ms);
                            }
                        }
                        
                        // Update metrics
                        self.update_metrics(&doc_state.document.metrics(), false);
                    }
                    
                    // Send diagnostics
                    self.send_diagnostics(uri, &[]);
                }
            }
        }
        
        Ok(())
    }
    
    /// Handle completion request
    fn handle_completion(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let position = params["position"].as_object();
            
            let documents = self.documents.lock().unwrap();
            if let Some(doc_state) = documents.get(uri) {
                if let Some(pos) = position {
                    let offset = self.position_to_offset(&doc_state.document.source, pos);
                    
                    // Use the incremental AST for completion
                    let provider = CompletionProvider::new();
                    let items = provider.get_completions(
                        doc_state.document.tree(),
                        &doc_state.document.source,
                        offset
                    );
                    
                    return Ok(Some(json!({
                        "isIncomplete": false,
                        "items": items
                    })));
                }
            }
        }
        
        Ok(Some(json!({
            "isIncomplete": false,
            "items": []
        })))
    }
    
    /// Handle hover request
    fn handle_hover(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let position = params["position"].as_object();
            
            let documents = self.documents.lock().unwrap();
            if let Some(doc_state) = documents.get(uri) {
                if let Some(pos) = position {
                    let offset = self.position_to_offset(&doc_state.document.source, pos);
                    
                    // Find node at position using incremental tree
                    if let Some(info) = self.get_hover_info(doc_state.document.tree(), &doc_state.document.source, offset) {
                        return Ok(Some(json!({
                            "contents": {
                                "kind": "markdown",
                                "value": info
                            }
                        })));
                    }
                }
            }
        }
        
        Ok(None)
    }
    
    /// Handle go to definition
    fn handle_definition(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let position = params["position"].as_object();
            
            let documents = self.documents.lock().unwrap();
            if let Some(doc_state) = documents.get(uri) {
                if let Some(pos) = position {
                    let offset = self.position_to_offset(&doc_state.document.source, pos);
                    
                    // Find definition using incremental tree
                    if let Some(location) = self.find_definition(doc_state.document.tree(), &doc_state.document.source, offset) {
                        return Ok(Some(json!({
                            "uri": uri,
                            "range": {
                                "start": self.offset_to_position(&doc_state.document.source, location.start),
                                "end": self.offset_to_position(&doc_state.document.source, location.end)
                            }
                        })));
                    }
                }
            }
        }
        
        Ok(None)
    }
    
    /// Handle find references
    fn handle_references(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let position = params["position"].as_object();
            
            let documents = self.documents.lock().unwrap();
            if let Some(doc_state) = documents.get(uri) {
                if let Some(pos) = position {
                    let offset = self.position_to_offset(&doc_state.document.source, pos);
                    
                    // Find all references using incremental tree
                    let references = self.find_references(doc_state.document.tree(), &doc_state.document.source, offset);
                    
                    let locations: Vec<Value> = references.into_iter().map(|loc| {
                        json!({
                            "uri": uri,
                            "range": {
                                "start": self.offset_to_position(&doc_state.document.source, loc.start),
                                "end": self.offset_to_position(&doc_state.document.source, loc.end)
                            }
                        })
                    }).collect();
                    
                    return Ok(Some(json!(locations)));
                }
            }
        }
        
        Ok(Some(json!([])))
    }
    
    /// Handle document symbols
    fn handle_document_symbol(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            
            let documents = self.documents.lock().unwrap();
            if let Some(doc_state) = documents.get(uri) {
                let provider = DocumentSymbolProvider::new();
                let symbols = provider.get_symbols(doc_state.document.tree());
                
                return Ok(Some(json!(symbols)));
            }
        }
        
        Ok(Some(json!([])))
    }
    
    /// Handle code actions
    fn handle_code_action(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let range = params["range"].as_object();
            
            let documents = self.documents.lock().unwrap();
            if let Some(doc_state) = documents.get(uri) {
                if let Some(range) = range {
                    let start = self.position_to_offset(&doc_state.document.source, range["start"].as_object());
                    let end = self.position_to_offset(&doc_state.document.source, range["end"].as_object());
                    
                    // Get code actions using incremental tree
                    let provider = EnhancedCodeActionProvider::new();
                    let actions = provider.get_code_actions(
                        doc_state.document.tree(),
                        &doc_state.document.source,
                        Range { start, end }
                    );
                    
                    return Ok(Some(json!(actions)));
                }
            }
        }
        
        Ok(Some(json!([])))
    }
    
    /// Handle rename
    fn handle_rename(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let position = params["position"].as_object();
            let new_name = params["newName"].as_str().unwrap_or("");
            
            let documents = self.documents.lock().unwrap();
            if let Some(doc_state) = documents.get(uri) {
                if let Some(pos) = position {
                    let offset = self.position_to_offset(&doc_state.document.source, pos);
                    
                    // Find all occurrences to rename
                    let edits = self.get_rename_edits(
                        doc_state.document.tree(),
                        &doc_state.document.source,
                        offset,
                        new_name
                    );
                    
                    if !edits.is_empty() {
                        return Ok(Some(json!({
                            "documentChanges": [{
                                "textDocument": {
                                    "uri": uri,
                                    "version": doc_state.document.version
                                },
                                "edits": edits
                            }]
                        })));
                    }
                }
            }
        }
        
        Ok(None)
    }
    
    /// Handle execute command
    fn handle_execute_command(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let command = params["command"].as_str().unwrap_or("");
            let arguments = params["arguments"].as_array();
            
            let executor = CommandExecutor::new();
            return executor.execute(command, arguments);
        }
        
        Ok(None)
    }
    
    // Helper methods
    
    fn position_to_offset(&self, source: &str, position: Option<&serde_json::Map<String, Value>>) -> usize {
        if let Some(pos) = position {
            let line = pos["line"].as_u64().unwrap_or(0) as usize;
            let character = pos["character"].as_u64().unwrap_or(0) as usize;
            
            let mut offset = 0;
            for (i, line_text) in source.lines().enumerate() {
                if i == line {
                    return offset + character.min(line_text.len());
                }
                offset += line_text.len() + 1; // +1 for newline
            }
        }
        0
    }
    
    fn offset_to_position(&self, source: &str, offset: usize) -> Value {
        let mut line = 0;
        let mut character = 0;
        let mut current_offset = 0;
        
        for line_text in source.lines() {
            if current_offset + line_text.len() >= offset {
                character = offset - current_offset;
                break;
            }
            current_offset += line_text.len() + 1;
            line += 1;
        }
        
        json!({
            "line": line,
            "character": character
        })
    }
    
    fn send_diagnostics(&self, uri: &str, errors: &[ParseError]) {
        let diagnostics: Vec<Value> = errors.iter().map(|e| {
            json!({
                "range": {
                    "start": {"line": 0, "character": 0},
                    "end": {"line": 0, "character": 0}
                },
                "severity": 1,
                "message": e.to_string()
            })
        }).collect();
        
        let notification = json!({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": {
                "uri": uri,
                "diagnostics": diagnostics
            }
        });
        
        // Send notification (would need output channel in real implementation)
        eprintln!("Diagnostics: {}", serde_json::to_string(&notification).unwrap());
    }
    
    fn update_metrics(&mut self, parse_metrics: &ParseMetrics, is_full_parse: bool) {
        let mut metrics = self.metrics.lock().unwrap();
        
        if is_full_parse {
            metrics.total_full_parses += 1;
        } else {
            metrics.total_incremental_parses += 1;
        }
        
        // Update average parse time
        let total_parses = metrics.total_full_parses + metrics.total_incremental_parses;
        metrics.avg_parse_time_ms = 
            (metrics.avg_parse_time_ms * (total_parses - 1) as f64 + parse_metrics.last_parse_time_ms) 
            / total_parses as f64;
        
        // Update best/worst times
        if metrics.best_parse_time_ms == 0.0 || parse_metrics.last_parse_time_ms < metrics.best_parse_time_ms {
            metrics.best_parse_time_ms = parse_metrics.last_parse_time_ms;
        }
        if parse_metrics.last_parse_time_ms > metrics.worst_parse_time_ms {
            metrics.worst_parse_time_ms = parse_metrics.last_parse_time_ms;
        }
        
        // Update node counts
        metrics.total_nodes_reused += parse_metrics.nodes_reused;
        metrics.total_nodes_reparsed += parse_metrics.nodes_reparsed;
        
        // Log summary periodically
        if total_parses % 100 == 0 {
            eprintln!("=== Performance Summary ===");
            eprintln!("Total parses: {} (incremental: {}, full: {})",
                    total_parses, metrics.total_incremental_parses, metrics.total_full_parses);
            eprintln!("Parse times: avg={:.2}ms, best={:.2}ms, worst={:.2}ms",
                    metrics.avg_parse_time_ms, metrics.best_parse_time_ms, metrics.worst_parse_time_ms);
            eprintln!("Node reuse: {} reused, {} reparsed ({:.1}% reuse rate)",
                    metrics.total_nodes_reused, metrics.total_nodes_reparsed,
                    100.0 * metrics.total_nodes_reused as f64 / 
                    (metrics.total_nodes_reused + metrics.total_nodes_reparsed) as f64);
        }
    }
    
    // Stub implementations for helper methods (would be fully implemented)
    
    fn get_hover_info(&self, _ast: &Node, _source: &str, _offset: usize) -> Option<String> {
        // Implementation would analyze AST and return hover info
        Some("Incremental parsing enabled".to_string())
    }
    
    fn find_definition(&self, _ast: &Node, _source: &str, _offset: usize) -> Option<crate::ast::SourceLocation> {
        // Implementation would find definition location
        None
    }
    
    fn find_references(&self, _ast: &Node, _source: &str, _offset: usize) -> Vec<crate::ast::SourceLocation> {
        // Implementation would find all references
        Vec::new()
    }
    
    fn get_rename_edits(&self, _ast: &Node, _source: &str, _offset: usize, _new_name: &str) -> Vec<Value> {
        // Implementation would generate rename edits
        Vec::new()
    }
}

/// JSON-RPC request
#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
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