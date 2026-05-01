#![allow(dead_code)] // Scaffold for staged lexer state migration.

use perl_position_tracking::Position;

use crate::{heredoc::HeredocSpec, mode::LexerMode, quote_handler::QuoteOperatorInfo, LexerConfig};

#[derive(Clone, Debug)]
pub(crate) struct SourceView<'a> {
    pub(crate) input: &'a str,
    pub(crate) input_bytes: &'a [u8],
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct CursorState {
    pub(crate) position: usize,
    pub(crate) current_pos: Position,
    pub(crate) line_start_offset: usize,
    pub(crate) after_newline: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct LexicalContext {
    pub(crate) delimiter_stack: Vec<char>,
    pub(crate) in_prototype: bool,
    pub(crate) prototype_depth: usize,
    pub(crate) after_sub: bool,
    pub(crate) after_arrow: bool,
    pub(crate) hash_brace_depth: usize,
    pub(crate) after_var_subscript: bool,
    pub(crate) paren_depth: usize,
}

#[derive(Clone)]
pub(crate) struct HeredocState {
    pub(crate) pending_heredocs: Vec<HeredocSpec>,
    pub(crate) emit_heredoc_body_tokens: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct QuoteState {
    pub(crate) current_quote_op: Option<QuoteOperatorInfo>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct EofState {
    pub(crate) eof_emitted: bool,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct BudgetState {
    pub(crate) start_time: std::time::Instant,
}

#[derive(Clone)]
pub(crate) struct LexerState<'a> {
    pub(crate) source: SourceView<'a>,
    pub(crate) cursor: CursorState,
    pub(crate) mode: LexerMode,
    pub(crate) config: LexerConfig,
    pub(crate) context: LexicalContext,
    pub(crate) heredocs: HeredocState,
    pub(crate) quotes: QuoteState,
    pub(crate) eof: EofState,
    pub(crate) budget: BudgetState,
}

impl<'a> LexerState<'a> {
    pub(crate) fn with_config(input: &'a str, config: LexerConfig) -> Self {
        Self {
            source: SourceView {
                input,
                input_bytes: input.as_bytes(),
            },
            cursor: CursorState {
                position: 0,
                current_pos: Position::start(),
                line_start_offset: 0,
                after_newline: true,
            },
            mode: LexerMode::ExpectTerm,
            config,
            context: LexicalContext {
                delimiter_stack: Vec::new(),
                in_prototype: false,
                prototype_depth: 0,
                after_sub: false,
                after_arrow: false,
                hash_brace_depth: 0,
                after_var_subscript: false,
                paren_depth: 0,
            },
            heredocs: HeredocState {
                pending_heredocs: Vec::new(),
                emit_heredoc_body_tokens: false,
            },
            quotes: QuoteState {
                current_quote_op: None,
            },
            eof: EofState { eof_emitted: false },
            budget: BudgetState {
                start_time: std::time::Instant::now(),
            },
        }
    }

    pub(crate) fn with_body_tokens(mut self) -> Self {
        self.heredocs.emit_heredoc_body_tokens = true;
        self
    }
}
