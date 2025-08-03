//! Language Server Protocol implementation for Perl
//!
//! This module provides a basic LSP server that uses the perl-parser
//! for IDE features like go-to-definition, find-references, hover,
//! and semantic highlighting.

use crate::ast::Node;
use crate::parser::Parser;
use crate::SourceLocation;
use crate::semantic::{SemanticAnalyzer, SemanticTokenType, SemanticTokenModifier};
use crate::symbol::{SymbolKind};
use std::collections::HashMap;
use std::path::PathBuf;

/// Document information stored by the language server
#[derive(Debug)]
pub struct Document {
    /// Document URI
    pub uri: String,
    /// Document version
    pub version: i32,
    /// Source text
    pub text: String,
    /// Parsed AST
    pub ast: Option<Node>,
    /// Semantic analyzer
    pub analyzer: Option<SemanticAnalyzer>,
    /// Line start offsets for position conversion
    pub line_starts: Vec<usize>,
}

impl Document {
    /// Create a new document
    pub fn new(uri: String, version: i32, text: String) -> Self {
        let line_starts = compute_line_starts(&text);
        Document {
            uri,
            version,
            text,
            ast: None,
            analyzer: None,
            line_starts,
        }
    }
    
    /// Update document text and reparse
    pub fn update(&mut self, version: i32, text: String) {
        self.version = version;
        self.text = text;
        self.line_starts = compute_line_starts(&self.text);
        self.parse();
    }
    
    /// Parse the document
    pub fn parse(&mut self) {
        let mut parser = Parser::new(&self.text);
        match parser.parse() {
            Ok(ast) => {
                let analyzer = SemanticAnalyzer::analyze(&ast);
                self.ast = Some(ast);
                self.analyzer = Some(analyzer);
            }
            Err(_) => {
                // Keep previous AST if parse fails
                // In practice, we'd want error recovery here
            }
        }
    }
    
    /// Convert LSP position to byte offset
    pub fn offset_at_position(&self, position: Position) -> Option<usize> {
        if position.line >= self.line_starts.len() {
            return None;
        }
        
        let line_start = self.line_starts[position.line];
        let line_end = if position.line + 1 < self.line_starts.len() {
            self.line_starts[position.line + 1]
        } else {
            self.text.len()
        };
        
        let line = &self.text[line_start..line_end];
        let mut col = 0;
        let mut byte_col = 0;
        
        for ch in line.chars() {
            if col >= position.character {
                break;
            }
            col += 1;
            byte_col += ch.len_utf8();
        }
        
        Some(line_start + byte_col)
    }
    
    /// Convert byte offset to LSP position
    pub fn position_at_offset(&self, offset: usize) -> Position {
        let line = self.line_starts.iter()
            .position(|&start| start > offset)
            .map(|i| i - 1)
            .unwrap_or(self.line_starts.len() - 1);
        
        let line_start = self.line_starts[line];
        let column = self.text[line_start..offset].chars().count();
        
        Position { line, character: column }
    }
}

/// Language server state
pub struct LanguageServer {
    /// Open documents
    pub documents: HashMap<String, Document>,
    /// Workspace root
    workspace_root: Option<PathBuf>,
}

impl LanguageServer {
    /// Create a new language server
    pub fn new() -> Self {
        LanguageServer {
            documents: HashMap::new(),
            workspace_root: None,
        }
    }
    
    /// Set workspace root
    pub fn set_workspace_root(&mut self, root: PathBuf) {
        self.workspace_root = Some(root);
    }
    
    /// Open a document
    pub fn open_document(&mut self, uri: String, version: i32, text: String) {
        let mut doc = Document::new(uri.clone(), version, text);
        doc.parse();
        self.documents.insert(uri, doc);
    }
    
    /// Update a document
    pub fn update_document(&mut self, uri: &str, version: i32, text: String) {
        if let Some(doc) = self.documents.get_mut(uri) {
            doc.update(version, text);
        }
    }
    
    /// Close a document
    pub fn close_document(&mut self, uri: &str) {
        self.documents.remove(uri);
    }
    
    /// Go to definition
    pub fn goto_definition(&self, uri: &str, position: Position) -> Option<LocationLink> {
        let doc = self.documents.get(uri)?;
        let analyzer = doc.analyzer.as_ref()?;
        let offset = doc.offset_at_position(position)?;
        
        // Find the node at this position
        let _location = SourceLocation { start: offset, end: offset + 1 };
        
        // Look for a symbol at this location
        for (_name, symbols) in &analyzer.symbol_table().symbols {
            for symbol in symbols {
                if symbol.location.start <= offset && symbol.location.end >= offset {
                    // This is a definition, not a reference
                    continue;
                }
            }
        }
        
        // Look for a reference at this location
        for (_name, refs) in &analyzer.symbol_table().references {
            for reference in refs {
                if reference.location.start <= offset && reference.location.end >= offset {
                    // Find the definition of this reference
                    let symbols = analyzer.symbol_table().find_symbol(
                        &reference.name,
                        reference.scope_id,
                        reference.kind
                    );
                    
                    if let Some(symbol) = symbols.first() {
                        let start = doc.position_at_offset(symbol.location.start);
                        let end = doc.position_at_offset(symbol.location.end);
                        
                        return Some(LocationLink {
                            target_uri: uri.to_string(),
                            target_range: Range { start, end },
                            target_selection_range: Range { start, end },
                        });
                    }
                }
            }
        }
        
        None
    }
    
    /// Find all references
    pub fn find_references(&self, uri: &str, position: Position, include_declaration: bool) -> Vec<Location> {
        let doc = match self.documents.get(uri) {
            Some(doc) => doc,
            None => return vec![],
        };
        
        let analyzer = match &doc.analyzer {
            Some(a) => a,
            None => return vec![],
        };
        
        let offset = match doc.offset_at_position(position) {
            Some(o) => o,
            None => return vec![],
        };
        
        let mut locations = Vec::new();
        
        // Find the symbol at this position
        let target_symbol = analyzer.symbol_at(SourceLocation { start: offset, end: offset + 1 });
        
        if let Some(symbol) = target_symbol {
            // Include the definition if requested
            if include_declaration {
                let start = doc.position_at_offset(symbol.location.start);
                let end = doc.position_at_offset(symbol.location.end);
                
                locations.push(Location {
                    uri: uri.to_string(),
                    range: Range { start, end },
                });
            }
            
            // Find all references to this symbol
            let references = analyzer.symbol_table().find_references(symbol);
            
            for reference in references {
                let start = doc.position_at_offset(reference.location.start);
                let end = doc.position_at_offset(reference.location.end);
                
                locations.push(Location {
                    uri: uri.to_string(),
                    range: Range { start, end },
                });
            }
        }
        
        locations
    }
    
    /// Get hover information
    pub fn hover(&self, uri: &str, position: Position) -> Option<Hover> {
        let doc = self.documents.get(uri)?;
        let analyzer = doc.analyzer.as_ref()?;
        let offset = doc.offset_at_position(position)?;
        
        let location = SourceLocation { start: offset, end: offset + 1 };
        let hover_info = analyzer.hover_at(location)?;
        
        let mut contents = vec![
            format!("```perl\n{}\n```", hover_info.signature),
        ];
        
        if let Some(doc) = &hover_info.documentation {
            contents.push(doc.clone());
        }
        
        for detail in &hover_info.details {
            contents.push(detail.clone());
        }
        
        Some(Hover {
            contents: contents.join("\n\n"),
            range: None,
        })
    }
    
    /// Get semantic tokens for a document
    pub fn semantic_tokens(&self, uri: &str) -> Option<Vec<SemanticToken>> {
        let doc = self.documents.get(uri)?;
        let analyzer = doc.analyzer.as_ref()?;
        
        let mut tokens = Vec::new();
        let mut prev_line = 0;
        let mut prev_char = 0;
        
        // Convert semantic tokens to LSP format
        let mut sem_tokens: Vec<_> = analyzer.semantic_tokens().to_vec();
        sem_tokens.sort_by_key(|t| t.location.start);
        
        for token in sem_tokens {
            let start = doc.position_at_offset(token.location.start);
            let length = token.location.end - token.location.start;
            
            let delta_line = start.line - prev_line;
            let delta_start = if delta_line == 0 {
                start.character - prev_char
            } else {
                start.character
            };
            
            tokens.push(SemanticToken {
                delta_line,
                delta_start,
                length,
                token_type: encode_token_type(token.token_type),
                token_modifiers: encode_token_modifiers(&token.modifiers),
            });
            
            prev_line = start.line;
            prev_char = start.character;
        }
        
        Some(tokens)
    }
    
    /// Get document symbols (outline)
    pub fn document_symbols(&self, uri: &str) -> Vec<DocumentSymbol> {
        let doc = match self.documents.get(uri) {
            Some(doc) => doc,
            None => return vec![],
        };
        
        let analyzer = match &doc.analyzer {
            Some(a) => a,
            None => return vec![],
        };
        
        let mut symbols = Vec::new();
        
        // Convert symbol table to document symbols
        for (name, syms) in &analyzer.symbol_table().symbols {
            for sym in syms {
                let start = doc.position_at_offset(sym.location.start);
                let end = doc.position_at_offset(sym.location.end);
                let range = Range { start, end };
                
                let kind = match sym.kind {
                    SymbolKind::ScalarVariable |
                    SymbolKind::ArrayVariable |
                    SymbolKind::HashVariable => SymbolKindEnum::Variable,
                    SymbolKind::Subroutine => SymbolKindEnum::Function,
                    SymbolKind::Package => SymbolKindEnum::Module,
                    SymbolKind::Constant => SymbolKindEnum::Constant,
                    SymbolKind::Label => SymbolKindEnum::Key,
                    SymbolKind::Format => SymbolKindEnum::Struct,
                };
                
                let detail = sym.declaration.clone()
                    .or_else(|| sym.kind.sigil().map(|s| s.to_string()));
                
                symbols.push(DocumentSymbol {
                    name: if let Some(sigil) = sym.kind.sigil() {
                        format!("{}{}", sigil, name)
                    } else {
                        name.clone()
                    },
                    detail,
                    kind,
                    range: range.clone(),
                    selection_range: range,
                    children: vec![], // TODO: Nested symbols
                });
            }
        }
        
        symbols
    }
}

// LSP type definitions (simplified versions)

#[derive(Debug, Clone, Copy)]
pub struct Position {
    pub line: usize,
    pub character: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Clone)]
pub struct Location {
    pub uri: String,
    pub range: Range,
}

#[derive(Debug, Clone)]
pub struct LocationLink {
    pub target_uri: String,
    pub target_range: Range,
    pub target_selection_range: Range,
}

#[derive(Debug, Clone)]
pub struct Hover {
    pub contents: String,
    pub range: Option<Range>,
}

#[derive(Debug, Clone)]
pub struct SemanticToken {
    pub delta_line: usize,
    pub delta_start: usize,
    pub length: usize,
    pub token_type: u32,
    pub token_modifiers: u32,
}

#[derive(Debug, Clone)]
pub struct DocumentSymbol {
    pub name: String,
    pub detail: Option<String>,
    pub kind: SymbolKindEnum,
    pub range: Range,
    pub selection_range: Range,
    pub children: Vec<DocumentSymbol>,
}

#[derive(Debug, Clone, Copy)]
pub enum SymbolKindEnum {
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

/// Compute line start offsets in a text
fn compute_line_starts(text: &str) -> Vec<usize> {
    let mut line_starts = vec![0];
    
    for (i, ch) in text.char_indices() {
        if ch == '\n' {
            line_starts.push(i + 1);
        }
    }
    
    line_starts
}

/// Encode semantic token type to LSP format
fn encode_token_type(token_type: SemanticTokenType) -> u32 {
    match token_type {
        SemanticTokenType::Variable => 0,
        SemanticTokenType::VariableDeclaration => 0,
        SemanticTokenType::VariableReadonly => 0,
        SemanticTokenType::Parameter => 1,
        SemanticTokenType::Function => 2,
        SemanticTokenType::FunctionDeclaration => 2,
        SemanticTokenType::Method => 3,
        SemanticTokenType::Class => 4,
        SemanticTokenType::Namespace => 5,
        SemanticTokenType::Type => 6,
        SemanticTokenType::Keyword => 7,
        SemanticTokenType::KeywordControl => 7,
        SemanticTokenType::Modifier => 8,
        SemanticTokenType::Number => 9,
        SemanticTokenType::String => 10,
        SemanticTokenType::Regex => 11,
        SemanticTokenType::Comment => 12,
        SemanticTokenType::CommentDoc => 12,
        SemanticTokenType::Operator => 13,
        SemanticTokenType::Punctuation => 14,
        SemanticTokenType::Label => 15,
    }
}

/// Encode semantic token modifiers to LSP format
fn encode_token_modifiers(modifiers: &[SemanticTokenModifier]) -> u32 {
    let mut result = 0u32;
    
    for modifier in modifiers {
        let bit = match modifier {
            SemanticTokenModifier::Declaration => 0,
            SemanticTokenModifier::Definition => 1,
            SemanticTokenModifier::Readonly => 2,
            SemanticTokenModifier::Static => 3,
            SemanticTokenModifier::Deprecated => 4,
            SemanticTokenModifier::Abstract => 5,
            SemanticTokenModifier::Async => 6,
            SemanticTokenModifier::Modification => 7,
            SemanticTokenModifier::Documentation => 8,
            SemanticTokenModifier::DefaultLibrary => 9,
        };
        
        result |= 1 << bit;
    }
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_document_parsing() {
        let mut server = LanguageServer::new();
        
        let code = r#"
package Test;

my $x = 42;

sub foo {
    return $x;
}
"#;
        
        server.open_document("file:///test.pl".to_string(), 1, code.to_string());
        
        let doc = server.documents.get("file:///test.pl").unwrap();
        assert!(doc.ast.is_some());
        assert!(doc.analyzer.is_some());
    }
    
    #[test]
    fn test_goto_definition() {
        let mut server = LanguageServer::new();
        
        let code = r#"my $x = 42;
print $x;"#;
        
        server.open_document("file:///test.pl".to_string(), 1, code.to_string());
        
        // Position of $x in "print $x"
        let position = Position { line: 1, character: 7 };
        
        let location = server.goto_definition("file:///test.pl", position);
        assert!(location.is_some());
        
        let loc = location.unwrap();
        assert_eq!(loc.target_range.start.line, 0);
    }
    
    #[test]
    fn test_hover() {
        let mut server = LanguageServer::new();
        
        let code = r#"my $count = 42;
print $count;"#;
        
        server.open_document("file:///test.pl".to_string(), 1, code.to_string());
        
        // Position of $count in declaration
        let position = Position { line: 0, character: 4 };
        
        let hover = server.hover("file:///test.pl", position);
        assert!(hover.is_some());
        
        let h = hover.unwrap();
        assert!(h.contents.contains("my $count"));
    }
}