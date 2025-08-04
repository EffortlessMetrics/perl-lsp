//! Enhanced LSP Server with modular feature support
//!
//! This is the proper way to implement LSP features in a maintainable way.

use crate::{
    Parser,
    lsp::{FeatureManager, workspace_symbols::WorkspaceSymbolsProvider},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::io::{self, BufRead, BufReader, Read, Write};

/// Enhanced LSP server with feature modules
pub struct LspServerV2 {
    /// Document storage
    documents: Arc<Mutex<HashMap<String, DocumentState>>>,
    /// Feature manager
    features: Arc<Mutex<FeatureManager>>,
    /// Workspace symbols provider
    workspace_symbols: Arc<WorkspaceSymbolsProvider>,
    /// Server capabilities
    capabilities: ServerCapabilities,
}

/// Document state
#[derive(Clone)]
struct DocumentState {
    content: String,
    version: i32,
    ast: Option<crate::ast::Node>,
}

/// Server capabilities (what features we support)
#[derive(Debug, Clone, Default)]
struct ServerCapabilities {
    // Text document sync
    text_document_sync: Option<TextDocumentSyncKind>,
    
    // Language features
    completion_provider: Option<CompletionOptions>,
    hover_provider: Option<bool>,
    signature_help_provider: Option<SignatureHelpOptions>,
    definition_provider: Option<bool>,
    references_provider: Option<bool>,
    document_symbol_provider: Option<bool>,
    workspace_symbol_provider: Option<bool>,
    code_action_provider: Option<CodeActionOptions>,
    code_lens_provider: Option<CodeLensOptions>,
    document_formatting_provider: Option<bool>,
    document_range_formatting_provider: Option<bool>,
    rename_provider: Option<RenameOptions>,
    
    // New in 3.16+
    semantic_tokens_provider: Option<SemanticTokensOptions>,
    call_hierarchy_provider: Option<bool>,
    inlay_hint_provider: Option<bool>,
    
    // Workspace features
    workspace: Option<WorkspaceCapabilities>,
}

#[derive(Debug, Clone)]
struct TextDocumentSyncKind(u8);

impl TextDocumentSyncKind {
    const NONE: u8 = 0;
    const FULL: u8 = 1;
    const INCREMENTAL: u8 = 2;
}

#[derive(Debug, Clone, Serialize)]
struct CompletionOptions {
    trigger_characters: Option<Vec<String>>,
    resolve_provider: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
struct SignatureHelpOptions {
    trigger_characters: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
struct CodeActionOptions {
    code_action_kinds: Option<Vec<String>>,
    resolve_provider: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
struct CodeLensOptions {
    resolve_provider: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
struct RenameOptions {
    prepare_provider: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
struct SemanticTokensOptions {
    legend: SemanticTokensLegend,
    range: Option<bool>,
    full: Option<SemanticTokensFullOptions>,
}

#[derive(Debug, Clone, Serialize)]
struct SemanticTokensLegend {
    token_types: Vec<String>,
    token_modifiers: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct SemanticTokensFullOptions {
    delta: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
struct WorkspaceCapabilities {
    workspace_folders: Option<WorkspaceFoldersServerCapabilities>,
    file_operations: Option<FileOperationsServerCapabilities>,
}

#[derive(Debug, Clone, Serialize)]
struct WorkspaceFoldersServerCapabilities {
    supported: Option<bool>,
    change_notifications: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
struct FileOperationsServerCapabilities {
    did_create: Option<FileOperationRegistrationOptions>,
    did_rename: Option<FileOperationRegistrationOptions>,
    did_delete: Option<FileOperationRegistrationOptions>,
}

#[derive(Debug, Clone, Serialize)]
struct FileOperationRegistrationOptions {
    filters: Vec<FileOperationFilter>,
}

#[derive(Debug, Clone, Serialize)]
struct FileOperationFilter {
    pattern: FileOperationPattern,
}

#[derive(Debug, Clone, Serialize)]
struct FileOperationPattern {
    glob: String,
}

impl LspServerV2 {
    /// Create a new enhanced LSP server
    pub fn new() -> Self {
        let workspace_symbols = Arc::new(WorkspaceSymbolsProvider::new());
        
        let mut features = FeatureManager::new();
        // Register all feature providers here
        // features.register_workspace(workspace_symbols.clone());
        
        Self {
            documents: Arc::new(Mutex::new(HashMap::new())),
            features: Arc::new(Mutex::new(features)),
            workspace_symbols,
            capabilities: Self::build_capabilities(),
        }
    }
    
    /// Build server capabilities
    fn build_capabilities() -> ServerCapabilities {
        ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncKind(TextDocumentSyncKind::INCREMENTAL)),
            
            // Basic features (already implemented)
            completion_provider: Some(CompletionOptions {
                trigger_characters: Some(vec![
                    "$".to_string(),
                    "@".to_string(),
                    "%".to_string(),
                    "->".to_string(),
                    "::".to_string(),
                ]),
                resolve_provider: Some(false),
            }),
            hover_provider: Some(true),
            signature_help_provider: Some(SignatureHelpOptions {
                trigger_characters: Some(vec!["(".to_string(), ",".to_string()]),
            }),
            definition_provider: Some(true),
            references_provider: Some(true),
            document_symbol_provider: Some(true),
            document_formatting_provider: Some(true),
            document_range_formatting_provider: Some(true),
            rename_provider: Some(RenameOptions {
                prepare_provider: Some(true),
            }),
            
            // New features
            workspace_symbol_provider: Some(true),
            code_action_provider: Some(CodeActionOptions {
                code_action_kinds: Some(vec![
                    "quickfix".to_string(),
                    "refactor".to_string(),
                    "refactor.extract".to_string(),
                    "refactor.inline".to_string(),
                ]),
                resolve_provider: Some(true),
            }),
            code_lens_provider: Some(CodeLensOptions {
                resolve_provider: Some(true),
            }),
            semantic_tokens_provider: Some(SemanticTokensOptions {
                legend: SemanticTokensLegend {
                    token_types: vec![
                        "namespace".to_string(),
                        "class".to_string(),
                        "enum".to_string(),
                        "interface".to_string(),
                        "struct".to_string(),
                        "typeParameter".to_string(),
                        "type".to_string(),
                        "parameter".to_string(),
                        "variable".to_string(),
                        "property".to_string(),
                        "enumMember".to_string(),
                        "decorator".to_string(),
                        "event".to_string(),
                        "function".to_string(),
                        "method".to_string(),
                        "macro".to_string(),
                        "label".to_string(),
                        "comment".to_string(),
                        "string".to_string(),
                        "keyword".to_string(),
                        "number".to_string(),
                        "regexp".to_string(),
                        "operator".to_string(),
                    ],
                    token_modifiers: vec![
                        "declaration".to_string(),
                        "definition".to_string(),
                        "readonly".to_string(),
                        "static".to_string(),
                        "deprecated".to_string(),
                        "abstract".to_string(),
                        "async".to_string(),
                        "modification".to_string(),
                        "documentation".to_string(),
                        "defaultLibrary".to_string(),
                    ],
                },
                range: Some(true),
                full: Some(SemanticTokensFullOptions {
                    delta: Some(true),
                }),
            }),
            call_hierarchy_provider: Some(true),
            inlay_hint_provider: Some(true),
            
            workspace: Some(WorkspaceCapabilities {
                workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                    supported: Some(true),
                    change_notifications: Some(true),
                }),
                file_operations: None,
            }),
        }
    }
    
    /// Handle initialize request
    fn handle_initialize(&mut self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        // Initialize feature providers
        if let Some(params) = &params {
            let _ = self.features.lock().unwrap().initialize(params);
        }
        
        // Build capabilities response
        let capabilities_json = json!({
            "textDocumentSync": self.capabilities.text_document_sync.as_ref().map(|k| k.0),
            "completionProvider": self.capabilities.completion_provider.as_ref(),
            "hoverProvider": self.capabilities.hover_provider,
            "signatureHelpProvider": self.capabilities.signature_help_provider.as_ref(),
            "definitionProvider": self.capabilities.definition_provider,
            "referencesProvider": self.capabilities.references_provider,
            "documentSymbolProvider": self.capabilities.document_symbol_provider,
            "workspaceSymbolProvider": self.capabilities.workspace_symbol_provider,
            "codeActionProvider": self.capabilities.code_action_provider.as_ref(),
            "codeLensProvider": self.capabilities.code_lens_provider.as_ref(),
            "documentFormattingProvider": self.capabilities.document_formatting_provider,
            "documentRangeFormattingProvider": self.capabilities.document_range_formatting_provider,
            "renameProvider": self.capabilities.rename_provider.as_ref(),
            "semanticTokensProvider": self.capabilities.semantic_tokens_provider.as_ref(),
            "callHierarchyProvider": self.capabilities.call_hierarchy_provider,
            "inlayHintProvider": self.capabilities.inlay_hint_provider,
            "workspace": self.capabilities.workspace.as_ref(),
        });
        
        Ok(Some(json!({
            "capabilities": capabilities_json,
            "serverInfo": {
                "name": "perl-language-server",
                "version": "0.6.0"
            }
        })))
    }
    
    /// Handle workspace/symbol request
    fn handle_workspace_symbol(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
        let params: WorkspaceSymbolParams = serde_json::from_value(
            params.ok_or_else(|| JsonRpcError {
                code: -32602,
                message: "Invalid params".to_string(),
                data: None,
            })?
        )?;
        
        let symbols = self.workspace_symbols.search(&params.query);
        
        // Convert to LSP format
        let lsp_symbols: Vec<Value> = symbols.into_iter().map(|sym| {
            json!({
                "name": sym.name,
                "kind": symbol_kind_to_lsp(&sym.kind),
                "location": {
                    "uri": sym.uri,
                    "range": sym.range
                },
                "containerName": sym.container_name
            })
        }).collect();
        
        Ok(Some(json!(lsp_symbols)))
    }
    
    /// Process document changes and update indices
    fn process_document(&self, uri: &str, content: &str) {
        // Parse the document
        let mut parser = Parser::new(content);
        if let Ok(ast) = parser.parse() {
            // Index for workspace symbols
            self.workspace_symbols.index_document(uri, &ast);
            
            // Process through all feature providers
            let _ = self.features.lock().unwrap().process_document(uri, content, &ast);
        }
    }
}

/// JSON-RPC error
#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    data: Option<Value>,
}

/// Workspace symbol params
#[derive(Debug, Deserialize)]
struct WorkspaceSymbolParams {
    query: String,
}

/// Convert symbol kind to LSP number
fn symbol_kind_to_lsp(kind: &crate::symbol::SymbolKind) -> u8 {
    use crate::symbol::SymbolKind;
    match kind {
        SymbolKind::File => 1,
        SymbolKind::Module => 2,
        SymbolKind::Namespace => 3,
        SymbolKind::Package => 4,
        SymbolKind::Class => 5,
        SymbolKind::Method => 6,
        SymbolKind::Property => 7,
        SymbolKind::Field => 8,
        SymbolKind::Constructor => 9,
        SymbolKind::Enum => 10,
        SymbolKind::Interface => 11,
        SymbolKind::Function => 12,
        SymbolKind::Variable => 13,
        SymbolKind::Constant => 14,
        SymbolKind::String => 15,
        SymbolKind::Number => 16,
        SymbolKind::Boolean => 17,
        SymbolKind::Array => 18,
        SymbolKind::Object => 19,
        SymbolKind::Key => 20,
        SymbolKind::Null => 21,
        SymbolKind::EnumMember => 22,
        SymbolKind::Struct => 23,
        SymbolKind::Event => 24,
        SymbolKind::Operator => 25,
        SymbolKind::TypeParameter => 26,
    }
}

impl Default for LspServerV2 {
    fn default() -> Self {
        Self::new()
    }
}