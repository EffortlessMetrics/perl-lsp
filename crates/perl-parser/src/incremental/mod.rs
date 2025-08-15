use anyhow::Result;
use lsp_types::{Diagnostic, TextDocumentContentChangeEvent};
use ropey::Rope;
use std::ops::Range;

use crate::ast::{Node, NodeKind, SourceLocation};
use crate::parser::Parser;
use perl_lexer::{LexerMode, PerlLexer, Token, TokenType};

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
        let line = self.line_starts.binary_search(&byte).unwrap_or_else(|i| i.saturating_sub(1));
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
            Err(e) => Node::new(
                NodeKind::Error { message: e.to_string() },
                SourceLocation { start: 0, end: source.len() },
            ),
        };

        // Get tokens from lexer
        let mut lexer = PerlLexer::new(&source);
        let mut tokens = Vec::new();
        loop {
            match lexer.next_token() {
                Some(token) => {
                    if token.token_type == TokenType::EOF {
                        break;
                    }
                    tokens.push(token);
                }
                None => break,
            }
        }

        // Create initial checkpoints
        let lex_checkpoints = Self::create_lex_checkpoints(&tokens);
        let parse_checkpoints = Self::create_parse_checkpoints(&ast);

        Self { rope, line_index, lex_checkpoints, parse_checkpoints, ast, tokens, source }
    }

    /// Create lexer checkpoints at safe boundaries
    fn create_lex_checkpoints(tokens: &[Token]) -> Vec<LexCheckpoint> {
        let mut checkpoints =
            vec![LexCheckpoint { byte: 0, mode: LexerMode::ExpectTerm, line: 0, column: 0 }];

        let mut mode = LexerMode::ExpectTerm;

        for token in tokens {
            // Update mode based on token
            mode = match token.token_type {
                TokenType::Semicolon | TokenType::LeftBrace | TokenType::RightBrace => {
                    // Safe boundary - reset to ExpectTerm
                    checkpoints.push(LexCheckpoint {
                        byte: token.end,
                        mode: LexerMode::ExpectTerm,
                        line: 0, // TODO: Calculate line/column from byte position
                        column: 0,
                    });
                    LexerMode::ExpectTerm
                }
                TokenType::Keyword(ref kw) if kw.as_ref() == "sub" || kw.as_ref() == "package" => {
                    checkpoints.push(LexCheckpoint {
                        byte: token.start,
                        mode: LexerMode::ExpectTerm, // ExpectIdentifier not available
                        line: 0,                     // TODO: Calculate line/column
                        column: 0,
                    });
                    LexerMode::ExpectTerm // ExpectIdentifier not available
                }
                TokenType::Identifier(_) | TokenType::Number(_) | TokenType::StringLiteral => {
                    LexerMode::ExpectOperator
                }
                TokenType::Operator(_) => LexerMode::ExpectTerm,
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
        // Process current node
        match &node.kind {
            NodeKind::Package { name, .. } => {
                scope.package_name = name.clone();
                checkpoints.push(ParseCheckpoint {
                    byte: node.location.start,
                    scope_snapshot: scope.clone(),
                    node_id,
                });
            }
            NodeKind::Subroutine { .. } | NodeKind::Block { .. } => {
                checkpoints.push(ParseCheckpoint {
                    byte: node.location.start,
                    scope_snapshot: scope.clone(),
                    node_id,
                });
            }
            NodeKind::VariableDeclaration { variable, .. } => {
                // Extract variable name from the variable node
                if let NodeKind::Variable { name, sigil, .. } = &variable.kind {
                    // Include sigil for proper variable tracking
                    scope.locals.push(format!("{}{}", sigil, name));
                }
            }
            NodeKind::VariableListDeclaration { variables, .. } => {
                // Handle list declarations like my ($x, $y, @z)
                for var in variables {
                    if let NodeKind::Variable { name, sigil, .. } = &var.kind {
                        scope.locals.push(format!("{}{}", sigil, name));
                    }
                }
            }
            _ => {}
        }

        // Recurse into children based on node kind
        match &node.kind {
            NodeKind::Program { statements } => {
                for (i, stmt) in statements.iter().enumerate() {
                    let child_id = node_id.wrapping_mul(101).wrapping_add(i);
                    Self::walk_ast_for_checkpoints(stmt, checkpoints, scope, child_id);
                }
            }
            NodeKind::Block { statements } => {
                // Enter new scope for blocks
                let mut local_scope = scope.clone();
                for (i, stmt) in statements.iter().enumerate() {
                    let child_id = node_id.wrapping_mul(101).wrapping_add(i);
                    Self::walk_ast_for_checkpoints(stmt, checkpoints, &mut local_scope, child_id);
                }
            }
            NodeKind::Subroutine { body, .. } => {
                // Subroutine body is a single node (Block), not Vec<Node>
                let mut local_scope = scope.clone();
                let child_id = node_id.wrapping_mul(101);
                Self::walk_ast_for_checkpoints(body, checkpoints, &mut local_scope, child_id);
            }
            NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
                let base_id = node_id.wrapping_mul(101);
                Self::walk_ast_for_checkpoints(condition, checkpoints, scope, base_id);

                // then_branch is Box<Node>, not Option<Box<Node>>
                Self::walk_ast_for_checkpoints(
                    then_branch,
                    checkpoints,
                    scope,
                    base_id.wrapping_add(1),
                );

                // elsif_branches is Vec<(Box<Node>, Box<Node>)>
                for (i, (elsif_cond, elsif_block)) in elsif_branches.iter().enumerate() {
                    let elsif_base = base_id.wrapping_add((i + 2) * 2);
                    Self::walk_ast_for_checkpoints(elsif_cond, checkpoints, scope, elsif_base);
                    Self::walk_ast_for_checkpoints(
                        elsif_block,
                        checkpoints,
                        scope,
                        elsif_base.wrapping_add(1),
                    );
                }
                if let Some(else_br) = else_branch {
                    let else_id = base_id.wrapping_add((elsif_branches.len() + 2) * 2);
                    Self::walk_ast_for_checkpoints(else_br, checkpoints, scope, else_id);
                }
            }
            NodeKind::While { condition, body, .. } => {
                let base_id = node_id.wrapping_mul(101);
                Self::walk_ast_for_checkpoints(condition, checkpoints, scope, base_id);
                // body is Box<Node>, not Option<Box<Node>>
                Self::walk_ast_for_checkpoints(body, checkpoints, scope, base_id.wrapping_add(1));
            }
            NodeKind::For { init, condition, update, body, .. } => {
                let base_id = node_id.wrapping_mul(101);
                let mut offset = 0;
                if let Some(init) = init {
                    Self::walk_ast_for_checkpoints(
                        init,
                        checkpoints,
                        scope,
                        base_id.wrapping_add(offset),
                    );
                    offset += 1;
                }
                if let Some(cond) = condition {
                    Self::walk_ast_for_checkpoints(
                        cond,
                        checkpoints,
                        scope,
                        base_id.wrapping_add(offset),
                    );
                    offset += 1;
                }
                if let Some(upd) = update {
                    Self::walk_ast_for_checkpoints(
                        upd,
                        checkpoints,
                        scope,
                        base_id.wrapping_add(offset),
                    );
                    offset += 1;
                }
                // body is Box<Node>, not Option<Box<Node>>
                Self::walk_ast_for_checkpoints(
                    body,
                    checkpoints,
                    scope,
                    base_id.wrapping_add(offset),
                );
            }
            NodeKind::Binary { left, right, .. } => {
                let base_id = node_id.wrapping_mul(101);
                Self::walk_ast_for_checkpoints(left, checkpoints, scope, base_id);
                Self::walk_ast_for_checkpoints(right, checkpoints, scope, base_id.wrapping_add(1));
            }
            NodeKind::Assignment { lhs, rhs, .. } => {
                let base_id = node_id.wrapping_mul(101);
                Self::walk_ast_for_checkpoints(lhs, checkpoints, scope, base_id);
                Self::walk_ast_for_checkpoints(rhs, checkpoints, scope, base_id.wrapping_add(1));
            }
            NodeKind::VariableDeclaration { initializer, .. } => {
                if let Some(init) = initializer {
                    let child_id = node_id.wrapping_mul(101);
                    Self::walk_ast_for_checkpoints(init, checkpoints, scope, child_id);
                }
            }
            // Add more cases as needed
            _ => {}
        }
    }

    /// Find the best checkpoint before a given byte offset
    pub fn find_lex_checkpoint(&self, byte: usize) -> Option<&LexCheckpoint> {
        self.lex_checkpoints.iter().rev().find(|cp| cp.byte <= byte)
    }

    /// Find the best parse checkpoint before a given byte offset
    pub fn find_parse_checkpoint(&self, byte: usize) -> Option<&ParseCheckpoint> {
        self.parse_checkpoints.iter().rev().find(|cp| cp.byte <= byte)
    }
}

/// Edit description
#[derive(Clone, Debug)]
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
            let start_byte = line_index
                .position_to_byte(range.start.line as usize, range.start.character as usize)?;
            let old_end_byte = line_index
                .position_to_byte(range.end.line as usize, range.end.character as usize)?;
            let new_end_byte = start_byte + change.text.len();

            Some(Edit { start_byte, old_end_byte, new_end_byte, new_text: change.text.clone() })
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
pub fn apply_edits(state: &mut IncrementalState, edits: &[Edit]) -> Result<ReparseResult> {
    // Handle multiple edits by sorting and applying in reverse order
    let mut sorted_edits = edits.to_vec();
    sorted_edits.sort_by_key(|e| e.start_byte);
    sorted_edits.reverse(); // Apply from end to start to avoid offset shifts

    // Check if we should fall back to full reparse
    let total_changed = sorted_edits.iter().map(|e| e.new_text.len()).sum::<usize>();

    // Fallback thresholds
    const MAX_EDIT_SIZE: usize = 64 * 1024; // 64KB
    #[allow(dead_code)] // TODO: Use for checkpoint-based incremental parsing
    const MAX_TOUCHED_CHECKPOINTS: usize = 20;

    if total_changed > MAX_EDIT_SIZE {
        return full_reparse(state);
    }

    // For MVP, handle single edit with incremental logic
    if sorted_edits.len() == 1 {
        let edit = &sorted_edits[0];

        // Heuristic: if edit is large (>1KB) or crosses many lines, do full reparse
        if edit.new_text.len() > 1024 || edit.new_text.matches('\n').count() > 10 {
            apply_single_edit(state, edit)?;
            return full_reparse(state);
        }

        // Find reparse window
        let window = find_reparse_window(state, edit)?;

        // If window is too large (>20% of doc), fall back to full parse
        if window.end - window.start > state.source.len() / 5 {
            apply_single_edit(state, edit)?;
            return full_reparse(state);
        }

        // Apply the edit to the source
        let mut new_source = String::with_capacity(
            state.source.len() - (edit.old_end_byte - edit.start_byte) + edit.new_text.len(),
        );
        new_source.push_str(&state.source[..edit.start_byte]);
        new_source.push_str(&edit.new_text);
        new_source.push_str(&state.source[edit.old_end_byte..]);

        // Re-lex the window
        let _window_text = &new_source[window.clone()];
        let _checkpoint = state
            .find_lex_checkpoint(window.start)
            .ok_or_else(|| anyhow::anyhow!("No checkpoint found"))?;

        // For now, fall back to full reparse
        // TODO: Implement actual incremental lexing and parsing
        state.source = new_source;
        state.rope = Rope::from_str(&state.source);
        state.line_index = LineIndex::new(&state.source);
        full_reparse(state)
    } else {
        // Multiple edits - apply them in sequence
        for edit in sorted_edits {
            apply_single_edit(state, &edit)?;
        }
        full_reparse(state)
    }
}

/// Apply a single edit to the state
fn apply_single_edit(state: &mut IncrementalState, edit: &Edit) -> Result<()> {
    let mut new_source = String::with_capacity(
        state.source.len() - (edit.old_end_byte - edit.start_byte) + edit.new_text.len(),
    );
    new_source.push_str(&state.source[..edit.start_byte]);
    new_source.push_str(&edit.new_text);
    new_source.push_str(&state.source[edit.old_end_byte..]);
    state.source = new_source;
    state.rope = Rope::from_str(&state.source);
    state.line_index = LineIndex::new(&state.source);
    Ok(())
}

/// Find the window to reparse
fn find_reparse_window(state: &IncrementalState, edit: &Edit) -> Result<Range<usize>> {
    // Find safe boundaries around the edit
    let start_checkpoint = state
        .find_lex_checkpoint(edit.start_byte)
        .ok_or_else(|| anyhow::anyhow!("No start checkpoint"))?;

    // Find next safe boundary after edit
    let end_byte = edit.new_end_byte;
    let end_checkpoint = state
        .lex_checkpoints
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
        Err(e) => Node::new(
            NodeKind::Error { message: e.to_string() },
            SourceLocation { start: 0, end: state.source.len() },
        ),
    };

    // Re-lex to get tokens
    let mut lexer = PerlLexer::new(&state.source);
    let mut tokens = Vec::new();
    loop {
        match lexer.next_token() {
            Some(token) => {
                if token.token_type == TokenType::EOF {
                    break;
                }
                tokens.push(token);
            }
            None => break,
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
