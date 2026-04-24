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

/// Broad classification used for token metadata and conformance checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenCategory {
    /// Reserved words and language keywords.
    Keyword,
    /// Operators and symbolic/word forms.
    Operator,
    /// Grouping and punctuation delimiters.
    Delimiter,
    /// Literal-like lexical forms.
    Literal,
    /// Identifiers and sigils.
    Identifier,
    /// Special sentinel and recovery tokens.
    Special,
}

/// Metadata associated with each [`TokenKind`] variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenKindMetadata {
    /// Stable category used in docs/tests/gates.
    pub category: TokenCategory,
    /// User-facing display label for diagnostics.
    pub display_name: &'static str,
}

impl TokenKind {
    /// Return every [`TokenKind`] variant in stable declaration order.
    pub const fn all() -> &'static [TokenKind] {
        &TOKEN_KIND_ALL
    }

    /// Number of token kinds expected to have metadata coverage.
    pub const fn metadata_count() -> usize {
        TOKEN_KIND_ALL.len()
    }

    /// Return compact metadata for this token kind.
    pub fn metadata(self) -> TokenKindMetadata {
        TokenKindMetadata { category: self.category(), display_name: self.display_name() }
    }

    /// Return the high-level category for this token kind.
    pub const fn category(self) -> TokenCategory {
        match self {
            TokenKind::My
            | TokenKind::Our
            | TokenKind::Local
            | TokenKind::State
            | TokenKind::Sub
            | TokenKind::If
            | TokenKind::Elsif
            | TokenKind::Else
            | TokenKind::Unless
            | TokenKind::While
            | TokenKind::Until
            | TokenKind::For
            | TokenKind::Foreach
            | TokenKind::Return
            | TokenKind::Package
            | TokenKind::Use
            | TokenKind::No
            | TokenKind::Begin
            | TokenKind::End
            | TokenKind::Check
            | TokenKind::Init
            | TokenKind::Unitcheck
            | TokenKind::Eval
            | TokenKind::Do
            | TokenKind::Given
            | TokenKind::When
            | TokenKind::Default
            | TokenKind::Try
            | TokenKind::Catch
            | TokenKind::Finally
            | TokenKind::Continue
            | TokenKind::Next
            | TokenKind::Last
            | TokenKind::Redo
            | TokenKind::Goto
            | TokenKind::Class
            | TokenKind::Method
            | TokenKind::Field
            | TokenKind::Format
            | TokenKind::Undef
            | TokenKind::Defer => TokenCategory::Keyword,
            TokenKind::Assign
            | TokenKind::Plus
            | TokenKind::Minus
            | TokenKind::Star
            | TokenKind::Slash
            | TokenKind::Percent
            | TokenKind::Power
            | TokenKind::LeftShift
            | TokenKind::RightShift
            | TokenKind::BitwiseAnd
            | TokenKind::BitwiseOr
            | TokenKind::BitwiseXor
            | TokenKind::BitwiseNot
            | TokenKind::PlusAssign
            | TokenKind::MinusAssign
            | TokenKind::StarAssign
            | TokenKind::SlashAssign
            | TokenKind::PercentAssign
            | TokenKind::DotAssign
            | TokenKind::AndAssign
            | TokenKind::OrAssign
            | TokenKind::XorAssign
            | TokenKind::PowerAssign
            | TokenKind::LeftShiftAssign
            | TokenKind::RightShiftAssign
            | TokenKind::LogicalAndAssign
            | TokenKind::LogicalOrAssign
            | TokenKind::DefinedOrAssign
            | TokenKind::Equal
            | TokenKind::NotEqual
            | TokenKind::Match
            | TokenKind::NotMatch
            | TokenKind::SmartMatch
            | TokenKind::Less
            | TokenKind::Greater
            | TokenKind::LessEqual
            | TokenKind::GreaterEqual
            | TokenKind::Spaceship
            | TokenKind::StringCompare
            | TokenKind::And
            | TokenKind::Or
            | TokenKind::Not
            | TokenKind::DefinedOr
            | TokenKind::WordAnd
            | TokenKind::WordOr
            | TokenKind::WordNot
            | TokenKind::WordXor
            | TokenKind::Arrow
            | TokenKind::FatArrow
            | TokenKind::Dot
            | TokenKind::Range
            | TokenKind::Ellipsis
            | TokenKind::Increment
            | TokenKind::Decrement
            | TokenKind::DoubleColon
            | TokenKind::Question
            | TokenKind::Colon
            | TokenKind::Backslash => TokenCategory::Operator,
            TokenKind::LeftParen
            | TokenKind::RightParen
            | TokenKind::LeftBrace
            | TokenKind::RightBrace
            | TokenKind::LeftBracket
            | TokenKind::RightBracket
            | TokenKind::Semicolon
            | TokenKind::Comma => TokenCategory::Delimiter,
            TokenKind::Number
            | TokenKind::String
            | TokenKind::Regex
            | TokenKind::Substitution
            | TokenKind::Transliteration
            | TokenKind::QuoteSingle
            | TokenKind::QuoteDouble
            | TokenKind::QuoteWords
            | TokenKind::QuoteCommand
            | TokenKind::HeredocStart
            | TokenKind::HeredocBody
            | TokenKind::FormatBody
            | TokenKind::DataMarker
            | TokenKind::DataBody
            | TokenKind::VString
            | TokenKind::UnknownRest
            | TokenKind::HeredocDepthLimit => TokenCategory::Literal,
            TokenKind::Identifier
            | TokenKind::ScalarSigil
            | TokenKind::ArraySigil
            | TokenKind::HashSigil
            | TokenKind::SubSigil
            | TokenKind::GlobSigil => TokenCategory::Identifier,
            TokenKind::Eof | TokenKind::Unknown => TokenCategory::Special,
        }
    }

    /// Return a user-friendly display name for this token kind.
    ///
    /// These names appear in parser error messages shown in the editor.
    /// They use the actual Perl syntax (e.g. `}` instead of `RightBrace`)
    /// so users can immediately understand what the parser expected.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_token::TokenKind;
    ///
    /// assert_eq!(TokenKind::Semicolon.display_name(), "';'");
    /// assert_eq!(TokenKind::Sub.display_name(), "'sub'");
    /// assert_eq!(TokenKind::Number.display_name(), "number");
    /// ```
    pub fn display_name(self) -> &'static str {
        match self {
            // Keywords
            TokenKind::My => "'my'",
            TokenKind::Our => "'our'",
            TokenKind::Local => "'local'",
            TokenKind::State => "'state'",
            TokenKind::Sub => "'sub'",
            TokenKind::If => "'if'",
            TokenKind::Elsif => "'elsif'",
            TokenKind::Else => "'else'",
            TokenKind::Unless => "'unless'",
            TokenKind::While => "'while'",
            TokenKind::Until => "'until'",
            TokenKind::For => "'for'",
            TokenKind::Foreach => "'foreach'",
            TokenKind::Return => "'return'",
            TokenKind::Package => "'package'",
            TokenKind::Use => "'use'",
            TokenKind::No => "'no'",
            TokenKind::Begin => "'BEGIN'",
            TokenKind::End => "'END'",
            TokenKind::Check => "'CHECK'",
            TokenKind::Init => "'INIT'",
            TokenKind::Unitcheck => "'UNITCHECK'",
            TokenKind::Eval => "'eval'",
            TokenKind::Do => "'do'",
            TokenKind::Given => "'given'",
            TokenKind::When => "'when'",
            TokenKind::Default => "'default'",
            TokenKind::Try => "'try'",
            TokenKind::Catch => "'catch'",
            TokenKind::Finally => "'finally'",
            TokenKind::Continue => "'continue'",
            TokenKind::Next => "'next'",
            TokenKind::Last => "'last'",
            TokenKind::Redo => "'redo'",
            TokenKind::Goto => "'goto'",
            TokenKind::Class => "'class'",
            TokenKind::Method => "'method'",
            TokenKind::Field => "'field'",
            TokenKind::Format => "'format'",
            TokenKind::Undef => "'undef'",
            TokenKind::Defer => "'defer'",

            // Operators
            TokenKind::Assign => "'='",
            TokenKind::Plus => "'+'",
            TokenKind::Minus => "'-'",
            TokenKind::Star => "'*'",
            TokenKind::Slash => "'/'",
            TokenKind::Percent => "'%'",
            TokenKind::Power => "'**'",
            TokenKind::LeftShift => "'<<'",
            TokenKind::RightShift => "'>>'",
            TokenKind::BitwiseAnd => "'&'",
            TokenKind::BitwiseOr => "'|'",
            TokenKind::BitwiseXor => "'^'",
            TokenKind::BitwiseNot => "'~'",
            TokenKind::PlusAssign => "'+='",
            TokenKind::MinusAssign => "'-='",
            TokenKind::StarAssign => "'*='",
            TokenKind::SlashAssign => "'/='",
            TokenKind::PercentAssign => "'%='",
            TokenKind::DotAssign => "'.='",
            TokenKind::AndAssign => "'&='",
            TokenKind::OrAssign => "'|='",
            TokenKind::XorAssign => "'^='",
            TokenKind::PowerAssign => "'**='",
            TokenKind::LeftShiftAssign => "'<<='",
            TokenKind::RightShiftAssign => "'>>='",
            TokenKind::LogicalAndAssign => "'&&='",
            TokenKind::LogicalOrAssign => "'||='",
            TokenKind::DefinedOrAssign => "'//='",
            TokenKind::Equal => "'=='",
            TokenKind::NotEqual => "'!='",
            TokenKind::Match => "'=~'",
            TokenKind::NotMatch => "'!~'",
            TokenKind::SmartMatch => "'~~'",
            TokenKind::Less => "'<'",
            TokenKind::Greater => "'>'",
            TokenKind::LessEqual => "'<='",
            TokenKind::GreaterEqual => "'>='",
            TokenKind::Spaceship => "'<=>'",
            TokenKind::StringCompare => "'cmp'",
            TokenKind::And => "'&&'",
            TokenKind::Or => "'||'",
            TokenKind::Not => "'!'",
            TokenKind::DefinedOr => "'//'",
            TokenKind::WordAnd => "'and'",
            TokenKind::WordOr => "'or'",
            TokenKind::WordNot => "'not'",
            TokenKind::WordXor => "'xor'",
            TokenKind::Arrow => "'->'",
            TokenKind::FatArrow => "'=>'",
            TokenKind::Dot => "'.'",
            TokenKind::Range => "'..'",
            TokenKind::Ellipsis => "'...'",
            TokenKind::Increment => "'++'",
            TokenKind::Decrement => "'--'",
            TokenKind::DoubleColon => "'::'",
            TokenKind::Question => "'?'",
            TokenKind::Colon => "':'",
            TokenKind::Backslash => "'\\'",

            // Delimiters
            TokenKind::LeftParen => "'('",
            TokenKind::RightParen => "')'",
            TokenKind::LeftBrace => "'{'",
            TokenKind::RightBrace => "'}'",
            TokenKind::LeftBracket => "'['",
            TokenKind::RightBracket => "']'",
            TokenKind::Semicolon => "';'",
            TokenKind::Comma => "','",

            // Literals
            TokenKind::Number => "number",
            TokenKind::String => "string",
            TokenKind::Regex => "regex",
            TokenKind::Substitution => "substitution (s///)",
            TokenKind::Transliteration => "transliteration (tr///)",
            TokenKind::QuoteSingle => "q// string",
            TokenKind::QuoteDouble => "qq// string",
            TokenKind::QuoteWords => "qw() word list",
            TokenKind::QuoteCommand => "qx// command",
            TokenKind::HeredocStart => "heredoc (<<)",
            TokenKind::HeredocBody => "heredoc body",
            TokenKind::FormatBody => "format body",
            TokenKind::DataMarker => "__DATA__",
            TokenKind::DataBody => "data section",
            TokenKind::VString => "version string",
            TokenKind::UnknownRest => "unparsed content",
            TokenKind::HeredocDepthLimit => "heredoc depth limit",

            // Identifiers and variables
            TokenKind::Identifier => "identifier",
            TokenKind::ScalarSigil => "'$'",
            TokenKind::ArraySigil => "'@'",
            TokenKind::HashSigil => "'%'",
            TokenKind::SubSigil => "'&'",
            TokenKind::GlobSigil => "'*'",

            // Special
            TokenKind::Eof => "end of input",
            TokenKind::Unknown => "unknown token",
        }
    }
}

const TOKEN_KIND_ALL: [TokenKind; 132] = [
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
