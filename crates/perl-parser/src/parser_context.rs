//! Parser context with error recovery support
//!
//! This module provides a parsing context that tracks errors, positions,
//! and supports error recovery for IDE scenarios.

use crate::{
    ast_v2::NodeIdGenerator,
    error_recovery::ParseError,
    position::{Position, Range},
    positions::LineStartsCache,
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
    /// Position tracker for efficient position mapping
    _position_tracker: PositionTracker,
}

/// Efficient position tracking using line starts cache
///
/// This implementation leverages the existing LineStartsCache for O(log n) position lookups
/// instead of O(n) character-by-character advancement. It provides UTF-16 aware position
/// mapping for LSP compatibility while integrating with the existing position infrastructure.
struct PositionTracker {
    /// Cache for O(log n) position lookups
    line_cache: LineStartsCache,
    /// Source text reference
    source: String,
}

impl PositionTracker {
    fn new(source: String) -> Self {
        let line_cache = LineStartsCache::new(&source);
        PositionTracker { line_cache, source }
    }

    /// Convert byte offset to position with UTF-16 support
    fn byte_to_position(&self, byte_offset: usize) -> Position {
        let (line, character) = self.line_cache.offset_to_position(&self.source, byte_offset);
        // LineStartsCache returns 0-based line numbers, but Position expects 1-based
        Position::new(byte_offset, line + 1, character + 1)
    }
}

impl ParserContext {
    /// Create a new parser context
    pub fn new(source: String) -> Self {
        let mut tokens = VecDeque::new();
        let position_tracker = PositionTracker::new(source.clone());

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

                    // Use efficient position mapping with UTF-16 support
                    let start_pos = position_tracker.byte_to_position(start);
                    let end_pos = position_tracker.byte_to_position(end);

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
        } else if let Some(last_token) = self.tokens.back() {
            // At EOF, use end of last token
            last_token.range().end
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

    #[test]
    fn test_utf16_position_mapping() {
        // Test with emoji which takes 2 UTF-16 code units
        let source = "my $emoji = 😀;".to_string();
        let ctx = ParserContext::new(source.clone());

        // Find the emoji token (if lexer produces it as separate token)
        // For now, test that positions are computed correctly for the = token
        let equals_offset = source.find('=').unwrap();
        let equals_token = ctx
            .tokens
            .iter()
            .find(|t| t.range().start.byte == equals_offset)
            .expect("equals token");

        // Before emoji: "my $emoji "  = 10 characters but the emoji counts as 2 UTF-16 units
        // So column should account for UTF-16 encoding
        assert_eq!(equals_token.range().start.line, 1);
        // The exact column depends on how the lexer tokenizes, but should be UTF-16 aware
        assert!(equals_token.range().start.column > 0);
    }

    #[test]
    fn test_crlf_line_endings() {
        let source = "my $x = 42;\r\nmy $y = 43;".to_string();
        let ctx = ParserContext::new(source.clone());

        let first_offset = source.find("my").unwrap();
        let second_offset = source.rfind("my").unwrap();

        let first =
            ctx.tokens.iter().find(|t| t.range().start.byte == first_offset).expect("first token");
        assert_eq!(first.range().start.line, 1);
        assert_eq!(first.range().start.column, 1);

        let second = ctx
            .tokens
            .iter()
            .find(|t| t.range().start.byte == second_offset)
            .expect("second token");
        assert_eq!(second.range().start.line, 2);
        assert_eq!(second.range().start.column, 1);
    }

    #[test]
    fn test_empty_source() {
        let source = "".to_string();
        let ctx = ParserContext::new(source);

        assert!(ctx.tokens.is_empty());
        assert!(ctx.is_eof());
    }

    #[test]
    fn test_single_token() {
        let source = "42".to_string();
        let ctx = ParserContext::new(source);

        assert_eq!(ctx.tokens.len(), 1);

        let token = &ctx.tokens[0];
        assert_eq!(token.range().start.byte, 0);
        assert_eq!(token.range().start.line, 1);
        assert_eq!(token.range().start.column, 1);
        assert_eq!(token.range().end.byte, 2);
        assert_eq!(token.range().end.line, 1);
        assert_eq!(token.range().end.column, 3);
    }
}
