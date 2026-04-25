//! Centralized position mapping for correct LSP position handling
//!
//! Handles:
//! - CRLF/LF/CR line endings
//! - UTF-16 code units (LSP protocol)
//! - Byte offsets (parser)
//! - Efficient conversions using rope data structure

use crate::WirePosition as Position;
use ropey::Rope;
use serde_json::Value;

/// Centralized position mapper using rope for efficiency.
///
/// Converts between byte offsets (used by the parser) and LSP positions
/// (line/character in UTF-16 code units) while handling mixed line endings.
///
/// # Examples
///
/// ```
/// use perl_position_tracking::PositionMapper;
///
/// let text = "my $x = 1;\nmy $y = 2;\n";
/// let mapper = PositionMapper::new(text);
///
/// // Convert byte offset 0 → LSP position (line 0, char 0)
/// let pos = mapper.byte_to_lsp_pos(0);
/// assert_eq!(pos.line, 0);
/// assert_eq!(pos.character, 0);
///
/// // Second line starts at byte 11
/// let pos = mapper.byte_to_lsp_pos(11);
/// assert_eq!(pos.line, 1);
/// assert_eq!(pos.character, 0);
/// ```
pub struct PositionMapper {
    /// The rope containing the document text
    rope: Rope,
    /// Cache of line ending style
    line_ending: LineEnding,
}

/// Line ending style detected in a document
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding {
    /// Unix-style line endings (LF only)
    Lf,
    /// Windows-style line endings (CRLF)
    CrLf,
    /// Classic Mac line endings (CR only)
    Cr,
    /// Mixed line endings detected
    Mixed,
}

impl PositionMapper {
    /// Create a new position mapper from text.
    ///
    /// Detects line endings and builds an internal rope for efficient
    /// position conversions.
    ///
    /// # Examples
    ///
    /// ```
    /// use perl_position_tracking::PositionMapper;
    ///
    /// let mapper = PositionMapper::new("print 'hello';\n");
    /// let pos = mapper.byte_to_lsp_pos(6);
    /// assert_eq!(pos.line, 0);
    /// assert_eq!(pos.character, 6);
    /// ```
    pub fn new(text: &str) -> Self {
        let rope = Rope::from_str(text);
        let line_ending = detect_line_ending(text);
        Self { rope, line_ending }
    }

    /// Update the text content
    pub fn update(&mut self, text: &str) {
        self.rope = Rope::from_str(text);
        self.line_ending = detect_line_ending(text);
    }

    /// Apply an incremental edit
    pub fn apply_edit(&mut self, start_byte: usize, end_byte: usize, new_text: &str) {
        // Clamp to valid range
        let start_byte = start_byte.min(self.rope.len_bytes());
        let end_byte = end_byte.min(self.rope.len_bytes());

        // Convert byte offsets to char indices (rope uses chars!)
        let start_char = self.rope.byte_to_char(start_byte);
        let end_char = self.rope.byte_to_char(end_byte);

        // Remove old text
        if end_char > start_char {
            self.rope.remove(start_char..end_char);
        }

        // Insert new text
        if !new_text.is_empty() {
            self.rope.insert(start_char, new_text);
        }

        // Update line ending detection
        self.line_ending = detect_line_ending(&self.rope.to_string());
    }

    /// Convert LSP position to byte offset.
    ///
    /// Takes a line/character position (UTF-16 code units, as specified by the
    /// LSP protocol) and returns the corresponding byte offset in the source.
    ///
    /// # Examples
    ///
    /// ```
    /// use perl_position_tracking::{PositionMapper, WirePosition};
    ///
    /// let mapper = PositionMapper::new("my $x = 1;\nmy $y = 2;\n");
    /// // Line 1, character 3 → "$y"
    /// let byte = mapper.lsp_pos_to_byte(WirePosition { line: 1, character: 3 });
    /// assert_eq!(byte, Some(14));
    /// ```
    pub fn lsp_pos_to_byte(&self, pos: Position) -> Option<usize> {
        let line_idx = pos.line as usize;
        if line_idx >= self.rope.len_lines() {
            return None;
        }

        let line_start_byte = self.rope.line_to_byte(line_idx);
        let line = self.rope.line(line_idx);

        // Convert UTF-16 code units to byte offset
        let mut utf16_offset = 0u32;
        let mut byte_offset = 0;

        for ch in line.chars() {
            if utf16_offset >= pos.character {
                break;
            }

            let ch_utf16_len = if ch as u32 > 0xFFFF { 2 } else { 1 };
            let next_utf16 = utf16_offset + ch_utf16_len;

            // Clamp positions inside a surrogate pair to the start of the
            // code point, matching `utf16_line_col_to_offset`.
            if next_utf16 > pos.character {
                break;
            }

            utf16_offset = next_utf16;
            byte_offset += ch.len_utf8();
        }

        Some(line_start_byte + byte_offset)
    }

    /// Convert byte offset to LSP position.
    ///
    /// Returns line/character (UTF-16 code units) suitable for LSP responses.
    ///
    /// # Examples
    ///
    /// ```
    /// use perl_position_tracking::PositionMapper;
    ///
    /// let mapper = PositionMapper::new("sub foo {\n    return 1;\n}\n");
    /// let pos = mapper.byte_to_lsp_pos(14);  // points into "return"
    /// assert_eq!(pos.line, 1);
    /// assert_eq!(pos.character, 4);
    /// ```
    pub fn byte_to_lsp_pos(&self, byte_offset: usize) -> Position {
        let byte_offset = byte_offset.min(self.rope.len_bytes());

        let line_idx = self.rope.byte_to_line(byte_offset);
        let line_start_byte = self.rope.line_to_byte(line_idx);
        let byte_in_line = byte_offset - line_start_byte;

        // Convert byte offset to UTF-16 code units
        let line = self.rope.line(line_idx);
        let mut utf16_offset = 0u32;
        let mut current_byte = 0;

        for ch in line.chars() {
            if current_byte >= byte_in_line {
                break;
            }
            let ch_len = ch.len_utf8();
            if current_byte + ch_len > byte_in_line {
                // We're in the middle of this character
                break;
            }
            current_byte += ch_len;
            let ch_utf16_len = if ch as u32 > 0xFFFF { 2 } else { 1 };
            utf16_offset += ch_utf16_len;
        }

        Position { line: line_idx as u32, character: utf16_offset }
    }

    /// Get the text content
    pub fn text(&self) -> String {
        self.rope.to_string()
    }

    /// Get a slice of text
    pub fn slice(&self, start_byte: usize, end_byte: usize) -> String {
        let start = start_byte.min(self.rope.len_bytes());
        let end = end_byte.min(self.rope.len_bytes());
        self.rope.slice(self.rope.byte_to_char(start)..self.rope.byte_to_char(end)).to_string()
    }

    /// Get total byte length
    pub fn len_bytes(&self) -> usize {
        self.rope.len_bytes()
    }

    /// Get total number of lines
    pub fn len_lines(&self) -> usize {
        self.rope.len_lines()
    }

    /// Convert LSP position to char index (for rope operations)
    pub fn lsp_pos_to_char(&self, pos: Position) -> Option<usize> {
        self.lsp_pos_to_byte(pos).map(|byte| self.rope.byte_to_char(byte))
    }

    /// Convert char index to LSP position
    pub fn char_to_lsp_pos(&self, char_idx: usize) -> Position {
        let byte_offset = self.rope.char_to_byte(char_idx);
        self.byte_to_lsp_pos(byte_offset)
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.rope.len_bytes() == 0
    }

    /// Get line ending style
    pub fn line_ending(&self) -> LineEnding {
        self.line_ending
    }
}

/// Convert JSON LSP position to our Position type.
///
/// Extracts line and character fields from a JSON object.
pub fn json_to_position(pos: &Value) -> Option<Position> {
    Some(Position {
        line: pos["line"].as_u64()? as u32,
        character: pos["character"].as_u64()? as u32,
    })
}

/// Convert Position to JSON for LSP.
///
/// Creates a JSON object with line and character fields.
pub fn position_to_json(pos: Position) -> Value {
    serde_json::json!({
        "line": pos.line,
        "character": pos.character
    })
}

/// Detect the predominant line ending style
fn detect_line_ending(text: &str) -> LineEnding {
    let mut crlf_count = 0;
    let mut lf_count = 0;
    let mut cr_count = 0;

    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'\r' && bytes[i + 1] == b'\n' {
            crlf_count += 1;
            i += 2;
        } else if bytes[i] == b'\n' {
            lf_count += 1;
            i += 1;
        } else if bytes[i] == b'\r' {
            cr_count += 1;
            i += 1;
        } else {
            i += 1;
        }
    }

    // Determine predominant style
    if crlf_count > 0 && lf_count == 0 && cr_count == 0 {
        LineEnding::CrLf
    } else if lf_count > 0 && crlf_count == 0 && cr_count == 0 {
        LineEnding::Lf
    } else if cr_count > 0 && crlf_count == 0 && lf_count == 0 {
        LineEnding::Cr
    } else if crlf_count > 0 || lf_count > 0 || cr_count > 0 {
        LineEnding::Mixed
    } else {
        LineEnding::Lf // Default
    }
}

/// Apply UTF-8 edit to a string.
///
/// Replaces the byte range with the given replacement text.
pub fn apply_edit_utf8(
    text: &mut String,
    start_byte: usize,
    old_end_byte: usize,
    replacement: &str,
) {
    if !text.is_char_boundary(start_byte) || !text.is_char_boundary(old_end_byte) {
        // Safety: ensure we're at char boundaries
        return;
    }
    text.replace_range(start_byte..old_end_byte, replacement);
}

/// Count newlines in text.
///
/// Returns the number of LF characters in the string.
pub fn newline_count(text: &str) -> usize {
    text.chars().filter(|&c| c == '\n').count()
}

/// Get the column (in UTF-8 bytes) of the last line.
///
/// Returns the byte offset from the last newline to the end of the string.
pub fn last_line_column_utf8(text: &str) -> u32 {
    if let Some(last_newline) = text.rfind('\n') {
        (text.len() - last_newline - 1) as u32
    } else {
        text.len() as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lf_positions() {
        let text = "line 1\nline 2\nline 3";
        let mapper = PositionMapper::new(text);

        // Start of document
        assert_eq!(mapper.lsp_pos_to_byte(Position { line: 0, character: 0 }), Some(0));
        assert_eq!(mapper.byte_to_lsp_pos(0), Position { line: 0, character: 0 });

        // Middle of first line
        assert_eq!(mapper.lsp_pos_to_byte(Position { line: 0, character: 3 }), Some(3));
        assert_eq!(mapper.byte_to_lsp_pos(3), Position { line: 0, character: 3 });

        // Start of second line
        assert_eq!(mapper.lsp_pos_to_byte(Position { line: 1, character: 0 }), Some(7));
        assert_eq!(mapper.byte_to_lsp_pos(7), Position { line: 1, character: 0 });
    }

    #[test]
    fn test_crlf_positions() {
        let text = "line 1\r\nline 2\r\nline 3";
        let mapper = PositionMapper::new(text);

        assert_eq!(mapper.line_ending(), LineEnding::CrLf);

        // Start of second line (after \r\n)
        assert_eq!(mapper.lsp_pos_to_byte(Position { line: 1, character: 0 }), Some(8));
        assert_eq!(mapper.byte_to_lsp_pos(8), Position { line: 1, character: 0 });

        // Start of third line
        assert_eq!(mapper.lsp_pos_to_byte(Position { line: 2, character: 0 }), Some(16));
        assert_eq!(mapper.byte_to_lsp_pos(16), Position { line: 2, character: 0 });
    }

    #[test]
    fn test_utf16_positions() {
        let text = "hello 😀 world"; // Emoji is 2 UTF-16 code units
        let mapper = PositionMapper::new(text);

        // Before emoji
        assert_eq!(mapper.lsp_pos_to_byte(Position { line: 0, character: 6 }), Some(6));

        // After emoji (6 + 2 UTF-16 units = 8)
        assert_eq!(mapper.lsp_pos_to_byte(Position { line: 0, character: 8 }), Some(10)); // 6 + 4 bytes for emoji

        // Convert back
        assert_eq!(mapper.byte_to_lsp_pos(10), Position { line: 0, character: 8 });
    }

    #[test]
    fn test_utf16_positions_clamp_mid_surrogate_to_char_start() {
        let text = "a😀b";
        let mapper = PositionMapper::new(text);

        // UTF-16 position 2 lands inside 😀 (which spans code units 1..3).
        assert_eq!(mapper.lsp_pos_to_byte(Position { line: 0, character: 2 }), Some(1));
    }

    #[test]
    fn test_utf16_surrogate_pair_boundaries() {
        // 💖 (U+1F496) is a non-BMP char requiring a surrogate pair.
        // Byte layout: 'x'=1 byte, '💖'=4 bytes (U+1F496), 'y'=1 byte.
        // UTF-16 layout: 'x'=1 unit, '💖'=2 units (surrogate pair), 'y'=1 unit.
        let text = "x💖y";
        let mapper = PositionMapper::new(text);

        // Before surrogate pair (column 0, 1)
        assert_eq!(mapper.lsp_pos_to_byte(Position { line: 0, character: 0 }), Some(0));
        assert_eq!(mapper.lsp_pos_to_byte(Position { line: 0, character: 1 }), Some(1));

        // Mid-surrogate (column 2) — must clamp to start of emoji (byte 1),
        // matching `utf16_line_col_to_offset` behavior.
        assert_eq!(mapper.lsp_pos_to_byte(Position { line: 0, character: 2 }), Some(1));

        // End of surrogate pair (column 3) — points just past emoji.
        assert_eq!(mapper.lsp_pos_to_byte(Position { line: 0, character: 3 }), Some(5));

        // After 'y' (column 4) — end of string.
        assert_eq!(mapper.lsp_pos_to_byte(Position { line: 0, character: 4 }), Some(6));
    }

    #[test]
    fn test_utf16_max_code_point() {
        // U+10FFFF is the highest valid Unicode code point.
        // Encoded as UTF-8 it's 4 bytes; in UTF-16 it's a surrogate pair (2 units).
        let max_char = '\u{10FFFF}';
        let text = format!("a{max_char}b");
        let mapper = PositionMapper::new(&text);

        // 'a' is col 0, U+10FFFF occupies cols 1..3, 'b' is col 3.
        assert_eq!(mapper.lsp_pos_to_byte(Position { line: 0, character: 0 }), Some(0));
        assert_eq!(mapper.lsp_pos_to_byte(Position { line: 0, character: 1 }), Some(1));
        // Mid-surrogate clamp
        assert_eq!(mapper.lsp_pos_to_byte(Position { line: 0, character: 2 }), Some(1));
        assert_eq!(mapper.lsp_pos_to_byte(Position { line: 0, character: 3 }), Some(5));
        assert_eq!(mapper.lsp_pos_to_byte(Position { line: 0, character: 4 }), Some(6));

        // Round-trip the byte offsets back to positions (non-mid-surrogate).
        assert_eq!(mapper.byte_to_lsp_pos(0), Position { line: 0, character: 0 });
        assert_eq!(mapper.byte_to_lsp_pos(1), Position { line: 0, character: 1 });
        assert_eq!(mapper.byte_to_lsp_pos(5), Position { line: 0, character: 3 });
        assert_eq!(mapper.byte_to_lsp_pos(6), Position { line: 0, character: 4 });
    }

    #[test]
    fn test_utf16_mixed_bmp_and_supplementary_plane() {
        // é (U+00E9, BMP, 2 bytes UTF-8, 1 UTF-16 unit)
        // 💖 (U+1F496, supplementary, 4 bytes UTF-8, 2 UTF-16 units)
        // ñ (U+00F1, BMP, 2 bytes UTF-8, 1 UTF-16 unit)
        // 🎉 (U+1F389, supplementary, 4 bytes UTF-8, 2 UTF-16 units)
        let text = "aé💖ñ🎉b";
        let mapper = PositionMapper::new(text);

        // Columns:
        //   a  = 0
        //   é  = 1
        //   💖 = 2..4 (surrogate pair)
        //   ñ  = 4
        //   🎉 = 5..7 (surrogate pair)
        //   b  = 7
        //   end = 8
        assert_eq!(mapper.lsp_pos_to_byte(Position { line: 0, character: 0 }), Some(0)); // a
        assert_eq!(mapper.lsp_pos_to_byte(Position { line: 0, character: 1 }), Some(1)); // é
        assert_eq!(mapper.lsp_pos_to_byte(Position { line: 0, character: 2 }), Some(3)); // 💖 start
        assert_eq!(mapper.lsp_pos_to_byte(Position { line: 0, character: 3 }), Some(3)); // mid-surrogate clamp
        assert_eq!(mapper.lsp_pos_to_byte(Position { line: 0, character: 4 }), Some(7)); // ñ
        assert_eq!(mapper.lsp_pos_to_byte(Position { line: 0, character: 5 }), Some(9)); // 🎉 start
        assert_eq!(mapper.lsp_pos_to_byte(Position { line: 0, character: 6 }), Some(9)); // mid-surrogate clamp
        assert_eq!(mapper.lsp_pos_to_byte(Position { line: 0, character: 7 }), Some(13)); // b
        assert_eq!(mapper.lsp_pos_to_byte(Position { line: 0, character: 8 }), Some(14)); // end
    }

    #[test]
    fn test_utf16_zero_length_input() {
        let text = "";
        let mapper = PositionMapper::new(text);

        // An empty rope has one logical line (line 0) of length 0.
        // Position (0, 0) should map to byte 0.
        assert_eq!(mapper.lsp_pos_to_byte(Position { line: 0, character: 0 }), Some(0));
        // Any character beyond 0 should clamp to byte 0 (end of empty line).
        assert_eq!(mapper.lsp_pos_to_byte(Position { line: 0, character: 5 }), Some(0));

        // Line past end of document returns None.
        assert!(mapper.lsp_pos_to_byte(Position { line: 1, character: 0 }).is_none());

        // Reverse direction: byte 0 should map to (0, 0).
        assert_eq!(mapper.byte_to_lsp_pos(0), Position { line: 0, character: 0 });
    }

    #[test]
    fn test_utf16_consecutive_surrogate_pairs() {
        // Back-to-back supplementary-plane chars to ensure mid-surrogate
        // clamping doesn't advance past the current char.
        let text = "💖💖";
        let mapper = PositionMapper::new(text);

        // First 💖 is cols 0..2, second 💖 is cols 2..4.
        // Bytes: first = 0..4, second = 4..8.
        assert_eq!(mapper.lsp_pos_to_byte(Position { line: 0, character: 0 }), Some(0));
        // Mid first surrogate pair — clamp to start of first emoji (byte 0).
        assert_eq!(mapper.lsp_pos_to_byte(Position { line: 0, character: 1 }), Some(0));
        assert_eq!(mapper.lsp_pos_to_byte(Position { line: 0, character: 2 }), Some(4));
        // Mid second surrogate pair — clamp to start of second emoji (byte 4).
        assert_eq!(mapper.lsp_pos_to_byte(Position { line: 0, character: 3 }), Some(4));
        assert_eq!(mapper.lsp_pos_to_byte(Position { line: 0, character: 4 }), Some(8));
    }

    #[test]
    fn test_utf16_clamp_matches_convert_helper() {
        // Parity: PositionMapper::lsp_pos_to_byte should agree with the
        // convert::utf16_line_col_to_offset helper at every column, including
        // mid-surrogate positions. These are the two canonical UTF-16 -> byte
        // converters and they must never disagree.
        use crate::convert::utf16_line_col_to_offset;

        let text = "a😀b💖c\nx💡y";
        let mapper = PositionMapper::new(text);

        // Line 0: "a😀b💖c"
        //   a=0, 😀=1..3, b=3, 💖=4..6, c=6, end=7
        for col in 0..=7 {
            let mapper_byte =
                mapper.lsp_pos_to_byte(Position { line: 0, character: col }).unwrap_or(usize::MAX);
            let helper_byte = utf16_line_col_to_offset(text, 0, col);
            assert_eq!(
                mapper_byte, helper_byte,
                "disagreement at line 0 col {col}: mapper={mapper_byte} helper={helper_byte}"
            );
        }
    }

    #[test]
    fn test_mixed_line_endings() {
        let text = "line 1\r\nline 2\nline 3\rline 4";
        let mapper = PositionMapper::new(text);

        assert_eq!(mapper.line_ending(), LineEnding::Mixed);

        // Each line start
        assert_eq!(mapper.byte_to_lsp_pos(0), Position { line: 0, character: 0 });
        assert_eq!(mapper.byte_to_lsp_pos(8), Position { line: 1, character: 0 });
        assert_eq!(mapper.byte_to_lsp_pos(15), Position { line: 2, character: 0 });
        assert_eq!(mapper.byte_to_lsp_pos(22), Position { line: 3, character: 0 });
    }

    #[test]
    fn test_incremental_edit() {
        let mut mapper = PositionMapper::new("hello world");

        // Replace "world" with "Rust"
        mapper.apply_edit(6, 11, "Rust");
        assert_eq!(mapper.text(), "hello Rust");

        // Insert in middle
        mapper.apply_edit(5, 5, " beautiful");
        assert_eq!(mapper.text(), "hello beautiful Rust");

        // Delete "beautiful " (keep one space)
        mapper.apply_edit(5, 16, " ");
        assert_eq!(mapper.text(), "hello Rust");
    }
}
