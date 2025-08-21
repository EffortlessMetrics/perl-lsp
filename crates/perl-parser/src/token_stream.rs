//! Token stream adapter for perl-lexer integration
//!
//! This module provides the bridge between perl-lexer's token output
//! and the parser's token consumption model.

use crate::error::{ParseError, ParseResult};
use perl_lexer::{LexerMode, PerlLexer, Token as LexerToken, TokenType as LexerTokenType};

/// A simplified token representation for the parser
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub text: String,
    pub start: usize,
    pub end: usize,
}

/// Simplified token types for parsing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    // Keywords
    My,
    Our,
    Local,
    State,
    Sub,
    If,
    Elsif,
    Else,
    Unless,
    While,
    Until,
    For,
    Foreach,
    Return,
    Package,
    Use,
    No,
    Begin,
    End,
    Check,
    Init,
    Unitcheck,
    Eval,
    Do,
    Given,
    When,
    Default,
    Try,
    Catch,
    Finally,
    Continue,
    Class,
    Method,
    Format,
    Undef,

    // Operators
    Assign,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Power,
    LeftShift,
    RightShift,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    BitwiseNot,
    // Compound assignments
    PlusAssign,
    MinusAssign,
    StarAssign,
    SlashAssign,
    PercentAssign,
    DotAssign,
    AndAssign,
    OrAssign,
    XorAssign,
    PowerAssign,
    LeftShiftAssign,
    RightShiftAssign,
    LogicalAndAssign,
    LogicalOrAssign,
    DefinedOrAssign,
    Equal,
    NotEqual,
    Match,
    NotMatch,
    SmartMatch,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    Spaceship,     // <=>
    StringCompare, // cmp
    And,
    Or,
    Not,
    DefinedOr,
    // Word operators
    WordAnd,
    WordOr,
    WordNot,
    WordXor,
    Arrow,
    FatArrow,
    Dot,
    Range,
    Ellipsis,
    Increment,
    Decrement,
    DoubleColon,
    Question,
    Colon,
    Backslash, // Reference operator

    // Delimiters
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Semicolon,
    Comma,

    // Literals
    Number,
    String,
    Regex,
    Substitution,
    Transliteration,
    HeredocStart,
    HeredocBody,
    FormatBody,
    DataMarker,
    DataBody,
    UnknownRest,

    // Identifiers and variables
    Identifier,
    ScalarSigil,
    ArraySigil,
    HashSigil,
    SubSigil,
    GlobSigil,

    // Special
    Eof,
    Unknown,
}

/// Token stream that wraps perl-lexer
pub struct TokenStream<'a> {
    lexer: PerlLexer<'a>,
    peeked: Option<Token>,
    peeked_second: Option<Token>,
}

impl<'a> TokenStream<'a> {
    /// Create a new token stream from source code
    pub fn new(input: &'a str) -> Self {
        TokenStream { lexer: PerlLexer::new(input), peeked: None, peeked_second: None }
    }

    /// Peek at the next token without consuming it
    pub fn peek(&mut self) -> ParseResult<&Token> {
        if self.peeked.is_none() {
            self.peeked = Some(self.next_token()?);
        }
        Ok(self.peeked.as_ref().unwrap())
    }

    /// Consume and return the next token
    pub fn next(&mut self) -> ParseResult<Token> {
        // If we have a peeked token, return it and move peeked_second to peeked

        if let Some(token) = self.peeked.take() {
            self.peeked = self.peeked_second.take();
            Ok(token)
        } else {
            self.next_token()
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

        Ok(self.peeked_second.as_ref().unwrap())
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

        // Reset lexer to expect a term (start of new statement)
        self.lexer.set_mode(LexerMode::ExpectTerm);
    }

    /// Pure peek cache invalidation - no mode changes
    pub fn invalidate_peek(&mut self) {
        self.peeked = None;
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
                        text: String::new(),
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
            LexerTokenType::Error(_msg) => {
                // Check if it's a brace that the lexer couldn't recognize
                match token.text.as_ref() {
                    "{" => TokenKind::LeftBrace,
                    "}" => TokenKind::RightBrace,
                    _ => TokenKind::Unknown,
                }
            }

            _ => TokenKind::Unknown,
        };

        Token { kind, text: token.text.to_string(), start: token.start, end: token.end }
    }
}
