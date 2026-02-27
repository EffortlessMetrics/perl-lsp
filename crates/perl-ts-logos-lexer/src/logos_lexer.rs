use logos::{Lexer, Logos};
use std::fmt;

/// Token types for the Perl lexer
#[derive(Logos, Debug, Clone, PartialEq, Eq, Hash)]
#[logos(skip r"[ \t]+")] // Skip horizontal whitespace
pub enum Token {
    // Numbers
    #[regex(r"-?0[xX][0-9a-fA-F_]+", |lex| lex.slice().to_string())]
    HexNumber(String),

    #[regex(r"-?0[bB][01_]+", |lex| lex.slice().to_string())]
    BinNumber(String),

    #[regex(r"-?0[0-7_]+", |lex| lex.slice().to_string())]
    OctNumber(String),

    #[regex(r"-?[0-9][0-9_]*\.?[0-9_]*([eE][+-]?[0-9_]+)?", |lex| lex.slice().to_string())]
    Number(String),

    // Strings (basic - full interpolation handled by parser)
    #[regex(r#""([^"\\]|\\.)*""#, |lex| lex.slice().to_string())]
    DoubleString(String),

    #[regex(r#"'([^'\\]|\\.)*'"#, |lex| lex.slice().to_string())]
    SingleString(String),

    #[regex(r"`([^`\\]|\\.)*`", |lex| lex.slice().to_string())]
    Backtick(String),

    // Barewords and identifiers
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", priority = 2, callback = |lex| {
        let slice = lex.slice();
        match slice {
            // Control flow keywords
            "if" | "elsif" | "else" | "unless" => Token::If,
            "while" | "until" => Token::While,
            "for" | "foreach" => Token::For,
            "do" => Token::Do,
            "given" => Token::Given,
            "when" => Token::When,
            "default" => Token::Default,
            "continue" => Token::Continue,
            "last" => Token::Last,
            "next" => Token::Next,
            "redo" => Token::Redo,
            "goto" => Token::Goto,
            "return" => Token::Return,

            // Declaration keywords
            "my" => Token::My,
            "our" => Token::Our,
            "local" => Token::Local,
            "state" => Token::State,
            "sub" => Token::Sub,
            "package" => Token::Package,
            "use" => Token::Use,
            "require" => Token::Require,
            "no" => Token::No,

            // Modern Perl keywords
            "try" => Token::Try,
            "catch" => Token::Catch,
            "finally" => Token::Finally,
            "defer" => Token::Defer,
            "class" => Token::Class,
            "method" => Token::Method,
            "field" => Token::Field,
            "role" => Token::Role,
            "with" => Token::With,

            // Built-in functions that affect parsing
            "print" | "say" | "die" | "warn" => Token::Print,
            "eval" => Token::Eval,
            "defined" => Token::Defined,
            "undef" => Token::Undef,
            "ref" => Token::Ref,
            "bless" => Token::Bless,
            "tie" | "tied" | "untie" => Token::Tie,
            "delete" => Token::Delete,
            "exists" => Token::Exists,
            "keys" | "values" | "each" => Token::Keys,
            "push" | "pop" | "shift" | "unshift" => Token::Push,
            "splice" => Token::Splice,
            "sort" => Token::Sort,
            "grep" => Token::Grep,
            "map" => Token::Map,
            "split" => Token::Split,
            "join" => Token::Join,
            "reverse" => Token::Reverse,

            // String operators (need special handling)
            "eq" => Token::StrEq,
            "ne" => Token::StrNe,
            "lt" => Token::StrLt,
            "gt" => Token::StrGt,
            "le" => Token::StrLe,
            "ge" => Token::StrGe,
            "cmp" => Token::StrCmp,

            // Logical operators
            "and" => Token::And,
            "or" => Token::Or,
            "xor" => Token::Xor,
            "not" => Token::Not,

            // Quote operators
            "q" => Token::Q,
            "qq" => Token::Qq,
            "qw" => Token::Qw,
            "qx" => Token::Qx,
            "qr" => Token::Qr,
            "m" => Token::M,
            "s" => Token::S,
            "tr" => Token::Tr,
            "y" => Token::Y,

            // Special keywords
            "format" => Token::Format,
            "BEGIN" => Token::Begin,
            "END" => Token::End,
            "CHECK" => Token::Check,
            "INIT" => Token::Init,
            "UNITCHECK" => Token::UnitCheck,
            "DESTROY" => Token::Destroy,
            "AUTOLOAD" => Token::AutoLoad,

            // ISA operator
            "ISA" => Token::Isa,

            // Everything else is an identifier
            _ => Token::Identifier(slice.to_string()),
        }
    })]
    Identifier(String),

    // Keywords (as separate tokens for easier parsing)
    If,
    Elsif,
    Else,
    Unless,
    While,
    Until,
    For,
    Foreach,
    Do,
    Given,
    When,
    Default,
    Continue,
    Last,
    Next,
    Redo,
    Goto,
    Return,
    My,
    Our,
    Local,
    State,
    Sub,
    Package,
    Use,
    Require,
    No,
    Try,
    Catch,
    Finally,
    Defer,
    Class,
    Method,
    Field,
    Role,
    With,
    Print,
    Say,
    Die,
    Warn,
    Eval,
    Defined,
    Undef,
    Ref,
    Bless,
    Tie,
    Tied,
    Untie,
    Delete,
    Exists,
    Keys,
    Values,
    Each,
    Push,
    Pop,
    Shift,
    Unshift,
    Splice,
    Sort,
    Grep,
    Map,
    Split,
    Join,
    Reverse,
    And,
    Or,
    Xor,
    Not,
    Q,
    Qq,
    Qw,
    Qx,
    Qr,
    M,
    S,
    Tr,
    Y,
    Format,
    Begin,
    End,
    Check,
    Init,
    UnitCheck,
    Destroy,
    AutoLoad,
    Isa,

    // Variables
    #[regex(r"\$[a-zA-Z_][a-zA-Z0-9_]*(::[a-zA-Z_][a-zA-Z0-9_]*)*", priority = 3, callback = |lex| lex.slice().to_string())]
    #[regex(r#"\$\{[^}]+\}"#, priority = 3, callback = |lex| lex.slice().to_string())]
    #[regex(r"\$[0-9]+", priority = 2, callback = |lex| lex.slice().to_string())]
    #[regex(r#"\$[!@#$%^&*()_+\-=\[\]{};':"|,.<>/?`~\\]"#, priority = 2, callback = |lex| lex.slice().to_string())]
    ScalarVar(String),

    #[regex(r"@[a-zA-Z_][a-zA-Z0-9_]*(::[a-zA-Z_][a-zA-Z0-9_]*)*", |lex| lex.slice().to_string())]
    #[regex(r#"@\{[^}]+\}"#, |lex| lex.slice().to_string())]
    ArrayVar(String),

    #[regex(r"%[a-zA-Z_][a-zA-Z0-9_]*(::[a-zA-Z_][a-zA-Z0-9_]*)*", |lex| lex.slice().to_string())]
    #[regex(r#"%\{[^}]+\}"#, |lex| lex.slice().to_string())]
    HashVar(String),

    #[regex(r"\*[a-zA-Z_][a-zA-Z0-9_]*(::[a-zA-Z_][a-zA-Z0-9_]*)*", |lex| lex.slice().to_string())]
    #[regex(r#"\*\{[^}]+\}"#, |lex| lex.slice().to_string())]
    GlobVar(String),

    #[regex(r"&[a-zA-Z_][a-zA-Z0-9_]*(::[a-zA-Z_][a-zA-Z0-9_]*)*", |lex| lex.slice().to_string())]
    SubCall(String),

    // Operators (in precedence order groups)
    #[token("->")]
    Arrow,

    #[token("++")]
    Incr,

    #[token("--")]
    Decr,

    #[token("**")]
    Pow,

    #[token("!")]
    Bang,

    #[token("~")]
    BitNot,

    #[token("\\")]
    Backslash,

    #[token("+", priority = 2)]
    UnaryPlus,

    #[token("-", priority = 2)]
    UnaryMinus,

    #[token("=~")]
    BindMatch,

    #[token("!~")]
    NotMatch,

    #[token("*")]
    Star,

    #[token("/")]
    Slash,

    #[token("%")]
    Mod,

    #[token("x", priority = 3)]
    StringRepeat,

    #[token("+", priority = 1)]
    Plus,

    #[token("-", priority = 1)]
    Minus,

    #[token(".")]
    Dot,

    #[token("<<")]
    LeftShift,

    #[token(">>")]
    RightShift,

    #[token("<")]
    Lt,

    #[token(">")]
    Gt,

    #[token("<=")]
    Le,

    #[token(">=")]
    Ge,

    // String comparison operators handled above as keywords
    StrLt,
    StrGt,
    StrLe,
    StrGe,
    StrEq,
    StrNe,
    StrCmp,

    #[token("==")]
    NumEq,

    #[token("!=")]
    NumNe,

    #[token("<=>")]
    Cmp,

    #[token("~~")]
    SmartMatch,

    #[token("&")]
    BitAnd,

    #[token("|")]
    BitOr,

    #[token("^")]
    BitXor,

    #[token("&&")]
    AndAnd,

    #[token("||")]
    OrOr,

    #[token("//")]
    DefinedOr,

    #[token("..")]
    Range,

    #[token("...")]
    RangeExclusive,

    #[token("?")]
    Question,

    #[token(":")]
    Colon,

    #[token("=")]
    Assign,

    #[token("+=")]
    PlusAssign,

    #[token("-=")]
    MinusAssign,

    #[token("*=")]
    StarAssign,

    #[token("/=")]
    SlashAssign,

    #[token("%=")]
    ModAssign,

    #[token("**=")]
    PowAssign,

    #[token(".=")]
    DotAssign,

    #[token("x=")]
    RepeatAssign,

    #[token("&=")]
    BitAndAssign,

    #[token("|=")]
    BitOrAssign,

    #[token("^=")]
    BitXorAssign,

    #[token("<<=")]
    LeftShiftAssign,

    #[token(">>=")]
    RightShiftAssign,

    #[token("&&=")]
    AndAndAssign,

    #[token("||=")]
    OrOrAssign,

    #[token("//=")]
    DefinedOrAssign,

    #[token(",")]
    Comma,

    #[token("=>")]
    FatComma,

    // Delimiters
    #[token("(")]
    LeftParen,

    #[token(")")]
    RightParen,

    #[token("[")]
    LeftBracket,

    #[token("]")]
    RightBracket,

    #[token("{")]
    LeftBrace,

    #[token("}")]
    RightBrace,

    #[token(";")]
    Semicolon,

    #[token("::")]
    PackageSep,

    #[token("'")] // For package separator Foo'Bar
    PackageQuote,

    // Here-doc starters
    #[regex(r"<<~?\s*([A-Z_][A-Z0-9_]*)", |lex| lex.slice().to_string())]
    #[regex(r#"<<~?\s*"([^"]+)""#, |lex| lex.slice().to_string())]
    #[regex(r"<<~?\s*'([^']+)'", |lex| lex.slice().to_string())]
    #[regex(r"<<~?\s*`([^`]+)`", |lex| lex.slice().to_string())]
    HereDocStart(String),

    // Comments and POD
    #[regex(r"#[^\n]*")]
    Comment,

    #[regex(r"=[a-zA-Z][^\n]*", |lex| Token::PodStart)]
    PodStart,

    // Newlines (important for statement boundaries and heredocs)
    #[regex(r"\n+")]
    Newline,

    // End of file
    Eof,

    // Error token
    Error,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Token::*;
        match self {
            Number(s) | HexNumber(s) | BinNumber(s) | OctNumber(s) | DoubleString(s)
            | SingleString(s) | Backtick(s) | Identifier(s) | ScalarVar(s) | ArrayVar(s)
            | HashVar(s) | GlobVar(s) | SubCall(s) | HereDocStart(s) => {
                write!(f, "{}", s)
            }
            _ => write!(f, "{:?}", self),
        }
    }
}

/// Lexer mode for context-sensitive parsing
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LexerMode {
    /// Expecting a term (value) - slash starts a regex
    ExpectTerm,
    /// Expecting an operator - slash is division
    ExpectOperator,
}

/// Context-aware Perl lexer
pub struct PerlLexer<'source> {
    inner: Lexer<'source, Token>,
    mode: LexerMode,
    peeked: Option<(Token, std::ops::Range<usize>)>,
    source: &'source str,
}

impl<'source> PerlLexer<'source> {
    pub fn new(source: &'source str) -> Self {
        Self { inner: Token::lexer(source), mode: LexerMode::ExpectTerm, peeked: None, source }
    }

    /// Get the next token with position information
    pub fn next(&mut self) -> Option<(Token, std::ops::Range<usize>)> {
        if let Some(peeked) = self.peeked.take() {
            self.update_mode(&peeked.0);
            return Some(peeked);
        }

        let token = self.inner.next()?;
        let span = self.inner.span();

        // Handle context-sensitive tokens
        let token = match (&token, self.mode) {
            // Slash disambiguation
            (Token::Slash, LexerMode::ExpectTerm) => {
                // This is a regex - parse it properly
                self.parse_regex_at(span.start)
            }
            _ => token,
        };

        // Handle other context-sensitive cases
        let token = match &token {
            // Quote-like operators need special handling
            Token::M
            | Token::S
            | Token::Tr
            | Token::Y
            | Token::Q
            | Token::Qq
            | Token::Qw
            | Token::Qx
            | Token::Qr => self.parse_quote_like(token.clone(), span.end),
            _ => token,
        };

        self.update_mode(&token);
        Some((token, span))
    }

    /// Parse a regex starting at the given position
    fn parse_regex_at(&mut self, start: usize) -> Token {
        // In a real implementation, this would:
        // 1. Parse the regex pattern with proper delimiter handling
        // 2. Parse any modifiers (i, m, s, x, etc.)
        // 3. Return a proper regex token

        // For now, simplified:
        Token::M // Treat bare // as m//
    }

    /// Parse quote-like operators (q//, qq//, etc.)
    fn parse_quote_like(&mut self, op: Token, pos: usize) -> Token {
        // In a real implementation, this would:
        // 1. Skip whitespace after the operator
        // 2. Find the delimiter
        // 3. Parse the contents respecting that delimiter
        // 4. Handle balanced delimiters like q{} vs q//

        // For now, return the operator as-is
        op
    }

    /// Update lexer mode based on the current token
    fn update_mode(&mut self, token: &Token) {
        use Token::*;

        self.mode = match token {
            // After these, we expect a term
            LeftParen | LeftBracket | LeftBrace | Comma | Semicolon | Arrow | FatComma | Bang
            | BitNot | UnaryPlus | UnaryMinus | Assign | PlusAssign | MinusAssign | StarAssign
            | SlashAssign | ModAssign | PowAssign | DotAssign | RepeatAssign | BitAndAssign
            | BitOrAssign | BitXorAssign | LeftShiftAssign | RightShiftAssign | AndAndAssign
            | OrOrAssign | DefinedOrAssign | Lt | Gt | Le | Ge | NumEq | NumNe | Cmp | StrLt
            | StrGt | StrLe | StrGe | StrEq | StrNe | StrCmp | AndAnd | OrOr | DefinedOr | And
            | Or | Xor | Not | SmartMatch | BindMatch | NotMatch | BitAnd | BitOr | BitXor
            | LeftShift | RightShift | Question | Colon | Newline | If | Elsif | Unless | While
            | Until | For | Foreach | Return | Print | Say | Die | Warn => LexerMode::ExpectTerm,

            // After these, we expect an operator
            RightParen | RightBracket | RightBrace | Identifier(_) | Number(_) | HexNumber(_)
            | BinNumber(_) | OctNumber(_) | DoubleString(_) | SingleString(_) | Backtick(_)
            | ScalarVar(_) | ArrayVar(_) | HashVar(_) | GlobVar(_) | Incr | Decr => {
                LexerMode::ExpectOperator
            }

            // Default to expecting term
            _ => LexerMode::ExpectTerm,
        };
    }

    /// Peek at the next token without consuming it
    pub fn peek(&mut self) -> Option<&Token> {
        if self.peeked.is_none() {
            self.peeked = self.next();
        }
        self.peeked.as_ref().map(|(token, _)| token)
    }

    /// Get the source text for a span
    pub fn span_text(&self, span: &std::ops::Range<usize>) -> &str {
        &self.source[span.clone()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_lexing() {
        use perl_tdd_support::must_some;
        let mut lexer = PerlLexer::new("$x = 42");

        let (token, _) = must_some(lexer.next());
        assert!(matches!(token, Token::ScalarVar(s) if s == "$x"));

        let (token, _) = must_some(lexer.next());
        assert!(matches!(token, Token::Assign));

        let (token, _) = must_some(lexer.next());
        assert!(matches!(token, Token::Number(s) if s == "42"));
    }

    #[test]
    fn test_keywords() {
        use perl_tdd_support::must_some;
        let mut lexer = PerlLexer::new("if my $x");

        assert!(matches!(must_some(lexer.next()).0, Token::If));
        assert!(matches!(must_some(lexer.next()).0, Token::My));
        assert!(matches!(must_some(lexer.next()).0, Token::ScalarVar(_)));
    }
}
