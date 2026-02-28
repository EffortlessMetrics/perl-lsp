//! Line index for efficient UTF-16 position calculations.
use ropey::Rope;
#[derive(Debug, Clone)]
pub struct LineStartsCache {
    line_starts: Vec<usize>,
}
impl LineStartsCache {
    pub fn new(text: &str) -> Self {
        let mut ls = vec![0];
        let mut i = 0;
        let b = text.as_bytes();
        while i < b.len() {
            if b[i] == b'\n' {
                ls.push(i + 1);
            } else if b[i] == b'\r' {
                if i + 1 < b.len() && b[i + 1] == b'\n' {
                    ls.push(i + 2);
                    i += 1;
                } else {
                    ls.push(i + 1);
                }
            }
            i += 1;
        }
        Self { line_starts: ls }
    }
    pub fn new_rope(rope: &Rope) -> Self {
        let mut ls = vec![0];
        for li in 0..rope.len_lines() {
            if li > 0 {
                ls.push(rope.line_to_byte(li));
            }
        }
        Self { line_starts: ls }
    }
    pub fn offset_to_position(&self, text: &str, offset: usize) -> (u32, u32) {
        let offset = offset.min(text.len());
        let line = self.line_starts.binary_search(&offset).unwrap_or_else(|i| i.saturating_sub(1));
        let ls = self.line_starts[line];
        (line as u32, text[ls..offset].chars().map(|c| c.len_utf16()).sum::<usize>() as u32)
    }
    pub fn position_to_offset(&self, text: &str, line: u32, character: u32) -> usize {
        let line = line as usize;
        if line >= self.line_starts.len() {
            return text.len();
        }
        let ls = self.line_starts[line];
        let le = if line + 1 < self.line_starts.len() {
            let ns = self.line_starts[line + 1];
            let mut end = ns.saturating_sub(1);
            let b = text.as_bytes();
            while end > ls && (b[end] == b'\n' || b[end] == b'\r') {
                end = end.saturating_sub(1);
            }
            end + 1
        } else {
            text.len()
        };
        let lt = &text[ls..le];
        let mut uc = 0;
        let mut bo = 0;
        for ch in lt.chars() {
            if uc >= character as usize {
                break;
            }
            uc += ch.len_utf16();
            bo += ch.len_utf8();
        }
        ls + bo.min(lt.len())
    }
    pub fn offset_to_position_rope(&self, rope: &Rope, offset: usize) -> (u32, u32) {
        let offset = offset.min(rope.len_bytes());
        let line = self.line_starts.binary_search(&offset).unwrap_or_else(|i| i.saturating_sub(1));
        let ls = self.line_starts[line];
        (
            line as u32,
            rope.byte_slice(ls..offset).chars().map(|c| c.len_utf16()).sum::<usize>() as u32,
        )
    }
    pub fn position_to_offset_rope(&self, rope: &Rope, line: u32, character: u32) -> usize {
        let line = line as usize;
        if line >= self.line_starts.len() {
            return rope.len_bytes();
        }
        let ls = self.line_starts[line];
        let le = if line + 1 < self.line_starts.len() {
            self.line_starts[line + 1]
        } else {
            rope.len_bytes()
        };
        let sl = rope.byte_slice(ls..le);
        let mut uc = 0;
        let mut bo = 0;
        for ch in sl.chars() {
            if uc >= character as usize {
                break;
            }
            uc += ch.len_utf16();
            bo += ch.len_utf8();
        }
        ls + bo
    }
}

/// Stores line information for efficient position lookups, owning the text.
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
    pub fn offset_to_position(&self, offset: usize) -> (u32, u32) {
        let line = self.line_starts.binary_search(&offset).unwrap_or_else(|i| i.saturating_sub(1));

        let line_start = self.line_starts[line];
        let column = self.utf16_column(line, offset - line_start);

        (line as u32, column as u32)
    }

    /// Convert position to byte offset
    pub fn position_to_offset(&self, line: u32, character: u32) -> Option<usize> {
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
    fn utf16_column(&self, line: usize, byte_offset: usize) -> usize {
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
        if current_utf16 == utf16_offset { Some(line_text.len()) } else { None }
    }

    /// Create a range from byte offsets
    pub fn range(&self, start: usize, end: usize) -> ((u32, u32), (u32, u32)) {
        let start_pos = self.offset_to_position(start);
        let end_pos = self.offset_to_position(end);
        (start_pos, end_pos)
    }
}
