//! Full JSON-RPC LSP Server implementation
//!
//! This module provides a complete Language Server Protocol implementation
//! that can be used with any LSP-compatible editor.

use crate::{
    Parser, 
    DiagnosticsProvider, DiagnosticSeverity as InternalDiagnosticSeverity,
    CodeActionsProvider, CodeActionKind as InternalCodeActionKind,
    CompletionProvider, CompletionItemKind,
    formatting::{CodeFormatter, FormattingOptions},
    workspace_symbols::WorkspaceSymbolsProvider,
    code_lens_provider::{CodeLensProvider, get_shebang_lens, resolve_code_lens},
    semantic_tokens_provider::{
        SemanticTokensProvider, 
        encode_semantic_tokens
    },
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::sync::{Arc, Mutex};

/// LSP server that handles JSON-RPC communication
pub struct LspServer {
    /// Document contents indexed by URI
    documents: Arc<Mutex<HashMap<String, DocumentState>>>,
    /// Whether the server is initialized
    initialized: bool,
    /// Workspace symbols provider
    workspace_symbols: Arc<Mutex<WorkspaceSymbolsProvider>>,
}

/// State of a document
#[derive(Clone)]
struct DocumentState {
    /// Document content
    content: String,
    /// Version number
    _version: i32,
    /// Parsed AST (cached)
    ast: Option<crate::ast::Node>,
    /// Parse errors
    parse_errors: Vec<crate::error::ParseError>,
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
struct JsonRpcError {
    code: i32,
    message: String,
    data: Option<Value>,
}

impl LspServer {
    /// Create a new LSP server
    pub fn new() -> Self {
        Self {
            documents: Arc::new(Mutex::new(HashMap::new())),
            initialized: false,
            workspace_symbols: Arc::new(Mutex::new(WorkspaceSymbolsProvider::new())),
        }
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
            if let Some(request) = self.read_message(&mut reader)? {
                eprintln!("Received request: {}", request.method);
                
                // Handle the request
                if let Some(response) = self.handle_request(request) {
                    // Send response
                    self.send_message(&mut stdout, &response)?;
                }
            }
        }
    }

    /// Read an LSP message from stdin
    fn read_message(&self, reader: &mut BufReader<io::StdinLock>) -> io::Result<Option<JsonRpcRequest>> {
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

    /// Send an LSP message to stdout
    fn send_message(&self, stdout: &mut io::StdoutLock, response: &JsonRpcResponse) -> io::Result<()> {
        let content = serde_json::to_string(response)?;
        let content_length = content.len();
        
        write!(stdout, "Content-Length: {}\r\n\r\n{}", content_length, content)?;
        stdout.flush()?;
        
        Ok(())
    }

    /// Handle a JSON-RPC request
    pub fn handle_request(&mut self, request: JsonRpcRequest) -> Option<JsonRpcResponse> {
        let id = request.id.clone();
        
        let result = match request.method.as_str() {
            "initialize" => self.handle_initialize(request.params),
            "initialized" => {
                self.initialized = true;
                eprintln!("Server initialized");
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
            "textDocument/codeAction" => self.handle_code_action(request.params),
            "textDocument/hover" => self.handle_hover(request.params),
            "textDocument/formatting" => self.handle_formatting(request.params),
            "textDocument/rangeFormatting" => self.handle_range_formatting(request.params),
            "workspace/symbol" => self.handle_workspace_symbols(request.params),
            "textDocument/codeLens" => self.handle_code_lens(request.params),
            "codeLens/resolve" => self.handle_code_lens_resolve(request.params),
            "textDocument/semanticTokens/full" => self.handle_semantic_tokens_full(request.params),
            "textDocument/semanticTokens/range" => self.handle_semantic_tokens_range(request.params),
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
                "textDocumentSync": 1,
                "completionProvider": {
                    "triggerCharacters": ["$", "@", "%", "->"]
                },
                "hoverProvider": true,
                "codeActionProvider": true,
                "documentFormattingProvider": true,
                "documentRangeFormattingProvider": true,
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
            },
            "serverInfo": {
                "name": "perl-language-server",
                "version": "0.1.0"
            }
        })))
    }

    /// Handle didOpen notification
    fn handle_did_open(&self, params: Option<Value>) -> Result<(), JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let text = params["textDocument"]["text"].as_str().unwrap_or("");
            let version = params["textDocument"]["version"].as_i64().unwrap_or(0) as i32;
            
            eprintln!("Document opened: {}", uri);
            
            // Parse the document
            let mut parser = Parser::new(text);
            let (ast, errors) = match parser.parse() {
                Ok(ast) => (Some(ast), vec![]),
                Err(e) => (None, vec![e]),
            };

            // Store document state
            self.documents.lock().unwrap().insert(
                uri.to_string(),
                DocumentState {
                    content: text.to_string(),
                    _version: version,
                    ast: ast.clone(),
                    parse_errors: errors,
                },
            );

            // Index symbols for workspace search
            if let Some(ref ast) = ast {
                self.workspace_symbols.lock().unwrap()
                    .index_document(uri, ast, text);
            }

            // Send diagnostics
            self.publish_diagnostics(uri);
        }

        Ok(())
    }

    /// Handle didChange notification
    fn handle_did_change(&self, params: Option<Value>) -> Result<(), JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let version = params["textDocument"]["version"].as_i64().unwrap_or(0) as i32;
            
            if let Some(changes) = params["contentChanges"].as_array() {
                if let Some(change) = changes.first() {
                    let text = change["text"].as_str().unwrap_or("");
                    
                    eprintln!("Document changed: {}", uri);
                    
                    // Parse the document
                    let mut parser = Parser::new(text);
                    let (ast, errors) = match parser.parse() {
                        Ok(ast) => (Some(ast), vec![]),
                        Err(e) => (None, vec![e]),
                    };

                    // Update document state
                    self.documents.lock().unwrap().insert(
                        uri.to_string(),
                        DocumentState {
                            content: text.to_string(),
                            _version: version,
                            ast: ast.clone(),
                            parse_errors: errors,
                        },
                    );

                    // Index symbols for workspace search
                    if let Some(ref ast) = ast {
                        self.workspace_symbols.lock().unwrap()
                            .index_document(uri, ast, text);
                    }

                    // Send diagnostics
                    self.publish_diagnostics(uri);
                }
            }
        }

        Ok(())
    }

    /// Publish diagnostics for a document
    fn publish_diagnostics(&self, uri: &str) {
        let documents = self.documents.lock().unwrap();
        if let Some(doc) = documents.get(uri) {
            if let Some(ast) = &doc.ast {
                // Get diagnostics
                let provider = DiagnosticsProvider::new(ast, doc.content.clone());
                let diagnostics = provider.get_diagnostics(ast, &doc.parse_errors);

                // Convert to LSP diagnostics
                let lsp_diagnostics: Vec<Value> = diagnostics
                    .into_iter()
                    .map(|d| {
                        let (start_line, start_char) = self.offset_to_position(&doc.content, d.range.0);
                        let (end_line, end_char) = self.offset_to_position(&doc.content, d.range.1);
                        
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
                    .collect();

                eprintln!("Publishing {} diagnostics for {}", lsp_diagnostics.len(), uri);
                
                // TODO: Send notification via stdout
            }
        }
    }

    /// Handle completion request
    fn handle_completion(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let line = params["position"]["line"].as_u64().unwrap_or(0) as u32;
            let character = params["position"]["character"].as_u64().unwrap_or(0) as u32;
            
            let documents = self.documents.lock().unwrap();
            if let Some(doc) = documents.get(uri) {
                if let Some(ast) = &doc.ast {
                    let offset = self.position_to_offset(&doc.content, line, character);
                    
                    let provider = CompletionProvider::new(ast);
                    let completions = provider.get_completions(&doc.content, offset);

                    let items: Vec<Value> = completions
                        .into_iter()
                        .map(|c| json!({
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
                            "insertTextFormat": 1,
                        }))
                        .collect();

                    eprintln!("Returning {} completions", items.len());
                    return Ok(Some(json!({"items": items})));
                }
            }
        }

        Ok(Some(json!({"items": []})))
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
                    let start_offset = self.position_to_offset(&doc.content, start_line, start_char);
                    let end_offset = self.position_to_offset(&doc.content, end_line, end_char);
                    
                    // Get diagnostics from the document
                    let diag_provider = DiagnosticsProvider::new(ast, doc.content.clone());
                    let diagnostics = diag_provider.get_diagnostics(ast, &doc.parse_errors);
                    
                    // Get code actions
                    let provider = CodeActionsProvider::new(doc.content.clone());
                    let actions = provider.get_code_actions(ast, (start_offset, end_offset), &diagnostics);

                    let code_actions: Vec<Value> = actions
                        .into_iter()
                        .map(|action| {
                            let mut changes = HashMap::new();
                            let edits: Vec<Value> = action.edit.changes
                                .into_iter()
                                .map(|edit| {
                                    let (start_line, start_char) = self.offset_to_position(&doc.content, edit.location.start);
                                    let (end_line, end_char) = self.offset_to_position(&doc.content, edit.location.end);
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

                            json!({
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
                            })
                        })
                        .collect();

                    eprintln!("Returning {} code actions", code_actions.len());
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
            
            let documents = self.documents.lock().unwrap();
            if let Some(doc) = documents.get(uri) {
                if let Some(_ast) = &doc.ast {
                    let offset = self.position_to_offset(&doc.content, line, character);
                    
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

    /// Get token at position (simple implementation)
    fn get_token_at_position(&self, content: &str, offset: usize) -> String {
        let chars: Vec<char> = content.chars().collect();
        if offset >= chars.len() {
            return String::new();
        }
        
        // Find word boundaries
        let mut start = offset;
        while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_' || chars[start - 1] == '$' || chars[start - 1] == '@' || chars[start - 1] == '%') {
            start -= 1;
        }
        
        let mut end = offset;
        while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
            end += 1;
        }
        
        chars[start..end].iter().collect()
    }

    /// Convert offset to line/column position
    fn offset_to_position(&self, content: &str, offset: usize) -> (u32, u32) {
        let mut line = 0;
        let mut col = 0;
        
        for (i, ch) in content.chars().enumerate() {
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

    /// Convert line/column position to offset
    fn position_to_offset(&self, content: &str, line: u32, character: u32) -> usize {
        let mut current_line = 0;
        let mut current_col = 0;
        
        for (i, ch) in content.chars().enumerate() {
            if current_line == line && current_col == character {
                return i;
            }
            if ch == '\n' {
                current_line += 1;
                current_col = 0;
            } else {
                current_col += 1;
            }
        }
        
        content.len()
    }
    
    /// Handle textDocument/formatting request
    fn handle_formatting(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
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
            
            eprintln!("Formatting document: {}", uri);
            
            let documents = self.documents.lock().unwrap();
            if let Some(doc) = documents.get(uri) {
                let formatter = CodeFormatter::new();
                match formatter.format_document(&doc.content, &options) {
                    Ok(edits) => {
                        let lsp_edits: Vec<Value> = edits.into_iter()
                            .map(|edit| json!({
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
                            }))
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
    fn handle_range_formatting(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
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
                        let lsp_edits: Vec<Value> = edits.into_iter()
                            .map(|edit| json!({
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
                            }))
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
    fn handle_workspace_symbols(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        let query = params
            .as_ref()
            .and_then(|p| p.get("query"))
            .and_then(|q| q.as_str())
            .unwrap_or("");
        
        eprintln!("Workspace symbol search: '{}'", query);
        
        // Get all document sources for offset conversion
        let documents = self.documents.lock().unwrap();
        let mut source_map = HashMap::new();
        for (uri, doc) in documents.iter() {
            source_map.insert(uri.clone(), doc.content.clone());
        }
        drop(documents);
        
        // Search for symbols
        let symbols = self.workspace_symbols.lock().unwrap()
            .search(query, &source_map);
        
        eprintln!("Found {} symbols", symbols.len());
        
        Ok(Some(json!(symbols)))
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
    fn handle_code_lens_resolve(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            // Parse the code lens
            if let Ok(lens) = serde_json::from_value::<crate::code_lens_provider::CodeLens>(params.clone()) {
                // For now, just resolve with a placeholder count
                // In a real implementation, you'd count actual references
                let reference_count = 0; // TODO: Count actual references
                
                let resolved = resolve_code_lens(lens, reference_count);
                return Ok(Some(json!(resolved)));
            }
        }
        
        Err(JsonRpcError {
            code: -32602,
            message: "Invalid parameters".to_string(),
            data: None,
        })
    }

    /// Handle semantic tokens full request
    fn handle_semantic_tokens_full(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
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
    fn handle_semantic_tokens_range(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let range = &params["range"];
            let start_line = range["start"]["line"].as_u64().unwrap_or(0) as u32;
            let end_line = range["end"]["line"].as_u64().unwrap_or(u32::MAX as u64) as u32;
            
            eprintln!("Getting semantic tokens for range: {} (lines {}-{})", uri, start_line, end_line);
            
            let documents = self.documents.lock().unwrap();
            if let Some(doc) = documents.get(uri) {
                if let Some(ref ast) = doc.ast {
                    let mut provider = SemanticTokensProvider::new(doc.content.clone());
                    let all_tokens = provider.extract(ast);
                    
                    // Filter tokens to the requested range
                    let range_tokens: Vec<_> = all_tokens.into_iter()
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
}

impl Default for LspServer {
    fn default() -> Self {
        Self::new()
    }
}