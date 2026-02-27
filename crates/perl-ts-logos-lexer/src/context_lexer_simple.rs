//! Simple context-aware lexer that works with the simple_token enum
//!
//! This version is compatible with the existing simple_token definitions

use crate::regex_parser::RegexParser;
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

/// Context-aware lexer that wraps logos lexer
pub struct ContextLexer<'source> {
    source: &'source str,
    lexer: logos::Lexer<'source, Token>,
    current: Option<Token>,
    context: SlashContext,
    position: usize,
}

impl<'source> ContextLexer<'source> {
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
            | IntegerLiteral | FloatLiteral | StringLiteral | Backtick | Regex => {
                SlashContext::ExpectOperator
            }

            // Special cases
            Newline => SlashContext::ExpectOperand,

            // Keep current context for other tokens
            _ => self.context,
        };
    }

    /// Get the next token, handling slash disambiguation
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Option<Token> {
        let token = self.current.take()?;

        // Update position (start of the token we're about to return)
        self.position = self.lexer.span().start;

        // Track whether skip_to_position already set self.current
        let mut already_advanced = false;

        // Handle slash disambiguation
        let result = if token == Token::Divide {
            match self.context {
                SlashContext::ExpectOperand => {
                    let regex_result = self.parse_regex();
                    if regex_result == Token::Regex {
                        already_advanced = true;
                    }
                    regex_result
                }
                SlashContext::ExpectOperator => {
                    // It's division
                    token
                }
            }
        } else {
            token.clone()
        };

        // Update context for next token
        self.update_context(&result);

        // Advance to next token (unless skip_to_position already did it)
        if !already_advanced {
            self.current = Self::next_raw_token(&mut self.lexer);
        }

        Some(result)
    }

    /// Parse a regex literal starting after the initial /
    fn parse_regex(&mut self) -> Token {
        let mut parser = RegexParser::new(self.source, self.position);
        match parser.parse_bare_regex() {
            Ok(_construct) => {
                let new_position = parser.position();
                self.skip_to_position(new_position);
                Token::Regex
            }
            Err(_) => Token::Divide,
        }
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
    fn test_slash_as_division() {
        let mut lexer = ContextLexer::new("$x / 2");

        assert_eq!(lexer.next(), Some(Token::ScalarVar));
        assert_eq!(lexer.next(), Some(Token::Divide)); // Division
        assert_eq!(lexer.next(), Some(Token::IntegerLiteral));
    }

    #[test]
    fn test_slash_as_regex() {
        let mut lexer = ContextLexer::new("if (/test/)");

        assert_eq!(lexer.next(), Some(Token::If));
        assert_eq!(lexer.next(), Some(Token::LParen));
        assert_eq!(lexer.next(), Some(Token::Regex)); // Regex
        assert_eq!(lexer.next(), Some(Token::RParen));
    }

    #[test]
    fn test_regex_with_modifiers() {
        let mut lexer = ContextLexer::new("=~ /pattern/gi");

        assert_eq!(lexer.next(), Some(Token::BinMatch));
        assert_eq!(lexer.next(), Some(Token::Regex));
    }

    #[test]
    fn test_complex_slash_disambiguation() {
        let mut lexer = ContextLexer::new("$x = 10 / 2 + /test/");

        assert_eq!(lexer.next(), Some(Token::ScalarVar));
        assert_eq!(lexer.next(), Some(Token::Assign));
        assert_eq!(lexer.next(), Some(Token::IntegerLiteral));
        assert_eq!(lexer.next(), Some(Token::Divide)); // Division
        assert_eq!(lexer.next(), Some(Token::IntegerLiteral));
        assert_eq!(lexer.next(), Some(Token::Plus));
        assert_eq!(lexer.next(), Some(Token::Regex)); // Regex
    }
}
