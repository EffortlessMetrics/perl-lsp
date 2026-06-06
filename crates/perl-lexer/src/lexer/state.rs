use std::time::Instant;

use perl_position_tracking::Position;

use crate::config::LexerConfig;
use crate::heredoc::HeredocSpec;
use crate::mode::LexerMode;
use crate::quote_handler;

/// Context-aware lexer for the Perl language.
///
/// Tokenizes Perl source text with mode tracking to correctly disambiguate
/// context-sensitive constructs such as `/` (division vs. regex) and heredocs.
pub struct PerlLexer<'a> {
    pub(crate) input: &'a str,
    pub(crate) input_bytes: &'a [u8],
    pub(crate) position: usize,
    pub(crate) mode: LexerMode,
    pub(crate) config: LexerConfig,
    pub(crate) delimiter_stack: Vec<char>,
    pub(crate) in_prototype: bool,
    pub(crate) prototype_depth: usize,
    pub(crate) after_sub: bool,
    pub(crate) after_arrow: bool,
    pub(crate) hash_brace_depth: usize,
    pub(crate) after_var_subscript: bool,
    pub(crate) paren_depth: usize,
    // Preserved for checkpoint restoration even when a tokenization path does not read it directly.
    #[allow(dead_code)]
    pub(crate) current_pos: Position,
    pub(crate) after_newline: bool,
    pub(crate) pending_heredocs: Vec<HeredocSpec>,
    pub(crate) line_start_offset: usize,
    pub(crate) emit_heredoc_body_tokens: bool,
    pub(crate) current_quote_op: Option<quote_handler::QuoteOperatorInfo>,
    pub(crate) eof_emitted: bool,
    pub(crate) start_time: Instant,
}
