//! Line index for efficient UTF-16 position calculations
//!
//! LSP requires UTF-16 code unit positions, but Rust uses UTF-8.
//! This module provides efficient conversion between byte offsets,
//! line/column positions, and UTF-16 positions.

/// Stores line information for efficient position lookups
#[derive(Debug, Clone)]
pub struct LineIndex {
    /// Byte offset of each line start
    line_starts: Vec<usize>,
    /// The source text
    text: String,
}

impl LineIndex {
    /// Create a new LineIndex from source text
    pub fn new(text: String) -> Self {
        let mut line_starts = vec![0];
        for (i, ch) in text.char_indices() {
            if ch == '\n' {
                line_starts.push(i + 1);
            }
        }

        Self { line_starts, text }
    }

    /// Convert byte offset to position (0-based line and UTF-16 column)
    pub fn offset_to_position(&mut self, offset: usize) -> (u32, u32) {
        let line = self
            .line_starts
            .binary_search(&offset)
            .unwrap_or_else(|i| i.saturating_sub(1));

        let line_start = self.line_starts[line];
        let column = self.utf16_column(line, offset - line_start);

        (line as u32, column as u32)
    }

    /// Convert position to byte offset
    pub fn position_to_offset(&mut self, line: u32, character: u32) -> Option<usize> {
        let line = line as usize;
        if line >= self.line_starts.len() {
            return None;
        }

        let line_start = self.line_starts[line];
        let line_end = if line + 1 < self.line_starts.len() {
            // Don't subtract 1 - include the newline in the line
            self.line_starts[line + 1]
        } else {
            self.text.len()
        };

        // Get the full line including newline
        let line_text = &self.text[line_start..line_end];

        // Find the byte offset for the UTF-16 character position
        let byte_offset = self.utf16_to_byte_offset(line_text, character as usize)?;

        Some(line_start + byte_offset)
    }

    /// Get UTF-16 column from byte offset within a line
    fn utf16_column(&mut self, line: usize, byte_offset: usize) -> usize {
        let line_start = self.line_starts[line];

        // Get the text from line start to the target byte offset
        let target_byte = line_start + byte_offset;
        if target_byte > self.text.len() {
            return 0;
        }

        let line_text = &self.text[line_start..target_byte];

        // Count UTF-16 code units in the substring
        line_text.chars().map(|ch| ch.len_utf16()).sum()
    }

    /// Convert UTF-16 offset to byte offset within a line
    fn utf16_to_byte_offset(&self, line_text: &str, utf16_offset: usize) -> Option<usize> {
        let mut current_utf16 = 0;

        for (byte_offset, ch) in line_text.char_indices() {
            if current_utf16 == utf16_offset {
                return Some(byte_offset);
            }
            current_utf16 += ch.len_utf16();
            if current_utf16 > utf16_offset {
                // UTF-16 offset is in the middle of a character
                return None;
            }
        }

        // Check if we're at the end of the line
        if current_utf16 == utf16_offset {
            Some(line_text.len())
        } else {
            None
        }
    }

    // Remove unused compute_utf16_line method

    /// Create a range from byte offsets
    pub fn range(&mut self, start: usize, end: usize) -> ((u32, u32), (u32, u32)) {
        let start_pos = self.offset_to_position(start);
        let end_pos = self.offset_to_position(end);
        (start_pos, end_pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ascii_positions() {
        let mut index = LineIndex::new("hello\nworld\n".to_string());

        // First line
        assert_eq!(index.offset_to_position(0), (0, 0));
        assert_eq!(index.offset_to_position(5), (0, 5));

        // Second line
        assert_eq!(index.offset_to_position(6), (1, 0));
        assert_eq!(index.offset_to_position(11), (1, 5));
    }

    #[test]
    fn test_utf16_positions() {
        // Emoji takes 2 UTF-16 code units
        let mut index = LineIndex::new("my $😀 = 1;\n".to_string());

        // Before emoji
        assert_eq!(index.offset_to_position(0), (0, 0));
        assert_eq!(index.offset_to_position(4), (0, 4));

        // After emoji (emoji is 4 bytes but 2 UTF-16 units)
        let after_emoji = "my $😀".len();
        assert_eq!(index.offset_to_position(after_emoji), (0, 6));
    }

    #[test]
    fn test_position_to_offset() {
        let mut index = LineIndex::new("hello\nworld\n".to_string());

        assert_eq!(index.position_to_offset(0, 0), Some(0));
        assert_eq!(index.position_to_offset(0, 5), Some(5));
        assert_eq!(index.position_to_offset(1, 0), Some(6));
        assert_eq!(index.position_to_offset(1, 5), Some(11));
    }

    #[test]
    fn test_utf16_roundtrip() {
        // Test simple ASCII
        let text = "hello\nworld\n";
        let mut index = LineIndex::new(text.to_string());

        for offset in [0, 5, 6, 11, 12] {
            let (line, col) = index.offset_to_position(offset);
            let back = index.position_to_offset(line, col);
            assert_eq!(
                back,
                Some(offset),
                "Failed roundtrip at offset {} (line={}, col={})",
                offset,
                line,
                col
            );
        }

        // Test with multi-byte characters
        let text2 = "café\n";
        let mut index2 = LineIndex::new(text2.to_string());

        // Only test at character boundaries
        for offset in [0, 1, 2, 3, 5, 6] {
            // Skip 4 which is mid-character
            let (line, col) = index2.offset_to_position(offset);
            let back = index2.position_to_offset(line, col);
            assert_eq!(
                back,
                Some(offset),
                "Failed roundtrip at offset {} (line={}, col={})",
                offset,
                line,
                col
            );
        }
    }
}
