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
///
/// This enum is `#[non_exhaustive]`: external code must include a wildcard `_`
/// arm when matching on it. This allows new categories to be added in future
/// releases without breaking downstream crates.
#[non_exhaustive]
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
///
/// This struct is `#[non_exhaustive]`: external code must not construct it
/// using struct literal syntax. Use [`TokenKind::metadata`] to obtain
/// instances. Additional fields may be added in future releases without
/// constituting a breaking change.
#[non_exhaustive]
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

    /// Return the canonical spelling for fixed-spelling tokens.
    ///
    /// Tokens whose spelling depends on source text, such as identifiers,
    /// strings, regexes, heredocs, and recovery tokens, return `None`.
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
