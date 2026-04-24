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

impl TokenKind {
    /// Return metadata for this token kind.
    pub fn metadata(self) -> &'static TokenKindMetadata {
        for metadata in &TOKEN_KIND_METADATA {
            if metadata.kind == self {
                return metadata;
            }
        }

        &UNKNOWN_TOKEN_KIND_METADATA
    }

    /// Return every known token kind in declaration order.
    pub fn all() -> &'static [TokenKind] {
        static ALL_TOKEN_KINDS: std::sync::LazyLock<Vec<TokenKind>> =
            std::sync::LazyLock::new(|| {
                TOKEN_KIND_METADATA.iter().map(|metadata| metadata.kind).collect()
            });

        ALL_TOKEN_KINDS.as_slice()
    }

    /// Return metadata for every known token kind.
    pub fn all_metadata() -> &'static [TokenKindMetadata] {
        &TOKEN_KIND_METADATA
    }

    /// Return a user-friendly display name for this token kind.
    pub fn display_name(self) -> &'static str {
        self.metadata().display_name
    }

    /// Return the coarse token category used by conformance tests and docs.
    pub fn category(self) -> TokenCategory {
        self.metadata().category
    }
}

/// Coarse groups used for token metadata and conformance checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenCategory {
    Keyword,
    Operator,
    Delimiter,
    Literal,
    IdentifierOrSigil,
    Special,
}

/// Stable metadata record for every [`TokenKind`] variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenKindMetadata {
    pub kind: TokenKind,
    pub display_name: &'static str,
    pub category: TokenCategory,
}

const UNKNOWN_TOKEN_KIND_METADATA: TokenKindMetadata = TokenKindMetadata {
    kind: TokenKind::Unknown,
    display_name: "unknown token",
    category: TokenCategory::Special,
};

const TOKEN_KIND_METADATA: [TokenKindMetadata; 132] = [
    TokenKindMetadata {
        kind: TokenKind::My,
        display_name: "'my'",
        category: TokenCategory::Keyword,
    },
    TokenKindMetadata {
        kind: TokenKind::Our,
        display_name: "'our'",
        category: TokenCategory::Keyword,
    },
    TokenKindMetadata {
        kind: TokenKind::Local,
        display_name: "'local'",
        category: TokenCategory::Keyword,
    },
    TokenKindMetadata {
        kind: TokenKind::State,
        display_name: "'state'",
        category: TokenCategory::Keyword,
    },
    TokenKindMetadata {
        kind: TokenKind::Sub,
        display_name: "'sub'",
        category: TokenCategory::Keyword,
    },
    TokenKindMetadata {
        kind: TokenKind::If,
        display_name: "'if'",
        category: TokenCategory::Keyword,
    },
    TokenKindMetadata {
        kind: TokenKind::Elsif,
        display_name: "'elsif'",
        category: TokenCategory::Keyword,
    },
    TokenKindMetadata {
        kind: TokenKind::Else,
        display_name: "'else'",
        category: TokenCategory::Keyword,
    },
    TokenKindMetadata {
        kind: TokenKind::Unless,
        display_name: "'unless'",
        category: TokenCategory::Keyword,
    },
    TokenKindMetadata {
        kind: TokenKind::While,
        display_name: "'while'",
        category: TokenCategory::Keyword,
    },
    TokenKindMetadata {
        kind: TokenKind::Until,
        display_name: "'until'",
        category: TokenCategory::Keyword,
    },
    TokenKindMetadata {
        kind: TokenKind::For,
        display_name: "'for'",
        category: TokenCategory::Keyword,
    },
    TokenKindMetadata {
        kind: TokenKind::Foreach,
        display_name: "'foreach'",
        category: TokenCategory::Keyword,
    },
    TokenKindMetadata {
        kind: TokenKind::Return,
        display_name: "'return'",
        category: TokenCategory::Keyword,
    },
    TokenKindMetadata {
        kind: TokenKind::Package,
        display_name: "'package'",
        category: TokenCategory::Keyword,
    },
    TokenKindMetadata {
        kind: TokenKind::Use,
        display_name: "'use'",
        category: TokenCategory::Keyword,
    },
    TokenKindMetadata {
        kind: TokenKind::No,
        display_name: "'no'",
        category: TokenCategory::Keyword,
    },
    TokenKindMetadata {
        kind: TokenKind::Begin,
        display_name: "'BEGIN'",
        category: TokenCategory::Keyword,
    },
    TokenKindMetadata {
        kind: TokenKind::End,
        display_name: "'END'",
        category: TokenCategory::Keyword,
    },
    TokenKindMetadata {
        kind: TokenKind::Check,
        display_name: "'CHECK'",
        category: TokenCategory::Keyword,
    },
    TokenKindMetadata {
        kind: TokenKind::Init,
        display_name: "'INIT'",
        category: TokenCategory::Keyword,
    },
    TokenKindMetadata {
        kind: TokenKind::Unitcheck,
        display_name: "'UNITCHECK'",
        category: TokenCategory::Keyword,
    },
    TokenKindMetadata {
        kind: TokenKind::Eval,
        display_name: "'eval'",
        category: TokenCategory::Keyword,
    },
    TokenKindMetadata {
        kind: TokenKind::Do,
        display_name: "'do'",
        category: TokenCategory::Keyword,
    },
    TokenKindMetadata {
        kind: TokenKind::Given,
        display_name: "'given'",
        category: TokenCategory::Keyword,
    },
    TokenKindMetadata {
        kind: TokenKind::When,
        display_name: "'when'",
        category: TokenCategory::Keyword,
    },
    TokenKindMetadata {
        kind: TokenKind::Default,
        display_name: "'default'",
        category: TokenCategory::Keyword,
    },
    TokenKindMetadata {
        kind: TokenKind::Try,
        display_name: "'try'",
        category: TokenCategory::Keyword,
    },
    TokenKindMetadata {
        kind: TokenKind::Catch,
        display_name: "'catch'",
        category: TokenCategory::Keyword,
    },
    TokenKindMetadata {
        kind: TokenKind::Finally,
        display_name: "'finally'",
        category: TokenCategory::Keyword,
    },
    TokenKindMetadata {
        kind: TokenKind::Continue,
        display_name: "'continue'",
        category: TokenCategory::Keyword,
    },
    TokenKindMetadata {
        kind: TokenKind::Next,
        display_name: "'next'",
        category: TokenCategory::Keyword,
    },
    TokenKindMetadata {
        kind: TokenKind::Last,
        display_name: "'last'",
        category: TokenCategory::Keyword,
    },
    TokenKindMetadata {
        kind: TokenKind::Redo,
        display_name: "'redo'",
        category: TokenCategory::Keyword,
    },
    TokenKindMetadata {
        kind: TokenKind::Goto,
        display_name: "'goto'",
        category: TokenCategory::Keyword,
    },
    TokenKindMetadata {
        kind: TokenKind::Class,
        display_name: "'class'",
        category: TokenCategory::Keyword,
    },
    TokenKindMetadata {
        kind: TokenKind::Method,
        display_name: "'method'",
        category: TokenCategory::Keyword,
    },
    TokenKindMetadata {
        kind: TokenKind::Field,
        display_name: "'field'",
        category: TokenCategory::Keyword,
    },
    TokenKindMetadata {
        kind: TokenKind::Format,
        display_name: "'format'",
        category: TokenCategory::Keyword,
    },
    TokenKindMetadata {
        kind: TokenKind::Undef,
        display_name: "'undef'",
        category: TokenCategory::Keyword,
    },
    TokenKindMetadata {
        kind: TokenKind::Defer,
        display_name: "'defer'",
        category: TokenCategory::Keyword,
    },
    TokenKindMetadata {
        kind: TokenKind::Assign,
        display_name: "'='",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::Plus,
        display_name: "'+'",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::Minus,
        display_name: "'-'",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::Star,
        display_name: "'*'",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::Slash,
        display_name: "'/'",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::Percent,
        display_name: "'%'",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::Power,
        display_name: "'**'",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::LeftShift,
        display_name: "'<<'",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::RightShift,
        display_name: "'>>'",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::BitwiseAnd,
        display_name: "'&'",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::BitwiseOr,
        display_name: "'|'",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::BitwiseXor,
        display_name: "'^'",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::BitwiseNot,
        display_name: "'~'",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::PlusAssign,
        display_name: "'+='",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::MinusAssign,
        display_name: "'-='",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::StarAssign,
        display_name: "'*='",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::SlashAssign,
        display_name: "'/='",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::PercentAssign,
        display_name: "'%='",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::DotAssign,
        display_name: "'.='",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::AndAssign,
        display_name: "'&='",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::OrAssign,
        display_name: "'|='",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::XorAssign,
        display_name: "'^='",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::PowerAssign,
        display_name: "'**='",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::LeftShiftAssign,
        display_name: "'<<='",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::RightShiftAssign,
        display_name: "'>>='",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::LogicalAndAssign,
        display_name: "'&&='",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::LogicalOrAssign,
        display_name: "'||='",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::DefinedOrAssign,
        display_name: "'//='",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::Equal,
        display_name: "'=='",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::NotEqual,
        display_name: "'!='",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::Match,
        display_name: "'=~'",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::NotMatch,
        display_name: "'!~'",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::SmartMatch,
        display_name: "'~~'",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::Less,
        display_name: "'<'",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::Greater,
        display_name: "'>'",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::LessEqual,
        display_name: "'<='",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::GreaterEqual,
        display_name: "'>='",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::Spaceship,
        display_name: "'<=>'",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::StringCompare,
        display_name: "'cmp'",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::And,
        display_name: "'&&'",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::Or,
        display_name: "'||'",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::Not,
        display_name: "'!'",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::DefinedOr,
        display_name: "'//'",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::WordAnd,
        display_name: "'and'",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::WordOr,
        display_name: "'or'",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::WordNot,
        display_name: "'not'",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::WordXor,
        display_name: "'xor'",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::Arrow,
        display_name: "'->'",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::FatArrow,
        display_name: "'=>'",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::Dot,
        display_name: "'.'",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::Range,
        display_name: "'..'",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::Ellipsis,
        display_name: "'...'",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::Increment,
        display_name: "'++'",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::Decrement,
        display_name: "'--'",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::DoubleColon,
        display_name: "'::'",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::Question,
        display_name: "'?'",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::Colon,
        display_name: "':'",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::Backslash,
        display_name: "'\\'",
        category: TokenCategory::Operator,
    },
    TokenKindMetadata {
        kind: TokenKind::LeftParen,
        display_name: "'('",
        category: TokenCategory::Delimiter,
    },
    TokenKindMetadata {
        kind: TokenKind::RightParen,
        display_name: "')'",
        category: TokenCategory::Delimiter,
    },
    TokenKindMetadata {
        kind: TokenKind::LeftBrace,
        display_name: "'{'",
        category: TokenCategory::Delimiter,
    },
    TokenKindMetadata {
        kind: TokenKind::RightBrace,
        display_name: "'}'",
        category: TokenCategory::Delimiter,
    },
    TokenKindMetadata {
        kind: TokenKind::LeftBracket,
        display_name: "'['",
        category: TokenCategory::Delimiter,
    },
    TokenKindMetadata {
        kind: TokenKind::RightBracket,
        display_name: "']'",
        category: TokenCategory::Delimiter,
    },
    TokenKindMetadata {
        kind: TokenKind::Semicolon,
        display_name: "';'",
        category: TokenCategory::Delimiter,
    },
    TokenKindMetadata {
        kind: TokenKind::Comma,
        display_name: "','",
        category: TokenCategory::Delimiter,
    },
    TokenKindMetadata {
        kind: TokenKind::Number,
        display_name: "number",
        category: TokenCategory::Literal,
    },
    TokenKindMetadata {
        kind: TokenKind::String,
        display_name: "string",
        category: TokenCategory::Literal,
    },
    TokenKindMetadata {
        kind: TokenKind::Regex,
        display_name: "regex",
        category: TokenCategory::Literal,
    },
    TokenKindMetadata {
        kind: TokenKind::Substitution,
        display_name: "substitution (s///)",
        category: TokenCategory::Literal,
    },
    TokenKindMetadata {
        kind: TokenKind::Transliteration,
        display_name: "transliteration (tr///)",
        category: TokenCategory::Literal,
    },
    TokenKindMetadata {
        kind: TokenKind::QuoteSingle,
        display_name: "q// string",
        category: TokenCategory::Literal,
    },
    TokenKindMetadata {
        kind: TokenKind::QuoteDouble,
        display_name: "qq// string",
        category: TokenCategory::Literal,
    },
    TokenKindMetadata {
        kind: TokenKind::QuoteWords,
        display_name: "qw() word list",
        category: TokenCategory::Literal,
    },
    TokenKindMetadata {
        kind: TokenKind::QuoteCommand,
        display_name: "qx// command",
        category: TokenCategory::Literal,
    },
    TokenKindMetadata {
        kind: TokenKind::HeredocStart,
        display_name: "heredoc (<<)",
        category: TokenCategory::Literal,
    },
    TokenKindMetadata {
        kind: TokenKind::HeredocBody,
        display_name: "heredoc body",
        category: TokenCategory::Literal,
    },
    TokenKindMetadata {
        kind: TokenKind::FormatBody,
        display_name: "format body",
        category: TokenCategory::Literal,
    },
    TokenKindMetadata {
        kind: TokenKind::DataMarker,
        display_name: "__DATA__",
        category: TokenCategory::Literal,
    },
    TokenKindMetadata {
        kind: TokenKind::DataBody,
        display_name: "data section",
        category: TokenCategory::Literal,
    },
    TokenKindMetadata {
        kind: TokenKind::VString,
        display_name: "version string",
        category: TokenCategory::Literal,
    },
    TokenKindMetadata {
        kind: TokenKind::UnknownRest,
        display_name: "unparsed content",
        category: TokenCategory::Literal,
    },
    TokenKindMetadata {
        kind: TokenKind::HeredocDepthLimit,
        display_name: "heredoc depth limit",
        category: TokenCategory::Literal,
    },
    TokenKindMetadata {
        kind: TokenKind::Identifier,
        display_name: "identifier",
        category: TokenCategory::IdentifierOrSigil,
    },
    TokenKindMetadata {
        kind: TokenKind::ScalarSigil,
        display_name: "'$'",
        category: TokenCategory::IdentifierOrSigil,
    },
    TokenKindMetadata {
        kind: TokenKind::ArraySigil,
        display_name: "'@'",
        category: TokenCategory::IdentifierOrSigil,
    },
    TokenKindMetadata {
        kind: TokenKind::HashSigil,
        display_name: "'%'",
        category: TokenCategory::IdentifierOrSigil,
    },
    TokenKindMetadata {
        kind: TokenKind::SubSigil,
        display_name: "'&'",
        category: TokenCategory::IdentifierOrSigil,
    },
    TokenKindMetadata {
        kind: TokenKind::GlobSigil,
        display_name: "'*'",
        category: TokenCategory::IdentifierOrSigil,
    },
    TokenKindMetadata {
        kind: TokenKind::Eof,
        display_name: "end of input",
        category: TokenCategory::Special,
    },
    TokenKindMetadata {
        kind: TokenKind::Unknown,
        display_name: "unknown token",
        category: TokenCategory::Special,
    },
];
