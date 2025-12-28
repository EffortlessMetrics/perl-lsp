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
    inlay_hints_provider::{InlayHintConfig, InlayHintsProvider},
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
                    let diag_provider = DiagnosticsProvider::new(ast, doc.text.clone());
                    let diagnostics =
                        diag_provider.get_diagnostics(ast, &doc.parse_errors, &doc.text);

                    // Get code actions from both providers
                    let mut code_actions: Vec<Value> = Vec::new();

                    // Add Perl::Critic quick fixes
                    let builtin_analyzer = BuiltInAnalyzer::new();
                    let violations = builtin_analyzer.analyze(ast, &doc.text);
                    for violation in &violations {
                        if let Some(quick_fix) =
                            builtin_analyzer.get_quick_fix(violation, &doc.text)
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
                    let provider_v2 = CodeActionsProviderV2::new(doc.text.clone());
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
                    let provider = CodeActionsProvider::new(doc.text.clone());
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
                                InternalCodeActionKind::RefactorInline => "refactor.inline",
                                InternalCodeActionKind::RefactorRewrite => "refactor.rewrite",
                                InternalCodeActionKind::Source => "source",
                                InternalCodeActionKind::SourceOrganizeImports => "source.organizeImports",
                                InternalCodeActionKind::SourceFixAll => "source.fixAll",
                            },
                            "edit": {
                                "changes": changes,
                            },
                        }));
                    }

                    // Get enhanced refactorings (extract variable, convert loops, etc.)
                    let enhanced_provider = EnhancedCodeActionsProvider::new(doc.text.clone());
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
                                InternalCodeActionKind::RefactorInline => "refactor.inline",
                                InternalCodeActionKind::RefactorRewrite => "refactor.rewrite",
                                InternalCodeActionKind::Source => "source",
                                InternalCodeActionKind::SourceOrganizeImports => "source.organizeImports",
                                InternalCodeActionKind::SourceFixAll => "source.fixAll",
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
                    if !doc.text.contains("use strict") || !doc.text.contains("use warnings") {
                        let mut changes = HashMap::new();
                        // Find first non-shebang line
                        let insert_pos = if doc.text.starts_with("#!") {
                            doc.text.find('\n').map(|p| p + 1).unwrap_or(0)
                        } else {
                            0
                        };

                        let new_text = if !doc.text.contains("use strict")
                            && !doc.text.contains("use warnings")
                        {
                            "use strict;\nuse warnings;\n\n"
                        } else if !doc.text.contains("use strict") {
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
                        if re.is_match(&doc.text) {
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

                            let mut workspace_locations: Vec<Value> = Vec::new();
                            if !all_refs.is_empty() {
                                eprintln!("Found {} references via find_refs", all_refs.len());
                                // Convert internal Locations to LSP Locations
                                let lsp_locations =
                                    crate::workspace_index::lsp_adapter::to_lsp_locations(all_refs);
                                for loc in lsp_locations {
                                    workspace_locations.push(json!(loc));
                                }
                            }

                            // Enhanced fallback: always search for both qualified and unqualified references
                            let docs_snapshot: Vec<(String, DocumentState)> =
                                documents.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

                            let mut enhanced_locations = Vec::new();
                            let symbol_name = &symbol_key.name;
                            let package_name = &symbol_key.pkg;

                            // Search patterns: both "symbol_name" and "package::symbol_name"
                            let patterns = vec![
                                format!(r"\b{}\b", regex::escape(symbol_name)),
                                format!(
                                    r"\b{}::{}\b",
                                    regex::escape(package_name),
                                    regex::escape(symbol_name)
                                ),
                            ];

                            for pattern in patterns {
                                if let Ok(search_regex) = regex::Regex::new(&pattern) {
                                    for (doc_uri, doc_state) in &docs_snapshot {
                                        let lines: Vec<&str> = doc_state.text.lines().collect();
                                        for (line_num, line) in lines.iter().enumerate() {
                                            for mat in search_regex.find_iter(line) {
                                                enhanced_locations.push(json!({
                                                    "uri": doc_uri,
                                                    "range": {
                                                        "start": {
                                                            "line": line_num,
                                                            "character": mat.start(),
                                                        },
                                                        "end": {
                                                            "line": line_num,
                                                            "character": mat.end(),
                                                        },
                                                    },
                                                }));
                                            }
                                        }
                                    }
                                }
                            }

                            // Combine workspace index results with text search results
                            workspace_locations.extend(enhanced_locations);
                            let all_combined_locations = workspace_locations;

                            if !all_combined_locations.is_empty() {
                                eprintln!(
                                    "Found {} total references via combined search",
                                    all_combined_locations.len()
                                );
                                return Ok(Some(json!(all_combined_locations)));
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

                        // Regex-based fallback for fully-qualified symbols like Package::sub references
                        let radius = 50;
                        let text_start = offset.saturating_sub(radius);
                        let text_around = self.get_text_around_offset(&doc.text, offset, radius);
                        let cursor_in_text = offset - text_start;

                        let re = regex::Regex::new(
                            r"([A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*)",
                        )
                        .unwrap();

                        for cap in re.captures_iter(&text_around) {
                            if let Some(m) = cap.get(1) {
                                if cursor_in_text >= m.start() && cursor_in_text <= m.end() {
                                    let parts: Vec<&str> = m.as_str().split("::").collect();
                                    if parts.len() >= 2 {
                                        let name = parts.last().unwrap().to_string();
                                        let pkg = parts[..parts.len() - 1].join("::");
                                        let key = crate::workspace_index::SymbolKey {
                                            pkg: pkg.clone().into(),
                                            name: name.clone().into(),
                                            sigil: None,
                                            kind: crate::workspace_index::SymKind::Sub,
                                        };

                                        if let Some(ref workspace_index) = self.workspace_index {
                                            // Search for all references to this qualified symbol
                                            let mut all_refs = Vec::new();

                                            // Find references via symbol key
                                            let refs = workspace_index.find_refs(&key);
                                            all_refs.extend(refs);

                                            // Also try with qualified name
                                            let symbol_name = format!("{}::{}", pkg, name);
                                            let alt_refs =
                                                workspace_index.find_references(&symbol_name);
                                            all_refs.extend(alt_refs);

                                            // Add definition if includeDeclaration is true
                                            if include_declaration {
                                                if let Some(def) = workspace_index.find_def(&key) {
                                                    all_refs.push(def);
                                                }
                                            }

                                            if !all_refs.is_empty() {
                                                // Convert internal Locations to LSP Locations
                                                let lsp_locations =
                                                    crate::workspace_index::lsp_adapter::to_lsp_locations(all_refs);
                                                if !lsp_locations.is_empty() {
                                                    return Ok(Some(json!(lsp_locations)));
                                                }
                                            }

                                            // Fallback: scan open documents for qualified name references
                                            let docs_snapshot: Vec<(String, DocumentState)> =
                                                documents
                                                    .iter()
                                                    .map(|(k, v)| (k.clone(), v.clone()))
                                                    .collect();

                                            let mut all_locations = Vec::new();
                                            let qualified_name = format!("{}::{}", pkg, name);
                                            let search_regex = regex::Regex::new(&format!(
                                                r"\b{}\b",
                                                regex::escape(&qualified_name)
                                            ))
                                            .unwrap();

                                            for (doc_uri, doc_state) in docs_snapshot {
                                                let lines: Vec<&str> =
                                                    doc_state.text.lines().collect();
                                                for (line_num, line) in lines.iter().enumerate() {
                                                    for mat in search_regex.find_iter(line) {
                                                        all_locations.push(json!({
                                                            "uri": doc_uri,
                                                            "range": {
                                                                "start": {
                                                                    "line": line_num,
                                                                    "character": mat.start(),
                                                                },
                                                                "end": {
                                                                    "line": line_num,
                                                                    "character": mat.end(),
                                                                },
                                                            },
                                                        }));
                                                    }
                                                }
                                            }

                                            if !all_locations.is_empty() {
                                                return Ok(Some(json!(all_locations)));
                                            }
                                        }
                                    }
                                    break;
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
                    let highlights = provider.find_highlights(ast, &doc.text, offset);

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
                    if let Some(items) = provider.prepare(ast, &doc.text, offset) {
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
                for cap in sub_regex.captures_iter(&doc.text) {
                    if let (Some(m), Some(name)) = (cap.get(0), cap.get(1)) {
                        if offset >= m.start() && offset <= m.end() {
                            // Exact match - cursor is on this sub
                            exact_sub = Some((name.as_str().to_string(), m.start(), m.end()));
                            break;
                        }
                    }
                }

                if let Some((name, start, end)) = exact_sub {
                    let start_pos = doc.line_starts.offset_to_position_rope(&doc.rope, start);
                    let end_pos = doc.line_starts.offset_to_position_rope(&doc.rope, end);
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
                for cap in package_regex.captures_iter(&doc.text) {
                    if let (Some(m), Some(name)) = (cap.get(0), cap.get(1)) {
                        if offset >= m.start() && offset <= m.end() {
                            // Exact match - cursor is on this package
                            exact_pkg = Some((name.as_str().to_string(), m.start(), m.end()));
                            break;
                        }
                    }
                }

                if let Some((name, start, end)) = exact_pkg {
                    let start_pos = doc.line_starts.offset_to_position_rope(&doc.rope, start);
                    let end_pos = doc.line_starts.offset_to_position_rope(&doc.rope, end);
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
                    let token = self.get_token_at_position(&doc.text, offset);
                    if !token.is_empty()
                        && (token.starts_with('$')
                            || token.starts_with('@')
                            || token.starts_with('%')
                            || token.chars().next().is_some_and(|c| c.is_alphabetic() || c == '_'))
                    {
                        // Find the token bounds
                        let (start_offset, end_offset) = self.get_token_bounds(&doc.text, offset);
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
                        crate::code_actions_pragmas::missing_pragmas_actions(uri, &doc.text);

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
                            if let Some(obj) = a.as_object_mut() {
                                let edit_range = if off as usize >= doc.text.len() {
                                    let end = self.get_document_end_position(&doc.text);
                                    json!({"start": end.clone(), "end": end })
                                } else {
                                    let (line, col) = self.offset_to_pos16(doc, off as usize);
                                    json!({
                                        "start": {"line": line, "character": col},
                                        "end": {"line": line, "character": col}
                                    })
                                };

                                obj.insert(
                                    "edit".into(),
                                    json!({
                                        "changes": {
                                            u: [{
                                                "range": edit_range,
                                                "newText": txt
                                            }]
                                        }
                                    }),
                                );
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
                code: INVALID_PARAMS,
                message: "Missing textDocument.uri".into(),
                data: None,
            })?;
            let documents = self.documents.lock().unwrap();
            let doc = self.get_document(&documents, uri).ok_or_else(|| JsonRpcError {
                code: INVALID_REQUEST,
                message: format!("Document not open: {}", uri),
                data: None,
            })?;
            if let Some(ref ast) = doc.ast {
                let data =
                    crate::semantic_tokens::collect_semantic_tokens(ast, &doc.text, &|off| {
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
                code: INVALID_PARAMS,
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
                code: INVALID_REQUEST,
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
                code: INVALID_PARAMS,
                message: "Missing textDocument.uri".into(),
                data: None,
            })?;
            let documents = self.documents.lock().unwrap();
            let doc = self.get_document(&documents, uri).ok_or_else(|| JsonRpcError {
                code: INVALID_REQUEST,
                message: format!("Document not open: {}", uri),
                data: None,
            })?;

            // Get workspace roots from initialization params
            let roots = self.workspace_roots();
            let links = crate::document_links::compute_links(uri, &doc.text, &roots);
            Ok(Some(json!(links)))
        } else {
            Ok(Some(json!([])))
        }
    }

    /// Handle textDocument/selectionRange request
    fn handle_selection_range(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        if let Some(p) = params {
            let uri = p["textDocument"]["uri"].as_str().ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing textDocument.uri".into(),
                data: None,
            })?;
            let positions = p["positions"].as_array().ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing positions array".into(),
                data: None,
            })?;

            let documents = self.documents.lock().unwrap();
            let doc = self.get_document(&documents, uri).ok_or_else(|| JsonRpcError {
                code: INVALID_REQUEST,
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
                code: INVALID_PARAMS,
                message: "Missing textDocument.uri".into(),
                data: None,
            })?;
            let ch = p["ch"].as_str().and_then(|s| s.chars().next()).unwrap_or('\n');
            let pos = &p["position"];
            let line = pos["line"].as_u64().unwrap_or(0) as u32;
            let col = pos["character"].as_u64().unwrap_or(0) as u32;

            let documents = self.documents.lock().unwrap();
            let doc = self.get_document(&documents, uri).ok_or_else(|| JsonRpcError {
                code: INVALID_REQUEST,
                message: format!("Document not open: {}", uri),
                data: None,
            })?;

            if let Some(edits) =
                crate::on_type_formatting::compute_on_type_edit(&doc.text, line, col, ch)
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
                    let extractor = crate::symbol::SymbolExtractor::new_with_source(&doc.text);
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
                    let symbols = self.extract_symbols_fallback(&doc.text);
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
                if let Some(marker_offset) = crate::util::find_data_marker_byte_lexed(&doc.text) {
                    let marker_line = self.offset_to_line(&doc.text, marker_offset);
                    let total_lines = doc.text.lines().count();

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
                    crate::folding::FoldingRangeExtractor::extract_heredoc_ranges(&doc.text);
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
                        let start_line = self.offset_to_line(&doc.text, range.start_offset);
                        let end_line = self.offset_to_line(&doc.text, range.end_offset);

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
                        return Ok(Some(json!(self.extract_folding_fallback(&doc.text))));
                    }

                    return Ok(Some(json!(lsp_ranges)));
                } else {
                    // No AST, use fallback
                    return Ok(Some(json!(self.extract_folding_fallback(&doc.text))));
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
                match formatter.format_document(&doc.text, &options) {
                    Ok(edits) => {
                        let doc_end = self.get_document_end_position(&doc.text);
                        let lsp_edits: Vec<Value> = edits
                            .into_iter()
                            .map(|edit| {
                                json!({
                                    "range": {
                                        "start": {
                                            "line": edit.range.start.line,
                                            "character": edit.range.start.character,
                                        },
                                        "end": doc_end.clone(),
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
                match formatter.format_range(&doc.text, &range, &options) {
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

    /// Handle textDocument/codeLens request
    fn handle_code_lens(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        // Gate unadvertised feature
        if !self.advertised_features.lock().unwrap().code_lens {
            return Err(crate::lsp_errors::method_not_advertised());
        }

        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");

            let documents = self.documents.lock().unwrap();
            if let Some(doc) = self.get_document(&documents, uri) {
                if let Some(ref ast) = doc.ast {
                    let provider = CodeLensProvider::new(doc.text.clone());
                    let mut lenses = provider.extract(ast);

                    // Add shebang lens if applicable
                    if let Some(shebang_lens) = get_shebang_lens(&doc.text) {
                        lenses.insert(0, shebang_lens);
                    }

                    return Ok(Some(json!(lenses)));
                } else {
                    // Text-based fallback when AST is not available
                    let text_lenses = self.extract_text_based_code_lenses(&doc.text, uri);
                    return Ok(Some(json!(text_lenses)));
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
                    } else {
                        // Text-based fallback when AST is not available
                        total_references +=
                            self.count_references_text_based(&doc.text, symbol_name, symbol_kind);
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
                let completions = provider.get_inline_completions(&doc.text, line, character);
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
                let lines: Vec<&str> = doc.text.lines().collect();
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
                    crate::linked_editing::handle_linked_editing(&doc.text, line, character);
                return Ok(Some(serde_json::to_value(result).unwrap_or(Value::Null)));
            }
        }

        Ok(Some(Value::Null))
    }

    /// Count references to a symbol using text-based search
    fn count_references_text_based(
        &self,
        text: &str,
        symbol_name: &str,
        symbol_kind: &str,
    ) -> usize {
        let mut count = 0;

        match symbol_kind {
            "package" => {
                // Count package usage (use statements, new() calls, etc.)
                use regex::Regex;

                // Count "use PackageName" statements
                if let Ok(use_regex) =
                    Regex::new(&format!(r"\buse\s+{}\b", regex::escape(symbol_name)))
                {
                    count += use_regex.find_iter(text).count();
                }

                // Count "PackageName->new()" or "PackageName->method()" calls
                if let Ok(call_regex) = Regex::new(&format!(r"\b{}->", regex::escape(symbol_name)))
                {
                    count += call_regex.find_iter(text).count();
                }

                // Count "bless ... PackageName" statements
                if let Ok(bless_regex) =
                    Regex::new(&format!(r"bless\s+.*?,\s*{}", regex::escape(symbol_name)))
                {
                    count += bless_regex.find_iter(text).count();
                }
            }
            "subroutine" => {
                // Count function calls
                use regex::Regex;

                // Count "function_name(" calls
                if let Ok(call_regex) =
                    Regex::new(&format!(r"\b{}\s*\(", regex::escape(symbol_name)))
                {
                    count += call_regex.find_iter(text).count();
                }

                // Count "&function_name" references
                if let Ok(ref_regex) = Regex::new(&format!(r"&{}\b", regex::escape(symbol_name))) {
                    count += ref_regex.find_iter(text).count();
                }
            }
            _ => {
                // Generic symbol name search for unknown kinds
                use regex::Regex;
                if let Ok(generic_regex) =
                    Regex::new(&format!(r"\b{}\b", regex::escape(symbol_name)))
                {
                    count += generic_regex.find_iter(text).count();
                    // Don't count the definition itself if it exists
                    if count > 0 {
                        count = count.saturating_sub(1);
                    }
                }
            }
        }

        count
    }

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
                    let provider = SemanticTokensProvider::new(doc.text.clone());
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
                    let provider = SemanticTokensProvider::new(doc.text.clone());
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
                    let provider = CallHierarchyProvider::new(doc.text.clone(), uri.to_string());
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

                    let provider = CallHierarchyProvider::new(doc.text.clone(), uri.to_string());
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

                    let provider = CallHierarchyProvider::new(doc.text.clone(), uri.to_string());
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

                    let provider = InlayHintsProvider::with_config(doc.text.clone(), config);

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
                    self.slice_in_range(&doc.text, (s_line, s_ch), (e_line, e_ch))
                } else {
                    (0, doc.text.len(), doc.text.as_str())
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
                        let (l, c) = doc.line_starts.offset_to_position_rope(&doc.rope, val_offset);

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
                    self.slice_in_range(&doc.text, (s_line, s_ch), (e_line, e_ch))
                } else {
                    (0, doc.text.len(), doc.text.as_str())
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
                        let (l, c) = doc.line_starts.offset_to_position_rope(&doc.rope, global_off);
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
                                doc.line_starts.offset_to_position_rope(&doc.rope, global_off);
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
                                    doc.line_starts.offset_to_position_rope(&doc.rope, var_global);
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
                    let runner = TestRunner::new(doc.text.clone(), uri.to_string());
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

            // LSP 3.17 compliance: arguments field is required even if empty
            if !params.as_object().unwrap_or(&serde_json::Map::new()).contains_key("arguments") {
                return Err(JsonRpcError {
                    code: -32602, // InvalidParams
                    message: "Missing required 'arguments' field in executeCommand request"
                        .to_string(),
                    data: Some(json!({
                        "command": command,
                        "errorType": "executeCommand",
                        "originalError": "Missing 'arguments' field"
                    })),
                });
            }

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
                "perl.runTests" | "perl.runFile" | "perl.runTestSub" | "perl.debugTests"
                | "perl.runCritic" => {
                    match provider.execute_command(command, arguments) {
                        Ok(result) => return Ok(Some(result)),
                        Err(e) => {
                            // Return proper JSON-RPC error according to LSP 3.17 specification
                            let error_code = if e.contains("Missing") || e.contains("argument") {
                                -32602 // InvalidParams
                            } else if e.contains("Unknown command") {
                                -32601 // MethodNotFound
                            } else if e.contains("Path traversal") || e.contains("security") {
                                -32603 // InternalError (security)
                            } else {
                                -32603 // InternalError (general)
                            };

                            return Err(JsonRpcError {
                                code: error_code,
                                message: format!("Execute command failed: {}", e),
                                data: Some(json!({
                                    "command": command,
                                    "errorType": "executeCommand",
                                    "originalError": e
                                })),
                            });
                        }
                    }
                }
                // Debug commands (stub implementation for now)
                "perl.debugFile" => {
                    eprintln!("Debug command requested: {}", command);
                    // Return a success status - actual DAP integration can be added later
                    return Ok(Some(
                        json!({"status": "started", "message": format!("Debug session {} initiated", command)}),
                    ));
                }
                _ => {
                    return Err(JsonRpcError {
                        code: METHOD_NOT_FOUND,
                        message: format!("Unknown command: {}", command),
                        data: None,
                    });
                }
            }
        }

        // Missing params entirely
        Err(JsonRpcError {
            code: -32602, // InvalidParams
            message: "Missing parameters for executeCommand request".to_string(),
            data: Some(json!({
                "errorType": "executeCommand",
                "originalError": "Missing params"
            })),
        })
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
    fn run_test_file(&self, uri: &str) -> Result<Option<Value>, JsonRpcError> {
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
                        let code_text = crate::util::code_slice(&doc.text);
                        let mut parser = Parser::new(code_text);
                        if let Ok(ast) = parser.parse() {
                            builtin.analyze(&ast, &doc.text)
                        } else {
                            builtin.analyze(
                                &Node::new(
                                    NodeKind::Error { message: "Parse error".to_string() },
                                    crate::ast::SourceLocation { start: 0, end: 0 },
                                ),
                                &doc.text,
                            )
                        }
                    }
                }
            } else {
                // Use built-in analyzer
                eprintln!("Using built-in Perl::Critic analyzer");
                let builtin = BuiltInAnalyzer::new();
                let code_text = crate::util::code_slice(&doc.text);
                let mut parser = Parser::new(code_text);
                if let Ok(ast) = parser.parse() {
                    builtin.analyze(&ast, &doc.text)
                } else {
                    builtin.analyze(
                        &Node::new(
                            NodeKind::Error { message: "Parse error".to_string() },
                            crate::ast::SourceLocation { start: 0, end: 0 },
                        ),
                        &doc.text,
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

        Ok(Some(document_not_found_error()))
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
                match crate::lsp_document_link::collect_document_links(&doc.text, &uri_parsed) {
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

    /// Legacy onTypeFormatting handler retained for compatibility.
    /// Delegates to the modern formatting pipeline.
    fn handle_on_type_formatting_old(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        self.handle_on_type_formatting(params)
    }
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
