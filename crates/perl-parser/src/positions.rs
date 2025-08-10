/// Utility module for efficient position conversions with line_starts cache

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
        assert_eq!(cache.offset_to_position(text, 0), (0, 0));   // Start of line1
        assert_eq!(cache.offset_to_position(text, 5), (0, 5));   // End of line1
        assert_eq!(cache.offset_to_position(text, 6), (1, 0));   // Start of line2
        assert_eq!(cache.offset_to_position(text, 11), (1, 5));  // End of line2 (before \r)
        assert_eq!(cache.offset_to_position(text, 13), (2, 0));  // Start of line3
        assert_eq!(cache.offset_to_position(text, 19), (3, 0));  // Start of line4
        
        // Test position to offset
        assert_eq!(cache.position_to_offset(text, 0, 0), 0);    // Start of line1
        assert_eq!(cache.position_to_offset(text, 0, 5), 5);    // End of line1
        assert_eq!(cache.position_to_offset(text, 1, 0), 6);    // Start of line2
        assert_eq!(cache.position_to_offset(text, 1, 5), 11);   // End of line2
        assert_eq!(cache.position_to_offset(text, 2, 0), 13);   // Start of line3
        assert_eq!(cache.position_to_offset(text, 3, 0), 19);   // Start of line4
    }
    
    #[test]
    fn test_emoji_with_cache() {
        let text = "🐍\ntest";
        let cache = LineStartsCache::new(text);
        
        // Snake emoji is 2 UTF-16 units, 4 bytes
        assert_eq!(cache.offset_to_position(text, 0), (0, 0));   // Before emoji
        assert_eq!(cache.offset_to_position(text, 4), (0, 2));   // After emoji
        assert_eq!(cache.offset_to_position(text, 5), (1, 0));   // Start of "test"
        
        assert_eq!(cache.position_to_offset(text, 0, 0), 0);    // Before emoji
        assert_eq!(cache.position_to_offset(text, 0, 2), 4);    // After emoji
        assert_eq!(cache.position_to_offset(text, 1, 0), 5);    // Start of "test"
    }
}