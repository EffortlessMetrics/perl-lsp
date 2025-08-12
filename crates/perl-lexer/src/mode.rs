//! Lexer modes for context-sensitive parsing

/// Perl lexer mode to disambiguate slash tokens
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LexerMode {
    /// Expecting a term (value) - slash starts a regex
    #[default]
    ExpectTerm,
    /// Expecting an operator - slash is division
    ExpectOperator,
    /// Expecting a delimiter for quote-like operators - # is not a comment
    ExpectDelimiter,
    /// Inside a format declaration body - consume until single dot on a line
    InFormatBody,
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
