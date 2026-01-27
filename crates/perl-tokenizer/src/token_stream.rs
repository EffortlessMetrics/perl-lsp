//! Token stream adapter between `perl-lexer` output and the parser.
//!
//! Provides buffered lookahead, skips trivia tokens, and resets lexer mode at
//! statement boundaries. This stream is optimized for parser consumption rather
//! than full-fidelity token preservation.
//!
//! # Basic usage
//!
//! ```
//! use perl_tokenizer::{TokenKind, TokenStream};
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

use perl_error::{ParseError, ParseResult};
use perl_lexer::{LexerMode, PerlLexer, Token as LexerToken, TokenType as LexerTokenType};
pub use perl_token::{Token, TokenKind};

/// Token stream that wraps perl-lexer
pub struct TokenStream<'a> {
    lexer: PerlLexer<'a>,
    peeked: Option<Token>,
    peeked_second: Option<Token>,
    peeked_third: Option<Token>,
}

impl<'a> TokenStream<'a> {
    /// Create a new token stream from source code
    pub fn new(input: &'a str) -> Self {
        TokenStream {
            lexer: PerlLexer::new(input),
            peeked: None,
            peeked_second: None,
            peeked_third: None,
        }
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

    /// Enter format body parsing mode in the lexer
    pub fn enter_format_mode(&mut self) {
        self.lexer.enter_format_mode();
    }

    /// Called at statement boundaries to reset lexer state and clear cached lookahead
    pub fn on_stmt_boundary(&mut self) {
        // Clear any cached lookahead tokens
        self.peeked = None;
        self.peeked_second = None;
        self.peeked_third = None;

        // Reset lexer to expect a term (start of new statement)
        self.lexer.set_mode(LexerMode::ExpectTerm);
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

    /// Get the next token from the lexer
    fn next_token(&mut self) -> ParseResult<Token> {
        // Skip whitespace and comments
        loop {
            let lexer_token = self.lexer.next_token().ok_or(ParseError::UnexpectedEof)?;

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
                    return Ok(self.convert_token(lexer_token));
                }
            }
        }
    }

    /// Convert a lexer token to a parser token
    fn convert_token(&self, token: LexerToken) -> Token {
        let kind = match &token.token_type {
            // Keywords
            LexerTokenType::Keyword(kw) => match kw.as_ref() {
                "my" => TokenKind::My,
                "our" => TokenKind::Our,
                "local" => TokenKind::Local,
                "state" => TokenKind::State,
                "sub" => TokenKind::Sub,
                "if" => TokenKind::If,
                "elsif" => TokenKind::Elsif,
                "else" => TokenKind::Else,
                "unless" => TokenKind::Unless,
                "while" => TokenKind::While,
                "until" => TokenKind::Until,
                "for" => TokenKind::For,
                "foreach" => TokenKind::Foreach,
                "return" => TokenKind::Return,
                "package" => TokenKind::Package,
                "use" => TokenKind::Use,
                "no" => TokenKind::No,
                "BEGIN" => TokenKind::Begin,
                "END" => TokenKind::End,
                "CHECK" => TokenKind::Check,
                "INIT" => TokenKind::Init,
                "UNITCHECK" => TokenKind::Unitcheck,
                "eval" => TokenKind::Eval,
                "do" => TokenKind::Do,
                "given" => TokenKind::Given,
                "when" => TokenKind::When,
                "default" => TokenKind::Default,
                "try" => TokenKind::Try,
                "catch" => TokenKind::Catch,
                "finally" => TokenKind::Finally,
                "continue" => TokenKind::Continue,
                "next" => TokenKind::Next,
                "last" => TokenKind::Last,
                "redo" => TokenKind::Redo,
                "class" => TokenKind::Class,
                "method" => TokenKind::Method,
                "format" => TokenKind::Format,
                "undef" => TokenKind::Undef,
                "and" => TokenKind::WordAnd,
                "or" => TokenKind::WordOr,
                "not" => TokenKind::WordNot,
                "xor" => TokenKind::WordXor,
                "cmp" => TokenKind::StringCompare,
                "qw" => TokenKind::Identifier, // Keep as identifier but handle specially
                _ => TokenKind::Identifier,
            },

            // Operators
            LexerTokenType::Operator(op) => match op.as_ref() {
                "=" => TokenKind::Assign,
                "+" => TokenKind::Plus,
                "-" => TokenKind::Minus,
                "*" => TokenKind::Star,
                "/" => TokenKind::Slash,
                "%" => TokenKind::Percent,
                "**" => TokenKind::Power,
                "<<" => TokenKind::LeftShift,
                ">>" => TokenKind::RightShift,
                "&" => TokenKind::BitwiseAnd,
                "|" => TokenKind::BitwiseOr,
                "^" => TokenKind::BitwiseXor,
                "~" => TokenKind::BitwiseNot,
                // Compound assignments
                "+=" => TokenKind::PlusAssign,
                "-=" => TokenKind::MinusAssign,
                "*=" => TokenKind::StarAssign,
                "/=" => TokenKind::SlashAssign,
                "%=" => TokenKind::PercentAssign,
                ".=" => TokenKind::DotAssign,
                "&=" => TokenKind::AndAssign,
                "|=" => TokenKind::OrAssign,
                "^=" => TokenKind::XorAssign,
                "**=" => TokenKind::PowerAssign,
                "<<=" => TokenKind::LeftShiftAssign,
                ">>=" => TokenKind::RightShiftAssign,
                "&&=" => TokenKind::LogicalAndAssign,
                "||=" => TokenKind::LogicalOrAssign,
                "//=" => TokenKind::DefinedOrAssign,
                "==" => TokenKind::Equal,
                "!=" => TokenKind::NotEqual,
                "=~" => TokenKind::Match,
                "!~" => TokenKind::NotMatch,
                "~~" => TokenKind::SmartMatch,
                "<" => TokenKind::Less,
                ">" => TokenKind::Greater,
                "<=" => TokenKind::LessEqual,
                ">=" => TokenKind::GreaterEqual,
                "<=>" => TokenKind::Spaceship,
                "&&" => TokenKind::And,
                "||" => TokenKind::Or,
                "!" => TokenKind::Not,
                "//" => TokenKind::DefinedOr,
                "->" => TokenKind::Arrow,
                "=>" => TokenKind::FatArrow,
                "." => TokenKind::Dot,
                ".." => TokenKind::Range,
                "..." => TokenKind::Ellipsis,
                "++" => TokenKind::Increment,
                "--" => TokenKind::Decrement,
                "::" => TokenKind::DoubleColon,
                "?" => TokenKind::Question,
                ":" => TokenKind::Colon,
                "\\" => TokenKind::Backslash,
                // Sigils (when used as operators in certain contexts)
                "$" => TokenKind::ScalarSigil,
                "@" => TokenKind::ArraySigil,
                // % is already handled as Percent above
                // & is already handled as BitwiseAnd above
                // * is already handled as Star above
                _ => TokenKind::Unknown,
            },

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
            LexerTokenType::DataMarker(_) => TokenKind::DataMarker,
            LexerTokenType::DataBody(_) => TokenKind::DataBody,
            LexerTokenType::UnknownRest => TokenKind::UnknownRest,

            // Identifiers
            LexerTokenType::Identifier(text) => {
                // Check if it's actually a keyword that the lexer didn't recognize
                match text.as_ref() {
                    "no" => TokenKind::No,
                    "*" => TokenKind::Star, // Special case: * by itself is multiplication
                    "$" => TokenKind::ScalarSigil,
                    "@" => TokenKind::ArraySigil,
                    "%" => TokenKind::HashSigil,
                    "&" => TokenKind::SubSigil,
                    _ => TokenKind::Identifier,
                }
            }

            // Handle error tokens that might be valid syntax
            LexerTokenType::Error(msg) => {
                // Check if it's a specific error we want to handle specially
                if msg.as_ref() == "Heredoc nesting too deep" {
                    TokenKind::HeredocDepthLimit
                } else {
                    // Check if it's a brace that the lexer couldn't recognize
                    match token.text.as_ref() {
                        "{" => TokenKind::LeftBrace,
                        "}" => TokenKind::RightBrace,
                        _ => TokenKind::Unknown,
                    }
                }
            }

            _ => TokenKind::Unknown,
        };

        Token { kind, text: token.text, start: token.start, end: token.end }
    }
}
