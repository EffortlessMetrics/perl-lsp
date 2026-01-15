/// Utility module for efficient position conversions with line_starts cache
use ropey::Rope;

/// Cache of line start positions for O(log n) position conversion
#[derive(Debug, Clone)]
pub struct LineStartsCache {
    /// Line start byte offsets
    line_starts: Vec<usize>,
}

impl LineStartsCache {
    /// Build a cache of line start positions
    pub fn new(text: &str) -> Self {
        let mut line_starts = vec![0];

        let mut i = 0;
        let bytes = text.as_bytes();

        while i < bytes.len() {
            if bytes[i] == b'\n' {
                line_starts.push(i + 1);
            } else if bytes[i] == b'\r' {
                if i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
                    // CRLF
                    line_starts.push(i + 2);
                    i += 1; // Skip the \n
                } else {
                    // Solo \r
                    line_starts.push(i + 1);
                }
            }
            i += 1;
        }

        LineStartsCache { line_starts }
    }

    /// Build a cache from a rope (avoids extra string allocation)
    pub fn new_rope(rope: &Rope) -> Self {
        let mut line_starts = vec![0];

        for line_idx in 0..rope.len_lines() {
            if line_idx > 0 {
                line_starts.push(rope.line_to_byte(line_idx));
            }
        }

        LineStartsCache { line_starts }
    }

    /// Convert byte offset to (line, utf16_column) using binary search
    pub fn offset_to_position(&self, text: &str, offset: usize) -> (u32, u32) {
        let offset = offset.min(text.len());

        // Binary search for the line
        let line = match self.line_starts.binary_search(&offset) {
            Ok(idx) => idx,
            Err(idx) => idx.saturating_sub(1),
        };

        let line_start = self.line_starts[line];
        let line_text = &text[line_start..offset];

        // Count UTF-16 units in this line segment
        let utf16_col = count_utf16_units(line_text);

        (line as u32, utf16_col as u32)
    }

    /// Convert (line, utf16_column) to byte offset with direct lookup
    pub fn position_to_offset(&self, text: &str, line: u32, character: u32) -> usize {
        let line = line as usize;

        if line >= self.line_starts.len() {
            return text.len();
        }

        let line_start = self.line_starts[line];
        let line_end = if line + 1 < self.line_starts.len() {
            // Find the actual end of the line (before \r or \n)
            let next_start = self.line_starts[line + 1];
            let mut end = next_start.saturating_sub(1);

            // Skip back over \n and \r
            let bytes = text.as_bytes();
            while end > line_start && (bytes[end] == b'\n' || bytes[end] == b'\r') {
                end = end.saturating_sub(1);
            }
            end + 1
        } else {
            text.len()
        };

        let line_text = &text[line_start..line_end];
        let byte_offset = utf16_position_to_byte_offset(line_text, character as usize);

        line_start + byte_offset
    }

    /// Convert byte offset to (line, utf16_column) using a Rope
    pub fn offset_to_position_rope(&self, rope: &Rope, offset: usize) -> (u32, u32) {
        let offset = offset.min(rope.len_bytes());
        let line = match self.line_starts.binary_search(&offset) {
            Ok(idx) => idx,
            Err(idx) => idx.saturating_sub(1),
        };
        let line_start = self.line_starts[line];
        let slice = rope.byte_slice(line_start..offset);
        let utf16_col: usize = slice.chars().map(|c| c.len_utf16()).sum();
        (line as u32, utf16_col as u32)
    }

    /// Convert (line, utf16_column) to byte offset using a Rope
    pub fn position_to_offset_rope(&self, rope: &Rope, line: u32, character: u32) -> usize {
        let line = line as usize;
        if line >= self.line_starts.len() {
            return rope.len_bytes();
        }
        let line_start = self.line_starts[line];
        let line_end = if line + 1 < self.line_starts.len() {
            self.line_starts[line + 1]
        } else {
            rope.len_bytes()
        };
        let slice = rope.byte_slice(line_start..line_end);
        let byte_offset = utf16_position_to_byte_offset(&slice.to_string(), character as usize);
        line_start + byte_offset
    }
}

/// Count UTF-16 code units in a string
fn count_utf16_units(s: &str) -> usize {
    s.chars().map(|c| c.len_utf16()).sum()
}

/// Convert UTF-16 position to byte offset
fn utf16_position_to_byte_offset(s: &str, utf16_pos: usize) -> usize {
    let mut utf16_count = 0;
    let mut byte_offset = 0;

    for ch in s.chars() {
        if utf16_count >= utf16_pos {
            break;
        }
        utf16_count += ch.len_utf16();
        byte_offset += ch.len_utf8();
    }

    // Clamp to string length
    byte_offset.min(s.len())
}

/// LSP Position
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

/// LSP Range [start inclusive, end exclusive] in (line, character) space
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

impl Position {
    pub fn new(line: u32, character: u32) -> Self {
        Self { line, character }
    }
}

impl Range {
    pub fn new(start_line: u32, start_char: u32, end_line: u32, end_char: u32) -> Self {
        Self {
            start: Position::new(start_line, start_char),
            end: Position::new(end_line, end_char),
        }
    }
}

/// Check if a position is within a range [start inclusive, end exclusive]
pub fn pos_in_range(pos: Position, range: Range) -> bool {
    // Position is before start of range
    if pos.line < range.start.line {
        return false;
    }
    if pos.line == range.start.line && pos.character < range.start.character {
        return false;
    }

    // Position is after end of range (end is exclusive)
    if pos.line > range.end.line {
        return false;
    }
    if pos.line == range.end.line && pos.character >= range.end.character {
        return false;
    }

    true
}

/// Compare two positions. Returns true if pos1 < pos2
pub fn pos_before(pos1: Position, pos2: Position) -> bool {
    pos1.line < pos2.line || (pos1.line == pos2.line && pos1.character < pos2.character)
}

/// Compare two positions. Returns true if pos1 <= pos2
pub fn pos_before_or_equal(pos1: Position, pos2: Position) -> bool {
    pos1.line < pos2.line || (pos1.line == pos2.line && pos1.character <= pos2.character)
}

/// Document state with cached line starts
pub struct CachedDocumentState {
    pub text: String,
    pub line_cache: LineStartsCache,
}

impl CachedDocumentState {
    pub fn new(text: String) -> Self {
        let line_cache = LineStartsCache::new(&text);
        Self { text, line_cache }
    }

    pub fn update(&mut self, new_text: String) {
        self.line_cache = LineStartsCache::new(&new_text);
        self.text = new_text;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_starts_cache() {
        let text = "line1\nline2\r\nline3\rline4";
        let cache = LineStartsCache::new(text);

        // Should have line starts at: 0, 6, 13, 19
        assert_eq!(cache.line_starts, vec![0, 6, 13, 19]);

        // Test offset to position
        assert_eq!(cache.offset_to_position(text, 0), (0, 0)); // Start of line1
        assert_eq!(cache.offset_to_position(text, 5), (0, 5)); // End of line1
        assert_eq!(cache.offset_to_position(text, 6), (1, 0)); // Start of line2
        assert_eq!(cache.offset_to_position(text, 11), (1, 5)); // End of line2 (before \r)
        assert_eq!(cache.offset_to_position(text, 13), (2, 0)); // Start of line3
        assert_eq!(cache.offset_to_position(text, 19), (3, 0)); // Start of line4

        // Test position to offset
        assert_eq!(cache.position_to_offset(text, 0, 0), 0); // Start of line1
        assert_eq!(cache.position_to_offset(text, 0, 5), 5); // End of line1
        assert_eq!(cache.position_to_offset(text, 1, 0), 6); // Start of line2
        assert_eq!(cache.position_to_offset(text, 1, 5), 11); // End of line2
        assert_eq!(cache.position_to_offset(text, 2, 0), 13); // Start of line3
        assert_eq!(cache.position_to_offset(text, 3, 0), 19); // Start of line4
    }

    #[test]
    fn test_emoji_with_cache() {
        let text = "🐍\ntest";
        let cache = LineStartsCache::new(text);

        // Snake emoji is 2 UTF-16 units, 4 bytes
        assert_eq!(cache.offset_to_position(text, 0), (0, 0)); // Before emoji
        assert_eq!(cache.offset_to_position(text, 4), (0, 2)); // After emoji
        assert_eq!(cache.offset_to_position(text, 5), (1, 0)); // Start of "test"

        assert_eq!(cache.position_to_offset(text, 0, 0), 0); // Before emoji
        assert_eq!(cache.position_to_offset(text, 0, 2), 4); // After emoji
        assert_eq!(cache.position_to_offset(text, 1, 0), 5); // Start of "test"
    }

    #[test]
    fn test_pos_in_range() {
        let range = Range::new(1, 0, 3, 0); // Lines 1-2 (line 3 excluded)

        // Before range
        assert!(!pos_in_range(Position::new(0, 0), range));
        assert!(!pos_in_range(Position::new(0, 10), range));

        // At start of range (inclusive)
        assert!(pos_in_range(Position::new(1, 0), range));

        // Inside range
        assert!(pos_in_range(Position::new(1, 5), range));
        assert!(pos_in_range(Position::new(2, 0), range));
        assert!(pos_in_range(Position::new(2, 10), range));

        // At end of range (exclusive)
        assert!(!pos_in_range(Position::new(3, 0), range));

        // After range
        assert!(!pos_in_range(Position::new(3, 5), range));
        assert!(!pos_in_range(Position::new(4, 0), range));
    }

    #[test]
    fn test_pos_in_range_same_line() {
        let range = Range::new(2, 5, 2, 10); // Characters 5-9 on line 2

        // Before range on same line
        assert!(!pos_in_range(Position::new(2, 0), range));
        assert!(!pos_in_range(Position::new(2, 4), range));

        // At start of range (inclusive)
        assert!(pos_in_range(Position::new(2, 5), range));

        // Inside range
        assert!(pos_in_range(Position::new(2, 7), range));
        assert!(pos_in_range(Position::new(2, 9), range));

        // At end of range (exclusive)
        assert!(!pos_in_range(Position::new(2, 10), range));

        // After range on same line
        assert!(!pos_in_range(Position::new(2, 15), range));
    }

    #[test]
    fn test_pos_comparisons() {
        let pos1 = Position::new(1, 5);
        let pos2 = Position::new(2, 0);
        let pos3 = Position::new(1, 10);

        // pos1 < pos2 (different lines)
        assert!(pos_before(pos1, pos2));
        assert!(!pos_before(pos2, pos1));

        // pos1 < pos3 (same line, different characters)
        assert!(pos_before(pos1, pos3));
        assert!(!pos_before(pos3, pos1));

        // Equal positions
        assert!(!pos_before(pos1, pos1));
        assert!(pos_before_or_equal(pos1, pos1));
    }
}
