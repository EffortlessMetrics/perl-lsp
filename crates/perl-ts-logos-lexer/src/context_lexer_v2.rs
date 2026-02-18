//! Enhanced context-aware lexer with proper regex handling
//!
//! This version properly parses regex constructs and advances the lexer

use crate::regex_parser::{QuoteConstruct, RegexParser};
use crate::simple_token::Token;
use logos::Logos;

/// Context for disambiguating slash tokens
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SlashContext {
    /// Expecting an operand (slash means regex)
    ExpectOperand,
    /// Expecting an operator (slash means division)
    ExpectOperator,
}

/// Enhanced token that can carry additional data
#[derive(Debug, Clone, PartialEq)]
pub enum EnhancedToken {
    /// Simple token
    Simple(Token),
    /// Regex literal with parsed content
    Regex(QuoteConstruct),
    /// Match operator (m//)
    MatchOp(QuoteConstruct),
    /// Substitution operator (s///)
    SubstituteOp(QuoteConstruct),
}

/// Context-aware lexer that properly handles regex constructs
pub struct ContextLexerV2<'source> {
    source: &'source str,
    lexer: logos::Lexer<'source, Token>,
    current: Option<Token>,
    context: SlashContext,
    position: usize,
}

impl<'source> ContextLexerV2<'source> {
    pub fn new(source: &'source str) -> Self {
        let mut lexer = Token::lexer(source);
        let current = Self::next_raw_token(&mut lexer);

        Self { source, lexer, current, context: SlashContext::ExpectOperand, position: 0 }
    }

    /// Get next raw token from logos lexer
    fn next_raw_token(lexer: &mut logos::Lexer<'source, Token>) -> Option<Token> {
        match lexer.next() {
            Some(Ok(token)) => Some(token),
            _ => None,
        }
    }

    /// Update context based on token
    fn update_context(&mut self, token: &Token) {
        use Token::*;

        self.context = match token {
            // After these tokens, we expect an operand (so / is regex)
            LParen | LBracket | LBrace | Comma | Semicolon | Arrow | Plus | Minus | Multiply
            | Divide | Modulo | Power | NumEq | NumNe | NumLt | NumGt | StrEq | StrNe | LogAnd
            | LogOr | Not | Assign | PlusAssign | MinusAssign | StarAssign | SlashAssign | If
            | Unless | While | Until | For | Foreach | My | Our | Local | Sub | Return
            | BinMatch | BinNotMatch => SlashContext::ExpectOperand,

            // After these tokens, we expect an operator (so / is division)
            RParen | RBracket | RBrace | Identifier | ScalarVar | ArrayVar | HashVar
            | IntegerLiteral | FloatLiteral | StringLiteral | Backtick | Bareword | Regex => {
                SlashContext::ExpectOperator
            }

            // Special cases
            Newline => SlashContext::ExpectOperand,

            // Keep current context for other tokens
            _ => self.context,
        };
    }

    /// Get the next token, handling special constructs
    pub fn next_enhanced(&mut self) -> Option<EnhancedToken> {
        let token = self.current.take()?;
        self.position = self.lexer.span().start;

        // Handle special tokens
        let result = match token {
            Token::Divide => {
                match self.context {
                    SlashContext::ExpectOperand => {
                        // Parse as regex
                        match self.parse_bare_regex() {
                            Ok(construct) => EnhancedToken::Regex(construct),
                            Err(_) => EnhancedToken::Simple(Token::Divide),
                        }
                    }
                    SlashContext::ExpectOperator => {
                        // It's division
                        EnhancedToken::Simple(Token::Divide)
                    }
                }
            }
            // Handle m// operator
            Token::Identifier
                if self.source[self.position..].starts_with("m")
                    && self.is_quote_operator_start(self.position + 1) =>
            {
                self.parse_match_operator()
            }
            // Handle s/// operator
            Token::Identifier
                if self.source[self.position..].starts_with("s")
                    && self.is_quote_operator_start(self.position + 1) =>
            {
                self.parse_substitute_operator()
            }
            _ => EnhancedToken::Simple(token.clone()),
        };

        // Update context based on the original token
        self.update_context(&token);

        // Advance to next token
        self.current = Self::next_raw_token(&mut self.lexer);

        Some(result)
    }

    /// Get the next token as a simple token
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Option<Token> {
        match self.next_enhanced() {
            Some(EnhancedToken::Simple(token)) => Some(token),
            Some(EnhancedToken::Regex(_)) => Some(Token::Regex),
            Some(EnhancedToken::MatchOp(_)) => Some(Token::Regex),
            Some(EnhancedToken::SubstituteOp(_)) => Some(Token::Regex),
            None => None,
        }
    }

    /// Parse a bare regex starting with /
    fn parse_bare_regex(&mut self) -> Result<QuoteConstruct, String> {
        let mut parser = RegexParser::new(self.source, self.position);
        let result = parser.parse_bare_regex()?;

        // Advance our position to after the regex
        let new_position = parser.position();
        self.skip_to_position(new_position);

        Ok(result)
    }

    /// Parse m// operator
    fn parse_match_operator(&mut self) -> EnhancedToken {
        // Skip the 'm' identifier token
        self.current = Self::next_raw_token(&mut self.lexer);
        self.position = self.lexer.span().start;

        let mut parser = RegexParser::new(self.source, self.position);
        match parser.parse_match_operator() {
            Ok(construct) => {
                let new_position = parser.position();
                self.skip_to_position(new_position);
                EnhancedToken::MatchOp(construct)
            }
            Err(_) => EnhancedToken::Simple(Token::Identifier),
        }
    }

    /// Parse s/// operator
    fn parse_substitute_operator(&mut self) -> EnhancedToken {
        // Skip the 's' identifier token
        self.current = Self::next_raw_token(&mut self.lexer);
        self.position = self.lexer.span().start;

        let mut parser = RegexParser::new(self.source, self.position);
        match parser.parse_substitute_operator() {
            Ok(construct) => {
                let new_position = parser.position();
                self.skip_to_position(new_position);
                EnhancedToken::SubstituteOp(construct)
            }
            Err(_) => EnhancedToken::Simple(Token::Identifier),
        }
    }

    /// Check if position starts a quote operator
    fn is_quote_operator_start(&self, position: usize) -> bool {
        if position >= self.source.len() {
            return false;
        }

        let ch = self.source.chars().nth(position).unwrap_or('\0');
        matches!(
            ch,
            '/' | '{'
                | '('
                | '['
                | '<'
                | '!'
                | '#'
                | '|'
                | '~'
                | '@'
                | '$'
                | '%'
                | '^'
                | '&'
                | '*'
                | '-'
                | '_'
                | '+'
                | '='
                | '\\'
                | ':'
                | ';'
                | '"'
                | '\''
                | ','
                | '.'
                | '?'
                | '`'
        )
    }

    /// Skip lexer to a specific position
    fn skip_to_position(&mut self, target_position: usize) {
        while self.lexer.span().end < target_position {
            if Self::next_raw_token(&mut self.lexer).is_none() {
                break;
            }
        }
        self.position = target_position;
        self.current = Self::next_raw_token(&mut self.lexer);
    }

    pub fn peek(&self) -> Option<&Token> {
        self.current.as_ref()
    }

    pub fn span(&self) -> std::ops::Range<usize> {
        self.lexer.span()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bare_regex_parsing() {
        let input = "/test/i + 1";
        let mut lexer = ContextLexerV2::new(input);

        // First token should be parsed as regex
        match lexer.next_enhanced() {
            Some(EnhancedToken::Regex(construct)) => {
                assert_eq!(construct.pattern, "test");
                assert_eq!(construct.modifiers, "i");
            }
            _ => unreachable!("Expected regex token"),
        }

        // Next should be plus
        assert_eq!(lexer.next(), Some(Token::Plus));
        assert_eq!(lexer.next(), Some(Token::IntegerLiteral));
    }

    #[test]
    fn test_match_operator() {
        let input = "$x =~ m/test/gi";
        let mut lexer = ContextLexerV2::new(input);

        assert_eq!(lexer.next(), Some(Token::ScalarVar));
        assert_eq!(lexer.next(), Some(Token::BinMatch));

        // The m// should be parsed as a match operator
        match lexer.next_enhanced() {
            Some(EnhancedToken::MatchOp(construct)) => {
                assert_eq!(construct.pattern, "test");
                assert_eq!(construct.modifiers, "gi");
            }
            _ => unreachable!("Expected match operator"),
        }
    }

    #[test]
    fn test_substitute_operator() {
        let input = "s/old/new/g";
        let mut lexer = ContextLexerV2::new(input);

        match lexer.next_enhanced() {
            Some(EnhancedToken::SubstituteOp(construct)) => {
                assert_eq!(construct.pattern, "old");
                assert_eq!(construct.replacement, Some("new".to_string()));
                assert_eq!(construct.modifiers, "g");
            }
            _ => unreachable!("Expected substitute operator"),
        }
    }
}
