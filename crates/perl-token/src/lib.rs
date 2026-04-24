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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

pub const ALL_TOKEN_KINDS: &[TokenKind] = &[
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
    pub fn info(self) -> TokenKindInfo {
        fn row(
            kind: TokenKind,
            display_name: &'static str,
            category: TokenCategory,
            canonical_lexeme: Option<&'static str>,
            keyword_spelling: Option<&'static str>,
            operator_spelling: Option<&'static str>,
        ) -> TokenKindInfo {
            TokenKindInfo {
                kind,
                display_name,
                category,
                canonical_lexeme,
                keyword_spelling,
                operator_spelling,
            }
        }

        match self {
            TokenKind::My => {
                row(self, "'my'", TokenCategory::Keyword, Some("my"), Some("my"), None)
            }
            TokenKind::Our => {
                row(self, "'our'", TokenCategory::Keyword, Some("our"), Some("our"), None)
            }
            TokenKind::Local => {
                row(self, "'local'", TokenCategory::Keyword, Some("local"), Some("local"), None)
            }
            TokenKind::State => {
                row(self, "'state'", TokenCategory::Keyword, Some("state"), Some("state"), None)
            }
            TokenKind::Sub => {
                row(self, "'sub'", TokenCategory::Keyword, Some("sub"), Some("sub"), None)
            }
            TokenKind::If => {
                row(self, "'if'", TokenCategory::Keyword, Some("if"), Some("if"), None)
            }
            TokenKind::Elsif => {
                row(self, "'elsif'", TokenCategory::Keyword, Some("elsif"), Some("elsif"), None)
            }
            TokenKind::Else => {
                row(self, "'else'", TokenCategory::Keyword, Some("else"), Some("else"), None)
            }
            TokenKind::Unless => {
                row(self, "'unless'", TokenCategory::Keyword, Some("unless"), Some("unless"), None)
            }
            TokenKind::While => {
                row(self, "'while'", TokenCategory::Keyword, Some("while"), Some("while"), None)
            }
            TokenKind::Until => {
                row(self, "'until'", TokenCategory::Keyword, Some("until"), Some("until"), None)
            }
            TokenKind::For => {
                row(self, "'for'", TokenCategory::Keyword, Some("for"), Some("for"), None)
            }
            TokenKind::Foreach => row(
                self,
                "'foreach'",
                TokenCategory::Keyword,
                Some("foreach"),
                Some("foreach"),
                None,
            ),
            TokenKind::Return => {
                row(self, "'return'", TokenCategory::Keyword, Some("return"), Some("return"), None)
            }
            TokenKind::Package => row(
                self,
                "'package'",
                TokenCategory::Keyword,
                Some("package"),
                Some("package"),
                None,
            ),
            TokenKind::Use => {
                row(self, "'use'", TokenCategory::Keyword, Some("use"), Some("use"), None)
            }
            TokenKind::No => {
                row(self, "'no'", TokenCategory::Keyword, Some("no"), Some("no"), None)
            }
            TokenKind::Begin => {
                row(self, "'BEGIN'", TokenCategory::Keyword, Some("BEGIN"), Some("BEGIN"), None)
            }
            TokenKind::End => {
                row(self, "'END'", TokenCategory::Keyword, Some("END"), Some("END"), None)
            }
            TokenKind::Check => {
                row(self, "'CHECK'", TokenCategory::Keyword, Some("CHECK"), Some("CHECK"), None)
            }
            TokenKind::Init => {
                row(self, "'INIT'", TokenCategory::Keyword, Some("INIT"), Some("INIT"), None)
            }
            TokenKind::Unitcheck => row(
                self,
                "'UNITCHECK'",
                TokenCategory::Keyword,
                Some("UNITCHECK"),
                Some("UNITCHECK"),
                None,
            ),
            TokenKind::Eval => {
                row(self, "'eval'", TokenCategory::Keyword, Some("eval"), Some("eval"), None)
            }
            TokenKind::Do => {
                row(self, "'do'", TokenCategory::Keyword, Some("do"), Some("do"), None)
            }
            TokenKind::Given => {
                row(self, "'given'", TokenCategory::Keyword, Some("given"), Some("given"), None)
            }
            TokenKind::When => {
                row(self, "'when'", TokenCategory::Keyword, Some("when"), Some("when"), None)
            }
            TokenKind::Default => row(
                self,
                "'default'",
                TokenCategory::Keyword,
                Some("default"),
                Some("default"),
                None,
            ),
            TokenKind::Try => {
                row(self, "'try'", TokenCategory::Keyword, Some("try"), Some("try"), None)
            }
            TokenKind::Catch => {
                row(self, "'catch'", TokenCategory::Keyword, Some("catch"), Some("catch"), None)
            }
            TokenKind::Finally => row(
                self,
                "'finally'",
                TokenCategory::Keyword,
                Some("finally"),
                Some("finally"),
                None,
            ),
            TokenKind::Continue => row(
                self,
                "'continue'",
                TokenCategory::Keyword,
                Some("continue"),
                Some("continue"),
                None,
            ),
            TokenKind::Next => {
                row(self, "'next'", TokenCategory::Keyword, Some("next"), Some("next"), None)
            }
            TokenKind::Last => {
                row(self, "'last'", TokenCategory::Keyword, Some("last"), Some("last"), None)
            }
            TokenKind::Redo => {
                row(self, "'redo'", TokenCategory::Keyword, Some("redo"), Some("redo"), None)
            }
            TokenKind::Goto => {
                row(self, "'goto'", TokenCategory::Keyword, Some("goto"), Some("goto"), None)
            }
            TokenKind::Class => {
                row(self, "'class'", TokenCategory::Keyword, Some("class"), Some("class"), None)
            }
            TokenKind::Method => {
                row(self, "'method'", TokenCategory::Keyword, Some("method"), Some("method"), None)
            }
            TokenKind::Field => {
                row(self, "'field'", TokenCategory::Keyword, Some("field"), Some("field"), None)
            }
            TokenKind::Format => {
                row(self, "'format'", TokenCategory::Keyword, Some("format"), Some("format"), None)
            }
            TokenKind::Undef => {
                row(self, "'undef'", TokenCategory::Keyword, Some("undef"), Some("undef"), None)
            }
            TokenKind::Defer => {
                row(self, "'defer'", TokenCategory::Keyword, Some("defer"), Some("defer"), None)
            }
            TokenKind::Assign => {
                row(self, "'='", TokenCategory::Operator, Some("="), None, Some("="))
            }
            TokenKind::Plus => {
                row(self, "'+'", TokenCategory::Operator, Some("+"), None, Some("+"))
            }
            TokenKind::Minus => {
                row(self, "'-'", TokenCategory::Operator, Some("-"), None, Some("-"))
            }
            TokenKind::Star => {
                row(self, "'*'", TokenCategory::Operator, Some("*"), None, Some("*"))
            }
            TokenKind::Slash => {
                row(self, "'/'", TokenCategory::Operator, Some("/"), None, Some("/"))
            }
            TokenKind::Percent => {
                row(self, "'%'", TokenCategory::Operator, Some("%"), None, Some("%"))
            }
            TokenKind::Power => {
                row(self, "'**'", TokenCategory::Operator, Some("**"), None, Some("**"))
            }
            TokenKind::LeftShift => {
                row(self, "'<<'", TokenCategory::Operator, Some("<<"), None, Some("<<"))
            }
            TokenKind::RightShift => {
                row(self, "'>>'", TokenCategory::Operator, Some(">>"), None, Some(">>"))
            }
            TokenKind::BitwiseAnd => {
                row(self, "'&'", TokenCategory::Operator, Some("&"), None, Some("&"))
            }
            TokenKind::BitwiseOr => {
                row(self, "'|'", TokenCategory::Operator, Some("|"), None, Some("|"))
            }
            TokenKind::BitwiseXor => {
                row(self, "'^'", TokenCategory::Operator, Some("^"), None, Some("^"))
            }
            TokenKind::BitwiseNot => {
                row(self, "'~'", TokenCategory::Operator, Some("~"), None, Some("~"))
            }
            TokenKind::PlusAssign => {
                row(self, "'+='", TokenCategory::Operator, Some("+="), None, Some("+="))
            }
            TokenKind::MinusAssign => {
                row(self, "'-='", TokenCategory::Operator, Some("-="), None, Some("-="))
            }
            TokenKind::StarAssign => {
                row(self, "'*='", TokenCategory::Operator, Some("*="), None, Some("*="))
            }
            TokenKind::SlashAssign => {
                row(self, "'/='", TokenCategory::Operator, Some("/="), None, Some("/="))
            }
            TokenKind::PercentAssign => {
                row(self, "'%='", TokenCategory::Operator, Some("%="), None, Some("%="))
            }
            TokenKind::DotAssign => {
                row(self, "'.='", TokenCategory::Operator, Some(".="), None, Some(".="))
            }
            TokenKind::AndAssign => {
                row(self, "'&='", TokenCategory::Operator, Some("&="), None, Some("&="))
            }
            TokenKind::OrAssign => {
                row(self, "'|='", TokenCategory::Operator, Some("|="), None, Some("|="))
            }
            TokenKind::XorAssign => {
                row(self, "'^='", TokenCategory::Operator, Some("^="), None, Some("^="))
            }
            TokenKind::PowerAssign => {
                row(self, "'**='", TokenCategory::Operator, Some("**="), None, Some("**="))
            }
            TokenKind::LeftShiftAssign => {
                row(self, "'<<='", TokenCategory::Operator, Some("<<="), None, Some("<<="))
            }
            TokenKind::RightShiftAssign => {
                row(self, "'>>='", TokenCategory::Operator, Some(">>="), None, Some(">>="))
            }
            TokenKind::LogicalAndAssign => {
                row(self, "'&&='", TokenCategory::Operator, Some("&&="), None, Some("&&="))
            }
            TokenKind::LogicalOrAssign => {
                row(self, "'||='", TokenCategory::Operator, Some("||="), None, Some("||="))
            }
            TokenKind::DefinedOrAssign => {
                row(self, "'//='", TokenCategory::Operator, Some("//="), None, Some("//="))
            }
            TokenKind::Equal => {
                row(self, "'=='", TokenCategory::Operator, Some("=="), None, Some("=="))
            }
            TokenKind::NotEqual => {
                row(self, "'!='", TokenCategory::Operator, Some("!="), None, Some("!="))
            }
            TokenKind::Match => {
                row(self, "'=~'", TokenCategory::Operator, Some("=~"), None, Some("=~"))
            }
            TokenKind::NotMatch => {
                row(self, "'!~'", TokenCategory::Operator, Some("!~"), None, Some("!~"))
            }
            TokenKind::SmartMatch => {
                row(self, "'~~'", TokenCategory::Operator, Some("~~"), None, Some("~~"))
            }
            TokenKind::Less => {
                row(self, "'<'", TokenCategory::Operator, Some("<"), None, Some("<"))
            }
            TokenKind::Greater => {
                row(self, "'>'", TokenCategory::Operator, Some(">"), None, Some(">"))
            }
            TokenKind::LessEqual => {
                row(self, "'<='", TokenCategory::Operator, Some("<="), None, Some("<="))
            }
            TokenKind::GreaterEqual => {
                row(self, "'>='", TokenCategory::Operator, Some(">="), None, Some(">="))
            }
            TokenKind::Spaceship => {
                row(self, "'<=>'", TokenCategory::Operator, Some("<=>"), None, Some("<=>"))
            }
            TokenKind::StringCompare => {
                row(self, "'cmp'", TokenCategory::Operator, Some("cmp"), None, Some("cmp"))
            }
            TokenKind::And => {
                row(self, "'&&'", TokenCategory::Operator, Some("&&"), None, Some("&&"))
            }
            TokenKind::Or => {
                row(self, "'||'", TokenCategory::Operator, Some("||"), None, Some("||"))
            }
            TokenKind::Not => row(self, "'!'", TokenCategory::Operator, Some("!"), None, Some("!")),
            TokenKind::DefinedOr => {
                row(self, "'//'", TokenCategory::Operator, Some("//"), None, Some("//"))
            }
            TokenKind::WordAnd => {
                row(self, "'and'", TokenCategory::Operator, Some("and"), None, Some("and"))
            }
            TokenKind::WordOr => {
                row(self, "'or'", TokenCategory::Operator, Some("or"), None, Some("or"))
            }
            TokenKind::WordNot => {
                row(self, "'not'", TokenCategory::Operator, Some("not"), None, Some("not"))
            }
            TokenKind::WordXor => {
                row(self, "'xor'", TokenCategory::Operator, Some("xor"), None, Some("xor"))
            }
            TokenKind::Arrow => {
                row(self, "'->'", TokenCategory::Operator, Some("->"), None, Some("->"))
            }
            TokenKind::FatArrow => {
                row(self, "'=>'", TokenCategory::Operator, Some("=>"), None, Some("=>"))
            }
            TokenKind::Dot => row(self, "'.'", TokenCategory::Operator, Some("."), None, Some(".")),
            TokenKind::Range => {
                row(self, "'..'", TokenCategory::Operator, Some(".."), None, Some(".."))
            }
            TokenKind::Ellipsis => {
                row(self, "'...'", TokenCategory::Operator, Some("..."), None, Some("..."))
            }
            TokenKind::Increment => {
                row(self, "'++'", TokenCategory::Operator, Some("++"), None, Some("++"))
            }
            TokenKind::Decrement => {
                row(self, "'--'", TokenCategory::Operator, Some("--"), None, Some("--"))
            }
            TokenKind::DoubleColon => {
                row(self, "'::'", TokenCategory::Operator, Some("::"), None, Some("::"))
            }
            TokenKind::Question => {
                row(self, "'?'", TokenCategory::Operator, Some("?"), None, Some("?"))
            }
            TokenKind::Colon => {
                row(self, "':'", TokenCategory::Operator, Some(":"), None, Some(":"))
            }
            TokenKind::Backslash => {
                row(self, "'\\'", TokenCategory::Operator, Some("\\"), None, Some("\\"))
            }
            TokenKind::LeftParen => {
                row(self, "'('", TokenCategory::Delimiter, Some("("), None, None)
            }
            TokenKind::RightParen => {
                row(self, "')'", TokenCategory::Delimiter, Some(")"), None, None)
            }
            TokenKind::LeftBrace => {
                row(self, "'{'", TokenCategory::Delimiter, Some("{"), None, None)
            }
            TokenKind::RightBrace => {
                row(self, "'}'", TokenCategory::Delimiter, Some("}"), None, None)
            }
            TokenKind::LeftBracket => {
                row(self, "'['", TokenCategory::Delimiter, Some("["), None, None)
            }
            TokenKind::RightBracket => {
                row(self, "']'", TokenCategory::Delimiter, Some("]"), None, None)
            }
            TokenKind::Semicolon => {
                row(self, "';'", TokenCategory::Delimiter, Some(";"), None, None)
            }
            TokenKind::Comma => row(self, "','", TokenCategory::Delimiter, Some(","), None, None),
            TokenKind::Number => row(self, "number", TokenCategory::Literal, None, None, None),
            TokenKind::String => row(self, "string", TokenCategory::Literal, None, None, None),
            TokenKind::Regex => row(self, "regex", TokenCategory::Literal, None, None, None),
            TokenKind::Substitution => {
                row(self, "substitution (s///)", TokenCategory::Literal, None, None, None)
            }
            TokenKind::Transliteration => {
                row(self, "transliteration (tr///)", TokenCategory::Literal, None, None, None)
            }
            TokenKind::QuoteSingle => {
                row(self, "q// string", TokenCategory::Literal, Some("q"), None, None)
            }
            TokenKind::QuoteDouble => {
                row(self, "qq// string", TokenCategory::Literal, Some("qq"), None, None)
            }
            TokenKind::QuoteWords => {
                row(self, "qw() word list", TokenCategory::Literal, Some("qw"), None, None)
            }
            TokenKind::QuoteCommand => {
                row(self, "qx// command", TokenCategory::Literal, Some("qx"), None, None)
            }
            TokenKind::HeredocStart => {
                row(self, "heredoc (<<)", TokenCategory::Literal, Some("<<"), None, None)
            }
            TokenKind::HeredocBody => {
                row(self, "heredoc body", TokenCategory::Literal, None, None, None)
            }
            TokenKind::FormatBody => {
                row(self, "format body", TokenCategory::Literal, None, None, None)
            }
            TokenKind::DataMarker => {
                row(self, "__DATA__", TokenCategory::Literal, Some("__DATA__"), None, None)
            }
            TokenKind::DataBody => {
                row(self, "data section", TokenCategory::Literal, None, None, None)
            }
            TokenKind::VString => {
                row(self, "version string", TokenCategory::Literal, None, None, None)
            }
            TokenKind::UnknownRest => {
                row(self, "unparsed content", TokenCategory::Literal, None, None, None)
            }
            TokenKind::HeredocDepthLimit => {
                row(self, "heredoc depth limit", TokenCategory::Literal, None, None, None)
            }
            TokenKind::Identifier => {
                row(self, "identifier", TokenCategory::Identifier, None, None, None)
            }
            TokenKind::ScalarSigil => row(self, "'$'", TokenCategory::Sigil, Some("$"), None, None),
            TokenKind::ArraySigil => row(self, "'@'", TokenCategory::Sigil, Some("@"), None, None),
            TokenKind::HashSigil => row(self, "'%'", TokenCategory::Sigil, Some("%"), None, None),
            TokenKind::SubSigil => row(self, "'&'", TokenCategory::Sigil, Some("&"), None, None),
            TokenKind::GlobSigil => row(self, "'*'", TokenCategory::Sigil, Some("*"), None, None),
            TokenKind::Eof => row(self, "end of input", TokenCategory::Special, None, None, None),
            TokenKind::Unknown => {
                row(self, "unknown token", TokenCategory::Special, None, None, None)
            }
        }
    }

    pub fn display_name(self) -> &'static str {
        self.info().display_name
    }
    pub fn category(self) -> TokenCategory {
        self.info().category
    }
    pub fn is_keyword(self) -> bool {
        self.category() == TokenCategory::Keyword
    }
    pub fn is_operator(self) -> bool {
        self.category() == TokenCategory::Operator
    }
    pub fn is_delimiter(self) -> bool {
        self.category() == TokenCategory::Delimiter
    }
    pub fn is_literal(self) -> bool {
        self.category() == TokenCategory::Literal
    }
    pub fn is_identifier_like(self) -> bool {
        matches!(self.category(), TokenCategory::Identifier | TokenCategory::Sigil)
    }
    pub fn is_sigil(self) -> bool {
        self.category() == TokenCategory::Sigil
    }
    pub fn is_special(self) -> bool {
        self.category() == TokenCategory::Special
    }
    pub fn canonical_lexeme(self) -> Option<&'static str> {
        self.info().canonical_lexeme
    }
}
