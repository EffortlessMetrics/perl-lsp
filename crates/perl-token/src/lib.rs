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

use std::{ops::Range, sync::Arc};

/// Byte span carried by a [`Token`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenSpan {
    /// Starting byte position.
    pub start: usize,
    /// Ending byte position.
    pub end: usize,
}

impl TokenSpan {
    /// Create a span from raw byte positions.
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Create a span, returning an error when `end < start`.
    pub fn try_new(start: usize, end: usize) -> Result<Self, TokenSpanError> {
        if end < start {
            return Err(TokenSpanError::EndBeforeStart { start, end });
        }

        Ok(Self { start, end })
    }

    /// Span length in bytes.
    pub const fn len(self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Whether the span length is zero bytes.
    pub const fn is_empty(self) -> bool {
        self.len() == 0
    }

    /// Convert this span to a standard `Range`.
    pub const fn range(self) -> Range<usize> {
        self.start..self.end
    }
}

/// Error type for checked token/span constructors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenSpanError {
    /// End offset is before start offset.
    EndBeforeStart { start: usize, end: usize },
    /// Empty span is only valid for EOF or explicit synthetic tokens.
    EmptySpanNotAllowed { kind: TokenKind, at: usize },
}

impl std::fmt::Display for TokenSpanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EndBeforeStart { start, end } => {
                write!(f, "token span invariant violated: end ({end}) < start ({start})")
            }
            Self::EmptySpanNotAllowed { kind, at } => {
                write!(f, "empty span not allowed for token kind {kind:?} at byte {at}")
            }
        }
    }
}

impl std::error::Error for TokenSpanError {}

/// Borrowed view over token data for allocation-sensitive paths.
///
/// Unlike [`Token`], this type borrows source text and does not allocate.
/// Convert to [`Token`] explicitly with [`TokenRef::to_owned_token`] or `From`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenRef<'src> {
    /// Token classification for parser decision making
    pub kind: TokenKind,
    /// Borrowed source text slice
    pub text: &'src str,
    /// Starting byte position for error reporting and location tracking
    pub start: usize,
    /// Ending byte position for span calculation and navigation
    pub end: usize,
}

impl<'src> TokenRef<'src> {
    /// Create a borrowed token view with the given kind, source text, and byte span.
    pub fn new(kind: TokenKind, text: &'src str, start: usize, end: usize) -> Self {
        Self { kind, text, start, end }
    }

    /// Create a borrowed token view with checked span ordering.
    ///
    /// Unlike [`TokenRef::new`], this rejects spans where `end < start`.
    pub fn try_new(
        kind: TokenKind,
        text: &'src str,
        start: usize,
        end: usize,
    ) -> Result<Self, TokenSpanError> {
        let span = TokenSpan::try_new(start, end)?;
        Ok(Self { kind, text, start: span.start, end: span.end })
    }

    /// Create a borrowed token view while enforcing span invariants.
    ///
    /// Rules:
    /// - `start <= end`
    /// - zero-length spans are accepted for EOF and explicit synthetic unknown tokens
    pub fn new_checked(
        kind: TokenKind,
        text: &'src str,
        start: usize,
        end: usize,
    ) -> Result<Self, TokenSpanError> {
        let token = Self::try_new(kind, text, start, end)?;
        if token.is_empty() && !matches!(token.kind, TokenKind::Eof | TokenKind::Unknown) {
            return Err(TokenSpanError::EmptySpanNotAllowed { kind: token.kind, at: token.start });
        }

        Ok(token)
    }

    /// Return the token span length in bytes.
    pub fn len(self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Return whether the token span is empty.
    pub fn is_empty(self) -> bool {
        self.len() == 0
    }

    /// Return the token span as `(start, end)`.
    pub fn span(self) -> (usize, usize) {
        (self.start, self.end)
    }

    /// Return a human-readable display name for this token.
    pub fn display_name(self) -> &'static str {
        self.kind.display_name()
    }

    /// Convert this borrowed token view into an owned [`Token`].
    pub fn to_owned_token(self) -> Token {
        Token::new(self.kind, self.text, self.start, self.end)
    }
}

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

    /// Create a token with checked span ordering.
    ///
    /// Unlike [`Token::new`], this rejects spans where `end < start`.
    pub fn try_new(
        kind: TokenKind,
        text: impl Into<Arc<str>>,
        start: usize,
        end: usize,
    ) -> Result<Self, TokenSpanError> {
        let span = TokenSpan::try_new(start, end)?;
        Ok(Self { kind, text: text.into(), start: span.start, end: span.end })
    }

    /// Create a token while enforcing span invariants.
    ///
    /// Rules:
    /// - `start <= end`
    /// - zero-length spans are accepted for EOF and explicit synthetic unknown tokens
    pub fn new_checked(
        kind: TokenKind,
        text: impl Into<Arc<str>>,
        start: usize,
        end: usize,
    ) -> Result<Self, TokenSpanError> {
        let token = Self::try_new(kind, text, start, end)?;
        if token.is_empty() && !matches!(token.kind, TokenKind::Eof | TokenKind::Unknown) {
            return Err(TokenSpanError::EmptySpanNotAllowed { kind: token.kind, at: token.start });
        }

        Ok(token)
    }

    /// Create an EOF token at `pos`.
    pub fn eof_at(pos: usize) -> Self {
        Self::new(TokenKind::Eof, "", pos, pos)
    }

    /// Create an unknown (synthetic) token at `start..end`.
    pub fn unknown_at(text: impl Into<Arc<str>>, start: usize, end: usize) -> Self {
        let bounded_end = end.max(start);
        Self::new(TokenKind::Unknown, text, start, bounded_end)
    }

    /// Return this token's byte span.
    pub fn span(&self) -> TokenSpan {
        TokenSpan::new(self.start, self.end)
    }

    /// Return this token's byte span as `Range<usize>`.
    pub fn range(&self) -> Range<usize> {
        self.span().range()
    }

    /// Clone this token with a new checked span.
    pub fn with_span(&self, start: usize, end: usize) -> Result<Self, TokenSpanError> {
        Self::new_checked(self.kind, self.text.clone(), start, end)
    }

    /// Clone this token with a new token kind.
    pub fn with_kind(&self, kind: TokenKind) -> Self {
        Self::new(kind, self.text.clone(), self.start, self.end)
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

    /// Return a human-readable display name for this token.
    pub fn display_name(&self) -> &'static str {
        self.kind.display_name()
    }

    /// Return a borrowed token view over this token.
    pub fn as_ref_token(&self) -> TokenRef<'_> {
        TokenRef { kind: self.kind, text: self.text.as_ref(), start: self.start, end: self.end }
    }
}

impl From<TokenRef<'_>> for Token {
    fn from(value: TokenRef<'_>) -> Self {
        value.to_owned_token()
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

/// Canonical lexer keyword spellings and their parser-facing token kinds.
///
/// Word-form operators (`and`, `or`, `not`, `xor`, `cmp`) are included here
/// because the lexer emits them as keyword tokens before the parser maps them
/// to their operator roles.
pub const KEYWORD_SPELLINGS: &[(&str, TokenKind)] = &[
    ("my", TokenKind::My),
    ("our", TokenKind::Our),
    ("local", TokenKind::Local),
    ("state", TokenKind::State),
    ("sub", TokenKind::Sub),
    ("if", TokenKind::If),
    ("elsif", TokenKind::Elsif),
    ("else", TokenKind::Else),
    ("unless", TokenKind::Unless),
    ("while", TokenKind::While),
    ("until", TokenKind::Until),
    ("for", TokenKind::For),
    ("foreach", TokenKind::Foreach),
    ("return", TokenKind::Return),
    ("package", TokenKind::Package),
    ("use", TokenKind::Use),
    ("no", TokenKind::No),
    ("BEGIN", TokenKind::Begin),
    ("END", TokenKind::End),
    ("CHECK", TokenKind::Check),
    ("INIT", TokenKind::Init),
    ("UNITCHECK", TokenKind::Unitcheck),
    ("eval", TokenKind::Eval),
    ("do", TokenKind::Do),
    ("given", TokenKind::Given),
    ("when", TokenKind::When),
    ("default", TokenKind::Default),
    ("try", TokenKind::Try),
    ("catch", TokenKind::Catch),
    ("finally", TokenKind::Finally),
    ("continue", TokenKind::Continue),
    ("next", TokenKind::Next),
    ("last", TokenKind::Last),
    ("redo", TokenKind::Redo),
    ("goto", TokenKind::Goto),
    ("class", TokenKind::Class),
    ("method", TokenKind::Method),
    ("field", TokenKind::Field),
    ("format", TokenKind::Format),
    ("undef", TokenKind::Undef),
    ("defer", TokenKind::Defer),
    ("and", TokenKind::WordAnd),
    ("or", TokenKind::WordOr),
    ("not", TokenKind::WordNot),
    ("xor", TokenKind::WordXor),
    ("cmp", TokenKind::StringCompare),
];

/// Canonical symbolic operator spellings and their parser-facing token kinds.
pub const OPERATOR_SPELLINGS: &[(&str, TokenKind)] = &[
    ("=", TokenKind::Assign),
    ("+", TokenKind::Plus),
    ("-", TokenKind::Minus),
    ("*", TokenKind::Star),
    ("/", TokenKind::Slash),
    ("%", TokenKind::Percent),
    ("**", TokenKind::Power),
    ("<<", TokenKind::LeftShift),
    (">>", TokenKind::RightShift),
    ("&", TokenKind::BitwiseAnd),
    ("|", TokenKind::BitwiseOr),
    ("^", TokenKind::BitwiseXor),
    ("~", TokenKind::BitwiseNot),
    ("+=", TokenKind::PlusAssign),
    ("-=", TokenKind::MinusAssign),
    ("*=", TokenKind::StarAssign),
    ("/=", TokenKind::SlashAssign),
    ("%=", TokenKind::PercentAssign),
    (".=", TokenKind::DotAssign),
    ("&=", TokenKind::AndAssign),
    ("|=", TokenKind::OrAssign),
    ("^=", TokenKind::XorAssign),
    ("**=", TokenKind::PowerAssign),
    ("<<=", TokenKind::LeftShiftAssign),
    (">>=", TokenKind::RightShiftAssign),
    ("&&=", TokenKind::LogicalAndAssign),
    ("||=", TokenKind::LogicalOrAssign),
    ("//=", TokenKind::DefinedOrAssign),
    ("==", TokenKind::Equal),
    ("!=", TokenKind::NotEqual),
    ("=~", TokenKind::Match),
    ("!~", TokenKind::NotMatch),
    ("~~", TokenKind::SmartMatch),
    ("<", TokenKind::Less),
    (">", TokenKind::Greater),
    ("<=", TokenKind::LessEqual),
    (">=", TokenKind::GreaterEqual),
    ("<=>", TokenKind::Spaceship),
    ("&&", TokenKind::And),
    ("||", TokenKind::Or),
    ("!", TokenKind::Not),
    ("//", TokenKind::DefinedOr),
    ("->", TokenKind::Arrow),
    ("=>", TokenKind::FatArrow),
    (".", TokenKind::Dot),
    ("..", TokenKind::Range),
    ("...", TokenKind::Ellipsis),
    ("++", TokenKind::Increment),
    ("--", TokenKind::Decrement),
    ("::", TokenKind::DoubleColon),
    ("?", TokenKind::Question),
    (":", TokenKind::Colon),
    ("\\", TokenKind::Backslash),
];

/// Canonical delimiter spellings and their parser-facing token kinds.
pub const DELIMITER_SPELLINGS: &[(&str, TokenKind)] = &[
    ("(", TokenKind::LeftParen),
    (")", TokenKind::RightParen),
    ("{", TokenKind::LeftBrace),
    ("}", TokenKind::RightBrace),
    ("[", TokenKind::LeftBracket),
    ("]", TokenKind::RightBracket),
    (";", TokenKind::Semicolon),
    (",", TokenKind::Comma),
];

/// Canonical sigil spellings and their parser-facing token kinds.
pub const SIGIL_SPELLINGS: &[(&str, TokenKind)] = &[
    ("$", TokenKind::ScalarSigil),
    ("@", TokenKind::ArraySigil),
    ("%", TokenKind::HashSigil),
    ("&", TokenKind::SubSigil),
    ("*", TokenKind::GlobSigil),
];

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

    // --- Category-based predicates (classify by TokenCategory) ---

    /// Returns `true` if this token kind is a keyword.
    pub const fn is_keyword(self) -> bool {
        matches!(self.category(), TokenCategory::Keyword)
    }

    /// Returns `true` if this token kind is an operator.
    pub const fn is_operator(self) -> bool {
        matches!(self.category(), TokenCategory::Operator)
    }

    /// Returns `true` if this token kind is a literal.
    pub const fn is_literal(self) -> bool {
        matches!(self.category(), TokenCategory::Literal)
    }

    /// Returns `true` if this token kind is a delimiter.
    pub const fn is_delimiter(self) -> bool {
        matches!(self.category(), TokenCategory::Delimiter)
    }

    /// Returns `true` if this token kind is an identifier or sigil.
    pub const fn is_identifier(self) -> bool {
        matches!(self.category(), TokenCategory::Identifier)
    }

    /// Returns `true` if this token kind is a special sentinel/recovery token.
    pub const fn is_special(self) -> bool {
        matches!(self.category(), TokenCategory::Special)
    }

    // --- Parser-facing role predicates (specific semantic roles) ---

    /// Return whether this token is an assignment operator.
    #[inline]
    pub fn is_assignment_operator(self) -> bool {
        matches!(
            self,
            TokenKind::Assign
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
        )
    }

    /// Return whether this token is a comparison operator.
    #[inline]
    pub fn is_comparison_operator(self) -> bool {
        matches!(
            self,
            TokenKind::Equal
                | TokenKind::NotEqual
                | TokenKind::Less
                | TokenKind::Greater
                | TokenKind::LessEqual
                | TokenKind::GreaterEqual
                | TokenKind::Spaceship
                | TokenKind::StringCompare
                | TokenKind::Match
                | TokenKind::NotMatch
                | TokenKind::SmartMatch
        )
    }

    /// Return whether this token is a logical operator.
    #[inline]
    pub fn is_logical_operator(self) -> bool {
        matches!(
            self,
            TokenKind::And
                | TokenKind::Or
                | TokenKind::Not
                | TokenKind::DefinedOr
                | TokenKind::WordAnd
                | TokenKind::WordOr
                | TokenKind::WordNot
                | TokenKind::WordXor
        )
    }

    /// Return whether this token is a word-form operator token.
    #[inline]
    pub fn is_word_operator(self) -> bool {
        matches!(
            self,
            TokenKind::StringCompare
                | TokenKind::WordAnd
                | TokenKind::WordOr
                | TokenKind::WordNot
                | TokenKind::WordXor
        )
    }

    /// Return whether this token is a low-precedence word operator.
    #[inline]
    pub fn is_low_precedence_word_operator(self) -> bool {
        matches!(
            self,
            TokenKind::WordAnd | TokenKind::WordOr | TokenKind::WordNot | TokenKind::WordXor
        )
    }

    /// Return whether this token is an opening paired delimiter.
    #[inline]
    pub fn is_open_delimiter(self) -> bool {
        matches!(self, TokenKind::LeftParen | TokenKind::LeftBrace | TokenKind::LeftBracket)
    }

    /// Return whether this token is a closing paired delimiter.
    #[inline]
    pub fn is_close_delimiter(self) -> bool {
        matches!(self, TokenKind::RightParen | TokenKind::RightBrace | TokenKind::RightBracket)
    }

    /// Return the matching paired delimiter for this token, if any.
    #[inline]
    pub fn matching_delimiter(self) -> Option<Self> {
        match self {
            TokenKind::LeftParen => Some(TokenKind::RightParen),
            TokenKind::RightParen => Some(TokenKind::LeftParen),
            TokenKind::LeftBrace => Some(TokenKind::RightBrace),
            TokenKind::RightBrace => Some(TokenKind::LeftBrace),
            TokenKind::LeftBracket => Some(TokenKind::RightBracket),
            TokenKind::RightBracket => Some(TokenKind::LeftBracket),
            _ => None,
        }
    }

    /// Return whether this token is quote-like syntax.
    #[inline]
    pub fn is_quote_like(self) -> bool {
        matches!(
            self,
            TokenKind::Regex
                | TokenKind::Substitution
                | TokenKind::Transliteration
                | TokenKind::QuoteSingle
                | TokenKind::QuoteDouble
                | TokenKind::QuoteWords
                | TokenKind::QuoteCommand
                | TokenKind::HeredocStart
        )
    }

    /// Return whether this token is a hard recovery boundary.
    #[inline]
    pub fn is_recovery_boundary(self) -> bool {
        self == TokenKind::Semicolon || self.is_close_delimiter() || self == TokenKind::Eof
    }

    /// Return the canonical source spelling for fixed-spelling token kinds.
    ///
    /// This returns `None` for value-carrying tokens (such as identifiers,
    /// numbers, strings, regexes, heredocs, and recovery sentinels) because
    /// those spellings come from the original source text rather than a stable
    /// token-kind table.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_token::TokenKind;
    ///
    /// assert_eq!(TokenKind::Sub.canonical_spelling(), Some("sub"));
    /// assert_eq!(TokenKind::LeftBrace.canonical_spelling(), Some("{"));
    /// assert_eq!(TokenKind::Identifier.canonical_spelling(), None);
    /// ```
    pub fn canonical_spelling(self) -> Option<&'static str> {
        spelling_for_kind(self, KEYWORD_SPELLINGS)
            .or_else(|| spelling_for_kind(self, OPERATOR_SPELLINGS))
            .or_else(|| spelling_for_kind(self, DELIMITER_SPELLINGS))
            .or_else(|| spelling_for_kind(self, SIGIL_SPELLINGS))
    }

    /// Map a canonical keyword spelling to its [`TokenKind`].
    ///
    /// This mapping is case-sensitive and only recognizes canonical Perl
    /// spellings used by the lexer/parser pipeline.
    pub fn from_keyword(spelling: &str) -> Option<TokenKind> {
        match spelling {
            "my" => Some(TokenKind::My),
            "our" => Some(TokenKind::Our),
            "local" => Some(TokenKind::Local),
            "state" => Some(TokenKind::State),
            "sub" => Some(TokenKind::Sub),
            "if" => Some(TokenKind::If),
            "elsif" => Some(TokenKind::Elsif),
            "else" => Some(TokenKind::Else),
            "unless" => Some(TokenKind::Unless),
            "while" => Some(TokenKind::While),
            "until" => Some(TokenKind::Until),
            "for" => Some(TokenKind::For),
            "foreach" => Some(TokenKind::Foreach),
            "return" => Some(TokenKind::Return),
            "package" => Some(TokenKind::Package),
            "use" => Some(TokenKind::Use),
            "no" => Some(TokenKind::No),
            "BEGIN" => Some(TokenKind::Begin),
            "END" => Some(TokenKind::End),
            "CHECK" => Some(TokenKind::Check),
            "INIT" => Some(TokenKind::Init),
            "UNITCHECK" => Some(TokenKind::Unitcheck),
            "eval" => Some(TokenKind::Eval),
            "do" => Some(TokenKind::Do),
            "given" => Some(TokenKind::Given),
            "when" => Some(TokenKind::When),
            "default" => Some(TokenKind::Default),
            "try" => Some(TokenKind::Try),
            "catch" => Some(TokenKind::Catch),
            "finally" => Some(TokenKind::Finally),
            "continue" => Some(TokenKind::Continue),
            "next" => Some(TokenKind::Next),
            "last" => Some(TokenKind::Last),
            "redo" => Some(TokenKind::Redo),
            "goto" => Some(TokenKind::Goto),
            "class" => Some(TokenKind::Class),
            "method" => Some(TokenKind::Method),
            "field" => Some(TokenKind::Field),
            "format" => Some(TokenKind::Format),
            "undef" => Some(TokenKind::Undef),
            "defer" => Some(TokenKind::Defer),
            // Word operators are emitted as Keyword tokens by the lexer.
            "and" => Some(TokenKind::WordAnd),
            "or" => Some(TokenKind::WordOr),
            "not" => Some(TokenKind::WordNot),
            "xor" => Some(TokenKind::WordXor),
            "cmp" => Some(TokenKind::StringCompare),
            _ => None,
        }
    }

    /// Map a canonical operator spelling to its [`TokenKind`].
    ///
    /// This mapping is case-sensitive.
    pub fn from_operator(spelling: &str) -> Option<TokenKind> {
        match spelling {
            "=" => Some(TokenKind::Assign),
            "+" => Some(TokenKind::Plus),
            "-" => Some(TokenKind::Minus),
            "*" => Some(TokenKind::Star),
            "/" => Some(TokenKind::Slash),
            "%" => Some(TokenKind::Percent),
            "**" => Some(TokenKind::Power),
            "<<" => Some(TokenKind::LeftShift),
            ">>" => Some(TokenKind::RightShift),
            "&" => Some(TokenKind::BitwiseAnd),
            "|" => Some(TokenKind::BitwiseOr),
            "^" => Some(TokenKind::BitwiseXor),
            "~" => Some(TokenKind::BitwiseNot),
            "+=" => Some(TokenKind::PlusAssign),
            "-=" => Some(TokenKind::MinusAssign),
            "*=" => Some(TokenKind::StarAssign),
            "/=" => Some(TokenKind::SlashAssign),
            "%=" => Some(TokenKind::PercentAssign),
            ".=" => Some(TokenKind::DotAssign),
            "&=" => Some(TokenKind::AndAssign),
            "|=" => Some(TokenKind::OrAssign),
            "^=" => Some(TokenKind::XorAssign),
            "**=" => Some(TokenKind::PowerAssign),
            "<<=" => Some(TokenKind::LeftShiftAssign),
            ">>=" => Some(TokenKind::RightShiftAssign),
            "&&=" => Some(TokenKind::LogicalAndAssign),
            "||=" => Some(TokenKind::LogicalOrAssign),
            "//=" => Some(TokenKind::DefinedOrAssign),
            "==" => Some(TokenKind::Equal),
            "!=" => Some(TokenKind::NotEqual),
            "=~" => Some(TokenKind::Match),
            "!~" => Some(TokenKind::NotMatch),
            "~~" => Some(TokenKind::SmartMatch),
            "<" => Some(TokenKind::Less),
            ">" => Some(TokenKind::Greater),
            "<=" => Some(TokenKind::LessEqual),
            ">=" => Some(TokenKind::GreaterEqual),
            "<=>" => Some(TokenKind::Spaceship),
            "&&" => Some(TokenKind::And),
            "||" => Some(TokenKind::Or),
            "!" => Some(TokenKind::Not),
            "//" => Some(TokenKind::DefinedOr),
            "->" => Some(TokenKind::Arrow),
            "=>" => Some(TokenKind::FatArrow),
            "." => Some(TokenKind::Dot),
            ".." => Some(TokenKind::Range),
            "..." => Some(TokenKind::Ellipsis),
            "++" => Some(TokenKind::Increment),
            "--" => Some(TokenKind::Decrement),
            "::" => Some(TokenKind::DoubleColon),
            "?" => Some(TokenKind::Question),
            ":" => Some(TokenKind::Colon),
            "\\" => Some(TokenKind::Backslash),
            _ => None,
        }
    }

    /// Map a delimiter spelling to its [`TokenKind`].
    pub fn from_delimiter(spelling: &str) -> Option<TokenKind> {
        match spelling {
            "(" => Some(TokenKind::LeftParen),
            ")" => Some(TokenKind::RightParen),
            "{" => Some(TokenKind::LeftBrace),
            "}" => Some(TokenKind::RightBrace),
            "[" => Some(TokenKind::LeftBracket),
            "]" => Some(TokenKind::RightBracket),
            ";" => Some(TokenKind::Semicolon),
            "," => Some(TokenKind::Comma),
            _ => None,
        }
    }

    /// Map a sigil spelling to its [`TokenKind`].
    pub fn from_sigil(spelling: &str) -> Option<TokenKind> {
        match spelling {
            "$" => Some(TokenKind::ScalarSigil),
            "@" => Some(TokenKind::ArraySigil),
            "%" => Some(TokenKind::HashSigil),
            "&" => Some(TokenKind::SubSigil),
            "*" => Some(TokenKind::GlobSigil),
            _ => None,
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
            TokenKind::DataMarker => "data marker (__DATA__ or __END__)",
            TokenKind::DataBody => "data section body",
            TokenKind::VString => "version string",
            TokenKind::UnknownRest => "unparsed remainder",
            TokenKind::HeredocDepthLimit => "heredoc depth limit exceeded",

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

fn spelling_for_kind(
    kind: TokenKind,
    spellings: &'static [(&'static str, TokenKind)],
) -> Option<&'static str> {
    spellings.iter().find_map(|&(spelling, candidate)| (candidate == kind).then_some(spelling))
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

#[cfg(test)]
mod tests {
    use super::*;

    // --- TokenSpan ---

    #[test]
    fn token_span_new_and_accessors() {
        let span = TokenSpan::new(5, 10);
        assert_eq!(span.start, 5);
        assert_eq!(span.end, 10);
        assert_eq!(span.len(), 5);
        assert!(!span.is_empty());
        assert_eq!(span.range(), 5..10);
    }

    #[test]
    fn token_span_is_empty_when_zero_length() {
        let span = TokenSpan::new(3, 3);
        assert!(span.is_empty());
        assert_eq!(span.len(), 0);
    }

    #[test]
    fn token_span_try_new_ok() -> Result<(), TokenSpanError> {
        let span = TokenSpan::try_new(0, 5)?;
        assert_eq!(span.start, 0);
        assert_eq!(span.end, 5);
        Ok(())
    }

    #[test]
    fn token_span_try_new_end_before_start_errors() {
        assert_eq!(
            TokenSpan::try_new(10, 5),
            Err(TokenSpanError::EndBeforeStart { start: 10, end: 5 })
        );
    }

    #[test]
    fn token_span_error_display_end_before_start() {
        let err = TokenSpanError::EndBeforeStart { start: 10, end: 5 };
        let msg = err.to_string();
        assert!(msg.contains("10"));
        assert!(msg.contains("5"));
    }

    #[test]
    fn token_span_error_display_empty_span_not_allowed() {
        let err = TokenSpanError::EmptySpanNotAllowed { kind: TokenKind::Identifier, at: 7 };
        let msg = err.to_string();
        assert!(msg.contains("Identifier"));
        assert!(msg.contains("7"));
    }

    // --- Token ---

    #[test]
    fn token_new_stores_fields() {
        let tok = Token::new(TokenKind::My, "my", 0, 2);
        assert_eq!(tok.kind, TokenKind::My);
        assert_eq!(&*tok.text, "my");
        assert_eq!(tok.start, 0);
        assert_eq!(tok.end, 2);
    }

    #[test]
    fn token_len_and_is_empty() {
        let tok = Token::new(TokenKind::Identifier, "foo", 10, 13);
        assert_eq!(tok.len(), 3);
        assert!(!tok.is_empty());

        let eof = Token::eof_at(8);
        assert_eq!(eof.len(), 0);
        assert!(eof.is_empty());
    }

    #[test]
    fn token_span_and_range() {
        let tok = Token::new(TokenKind::Number, "42", 5, 7);
        assert_eq!(tok.span(), TokenSpan::new(5, 7));
        assert_eq!(tok.range(), 5..7);
    }

    #[test]
    fn token_try_new_allows_ordered_spans() -> Result<(), TokenSpanError> {
        let tok = Token::try_new(TokenKind::Identifier, "name", 4, 8)?;
        assert_eq!(tok.kind, TokenKind::Identifier);
        assert_eq!(&*tok.text, "name");
        assert_eq!(tok.span(), TokenSpan::new(4, 8));
        Ok(())
    }

    #[test]
    fn token_try_new_rejects_end_before_start() {
        assert_eq!(
            Token::try_new(TokenKind::Identifier, "x", 10, 5),
            Err(TokenSpanError::EndBeforeStart { start: 10, end: 5 })
        );
    }

    #[test]
    fn token_new_checked_rejects_empty_non_eof() {
        assert_eq!(
            Token::new_checked(TokenKind::Identifier, "", 5, 5),
            Err(TokenSpanError::EmptySpanNotAllowed { kind: TokenKind::Identifier, at: 5 })
        );
    }

    #[test]
    fn token_new_checked_allows_empty_eof() -> Result<(), TokenSpanError> {
        let tok = Token::new_checked(TokenKind::Eof, "", 5, 5)?;
        assert_eq!(tok.kind, TokenKind::Eof);
        assert_eq!(tok.start, 5);
        Ok(())
    }

    #[test]
    fn token_new_checked_allows_empty_unknown() -> Result<(), TokenSpanError> {
        let tok = Token::new_checked(TokenKind::Unknown, "", 6, 6)?;
        assert_eq!(tok.kind, TokenKind::Unknown);
        assert_eq!(tok.start, 6);
        assert!(tok.is_empty());
        Ok(())
    }

    #[test]
    fn token_eof_at() {
        let eof = Token::eof_at(42);
        assert_eq!(eof.kind, TokenKind::Eof);
        assert_eq!(eof.start, 42);
        assert_eq!(eof.end, 42);
        assert!(eof.is_empty());
    }

    #[test]
    fn token_unknown_at_normalises_inverted_span() {
        let tok = Token::unknown_at("?", 5, 3); // end < start
        assert_eq!(tok.kind, TokenKind::Unknown);
        assert_eq!(tok.start, 5);
        assert_eq!(tok.end, 5); // bounded to start
    }

    #[test]
    fn token_with_kind() {
        let tok = Token::new(TokenKind::Identifier, "sub", 0, 3);
        let retyped = tok.with_kind(TokenKind::Sub);
        assert_eq!(retyped.kind, TokenKind::Sub);
        assert_eq!(&*retyped.text, "sub");
        assert_eq!(retyped.start, 0);
        assert_eq!(retyped.end, 3);
    }

    #[test]
    fn token_with_span_ok() -> Result<(), TokenSpanError> {
        let tok = Token::new(TokenKind::String, "hello", 0, 5);
        let moved = tok.with_span(10, 15)?;
        assert_eq!(moved.start, 10);
        assert_eq!(moved.end, 15);
        Ok(())
    }

    #[test]
    fn token_with_span_rejects_empty_non_eof() {
        let tok = Token::new(TokenKind::String, "hello", 0, 5);
        assert_eq!(
            tok.with_span(10, 10),
            Err(TokenSpanError::EmptySpanNotAllowed { kind: TokenKind::String, at: 10 })
        );
    }

    #[test]
    fn token_display_name_delegates_to_kind() {
        let tok = Token::new(TokenKind::LeftBrace, "{", 0, 1);
        assert_eq!(tok.display_name(), "'{'");
    }

    #[test]
    fn token_as_ref_token_round_trip() {
        let tok = Token::new(TokenKind::Sub, "sub", 0, 3);
        let tok_ref = tok.as_ref_token();
        assert_eq!(tok_ref.kind, TokenKind::Sub);
        assert_eq!(tok_ref.text, "sub");
        assert_eq!(tok_ref.start, 0);
        assert_eq!(tok_ref.end, 3);

        let owned: Token = tok_ref.into();
        assert_eq!(owned.kind, TokenKind::Sub);
        assert_eq!(&*owned.text, "sub");
    }

    // --- TokenRef ---

    #[test]
    fn token_ref_accessors() {
        let r = TokenRef::new(TokenKind::Number, "99", 4, 6);
        assert_eq!(r.len(), 2);
        assert!(!r.is_empty());
        assert_eq!(r.span(), (4, 6));
        assert_eq!(r.display_name(), "number");
    }

    #[test]
    fn token_ref_try_new_allows_ordered_spans() -> Result<(), TokenSpanError> {
        let r = TokenRef::try_new(TokenKind::Number, "99", 4, 6)?;
        assert_eq!(r.kind, TokenKind::Number);
        assert_eq!(r.text, "99");
        assert_eq!(r.span(), (4, 6));
        Ok(())
    }

    #[test]
    fn token_ref_to_owned_token() {
        let r = TokenRef::new(TokenKind::Identifier, "foo", 1, 4);
        let owned = r.to_owned_token();
        assert_eq!(owned.kind, TokenKind::Identifier);
        assert_eq!(&*owned.text, "foo");
    }

    // --- TokenKind::from_keyword ---

    #[test]
    fn from_keyword_recognises_perl_keywords() {
        assert_eq!(TokenKind::from_keyword("my"), Some(TokenKind::My));
        assert_eq!(TokenKind::from_keyword("sub"), Some(TokenKind::Sub));
        assert_eq!(TokenKind::from_keyword("if"), Some(TokenKind::If));
        assert_eq!(TokenKind::from_keyword("elsif"), Some(TokenKind::Elsif));
        assert_eq!(TokenKind::from_keyword("else"), Some(TokenKind::Else));
        assert_eq!(TokenKind::from_keyword("while"), Some(TokenKind::While));
        assert_eq!(TokenKind::from_keyword("for"), Some(TokenKind::For));
        assert_eq!(TokenKind::from_keyword("foreach"), Some(TokenKind::Foreach));
        assert_eq!(TokenKind::from_keyword("return"), Some(TokenKind::Return));
        assert_eq!(TokenKind::from_keyword("package"), Some(TokenKind::Package));
        assert_eq!(TokenKind::from_keyword("use"), Some(TokenKind::Use));
        assert_eq!(TokenKind::from_keyword("BEGIN"), Some(TokenKind::Begin));
        assert_eq!(TokenKind::from_keyword("END"), Some(TokenKind::End));
        assert_eq!(TokenKind::from_keyword("eval"), Some(TokenKind::Eval));
        assert_eq!(TokenKind::from_keyword("class"), Some(TokenKind::Class));
        assert_eq!(TokenKind::from_keyword("defer"), Some(TokenKind::Defer));
        assert_eq!(TokenKind::from_keyword("and"), Some(TokenKind::WordAnd));
        assert_eq!(TokenKind::from_keyword("or"), Some(TokenKind::WordOr));
        assert_eq!(TokenKind::from_keyword("not"), Some(TokenKind::WordNot));
        assert_eq!(TokenKind::from_keyword("xor"), Some(TokenKind::WordXor));
        assert_eq!(TokenKind::from_keyword("cmp"), Some(TokenKind::StringCompare));
    }

    #[test]
    fn from_keyword_unknown_returns_none() {
        assert_eq!(TokenKind::from_keyword("MY"), None);
        assert_eq!(TokenKind::from_keyword("Sub"), None);
        assert_eq!(TokenKind::from_keyword("unknown"), None);
        assert_eq!(TokenKind::from_keyword(""), None);
    }

    // --- TokenKind::from_operator ---

    #[test]
    fn from_operator_recognises_operators() {
        assert_eq!(TokenKind::from_operator("="), Some(TokenKind::Assign));
        assert_eq!(TokenKind::from_operator("+"), Some(TokenKind::Plus));
        assert_eq!(TokenKind::from_operator("**"), Some(TokenKind::Power));
        assert_eq!(TokenKind::from_operator("->"), Some(TokenKind::Arrow));
        assert_eq!(TokenKind::from_operator("=>"), Some(TokenKind::FatArrow));
        assert_eq!(TokenKind::from_operator("<=>"), Some(TokenKind::Spaceship));
        assert_eq!(TokenKind::from_operator("//="), Some(TokenKind::DefinedOrAssign));
        assert_eq!(TokenKind::from_operator("..."), Some(TokenKind::Ellipsis));
        assert_eq!(TokenKind::from_operator("~~"), Some(TokenKind::SmartMatch));
    }

    #[test]
    fn from_operator_unknown_returns_none() {
        assert_eq!(TokenKind::from_operator(""), None);
        assert_eq!(TokenKind::from_operator("xyz"), None);
    }

    // --- TokenKind::from_delimiter ---

    #[test]
    fn from_delimiter_recognises_all() {
        assert_eq!(TokenKind::from_delimiter("("), Some(TokenKind::LeftParen));
        assert_eq!(TokenKind::from_delimiter(")"), Some(TokenKind::RightParen));
        assert_eq!(TokenKind::from_delimiter("{"), Some(TokenKind::LeftBrace));
        assert_eq!(TokenKind::from_delimiter("}"), Some(TokenKind::RightBrace));
        assert_eq!(TokenKind::from_delimiter("["), Some(TokenKind::LeftBracket));
        assert_eq!(TokenKind::from_delimiter("]"), Some(TokenKind::RightBracket));
        assert_eq!(TokenKind::from_delimiter(";"), Some(TokenKind::Semicolon));
        assert_eq!(TokenKind::from_delimiter(","), Some(TokenKind::Comma));
        assert_eq!(TokenKind::from_delimiter("x"), None);
    }

    // --- TokenKind::from_sigil ---

    #[test]
    fn from_sigil_recognises_all() {
        assert_eq!(TokenKind::from_sigil("$"), Some(TokenKind::ScalarSigil));
        assert_eq!(TokenKind::from_sigil("@"), Some(TokenKind::ArraySigil));
        assert_eq!(TokenKind::from_sigil("%"), Some(TokenKind::HashSigil));
        assert_eq!(TokenKind::from_sigil("&"), Some(TokenKind::SubSigil));
        assert_eq!(TokenKind::from_sigil("*"), Some(TokenKind::GlobSigil));
        assert_eq!(TokenKind::from_sigil("!"), None);
    }

    // --- TokenKind::category ---

    #[test]
    fn category_keyword_variants() {
        assert_eq!(TokenKind::My.category(), TokenCategory::Keyword);
        assert_eq!(TokenKind::Sub.category(), TokenCategory::Keyword);
        assert_eq!(TokenKind::Defer.category(), TokenCategory::Keyword);
    }

    #[test]
    fn category_operator_variants() {
        assert_eq!(TokenKind::Plus.category(), TokenCategory::Operator);
        assert_eq!(TokenKind::Spaceship.category(), TokenCategory::Operator);
        assert_eq!(TokenKind::WordAnd.category(), TokenCategory::Operator);
    }

    #[test]
    fn category_delimiter_variants() {
        assert_eq!(TokenKind::LeftParen.category(), TokenCategory::Delimiter);
        assert_eq!(TokenKind::Comma.category(), TokenCategory::Delimiter);
    }

    #[test]
    fn category_literal_variants() {
        assert_eq!(TokenKind::Number.category(), TokenCategory::Literal);
        assert_eq!(TokenKind::HeredocStart.category(), TokenCategory::Literal);
        assert_eq!(TokenKind::DataMarker.category(), TokenCategory::Literal);
    }

    #[test]
    fn category_identifier_variants() {
        assert_eq!(TokenKind::Identifier.category(), TokenCategory::Identifier);
        assert_eq!(TokenKind::ScalarSigil.category(), TokenCategory::Identifier);
        assert_eq!(TokenKind::GlobSigil.category(), TokenCategory::Identifier);
    }

    #[test]
    fn category_special_variants() {
        assert_eq!(TokenKind::Eof.category(), TokenCategory::Special);
        assert_eq!(TokenKind::Unknown.category(), TokenCategory::Special);
    }

    // --- TokenKind::display_name ---

    #[test]
    fn display_name_selected_variants() {
        assert_eq!(TokenKind::LeftBrace.display_name(), "'{'");
        assert_eq!(TokenKind::RightBrace.display_name(), "'}'");
        assert_eq!(TokenKind::Identifier.display_name(), "identifier");
        assert_eq!(TokenKind::Eof.display_name(), "end of input");
        assert_eq!(TokenKind::Number.display_name(), "number");
        assert_eq!(TokenKind::Sub.display_name(), "'sub'");
        assert_eq!(TokenKind::Semicolon.display_name(), "';'");
        assert_eq!(TokenKind::HeredocStart.display_name(), "heredoc (<<)");
        assert_eq!(TokenKind::DataMarker.display_name(), "data marker (__DATA__ or __END__)");
    }

    // --- TokenKind::all / metadata_count ---

    #[test]
    fn all_returns_132_variants() {
        assert_eq!(TokenKind::all().len(), 132);
        assert_eq!(TokenKind::metadata_count(), 132);
    }

    #[test]
    fn metadata_round_trips_through_kind() {
        let m = TokenKind::Sub.metadata();
        assert_eq!(m.category, TokenCategory::Keyword);
        assert_eq!(m.display_name, "'sub'");
    }

    // --- TokenKind role predicates ---

    #[test]
    fn is_assignment_operator_returns_true_for_assign_variants() {
        assert!(TokenKind::Assign.is_assignment_operator());
        assert!(TokenKind::PlusAssign.is_assignment_operator());
        assert!(TokenKind::MinusAssign.is_assignment_operator());
        assert!(TokenKind::StarAssign.is_assignment_operator());
        assert!(TokenKind::SlashAssign.is_assignment_operator());
        assert!(TokenKind::PercentAssign.is_assignment_operator());
        assert!(TokenKind::DotAssign.is_assignment_operator());
        assert!(TokenKind::AndAssign.is_assignment_operator());
        assert!(TokenKind::OrAssign.is_assignment_operator());
        assert!(TokenKind::XorAssign.is_assignment_operator());
        assert!(TokenKind::PowerAssign.is_assignment_operator());
        assert!(TokenKind::LeftShiftAssign.is_assignment_operator());
        assert!(TokenKind::RightShiftAssign.is_assignment_operator());
        assert!(TokenKind::LogicalAndAssign.is_assignment_operator());
        assert!(TokenKind::LogicalOrAssign.is_assignment_operator());
        assert!(TokenKind::DefinedOrAssign.is_assignment_operator());
    }

    #[test]
    fn is_assignment_operator_returns_false_for_non_assign() {
        assert!(!TokenKind::Plus.is_assignment_operator());
        assert!(!TokenKind::Equal.is_assignment_operator());
        assert!(!TokenKind::Identifier.is_assignment_operator());
    }

    #[test]
    fn is_logical_operator_returns_true_for_logical_variants() {
        assert!(TokenKind::And.is_logical_operator());
        assert!(TokenKind::Or.is_logical_operator());
        assert!(TokenKind::Not.is_logical_operator());
        assert!(TokenKind::DefinedOr.is_logical_operator());
        assert!(TokenKind::WordAnd.is_logical_operator());
        assert!(TokenKind::WordOr.is_logical_operator());
        assert!(TokenKind::WordNot.is_logical_operator());
        assert!(TokenKind::WordXor.is_logical_operator());
    }

    #[test]
    fn is_logical_operator_returns_false_for_non_logical() {
        assert!(!TokenKind::Plus.is_logical_operator());
        assert!(!TokenKind::Assign.is_logical_operator());
        assert!(!TokenKind::Identifier.is_logical_operator());
    }

    #[test]
    fn is_open_delimiter_returns_true_for_open_delimiters() {
        assert!(TokenKind::LeftParen.is_open_delimiter());
        assert!(TokenKind::LeftBrace.is_open_delimiter());
        assert!(TokenKind::LeftBracket.is_open_delimiter());
    }

    #[test]
    fn is_open_delimiter_returns_false_for_non_open() {
        assert!(!TokenKind::RightParen.is_open_delimiter());
        assert!(!TokenKind::Semicolon.is_open_delimiter());
        assert!(!TokenKind::Plus.is_open_delimiter());
    }

    #[test]
    fn is_quote_like_returns_true_for_quote_variants() {
        assert!(TokenKind::Regex.is_quote_like());
        assert!(TokenKind::Substitution.is_quote_like());
        assert!(TokenKind::Transliteration.is_quote_like());
        assert!(TokenKind::QuoteSingle.is_quote_like());
        assert!(TokenKind::QuoteDouble.is_quote_like());
        assert!(TokenKind::QuoteWords.is_quote_like());
        assert!(TokenKind::QuoteCommand.is_quote_like());
        assert!(TokenKind::HeredocStart.is_quote_like());
    }

    #[test]
    fn is_quote_like_returns_false_for_non_quote() {
        assert!(!TokenKind::String.is_quote_like());
        assert!(!TokenKind::Identifier.is_quote_like());
        assert!(!TokenKind::LeftParen.is_quote_like());
    }

    #[test]
    fn is_recovery_boundary_returns_true_for_boundaries() {
        assert!(TokenKind::Semicolon.is_recovery_boundary());
        assert!(TokenKind::RightParen.is_recovery_boundary());
        assert!(TokenKind::RightBrace.is_recovery_boundary());
        assert!(TokenKind::RightBracket.is_recovery_boundary());
        assert!(TokenKind::Eof.is_recovery_boundary());
    }

    #[test]
    fn is_recovery_boundary_returns_false_for_non_boundary() {
        assert!(!TokenKind::Plus.is_recovery_boundary());
        assert!(!TokenKind::Identifier.is_recovery_boundary());
        assert!(!TokenKind::LeftParen.is_recovery_boundary());
    }

    // --- TokenRef::new_checked branches ---

    #[test]
    fn token_ref_new_checked_rejects_end_before_start() {
        assert_eq!(
            TokenRef::new_checked(TokenKind::Identifier, "x", 10, 3),
            Err(TokenSpanError::EndBeforeStart { start: 10, end: 3 })
        );
    }

    #[test]
    fn token_ref_new_checked_allows_empty_eof() -> Result<(), Box<dyn std::error::Error>> {
        let tok = TokenRef::new_checked(TokenKind::Eof, "", 7, 7)?;
        assert_eq!(tok.kind, TokenKind::Eof);
        assert_eq!(tok.start, 7);
        assert!(tok.is_empty());
        Ok(())
    }

    #[test]
    fn token_ref_new_checked_allows_empty_unknown() -> Result<(), Box<dyn std::error::Error>> {
        let tok = TokenRef::new_checked(TokenKind::Unknown, "", 3, 3)?;
        assert_eq!(tok.kind, TokenKind::Unknown);
        assert_eq!(tok.start, 3);
        assert!(tok.is_empty());
        Ok(())
    }

    #[test]
    fn token_ref_new_checked_rejects_empty_non_eof() {
        assert_eq!(
            TokenRef::new_checked(TokenKind::Identifier, "", 5, 5),
            Err(TokenSpanError::EmptySpanNotAllowed { kind: TokenKind::Identifier, at: 5 })
        );
    }
}
