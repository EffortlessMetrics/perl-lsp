//! Line index for efficient UTF-16 position calculations
//!
//! LSP requires UTF-16 code unit positions, but Rust uses UTF-8.
//! This module provides efficient conversion between byte offsets,
//! line/column positions, and UTF-16 positions.

use std::collections::HashMap;

/// Stores line information for efficient position lookups
#[derive(Debug, Clone)]
pub struct LineIndex {
    /// Byte offset of each line start
    line_starts: Vec<usize>,
    /// UTF-16 code unit offsets for each line (lazy computed)
    utf16_lines: HashMap<usize, Vec<usize>>,
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
        
        Self {
            line_starts,
            utf16_lines: HashMap::new(),
            text,
        }
    }
    
    /// Convert byte offset to position (0-based line and UTF-16 column)
    pub fn offset_to_position(&mut self, offset: usize) -> (u32, u32) {
        let line = self.line_starts.binary_search(&offset)
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
            self.line_starts[line + 1] - 1
        } else {
            self.text.len()
        };
        
        let line_text = &self.text[line_start..line_end];
        let byte_offset = self.utf16_to_byte_offset(line_text, character as usize)?;
        
        Some(line_start + byte_offset)
    }
    
    /// Get UTF-16 column from byte offset within a line
    fn utf16_column(&mut self, line: usize, byte_offset: usize) -> usize {
        // Ensure UTF-16 offsets are computed for this line
        if !self.utf16_lines.contains_key(&line) {
            self.compute_utf16_line(line);
        }
        
        let utf16_offsets = &self.utf16_lines[&line];
        
        // Find the UTF-16 position for this byte offset
        let line_start = self.line_starts[line];
        let target = line_start + byte_offset;
        
        // Count UTF-16 code units up to the target byte
        let mut utf16_pos = 0;
        
        for &ch_start in utf16_offsets {
            if ch_start >= target {
                break;
            }
            // Count UTF-16 units for this character
            let ch_bytes = self.text[ch_start..].chars().next().unwrap();
            utf16_pos += ch_bytes.len_utf16();
        }
        
        utf16_pos
    }
    
    /// Convert UTF-16 offset to byte offset within a line
    fn utf16_to_byte_offset(&self, line_text: &str, utf16_offset: usize) -> Option<usize> {
        let mut current_utf16 = 0;
        let mut byte_offset = 0;
        
        for ch in line_text.chars() {
            if current_utf16 >= utf16_offset {
                return Some(byte_offset);
            }
            current_utf16 += ch.len_utf16();
            byte_offset += ch.len_utf8();
        }
        
        if current_utf16 == utf16_offset {
            Some(byte_offset)
        } else {
            None
        }
    }
    
    /// Compute UTF-16 offsets for a line
    fn compute_utf16_line(&mut self, line: usize) {
        let line_start = self.line_starts[line];
        let line_end = if line + 1 < self.line_starts.len() {
            self.line_starts[line + 1] - 1
        } else {
            self.text.len()
        };
        
        let mut offsets = Vec::new();
        for (i, _ch) in self.text[line_start..line_end].char_indices() {
            offsets.push(line_start + i);
        }
        
        self.utf16_lines.insert(line, offsets);
    }
    
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
        let text = "my $café = '☕';\nmy $π = 3.14;\n";
        let mut index = LineIndex::new(text.to_string());
        
        // Test various positions roundtrip correctly
        for offset in [0, 4, 8, 12, 16, 20] {
            if offset < text.len() {
                let (line, col) = index.offset_to_position(offset);
                let back = index.position_to_offset(line, col);
                assert_eq!(back, Some(offset), "Failed roundtrip at offset {}", offset);
            }
        }
    }
}