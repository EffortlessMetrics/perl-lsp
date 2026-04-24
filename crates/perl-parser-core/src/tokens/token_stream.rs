//! Token stream adapter between `perl-lexer` output and the parser.
//!
//! Provides buffered lookahead, skips trivia tokens, and resets lexer mode at
//! statement boundaries. This stream is optimized for parser consumption rather
//! than full-fidelity token preservation.
//!
//! # Basic usage
//!
//! ```
//! use perl_parser_core::tokens::token_stream::{TokenKind, TokenStream};
//!
//! let mut stream = TokenStream::new("my $x = 42;");
//! assert!(matches!(stream.peek(), Ok(token) if token.kind == TokenKind::My));
//!
//! while let Ok(token) = stream.next() {
//!     if token.kind == TokenKind::Eof {
//!         break;
//!     }
//! }
//! ```
//!
//! # Pre-lexed token stream
//!
//! For incremental parsing, use [`TokenStream::from_vec`] to create a stream
//! from pre-lexed tokens without re-lexing from source:
//!
//! ```
//! use perl_parser_core::tokens::token_stream::{Token, TokenKind, TokenStream};
//!
//! let tokens = vec![
//!     Token::new(TokenKind::My, "my", 0, 2),
//!     Token::new(TokenKind::ScalarSigil, "$", 3, 4),
//!     Token::new(TokenKind::Identifier, "x", 4, 5),
//!     Token::new(TokenKind::Assign, "=", 6, 7),
//!     Token::new(TokenKind::Number, "1", 8, 9),
//!     Token::new(TokenKind::Semicolon, ";", 9, 10),
//!     Token::new(TokenKind::Eof, "", 10, 10),
//! ];
//! let mut stream = TokenStream::from_vec(tokens);
//! assert!(matches!(stream.peek(), Ok(t) if t.kind == TokenKind::My));
//! ```

use crate::syntax::error::{ParseError, ParseResult};
use perl_lexer::{LexerMode, PerlLexer, Token as LexerToken, TokenType as LexerTokenType};
pub use perl_token::{Token, TokenKind};
use std::collections::VecDeque;

/// Backing source for the token stream — either a live lexer or pre-lexed tokens.
enum TokenStreamInner<'a> {
    /// Live lexer producing tokens on demand from source text.
    Lexer(PerlLexer<'a>),
    /// Pre-lexed token buffer; used by [`TokenStream::from_vec`].
    Buffered(VecDeque<Token>),
}

/// Token stream that wraps perl-lexer or a pre-lexed token buffer.
///
/// Provides three-token lookahead, transparent trivia skipping (in lexer mode),
/// and statement-boundary state management used by the recursive-descent parser.
pub struct TokenStream<'a> {
    inner: TokenStreamInner<'a>,
    buffered_eof_pos: usize,
    peeked: Option<Token>,
    peeked_second: Option<Token>,
    peeked_third: Option<Token>,
}

impl<'a> TokenStream<'a> {
    /// Create a new token stream from source code.
    pub fn new(input: &'a str) -> Self {
        TokenStream {
            inner: TokenStreamInner::Lexer(PerlLexer::new(input)),
            buffered_eof_pos: input.len(),
            peeked: None,
            peeked_second: None,
            peeked_third: None,
        }
    }

    /// Create a token stream from a pre-lexed token list.
    ///
    /// This constructor skips lexing entirely and feeds tokens directly from the
    /// provided `Vec`. It is intended for the incremental parsing pipeline where
    /// tokens from a prior parse run can be reused for unchanged regions.
    ///
    /// # Behaviour differences from [`TokenStream::new`]
    ///
    /// - [`on_stmt_boundary`](Self::on_stmt_boundary): clears lookahead cache only;
    ///   no lexer mode reset (tokens are already classified).
    /// - [`relex_as_term`](Self::relex_as_term): clears lookahead cache only;
    ///   no re-lexing (token kinds are fixed from the original lex pass).
    /// - [`enter_format_mode`](Self::enter_format_mode): no-op.
    ///
    /// # Arguments
    ///
    /// * `tokens` — Pre-lexed tokens. An `Eof` token does **not** need to be
    ///   included; the stream synthesises one when the buffer is exhausted.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_parser_core::tokens::token_stream::{Token, TokenKind, TokenStream};
    ///
    /// let tokens = vec![
    ///     Token::new(TokenKind::My, "my", 0, 2),
    ///     Token::new(TokenKind::Eof, "", 2, 2),
    /// ];
    /// let mut stream = TokenStream::from_vec(tokens);
    /// assert!(matches!(stream.peek(), Ok(t) if t.kind == TokenKind::My));
    /// ```
    pub fn from_vec(tokens: Vec<Token>) -> Self {
        let buffered_eof_pos = tokens
            .last()
            .map(|token| if token.kind == TokenKind::Eof { token.start } else { token.end })
            .unwrap_or(0);

        TokenStream {
            inner: TokenStreamInner::Buffered(VecDeque::from(tokens)),
            buffered_eof_pos,
            peeked: None,
            peeked_second: None,
            peeked_third: None,
        }
    }

    /// Convert a slice of raw [`LexerToken`]s to parser [`Token`]s, filtering out trivia.
    ///
    /// This is a convenience method for the incremental parsing pipeline where the
    /// token cache stores raw lexer tokens (including whitespace and comments) and
    /// needs to convert them to parser tokens before feeding to [`Self::from_vec`].
    ///
    /// Trivia token types (whitespace, newlines, comments, EOF) are discarded.
    /// All other token types are converted using the same mapping as the live
    /// [`TokenStream`] would apply.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_parser_core::tokens::token_stream::{TokenKind, TokenStream};
    /// use perl_lexer::{PerlLexer, TokenType};
    ///
    /// // Collect raw lexer tokens
    /// let mut lexer = PerlLexer::new("my $x = 1;");
    /// let mut raw = Vec::new();
    /// while let Some(t) = lexer.next_token() {
    ///     if matches!(t.token_type, TokenType::EOF) { break; }
    ///     raw.push(t);
    /// }
    ///
    /// // Convert to parser tokens and build a stream
    /// let parser_tokens = TokenStream::lexer_tokens_to_parser_tokens(raw);
    /// let mut stream = TokenStream::from_vec(parser_tokens);
    /// assert!(matches!(stream.peek(), Ok(t) if t.kind == TokenKind::My));
    /// ```
    pub fn lexer_tokens_to_parser_tokens(tokens: Vec<LexerToken>) -> Vec<Token> {
        tokens
            .into_iter()
            .filter(|t| {
                !matches!(
                    t.token_type,
                    LexerTokenType::Whitespace | LexerTokenType::Newline | LexerTokenType::EOF
                ) && !matches!(t.token_type, LexerTokenType::Comment(_))
            })
            .map(Self::convert_lexer_token)
            .collect()
    }

    /// Peek at the next token without consuming it
    pub fn peek(&mut self) -> ParseResult<&Token> {
        if self.peeked.is_none() {
            self.peeked = Some(self.next_token()?);
        }
        // Safe: we just ensured peeked is Some
        self.peeked.as_ref().ok_or(ParseError::UnexpectedEof)
    }

    /// Consume and return the next token
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> ParseResult<Token> {
        // If we have a peeked token, return it and shift the peek chain down

        if let Some(token) = self.peeked.take() {
            // Make EOF sticky - if we're returning EOF, put it back in the peek buffer
            // so future peeks still see EOF instead of getting an error
            if token.kind == TokenKind::Eof {
                self.peeked = Some(token.clone());
            } else {
                self.peeked = self.peeked_second.take();
                self.peeked_second = self.peeked_third.take();
            }
            Ok(token)
        } else {
            let token = self.next_token()?;
            // Make EOF sticky for fresh tokens too
            if token.kind == TokenKind::Eof {
                self.peeked = Some(token.clone());
            }
            Ok(token)
        }
    }

    /// Check if we're at the end of input
    pub fn is_eof(&mut self) -> bool {
        matches!(self.peek(), Ok(token) if token.kind == TokenKind::Eof)
    }

    /// Peek at the second token (two tokens ahead)
    pub fn peek_second(&mut self) -> ParseResult<&Token> {
        // First ensure we have a peeked token
        self.peek()?;

        // If we don't have a second peeked token, get it
        if self.peeked_second.is_none() {
            self.peeked_second = Some(self.next_token()?);
        }

        // Safe: we just ensured peeked_second is Some
        self.peeked_second.as_ref().ok_or(ParseError::UnexpectedEof)
    }

    /// Peek at the third token (three tokens ahead)
    pub fn peek_third(&mut self) -> ParseResult<&Token> {
        // First ensure we have peeked and second peeked tokens
        self.peek_second()?;

        // If we don't have a third peeked token, get it
        if self.peeked_third.is_none() {
            self.peeked_third = Some(self.next_token()?);
        }

        // Safe: we just ensured peeked_third is Some
        self.peeked_third.as_ref().ok_or(ParseError::UnexpectedEof)
    }

    /// Enter format body parsing mode in the lexer.
    ///
    /// No-op when operating in buffered (pre-lexed) mode — the tokens are
    /// already fully classified.
    pub fn enter_format_mode(&mut self) {
        if let TokenStreamInner::Lexer(ref mut lexer) = self.inner {
            lexer.enter_format_mode();
        }
        // Buffered mode: no-op — tokens are pre-classified.
    }

    /// Called at statement boundaries to reset lexer state and clear cached lookahead.
    ///
    /// In buffered mode only the lookahead cache is cleared; no lexer mode reset
    /// is performed because the tokens are already fully classified.
    pub fn on_stmt_boundary(&mut self) {
        // Clear any cached lookahead tokens
        self.peeked = None;
        self.peeked_second = None;
        self.peeked_third = None;

        // Reset lexer to expect a term (start of new statement)
        if let TokenStreamInner::Lexer(ref mut lexer) = self.inner {
            lexer.set_mode(LexerMode::ExpectTerm);
        }
        // Buffered mode: no lexer mode reset needed — tokens are pre-classified.
    }

    /// Re-lex the current peeked token in `ExpectTerm` mode.
    ///
    /// This is needed for context-sensitive constructs like `split /regex/`
    /// where the `/` was lexed as division (`Slash`) but should be a regex
    /// delimiter. Rolls the lexer back to the peeked token's start position,
    /// switches to `ExpectTerm` mode, and clears the peek cache so the next
    /// `peek()` or `next()` re-lexes it as a regex.
    ///
    /// In buffered mode the peek cache is cleared but no re-lexing occurs —
    /// token kinds are fixed from the original lex pass.
    pub fn relex_as_term(&mut self) {
        if let TokenStreamInner::Lexer(ref mut lexer) = self.inner {
            if let Some(ref token) = self.peeked {
                use perl_lexer::Checkpointable;
                let pos = token.start;
                // Build a checkpoint at the peeked token's position with ExpectTerm mode
                let cp = perl_lexer::LexerCheckpoint::at_position(pos);
                lexer.restore(&cp);
            }
        }
        // Both modes: clear the peek cache.
        self.peeked = None;
        self.peeked_second = None;
        self.peeked_third = None;
    }

    /// Pure peek cache invalidation - no mode changes
    pub fn invalidate_peek(&mut self) {
        self.peeked = None;
        self.peeked_third = None;
        self.peeked_second = None;
    }

    /// Convenience method for a one-shot fresh peek
    pub fn peek_fresh_kind(&mut self) -> Option<TokenKind> {
        self.invalidate_peek();
        match self.peek() {
            Ok(token) => Some(token.kind),
            Err(_) => None,
        }
    }

    /// Get the next token from the backing source.
    fn next_token(&mut self) -> ParseResult<Token> {
        match &mut self.inner {
            TokenStreamInner::Lexer(lexer) => Self::next_token_from_lexer(lexer),
            TokenStreamInner::Buffered(buf) => {
                Self::next_token_from_buf(buf, &mut self.buffered_eof_pos)
            }
        }
    }

    /// Drain the next non-trivia token from the live lexer.
    fn next_token_from_lexer(lexer: &mut PerlLexer<'_>) -> ParseResult<Token> {
        // Skip whitespace and comments
        loop {
            let lexer_token = lexer.next_token().ok_or(ParseError::UnexpectedEof)?;

            match &lexer_token.token_type {
                LexerTokenType::Whitespace | LexerTokenType::Newline => continue,
                LexerTokenType::Comment(_) => continue,
                LexerTokenType::EOF => {
                    return Ok(Token {
                        kind: TokenKind::Eof,
                        text: String::new().into(),
                        start: lexer_token.start,
                        end: lexer_token.end,
                    });
                }
                _ => {
                    return Ok(Self::convert_lexer_token(lexer_token));
                }
            }
        }
    }

    /// Return the next token from the pre-lexed buffer.
    fn next_token_from_buf(
        buf: &mut VecDeque<Token>,
        buffered_eof_pos: &mut usize,
    ) -> ParseResult<Token> {
        match buf.pop_front() {
            Some(token) => {
                *buffered_eof_pos =
                    if token.kind == TokenKind::Eof { token.start } else { token.end };
                Ok(token)
            }
            // Synthesise EOF at the most recently known source position.
            None => Ok(Token::eof_at(*buffered_eof_pos)),
        }
    }

    /// Convert a raw lexer token to the parser `Token` type.
    ///
    /// Extracted from `next_token_from_lexer` to keep the match arm readable.
    fn convert_lexer_token(token: LexerToken) -> Token {
        let kind = match &token.token_type {
            // Keywords
            LexerTokenType::Keyword(kw) => match kw.as_ref() {
                "qw" => TokenKind::Identifier, // Keep as identifier but handle specially
                keyword => TokenKind::from_keyword(keyword).unwrap_or(TokenKind::Identifier),
            },

            // Operators
            LexerTokenType::Operator(op) => TokenKind::from_operator(op)
                // Sigils may be surfaced as operator tokens in some contexts.
                .or_else(|| TokenKind::from_sigil(op))
                .unwrap_or(TokenKind::Unknown),

            // Arrow tokens
            LexerTokenType::Arrow => TokenKind::Arrow,
            LexerTokenType::FatComma => TokenKind::FatArrow,

            // Delimiters
            LexerTokenType::LeftParen => TokenKind::LeftParen,
            LexerTokenType::RightParen => TokenKind::RightParen,
            LexerTokenType::LeftBrace => TokenKind::LeftBrace,
            LexerTokenType::RightBrace => TokenKind::RightBrace,
            LexerTokenType::LeftBracket => TokenKind::LeftBracket,
            LexerTokenType::RightBracket => TokenKind::RightBracket,
            LexerTokenType::Semicolon => TokenKind::Semicolon,
            LexerTokenType::Comma => TokenKind::Comma,

            // Division operator (important to handle before other tokens)
            LexerTokenType::Division => TokenKind::Slash,

            // Literals
            LexerTokenType::Number(_) => TokenKind::Number,
            LexerTokenType::StringLiteral | LexerTokenType::InterpolatedString(_) => {
                TokenKind::String
            }
            LexerTokenType::RegexMatch | LexerTokenType::QuoteRegex => TokenKind::Regex,
            LexerTokenType::Substitution => TokenKind::Substitution,
            LexerTokenType::Transliteration => TokenKind::Transliteration,
            LexerTokenType::QuoteSingle => TokenKind::QuoteSingle,
            LexerTokenType::QuoteDouble => TokenKind::QuoteDouble,
            LexerTokenType::QuoteWords => TokenKind::QuoteWords,
            LexerTokenType::QuoteCommand => TokenKind::QuoteCommand,
            LexerTokenType::HeredocStart => TokenKind::HeredocStart,
            LexerTokenType::HeredocBody(_) => TokenKind::HeredocBody,
            LexerTokenType::FormatBody(_) => TokenKind::FormatBody,
            LexerTokenType::Version(_) => TokenKind::VString,
            LexerTokenType::DataMarker(_) => TokenKind::DataMarker,
            LexerTokenType::DataBody(_) => TokenKind::DataBody,
            LexerTokenType::UnknownRest => TokenKind::UnknownRest,

            // Identifiers
            LexerTokenType::Identifier(text) => {
                // Check if it's actually a keyword that the lexer didn't recognize
                TokenKind::from_keyword(text)
                    .or_else(|| TokenKind::from_operator(text))
                    .or_else(|| TokenKind::from_sigil(text))
                    .unwrap_or(TokenKind::Identifier)
            }

            // Handle error tokens that might be valid syntax
            LexerTokenType::Error(msg) => {
                // Check if it's a specific error we want to handle specially
                if msg.as_ref() == "Heredoc nesting too deep" {
                    TokenKind::HeredocDepthLimit
                } else {
                    // Check if it's a brace that the lexer couldn't recognize
                    TokenKind::from_delimiter(token.text.as_ref()).unwrap_or(TokenKind::Unknown)
                }
            }

            _ => TokenKind::Unknown,
        };

        Token { kind, text: token.text, start: token.start, end: token.end }
    }
}
