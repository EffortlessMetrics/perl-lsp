//! Token types and structures for the Perl lexer.
//!
//! [`TokenType`] classifies every token the lexer can emit, and [`Token`]
//! bundles a type with its source text and byte span.

use std::sync::Arc;

/// Parts of an interpolated string
#[derive(Debug, Clone, PartialEq)]
pub enum StringPart {
    /// Literal text
    Literal(Arc<str>),
    /// Variable interpolation: $var, @array, %hash
    Variable(Arc<str>),
    /// Expression interpolation: ${expr}, @{expr}
    Expression(Arc<str>),
    /// Method call: `->method()`
    MethodCall(Arc<str>),
    /// Array slice: [1..3]
    ArraySlice(Arc<str>),
}

/// Token types for Perl
#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    // Slash-derived tokens
    /// Division operator: /
    Division,
    /// Regex match: m// or //
    RegexMatch,
    /// Substitution: s///
    Substitution,
    /// Transliteration: tr/// or y///
    Transliteration,
    /// Quote regex: qr//
    QuoteRegex,

    // String and quote tokens
    /// String literal: "string" or 'string'
    StringLiteral,
    /// Single quote: q//
    QuoteSingle,
    /// Double quote: qq//
    QuoteDouble,
    /// Quote words: qw//
    QuoteWords,
    /// Quote command: qx// or `backticks`
    QuoteCommand,

    // String interpolation tokens
    /// String with interpolated parts
    InterpolatedString(Vec<StringPart>),

    // Heredoc tokens
    /// Heredoc start: <<EOF or <<'EOF'
    HeredocStart,
    /// Heredoc body content
    HeredocBody(Arc<str>),

    // Format declarations
    /// Format body content
    FormatBody(Arc<str>),

    // Version strings
    /// Version string: v5.32.0
    Version(Arc<str>),

    // POD documentation
    /// POD documentation block
    Pod,

    // Data sections
    /// Data section marker: __DATA__ or __END__
    DataMarker(Arc<str>),
    /// Data section body content
    DataBody(Arc<str>),

    // Error recovery
    /// Unknown rest of input (used when budget exceeded)
    UnknownRest,

    // Identifiers and literals
    /// Identifier or variable name
    Identifier(Arc<str>),
    /// Numeric literal
    Number(Arc<str>),
    /// Operator
    Operator(Arc<str>),
    /// Keyword
    Keyword(Arc<str>),

    // Delimiters
    /// Left parenthesis: (
    LeftParen,
    /// Right parenthesis: )
    RightParen,
    /// Left bracket: [
    LeftBracket,
    /// Right bracket: ]
    RightBracket,
    /// Left brace: {
    LeftBrace,
    /// Right brace: }
    RightBrace,

    // Punctuation
    /// Semicolon: ;
    Semicolon,
    /// Comma: ,
    Comma,
    /// Colon: :
    Colon,
    /// Arrow: ->
    Arrow,
    /// Fat comma: =>
    FatComma,

    // Whitespace and comments
    /// Whitespace (usually not returned)
    Whitespace,
    /// Newline character
    Newline,
    /// Comment text
    Comment(Arc<str>),

    // Special tokens
    /// End of file
    EOF,
    /// Error token for invalid input
    Error(Arc<str>),
}

/// A single token produced by [`PerlLexer`](crate::PerlLexer).
///
/// Carries its [`TokenType`], the original source text (as a cheap-to-clone
/// `Arc<str>`), and the byte span within the input.
#[derive(Debug, Clone)]
pub struct Token {
    /// Classification of this token (keyword, operator, literal, etc.).
    pub token_type: TokenType,
    /// Original source text that this token spans.
    pub text: Arc<str>,
    /// Starting byte offset (inclusive) in the source input.
    pub start: usize,
    /// Ending byte offset (exclusive) in the source input.
    pub end: usize,
}

impl Token {
    /// Create a new token with the given type, source text, and byte span.
    pub fn new(token_type: TokenType, text: impl Into<Arc<str>>, start: usize, end: usize) -> Self {
        Self { token_type, text: text.into(), start, end }
    }

    /// Return the byte length of this token's span (`end - start`).
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    /// Return `true` if the token has a zero-length span.
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
}
