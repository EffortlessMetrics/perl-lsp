use crate::incremental::{
    IncrementalState, diagnostics::ReparseResult, edit::Edit, lex::create_lex_checkpoints,
};
use anyhow::Result;
use perl_lexer::{PerlLexer, TokenType};
use perl_parser_core::ast::{Node, NodeKind, SourceLocation};
use perl_parser_core::parser::Parser;
use ropey::Rope;
use std::ops::Range;

pub(crate) struct SingleEditReparse {
    pub(crate) range: Range<usize>,
    pub(crate) reused_tokens: usize,
    pub(crate) token_count: usize,
}

/// Apply a (possibly negative) `byte_shift` to a token offset without wrapping.
///
/// A negative `byte_shift` whose magnitude exceeds `offset` would underflow when
/// `(offset as isize + byte_shift)` is cast back to `usize`, silently wrapping to
/// a huge value and corrupting the token stream. Clamp the result to 0 instead so
/// the shifted offset stays valid (fixes #2471).
fn shift_offset(offset: usize, byte_shift: isize) -> usize {
    (offset as isize).saturating_add(byte_shift).max(0) as usize
}

pub(crate) fn apply_text_edit_to_state(state: &mut IncrementalState, edit: &Edit) -> Result<()> {
    let old_end = edit.old_end_byte.min(state.source.len());
    let start = edit.start_byte.min(state.source.len());
    if !state.source.is_char_boundary(start) || !state.source.is_char_boundary(old_end) {
        anyhow::bail!("edit range is not on UTF-8 boundaries");
    }

    let mut new_source =
        String::with_capacity(state.source.len() - (old_end - start) + edit.new_text.len());
    new_source.push_str(&state.source[..start]);
    new_source.push_str(&edit.new_text);
    new_source.push_str(&state.source[old_end..]);
    state.source = new_source;
    state.rope = Rope::from_str(&state.source);
    state.line_index = perl_line_index::LineIndex::new(&state.source);

    Ok(())
}

pub(crate) fn apply_single_edit(
    state: &mut IncrementalState,
    edit: &Edit,
) -> Result<SingleEditReparse> {
    let Some(cp) = state.find_lex_checkpoint(edit.start_byte).copied() else {
        apply_text_edit_to_state(state, edit)?;
        anyhow::bail!("No checkpoint found");
    };
    let old_end = edit.old_end_byte.min(state.source.len());
    let start = edit.start_byte.min(state.source.len());
    let byte_shift = edit.new_text.len() as isize - (old_end - start) as isize;
    apply_text_edit_to_state(state, edit)?;

    use perl_lexer::{Checkpointable, LexerCheckpoint, Position};
    let mut lexer = PerlLexer::new(&state.source);
    let mut lex_cp = LexerCheckpoint::new();
    lex_cp.position = cp.byte;
    lex_cp.mode = cp.mode;
    lex_cp.current_pos =
        Position { byte: cp.byte, line: (cp.line + 1) as u32, column: (cp.column + 1) as u32 };
    lexer.restore(&lex_cp);
    let start_idx =
        state.tokens.iter().position(|t| t.start >= cp.byte).unwrap_or(state.tokens.len());
    let edit_end_in_new = start + edit.new_text.len();
    let old_sync_start =
        state.tokens.iter().position(|t| t.start >= old_end).unwrap_or(state.tokens.len());
    let mut new_tokens = Vec::new();
    let mut last = cp.byte;
    let mut synced = false;
    let mut sync_old_idx = state.tokens.len();
    while let Some(token) = lexer.next_token() {
        if token.token_type == TokenType::EOF {
            break;
        }
        if token.end <= last {
            anyhow::bail!("incremental lexer did not advance at byte {}", token.start);
        }
        last = token.end;
        if token.start >= edit_end_in_new {
            let mut found = false;
            for (off, old_tok) in state.tokens[old_sync_start..].iter().enumerate() {
                let shifted_start = shift_offset(old_tok.start, byte_shift);
                let shifted_end = shift_offset(old_tok.end, byte_shift);
                if token.start == shifted_start
                    && token.end == shifted_end
                    && token.token_type == old_tok.token_type
                {
                    found = true;
                    sync_old_idx = old_sync_start + off + 1;
                    break;
                }
            }
            new_tokens.push(token);
            if found {
                synced = true;
                break;
            }
        } else {
            new_tokens.push(token);
        }
    }
    let reused_tokens = if synced { state.tokens.len().saturating_sub(sync_old_idx) } else { 0 };
    if synced {
        for old_tok in &state.tokens[sync_old_idx..] {
            let mut adjusted = old_tok.clone();
            adjusted.start = shift_offset(adjusted.start, byte_shift);
            adjusted.end = shift_offset(adjusted.end, byte_shift);
            last = adjusted.end;
            new_tokens.push(adjusted);
        }
    }
    state.tokens.splice(start_idx.., new_tokens);
    state.lex_checkpoints = create_lex_checkpoints(&state.tokens, &state.line_index);
    Ok(SingleEditReparse { range: cp.byte..last, reused_tokens, token_count: state.tokens.len() })
}

pub(crate) fn full_reparse(state: &mut IncrementalState) -> Result<ReparseResult> {
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
    while let Some(token) = lexer.next_token() {
        if token.token_type == TokenType::EOF {
            break;
        }
        tokens.push(token);
    }
    state.tokens = tokens;
    state.rope = Rope::from_str(&state.source);
    state.line_index = perl_line_index::LineIndex::new(&state.source);
    state.lex_checkpoints = create_lex_checkpoints(&state.tokens, &state.line_index);
    state.parse_checkpoints = IncrementalState::create_parse_checkpoints(&state.ast);
    Ok(ReparseResult {
        changed_ranges: vec![0..state.source.len()],
        diagnostics: vec![],
        reparsed_bytes: state.source.len(),
        reused_tokens: 0,
        token_count: state.tokens.len(),
    })
}

#[cfg(test)]
mod reparse_offset_tests {
    use super::*;
    use crate::incremental::IncrementalState;
    use anyhow::Result;

    #[test]
    fn shift_offset_clamps_negative_underflow_to_zero() {
        // A negative `byte_shift` whose magnitude exceeds the offset previously
        // wrapped to a huge usize via `(offset as isize + byte_shift) as usize`.
        // It must clamp to 0 instead (regression for #2471).
        assert_eq!(shift_offset(3, -10), 0);
        assert_eq!(shift_offset(0, -1), 0);
        // Exact boundary: shifting to zero is fine, one past zero clamps.
        assert_eq!(shift_offset(5, -5), 0);
        assert_eq!(shift_offset(5, -6), 0);
        // Non-underflowing shifts are unaffected.
        assert_eq!(shift_offset(5, -2), 3);
        assert_eq!(shift_offset(5, 0), 5);
        assert_eq!(shift_offset(5, 4), 9);
    }

    #[test]
    fn apply_single_edit_negative_shift_does_not_wrap_token_offsets() -> Result<()> {
        // Build a document whose tokens near the start sit at small byte offsets,
        // then delete a large run from the very beginning so `byte_shift` is
        // negative and larger in magnitude than those token positions. Before the
        // fix, the reused/sync token offsets wrapped to enormous usize values; now
        // they must stay bounded by the (much smaller) new source length.
        let source = "my $a = 1;\nmy $b = 2;\nmy $c = 3;\nmy $d = 4;\n".to_string();
        let mut state = IncrementalState::new(source.clone());

        // Delete the first statement plus newline (12 bytes) from offset 0.
        let delete_len = "my $a = 1;\n".len();
        let edit = Edit {
            start_byte: 0,
            old_end_byte: delete_len,
            new_end_byte: 0,
            new_text: String::new(),
        };

        // `apply_single_edit` may legitimately bail (e.g. no usable checkpoint),
        // in which case the wrapping branch was never reached. The invariant we
        // assert is: if it succeeds, every resulting token offset is in range.
        if apply_single_edit(&mut state, &edit).is_ok() {
            let new_len = state.source.len();
            for tok in &state.tokens {
                assert!(
                    tok.start <= new_len && tok.end <= new_len,
                    "token offset wrapped: start={}, end={}, source len={}",
                    tok.start,
                    tok.end,
                    new_len,
                );
                assert!(tok.start <= tok.end, "token start exceeds end: {tok:?}");
            }
        }
        Ok(())
    }
}
