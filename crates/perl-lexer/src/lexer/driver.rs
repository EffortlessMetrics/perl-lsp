use crate::{LexerConfig, LexerMode, PerlLexer, Position};

impl<'a> PerlLexer<'a> {
    /// Create a new lexer with default configuration.
    pub fn new(input: &'a str) -> Self {
        Self::with_config(input, LexerConfig::default())
    }

    /// Create a new lexer with explicit configuration.
    pub fn with_config(input: &'a str, config: LexerConfig) -> Self {
        Self {
            input,
            input_bytes: input.as_bytes(),
            position: 0,
            mode: LexerMode::ExpectTerm,
            config,
            delimiter_stack: Vec::new(),
            in_prototype: false,
            prototype_depth: 0,
            after_sub: false,
            after_arrow: false,
            hash_brace_depth: 0,
            after_var_subscript: false,
            paren_depth: 0,
            current_pos: Position::start(),
            after_newline: true,
            pending_heredocs: Vec::new(),
            line_start_offset: 0,
            emit_heredoc_body_tokens: false,
            current_quote_op: None,
            eof_emitted: false,
            start_time: std::time::Instant::now(),
        }
    }
}
