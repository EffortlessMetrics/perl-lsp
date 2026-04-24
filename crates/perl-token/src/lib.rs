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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenCategory {
    Keyword,
    Operator,
    Delimiter,
    Literal,
    Identifier,
    Sigil,
    Special,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenKindInfo {
    pub kind: TokenKind,
    pub display_name: &'static str,
    pub category: TokenCategory,
    pub canonical_lexeme: Option<&'static str>,
    pub keyword_spelling: Option<&'static str>,
    pub operator_spelling: Option<&'static str>,
}

impl TokenKind {
    pub const ALL: &'static [TokenKind] = &[
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

    pub const fn all() -> &'static [TokenKind] {
        Self::ALL
    }

    pub const fn info(self) -> TokenKindInfo {
        match self {
            TokenKind::My => TokenKindInfo {
                kind: self,
                display_name: "'my'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("my"),
                keyword_spelling: Some("my"),
                operator_spelling: None,
            },
            TokenKind::Our => TokenKindInfo {
                kind: self,
                display_name: "'our'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("our"),
                keyword_spelling: Some("our"),
                operator_spelling: None,
            },
            TokenKind::Local => TokenKindInfo {
                kind: self,
                display_name: "'local'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("local"),
                keyword_spelling: Some("local"),
                operator_spelling: None,
            },
            TokenKind::State => TokenKindInfo {
                kind: self,
                display_name: "'state'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("state"),
                keyword_spelling: Some("state"),
                operator_spelling: None,
            },
            TokenKind::Sub => TokenKindInfo {
                kind: self,
                display_name: "'sub'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("sub"),
                keyword_spelling: Some("sub"),
                operator_spelling: None,
            },
            TokenKind::If => TokenKindInfo {
                kind: self,
                display_name: "'if'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("if"),
                keyword_spelling: Some("if"),
                operator_spelling: None,
            },
            TokenKind::Elsif => TokenKindInfo {
                kind: self,
                display_name: "'elsif'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("elsif"),
                keyword_spelling: Some("elsif"),
                operator_spelling: None,
            },
            TokenKind::Else => TokenKindInfo {
                kind: self,
                display_name: "'else'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("else"),
                keyword_spelling: Some("else"),
                operator_spelling: None,
            },
            TokenKind::Unless => TokenKindInfo {
                kind: self,
                display_name: "'unless'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("unless"),
                keyword_spelling: Some("unless"),
                operator_spelling: None,
            },
            TokenKind::While => TokenKindInfo {
                kind: self,
                display_name: "'while'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("while"),
                keyword_spelling: Some("while"),
                operator_spelling: None,
            },
            TokenKind::Until => TokenKindInfo {
                kind: self,
                display_name: "'until'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("until"),
                keyword_spelling: Some("until"),
                operator_spelling: None,
            },
            TokenKind::For => TokenKindInfo {
                kind: self,
                display_name: "'for'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("for"),
                keyword_spelling: Some("for"),
                operator_spelling: None,
            },
            TokenKind::Foreach => TokenKindInfo {
                kind: self,
                display_name: "'foreach'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("foreach"),
                keyword_spelling: Some("foreach"),
                operator_spelling: None,
            },
            TokenKind::Return => TokenKindInfo {
                kind: self,
                display_name: "'return'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("return"),
                keyword_spelling: Some("return"),
                operator_spelling: None,
            },
            TokenKind::Package => TokenKindInfo {
                kind: self,
                display_name: "'package'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("package"),
                keyword_spelling: Some("package"),
                operator_spelling: None,
            },
            TokenKind::Use => TokenKindInfo {
                kind: self,
                display_name: "'use'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("use"),
                keyword_spelling: Some("use"),
                operator_spelling: None,
            },
            TokenKind::No => TokenKindInfo {
                kind: self,
                display_name: "'no'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("no"),
                keyword_spelling: Some("no"),
                operator_spelling: None,
            },
            TokenKind::Begin => TokenKindInfo {
                kind: self,
                display_name: "'BEGIN'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("BEGIN"),
                keyword_spelling: Some("BEGIN"),
                operator_spelling: None,
            },
            TokenKind::End => TokenKindInfo {
                kind: self,
                display_name: "'END'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("END"),
                keyword_spelling: Some("END"),
                operator_spelling: None,
            },
            TokenKind::Check => TokenKindInfo {
                kind: self,
                display_name: "'CHECK'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("CHECK"),
                keyword_spelling: Some("CHECK"),
                operator_spelling: None,
            },
            TokenKind::Init => TokenKindInfo {
                kind: self,
                display_name: "'INIT'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("INIT"),
                keyword_spelling: Some("INIT"),
                operator_spelling: None,
            },
            TokenKind::Unitcheck => TokenKindInfo {
                kind: self,
                display_name: "'UNITCHECK'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("UNITCHECK"),
                keyword_spelling: Some("UNITCHECK"),
                operator_spelling: None,
            },
            TokenKind::Eval => TokenKindInfo {
                kind: self,
                display_name: "'eval'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("eval"),
                keyword_spelling: Some("eval"),
                operator_spelling: None,
            },
            TokenKind::Do => TokenKindInfo {
                kind: self,
                display_name: "'do'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("do"),
                keyword_spelling: Some("do"),
                operator_spelling: None,
            },
            TokenKind::Given => TokenKindInfo {
                kind: self,
                display_name: "'given'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("given"),
                keyword_spelling: Some("given"),
                operator_spelling: None,
            },
            TokenKind::When => TokenKindInfo {
                kind: self,
                display_name: "'when'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("when"),
                keyword_spelling: Some("when"),
                operator_spelling: None,
            },
            TokenKind::Default => TokenKindInfo {
                kind: self,
                display_name: "'default'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("default"),
                keyword_spelling: Some("default"),
                operator_spelling: None,
            },
            TokenKind::Try => TokenKindInfo {
                kind: self,
                display_name: "'try'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("try"),
                keyword_spelling: Some("try"),
                operator_spelling: None,
            },
            TokenKind::Catch => TokenKindInfo {
                kind: self,
                display_name: "'catch'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("catch"),
                keyword_spelling: Some("catch"),
                operator_spelling: None,
            },
            TokenKind::Finally => TokenKindInfo {
                kind: self,
                display_name: "'finally'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("finally"),
                keyword_spelling: Some("finally"),
                operator_spelling: None,
            },
            TokenKind::Continue => TokenKindInfo {
                kind: self,
                display_name: "'continue'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("continue"),
                keyword_spelling: Some("continue"),
                operator_spelling: None,
            },
            TokenKind::Next => TokenKindInfo {
                kind: self,
                display_name: "'next'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("next"),
                keyword_spelling: Some("next"),
                operator_spelling: None,
            },
            TokenKind::Last => TokenKindInfo {
                kind: self,
                display_name: "'last'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("last"),
                keyword_spelling: Some("last"),
                operator_spelling: None,
            },
            TokenKind::Redo => TokenKindInfo {
                kind: self,
                display_name: "'redo'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("redo"),
                keyword_spelling: Some("redo"),
                operator_spelling: None,
            },
            TokenKind::Goto => TokenKindInfo {
                kind: self,
                display_name: "'goto'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("goto"),
                keyword_spelling: Some("goto"),
                operator_spelling: None,
            },
            TokenKind::Class => TokenKindInfo {
                kind: self,
                display_name: "'class'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("class"),
                keyword_spelling: Some("class"),
                operator_spelling: None,
            },
            TokenKind::Method => TokenKindInfo {
                kind: self,
                display_name: "'method'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("method"),
                keyword_spelling: Some("method"),
                operator_spelling: None,
            },
            TokenKind::Field => TokenKindInfo {
                kind: self,
                display_name: "'field'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("field"),
                keyword_spelling: Some("field"),
                operator_spelling: None,
            },
            TokenKind::Format => TokenKindInfo {
                kind: self,
                display_name: "'format'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("format"),
                keyword_spelling: Some("format"),
                operator_spelling: None,
            },
            TokenKind::Undef => TokenKindInfo {
                kind: self,
                display_name: "'undef'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("undef"),
                keyword_spelling: Some("undef"),
                operator_spelling: None,
            },
            TokenKind::Defer => TokenKindInfo {
                kind: self,
                display_name: "'defer'",
                category: TokenCategory::Keyword,
                canonical_lexeme: Some("defer"),
                keyword_spelling: Some("defer"),
                operator_spelling: None,
            },
            TokenKind::Assign => TokenKindInfo {
                kind: self,
                display_name: "'='",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("="),
                keyword_spelling: None,
                operator_spelling: Some("="),
            },
            TokenKind::Plus => TokenKindInfo {
                kind: self,
                display_name: "'+'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("+"),
                keyword_spelling: None,
                operator_spelling: Some("+"),
            },
            TokenKind::Minus => TokenKindInfo {
                kind: self,
                display_name: "'-'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("-"),
                keyword_spelling: None,
                operator_spelling: Some("-"),
            },
            TokenKind::Star => TokenKindInfo {
                kind: self,
                display_name: "'*'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("*"),
                keyword_spelling: None,
                operator_spelling: Some("*"),
            },
            TokenKind::Slash => TokenKindInfo {
                kind: self,
                display_name: "'/'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("/"),
                keyword_spelling: None,
                operator_spelling: Some("/"),
            },
            TokenKind::Percent => TokenKindInfo {
                kind: self,
                display_name: "'%'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("%"),
                keyword_spelling: None,
                operator_spelling: Some("%"),
            },
            TokenKind::Power => TokenKindInfo {
                kind: self,
                display_name: "'**'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("**"),
                keyword_spelling: None,
                operator_spelling: Some("**"),
            },
            TokenKind::LeftShift => TokenKindInfo {
                kind: self,
                display_name: "'<<'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("<<"),
                keyword_spelling: None,
                operator_spelling: Some("<<"),
            },
            TokenKind::RightShift => TokenKindInfo {
                kind: self,
                display_name: "'>>'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some(">>"),
                keyword_spelling: None,
                operator_spelling: Some(">>"),
            },
            TokenKind::BitwiseAnd => TokenKindInfo {
                kind: self,
                display_name: "'&'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("&"),
                keyword_spelling: None,
                operator_spelling: Some("&"),
            },
            TokenKind::BitwiseOr => TokenKindInfo {
                kind: self,
                display_name: "'|'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("|"),
                keyword_spelling: None,
                operator_spelling: Some("|"),
            },
            TokenKind::BitwiseXor => TokenKindInfo {
                kind: self,
                display_name: "'^'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("^"),
                keyword_spelling: None,
                operator_spelling: Some("^"),
            },
            TokenKind::BitwiseNot => TokenKindInfo {
                kind: self,
                display_name: "'~'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("~"),
                keyword_spelling: None,
                operator_spelling: Some("~"),
            },
            TokenKind::PlusAssign => TokenKindInfo {
                kind: self,
                display_name: "'+='",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("+="),
                keyword_spelling: None,
                operator_spelling: Some("+="),
            },
            TokenKind::MinusAssign => TokenKindInfo {
                kind: self,
                display_name: "'-='",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("-="),
                keyword_spelling: None,
                operator_spelling: Some("-="),
            },
            TokenKind::StarAssign => TokenKindInfo {
                kind: self,
                display_name: "'*='",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("*="),
                keyword_spelling: None,
                operator_spelling: Some("*="),
            },
            TokenKind::SlashAssign => TokenKindInfo {
                kind: self,
                display_name: "'/='",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("/="),
                keyword_spelling: None,
                operator_spelling: Some("/="),
            },
            TokenKind::PercentAssign => TokenKindInfo {
                kind: self,
                display_name: "'%='",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("%="),
                keyword_spelling: None,
                operator_spelling: Some("%="),
            },
            TokenKind::DotAssign => TokenKindInfo {
                kind: self,
                display_name: "'.='",
                category: TokenCategory::Operator,
                canonical_lexeme: Some(".="),
                keyword_spelling: None,
                operator_spelling: Some(".="),
            },
            TokenKind::AndAssign => TokenKindInfo {
                kind: self,
                display_name: "'&='",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("&="),
                keyword_spelling: None,
                operator_spelling: Some("&="),
            },
            TokenKind::OrAssign => TokenKindInfo {
                kind: self,
                display_name: "'|='",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("|="),
                keyword_spelling: None,
                operator_spelling: Some("|="),
            },
            TokenKind::XorAssign => TokenKindInfo {
                kind: self,
                display_name: "'^='",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("^="),
                keyword_spelling: None,
                operator_spelling: Some("^="),
            },
            TokenKind::PowerAssign => TokenKindInfo {
                kind: self,
                display_name: "'**='",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("**="),
                keyword_spelling: None,
                operator_spelling: Some("**="),
            },
            TokenKind::LeftShiftAssign => TokenKindInfo {
                kind: self,
                display_name: "'<<='",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("<<="),
                keyword_spelling: None,
                operator_spelling: Some("<<="),
            },
            TokenKind::RightShiftAssign => TokenKindInfo {
                kind: self,
                display_name: "'>>='",
                category: TokenCategory::Operator,
                canonical_lexeme: Some(">>="),
                keyword_spelling: None,
                operator_spelling: Some(">>="),
            },
            TokenKind::LogicalAndAssign => TokenKindInfo {
                kind: self,
                display_name: "'&&='",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("&&="),
                keyword_spelling: None,
                operator_spelling: Some("&&="),
            },
            TokenKind::LogicalOrAssign => TokenKindInfo {
                kind: self,
                display_name: "'||='",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("||="),
                keyword_spelling: None,
                operator_spelling: Some("||="),
            },
            TokenKind::DefinedOrAssign => TokenKindInfo {
                kind: self,
                display_name: "'//='",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("//="),
                keyword_spelling: None,
                operator_spelling: Some("//="),
            },
            TokenKind::Equal => TokenKindInfo {
                kind: self,
                display_name: "'=='",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("=="),
                keyword_spelling: None,
                operator_spelling: Some("=="),
            },
            TokenKind::NotEqual => TokenKindInfo {
                kind: self,
                display_name: "'!='",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("!="),
                keyword_spelling: None,
                operator_spelling: Some("!="),
            },
            TokenKind::Match => TokenKindInfo {
                kind: self,
                display_name: "'=~'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("=~"),
                keyword_spelling: None,
                operator_spelling: Some("=~"),
            },
            TokenKind::NotMatch => TokenKindInfo {
                kind: self,
                display_name: "'!~'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("!~"),
                keyword_spelling: None,
                operator_spelling: Some("!~"),
            },
            TokenKind::SmartMatch => TokenKindInfo {
                kind: self,
                display_name: "'~~'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("~~"),
                keyword_spelling: None,
                operator_spelling: Some("~~"),
            },
            TokenKind::Less => TokenKindInfo {
                kind: self,
                display_name: "'<'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("<"),
                keyword_spelling: None,
                operator_spelling: Some("<"),
            },
            TokenKind::Greater => TokenKindInfo {
                kind: self,
                display_name: "'>'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some(">"),
                keyword_spelling: None,
                operator_spelling: Some(">"),
            },
            TokenKind::LessEqual => TokenKindInfo {
                kind: self,
                display_name: "'<='",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("<="),
                keyword_spelling: None,
                operator_spelling: Some("<="),
            },
            TokenKind::GreaterEqual => TokenKindInfo {
                kind: self,
                display_name: "'>='",
                category: TokenCategory::Operator,
                canonical_lexeme: Some(">="),
                keyword_spelling: None,
                operator_spelling: Some(">="),
            },
            TokenKind::Spaceship => TokenKindInfo {
                kind: self,
                display_name: "'<=>'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("<=>"),
                keyword_spelling: None,
                operator_spelling: Some("<=>"),
            },
            TokenKind::StringCompare => TokenKindInfo {
                kind: self,
                display_name: "'cmp'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("cmp"),
                keyword_spelling: None,
                operator_spelling: Some("cmp"),
            },
            TokenKind::And => TokenKindInfo {
                kind: self,
                display_name: "'&&'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("&&"),
                keyword_spelling: None,
                operator_spelling: Some("&&"),
            },
            TokenKind::Or => TokenKindInfo {
                kind: self,
                display_name: "'||'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("||"),
                keyword_spelling: None,
                operator_spelling: Some("||"),
            },
            TokenKind::Not => TokenKindInfo {
                kind: self,
                display_name: "'!'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("!"),
                keyword_spelling: None,
                operator_spelling: Some("!"),
            },
            TokenKind::DefinedOr => TokenKindInfo {
                kind: self,
                display_name: "'//'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("//"),
                keyword_spelling: None,
                operator_spelling: Some("//"),
            },
            TokenKind::WordAnd => TokenKindInfo {
                kind: self,
                display_name: "'and'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("and"),
                keyword_spelling: None,
                operator_spelling: Some("and"),
            },
            TokenKind::WordOr => TokenKindInfo {
                kind: self,
                display_name: "'or'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("or"),
                keyword_spelling: None,
                operator_spelling: Some("or"),
            },
            TokenKind::WordNot => TokenKindInfo {
                kind: self,
                display_name: "'not'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("not"),
                keyword_spelling: None,
                operator_spelling: Some("not"),
            },
            TokenKind::WordXor => TokenKindInfo {
                kind: self,
                display_name: "'xor'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("xor"),
                keyword_spelling: None,
                operator_spelling: Some("xor"),
            },
            TokenKind::Arrow => TokenKindInfo {
                kind: self,
                display_name: "'->'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("->"),
                keyword_spelling: None,
                operator_spelling: Some("->"),
            },
            TokenKind::FatArrow => TokenKindInfo {
                kind: self,
                display_name: "'=>'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("=>"),
                keyword_spelling: None,
                operator_spelling: Some("=>"),
            },
            TokenKind::Dot => TokenKindInfo {
                kind: self,
                display_name: "'.'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("."),
                keyword_spelling: None,
                operator_spelling: Some("."),
            },
            TokenKind::Range => TokenKindInfo {
                kind: self,
                display_name: "'..'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some(".."),
                keyword_spelling: None,
                operator_spelling: Some(".."),
            },
            TokenKind::Ellipsis => TokenKindInfo {
                kind: self,
                display_name: "'...'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("..."),
                keyword_spelling: None,
                operator_spelling: Some("..."),
            },
            TokenKind::Increment => TokenKindInfo {
                kind: self,
                display_name: "'++'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("++"),
                keyword_spelling: None,
                operator_spelling: Some("++"),
            },
            TokenKind::Decrement => TokenKindInfo {
                kind: self,
                display_name: "'--'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("--"),
                keyword_spelling: None,
                operator_spelling: Some("--"),
            },
            TokenKind::DoubleColon => TokenKindInfo {
                kind: self,
                display_name: "'::'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("::"),
                keyword_spelling: None,
                operator_spelling: Some("::"),
            },
            TokenKind::Question => TokenKindInfo {
                kind: self,
                display_name: "'?'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("?"),
                keyword_spelling: None,
                operator_spelling: Some("?"),
            },
            TokenKind::Colon => TokenKindInfo {
                kind: self,
                display_name: "':'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some(":"),
                keyword_spelling: None,
                operator_spelling: Some(":"),
            },
            TokenKind::Backslash => TokenKindInfo {
                kind: self,
                display_name: "'\\\\'",
                category: TokenCategory::Operator,
                canonical_lexeme: Some("\\"),
                keyword_spelling: None,
                operator_spelling: Some("\\"),
            },
            TokenKind::LeftParen => TokenKindInfo {
                kind: self,
                display_name: "'('",
                category: TokenCategory::Delimiter,
                canonical_lexeme: Some("("),
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::RightParen => TokenKindInfo {
                kind: self,
                display_name: "')'",
                category: TokenCategory::Delimiter,
                canonical_lexeme: Some(")"),
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::LeftBrace => TokenKindInfo {
                kind: self,
                display_name: "'{'",
                category: TokenCategory::Delimiter,
                canonical_lexeme: Some("{"),
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::RightBrace => TokenKindInfo {
                kind: self,
                display_name: "'}'",
                category: TokenCategory::Delimiter,
                canonical_lexeme: Some("}"),
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::LeftBracket => TokenKindInfo {
                kind: self,
                display_name: "'['",
                category: TokenCategory::Delimiter,
                canonical_lexeme: Some("["),
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::RightBracket => TokenKindInfo {
                kind: self,
                display_name: "']'",
                category: TokenCategory::Delimiter,
                canonical_lexeme: Some("]"),
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::Semicolon => TokenKindInfo {
                kind: self,
                display_name: "';'",
                category: TokenCategory::Delimiter,
                canonical_lexeme: Some(";"),
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::Comma => TokenKindInfo {
                kind: self,
                display_name: "','",
                category: TokenCategory::Delimiter,
                canonical_lexeme: Some(","),
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::Number => TokenKindInfo {
                kind: self,
                display_name: "number",
                category: TokenCategory::Literal,
                canonical_lexeme: None,
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::String => TokenKindInfo {
                kind: self,
                display_name: "string",
                category: TokenCategory::Literal,
                canonical_lexeme: None,
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::Regex => TokenKindInfo {
                kind: self,
                display_name: "regex",
                category: TokenCategory::Literal,
                canonical_lexeme: None,
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::Substitution => TokenKindInfo {
                kind: self,
                display_name: "substitution (s///)",
                category: TokenCategory::Literal,
                canonical_lexeme: None,
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::Transliteration => TokenKindInfo {
                kind: self,
                display_name: "transliteration (tr///)",
                category: TokenCategory::Literal,
                canonical_lexeme: None,
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::QuoteSingle => TokenKindInfo {
                kind: self,
                display_name: "q// string",
                category: TokenCategory::Literal,
                canonical_lexeme: Some("q"),
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::QuoteDouble => TokenKindInfo {
                kind: self,
                display_name: "qq// string",
                category: TokenCategory::Literal,
                canonical_lexeme: Some("qq"),
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::QuoteWords => TokenKindInfo {
                kind: self,
                display_name: "qw() word list",
                category: TokenCategory::Literal,
                canonical_lexeme: Some("qw"),
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::QuoteCommand => TokenKindInfo {
                kind: self,
                display_name: "qx// command",
                category: TokenCategory::Literal,
                canonical_lexeme: Some("qx"),
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::HeredocStart => TokenKindInfo {
                kind: self,
                display_name: "heredoc (<<)",
                category: TokenCategory::Literal,
                canonical_lexeme: Some("<<"),
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::HeredocBody => TokenKindInfo {
                kind: self,
                display_name: "heredoc body",
                category: TokenCategory::Literal,
                canonical_lexeme: None,
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::FormatBody => TokenKindInfo {
                kind: self,
                display_name: "format body",
                category: TokenCategory::Literal,
                canonical_lexeme: None,
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::DataMarker => TokenKindInfo {
                kind: self,
                display_name: "__DATA__",
                category: TokenCategory::Literal,
                canonical_lexeme: Some("__DATA__"),
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::DataBody => TokenKindInfo {
                kind: self,
                display_name: "data section",
                category: TokenCategory::Literal,
                canonical_lexeme: None,
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::VString => TokenKindInfo {
                kind: self,
                display_name: "version string",
                category: TokenCategory::Literal,
                canonical_lexeme: None,
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::UnknownRest => TokenKindInfo {
                kind: self,
                display_name: "unparsed content",
                category: TokenCategory::Literal,
                canonical_lexeme: None,
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::HeredocDepthLimit => TokenKindInfo {
                kind: self,
                display_name: "heredoc depth limit",
                category: TokenCategory::Literal,
                canonical_lexeme: None,
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::Identifier => TokenKindInfo {
                kind: self,
                display_name: "identifier",
                category: TokenCategory::Identifier,
                canonical_lexeme: None,
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::ScalarSigil => TokenKindInfo {
                kind: self,
                display_name: "'$'",
                category: TokenCategory::Sigil,
                canonical_lexeme: Some("$"),
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::ArraySigil => TokenKindInfo {
                kind: self,
                display_name: "'@'",
                category: TokenCategory::Sigil,
                canonical_lexeme: Some("@"),
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::HashSigil => TokenKindInfo {
                kind: self,
                display_name: "'%'",
                category: TokenCategory::Sigil,
                canonical_lexeme: Some("%"),
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::SubSigil => TokenKindInfo {
                kind: self,
                display_name: "'&'",
                category: TokenCategory::Sigil,
                canonical_lexeme: Some("&"),
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::GlobSigil => TokenKindInfo {
                kind: self,
                display_name: "'*'",
                category: TokenCategory::Sigil,
                canonical_lexeme: Some("*"),
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::Eof => TokenKindInfo {
                kind: self,
                display_name: "end of input",
                category: TokenCategory::Special,
                canonical_lexeme: None,
                keyword_spelling: None,
                operator_spelling: None,
            },
            TokenKind::Unknown => TokenKindInfo {
                kind: self,
                display_name: "unknown token",
                category: TokenCategory::Special,
                canonical_lexeme: None,
                keyword_spelling: None,
                operator_spelling: None,
            },
        }
    }

    pub const fn display_name(self) -> &'static str {
        self.info().display_name
    }

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

    pub const fn is_identifier_like(self) -> bool {
        matches!(self.category(), TokenCategory::Identifier | TokenCategory::Sigil)
    }

    pub const fn is_sigil(self) -> bool {
        matches!(self.category(), TokenCategory::Sigil)
    }

    pub const fn is_special(self) -> bool {
        matches!(self.category(), TokenCategory::Special)
    }

    pub const fn canonical_lexeme(self) -> Option<&'static str> {
        self.info().canonical_lexeme
    }
}
