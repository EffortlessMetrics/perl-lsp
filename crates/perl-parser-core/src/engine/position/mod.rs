//! Enhanced position tracking for incremental parsing
//!
//! This module provides position and range types that track byte offsets,
//! lines, and columns for efficient incremental parsing and error reporting.
//!
//! # Wire Types vs Engine Types
//!
//! This module defines **engine types** used internally for parsing and AST tracking.
//! For LSP wire protocol serialization, use `perl_lsp::convert::{WirePosition, WireRange}`.
//!
//! Engine Position uses 1-based line/column for human-readable display.
//! Wire Position uses 0-based line/character (UTF-16) per LSP protocol.
//!
//! **Important**: This type serializes as `{byte, line, column}` (engine semantics),
//! NOT as `{line, character}` (LSP semantics). For LSP responses, always convert
//! to WirePosition.

use serde::{Deserialize, Serialize};
use std::fmt;

/// A position in a source file with byte offset, line, and column
///
/// This is an **engine type** for internal parsing use. It tracks byte offsets
/// and 1-based line/column for human-friendly display.
///
/// **Note**: This type serializes as `{byte, line, column}` (engine semantics).
/// For LSP JSON serialization, convert to `perl_lsp::convert::WirePosition` which uses
/// 0-based line numbers and UTF-16 character offsets with the `character` field name.
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
///
/// This is an **engine type**. For LSP wire protocol, use `perl_lsp::convert::WireRange`.
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

pub mod line_index;
pub mod position_mapper;
#[doc(hidden)]
pub mod positions;

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

/// Convert byte offset to UTF-16 line and column for LSP
///
/// Provides symmetric position conversion that handles Unicode characters correctly
/// and ensures roundtrip consistency between offset ↔ (line, column) conversions.
///
/// # Arguments
///
/// * `text` - The source text containing the position to convert
/// * `offset` - Byte offset from the start of the text (0-based)
///
/// # Returns
///
/// A tuple `(line, column)` where:
/// - `line` is the 0-based line number
/// - `column` is the UTF-16 code unit offset within the line
///
/// # LSP Workflow Context
///
/// Critical for LSP protocol compliance throughout the workflow pipeline:
/// - **Navigate stage**: Convert parser byte offsets to LSP positions for go-to-definition
/// - **Complete stage**: Map completion trigger points to LSP protocol positions
/// - **Analyze stage**: Transform diagnostic byte ranges to LSP line/column ranges
///
/// # Examples
///
/// ```
/// use perl_parser::position::offset_to_utf16_line_col;
///
/// let text = "hello\nworld";
/// let (line, col) = offset_to_utf16_line_col(text, 6);
/// assert_eq!((line, col), (1, 0)); // Start of "world"
/// ```
pub fn offset_to_utf16_line_col(text: &str, offset: usize) -> (u32, u32) {
    // If offset is beyond text length, clamp to end
    if offset > text.len() {
        let lines: Vec<&str> = text.lines().collect();
        let last_line = lines.len().saturating_sub(1) as u32;
        let last_col = lines.last().map(|l| l.encode_utf16().count()).unwrap_or(0) as u32;
        return (last_line, last_col);
    }

    // If offset equals text length and text ends with newline, we're at start of next line
    if offset == text.len() && (text.ends_with('\n') || text.ends_with("\r\n")) {
        let line_count = text.split_inclusive('\n').count() as u32;
        return (line_count, 0);
    }

    let mut acc = 0usize;
    for (line_idx, line) in text.split_inclusive('\n').enumerate() {
        let next = acc + line.len();
        if offset < next {
            // Found the line containing our offset
            let rel = offset - acc;

            // Special case: handle mixed line endings like "...\n\r\n"
            // Only jump to next line if this is specifically a mixed line ending scenario
            if rel == line.len() - 1 && line.ends_with('\n') && offset == text.len() - 1 {
                // Check if this is a mixed line ending scenario by looking for \r\n following \n
                if line_idx > 0 && text.contains("\n\r\n") {
                    return ((line_idx + 1) as u32, 0);
                }
            }

            // Handle cases where offset points inside a multibyte character
            if rel == 0 {
                return (line_idx as u32, 0);
            } else if rel >= line.len() {
                let utf16_col = line.encode_utf16().count() as u32;
                return (line_idx as u32, utf16_col);
            } else if line.is_char_boundary(rel) {
                // Valid character boundary, slice normally
                let prefix = &line[..rel];
                let utf16_col = prefix.encode_utf16().count() as u32;
                return (line_idx as u32, utf16_col);
            } else {
                // Invalid character boundary - map to the start of the containing character
                // This ensures symmetric behavior with utf16_line_col_to_offset
                let mut char_start = rel;
                while char_start > 0 && !line.is_char_boundary(char_start) {
                    char_start -= 1;
                }

                // Map mid-character bytes to the next UTF-16 position to handle emoji correctly
                // For emoji like 😀 (4 UTF-8 bytes, 2 UTF-16 units), offset 1-3 should map to column 1
                let prefix = &line[..char_start];
                let utf16_col = prefix.encode_utf16().count() as u32;
                // If we're in the middle of a character, advance to the next UTF-16 position
                let next_utf16_col = utf16_col + 1;
                return (line_idx as u32, next_utf16_col);
            }
        }
        acc = next;
    }

    // Fallback: clamp to end
    let last_line = text.lines().count().saturating_sub(1) as u32;
    let last_col = text.lines().last().map(|l| l.encode_utf16().count()).unwrap_or(0) as u32;
    (last_line, last_col)
}

/// Convert UTF-16 line and column to byte offset
///
/// Provides symmetric position conversion that maps UTF-16 positions back to
/// exact character boundaries, ensuring roundtrip consistency.
///
/// # Arguments
///
/// * `text` - The source text to find the position in
/// * `line` - 0-based line number from LSP protocol
/// * `col` - UTF-16 code unit offset within the line
///
/// # Returns
///
/// The byte offset from the start of the text corresponding to the LSP position.
/// Returns the end of the text if the position is beyond the document bounds.
///
/// # LSP Workflow Context
///
/// Essential for processing LSP client requests throughout the workflow:
/// - **Navigate stage**: Convert LSP positions to byte offsets for symbol lookup
/// - **Complete stage**: Map completion request positions to parser coordinates
/// - **Analyze stage**: Transform LSP range edits to byte-based text modifications
///
/// # Examples
///
/// ```
/// use perl_parser::position::utf16_line_col_to_offset;
///
/// let text = "hello\nworld";
/// let offset = utf16_line_col_to_offset(text, 1, 0);
/// assert_eq!(offset, 6); // Start of "world"
/// ```
pub fn utf16_line_col_to_offset(text: &str, line: u32, col: u32) -> usize {
    let mut offset = 0usize;

    for (current_line, line_text) in text.split_inclusive('\n').enumerate() {
        if current_line as u32 == line {
            // Found the target line, now find the column
            let mut utf16_pos = 0u32;

            // Handle edge case: column 0 always maps to start of line
            if col == 0 {
                return offset;
            }

            for (byte_idx, ch) in line_text.char_indices() {
                let char_utf16_len = ch.len_utf16() as u32;

                // Check if we've reached or passed the target column
                if utf16_pos == col {
                    return offset + byte_idx;
                }

                // If the target column falls within this character's UTF-16 range,
                // map it to the start of the character for symmetric behavior
                if utf16_pos < col && col < utf16_pos + char_utf16_len {
                    return offset + byte_idx;
                }

                utf16_pos += char_utf16_len;

                // If we've passed the target column, return current position
                if utf16_pos > col {
                    return offset + byte_idx;
                }
            }

            // Column is beyond line end - clamp to end of line content
            // For lines ending with \n, don't include the newline in the position
            let line_content_len = if line_text.ends_with('\n') {
                line_text.len().saturating_sub(1)
            } else {
                line_text.len()
            };
            return offset + line_content_len.min(text.len() - offset);
        }
        offset += line_text.len();
    }

    // Line is beyond document end
    text.len()
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

    #[test]
    fn test_offset_to_utf16_line_col() {
        let text = "hello\nworld\n";
        assert_eq!(offset_to_utf16_line_col(text, 0), (0, 0));
        assert_eq!(offset_to_utf16_line_col(text, 5), (0, 5));
        assert_eq!(offset_to_utf16_line_col(text, 6), (1, 0));
        assert_eq!(offset_to_utf16_line_col(text, 11), (1, 5));
    }

    #[test]
    fn test_utf16_with_emojis() {
        let text = "hello 😀\nworld";
        // The emoji takes 2 UTF-16 code units
        assert_eq!(offset_to_utf16_line_col(text, 6), (0, 6)); // Before emoji
        assert_eq!(offset_to_utf16_line_col(text, 10), (0, 8)); // After emoji (6 + 2 UTF-16 units)
        assert_eq!(offset_to_utf16_line_col(text, 11), (1, 0)); // Next line
    }

    #[test]
    fn test_roundtrip() {
        let text = "hello\nworld\n";
        for offset in 0..text.len() {
            let (line, col) = offset_to_utf16_line_col(text, offset);
            let roundtrip = utf16_line_col_to_offset(text, line, col);
            assert_eq!(offset, roundtrip, "Failed roundtrip at offset {}", offset);
        }
    }

    #[test]
    fn test_crlf_handling() {
        let text = "hello\r\nworld\r\n";
        // Position at 'w' in world (after hello\r\n)
        assert_eq!(offset_to_utf16_line_col(text, 7), (1, 0));
        // Position at 'd' in world
        assert_eq!(offset_to_utf16_line_col(text, 11), (1, 4));
    }

    #[test]
    fn test_crlf_with_emoji() {
        let text = "hello 😀\r\nworld";
        // Position after emoji (which takes 4 bytes but 2 UTF-16 units)
        assert_eq!(offset_to_utf16_line_col(text, 6), (0, 6)); // Before emoji
        assert_eq!(offset_to_utf16_line_col(text, 10), (0, 8)); // After emoji (6 + 2 UTF-16 units)
        assert_eq!(offset_to_utf16_line_col(text, 12), (1, 0)); // Next line after \r\n
    }
}
