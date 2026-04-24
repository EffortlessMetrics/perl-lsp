#![allow(missing_docs)]

use anyhow::Result;
use lsp_types::Diagnostic;
use lsp_types::TextDocumentContentChangeEvent;
pub use perl_line_index::LineIndex;
use ropey::Rope;
use std::ops::Range;

use perl_lexer::{LexerMode, PerlLexer, Token, TokenType};
use perl_parser_core::ast::{Node, NodeKind, SourceLocation};
use perl_parser_core::parser::Parser;

pub mod incremental_advanced_reuse;
pub mod incremental_checkpoint;
pub mod incremental_document;
pub mod incremental_edit;
pub mod incremental_handler_v2;
pub mod incremental_integration;
pub mod incremental_simple;
pub mod incremental_v2;

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

/// Incremental parsing state
#[derive(Clone)]
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
                NodeKind::Error {
                    message: e.to_string(),
                    expected: vec![],
                    found: None,
                    partial: None,
                },
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
        let lex_checkpoints = Self::create_lex_checkpoints(&tokens, &line_index);
        let parse_checkpoints = Self::create_parse_checkpoints(&ast);

        Self { rope, line_index, lex_checkpoints, parse_checkpoints, ast, tokens, source }
    }

    /// Create lexer checkpoints at safe boundaries
    fn create_lex_checkpoints(tokens: &[Token], line_index: &LineIndex) -> Vec<LexCheckpoint> {
        let mut checkpoints =
            vec![LexCheckpoint { byte: 0, mode: LexerMode::ExpectTerm, line: 0, column: 0 }];

        let mut mode = LexerMode::ExpectTerm;

        for token in tokens {
            // Update mode based on token
            mode = match token.token_type {
                TokenType::Semicolon | TokenType::LeftBrace | TokenType::RightBrace => {
                    // Safe boundary - reset to ExpectTerm
                    let (line, column) = line_index.byte_to_position(token.end);
                    checkpoints.push(LexCheckpoint {
                        byte: token.end,
                        mode: LexerMode::ExpectTerm,
                        line,
                        column,
                    });
                    LexerMode::ExpectTerm
                }
                TokenType::Keyword(ref kw) if kw.as_ref() == "sub" || kw.as_ref() == "package" => {
                    let (line, column) = line_index.byte_to_position(token.start);
                    checkpoints.push(LexCheckpoint {
                        byte: token.start,
                        mode: LexerMode::ExpectTerm, // ExpectIdentifier not available
                        line,
                        column,
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

impl Edit {
    /// Returns the size of text touched by this edit (inserted or deleted).
    ///
    /// This is used for fallback heuristics so that large deletions are treated
    /// the same as large insertions.
    fn touched_bytes(&self) -> usize {
        let replaced_len = self.old_end_byte.saturating_sub(self.start_byte);
        replaced_len.max(self.new_text.len())
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
    let total_changed = sorted_edits.iter().map(Edit::touched_bytes).sum::<usize>();

    // Fallback thresholds
    const MAX_EDIT_SIZE: usize = 64 * 1024; // 64KB

    if total_changed > MAX_EDIT_SIZE {
        return full_reparse(state);
    }

    // For MVP, handle single edit with incremental logic
    if sorted_edits.len() == 1 {
        let edit = &sorted_edits[0];

        // Heuristic: if edit is large (>1KB) or crosses many lines, do full reparse
        if edit.touched_bytes() > 1024 || edit.new_text.matches('\n').count() > 10 {
            apply_single_edit(state, edit)?;
            return full_reparse(state);
        }

        // Apply the edit with incremental lexing
        let reparsed_range = apply_single_edit(state, edit)?;
        let reparsed_bytes = reparsed_range.end - reparsed_range.start;

        // If reparsed too much (>20% of doc), might need full parse in future
        // But for now, trust the incremental result

        Ok(ReparseResult {
            changed_ranges: vec![reparsed_range],
            diagnostics: vec![],
            reparsed_bytes,
        })
    } else {
        // Multiple edits - apply them in sequence
        for edit in sorted_edits {
            apply_single_edit(state, &edit)?;
        }
        full_reparse(state)
    }
}

/// Apply a single edit to the state
fn apply_single_edit(state: &mut IncrementalState, edit: &Edit) -> Result<Range<usize>> {
    // Find checkpoint before edit to resume lexing
    let checkpoint = state
        .find_lex_checkpoint(edit.start_byte)
        .copied()
        .ok_or_else(|| anyhow::anyhow!("No checkpoint found"))?;

    // Apply text edit with safe boundary checking
    let old_end_byte = edit.old_end_byte.min(state.source.len());
    let start_byte = edit.start_byte.min(state.source.len());

    // Compute the byte shift caused by this edit
    let byte_shift: isize = edit.new_text.len() as isize - (old_end_byte - start_byte) as isize;

    let mut new_source = String::with_capacity(
        state.source.len() - (old_end_byte - start_byte) + edit.new_text.len(),
    );
    new_source.push_str(&state.source[..start_byte]);
    new_source.push_str(&edit.new_text);
    new_source.push_str(&state.source[old_end_byte..]);
    state.source = new_source;
    state.rope = Rope::from_str(&state.source);
    state.line_index = LineIndex::new(&state.source);

    // Start lexing from checkpoint
    use perl_lexer::{Checkpointable, LexerCheckpoint, Position};
    let mut lexer = PerlLexer::new(&state.source);
    let mut lex_cp = LexerCheckpoint::new();
    lex_cp.position = checkpoint.byte;
    lex_cp.mode = checkpoint.mode;
    lex_cp.current_pos = Position {
        byte: checkpoint.byte,
        line: (checkpoint.line + 1) as u32,
        column: (checkpoint.column + 1) as u32,
    };
    lexer.restore(&lex_cp);

    // Determine token index to splice
    let start_idx =
        state.tokens.iter().position(|t| t.start >= checkpoint.byte).unwrap_or(state.tokens.len());

    // Build a lookup of old tokens past the edit for synchronisation.
    // Once a newly-lexed token matches an old token (shifted by the byte delta),
    // we can stop re-lexing and reuse the remaining old tokens with adjusted positions.
    let edit_end_in_new = start_byte + edit.new_text.len();

    // Find the first old-token index whose start >= old_end_byte (i.e. fully past the edit)
    let old_sync_start =
        state.tokens.iter().position(|t| t.start >= old_end_byte).unwrap_or(state.tokens.len());

    let mut new_tokens = Vec::new();
    let mut last_token_end = checkpoint.byte;
    let mut synced = false;
    let mut sync_old_idx = state.tokens.len();
    loop {
        match lexer.next_token() {
            Some(token) => {
                if token.token_type == TokenType::EOF {
                    break;
                }
                last_token_end = token.end;

                // Once we are past the edit region, check if the new token matches
                // an old token (shifted by byte_shift). If so, we can stop re-lexing
                // and reuse the rest of the old tokens with position adjustment.
                if token.start >= edit_end_in_new {
                    let mut found_sync = false;
                    for (idx_offset, old_tok) in state.tokens[old_sync_start..].iter().enumerate() {
                        let shifted_start = (old_tok.start as isize + byte_shift) as usize;
                        let shifted_end = (old_tok.end as isize + byte_shift) as usize;
                        if token.start == shifted_start
                            && token.end == shifted_end
                            && token.token_type == old_tok.token_type
                        {
                            // Token synchronised -- reuse the rest
                            found_sync = true;
                            sync_old_idx = old_sync_start + idx_offset + 1;
                            break;
                        }
                    }
                    // Push the token regardless (it's valid either way)
                    new_tokens.push(token);
                    if found_sync {
                        synced = true;
                        break;
                    }
                } else {
                    new_tokens.push(token);
                }
            }
            None => break,
        }
    }

    // If we synchronised, append remaining old tokens with shifted positions
    if synced {
        for old_tok in &state.tokens[sync_old_idx..] {
            let mut adjusted = old_tok.clone();
            adjusted.start = (adjusted.start as isize + byte_shift) as usize;
            adjusted.end = (adjusted.end as isize + byte_shift) as usize;
            last_token_end = adjusted.end;
            new_tokens.push(adjusted);
        }
    }

    state.tokens.splice(start_idx.., new_tokens);

    // Rebuild checkpoints with updated line index
    state.lex_checkpoints =
        IncrementalState::create_lex_checkpoints(&state.tokens, &state.line_index);

    // Return the actual reparsed range (from checkpoint to end of last new token)
    Ok(checkpoint.byte..last_token_end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[derive(Clone, Debug)]
    struct FuzzEdit {
        start: usize,
        delete_len: usize,
        insert_text: String,
    }

    fn apply_edit_to_ground_truth(source: &mut String, edit: &FuzzEdit) {
        let start = edit.start.min(source.len());
        let old_end = (start + edit.delete_len).min(source.len());
        source.replace_range(start..old_end, &edit.insert_text);
    }

    /// Verify that a small ranged edit re-lexes substantially fewer bytes than
    /// the total document length, confirming the checkpoint fast-path fires.
    #[test]
    fn test_incremental_state_small_edit_uses_checkpoint() -> Result<()> {
        // Build a document large enough that a checkpoint will exist before the edit.
        // We need at least one statement-boundary token (semicolon / brace) to create
        // a checkpoint entry that sits before the edit site.
        let mut lines = Vec::with_capacity(30);
        for i in 0..30usize {
            lines.push(format!("my $var_{i} = {i};"));
        }
        let source = lines.join("\n");
        let doc_len = source.len();

        let mut state = IncrementalState::new(source.clone());

        // Sanity: we should have at least one lex checkpoint from all those semicolons.
        assert!(
            state.lex_checkpoints.len() > 1,
            "expected multiple lex checkpoints, got {}",
            state.lex_checkpoints.len()
        );

        // Edit the LAST line: change `my $var_29 = 29;` -> `my $var_29 = 999;`
        // The checkpoint at some semicolon earlier in the doc should be used, meaning
        // we re-lex far less than the full document.
        let edit_text = "999";
        let edit_start = source.rfind("29;").unwrap_or(source.len() - 3);
        let edit_end = edit_start + 2; // replace "29"

        let edit = Edit {
            start_byte: edit_start,
            old_end_byte: edit_end,
            new_end_byte: edit_start + edit_text.len(),
            new_text: edit_text.to_string(),
        };

        let result = apply_edits(&mut state, &[edit])?;

        // The key assertion: reparsed_bytes must be strictly less than the full
        // document size, proving the incremental path was taken.
        assert!(
            result.reparsed_bytes < doc_len,
            "incremental reparse should cover less than the full document ({} bytes reparsed, {} doc len)",
            result.reparsed_bytes,
            doc_len
        );

        // Source must reflect the edit.
        assert!(state.source.contains("999"), "source must contain the new value after edit");
        assert!(
            !state.source.contains("= 29;"),
            "source must not contain the old value after edit"
        );

        Ok(())
    }

    /// Verify that the full-reparse fallback is triggered for an edit >64KB.
    #[test]
    fn test_incremental_state_large_edit_falls_back_to_full_reparse() -> Result<()> {
        let source = "my $x = 1;\n".repeat(10);
        let mut state = IncrementalState::new(source.clone());

        // A new_text larger than 64KB must trigger full reparse
        let big_text = "x".repeat(65 * 1024);
        let edit = Edit {
            start_byte: 0,
            old_end_byte: 0,
            new_end_byte: big_text.len(),
            new_text: big_text.clone(),
        };

        let result = apply_edits(&mut state, &[edit])?;

        // Full reparse: reparsed_bytes == full (new) source length
        assert_eq!(
            result.reparsed_bytes,
            state.source.len(),
            "large edit must trigger full reparse, reparsed_bytes should equal doc length"
        );

        Ok(())
    }

    /// Verify large deletions trigger full-reparse fallback the same as large insertions.
    #[test]
    fn test_incremental_state_large_deletion_falls_back_to_full_reparse() -> Result<()> {
        let source = "my $x = 1;\n".repeat(10_000);
        let mut state = IncrementalState::new(source);

        // Delete almost the entire document; this should use full-reparse fallback
        // to keep incremental behavior predictable for large edits.
        let old_end_byte = state.source.len().saturating_sub(1);
        let edit = Edit { start_byte: 0, old_end_byte, new_end_byte: 0, new_text: String::new() };

        let result = apply_edits(&mut state, &[edit])?;

        assert_eq!(
            result.reparsed_bytes,
            state.source.len(),
            "large deletion must trigger full reparse, reparsed_bytes should equal doc length"
        );

        Ok(())
    }

    proptest! {
        #[test]
        fn prop_incremental_apply_edits_matches_ground_truth(
            edits in prop::collection::vec(
                (
                    0usize..900usize,
                    0usize..80usize,
                    "[a-zA-Z0-9_ \\n;\\$\\(\\)\\{\\}\\[\\],]{0,40}"
                ),
                1..60
            )
        ) {
            let mut state = IncrementalState::new("my $seed = 0;\n".repeat(80));
            let mut expected_source = state.source.clone();

            for (start, delete_len, insert_text) in edits {
                let fuzz_edit = FuzzEdit { start, delete_len, insert_text };
                let start_byte = fuzz_edit.start.min(state.source.len());
                let old_end_byte = (start_byte + fuzz_edit.delete_len).min(state.source.len());

                apply_edit_to_ground_truth(&mut expected_source, &fuzz_edit);

                let incremental_edit = Edit {
                    start_byte,
                    old_end_byte,
                    new_end_byte: start_byte + fuzz_edit.insert_text.len(),
                    new_text: fuzz_edit.insert_text,
                };

                let result = apply_edits(&mut state, &[incremental_edit]);
                prop_assert!(result.is_ok());
                prop_assert_eq!(&state.source, &expected_source);
            }

            let reparsed = IncrementalState::new(expected_source);
            prop_assert_eq!(&state.source, &reparsed.source);
        }
    }
}

/// Full document reparse fallback
fn full_reparse(state: &mut IncrementalState) -> Result<ReparseResult> {
    let mut parser = Parser::new(&state.source);
    state.ast = match parser.parse() {
        Ok(ast) => ast,
        Err(e) => Node::new(
            NodeKind::Error {
                message: e.to_string(),
                expected: vec![],
                found: None,
                partial: None,
            },
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
    state.lex_checkpoints =
        IncrementalState::create_lex_checkpoints(&state.tokens, &state.line_index);
    state.parse_checkpoints = IncrementalState::create_parse_checkpoints(&state.ast);

    // No diagnostics for now, will be handled by the LSP server
    let diagnostics = vec![];

    Ok(ReparseResult {
        changed_ranges: vec![0..state.source.len()],
        diagnostics,
        reparsed_bytes: state.source.len(),
    })
}
