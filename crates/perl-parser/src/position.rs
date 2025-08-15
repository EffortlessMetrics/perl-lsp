//! Enhanced position tracking for incremental parsing
//!
//! This module provides position and range types that track byte offsets,
//! lines, and columns for efficient incremental parsing and error reporting.

use serde::{Deserialize, Serialize};
use std::fmt;

/// A position in a source file with byte offset, line, and column
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Position {
    /// Byte offset in the source (0-based)
    pub byte: usize,
    /// Line number (1-based for user display)
    pub line: u32,
    /// Column number (1-based for user display)
    pub column: u32,
}

impl Position {
    /// Create a new position
    pub fn new(byte: usize, line: u32, column: u32) -> Self {
        Position { byte, line, column }
    }

    /// Create a position at the start of a file
    pub fn start() -> Self {
        Position { byte: 0, line: 1, column: 1 }
    }

    /// Advance the position by the given text
    pub fn advance(&mut self, text: &str) {
        for ch in text.chars() {
            if ch == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
            self.byte += ch.len_utf8();
        }
    }

    /// Advance by a single character
    pub fn advance_char(&mut self, ch: char) {
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        self.byte += ch.len_utf8();
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

/// A range in a source file defined by start and end positions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Range {
    /// Start position (inclusive)
    pub start: Position,
    /// End position (exclusive)
    pub end: Position,
}

impl Range {
    /// Create a new range
    pub fn new(start: Position, end: Position) -> Self {
        Range { start, end }
    }

    /// Create an empty range at a position
    pub fn empty(pos: Position) -> Self {
        Range { start: pos, end: pos }
    }

    /// Check if the range contains a byte offset
    pub fn contains_byte(&self, byte: usize) -> bool {
        self.start.byte <= byte && byte < self.end.byte
    }

    /// Check if the range contains a position
    pub fn contains(&self, pos: Position) -> bool {
        self.start.byte <= pos.byte && pos.byte < self.end.byte
    }

    /// Check if this range overlaps with another
    pub fn overlaps(&self, other: &Range) -> bool {
        self.start.byte < other.end.byte && other.start.byte < self.end.byte
    }

    /// Get the length in bytes
    pub fn len(&self) -> usize {
        self.end.byte.saturating_sub(self.start.byte)
    }

    /// Check if the range is empty
    pub fn is_empty(&self) -> bool {
        self.start.byte >= self.end.byte
    }

    /// Extend this range to include another range
    pub fn extend(&mut self, other: &Range) {
        if other.start.byte < self.start.byte {
            self.start = other.start;
        }
        if other.end.byte > self.end.byte {
            self.end = other.end;
        }
    }

    /// Create a range that spans from this range to another
    pub fn span_to(&self, other: &Range) -> Range {
        Range {
            start: if self.start.byte < other.start.byte { self.start } else { other.start },
            end: if self.end.byte > other.end.byte { self.end } else { other.end },
        }
    }
}

impl fmt::Display for Range {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}-{}", self.start, self.end)
    }
}

/// Convert old SourceLocation to Range (for migration)
impl From<crate::ast::SourceLocation> for Range {
    fn from(loc: crate::ast::SourceLocation) -> Self {
        // For migration, we'll need to calculate line/column later
        Range {
            start: Position { byte: loc.start, line: 0, column: 0 },
            end: Position { byte: loc.end, line: 0, column: 0 },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_advance() {
        let mut pos = Position::start();
        assert_eq!(pos, Position { byte: 0, line: 1, column: 1 });

        pos.advance("hello");
        assert_eq!(pos, Position { byte: 5, line: 1, column: 6 });

        pos.advance("\n");
        assert_eq!(pos, Position { byte: 6, line: 2, column: 1 });

        pos.advance("世界"); // UTF-8 multibyte
        assert_eq!(pos, Position { byte: 12, line: 2, column: 3 });
    }

    #[test]
    fn test_range_operations() {
        let start = Position::new(10, 2, 5);
        let end = Position::new(20, 3, 10);
        let range = Range::new(start, end);

        assert!(range.contains_byte(15));
        assert!(!range.contains_byte(25));
        assert_eq!(range.len(), 10);

        let other = Range::new(Position::new(15, 2, 10), Position::new(25, 4, 5));

        assert!(range.overlaps(&other));

        let span = range.span_to(&other);
        assert_eq!(span.start.byte, 10);
        assert_eq!(span.end.byte, 25);
    }

    #[test]
    fn test_range_empty() {
        let pos = Position::new(10, 2, 5);
        let range = Range::empty(pos);

        assert!(range.is_empty());
        assert_eq!(range.len(), 0);
    }
}
