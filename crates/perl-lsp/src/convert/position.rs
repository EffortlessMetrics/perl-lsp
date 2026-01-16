//! LSP wire types for Position and Range.
//!
//! These types represent positions and ranges as they appear on the LSP wire protocol.
//! They are 0-based and use UTF-16 code units for character offsets.
//!
//! # Conversion Guarantees
//!
//! - `WirePosition` and `WireRange` use 0-based line numbers
//! - `character` field is in UTF-16 code units (not bytes, not chars)
//! - All conversions go through byte offsets for correctness
//!
//! # Usage
//!
//! ```ignore
//! use perl_lsp::convert::{WirePosition, WireRange};
//!
//! // Convert from engine position (requires source text for UTF-16 calculation)
//! let wire = WirePosition::from_engine(&engine_pos, source_text);
//!
//! // Convert to lsp_types for protocol responses
//! let lsp_pos: lsp_types::Position = wire.into();
//! ```

use serde::{Deserialize, Serialize};

/// LSP wire position - 0-based line and UTF-16 character offset.
///
/// This is the canonical type for positions in LSP JSON responses.
/// Use this instead of `perl_parser_core::position::Position` when serializing to JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WirePosition {
    /// 0-based line number
    pub line: u32,
    /// 0-based UTF-16 code unit offset within the line
    pub character: u32,
}

impl WirePosition {
    /// Create a new wire position.
    pub fn new(line: u32, character: u32) -> Self {
        WirePosition { line, character }
    }

    /// Convert from a byte offset in source text to a wire position.
    ///
    /// This is the primary constructor for converting engine coordinates (byte offsets)
    /// to wire coordinates (UTF-16 line/character). All position conversions should
    /// go through this method to ensure UTF-16 correctness.
    ///
    /// # Arguments
    ///
    /// * `source` - The source text (used to calculate line breaks and UTF-16 offsets)
    /// * `byte_offset` - The byte offset within the source text
    ///
    /// # Example
    ///
    /// ```ignore
    /// let source = "hello\nworld";
    /// let pos = WirePosition::from_byte_offset(source, 7); // 'o' in "world"
    /// assert_eq!(pos.line, 1);
    /// assert_eq!(pos.character, 1);
    /// ```
    pub fn from_byte_offset(source: &str, byte_offset: usize) -> Self {
        let (line, character) =
            perl_parser_core::position::offset_to_utf16_line_col(source, byte_offset);
        WirePosition { line, character }
    }

    /// Convert from engine position using byte offset and source text.
    ///
    /// The engine position's byte offset is used to compute the correct
    /// 0-based line and UTF-16 character offset.
    pub fn from_engine(pos: &perl_parser_core::position::Position, source: &str) -> Self {
        Self::from_byte_offset(source, pos.byte)
    }

    /// Convert this wire position to a byte offset in the source text.
    pub fn to_byte_offset(&self, source: &str) -> usize {
        perl_parser_core::position::utf16_line_col_to_offset(source, self.line, self.character)
    }
}

/// LSP wire range - start and end wire positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WireRange {
    /// Start position (inclusive)
    pub start: WirePosition,
    /// End position (exclusive)
    pub end: WirePosition,
}

impl WireRange {
    /// Create a new wire range.
    pub fn new(start: WirePosition, end: WirePosition) -> Self {
        WireRange { start, end }
    }

    /// Convert from byte offsets in source text to a wire range.
    ///
    /// This is the primary constructor for converting engine coordinates (byte offsets)
    /// to wire coordinates (UTF-16 ranges). All range conversions should go through
    /// this method to ensure UTF-16 correctness.
    ///
    /// # Arguments
    ///
    /// * `source` - The source text (used to calculate line breaks and UTF-16 offsets)
    /// * `start_byte` - The start byte offset (inclusive)
    /// * `end_byte` - The end byte offset (exclusive)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let source = "hello\nworld";
    /// let range = WireRange::from_byte_offsets(source, 0, 5); // "hello"
    /// assert_eq!(range.start, WirePosition::new(0, 0));
    /// assert_eq!(range.end, WirePosition::new(0, 5));
    /// ```
    pub fn from_byte_offsets(source: &str, start_byte: usize, end_byte: usize) -> Self {
        WireRange {
            start: WirePosition::from_byte_offset(source, start_byte),
            end: WirePosition::from_byte_offset(source, end_byte),
        }
    }

    /// Convert from engine range using source text.
    pub fn from_engine(range: &perl_parser_core::position::Range, source: &str) -> Self {
        Self::from_byte_offsets(source, range.start.byte, range.end.byte)
    }

    /// Create a zero-width range at a position.
    pub fn empty(pos: WirePosition) -> Self {
        WireRange { start: pos, end: pos }
    }

    /// Create a range covering the entire document.
    ///
    /// This correctly computes the UTF-16 position of the end of the document,
    /// handling multi-byte UTF-8 characters and surrogate pairs properly.
    ///
    /// # Arguments
    ///
    /// * `source` - The full source text
    ///
    /// # Example
    ///
    /// ```ignore
    /// let source = "hello\nworld";
    /// let range = WireRange::whole_document(source);
    /// assert_eq!(range.start, WirePosition::new(0, 0));
    /// // End position is computed correctly via byte offset
    /// ```
    pub fn whole_document(source: &str) -> Self {
        WireRange {
            start: WirePosition::new(0, 0),
            end: WirePosition::from_byte_offset(source, source.len()),
        }
    }
}

/// LSP wire location - URI and range.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireLocation {
    /// Document URI
    pub uri: String,
    /// Range within the document
    pub range: WireRange,
}

impl WireLocation {
    /// Create a new wire location.
    pub fn new(uri: String, range: WireRange) -> Self {
        WireLocation { uri, range }
    }
}

// =============================================================================
// Conversions to/from lsp_types
// =============================================================================

impl From<WirePosition> for lsp_types::Position {
    fn from(p: WirePosition) -> Self {
        lsp_types::Position { line: p.line, character: p.character }
    }
}

impl From<lsp_types::Position> for WirePosition {
    fn from(p: lsp_types::Position) -> Self {
        WirePosition { line: p.line, character: p.character }
    }
}

impl From<WireRange> for lsp_types::Range {
    fn from(r: WireRange) -> Self {
        lsp_types::Range { start: r.start.into(), end: r.end.into() }
    }
}

impl From<lsp_types::Range> for WireRange {
    fn from(r: lsp_types::Range) -> Self {
        WireRange { start: r.start.into(), end: r.end.into() }
    }
}

impl From<WireLocation> for lsp_types::Location {
    fn from(l: WireLocation) -> Self {
        // SAFETY: "file:///unknown" is a valid URI constant, parse will not fail
        #[allow(clippy::expect_used)]
        static FALLBACK_URI: std::sync::LazyLock<lsp_types::Uri> =
            std::sync::LazyLock::new(|| "file:///unknown".parse().expect("valid fallback URL"));

        lsp_types::Location {
            uri: l.uri.parse().unwrap_or_else(|_| FALLBACK_URI.clone()),
            range: l.range.into(),
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use perl_parser_core::position::Position as EnginePosition;

    #[test]
    fn test_wire_position_from_engine_simple() {
        // "hello\nworld" - position at 'w' is byte 6, line 2, col 1 (1-based)
        let source = "hello\nworld";
        let engine_pos = EnginePosition::new(6, 2, 1);

        let wire = WirePosition::from_engine(&engine_pos, source);

        // LSP: 0-based line 1, character 0
        assert_eq!(wire.line, 1);
        assert_eq!(wire.character, 0);
    }

    #[test]
    fn test_wire_position_from_engine_with_emoji() {
        // "hi \u{1F600}\nworld" - emoji is 4 bytes but 2 UTF-16 units
        let source = "hi \u{1F600}\nworld";
        let engine_pos = EnginePosition::new(8, 2, 1); // byte 8 = after emoji + newline

        let wire = WirePosition::from_engine(&engine_pos, source);

        // Should be on line 1 (0-based), character 0
        assert_eq!(wire.line, 1);
        assert_eq!(wire.character, 0);
    }

    #[test]
    fn test_wire_position_roundtrip() {
        let source = "hello\nworld";
        let engine_pos = EnginePosition::new(7, 2, 2); // byte 7 = 'o' in "world"

        let wire = WirePosition::from_engine(&engine_pos, source);
        let byte_offset = wire.to_byte_offset(source);

        assert_eq!(byte_offset, 7);
    }

    #[test]
    fn test_wire_range_from_engine() {
        let source = "hello\nworld";
        let engine_range = perl_parser_core::position::Range::new(
            EnginePosition::new(0, 1, 1), // start of "hello"
            EnginePosition::new(5, 1, 6), // end of "hello"
        );

        let wire = WireRange::from_engine(&engine_range, source);

        assert_eq!(wire.start.line, 0);
        assert_eq!(wire.start.character, 0);
        assert_eq!(wire.end.line, 0);
        assert_eq!(wire.end.character, 5);
    }

    #[test]
    fn test_lsp_types_conversion() {
        let wire = WirePosition::new(5, 10);
        let lsp: lsp_types::Position = wire.into();

        assert_eq!(lsp.line, 5);
        assert_eq!(lsp.character, 10);

        let back: WirePosition = lsp.into();
        assert_eq!(back, wire);
    }

    #[test]
    fn test_wire_position_from_byte_offset_simple() {
        let source = "hello\nworld";
        let pos = WirePosition::from_byte_offset(source, 7); // 'o' in "world"

        assert_eq!(pos.line, 1);
        assert_eq!(pos.character, 1);
    }

    #[test]
    fn test_wire_position_from_byte_offset_with_emoji() {
        // "hi 😀x" - emoji is 4 bytes but 2 UTF-16 units, 'x' is at byte 7
        let source = "hi \u{1F600}x";
        let pos = WirePosition::from_byte_offset(source, 7); // 'x' after emoji

        assert_eq!(pos.line, 0);
        // "hi " = 3 UTF-16 units, emoji = 2 UTF-16 units, total = 5
        assert_eq!(pos.character, 5);
    }

    #[test]
    fn test_wire_range_from_byte_offsets() {
        let source = "hello\nworld";
        let range = WireRange::from_byte_offsets(source, 0, 5); // "hello"

        assert_eq!(range.start.line, 0);
        assert_eq!(range.start.character, 0);
        assert_eq!(range.end.line, 0);
        assert_eq!(range.end.character, 5);
    }

    #[test]
    fn test_wire_range_whole_document_simple() {
        let source = "hello\nworld";
        let range = WireRange::whole_document(source);

        assert_eq!(range.start.line, 0);
        assert_eq!(range.start.character, 0);
        assert_eq!(range.end.line, 1);
        assert_eq!(range.end.character, 5);
    }

    #[test]
    fn test_wire_range_whole_document_with_emoji() {
        // Document ends with emoji - ensures UTF-16 is computed correctly
        let source = "hello\n\u{1F600}";
        let range = WireRange::whole_document(source);

        assert_eq!(range.start.line, 0);
        assert_eq!(range.start.character, 0);
        assert_eq!(range.end.line, 1);
        // Emoji is 2 UTF-16 code units (not 4 bytes)
        assert_eq!(range.end.character, 2);
    }

    #[test]
    fn test_wire_range_whole_document_empty() {
        let source = "";
        let range = WireRange::whole_document(source);

        assert_eq!(range.start.line, 0);
        assert_eq!(range.start.character, 0);
        assert_eq!(range.end.line, 0);
        assert_eq!(range.end.character, 0);
    }
}
