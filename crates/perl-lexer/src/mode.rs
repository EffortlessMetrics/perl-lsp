//! Lexer modes for context-sensitive parsing

/// Perl lexer mode to disambiguate slash tokens
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LexerMode {
    /// Expecting a term (value) - slash starts a regex
    ExpectTerm,
    /// Expecting an operator - slash is division
    ExpectOperator,
}

impl LexerMode {
    /// Check if we're expecting a term
    pub fn is_expect_term(&self) -> bool {
        matches!(self, LexerMode::ExpectTerm)
    }
    
    /// Check if we're expecting an operator
    pub fn is_expect_operator(&self) -> bool {
        matches!(self, LexerMode::ExpectOperator)
    }
}

impl Default for LexerMode {
    fn default() -> Self {
        LexerMode::ExpectTerm
    }
}