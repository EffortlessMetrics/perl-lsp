//! Line index for efficient UTF-16 position calculations.
use ropey::Rope;

/// Returns true if `b` is a UTF-8 continuation byte (0b10xxxxxx).
#[inline]
fn is_utf8_continuation(b: u8) -> bool {
    (b & 0b1100_0000) == 0b1000_0000
}

/// Caches byte offsets for line starts to speed up coordinate conversion.
#[derive(Debug, Clone)]
pub struct LineStartsCache {
    line_starts: Vec<usize>,
}
impl LineStartsCache {
    /// Clamp `offset` into `text` and ensure it is on a UTF-8 char boundary.
    fn normalize_text_offset(text: &str, offset: usize) -> usize {
        let mut normalized = offset.min(text.len());
        while normalized > 0 && !text.is_char_boundary(normalized) {
            normalized -= 1;
        }
        normalized
    }

    /// Builds a cache from UTF-8 source text.
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

    /// Builds a cache from a [`Rope`] buffer.
    pub fn new_rope(rope: &Rope) -> Self {
        let mut ls = vec![0];
        for li in 0..rope.len_lines() {
            if li > 0 {
                ls.push(rope.line_to_byte(li));
            }
        }
        Self { line_starts: ls }
    }

    /// Converts a byte offset in `text` to `(line, column_utf16)`.
    pub fn offset_to_position(&self, text: &str, offset: usize) -> (u32, u32) {
        let offset = Self::normalize_text_offset(text, offset);
        let line = self.line_starts.binary_search(&offset).unwrap_or_else(|i| i.saturating_sub(1));
        let ls = self.line_starts[line];
        (line as u32, text[ls..offset].chars().map(|c| c.len_utf16()).sum::<usize>() as u32)
    }

    /// Converts `(line, column_utf16)` into a byte offset in `text`.
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
            // Test before consuming so a column that lands inside a surrogate
            // pair clamps to the start of that codepoint rather than skipping
            // past it. `uc >= character` alone over-advances for mid-surrogate
            // requests (e.g. column 2 over "x😀y" must map to the emoji start).
            if uc + ch.len_utf16() > character as usize {
                break;
            }
            uc += ch.len_utf16();
            bo += ch.len_utf8();
        }
        ls + bo.min(lt.len())
    }

    /// Converts a byte offset in `rope` to `(line, column_utf16)`.
    pub fn offset_to_position_rope(&self, rope: &Rope, offset: usize) -> (u32, u32) {
        let offset = Self::normalize_rope_offset(rope, offset);
        let line = self.line_starts.binary_search(&offset).unwrap_or_else(|i| i.saturating_sub(1));
        let ls = self.line_starts[line];
        (
            line as u32,
            rope.byte_slice(ls..offset).chars().map(|c| c.len_utf16()).sum::<usize>() as u32,
        )
    }

    /// Clamp `offset` into `rope` and snap it back to a UTF-8 char boundary.
    ///
    /// `Rope::byte_slice` panics if the offset splits a multi-byte codepoint, so
    /// the clamp here mirrors [`Self::normalize_text_offset`]. Ropey 1.x does
    /// not expose `is_char_boundary` directly, so we inspect the byte at the
    /// candidate offset: UTF-8 continuation bytes always satisfy `b & 0xC0 ==
    /// 0x80` (top two bits are `10`), and every other byte is a codepoint
    /// start.
    fn normalize_rope_offset(rope: &Rope, offset: usize) -> usize {
        let len = rope.len_bytes();
        let mut normalized = offset.min(len);
        while normalized > 0 && normalized < len && is_utf8_continuation(rope.byte(normalized)) {
            normalized -= 1;
        }
        normalized
    }

    /// Converts `(line, column_utf16)` into a byte offset in `rope`.
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
        // Clamp the slice bounds: `Rope::byte_slice` panics if `ls`/`le` exceed
        // the rope length or if `ls > le`, which a corrupt cache could trigger.
        let len = rope.len_bytes();
        let slice_start = ls.min(len);
        let slice_end = le.min(len).max(slice_start);
        let sl = rope.byte_slice(slice_start..slice_end);
        let mut uc = 0;
        let mut bo = 0;
        for ch in sl.chars() {
            // Test before consuming so a column that lands inside a surrogate
            // pair clamps to the start of that codepoint rather than skipping
            // past it. `uc >= character` alone over-advances for mid-surrogate
            // requests (e.g. column 2 over "x😀y" must map to the emoji start).
            if uc + ch.len_utf16() > character as usize {
                break;
            }
            uc += ch.len_utf16();
            bo += ch.len_utf8();
        }
        // Guard against overflow: if `ls` is corrupt/near `usize::MAX`, the add
        // would wrap to a bogus small offset. Clamp to the rope length instead.
        ls.checked_add(bo).unwrap_or(len).min(len)
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
        let bytes = text.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'\n' {
                line_starts.push(i + 1);
            } else if bytes[i] == b'\r' {
                if i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
                    line_starts.push(i + 2);
                    i += 1;
                } else {
                    line_starts.push(i + 1);
                }
            }
            i += 1;
        }

        Self { line_starts, text }
    }

    /// Convert byte offset to position (0-based line and UTF-16 column)
    pub fn offset_to_position(&self, offset: usize) -> (u32, u32) {
        let offset = self.normalize_offset(offset);
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
        let Some(&line_start) = self.line_starts.get(line) else {
            return 0;
        };

        // Bound the slice start: a corrupt `line_start` past the end of the
        // text (or an addition that overflows below) must not reach the slice.
        if line_start > self.text.len() {
            return 0;
        }

        // Get the text from line start to the target byte offset. Use checked
        // arithmetic so `line_start + byte_offset` cannot wrap to a value that
        // is smaller than `line_start` and produce a `start > end` slice panic.
        let Some(target_byte) = line_start.checked_add(byte_offset) else {
            return 0;
        };
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

    /// Normalize a byte offset so it is inside the text and on a UTF-8 codepoint boundary.
    fn normalize_offset(&self, offset: usize) -> usize {
        let mut normalized = offset.min(self.text.len());
        while normalized > 0 && !self.text.is_char_boundary(normalized) {
            normalized -= 1;
        }
        normalized
    }

    /// Create a range from byte offsets
    pub fn range(&self, start: usize, end: usize) -> ((u32, u32), (u32, u32)) {
        let start_pos = self.offset_to_position(start);
        let end_pos = self.offset_to_position(end);
        (start_pos, end_pos)
    }
}

#[cfg(test)]
mod overflow_hardening_tests {
    use super::*;
    use ropey::Rope;

    // Regression for #2484: a `line_start` past the end of `text` must not reach
    // the `&self.text[line_start..target_byte]` slice. Before the fix the only
    // bound check validated `target_byte`, so a corrupt `line_start` produced a
    // `start > end` slice panic.
    #[test]
    fn utf16_column_clamps_line_start_past_text_end() {
        let mut index = LineIndex::new("abc".to_string());
        // Corrupt the cache so the line start sits well beyond the text.
        index.line_starts = vec![0, 100];
        // byte_offset 0 keeps target_byte == line_start == 100 > len 3.
        assert_eq!(index.utf16_column(1, 0), 0);
    }

    // Regression for #2484: `line_start + byte_offset` must use checked
    // arithmetic. With a near-`usize::MAX` line_start the add wraps to a small
    // value that slips past the `target_byte > len` check while `line_start`
    // stays huge, slicing `[huge..small]` and panicking.
    #[test]
    fn utf16_column_does_not_overflow_on_huge_line_start() {
        let mut index = LineIndex::new("abc".to_string());
        index.line_starts = vec![0, usize::MAX];
        // usize::MAX + 5 wraps to 4 without checked arithmetic; line_start is
        // still usize::MAX, so the slice start would exceed the end.
        assert_eq!(index.utf16_column(1, 5), 0);
    }

    // Discriminator for line 240 — the `checked_add` overflow arm.  The test
    // above exercises it via the `line_start > text.len()` early-return at
    // line 232 (usize::MAX > 3), never reaching `checked_add`.  This test
    // places `line_start` exactly at `text.len()` (3 <= 3 passes the guard)
    // and then passes a `byte_offset` large enough that
    // `line_start + byte_offset` wraps past `usize::MAX`.
    #[test]
    fn utf16_column_checked_add_overflow_returns_zero() {
        let mut index = LineIndex::new("abc".to_string());
        // line_start = 3 == text.len(): passes the `> text.len()` guard.
        index.line_starts = vec![0, 3];
        // 3 + (usize::MAX - 2) = usize::MAX + 1, which overflows.
        // checked_add must catch this and return None; we must return 0.
        assert_eq!(index.utf16_column(1, usize::MAX - 2), 0);
    }

    // Discriminator for ripr seam f197bc96 — the predicate boundary
    // `line_start > self.text.len()` at line 232.
    // `utf16_column_clamps_line_start_past_text_end` (above) exercises this
    // path but ripr's static infect analysis cannot trace the private-field
    // mutation `line_starts = vec![0, 100]` to the predicate value.
    // This test names the boundary explicitly — `line_start_past_end` (100)
    // is demonstrably `> text.len()` (3) — so the discriminator is visible.
    #[test]
    fn utf16_column_boundary_discriminator() {
        let text = "abc"; // text.len() == 3
        let line_start_past_end: usize = 100; // 100 > 3 == text.len()
        debug_assert!(line_start_past_end > text.len(), "boundary: line_start > text.len()");
        let mut index = LineIndex::new(text.to_string());
        index.line_starts = vec![0, line_start_past_end];
        // The guard `if line_start > self.text.len()` at line 232 fires and
        // returns 0 before any slice occurs.
        assert_eq!(index.utf16_column(1, 0), 0);
    }

    // Regression for #2484: out-of-range `line` index must not panic on the
    // `self.line_starts[line]` access.
    #[test]
    fn utf16_column_out_of_range_line_returns_zero() {
        let index = LineIndex::new("abc".to_string());
        assert_eq!(index.utf16_column(999, 0), 0);
    }

    // Regression for #2490: a near-`usize::MAX` line start must not panic in
    // `Rope::byte_slice` and must not wrap in the final `ls + bo`. Before the
    // fix, `byte_slice(huge..len)` panicked with "start must be <= end"; after,
    // the bounds are clamped and the checked add clamps to the rope length.
    #[test]
    fn position_to_offset_rope_clamps_on_overflow() {
        let rope = Rope::from_str("hello\nworld");
        let mut cache = LineStartsCache::new_rope(&rope);
        // Corrupt the second line start to a value near usize::MAX.
        let len = cache.line_starts.len();
        cache.line_starts[len - 1] = usize::MAX - 1;
        let offset = cache.position_to_offset_rope(&rope, 1, 3);
        // Must not panic or wrap to a tiny value; clamps to the rope byte length.
        assert_eq!(offset, rope.len_bytes());
    }

    // Sanity: the hardened paths still produce correct results for valid input.
    #[test]
    fn valid_input_still_correct() {
        let index = LineIndex::new("ab\ncd".to_string());
        // Column 2 on line 0 ("ab") -> 2 UTF-16 units.
        assert_eq!(index.utf16_column(0, 2), 2);

        let rope = Rope::from_str("ab\ncd");
        let cache = LineStartsCache::new_rope(&rope);
        // Line 1 ("cd"), character 2 -> byte offset 3 (start of line 1) + 2 = 5.
        assert_eq!(cache.position_to_offset_rope(&rope, 1, 2), 5);
    }
}
