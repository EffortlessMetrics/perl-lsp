//! Simplified token enum that compiles with logos
//!
//! This is a cleaner approach that avoids the callback issues

use logos::Logos;

#[derive(Logos, Debug, Clone, PartialEq, Eq, Hash)]
#[logos(skip r"[ \t]+")]
pub enum Token {
    // Numbers
    #[regex(r"-?0[xX][0-9a-fA-F_]+")]
    #[regex(r"-?0[bB][01_]+")]
    #[regex(r"-?0[0-7_]+")]
    #[regex(r"-?[0-9]+")]
    IntegerLiteral,

    #[regex(r"-?[0-9]*\.[0-9]+([eE][+-]?[0-9]+)?")]
    #[regex(r"-?[0-9]+[eE][+-]?[0-9]+")]
    FloatLiteral,

    // Legacy combined number token
    Number,

    // Strings
    #[regex(r#""([^"\\]|\\.)*""#)]
    #[regex(r"'([^'\\]|\\.)*'")]
    StringLiteral,

    #[regex(r"`([^`\\]|\\.)*`")]
    Backtick,

    // Legacy string token
    String,

    // Variables
    #[regex(r"\$[a-zA-Z_][a-zA-Z0-9_]*(::[a-zA-Z_][a-zA-Z0-9_]*)*", priority = 2)]
    #[regex(r"\$\{[^}]+\}", priority = 2)]
    #[regex(r"\$[0-9]+", priority = 1)]
    #[regex(r"\$[#_!@\$&*+\-.]", priority = 1)]
    ScalarVar,

    #[regex(r"@[a-zA-Z_][a-zA-Z0-9_]*(::[a-zA-Z_][a-zA-Z0-9_]*)*")]
    #[regex(r"@\{[^}]+\}")]
    ArrayVar,

    #[regex(r"%[a-zA-Z_][a-zA-Z0-9_]*(::[a-zA-Z_][a-zA-Z0-9_]*)*")]
    #[regex(r"%\{[^}]+\}")]
    HashVar,

    // Keywords
    #[token("if")]
    If,
    #[token("elsif")]
    Elsif,
    #[token("else")]
    Else,
    #[token("unless")]
    Unless,
    #[token("while")]
    While,
    #[token("until")]
    Until,
    #[token("for")]
    For,
    #[token("foreach")]
    Foreach,
    #[token("my")]
    My,
    #[token("our")]
    Our,
    #[token("local")]
    Local,
    #[token("sub")]
    Sub,
    #[token("return")]
    Return,
    #[token("package")]
    Package,
    #[token("use")]
    Use,
    #[token("require")]
    Require,
    #[token("class")]
    Class,
    #[token("method")]
    Method,
    #[token("has")]
    Has,

    // Operators
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
    PercentAssign,
    #[token(".=")]
    DotAssign,
    #[token("&=")]
    AndAssign,
    #[token("|=")]
    OrAssign,
    #[token("^=")]
    XorAssign,
    #[token("<<=")]
    LshiftAssign,
    #[token(">>=")]
    RshiftAssign,

    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Multiply,
    #[token("/")]
    Divide,
    #[token("%")]
    Modulo,
    #[token("**")]
    Power,

    #[token("==")]
    NumEq,
    #[token("!=")]
    NumNe,
    #[token("<")]
    NumLt,
    #[token(">")]
    NumGt,
    #[token("<=")]
    NumLe,
    #[token(">=")]
    NumGe,
    #[token("<=>")]
    Spaceship,

    #[token("eq")]
    StrEq,
    #[token("ne")]
    StrNe,
    #[token("lt")]
    StrLt,
    #[token("gt")]
    StrGt,
    #[token("le")]
    StrLe,
    #[token("ge")]
    StrGe,
    #[token("cmp")]
    Cmp,

    #[token("isa")]
    Isa,

    #[token("&&")]
    LogAnd,
    #[token("||")]
    LogOr,
    #[token("!")]
    Not,
    #[token("&")]
    BitAnd,
    #[token("|")]
    BitOr,
    #[token("^")]
    BitXor,
    #[token("~")]
    BitNot,

    #[token("=~")]
    BinMatch,
    #[token("!~")]
    BinNotMatch,

    #[token("->")]
    Arrow,
    #[token("=>")]
    FatArrow,

    // Delimiters
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,

    // Other
    #[token(";")]
    Semicolon,
    #[token(",")]
    Comma,
    #[token(".")]
    Dot,
    #[token("..")]
    Range,
    #[token("...")]
    Ellipsis,
    #[token(":")]
    Colon,
    #[token("?")]
    Question,

    #[regex(r"\n")]
    Newline,

    #[regex(r"#[^\n]*", allow_greedy = true)]
    Comment,

    // Identifiers (must be after keywords)
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*")]
    Identifier,

    // Barewords (same pattern but treated differently)
    Bareword,

    // Special tokens
    // Note: Regex is not defined here - it's handled by context-aware lexing
    Regex,

    // More keywords
    #[token("state")]
    State,

    // EOF
    Eof,

    // Error (logos 0.13+ doesn't need #[error] attribute)
    Error,
}

/// Context-aware Perl lexer
pub struct PerlLexer<'source> {
    lexer: logos::Lexer<'source, Token>,
    peeked: Option<Token>,
}

impl<'source> PerlLexer<'source> {
    pub fn new(input: &'source str) -> Self {
        Self { lexer: Token::lexer(input), peeked: None }
    }

    pub fn next_token(&mut self) -> Token {
        if let Some(token) = self.peeked.take() {
            return token;
        }

        match self.lexer.next() {
            Some(Ok(token)) => token,
            _ => Token::Eof,
        }
    }

    pub fn peek(&mut self) -> &Token {
        if self.peeked.is_none() {
            self.peeked = Some(self.next_token());
        }
        match &self.peeked {
            Some(token) => token,
            None => unreachable!("peeked should be Some after next_token call"),
        }
    }

    pub fn span(&self) -> logos::Span {
        self.lexer.span()
    }

    pub fn slice(&self) -> &'source str {
        self.lexer.slice()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_lexing() {
        let input = "my $x = 42;";
        let mut lexer = PerlLexer::new(input);

        assert_eq!(lexer.next_token(), Token::My);
        assert_eq!(lexer.next_token(), Token::ScalarVar);
        assert_eq!(lexer.next_token(), Token::Assign);
        assert_eq!(lexer.next_token(), Token::IntegerLiteral);
        assert_eq!(lexer.next_token(), Token::Semicolon);
        assert_eq!(lexer.next_token(), Token::Eof);
    }

    #[test]
    fn test_operators() {
        let input = "$a + $b * $c";
        let mut lexer = PerlLexer::new(input);

        assert_eq!(lexer.next_token(), Token::ScalarVar);
        assert_eq!(lexer.next_token(), Token::Plus);
        assert_eq!(lexer.next_token(), Token::ScalarVar);
        assert_eq!(lexer.next_token(), Token::Multiply);
        assert_eq!(lexer.next_token(), Token::ScalarVar);
    }
}
