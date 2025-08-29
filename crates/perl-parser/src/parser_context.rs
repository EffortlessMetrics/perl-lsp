//! Parser context with error recovery support
//!
//! This module provides a parsing context that tracks errors, positions,
//! and supports error recovery for IDE scenarios.

use crate::{
    ast_v2::NodeIdGenerator,
    error_recovery::ParseError,
    position::{Position, Range},
    token_wrapper::TokenWithPosition,
};
use perl_lexer::TokenType;
use std::collections::VecDeque;

/// Parser context with error tracking and recovery
pub struct ParserContext {
    /// Token stream with positions
    tokens: VecDeque<TokenWithPosition>,
    /// Current token index
    current: usize,
    /// Node ID generator
    pub id_generator: NodeIdGenerator,
    /// Accumulated parse errors
    errors: Vec<ParseError>,
    /// Source text
    source: String,
    /// Current position tracker
    _position_tracker: PositionTracker,
}

/// Tracks current position in the source
struct PositionTracker {
    _byte_offset: usize,
    _line: usize,
    _column: usize,
}

#[allow(dead_code)]
impl PositionTracker {
    fn new() -> Self {
        PositionTracker { _byte_offset: 0, _line: 1, _column: 1 }
    }

    fn current_position(&self) -> Position {
        Position::new(self._byte_offset, self._line as u32, self._column as u32)
    }

    fn advance(&mut self, text: &str) {
        for ch in text.chars() {
            self._byte_offset += ch.len_utf8();
            if ch == '\n' {
                self._line += 1;
                self._column = 1;
            } else {
                self._column += 1;
            }
        }
    }
}

impl ParserContext {
    /// Create a new parser context
    pub fn new(source: String) -> Self {
        let mut tokens = VecDeque::new();
        let mut position_tracker = PositionTracker::new();

        // Tokenize the source using mode-aware lexer
        let mut lexer = perl_lexer::PerlLexer::new(&source);
        loop {
            match lexer.next_token() {
                Some(token) => {
                    // Skip EOF tokens to avoid infinite loop
                    if matches!(token.token_type, TokenType::EOF) {
                        break;
                    }

                    let start = token.start;
                    let end = token.end;

                    if start < position_tracker._byte_offset {
                        // In case of out-of-order tokens, reset tracker
                        position_tracker = PositionTracker::new();
                    }

                    // Advance to start position
                    position_tracker.advance(&source[position_tracker._byte_offset..start]);
                    let start_pos = position_tracker.current_position();

                    // Advance to end position and record
                    position_tracker.advance(&source[start..end]);
                    let end_pos = position_tracker.current_position();

                    tokens.push_back(TokenWithPosition::new(token, start_pos, end_pos));
                }
                None => break,
            }
        }

        ParserContext {
            tokens,
            current: 0,
            id_generator: NodeIdGenerator::new(),
            errors: Vec::new(),
            source,
            _position_tracker: position_tracker,
        }
    }

    /// Get current token
    pub fn current_token(&self) -> Option<&TokenWithPosition> {
        self.tokens.get(self.current)
    }

    /// Peek at next token
    pub fn peek_token(&self, offset: usize) -> Option<&TokenWithPosition> {
        self.tokens.get(self.current + offset)
    }

    /// Advance to next token
    pub fn advance(&mut self) -> Option<&TokenWithPosition> {
        if self.current < self.tokens.len() {
            self.current += 1;
        }
        self.current_token()
    }

    /// Check if at end of tokens
    pub fn is_eof(&self) -> bool {
        self.current >= self.tokens.len()
    }

    /// Get current position
    pub fn current_position(&self) -> Position {
        if let Some(token) = self.current_token() {
            token.range().start
        } else if !self.tokens.is_empty() {
            // At EOF, use end of last token
            self.tokens.back().unwrap().range().end
        } else {
            Position::new(0, 1, 1)
        }
    }

    /// Get current position range
    pub fn current_position_range(&self) -> Range {
        if let Some(token) = self.current_token() {
            token.range()
        } else {
            let pos = self.current_position();
            Range::new(pos, pos)
        }
    }

    /// Add a parse error
    pub fn add_error(&mut self, error: ParseError) {
        self.errors.push(error);
    }

    /// Get all accumulated errors
    pub fn take_errors(&mut self) -> Vec<ParseError> {
        std::mem::take(&mut self.errors)
    }

    /// Get current token index (for saving/restoring)
    pub fn current_index(&self) -> usize {
        self.current
    }

    /// Set current token index (for restoring)
    pub fn set_index(&mut self, index: usize) {
        self.current = index.min(self.tokens.len());
    }

    /// Match and consume a specific token type
    pub fn expect(&mut self, expected: TokenType) -> Result<&TokenWithPosition, ParseError> {
        match self.current_token() {
            Some(token) if token.token.token_type == expected => {
                self.advance();
                Ok(&self.tokens[self.current - 1])
            }
            Some(token) => Err(ParseError::new(
                format!("Expected {:?}, found {:?}", expected, token.token.token_type),
                token.range(),
            )
            .with_expected(vec![format!("{:?}", expected)])
            .with_found(format!("{:?}", token.token.token_type))),
            None => Err(ParseError::new(
                format!("Expected {:?}, found end of file", expected),
                self.current_position_range(),
            )
            .with_expected(vec![format!("{:?}", expected)])
            .with_found("EOF".to_string())),
        }
    }

    /// Check if current token matches
    pub fn check(&self, token_type: &TokenType) -> bool {
        self.current_token().map(|t| &t.token.token_type == token_type).unwrap_or(false)
    }

    /// Consume token if it matches
    pub fn consume(&mut self, token_type: &TokenType) -> bool {
        if self.check(token_type) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Get source slice for a range
    pub fn source_slice(&self, range: &Range) -> &str {
        &self.source[range.start.byte..range.end.byte]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_context_creation() {
        let source = "my $x = 42;".to_string();
        let ctx = ParserContext::new(source);

        assert!(!ctx.is_eof());
        assert!(!ctx.tokens.is_empty());
    }

    #[test]
    fn test_token_advancement() {
        let source = "my $x".to_string();
        let mut ctx = ParserContext::new(source);

        // First token should be 'my'
        assert!(matches!(
            ctx.current_token().map(|t| &t.token.token_type),
            Some(TokenType::Keyword(k)) if k.as_ref() == "my"
        ));

        // Advance to next token
        ctx.advance();
        assert!(ctx.current_token().is_some());
    }

    #[test]
    fn test_error_accumulation() {
        let mut ctx = ParserContext::new("test".to_string());

        let error1 = ParseError::new("Error 1".to_string(), ctx.current_position_range());
        let error2 = ParseError::new("Error 2".to_string(), ctx.current_position_range());

        ctx.add_error(error1);
        ctx.add_error(error2);

        let errors = ctx.take_errors();
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].message, "Error 1");
        assert_eq!(errors[1].message, "Error 2");
    }

    #[test]
    fn test_multiline_positions() {
        let source = "my $x = 42;\nmy $y = 43;".to_string();
        let ctx = ParserContext::new(source.clone());

        let first_offset = source.find("my").unwrap();
        let second_offset = source.rfind("my").unwrap();

        let first =
            ctx.tokens.iter().find(|t| t.range().start.byte == first_offset).expect("first token");
        assert_eq!(first.range().start.line, 1);
        assert_eq!(first.range().start.column, 1);
        assert_eq!(first.range().end.line, 1);
        assert_eq!(first.range().end.column, 3);

        let second = ctx
            .tokens
            .iter()
            .find(|t| t.range().start.byte == second_offset)
            .expect("second token");
        assert_eq!(second.range().start.line, 2);
        assert_eq!(second.range().start.column, 1);
        assert_eq!(second.range().end.line, 2);
        assert_eq!(second.range().end.column, 3);
    }

    #[test]
    fn test_multiline_string_token_positions() {
        let source = "my $s = \"a\nb\";".to_string();
        let ctx = ParserContext::new(source.clone());

        let string_offset = source.find('"').unwrap();
        let token = ctx
            .tokens
            .iter()
            .find(|t| t.range().start.byte == string_offset)
            .expect("string token");

        assert_eq!(token.range().start.line, 1);
        assert_eq!(token.range().start.column, 9);
        assert_eq!(token.range().end.line, 2);
        assert_eq!(token.range().end.column, 3);
    }
}
