use crate::incremental::strategy::{should_force_full_reparse, MAX_EDIT_SIZE};
use crate::incremental::{Edit, IncrementalState, LineIndex, ReparseResult};
use anyhow::Result;
use perl_lexer::{PerlLexer, TokenType};
use perl_parser_core::ast::{Node, NodeKind, SourceLocation};
use perl_parser_core::parser::Parser;
use ropey::Rope;
use std::ops::Range;

/// Apply edits incrementally
pub fn apply_edits(state: &mut IncrementalState, edits: &[Edit]) -> Result<ReparseResult> {
    let mut sorted_edits = edits.to_vec();
    sorted_edits.sort_by_key(|e| e.start_byte);
    sorted_edits.reverse();

    let total_changed = sorted_edits.iter().map(Edit::touched_bytes).sum::<usize>();
    if total_changed > MAX_EDIT_SIZE {
        return full_reparse(state);
    }

    if sorted_edits.len() == 1 {
        let edit = &sorted_edits[0];
        if should_force_full_reparse(edit.touched_bytes(), &edit.new_text) {
            apply_single_edit(state, edit)?;
            return full_reparse(state);
        }

        let reparsed_range = apply_single_edit(state, edit)?;
        let reparsed_bytes = reparsed_range.end - reparsed_range.start;

        Ok(ReparseResult {
            changed_ranges: vec![reparsed_range],
            diagnostics: vec![],
            reparsed_bytes,
        })
    } else {
        for edit in sorted_edits {
            apply_single_edit(state, &edit)?;
        }
        full_reparse(state)
    }
}

fn apply_single_edit(state: &mut IncrementalState, edit: &Edit) -> Result<Range<usize>> {
    let checkpoint = state
        .find_lex_checkpoint(edit.start_byte)
        .copied()
        .ok_or_else(|| anyhow::anyhow!("No checkpoint found"))?;

    let old_end_byte = edit.old_end_byte.min(state.source.len());
    let start_byte = edit.start_byte.min(state.source.len());
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

    let start_idx =
        state.tokens.iter().position(|t| t.start >= checkpoint.byte).unwrap_or(state.tokens.len());

    let edit_end_in_new = start_byte + edit.new_text.len();
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

                if token.start >= edit_end_in_new {
                    let mut found_sync = false;
                    for (idx_offset, old_tok) in state.tokens[old_sync_start..].iter().enumerate() {
                        let shifted_start = (old_tok.start as isize + byte_shift) as usize;
                        let shifted_end = (old_tok.end as isize + byte_shift) as usize;
                        if token.start == shifted_start
                            && token.end == shifted_end
                            && token.token_type == old_tok.token_type
                        {
                            found_sync = true;
                            sync_old_idx = old_sync_start + idx_offset + 1;
                            break;
                        }
                    }
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
    state.lex_checkpoints =
        IncrementalState::create_lex_checkpoints(&state.tokens, &state.line_index);

    Ok(checkpoint.byte..last_token_end)
}

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
    state.lex_checkpoints =
        IncrementalState::create_lex_checkpoints(&state.tokens, &state.line_index);
    state.parse_checkpoints = IncrementalState::create_parse_checkpoints(&state.ast);

    Ok(ReparseResult {
        changed_ranges: vec![0..state.source.len()],
        diagnostics: vec![],
        reparsed_bytes: state.source.len(),
    })
}
