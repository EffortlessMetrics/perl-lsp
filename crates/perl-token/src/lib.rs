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

/// High-level category for a [`TokenKind`].
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

/// Executable metadata for a [`TokenKind`] variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenKindInfo {
    pub kind: TokenKind,
    pub display_name: &'static str,
    pub category: TokenCategory,
    pub canonical_lexeme: Option<&'static str>,
    pub keyword_spelling: Option<&'static str>,
    pub operator_spelling: Option<&'static str>,
}

macro_rules! token_info {
    ($kind:ident, $display_name:literal, $category:ident, $lexeme:expr, $keyword:expr, $operator:expr) => {
        TokenKindInfo {
            kind: TokenKind::$kind,
            display_name: $display_name,
            category: TokenCategory::$category,
            canonical_lexeme: $lexeme,
            keyword_spelling: $keyword,
            operator_spelling: $operator,
        }
    };
}

const TOKEN_KIND_INFO: &[TokenKindInfo] = &[
    // Keywords
    token_info!(My, "'my'", Keyword, Some("my"), Some("my"), None),
    token_info!(Our, "'our'", Keyword, Some("our"), Some("our"), None),
    token_info!(Local, "'local'", Keyword, Some("local"), Some("local"), None),
    token_info!(State, "'state'", Keyword, Some("state"), Some("state"), None),
    token_info!(Sub, "'sub'", Keyword, Some("sub"), Some("sub"), None),
    token_info!(If, "'if'", Keyword, Some("if"), Some("if"), None),
    token_info!(Elsif, "'elsif'", Keyword, Some("elsif"), Some("elsif"), None),
    token_info!(Else, "'else'", Keyword, Some("else"), Some("else"), None),
    token_info!(Unless, "'unless'", Keyword, Some("unless"), Some("unless"), None),
    token_info!(While, "'while'", Keyword, Some("while"), Some("while"), None),
    token_info!(Until, "'until'", Keyword, Some("until"), Some("until"), None),
    token_info!(For, "'for'", Keyword, Some("for"), Some("for"), None),
    token_info!(Foreach, "'foreach'", Keyword, Some("foreach"), Some("foreach"), None),
    token_info!(Return, "'return'", Keyword, Some("return"), Some("return"), None),
    token_info!(Package, "'package'", Keyword, Some("package"), Some("package"), None),
    token_info!(Use, "'use'", Keyword, Some("use"), Some("use"), None),
    token_info!(No, "'no'", Keyword, Some("no"), Some("no"), None),
    token_info!(Begin, "'BEGIN'", Keyword, Some("BEGIN"), Some("BEGIN"), None),
    token_info!(End, "'END'", Keyword, Some("END"), Some("END"), None),
    token_info!(Check, "'CHECK'", Keyword, Some("CHECK"), Some("CHECK"), None),
    token_info!(Init, "'INIT'", Keyword, Some("INIT"), Some("INIT"), None),
    token_info!(Unitcheck, "'UNITCHECK'", Keyword, Some("UNITCHECK"), Some("UNITCHECK"), None),
    token_info!(Eval, "'eval'", Keyword, Some("eval"), Some("eval"), None),
    token_info!(Do, "'do'", Keyword, Some("do"), Some("do"), None),
    token_info!(Given, "'given'", Keyword, Some("given"), Some("given"), None),
    token_info!(When, "'when'", Keyword, Some("when"), Some("when"), None),
    token_info!(Default, "'default'", Keyword, Some("default"), Some("default"), None),
    token_info!(Try, "'try'", Keyword, Some("try"), Some("try"), None),
    token_info!(Catch, "'catch'", Keyword, Some("catch"), Some("catch"), None),
    token_info!(Finally, "'finally'", Keyword, Some("finally"), Some("finally"), None),
    token_info!(Continue, "'continue'", Keyword, Some("continue"), Some("continue"), None),
    token_info!(Next, "'next'", Keyword, Some("next"), Some("next"), None),
    token_info!(Last, "'last'", Keyword, Some("last"), Some("last"), None),
    token_info!(Redo, "'redo'", Keyword, Some("redo"), Some("redo"), None),
    token_info!(Goto, "'goto'", Keyword, Some("goto"), Some("goto"), None),
    token_info!(Class, "'class'", Keyword, Some("class"), Some("class"), None),
    token_info!(Method, "'method'", Keyword, Some("method"), Some("method"), None),
    token_info!(Field, "'field'", Keyword, Some("field"), Some("field"), None),
    token_info!(Format, "'format'", Keyword, Some("format"), Some("format"), None),
    token_info!(Undef, "'undef'", Keyword, Some("undef"), Some("undef"), None),
    token_info!(Defer, "'defer'", Keyword, Some("defer"), Some("defer"), None),
    // Operators
    token_info!(Assign, "'='", Operator, Some("="), None, Some("=")),
    token_info!(Plus, "'+'", Operator, Some("+"), None, Some("+")),
    token_info!(Minus, "'-'", Operator, Some("-"), None, Some("-")),
    token_info!(Star, "'*'", Operator, Some("*"), None, Some("*")),
    token_info!(Slash, "'/'", Operator, Some("/"), None, Some("/")),
    token_info!(Percent, "'%'", Operator, Some("%"), None, Some("%")),
    token_info!(Power, "'**'", Operator, Some("**"), None, Some("**")),
    token_info!(LeftShift, "'<<'", Operator, Some("<<"), None, Some("<<")),
    token_info!(RightShift, "'>>'", Operator, Some(">>"), None, Some(">>")),
    token_info!(BitwiseAnd, "'&'", Operator, Some("&"), None, Some("&")),
    token_info!(BitwiseOr, "'|'", Operator, Some("|"), None, Some("|")),
    token_info!(BitwiseXor, "'^'", Operator, Some("^"), None, Some("^")),
    token_info!(BitwiseNot, "'~'", Operator, Some("~"), None, Some("~")),
    token_info!(PlusAssign, "'+='", Operator, Some("+="), None, Some("+=")),
    token_info!(MinusAssign, "'-='", Operator, Some("-="), None, Some("-=")),
    token_info!(StarAssign, "'*='", Operator, Some("*="), None, Some("*=")),
    token_info!(SlashAssign, "'/='", Operator, Some("/="), None, Some("/=")),
    token_info!(PercentAssign, "'%='", Operator, Some("%="), None, Some("%=")),
    token_info!(DotAssign, "'.='", Operator, Some(".="), None, Some(".=")),
    token_info!(AndAssign, "'&='", Operator, Some("&="), None, Some("&=")),
    token_info!(OrAssign, "'|='", Operator, Some("|="), None, Some("|=")),
    token_info!(XorAssign, "'^='", Operator, Some("^="), None, Some("^=")),
    token_info!(PowerAssign, "'**='", Operator, Some("**="), None, Some("**=")),
    token_info!(LeftShiftAssign, "'<<='", Operator, Some("<<="), None, Some("<<=")),
    token_info!(RightShiftAssign, "'>>='", Operator, Some(">>="), None, Some(">>=")),
    token_info!(LogicalAndAssign, "'&&='", Operator, Some("&&="), None, Some("&&=")),
    token_info!(LogicalOrAssign, "'||='", Operator, Some("||="), None, Some("||=")),
    token_info!(DefinedOrAssign, "'//='", Operator, Some("//="), None, Some("//=")),
    token_info!(Equal, "'=='", Operator, Some("=="), None, Some("==")),
    token_info!(NotEqual, "'!='", Operator, Some("!="), None, Some("!=")),
    token_info!(Match, "'=~'", Operator, Some("=~"), None, Some("=~")),
    token_info!(NotMatch, "'!~'", Operator, Some("!~"), None, Some("!~")),
    token_info!(SmartMatch, "'~~'", Operator, Some("~~"), None, Some("~~")),
    token_info!(Less, "'<'", Operator, Some("<"), None, Some("<")),
    token_info!(Greater, "'>'", Operator, Some(">"), None, Some(">")),
    token_info!(LessEqual, "'<='", Operator, Some("<="), None, Some("<=")),
    token_info!(GreaterEqual, "'>='", Operator, Some(">="), None, Some(">=")),
    token_info!(Spaceship, "'<=>'", Operator, Some("<=>"), None, Some("<=>")),
    token_info!(StringCompare, "'cmp'", Operator, Some("cmp"), None, Some("cmp")),
    token_info!(And, "'&&'", Operator, Some("&&"), None, Some("&&")),
    token_info!(Or, "'||'", Operator, Some("||"), None, Some("||")),
    token_info!(Not, "'!'", Operator, Some("!"), None, Some("!")),
    token_info!(DefinedOr, "'//'", Operator, Some("//"), None, Some("//")),
    token_info!(WordAnd, "'and'", Operator, Some("and"), None, Some("and")),
    token_info!(WordOr, "'or'", Operator, Some("or"), None, Some("or")),
    token_info!(WordNot, "'not'", Operator, Some("not"), None, Some("not")),
    token_info!(WordXor, "'xor'", Operator, Some("xor"), None, Some("xor")),
    token_info!(Arrow, "'->'", Operator, Some("->"), None, Some("->")),
    token_info!(FatArrow, "'=>'", Operator, Some("=>"), None, Some("=>")),
    token_info!(Dot, "'.'", Operator, Some("."), None, Some(".")),
    token_info!(Range, "'..'", Operator, Some(".."), None, Some("..")),
    token_info!(Ellipsis, "'...'", Operator, Some("..."), None, Some("...")),
    token_info!(Increment, "'++'", Operator, Some("++"), None, Some("++")),
    token_info!(Decrement, "'--'", Operator, Some("--"), None, Some("--")),
    token_info!(DoubleColon, "'::'", Operator, Some("::"), None, Some("::")),
    token_info!(Question, "'?'", Operator, Some("?"), None, Some("?")),
    token_info!(Colon, "':'", Operator, Some(":"), None, Some(":")),
    token_info!(Backslash, "'\\'", Operator, Some("\\"), None, Some("\\")),
    // Delimiters
    token_info!(LeftParen, "'('", Delimiter, Some("("), None, None),
    token_info!(RightParen, "')'", Delimiter, Some(")"), None, None),
    token_info!(LeftBrace, "'{'", Delimiter, Some("{"), None, None),
    token_info!(RightBrace, "'}'", Delimiter, Some("}"), None, None),
    token_info!(LeftBracket, "'['", Delimiter, Some("["), None, None),
    token_info!(RightBracket, "']'", Delimiter, Some("]"), None, None),
    token_info!(Semicolon, "';'", Delimiter, Some(";"), None, None),
    token_info!(Comma, "','", Delimiter, Some(","), None, None),
    // Literals
    token_info!(Number, "number", Literal, None, None, None),
    token_info!(String, "string", Literal, None, None, None),
    token_info!(Regex, "regex", Literal, None, None, None),
    token_info!(Substitution, "substitution (s///)", Literal, None, None, None),
    token_info!(Transliteration, "transliteration (tr///)", Literal, None, None, None),
    token_info!(QuoteSingle, "q// string", Literal, None, None, None),
    token_info!(QuoteDouble, "qq// string", Literal, None, None, None),
    token_info!(QuoteWords, "qw() word list", Literal, None, None, None),
    token_info!(QuoteCommand, "qx// command", Literal, None, None, None),
    token_info!(HeredocStart, "heredoc (<<)", Literal, None, None, None),
    token_info!(HeredocBody, "heredoc body", Literal, None, None, None),
    token_info!(FormatBody, "format body", Literal, None, None, None),
    token_info!(DataMarker, "__DATA__", Literal, Some("__DATA__"), None, None),
    token_info!(DataBody, "data section", Literal, None, None, None),
    token_info!(VString, "version string", Literal, None, None, None),
    token_info!(UnknownRest, "unparsed content", Literal, None, None, None),
    token_info!(HeredocDepthLimit, "heredoc depth limit", Literal, None, None, None),
    // Identifiers and sigils
    token_info!(Identifier, "identifier", Identifier, None, None, None),
    token_info!(ScalarSigil, "'$'", Sigil, Some("$"), None, None),
    token_info!(ArraySigil, "'@'", Sigil, Some("@"), None, None),
    token_info!(HashSigil, "'%'", Sigil, Some("%"), None, None),
    token_info!(SubSigil, "'&'", Sigil, Some("&"), None, None),
    token_info!(GlobSigil, "'*'", Sigil, Some("*"), None, None),
    // Special
    token_info!(Eof, "end of input", Special, None, None, None),
    token_info!(Unknown, "unknown token", Special, None, None, None),
];

impl TokenKind {
    pub const VARIANT_COUNT: usize = 132;

    pub const fn all() -> &'static [TokenKind] {
        const KINDS: &[TokenKind] = &[
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

        KINDS
    }

    const fn table_index(self) -> usize {
        match self {
            TokenKind::My => 0,
            TokenKind::Our => 1,
            TokenKind::Local => 2,
            TokenKind::State => 3,
            TokenKind::Sub => 4,
            TokenKind::If => 5,
            TokenKind::Elsif => 6,
            TokenKind::Else => 7,
            TokenKind::Unless => 8,
            TokenKind::While => 9,
            TokenKind::Until => 10,
            TokenKind::For => 11,
            TokenKind::Foreach => 12,
            TokenKind::Return => 13,
            TokenKind::Package => 14,
            TokenKind::Use => 15,
            TokenKind::No => 16,
            TokenKind::Begin => 17,
            TokenKind::End => 18,
            TokenKind::Check => 19,
            TokenKind::Init => 20,
            TokenKind::Unitcheck => 21,
            TokenKind::Eval => 22,
            TokenKind::Do => 23,
            TokenKind::Given => 24,
            TokenKind::When => 25,
            TokenKind::Default => 26,
            TokenKind::Try => 27,
            TokenKind::Catch => 28,
            TokenKind::Finally => 29,
            TokenKind::Continue => 30,
            TokenKind::Next => 31,
            TokenKind::Last => 32,
            TokenKind::Redo => 33,
            TokenKind::Goto => 34,
            TokenKind::Class => 35,
            TokenKind::Method => 36,
            TokenKind::Field => 37,
            TokenKind::Format => 38,
            TokenKind::Undef => 39,
            TokenKind::Defer => 40,
            TokenKind::Assign => 41,
            TokenKind::Plus => 42,
            TokenKind::Minus => 43,
            TokenKind::Star => 44,
            TokenKind::Slash => 45,
            TokenKind::Percent => 46,
            TokenKind::Power => 47,
            TokenKind::LeftShift => 48,
            TokenKind::RightShift => 49,
            TokenKind::BitwiseAnd => 50,
            TokenKind::BitwiseOr => 51,
            TokenKind::BitwiseXor => 52,
            TokenKind::BitwiseNot => 53,
            TokenKind::PlusAssign => 54,
            TokenKind::MinusAssign => 55,
            TokenKind::StarAssign => 56,
            TokenKind::SlashAssign => 57,
            TokenKind::PercentAssign => 58,
            TokenKind::DotAssign => 59,
            TokenKind::AndAssign => 60,
            TokenKind::OrAssign => 61,
            TokenKind::XorAssign => 62,
            TokenKind::PowerAssign => 63,
            TokenKind::LeftShiftAssign => 64,
            TokenKind::RightShiftAssign => 65,
            TokenKind::LogicalAndAssign => 66,
            TokenKind::LogicalOrAssign => 67,
            TokenKind::DefinedOrAssign => 68,
            TokenKind::Equal => 69,
            TokenKind::NotEqual => 70,
            TokenKind::Match => 71,
            TokenKind::NotMatch => 72,
            TokenKind::SmartMatch => 73,
            TokenKind::Less => 74,
            TokenKind::Greater => 75,
            TokenKind::LessEqual => 76,
            TokenKind::GreaterEqual => 77,
            TokenKind::Spaceship => 78,
            TokenKind::StringCompare => 79,
            TokenKind::And => 80,
            TokenKind::Or => 81,
            TokenKind::Not => 82,
            TokenKind::DefinedOr => 83,
            TokenKind::WordAnd => 84,
            TokenKind::WordOr => 85,
            TokenKind::WordNot => 86,
            TokenKind::WordXor => 87,
            TokenKind::Arrow => 88,
            TokenKind::FatArrow => 89,
            TokenKind::Dot => 90,
            TokenKind::Range => 91,
            TokenKind::Ellipsis => 92,
            TokenKind::Increment => 93,
            TokenKind::Decrement => 94,
            TokenKind::DoubleColon => 95,
            TokenKind::Question => 96,
            TokenKind::Colon => 97,
            TokenKind::Backslash => 98,
            TokenKind::LeftParen => 99,
            TokenKind::RightParen => 100,
            TokenKind::LeftBrace => 101,
            TokenKind::RightBrace => 102,
            TokenKind::LeftBracket => 103,
            TokenKind::RightBracket => 104,
            TokenKind::Semicolon => 105,
            TokenKind::Comma => 106,
            TokenKind::Number => 107,
            TokenKind::String => 108,
            TokenKind::Regex => 109,
            TokenKind::Substitution => 110,
            TokenKind::Transliteration => 111,
            TokenKind::QuoteSingle => 112,
            TokenKind::QuoteDouble => 113,
            TokenKind::QuoteWords => 114,
            TokenKind::QuoteCommand => 115,
            TokenKind::HeredocStart => 116,
            TokenKind::HeredocBody => 117,
            TokenKind::FormatBody => 118,
            TokenKind::DataMarker => 119,
            TokenKind::DataBody => 120,
            TokenKind::VString => 121,
            TokenKind::UnknownRest => 122,
            TokenKind::HeredocDepthLimit => 123,
            TokenKind::Identifier => 124,
            TokenKind::ScalarSigil => 125,
            TokenKind::ArraySigil => 126,
            TokenKind::HashSigil => 127,
            TokenKind::SubSigil => 128,
            TokenKind::GlobSigil => 129,
            TokenKind::Eof => 130,
            TokenKind::Unknown => 131,
        }
    }

    pub fn info(self) -> &'static TokenKindInfo {
        &TOKEN_KIND_INFO[self.table_index()]
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

const _: [(); TokenKind::VARIANT_COUNT] = [(); TOKEN_KIND_INFO.len()];
const _: [(); TokenKind::VARIANT_COUNT] = [(); TokenKind::all().len()];
