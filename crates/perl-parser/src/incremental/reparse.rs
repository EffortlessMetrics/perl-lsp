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
                let shifted_start = (old_tok.start as isize + byte_shift) as usize;
                let shifted_end = (old_tok.end as isize + byte_shift) as usize;
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
            adjusted.start = (adjusted.start as isize + byte_shift) as usize;
            adjusted.end = (adjusted.end as isize + byte_shift) as usize;
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
