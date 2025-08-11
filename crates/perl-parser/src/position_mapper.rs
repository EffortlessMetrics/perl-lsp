//! Centralized position mapping for correct LSP position handling
//! 
//! Handles:
//! - CRLF/LF/CR line endings
//! - UTF-16 code units (LSP protocol)
//! - Byte offsets (parser)
//! - Efficient conversions using rope data structure

use ropey::Rope;
use serde_json::Value;

/// Position in the document
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    /// Zero-based line number
    pub line: u32,
    /// Zero-based character offset (UTF-16 code units)
    pub character: u32,
}

/// Centralized position mapper using rope for efficiency
pub struct PositionMapper {
    /// The rope containing the document text
    rope: Rope,
    /// Cache of line ending style
    line_ending: LineEnding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding {
    Lf,
    CrLf,
    Cr,
    Mixed,
}

impl PositionMapper {
    /// Create a new position mapper from text
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

    /// Convert LSP position to byte offset
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
            utf16_offset += ch_utf16_len;
            byte_offset += ch.len_utf8();
        }
        
        Some(line_start_byte + byte_offset)
    }

    /// Convert byte offset to LSP position
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
        
        Position {
            line: line_idx as u32,
            character: utf16_offset,
        }
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

/// Convert JSON LSP position to our Position type
pub fn json_to_position(pos: &Value) -> Option<Position> {
    Some(Position {
        line: pos["line"].as_u64()? as u32,
        character: pos["character"].as_u64()? as u32,
    })
}

/// Convert Position to JSON for LSP
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


/// Apply UTF-8 edit to a string
pub fn apply_edit_utf8(text: &mut String, start_byte: usize, old_end_byte: usize, replacement: &str) {
    if !text.is_char_boundary(start_byte) || !text.is_char_boundary(old_end_byte) {
        // Safety: ensure we're at char boundaries
        return;
    }
    text.replace_range(start_byte..old_end_byte, replacement);
}

/// Count newlines in text
pub fn newline_count(text: &str) -> usize {
    text.chars().filter(|&c| c == '\n').count()
}

/// Get the column (in UTF-8 bytes) of the last line
pub fn last_line_column_utf8(text: &str) -> u32 {
    if let Some(last_newline) = text.rfind('\n') {
        (text.len() - last_newline - 1) as u32
    } else {
        text.len() as u32
    }
}