use std::collections::HashMap;
use std::ops::Range;
use lsp_types::{Diagnostic, TextDocumentContentChangeEvent};
use ropey::Rope;
use anyhow::Result;

use perl_lexer::{PerlLexer, Token, TokenType, LexerMode};
use crate::parser::Parser;
use crate::ast::{Node, NodeKind, SourceLocation};
use crate::error::ParseDiagnostic;


/// Stable restart points to avoid re-lexing the whole world
#[derive(Clone, Copy, Debug)]
pub struct LexCheckpoint {
    pub byte: usize,
    pub mode: LexerMode,
    pub line: usize,
    pub column: usize,
}

/// Scope information at a parse checkpoint
#[derive(Clone, Debug, Default)]
pub struct ScopeSnapshot {
    pub package_name: String,
    pub locals: Vec<String>,
    pub our_vars: Vec<String>,
    pub parent_isa: Vec<String>,
}

/// Parse checkpoint with scope context
#[derive(Clone, Debug)]
pub struct ParseCheckpoint {
    pub byte: usize,
    pub scope_snapshot: ScopeSnapshot,
    pub node_id: usize, // ID of AST node at this point
}

/// Line index for byte <-> (line, col) mapping
#[derive(Clone, Debug)]
pub struct LineIndex {
    /// Byte offset of each line start
    line_starts: Vec<usize>,
}

impl LineIndex {
    pub fn new(text: &str) -> Self {
        let mut line_starts = vec![0];
        for (i, ch) in text.char_indices() {
            if ch == '\n' {
                line_starts.push(i + 1);
            }
        }
        Self { line_starts }
    }

    pub fn byte_to_position(&self, byte: usize) -> (usize, usize) {
        let line = self.line_starts.binary_search(&byte)
            .unwrap_or_else(|i| i.saturating_sub(1));
        let column = byte - self.line_starts[line];
        (line, column)
    }

    pub fn position_to_byte(&self, line: usize, column: usize) -> Option<usize> {
        self.line_starts.get(line).map(|&start| start + column)
    }
}

/// Incremental parsing state
pub struct IncrementalState {
    pub rope: Rope,
    pub line_index: LineIndex,
    pub lex_checkpoints: Vec<LexCheckpoint>,
    pub parse_checkpoints: Vec<ParseCheckpoint>,
    pub ast: Node,
    pub tokens: Vec<Token>,
    pub source: String,
}

impl IncrementalState {
    pub fn new(source: String) -> Self {
        let rope = Rope::from_str(&source);
        let line_index = LineIndex::new(&source);
        
        // Parse the initial document
        let mut parser = Parser::new(&source);
        let ast = match parser.parse() {
            Ok(ast) => ast,
            Err(_) => Node::new(
                NodeKind::Error { message: "Parse error".to_string() },
                SourceLocation { start: 0, end: source.len() }
            ),
        };
        
        // Get tokens from lexer
        let mut lexer = PerlLexer::new(&source);
        let mut tokens = Vec::new();
        loop {
            match lexer.next_token() {
                Ok(token) => {
                    if token.token_type == TokenType::Eof {
                        break;
                    }
                    tokens.push(token);
                }
                Err(_) => break,
            }
        }
        
        // Create initial checkpoints
        let lex_checkpoints = Self::create_lex_checkpoints(&tokens);
        let parse_checkpoints = Self::create_parse_checkpoints(&ast);
        
        Self {
            rope,
            line_index,
            lex_checkpoints,
            parse_checkpoints,
            ast,
            tokens,
            source,
        }
    }

    /// Create lexer checkpoints at safe boundaries
    fn create_lex_checkpoints(tokens: &[Token]) -> Vec<LexCheckpoint> {
        let mut checkpoints = vec![LexCheckpoint {
            byte: 0,
            mode: LexerMode::ExpectTerm,
            line: 0,
            column: 0,
        }];
        
        let mut mode = LexerMode::ExpectTerm;
        
        for token in tokens {
            // Update mode based on token
            mode = match token.token_type {
                TokenType::Semicolon | TokenType::LeftBrace | TokenType::RightBrace => {
                    // Safe boundary - reset to ExpectTerm
                    checkpoints.push(LexCheckpoint {
                        byte: token.span.end,
                        mode: LexerMode::ExpectTerm,
                        line: token.span.end_line,
                        column: token.span.end_column,
                    });
                    LexerMode::ExpectTerm
                }
                TokenType::Keyword if matches!(token.text.as_str(), "sub" | "package") => {
                    checkpoints.push(LexCheckpoint {
                        byte: token.span.start,
                        mode: LexerMode::ExpectIdentifier,
                        line: token.span.start_line,
                        column: token.span.start_column,
                    });
                    LexerMode::ExpectIdentifier
                }
                TokenType::Variable | TokenType::Number | TokenType::String => {
                    LexerMode::ExpectOperator
                }
                TokenType::Operator => LexerMode::ExpectTerm,
                _ => mode,
            };
        }
        
        checkpoints
    }

    /// Create parse checkpoints at statement boundaries
    fn create_parse_checkpoints(ast: &Node) -> Vec<ParseCheckpoint> {
        let mut checkpoints = vec![];
        let mut scope = ScopeSnapshot::default();
        
        Self::walk_ast_for_checkpoints(ast, &mut checkpoints, &mut scope, 0);
        checkpoints
    }

    fn walk_ast_for_checkpoints(
        node: &Node,
        checkpoints: &mut Vec<ParseCheckpoint>,
        scope: &mut ScopeSnapshot,
        node_id: usize,
    ) {
        match &node.kind {
            NodeKind::Package { name, .. } => {
                scope.package_name = name.clone();
                checkpoints.push(ParseCheckpoint {
                    byte: node.span.start,
                    scope_snapshot: scope.clone(),
                    node_id,
                });
            }
            NodeKind::Subroutine { .. } | NodeKind::Block { .. } => {
                checkpoints.push(ParseCheckpoint {
                    byte: node.span.start,
                    scope_snapshot: scope.clone(),
                    node_id,
                });
            }
            NodeKind::VariableDeclaration { name, .. } => {
                if let Some(name) = name {
                    scope.locals.push(name.clone());
                }
            }
            _ => {}
        }
        
        // Recurse into children
        for (i, child) in node.children.iter().enumerate() {
            Self::walk_ast_for_checkpoints(child, checkpoints, scope, node_id * 100 + i);
        }
    }

    /// Find the best checkpoint before a given byte offset
    pub fn find_lex_checkpoint(&self, byte: usize) -> Option<&LexCheckpoint> {
        self.lex_checkpoints
            .iter()
            .rev()
            .find(|cp| cp.byte <= byte)
    }

    /// Find the best parse checkpoint before a given byte offset
    pub fn find_parse_checkpoint(&self, byte: usize) -> Option<&ParseCheckpoint> {
        self.parse_checkpoints
            .iter()
            .rev()
            .find(|cp| cp.byte <= byte)
    }
}

/// Edit description
#[derive(Debug, Clone)]
pub struct Edit {
    pub start_byte: usize,
    pub old_end_byte: usize,
    pub new_end_byte: usize,
    pub new_text: String,
}

impl Edit {
    /// Convert LSP change to Edit
    pub fn from_lsp_change(
        change: &TextDocumentContentChangeEvent,
        line_index: &LineIndex,
        old_text: &str,
    ) -> Option<Self> {
        if let Some(range) = change.range {
            let start_byte = line_index.position_to_byte(
                range.start.line as usize,
                range.start.character as usize,
            )?;
            let old_end_byte = line_index.position_to_byte(
                range.end.line as usize,
                range.end.character as usize,
            )?;
            let new_end_byte = start_byte + change.text.len();
            
            Some(Edit {
                start_byte,
                old_end_byte,
                new_end_byte,
                new_text: change.text.clone(),
            })
        } else {
            // Full document change
            Some(Edit {
                start_byte: 0,
                old_end_byte: old_text.len(),
                new_end_byte: change.text.len(),
                new_text: change.text.clone(),
            })
        }
    }
}

/// Result of incremental reparse
#[derive(Debug)]
pub struct ReparseResult {
    pub changed_ranges: Vec<Range<usize>>,
    pub diagnostics: Vec<Diagnostic>,
    pub reparsed_bytes: usize,
}

/// Apply edits incrementally
pub fn apply_edits(
    state: &mut IncrementalState,
    edits: &[Edit],
) -> Result<ReparseResult> {
    // For MVP, we'll handle single edits first
    if edits.len() != 1 {
        return full_reparse(state);
    }
    
    let edit = &edits[0];
    
    // Heuristic: if edit is large (>1KB) or crosses many lines, do full reparse
    if edit.new_text.len() > 1024 || edit.new_text.matches('\n').count() > 10 {
        return full_reparse(state);
    }
    
    // Find reparse window
    let window = find_reparse_window(state, edit)?;
    
    // If window is too large (>20% of doc), fall back to full parse
    if window.end - window.start > state.source.len() / 5 {
        return full_reparse(state);
    }
    
    // Apply the edit to the source
    let mut new_source = String::with_capacity(
        state.source.len() - (edit.old_end_byte - edit.start_byte) + edit.new_text.len()
    );
    new_source.push_str(&state.source[..edit.start_byte]);
    new_source.push_str(&edit.new_text);
    new_source.push_str(&state.source[edit.old_end_byte..]);
    
    // Re-lex the window
    let window_text = &new_source[window.clone()];
    let checkpoint = state.find_lex_checkpoint(window.start)
        .ok_or_else(|| anyhow::anyhow!("No checkpoint found"))?;
    
    // For now, fall back to full reparse
    // TODO: Implement actual incremental lexing and parsing
    state.source = new_source;
    full_reparse(state)
}

/// Find the window to reparse
fn find_reparse_window(state: &IncrementalState, edit: &Edit) -> Result<Range<usize>> {
    // Find safe boundaries around the edit
    let start_checkpoint = state.find_lex_checkpoint(edit.start_byte)
        .ok_or_else(|| anyhow::anyhow!("No start checkpoint"))?;
    
    // Find next safe boundary after edit
    let end_byte = edit.new_end_byte;
    let end_checkpoint = state.lex_checkpoints
        .iter()
        .find(|cp| cp.byte > end_byte)
        .map(|cp| cp.byte)
        .unwrap_or(state.source.len());
    
    Ok(start_checkpoint.byte..end_checkpoint)
}

/// Full document reparse fallback
fn full_reparse(state: &mut IncrementalState) -> Result<ReparseResult> {
    let mut parser = Parser::new(&state.source);
    state.ast = match parser.parse() {
        Ok(ast) => ast,
        Err(_) => Node::new(
            NodeKind::Error { message: "Parse error".to_string() },
            SourceLocation { start: 0, end: state.source.len() }
        ),
    };
    
    // Re-lex to get tokens
    let mut lexer = PerlLexer::new(&state.source);
    let mut tokens = Vec::new();
    loop {
        match lexer.next_token() {
            Ok(token) => {
                if token.token_type == TokenType::Eof {
                    break;
                }
                tokens.push(token);
            }
            Err(_) => break,
        }
    }
    state.tokens = tokens;
    
    state.rope = Rope::from_str(&state.source);
    state.line_index = LineIndex::new(&state.source);
    
    // Rebuild checkpoints
    state.lex_checkpoints = IncrementalState::create_lex_checkpoints(&state.tokens);
    state.parse_checkpoints = IncrementalState::create_parse_checkpoints(&state.ast);
    
    // No diagnostics for now, will be handled by the LSP server
    let diagnostics = vec![];
    
    Ok(ReparseResult {
        changed_ranges: vec![0..state.source.len()],
        diagnostics,
        reparsed_bytes: state.source.len(),
    })
}