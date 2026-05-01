use crate::incremental::state::{IncrementalState, LineIndex};
use anyhow::Result;
use lsp_types::{Diagnostic, TextDocumentContentChangeEvent};
use perl_lexer::{LexerMode, PerlLexer, TokenType};
use perl_parser_core::ast::{Node, NodeKind, SourceLocation};
use perl_parser_core::parser::Parser;
use ropey::Rope;
use std::ops::Range;

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
