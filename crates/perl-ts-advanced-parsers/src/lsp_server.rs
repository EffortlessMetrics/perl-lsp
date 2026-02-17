//! Language Server Protocol implementation for Perl
//!
//! This module provides LSP support for Perl development, including
//! syntax checking, code completion, and navigation features.

use crate::enhanced_full_parser::EnhancedFullParser;
use crate::error_recovery::ErrorRecoveryParser;
use crate::incremental_parser::{Edit, IncrementalParser, Position as ParsePosition};
use perl_parser_pest::AstNode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// LSP Position
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

/// LSP Range
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

/// LSP Location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub uri: String,
    pub range: Range,
}

/// Diagnostic severity
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DiagnosticSeverity {
    Error = 1,
    Warning = 2,
    Information = 3,
    Hint = 4,
}

/// LSP Diagnostic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub range: Range,
    pub severity: Option<DiagnosticSeverity>,
    pub code: Option<String>,
    pub source: Option<String>,
    pub message: String,
}

/// Completion item kind
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CompletionItemKind {
    Text = 1,
    Method = 2,
    Function = 3,
    Constructor = 4,
    Field = 5,
    Variable = 6,
    Class = 7,
    Interface = 8,
    Module = 9,
    Property = 10,
    Unit = 11,
    Value = 12,
    Enum = 13,
    Keyword = 14,
    Snippet = 15,
    Color = 16,
    File = 17,
    Reference = 18,
    Folder = 19,
    EnumMember = 20,
    Constant = 21,
    Struct = 22,
    Event = 23,
    Operator = 24,
    TypeParameter = 25,
}

/// Completion item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionItem {
    pub label: String,
    pub kind: Option<CompletionItemKind>,
    pub detail: Option<String>,
    pub documentation: Option<String>,
    pub insert_text: Option<String>,
}

/// Symbol information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolInformation {
    pub name: String,
    pub kind: SymbolKind,
    pub location: Location,
    pub container_name: Option<String>,
}

/// Symbol kind
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SymbolKind {
    File = 1,
    Module = 2,
    Namespace = 3,
    Package = 4,
    Class = 5,
    Method = 6,
    Property = 7,
    Field = 8,
    Constructor = 9,
    Enum = 10,
    Interface = 11,
    Function = 12,
    Variable = 13,
    Constant = 14,
    String = 15,
    Number = 16,
    Boolean = 17,
    Array = 18,
    Object = 19,
    Key = 20,
    Null = 21,
    EnumMember = 22,
    Struct = 23,
    Event = 24,
    Operator = 25,
    TypeParameter = 26,
}

/// Document state
struct DocumentState {
    #[allow(dead_code)]
    uri: String,
    content: String,
    version: i32,
    parser: IncrementalParser,
    symbols: Vec<SymbolInformation>,
    diagnostics: Vec<Diagnostic>,
}

/// Perl Language Server
pub struct PerlLanguageServer {
    documents: Arc<Mutex<HashMap<String, DocumentState>>>,
    /// Built-in Perl functions
    builtin_functions: Vec<&'static str>,
    /// Common Perl variables
    builtin_variables: Vec<&'static str>,
}

impl PerlLanguageServer {
    pub fn new() -> Self {
        Self {
            documents: Arc::new(Mutex::new(HashMap::new())),
            builtin_functions: vec![
                "print", "say", "die", "warn", "open", "close", "read", "write", "push", "pop",
                "shift", "unshift", "splice", "sort", "grep", "map", "split", "join", "substr",
                "index", "rindex", "length", "chomp", "sprintf", "printf", "ref", "defined",
                "undef", "bless", "eval",
            ],
            builtin_variables: vec![
                "$_", "@_", "$!", "$@", "$?", "$$", "$0", "@ARGV", "%ENV", "$^O", "$^V", "@INC",
                "%INC", "$.", "$,", "$/", "$\\",
            ],
        }
    }

    /// Handle document open
    pub fn did_open(&self, uri: String, text: String, version: i32) {
        let mut parser = IncrementalParser::new();
        let mut diagnostics = Vec::new();

        // Parse and check for errors
        match parser.parse_initial(&text) {
            Ok(_) => {
                // Success - check with error recovery parser for warnings
                let mut error_parser = ErrorRecoveryParser::new();
                if error_parser.parse(&text).is_ok() {
                    for error in error_parser.errors() {
                        diagnostics.push(Diagnostic {
                            range: Range {
                                start: Position {
                                    line: error.line as u32,
                                    character: error.column as u32,
                                },
                                end: Position {
                                    line: error.line as u32,
                                    character: (error.column + 10) as u32,
                                },
                            },
                            severity: Some(DiagnosticSeverity::Warning),
                            code: None,
                            source: Some("perl-lsp".to_string()),
                            message: error.message.clone(),
                        });
                    }
                }
            }
            Err(e) => {
                diagnostics.push(Diagnostic {
                    range: Range {
                        start: Position { line: 0, character: 0 },
                        end: Position { line: 0, character: 1 },
                    },
                    severity: Some(DiagnosticSeverity::Error),
                    code: None,
                    source: Some("perl-lsp".to_string()),
                    message: format!("Parse error: {:?}", e),
                });
            }
        }

        // Extract symbols
        let symbols = self.extract_symbols(&uri, &text);

        let state = DocumentState {
            uri: uri.clone(),
            content: text,
            version,
            parser,
            symbols,
            diagnostics,
        };

        let mut docs = self.documents.lock().unwrap_or_else(|e| e.into_inner());
        docs.insert(uri, state);
    }

    /// Handle document change
    pub fn did_change(
        &self,
        uri: String,
        changes: Vec<TextDocumentContentChangeEvent>,
        version: i32,
    ) {
        let mut docs = self.documents.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(state) = docs.get_mut(&uri) {
            // Apply changes
            for change in changes {
                if let Some(range) = change.range {
                    // Incremental change
                    let edit = self.range_to_edit(&range, &state.content, &change.text);
                    state.content = self.apply_text_edit(&state.content, &range, &change.text);

                    // Re-parse incrementally
                    match state.parser.apply_edit(edit, &state.content) {
                        Ok(_) => {
                            state.diagnostics.clear();
                        }
                        Err(e) => {
                            state.diagnostics = vec![Diagnostic {
                                range,
                                severity: Some(DiagnosticSeverity::Error),
                                code: None,
                                source: Some("perl-lsp".to_string()),
                                message: format!("Parse error: {:?}", e),
                            }];
                        }
                    }
                } else {
                    // Full document change
                    state.content = change.text;
                    match state.parser.parse_initial(&state.content) {
                        Ok(_) => state.diagnostics.clear(),
                        Err(e) => {
                            state.diagnostics = vec![Diagnostic {
                                range: Range {
                                    start: Position { line: 0, character: 0 },
                                    end: Position { line: 0, character: 1 },
                                },
                                severity: Some(DiagnosticSeverity::Error),
                                code: None,
                                source: Some("perl-lsp".to_string()),
                                message: format!("Parse error: {:?}", e),
                            }];
                        }
                    }
                }
            }

            // Update symbols
            state.symbols = self.extract_symbols(&uri, &state.content);
            state.version = version;
        }
    }

    /// Get diagnostics for a document
    pub fn get_diagnostics(&self, uri: &str) -> Vec<Diagnostic> {
        let docs = self.documents.lock().unwrap_or_else(|e| e.into_inner());
        docs.get(uri).map(|state| state.diagnostics.clone()).unwrap_or_default()
    }

    /// Get completions at a position
    pub fn get_completions(&self, uri: &str, _position: Position) -> Vec<CompletionItem> {
        let mut completions = Vec::new();

        // Add built-in functions
        for func in &self.builtin_functions {
            completions.push(CompletionItem {
                label: func.to_string(),
                kind: Some(CompletionItemKind::Function),
                detail: Some("Built-in function".to_string()),
                documentation: None,
                insert_text: Some(format!("{}($0)", func)),
            });
        }

        // Add built-in variables
        for var in &self.builtin_variables {
            completions.push(CompletionItem {
                label: var.to_string(),
                kind: Some(CompletionItemKind::Variable),
                detail: Some("Built-in variable".to_string()),
                documentation: None,
                insert_text: Some(var.to_string()),
            });
        }

        // Add symbols from current document
        let docs = self.documents.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(state) = docs.get(uri) {
            for symbol in &state.symbols {
                completions.push(CompletionItem {
                    label: symbol.name.clone(),
                    kind: match symbol.kind {
                        SymbolKind::Function => Some(CompletionItemKind::Function),
                        SymbolKind::Variable => Some(CompletionItemKind::Variable),
                        SymbolKind::Package => Some(CompletionItemKind::Module),
                        _ => None,
                    },
                    detail: symbol.container_name.clone(),
                    documentation: None,
                    insert_text: Some(symbol.name.clone()),
                });
            }
        }

        completions
    }

    /// Get document symbols
    pub fn get_document_symbols(&self, uri: &str) -> Vec<SymbolInformation> {
        let docs = self.documents.lock().unwrap_or_else(|e| e.into_inner());
        docs.get(uri).map(|state| state.symbols.clone()).unwrap_or_default()
    }

    /// Extract symbols from source
    fn extract_symbols(&self, uri: &str, source: &str) -> Vec<SymbolInformation> {
        let mut symbols = Vec::new();
        let mut parser = EnhancedFullParser::new();

        if let Ok(ast) = parser.parse(source) {
            self.extract_symbols_from_ast(&ast, uri, &mut symbols, None);
        }

        symbols
    }

    /// Extract symbols from AST
    #[allow(clippy::only_used_in_recursion)]
    fn extract_symbols_from_ast(
        &self,
        node: &AstNode,
        uri: &str,
        symbols: &mut Vec<SymbolInformation>,
        container: Option<&str>,
    ) {
        match node {
            AstNode::SubDeclaration { name, body, .. } => {
                symbols.push(SymbolInformation {
                    name: name.to_string(),
                    kind: SymbolKind::Function,
                    location: Location {
                        uri: uri.to_string(),
                        range: Range {
                            start: Position { line: 0, character: 0 },
                            end: Position { line: 0, character: 0 },
                        },
                    },
                    container_name: container.map(|s| s.to_string()),
                });
                self.extract_symbols_from_ast(body, uri, symbols, Some(name));
            }
            AstNode::PackageDeclaration { name, block, .. } => {
                symbols.push(SymbolInformation {
                    name: name.to_string(),
                    kind: SymbolKind::Package,
                    location: Location {
                        uri: uri.to_string(),
                        range: Range {
                            start: Position { line: 0, character: 0 },
                            end: Position { line: 0, character: 0 },
                        },
                    },
                    container_name: container.map(|s| s.to_string()),
                });
                if let Some(block) = block {
                    self.extract_symbols_from_ast(block, uri, symbols, Some(name));
                }
            }
            AstNode::VariableDeclaration { variables, .. } => {
                for var in variables {
                    if let AstNode::ScalarVariable(name)
                    | AstNode::ArrayVariable(name)
                    | AstNode::HashVariable(name) = var
                    {
                        symbols.push(SymbolInformation {
                            name: name.to_string(),
                            kind: SymbolKind::Variable,
                            location: Location {
                                uri: uri.to_string(),
                                range: Range {
                                    start: Position { line: 0, character: 0 },
                                    end: Position { line: 0, character: 0 },
                                },
                            },
                            container_name: container.map(|s| s.to_string()),
                        });
                    }
                }
            }
            AstNode::Program(nodes) | AstNode::Block(nodes) => {
                for child in nodes {
                    self.extract_symbols_from_ast(child, uri, symbols, container);
                }
            }
            AstNode::Statement(inner) => {
                self.extract_symbols_from_ast(inner, uri, symbols, container);
            }
            _ => {}
        }
    }

    /// Convert LSP range to parser edit
    fn range_to_edit(&self, range: &Range, content: &str, new_text: &str) -> Edit {
        let start_byte = self.position_to_byte(&range.start, content);
        let end_byte = self.position_to_byte(&range.end, content);

        Edit {
            start_byte,
            old_end_byte: end_byte,
            new_end_byte: start_byte + new_text.len(),
            start_position: ParsePosition {
                line: range.start.line as usize,
                column: range.start.character as usize,
            },
            old_end_position: ParsePosition {
                line: range.end.line as usize,
                column: range.end.character as usize,
            },
            new_end_position: ParsePosition {
                line: range.start.line as usize,
                column: range.start.character as usize + new_text.len(),
            },
        }
    }

    /// Convert position to byte offset
    fn position_to_byte(&self, pos: &Position, content: &str) -> usize {
        let mut line = 0;
        let mut byte = 0;

        for ch in content.chars() {
            if line == pos.line as usize && byte == pos.character as usize {
                return byte;
            }
            if ch == '\n' {
                line += 1;
            }
            byte += ch.len_utf8();
        }

        byte
    }

    /// Apply text edit to content
    fn apply_text_edit(&self, content: &str, range: &Range, new_text: &str) -> String {
        let start = self.position_to_byte(&range.start, content);
        let end = self.position_to_byte(&range.end, content);

        let mut result = String::new();
        result.push_str(&content[..start]);
        result.push_str(new_text);
        result.push_str(&content[end..]);
        result
    }
}

/// Text document content change event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextDocumentContentChangeEvent {
    pub range: Option<Range>,
    pub text: String,
}

impl Default for PerlLanguageServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lsp_server_creation() {
        let server = PerlLanguageServer::new();
        assert!(!server.builtin_functions.is_empty());
        assert!(!server.builtin_variables.is_empty());
    }

    #[test]
    fn test_document_open() {
        let server = PerlLanguageServer::new();
        let uri = "file:///test.pl".to_string();
        let text = "my $x = 42;\nprint $x;".to_string();

        server.did_open(uri.clone(), text, 1);

        let diagnostics = server.get_diagnostics(&uri);
        assert_eq!(diagnostics.len(), 0);

        let symbols = server.get_document_symbols(&uri);
        assert!(symbols.iter().any(|s| s.name == "$x"));
    }

    #[test]
    fn test_completions() {
        let server = PerlLanguageServer::new();
        let uri = "file:///test.pl".to_string();
        let text = "sub test { }\nmy $var = 1;".to_string();

        server.did_open(uri.clone(), text, 1);

        let completions = server.get_completions(&uri, Position { line: 1, character: 5 });
        assert!(completions.iter().any(|c| c.label == "print"));
        assert!(completions.iter().any(|c| c.label == "$_"));
        assert!(completions.iter().any(|c| c.label == "test"));
    }
}
