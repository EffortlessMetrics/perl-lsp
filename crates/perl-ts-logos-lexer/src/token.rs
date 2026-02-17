use logos::Logos;
use std::fmt;

/// Token types for the Perl lexer
#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t]+")]  // Skip whitespace by default
pub enum Token {
    // Literals
    #[regex(r"-?(?:0[xX][0-9a-fA-F_]+|0[bB][01_]+|0[0-7_]+|[0-9][0-9_]*(?:\.[0-9_]+)?(?:[eE][+-]?[0-9_]+)?)", |lex| lex.slice().to_string())]
    Number(String),
    
    #[regex(r#""([^"\\]|\\.)*""#, |lex| lex.slice().to_string())]
    #[regex(r#"'([^'\\]|\\.)*'"#, |lex| lex.slice().to_string())]
    String(String),
    
    #[regex(r"`([^`\\]|\\.)*`", |lex| lex.slice().to_string())]
    Backtick(String),
    
    // Identifiers and keywords
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| {
        let s = lex.slice();
        match s {
            // Keywords that affect parsing mode
            "if" | "elsif" | "unless" | "while" | "until" | "for" | "foreach" |
            "given" | "when" | "sub" | "do" | "eval" | "require" | "use" |
            "package" | "class" | "method" | "try" | "catch" | "finally" |
            "defer" => Token::Keyword(s.to_string()),
            
            // Reserved words
            "my" | "our" | "local" | "state" | "return" | "last" | "next" |
            "redo" | "goto" | "die" | "warn" | "print" | "say" | "chomp" |
            "chop" | "defined" | "undef" | "delete" | "exists" | "ref" |
            "bless" | "tie" | "tied" | "untie" => Token::Keyword(s.to_string()),
            
            // Special case for format
            "format" => Token::Format,
            
            // Everything else is an identifier
            _ => Token::Identifier(s.to_string()),
        }
    })]
    Identifier(String),
    
    #[token("format")]
    Format,
    
    Keyword(String),
    
    // Variables
    #[regex(r"\$[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    #[regex(r"\$\{[^}]+\}", |lex| lex.slice().to_string())]
    #[regex(r"\$[0-9]+", |lex| lex.slice().to_string())]
    #[regex(r"\$[!@#\$%^&*()_+\-=\[\]{};':\"\\|,.<>/?]", |lex| lex.slice().to_string())]
    ScalarVar(String),
    
    #[regex(r"@[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    #[regex(r"@\{[^}]+\}", |lex| lex.slice().to_string())]
    ArrayVar(String),
    
    #[regex(r"%[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    #[regex(r"%\{[^}]+\}", |lex| lex.slice().to_string())]
    HashVar(String),
    
    #[regex(r"\*[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    #[regex(r"\*\{[^}]+\}", |lex| lex.slice().to_string())]
    GlobVar(String),
    
    #[regex(r"&[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    SubCall(String),
    
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
    ModAssign,
    
    #[token("**=")]
    PowAssign,
    
    #[token(".=")]
    DotAssign,
    
    #[token("//=")]
    OrAssign,
    
    #[token("||=")]
    OrOrAssign,
    
    #[token("&&=")]
    AndAndAssign,
    
    #[token("+")]
    Plus,
    
    #[token("-")]
    Minus,
    
    #[token("*")]
    Star,
    
    // Division token - only in ExpectOperator mode
    #[token("/")]
    Slash,
    
    #[token("%")]
    Mod,
    
    #[token("**")]
    Pow,
    
    #[token(".")]
    Dot,
    
    #[token("..")]
    Range,
    
    #[token("...")]
    RangeExclusive,
    
    #[token("==")]
    NumEq,
    
    #[token("!=")]
    NumNe,
    
    #[token("<")]
    Lt,
    
    #[token(">")]
    Gt,
    
    #[token("<=")]
    Le,
    
    #[token(">=")]
    Ge,
    
    #[token("<=>")]
    Cmp,
    
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
    StrCmp,
    
    #[token("&&")]
    AndAnd,
    
    #[token("||")]
    OrOr,
    
    #[token("//")]
    DefinedOr,
    
    #[token("!")]
    Not,
    
    #[token("and")]
    And,
    
    #[token("or")]
    Or,
    
    #[token("xor")]
    Xor,
    
    #[token("not")]
    LogicalNot,
    
    #[token("~~")]
    SmartMatch,
    
    #[token("=~")]
    BindMatch,
    
    #[token("!~")]
    NotMatch,
    
    #[token("&")]
    BitAnd,
    
    #[token("|")]
    BitOr,
    
    #[token("^")]
    BitXor,
    
    #[token("~")]
    BitNot,
    
    #[token("<<")]
    LeftShift,
    
    #[token(">>")]
    RightShift,
    
    #[token("x")]
    StringRepeat,
    
    #[token("->")]
    Arrow,
    
    #[token("=>")]
    FatComma,
    
    #[token("::")]
    PackageSep,
    
    #[token("?")]
    Question,
    
    #[token(":")]
    Colon,
    
    #[token("++")]
    Incr,
    
    #[token("--")]
    Decr,
    
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
    
    #[token(",")]
    Comma,
    
    #[token(";")]
    Semicolon,
    
    // Special tokens for context-sensitive parsing
    Division,          // / in operator context
    RegexMatch,        // // or m// in term context  
    Substitution,      // s/// in term context
    Transliteration,   // tr/// or y/// in term context
    QuoteRegex,        // qr// in term context
    
    // Quote-like operators
    #[regex(r"q[qwrx]?\s*([^a-zA-Z0-9\s])", |lex| lex.slice().chars().last().unwrap_or('\0'))]
    QuoteOp(char),     // q//, qq//, qw//, qr//, qx//
    
    // Here-doc marker
    #[regex(r"<<~?\s*([A-Z_][A-Z0-9_]*)", |lex| lex.slice().to_string())]
    #[regex(r#"<<~?\s*"([^"]+)""#, |lex| lex.slice().to_string())]
    #[regex(r"<<~?\s*'([^']+)'", |lex| lex.slice().to_string())]
    HereDoc(String),
    
    // Comments and POD
    #[regex(r"#[^\n]*", |lex| lex.slice().to_string())]
    Comment(String),
    
    #[regex(r"^=\w+.*?^=cut", |lex| lex.slice().to_string())]
    Pod(String),
    
    // Newlines (important for statement boundaries)
    #[regex(r"\n+")]
    Newline,
    
    // End of file
    Eof,
    
    // Error token for unrecognized input
    #[error]
    Error,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Number(s) | Token::String(s) | Token::Backtick(s) |
            Token::Identifier(s) | Token::Keyword(s) | Token::ScalarVar(s) |
            Token::ArrayVar(s) | Token::HashVar(s) | Token::GlobVar(s) |
            Token::SubCall(s) | Token::HereDoc(s) | Token::Comment(s) |
            Token::Pod(s) => write!(f, "{}", s),
            Token::QuoteOp(c) => write!(f, "q{}", c),
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

/// Context-aware wrapper around logos lexer
pub struct PerlLexer<'a> {
    inner: logos::Lexer<'a, Token>,
    mode: LexerMode,
    peeked: Option<Token>,
}

impl<'a> PerlLexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            inner: Token::lexer(input),
            mode: LexerMode::ExpectTerm,
            peeked: None,
        }
    }
    
    /// Get the next token, handling context-sensitive cases
    pub fn next_token(&mut self) -> Option<Token> {
        if let Some(token) = self.peeked.take() {
            return Some(token);
        }
        
        let token = self.inner.next()?;
        
        // Handle context-sensitive tokens
        let token = match (&token, self.mode) {
            // Slash disambiguation
            (Token::Slash, LexerMode::ExpectTerm) => {
                // Look ahead to see if it's a regex
                self.parse_regex()
            }
            (Token::Slash, LexerMode::ExpectOperator) => {
                Token::Division
            }
            
            // Update mode based on token
            _ => token,
        };
        
        // Update mode for next token
        self.update_mode(&token);
        
        Some(token)
    }
    
    /// Parse a regex starting with /
    fn parse_regex(&mut self) -> Token {
        // This is a simplified version - real implementation would
        // need to handle regex modifiers, escaped delimiters, etc.
        Token::RegexMatch
    }
    
    /// Update lexer mode based on current token
    fn update_mode(&mut self, token: &Token) {
        use Token::*;
        
        self.mode = match token {
            // After these, we expect a term
            LeftParen | LeftBracket | LeftBrace | Comma | Semicolon |
            Arrow | FatComma | Not | LogicalNot | BitNot |
            Plus | Minus | Star | Slash | Assign | PlusAssign |
            MinusAssign | StarAssign | SlashAssign | ModAssign |
            PowAssign | DotAssign | OrAssign | OrOrAssign | AndAndAssign |
            Lt | Gt | Le | Ge | NumEq | NumNe | Cmp |
            StrLt | StrGt | StrLe | StrGe | StrEq | StrNe | StrCmp |
            AndAnd | OrOr | DefinedOr | And | Or | Xor |
            SmartMatch | BindMatch | NotMatch |
            BitAnd | BitOr | BitXor | LeftShift | RightShift |
            Question | Colon | Newline => LexerMode::ExpectTerm,
            
            // After these, we expect an operator
            RightParen | RightBracket | RightBrace |
            Identifier(_) | Number(_) | String(_) | Backtick(_) |
            ScalarVar(_) | ArrayVar(_) | HashVar(_) | GlobVar(_) |
            Incr | Decr => LexerMode::ExpectOperator,
            
            // Keywords depend on which keyword
            Keyword(k) => match k.as_str() {
                "return" | "die" | "warn" | "print" | "say" => LexerMode::ExpectTerm,
                _ => LexerMode::ExpectTerm,
            },
            
            // Default to expecting term
            _ => LexerMode::ExpectTerm,
        };
    }
    
    /// Peek at the next token without consuming it
    pub fn peek(&mut self) -> Option<&Token> {
        if self.peeked.is_none() {
            self.peeked = self.next_token();
        }
        self.peeked.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_tokens() {
        let mut lexer = PerlLexer::new("$x = 42 + 3.14");
        
        assert!(matches!(lexer.next_token(), Some(Token::ScalarVar(s)) if s == "$x"));
        assert!(matches!(lexer.next_token(), Some(Token::Assign)));
        assert!(matches!(lexer.next_token(), Some(Token::Number(s)) if s == "42"));
        assert!(matches!(lexer.next_token(), Some(Token::Plus)));
        assert!(matches!(lexer.next_token(), Some(Token::Number(s)) if s == "3.14"));
    }
    
    #[test]
    fn test_slash_disambiguation() {
        // Division context
        let mut lexer = PerlLexer::new("10 / 2");
        assert!(matches!(lexer.next_token(), Some(Token::Number(_))));
        assert!(matches!(lexer.next_token(), Some(Token::Division)));
        
        // Regex context
        let mut lexer = PerlLexer::new("if (/pattern/)");
        assert!(matches!(lexer.next_token(), Some(Token::Keyword(k)) if k == "if"));
        assert!(matches!(lexer.next_token(), Some(Token::LeftParen)));
        assert!(matches!(lexer.next_token(), Some(Token::RegexMatch)));
    }
}