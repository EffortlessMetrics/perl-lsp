//! Token wrapper with enhanced position tracking
//!
//! This module provides a wrapper around lexer tokens that adds
//! line and column information for incremental parsing support.

use crate::position::Position;
use perl_lexer::Token;

/// Token with full position information
#[derive(Debug, Clone)]
pub struct TokenWithPosition {
    /// The original token
    pub token: Token,
    /// Start position with line/column
    pub start_pos: Position,
    /// End position with line/column
    pub end_pos: Position,
}

impl TokenWithPosition {
    /// Create a new token with position
    pub fn new(token: Token, start_pos: Position, end_pos: Position) -> Self {
        TokenWithPosition {
            token,
            start_pos,
            end_pos,
        }
    }
    
    /// Get the token type
    pub fn kind(&self) -> &perl_lexer::TokenType {
        &self.token.token_type
    }
    
    /// Get the token text
    pub fn text(&self) -> &str {
        &self.token.text
    }
    
    /// Get byte range
    pub fn byte_range(&self) -> (usize, usize) {
        (self.token.start, self.token.end)
    }
    
    /// Get the position range
    pub fn range(&self) -> crate::position::Range {
        crate::position::Range::new(self.start_pos.clone(), self.end_pos.clone())
    }
}

/// Position tracker for converting byte offsets to line/column
pub struct PositionTracker<'a> {
    source: &'a str,
    line_starts: Vec<usize>,
}

impl<'a> PositionTracker<'a> {
    /// Create a new position tracker for the given source
    pub fn new(source: &'a str) -> Self {
        let mut line_starts = vec![0];
        
        for (i, ch) in source.char_indices() {
            if ch == '\n' {
                line_starts.push(i + 1);
            }
        }
        
        PositionTracker { source, line_starts }
    }
    
    /// Convert a byte offset to a Position
    pub fn byte_to_position(&self, byte: usize) -> Position {
        // Binary search for the line
        let line = match self.line_starts.binary_search(&byte) {
            Ok(line) => line,
            Err(line) => line.saturating_sub(1),
        };
        
        let line_start = self.line_starts[line];
        let column = self.calculate_column(line_start, byte);
        
        Position::new(byte, (line + 1) as u32, column)
    }
    
    /// Calculate column number accounting for UTF-8
    fn calculate_column(&self, line_start: usize, byte: usize) -> u32 {
        let line_slice = &self.source[line_start..byte.min(self.source.len())];
        (line_slice.chars().count() + 1) as u32
    }
    
    /// Wrap a token with position information
    pub fn wrap_token(&self, token: Token) -> TokenWithPosition {
        let start_pos = self.byte_to_position(token.start);
        let end_pos = self.byte_to_position(token.end);
        TokenWithPosition::new(token, start_pos, end_pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_lexer::{Token, TokenType};
    use std::sync::Arc;
    
    #[test]
    fn test_position_tracker() {
        let source = "hello\nworld\n";
        let tracker = PositionTracker::new(source);
        
        // First line
        let pos = tracker.byte_to_position(0);
        assert_eq!(pos.line, 1);
        assert_eq!(pos.column, 1);
        
        let pos = tracker.byte_to_position(3);
        assert_eq!(pos.line, 1);
        assert_eq!(pos.column, 4);
        
        // Second line
        let pos = tracker.byte_to_position(6);
        assert_eq!(pos.line, 2);
        assert_eq!(pos.column, 1);
    }
    
    #[test]
    fn test_token_wrapping() {
        let source = "my $x";
        let tracker = PositionTracker::new(source);
        
        let token = Token::new(
            TokenType::Keyword(Arc::from("my")),
            Arc::from("my"),
            0,
            2
        );
        
        let wrapped = tracker.wrap_token(token);
        assert_eq!(wrapped.start_pos.line, 1);
        assert_eq!(wrapped.start_pos.column, 1);
        assert_eq!(wrapped.end_pos.column, 3);
    }
}