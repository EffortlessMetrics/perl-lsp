//! Perl token definitions shared across the parser ecosystem.
//!
//! This crate defines [`Token`] and [`TokenKind`], the fundamental types that
//! flow from the lexer (`perl-lexer`) into the parser (`perl-parser-core`).
//! Downstream crates re-export these types so consumers rarely need to depend
//! on `perl-token` directly.
//!
//! # Examples
//!
//! Create and inspect tokens:
//!
//! ```rust
//! use perl_token::{Token, TokenKind};
//!
//! // Create a keyword token for `my`
//! let token = Token::new(TokenKind::My, "my", 0, 2);
//! assert_eq!(token.kind, TokenKind::My);
//! assert_eq!(&*token.text, "my");
//! assert_eq!(token.start, 0);
//! assert_eq!(token.end, 2);
//!
//! // Create a numeric literal token
//! let num = Token::new(TokenKind::Number, "42", 7, 9);
//! assert_eq!(num.kind, TokenKind::Number);
//! assert_eq!(&*num.text, "42");
//! ```
//!
//! Use [`TokenKind::display_name`] for user-facing error messages:
//!
//! ```rust
//! use perl_token::TokenKind;
//!
//! assert_eq!(TokenKind::LeftBrace.display_name(), "'{'");
//! assert_eq!(TokenKind::Identifier.display_name(), "identifier");
//! assert_eq!(TokenKind::Eof.display_name(), "end of input");
//! ```

use std::sync::Arc;

/// Token produced by the lexer and consumed by the parser.
///
/// Stores the token kind, original source text, and byte span. The text is kept
/// in an `Arc<str>` so buffering and lookahead can clone tokens cheaply.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// Token classification for parser decision making
    pub kind: TokenKind,
    /// Original source text for precise reconstruction
    pub text: Arc<str>,
    /// Starting byte position for error reporting and location tracking
    pub start: usize,
    /// Ending byte position for span calculation and navigation
    pub end: usize,
}

impl Token {
    /// Create a new token with the given kind, source text, and byte span.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_token::{Token, TokenKind};
    ///
    /// let tok = Token::new(TokenKind::Sub, "sub", 0, 3);
    /// assert_eq!(tok.kind, TokenKind::Sub);
    /// assert_eq!(&*tok.text, "sub");
    /// ```
    pub fn new(kind: TokenKind, text: impl Into<Arc<str>>, start: usize, end: usize) -> Self {
        Token { kind, text: text.into(), start, end }
    }

    /// Return the token span length in bytes.
    ///
    /// This uses saturating subtraction so malformed spans (where `end < start`)
    /// are treated as zero-length instead of underflowing.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_token::{Token, TokenKind};
    ///
    /// let tok = Token::new(TokenKind::Identifier, "foo", 10, 13);
    /// assert_eq!(tok.len(), 3);
    /// ```
    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Return whether the token span is empty.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_token::{Token, TokenKind};
    ///
    /// let tok = Token::new(TokenKind::Eof, "", 8, 8);
    /// assert!(tok.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Token classification for Perl parsing.
///
/// The set is intentionally simplified for fast parser matching while covering
/// keywords, operators, delimiters, literals, identifiers, and special tokens.
///
/// Use [`TokenKind::display_name`] to get a human-readable string suitable for
/// error messages shown to the user.
///
/// # Categories
///
/// | Group | Examples |
/// |-------|----------|
/// | Keywords | [`My`](Self::My), [`Sub`](Self::Sub), [`If`](Self::If), ... |
/// | Operators | [`Plus`](Self::Plus), [`Arrow`](Self::Arrow), [`And`](Self::And), ... |
/// | Delimiters | [`LeftParen`](Self::LeftParen), [`LeftBrace`](Self::LeftBrace), ... |
/// | Literals | [`Number`](Self::Number), [`String`](Self::String), [`Regex`](Self::Regex), ... |
/// | Identifiers | [`Identifier`](Self::Identifier), [`ScalarSigil`](Self::ScalarSigil), ... |
/// | Special | [`Eof`](Self::Eof), [`Unknown`](Self::Unknown) |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    // ===== Keywords =====
    /// Lexical variable declaration: `my $x`
    My,
    /// Package variable declaration: `our $x`
    Our,
    /// Dynamic scoping: `local $x`
    Local,
    /// Persistent variable: `state $x`
    State,
    /// Subroutine declaration: `sub foo`
    Sub,
    /// Conditional: `if (cond)`
    If,
    /// Else-if conditional: `elsif (cond)`
    Elsif,
    /// Else branch: `else { }`
    Else,
    /// Negated conditional: `unless (cond)`
    Unless,
    /// While loop: `while (cond)`
    While,
    /// Until loop: `until (cond)`
    Until,
    /// C-style for loop: `for (init; cond; update)`
    For,
    /// Iterator loop: `foreach $x (@list)`
    Foreach,
    /// Return statement: `return $value`
    Return,
    /// Package declaration: `package Foo`
    Package,
    /// Module import: `use Module`
    Use,
    /// Disable pragma/module: `no strict`
    No,
    /// Compile-time block: `BEGIN { }`
    Begin,
    /// Exit-time block: `END { }`
    End,
    /// Check phase block: `CHECK { }`
    Check,
    /// Init phase block: `INIT { }`
    Init,
    /// Unit check block: `UNITCHECK { }`
    Unitcheck,
    /// Exception handling: `eval { }`
    Eval,
    /// Block execution: `do { }` or `do "file"`
    Do,
    /// Switch expression: `given ($x)`
    Given,
    /// Case clause: `when ($pattern)`
    When,
    /// Default case: `default { }`
    Default,
    /// Try block: `try { }`
    Try,
    /// Catch block: `catch ($e) { }`
    Catch,
    /// Finally block: `finally { }`
    Finally,
    /// Continue block: `continue { }`
    Continue,
    /// Loop control: `next`
    Next,
    /// Loop control: `last`
    Last,
    /// Loop control: `redo`
    Redo,
    /// Goto statement: `goto LABEL`, `goto &sub`, `goto EXPR`
    Goto,
    /// Class declaration (5.38+): `class Foo`
    Class,
    /// Method declaration (5.38+): `method foo`
    Method,
    /// Class field declaration (5.38+): `field $name`
    Field,
    /// Format declaration: `format STDOUT =`
    Format,
    /// Undefined value: `undef`
    Undef,
    /// Defer block: `defer { ... }` (Perl 5.36+ experimental, stable in 5.40)
    Defer,

    // ===== Operators =====
    /// Assignment: `=`
    Assign,
    /// Addition: `+`
    Plus,
    /// Subtraction: `-`
    Minus,
    /// Multiplication: `*`
    Star,
    /// Division: `/`
    Slash,
    /// Modulo: `%`
    Percent,
    /// Exponentiation: `**`
    Power,
    /// Left bit shift: `<<`
    LeftShift,
    /// Right bit shift: `>>`
    RightShift,
    /// Bitwise AND: `&`
    BitwiseAnd,
    /// Bitwise OR: `|`
    BitwiseOr,
    /// Bitwise XOR: `^`
    BitwiseXor,
    /// Bitwise NOT: `~`
    BitwiseNot,
    /// Add and assign: `+=`
    PlusAssign,
    /// Subtract and assign: `-=`
    MinusAssign,
    /// Multiply and assign: `*=`
    StarAssign,
    /// Divide and assign: `/=`
    SlashAssign,
    /// Modulo and assign: `%=`
    PercentAssign,
    /// Concatenate and assign: `.=`
    DotAssign,
    /// Bitwise AND and assign: `&=`
    AndAssign,
    /// Bitwise OR and assign: `|=`
    OrAssign,
    /// Bitwise XOR and assign: `^=`
    XorAssign,
    /// Power and assign: `**=`
    PowerAssign,
    /// Left shift and assign: `<<=`
    LeftShiftAssign,
    /// Right shift and assign: `>>=`
    RightShiftAssign,
    /// Logical AND and assign: `&&=`
    LogicalAndAssign,
    /// Logical OR and assign: `||=`
    LogicalOrAssign,
    /// Defined-or and assign: `//=`
    DefinedOrAssign,
    /// Numeric equality: `==`
    Equal,
    /// Numeric inequality: `!=`
    NotEqual,
    /// Pattern match binding: `=~`
    Match,
    /// Negated pattern match: `!~`
    NotMatch,
    /// Smart match: `~~`
    SmartMatch,
    /// Less than: `<`
    Less,
    /// Greater than: `>`
    Greater,
    /// Less than or equal: `<=`
    LessEqual,
    /// Greater than or equal: `>=`
    GreaterEqual,
    /// Numeric comparison (spaceship): `<=>`
    Spaceship,
    /// String comparison: `cmp`
    StringCompare,
    /// Logical AND: `&&`
    And,
    /// Logical OR: `||`
    Or,
    /// Logical NOT: `!`
    Not,
    /// Defined-or: `//`
    DefinedOr,
    /// Word AND operator: `and`
    WordAnd,
    /// Word OR operator: `or`
    WordOr,
    /// Word NOT operator: `not`
    WordNot,
    /// Word XOR operator: `xor`
    WordXor,
    /// Method/dereference arrow: `->`
    Arrow,
    /// Hash key separator: `=>`
    FatArrow,
    /// String concatenation: `.`
    Dot,
    /// Range operator: `..`
    Range,
    /// Yada-yada (unimplemented): `...`
    Ellipsis,
    /// Increment: `++`
    Increment,
    /// Decrement: `--`
    Decrement,
    /// Package separator: `::`
    DoubleColon,
    /// Ternary condition: `?`
    Question,
    /// Ternary/label separator: `:`
    Colon,
    /// Reference operator: `\`
    Backslash,

    // ===== Delimiters =====
    /// Left parenthesis: `(`
    LeftParen,
    /// Right parenthesis: `)`
    RightParen,
    /// Left brace: `{`
    LeftBrace,
    /// Right brace: `}`
    RightBrace,
    /// Left bracket: `[`
    LeftBracket,
    /// Right bracket: `]`
    RightBracket,
    /// Statement terminator: `;`
    Semicolon,
    /// List separator: `,`
    Comma,

    // ===== Literals =====
    /// Numeric literal: `42`, `3.14`, `0xFF`
    Number,
    /// String literal: `"hello"` or `'world'`
    String,
    /// Regular expression: `/pattern/flags`
    Regex,
    /// Substitution: `s/pattern/replacement/flags`
    Substitution,
    /// Transliteration: `tr/abc/xyz/` or `y///`
    Transliteration,
    /// Single-quoted string: `q/text/`
    QuoteSingle,
    /// Double-quoted string: `qq/text/`
    QuoteDouble,
    /// Quote words: `qw(list of words)`
    QuoteWords,
    /// Backtick command: `` `cmd` `` or `qx/cmd/`
    QuoteCommand,
    /// Heredoc start marker: `<<EOF`
    HeredocStart,
    /// Heredoc content body
    HeredocBody,
    /// Format specification body
    FormatBody,
    /// Data section marker: `__DATA__` or `__END__`
    DataMarker,
    /// Data section content
    DataBody,
    /// Version string literal: `v5.26.0`, `v5.10`
    VString,
    /// Unparsed remainder (budget exceeded)
    UnknownRest,
    /// Heredoc depth limit exceeded (special error token)
    HeredocDepthLimit,

    // ===== Identifiers and Variables =====
    /// Bareword identifier or function name
    Identifier,
    /// Scalar sigil: `$`
    ScalarSigil,
    /// Array sigil: `@`
    ArraySigil,
    /// Hash sigil: `%`
    HashSigil,
    /// Subroutine sigil: `&`
    SubSigil,
    /// Glob/typeglob sigil: `*`
    GlobSigil,

    // ===== Special =====
    /// End of file/input
    Eof,
    /// Unknown/unrecognized token
    Unknown,
}

/// High-level category for [`TokenKind`] metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TokenCategory {
    Keyword,
    Operator,
    Delimiter,
    Literal,
    Identifier,
    Sigil,
    Special,
}

/// Metadata row describing one [`TokenKind`] variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct TokenKindInfo {
    pub kind: TokenKind,
    pub display_name: &'static str,
    pub category: TokenCategory,
    pub canonical_lexeme: Option<&'static str>,
    pub keyword_spelling: Option<&'static str>,
    pub operator_spelling: Option<&'static str>,
}

const ALL_TOKEN_KINDS: [TokenKind; 132] = [
    TokenKind::My,
    TokenKind::Our,
    TokenKind::Local,
    TokenKind::State,
    TokenKind::Sub,
    TokenKind::If,
    TokenKind::Elsif,
    TokenKind::Else,
    TokenKind::Unless,
    TokenKind::While,
    TokenKind::Until,
    TokenKind::For,
    TokenKind::Foreach,
    TokenKind::Return,
    TokenKind::Package,
    TokenKind::Use,
    TokenKind::No,
    TokenKind::Begin,
    TokenKind::End,
    TokenKind::Check,
    TokenKind::Init,
    TokenKind::Unitcheck,
    TokenKind::Eval,
    TokenKind::Do,
    TokenKind::Given,
    TokenKind::When,
    TokenKind::Default,
    TokenKind::Try,
    TokenKind::Catch,
    TokenKind::Finally,
    TokenKind::Continue,
    TokenKind::Next,
    TokenKind::Last,
    TokenKind::Redo,
    TokenKind::Goto,
    TokenKind::Class,
    TokenKind::Method,
    TokenKind::Field,
    TokenKind::Format,
    TokenKind::Undef,
    TokenKind::Defer,
    TokenKind::Assign,
    TokenKind::Plus,
    TokenKind::Minus,
    TokenKind::Star,
    TokenKind::Slash,
    TokenKind::Percent,
    TokenKind::Power,
    TokenKind::LeftShift,
    TokenKind::RightShift,
    TokenKind::BitwiseAnd,
    TokenKind::BitwiseOr,
    TokenKind::BitwiseXor,
    TokenKind::BitwiseNot,
    TokenKind::PlusAssign,
    TokenKind::MinusAssign,
    TokenKind::StarAssign,
    TokenKind::SlashAssign,
    TokenKind::PercentAssign,
    TokenKind::DotAssign,
    TokenKind::AndAssign,
    TokenKind::OrAssign,
    TokenKind::XorAssign,
    TokenKind::PowerAssign,
    TokenKind::LeftShiftAssign,
    TokenKind::RightShiftAssign,
    TokenKind::LogicalAndAssign,
    TokenKind::LogicalOrAssign,
    TokenKind::DefinedOrAssign,
    TokenKind::Equal,
    TokenKind::NotEqual,
    TokenKind::Match,
    TokenKind::NotMatch,
    TokenKind::SmartMatch,
    TokenKind::Less,
    TokenKind::Greater,
    TokenKind::LessEqual,
    TokenKind::GreaterEqual,
    TokenKind::Spaceship,
    TokenKind::StringCompare,
    TokenKind::And,
    TokenKind::Or,
    TokenKind::Not,
    TokenKind::DefinedOr,
    TokenKind::WordAnd,
    TokenKind::WordOr,
    TokenKind::WordNot,
    TokenKind::WordXor,
    TokenKind::Arrow,
    TokenKind::FatArrow,
    TokenKind::Dot,
    TokenKind::Range,
    TokenKind::Ellipsis,
    TokenKind::Increment,
    TokenKind::Decrement,
    TokenKind::DoubleColon,
    TokenKind::Question,
    TokenKind::Colon,
    TokenKind::Backslash,
    TokenKind::LeftParen,
    TokenKind::RightParen,
    TokenKind::LeftBrace,
    TokenKind::RightBrace,
    TokenKind::LeftBracket,
    TokenKind::RightBracket,
    TokenKind::Semicolon,
    TokenKind::Comma,
    TokenKind::Number,
    TokenKind::String,
    TokenKind::Regex,
    TokenKind::Substitution,
    TokenKind::Transliteration,
    TokenKind::QuoteSingle,
    TokenKind::QuoteDouble,
    TokenKind::QuoteWords,
    TokenKind::QuoteCommand,
    TokenKind::HeredocStart,
    TokenKind::HeredocBody,
    TokenKind::FormatBody,
    TokenKind::DataMarker,
    TokenKind::DataBody,
    TokenKind::VString,
    TokenKind::UnknownRest,
    TokenKind::HeredocDepthLimit,
    TokenKind::Identifier,
    TokenKind::ScalarSigil,
    TokenKind::ArraySigil,
    TokenKind::HashSigil,
    TokenKind::SubSigil,
    TokenKind::GlobSigil,
    TokenKind::Eof,
    TokenKind::Unknown,
];

impl TokenKind {
    /// Return metadata for this token kind.
    pub const fn info(self) -> TokenKindInfo {
        match self {
            TokenKind::My => TokenKindInfo {
                kind: TokenKind::My,
                display_name: "'my'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("my"),
                keyword_spelling: Some("my"),
                operator_spelling: None,
            },
            TokenKind::Our => TokenKindInfo {
                kind: TokenKind::Our,
                display_name: "'our'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("our"),
                keyword_spelling: Some("our"),
                operator_spelling: None,
            },
            TokenKind::Local => TokenKindInfo {
                kind: TokenKind::Local,
                display_name: "'local'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("local"),
                keyword_spelling: Some("local"),
                operator_spelling: None,
            },
            TokenKind::State => TokenKindInfo {
                kind: TokenKind::State,
                display_name: "'state'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("state"),
                keyword_spelling: Some("state"),
                operator_spelling: None,
            },
            TokenKind::Sub => TokenKindInfo {
                kind: TokenKind::Sub,
                display_name: "'sub'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("sub"),
                keyword_spelling: Some("sub"),
                operator_spelling: None,
            },
            TokenKind::If => TokenKindInfo {
                kind: TokenKind::If,
                display_name: "'if'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("if"),
                keyword_spelling: Some("if"),
                operator_spelling: None,
            },
            TokenKind::Elsif => TokenKindInfo {
                kind: TokenKind::Elsif,
                display_name: "'elsif'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("elsif"),
                keyword_spelling: Some("elsif"),
                operator_spelling: None,
            },
            TokenKind::Else => TokenKindInfo {
                kind: TokenKind::Else,
                display_name: "'else'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("else"),
                keyword_spelling: Some("else"),
                operator_spelling: None,
            },
            TokenKind::Unless => TokenKindInfo {
                kind: TokenKind::Unless,
                display_name: "'unless'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("unless"),
                keyword_spelling: Some("unless"),
                operator_spelling: None,
            },
            TokenKind::While => TokenKindInfo {
                kind: TokenKind::While,
                display_name: "'while'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("while"),
                keyword_spelling: Some("while"),
                operator_spelling: None,
            },
            TokenKind::Until => TokenKindInfo {
                kind: TokenKind::Until,
                display_name: "'until'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("until"),
                keyword_spelling: Some("until"),
                operator_spelling: None,
            },
            TokenKind::For => TokenKindInfo {
                kind: TokenKind::For,
                display_name: "'for'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("for"),
                keyword_spelling: Some("for"),
                operator_spelling: None,
            },
            TokenKind::Foreach => TokenKindInfo {
                kind: TokenKind::Foreach,
                display_name: "'foreach'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("foreach"),
                keyword_spelling: Some("foreach"),
                operator_spelling: None,
            },
            TokenKind::Return => TokenKindInfo {
                kind: TokenKind::Return,
                display_name: "'return'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("return"),
                keyword_spelling: Some("return"),
                operator_spelling: None,
            },
            TokenKind::Package => TokenKindInfo {
                kind: TokenKind::Package,
                display_name: "'package'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("package"),
                keyword_spelling: Some("package"),
                operator_spelling: None,
            },
            TokenKind::Use => TokenKindInfo {
                kind: TokenKind::Use,
                display_name: "'use'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("use"),
                keyword_spelling: Some("use"),
                operator_spelling: None,
            },
            TokenKind::No => TokenKindInfo {
                kind: TokenKind::No,
                display_name: "'no'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("no"),
                keyword_spelling: Some("no"),
                operator_spelling: None,
            },
            TokenKind::Begin => TokenKindInfo {
                kind: TokenKind::Begin,
                display_name: "'BEGIN'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("BEGIN"),
                keyword_spelling: Some("BEGIN"),
                operator_spelling: None,
            },
            TokenKind::End => TokenKindInfo {
                kind: TokenKind::End,
                display_name: "'END'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("END"),
                keyword_spelling: Some("END"),
                operator_spelling: None,
            },
            TokenKind::Check => TokenKindInfo {
                kind: TokenKind::Check,
                display_name: "'CHECK'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("CHECK"),
                keyword_spelling: Some("CHECK"),
                operator_spelling: None,
            },
            TokenKind::Init => TokenKindInfo {
                kind: TokenKind::Init,
                display_name: "'INIT'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("INIT"),
                keyword_spelling: Some("INIT"),
                operator_spelling: None,
            },
            TokenKind::Unitcheck => TokenKindInfo {
                kind: TokenKind::Unitcheck,
                display_name: "'UNITCHECK'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("UNITCHECK"),
                keyword_spelling: Some("UNITCHECK"),
                operator_spelling: None,
            },
            TokenKind::Eval => TokenKindInfo {
                kind: TokenKind::Eval,
                display_name: "'eval'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("eval"),
                keyword_spelling: Some("eval"),
                operator_spelling: None,
            },
            TokenKind::Do => TokenKindInfo {
                kind: TokenKind::Do,
                display_name: "'do'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("do"),
                keyword_spelling: Some("do"),
                operator_spelling: None,
            },
            TokenKind::Given => TokenKindInfo {
                kind: TokenKind::Given,
                display_name: "'given'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("given"),
                keyword_spelling: Some("given"),
                operator_spelling: None,
            },
            TokenKind::When => TokenKindInfo {
                kind: TokenKind::When,
                display_name: "'when'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("when"),
                keyword_spelling: Some("when"),
                operator_spelling: None,
            },
            TokenKind::Default => TokenKindInfo {
                kind: TokenKind::Default,
                display_name: "'default'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("default"),
                keyword_spelling: Some("default"),
                operator_spelling: None,
            },
            TokenKind::Try => TokenKindInfo {
                kind: TokenKind::Try,
                display_name: "'try'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("try"),
                keyword_spelling: Some("try"),
                operator_spelling: None,
            },
            TokenKind::Catch => TokenKindInfo {
                kind: TokenKind::Catch,
                display_name: "'catch'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("catch"),
                keyword_spelling: Some("catch"),
                operator_spelling: None,
            },
            TokenKind::Finally => TokenKindInfo {
                kind: TokenKind::Finally,
                display_name: "'finally'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("finally"),
                keyword_spelling: Some("finally"),
                operator_spelling: None,
            },
            TokenKind::Continue => TokenKindInfo {
                kind: TokenKind::Continue,
                display_name: "'continue'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("continue"),
                keyword_spelling: Some("continue"),
                operator_spelling: None,
            },
            TokenKind::Next => TokenKindInfo {
                kind: TokenKind::Next,
                display_name: "'next'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("next"),
                keyword_spelling: Some("next"),
                operator_spelling: None,
            },
            TokenKind::Last => TokenKindInfo {
                kind: TokenKind::Last,
                display_name: "'last'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("last"),
                keyword_spelling: Some("last"),
                operator_spelling: None,
            },
            TokenKind::Redo => TokenKindInfo {
                kind: TokenKind::Redo,
                display_name: "'redo'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("redo"),
                keyword_spelling: Some("redo"),
                operator_spelling: None,
            },
            TokenKind::Goto => TokenKindInfo {
                kind: TokenKind::Goto,
                display_name: "'goto'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("goto"),
                keyword_spelling: Some("goto"),
                operator_spelling: None,
            },
            TokenKind::Class => TokenKindInfo {
                kind: TokenKind::Class,
                display_name: "'class'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("class"),
                keyword_spelling: Some("class"),
                operator_spelling: None,
            },
            TokenKind::Method => TokenKindInfo {
                kind: TokenKind::Method,
                display_name: "'method'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("method"),
                keyword_spelling: Some("method"),
                operator_spelling: None,
            },
            TokenKind::Field => TokenKindInfo {
                kind: TokenKind::Field,
                display_name: "'field'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("field"),
                keyword_spelling: Some("field"),
                operator_spelling: None,
            },
            TokenKind::Format => TokenKindInfo {
                kind: TokenKind::Format,
                display_name: "'format'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("format"),
                keyword_spelling: Some("format"),
                operator_spelling: None,
            },
            TokenKind::Undef => TokenKindInfo {
                kind: TokenKind::Undef,
                display_name: "'undef'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("undef"),
                keyword_spelling: Some("undef"),
                operator_spelling: None,
            },
            TokenKind::Defer => TokenKindInfo {
                kind: TokenKind::Defer,
                display_name: "'defer'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("defer"),
                keyword_spelling: Some("defer"),
                operator_spelling: None,
            },
            TokenKind::Assign => TokenKindInfo {
                kind: TokenKind::Assign,
                display_name: "'='",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("="),
                keyword_spelling: None,
                operator_spelling: Some("="),
            },
            TokenKind::Plus => TokenKindInfo {
                kind: TokenKind::Plus,
                display_name: "'+'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("+"),
                keyword_spelling: None,
                operator_spelling: Some("+"),
            },
            TokenKind::Minus => TokenKindInfo {
                kind: TokenKind::Minus,
                display_name: "'-'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("-"),
                keyword_spelling: None,
                operator_spelling: Some("-"),
            },
            TokenKind::Star => TokenKindInfo {
                kind: TokenKind::Star,
                display_name: "'*'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("*"),
                keyword_spelling: None,
                operator_spelling: Some("*"),
            },
            TokenKind::Slash => TokenKindInfo {
                kind: TokenKind::Slash,
                display_name: "'/'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("/"),
                keyword_spelling: None,
                operator_spelling: Some("/"),
            },
            TokenKind::Percent => TokenKindInfo {
                kind: TokenKind::Percent,
                display_name: "'%'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("%"),
                keyword_spelling: None,
                operator_spelling: Some("%"),
            },
            TokenKind::Power => TokenKindInfo {
                kind: TokenKind::Power,
                display_name: "'**'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("**"),
                keyword_spelling: None,
                operator_spelling: Some("**"),
            },
            TokenKind::LeftShift => TokenKindInfo {
                kind: TokenKind::LeftShift,
                display_name: "'<<'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("<<"),
                keyword_spelling: None,
                operator_spelling: Some("<<"),
            },
            TokenKind::RightShift => TokenKindInfo {
                kind: TokenKind::RightShift,
                display_name: "'>>'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some(">>"),
                keyword_spelling: None,
                operator_spelling: Some(">>"),
            },
            TokenKind::BitwiseAnd => TokenKindInfo {
                kind: TokenKind::BitwiseAnd,
                display_name: "'&'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("&"),
                keyword_spelling: None,
                operator_spelling: Some("&"),
            },
            TokenKind::BitwiseOr => TokenKindInfo {
                kind: TokenKind::BitwiseOr,
                display_name: "'|'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("|"),
                keyword_spelling: None,
                operator_spelling: Some("|"),
            },
            TokenKind::BitwiseXor => TokenKindInfo {
                kind: TokenKind::BitwiseXor,
                display_name: "'^'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("^"),
                keyword_spelling: None,
                operator_spelling: Some("^"),
            },
            TokenKind::BitwiseNot => TokenKindInfo {
                kind: TokenKind::BitwiseNot,
                display_name: "'~'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("~"),
                keyword_spelling: None,
                operator_spelling: Some("~"),
            },
            TokenKind::PlusAssign => TokenKindInfo {
                kind: TokenKind::PlusAssign,
                display_name: "'+='",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("+="),
                keyword_spelling: None,
                operator_spelling: Some("+="),
            },
            TokenKind::MinusAssign => TokenKindInfo {
                kind: TokenKind::MinusAssign,
                display_name: "'-='",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("-="),
                keyword_spelling: None,
                operator_spelling: Some("-="),
            },
            TokenKind::StarAssign => TokenKindInfo {
                kind: TokenKind::StarAssign,
                display_name: "'*='",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("*="),
                keyword_spelling: None,
                operator_spelling: Some("*="),
            },
            TokenKind::SlashAssign => TokenKindInfo {
                kind: TokenKind::SlashAssign,
                display_name: "'/='",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("/="),
                keyword_spelling: None,
                operator_spelling: Some("/="),
            },
            TokenKind::PercentAssign => TokenKindInfo {
                kind: TokenKind::PercentAssign,
                display_name: "'%='",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("%="),
                keyword_spelling: None,
                operator_spelling: Some("%="),
            },
            TokenKind::DotAssign => TokenKindInfo {
                kind: TokenKind::DotAssign,
                display_name: "'.='",
                category: TokenCategory::Operator,
                canonical_lexeme: Some(".="),
                keyword_spelling: None,
                operator_spelling: Some(".="),
            },
            TokenKind::AndAssign => TokenKindInfo {
                kind: TokenKind::AndAssign,
                display_name: "'&='",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("&="),
                keyword_spelling: None,
                operator_spelling: Some("&="),
            },
            TokenKind::OrAssign => TokenKindInfo {
                kind: TokenKind::OrAssign,
                display_name: "'|='",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("|="),
                keyword_spelling: None,
                operator_spelling: Some("|="),
            },
            TokenKind::XorAssign => TokenKindInfo {
                kind: TokenKind::XorAssign,
                display_name: "'^='",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("^="),
                keyword_spelling: None,
                operator_spelling: Some("^="),
            },
            TokenKind::PowerAssign => TokenKindInfo {
                kind: TokenKind::PowerAssign,
                display_name: "'**='",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("**="),
                keyword_spelling: None,
                operator_spelling: Some("**="),
            },
            TokenKind::LeftShiftAssign => TokenKindInfo {
                kind: TokenKind::LeftShiftAssign,
                display_name: "'<<='",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("<<="),
                keyword_spelling: None,
                operator_spelling: Some("<<="),
            },
            TokenKind::RightShiftAssign => TokenKindInfo {
                kind: TokenKind::RightShiftAssign,
                display_name: "'>>='",
                category: TokenCategory::Operator,
                canonical_lexeme: Some(">>="),
                keyword_spelling: None,
                operator_spelling: Some(">>="),
            },
            TokenKind::LogicalAndAssign => TokenKindInfo {
                kind: TokenKind::LogicalAndAssign,
                display_name: "'&&='",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("&&="),
                keyword_spelling: None,
                operator_spelling: Some("&&="),
            },
            TokenKind::LogicalOrAssign => TokenKindInfo {
                kind: TokenKind::LogicalOrAssign,
                display_name: "'||='",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("||="),
                keyword_spelling: None,
                operator_spelling: Some("||="),
            },
            TokenKind::DefinedOrAssign => TokenKindInfo {
                kind: TokenKind::DefinedOrAssign,
                display_name: "'//='",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("//="),
                keyword_spelling: None,
                operator_spelling: Some("//="),
            },
            TokenKind::Equal => TokenKindInfo {
                kind: TokenKind::Equal,
                display_name: "'=='",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("=="),
                keyword_spelling: None,
                operator_spelling: Some("=="),
            },
            TokenKind::NotEqual => TokenKindInfo {
                kind: TokenKind::NotEqual,
                display_name: "'!='",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("!="),
                keyword_spelling: None,
                operator_spelling: Some("!="),
            },
            TokenKind::Match => TokenKindInfo {
                kind: TokenKind::Match,
                display_name: "'=~'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("=~"),
                keyword_spelling: None,
                operator_spelling: Some("=~"),
            },
            TokenKind::NotMatch => TokenKindInfo {
                kind: TokenKind::NotMatch,
                display_name: "'!~'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("!~"),
                keyword_spelling: None,
                operator_spelling: Some("!~"),
            },
            TokenKind::SmartMatch => TokenKindInfo {
                kind: TokenKind::SmartMatch,
                display_name: "'~~'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("~~"),
                keyword_spelling: None,
                operator_spelling: Some("~~"),
            },
            TokenKind::Less => TokenKindInfo {
                kind: TokenKind::Less,
                display_name: "'<'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("<"),
                keyword_spelling: None,
                operator_spelling: Some("<"),
            },
            TokenKind::Greater => TokenKindInfo {
                kind: TokenKind::Greater,
                display_name: "'>'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some(">"),
                keyword_spelling: None,
                operator_spelling: Some(">"),
            },
            TokenKind::LessEqual => TokenKindInfo {
                kind: TokenKind::LessEqual,
                display_name: "'<='",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("<="),
                keyword_spelling: None,
                operator_spelling: Some("<="),
            },
            TokenKind::GreaterEqual => TokenKindInfo {
                kind: TokenKind::GreaterEqual,
                display_name: "'>='",
                category: TokenCategory::Operator,
                canonical_lexeme: Some(">="),
                keyword_spelling: None,
                operator_spelling: Some(">="),
            },
            TokenKind::Spaceship => TokenKindInfo {
                kind: TokenKind::Spaceship,
                display_name: "'<=>'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("<=>"),
                keyword_spelling: None,
                operator_spelling: Some("<=>"),
            },
            TokenKind::StringCompare => TokenKindInfo {
                kind: TokenKind::StringCompare,
                display_name: "'cmp'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("cmp"),
                keyword_spelling: None,
                operator_spelling: Some("cmp"),
            },
            TokenKind::And => TokenKindInfo {
                kind: TokenKind::And,
                display_name: "'&&'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("&&"),
                keyword_spelling: None,
                operator_spelling: Some("&&"),
            },
            TokenKind::Or => TokenKindInfo {
                kind: TokenKind::Or,
                display_name: "'||'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("||"),
                keyword_spelling: None,
                operator_spelling: Some("||"),
            },
            TokenKind::Not => TokenKindInfo {
                kind: TokenKind::Not,
                display_name: "'!'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("!"),
                keyword_spelling: None,
                operator_spelling: Some("!"),
            },
            TokenKind::DefinedOr => TokenKindInfo {
                kind: TokenKind::DefinedOr,
                display_name: "'//'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("//"),
                keyword_spelling: None,
                operator_spelling: Some("//"),
            },
            TokenKind::WordAnd => TokenKindInfo {
                kind: TokenKind::WordAnd,
                display_name: "'and'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("and"),
                keyword_spelling: None,
                operator_spelling: Some("and"),
            },
            TokenKind::WordOr => TokenKindInfo {
                kind: TokenKind::WordOr,
                display_name: "'or'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("or"),
                keyword_spelling: None,
                operator_spelling: Some("or"),
            },
            TokenKind::WordNot => TokenKindInfo {
                kind: TokenKind::WordNot,
                display_name: "'not'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("not"),
                keyword_spelling: None,
                operator_spelling: Some("not"),
            },
            TokenKind::WordXor => TokenKindInfo {
                kind: TokenKind::WordXor,
                display_name: "'xor'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("xor"),
                keyword_spelling: None,
                operator_spelling: Some("xor"),
            },
            TokenKind::Arrow => TokenKindInfo {
                kind: TokenKind::Arrow,
                display_name: "'->'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("->"),
                keyword_spelling: None,
                operator_spelling: Some("->"),
            },
            TokenKind::FatArrow => TokenKindInfo {
                kind: TokenKind::FatArrow,
                display_name: "'=>'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("=>"),
                keyword_spelling: None,
                operator_spelling: Some("=>"),
            },
            TokenKind::Dot => TokenKindInfo {
                kind: TokenKind::Dot,
                display_name: "'.'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("."),
                keyword_spelling: None,
                operator_spelling: Some("."),
            },
            TokenKind::Range => TokenKindInfo {
                kind: TokenKind::Range,
                display_name: "'..'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some(".."),
                keyword_spelling: None,
                operator_spelling: Some(".."),
            },
            TokenKind::Ellipsis => TokenKindInfo {
                kind: TokenKind::Ellipsis,
                display_name: "'...'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("..."),
                keyword_spelling: None,
                operator_spelling: Some("..."),
            },
            TokenKind::Increment => TokenKindInfo {
                kind: TokenKind::Increment,
                display_name: "'++'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("++"),
                keyword_spelling: None,
                operator_spelling: Some("++"),
            },
            TokenKind::Decrement => TokenKindInfo {
                kind: TokenKind::Decrement,
                display_name: "'--'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("--"),
                keyword_spelling: None,
                operator_spelling: Some("--"),
            },
            TokenKind::DoubleColon => TokenKindInfo {
                kind: TokenKind::DoubleColon,
                display_name: "'::'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("::"),
                keyword_spelling: None,
                operator_spelling: Some("::"),
            },
            TokenKind::Question => TokenKindInfo {
                kind: TokenKind::Question,
                display_name: "'?'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("?"),
                keyword_spelling: None,
                operator_spelling: Some("?"),
            },
            TokenKind::Colon => TokenKindInfo {
                kind: TokenKind::Colon,
                display_name: "':'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some(":"),
                keyword_spelling: None,
                operator_spelling: Some(":"),
            },
            TokenKind::Backslash => TokenKindInfo {
                kind: TokenKind::Backslash,
                display_name: "'\\'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("\\"),
                keyword_spelling: None,
                operator_spelling: Some("\\"),
            },
            TokenKind::LeftParen => TokenKindInfo {
                kind: TokenKind::LeftParen,
                display_name: "'('",
                category: TokenCategory::Delimiter,
                canonical_lexeme: Some("("),
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::RightParen => TokenKindInfo {
                kind: TokenKind::RightParen,
                display_name: "')'",
                category: TokenCategory::Delimiter,
                canonical_lexeme: Some(")"),
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::LeftBrace => TokenKindInfo {
                kind: TokenKind::LeftBrace,
                display_name: "'{'",
                category: TokenCategory::Delimiter,
                canonical_lexeme: Some("{"),
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::RightBrace => TokenKindInfo {
                kind: TokenKind::RightBrace,
                display_name: "'}'",
                category: TokenCategory::Delimiter,
                canonical_lexeme: Some("}"),
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::LeftBracket => TokenKindInfo {
                kind: TokenKind::LeftBracket,
                display_name: "'['",
                category: TokenCategory::Delimiter,
                canonical_lexeme: Some("["),
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::RightBracket => TokenKindInfo {
                kind: TokenKind::RightBracket,
                display_name: "']'",
                category: TokenCategory::Delimiter,
                canonical_lexeme: Some("]"),
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::Semicolon => TokenKindInfo {
                kind: TokenKind::Semicolon,
                display_name: "';'",
                category: TokenCategory::Delimiter,
                canonical_lexeme: Some(";"),
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::Comma => TokenKindInfo {
                kind: TokenKind::Comma,
                display_name: "','",
                category: TokenCategory::Delimiter,
                canonical_lexeme: Some(","),
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::Number => TokenKindInfo {
                kind: TokenKind::Number,
                display_name: "number",
                category: TokenCategory::Literal,
                canonical_lexeme: None,
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::String => TokenKindInfo {
                kind: TokenKind::String,
                display_name: "string",
                category: TokenCategory::Literal,
                canonical_lexeme: None,
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::Regex => TokenKindInfo {
                kind: TokenKind::Regex,
                display_name: "regex",
                category: TokenCategory::Literal,
                canonical_lexeme: None,
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::Substitution => TokenKindInfo {
                kind: TokenKind::Substitution,
                display_name: "substitution (s///)",
                category: TokenCategory::Literal,
                canonical_lexeme: None,
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::Transliteration => TokenKindInfo {
                kind: TokenKind::Transliteration,
                display_name: "transliteration (tr///)",
                category: TokenCategory::Literal,
                canonical_lexeme: None,
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::QuoteSingle => TokenKindInfo {
                kind: TokenKind::QuoteSingle,
                display_name: "q// string",
                category: TokenCategory::Literal,
                canonical_lexeme: None,
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::QuoteDouble => TokenKindInfo {
                kind: TokenKind::QuoteDouble,
                display_name: "qq// string",
                category: TokenCategory::Literal,
                canonical_lexeme: None,
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::QuoteWords => TokenKindInfo {
                kind: TokenKind::QuoteWords,
                display_name: "qw() word list",
                category: TokenCategory::Literal,
                canonical_lexeme: None,
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::QuoteCommand => TokenKindInfo {
                kind: TokenKind::QuoteCommand,
                display_name: "qx// command",
                category: TokenCategory::Literal,
                canonical_lexeme: None,
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::HeredocStart => TokenKindInfo {
                kind: TokenKind::HeredocStart,
                display_name: "heredoc (<<)",
                category: TokenCategory::Literal,
                canonical_lexeme: None,
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::HeredocBody => TokenKindInfo {
                kind: TokenKind::HeredocBody,
                display_name: "heredoc body",
                category: TokenCategory::Literal,
                canonical_lexeme: None,
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::FormatBody => TokenKindInfo {
                kind: TokenKind::FormatBody,
                display_name: "format body",
                category: TokenCategory::Literal,
                canonical_lexeme: None,
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::DataMarker => TokenKindInfo {
                kind: TokenKind::DataMarker,
                display_name: "__DATA__",
                category: TokenCategory::Literal,
                canonical_lexeme: Some("__DATA__"),
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::DataBody => TokenKindInfo {
                kind: TokenKind::DataBody,
                display_name: "data section",
                category: TokenCategory::Literal,
                canonical_lexeme: None,
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::VString => TokenKindInfo {
                kind: TokenKind::VString,
                display_name: "version string",
                category: TokenCategory::Literal,
                canonical_lexeme: None,
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::UnknownRest => TokenKindInfo {
                kind: TokenKind::UnknownRest,
                display_name: "unparsed content",
                category: TokenCategory::Literal,
                canonical_lexeme: None,
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::HeredocDepthLimit => TokenKindInfo {
                kind: TokenKind::HeredocDepthLimit,
                display_name: "heredoc depth limit",
                category: TokenCategory::Literal,
                canonical_lexeme: None,
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::Identifier => TokenKindInfo {
                kind: TokenKind::Identifier,
                display_name: "identifier",
                category: TokenCategory::Identifier,
                canonical_lexeme: None,
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::ScalarSigil => TokenKindInfo {
                kind: TokenKind::ScalarSigil,
                display_name: "'$'",
                category: TokenCategory::Sigil,
                canonical_lexeme: Some("$"),
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::ArraySigil => TokenKindInfo {
                kind: TokenKind::ArraySigil,
                display_name: "'@'",
                category: TokenCategory::Sigil,
                canonical_lexeme: Some("@"),
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::HashSigil => TokenKindInfo {
                kind: TokenKind::HashSigil,
                display_name: "'%'",
                category: TokenCategory::Sigil,
                canonical_lexeme: Some("%"),
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::SubSigil => TokenKindInfo {
                kind: TokenKind::SubSigil,
                display_name: "'&'",
                category: TokenCategory::Sigil,
                canonical_lexeme: Some("&"),
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::GlobSigil => TokenKindInfo {
                kind: TokenKind::GlobSigil,
                display_name: "'*'",
                category: TokenCategory::Sigil,
                canonical_lexeme: Some("*"),
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::Eof => TokenKindInfo {
                kind: TokenKind::Eof,
                display_name: "end of input",
                category: TokenCategory::Special,
                canonical_lexeme: None,
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::Unknown => TokenKindInfo {
                kind: TokenKind::Unknown,
                display_name: "unknown token",
                category: TokenCategory::Special,
                canonical_lexeme: None,
                keyword_spelling: None,
                operator_spelling: None,
            },
        }
    }

    /// Return all token kinds in stable declaration order.
    pub const fn all() -> &'static [TokenKind] {
        &ALL_TOKEN_KINDS
    }

    /// Return a user-friendly display name for this token kind.
    pub const fn display_name(self) -> &'static str {
        self.info().display_name
    }

    /// Return the high-level token category.
    pub const fn category(self) -> TokenCategory {
        self.info().category
    }

    pub const fn is_keyword(self) -> bool {
        matches!(self.category(), TokenCategory::Keyword)
    }

    pub const fn is_operator(self) -> bool {
        matches!(self.category(), TokenCategory::Operator)
    }

    pub const fn is_delimiter(self) -> bool {
        matches!(self.category(), TokenCategory::Delimiter)
    }

    pub const fn is_literal(self) -> bool {
        matches!(self.category(), TokenCategory::Literal)
    }

    pub const fn is_sigil(self) -> bool {
        matches!(self.category(), TokenCategory::Sigil)
    }

    pub const fn is_special(self) -> bool {
        matches!(self.category(), TokenCategory::Special)
    }

    pub const fn is_identifier_like(self) -> bool {
        matches!(self.category(), TokenCategory::Identifier | TokenCategory::Sigil)
    }

    pub const fn canonical_lexeme(self) -> Option<&'static str> {
        self.info().canonical_lexeme
    }
}
