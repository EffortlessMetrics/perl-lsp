//! Byte-oriented line/column indexing helpers.
//!
//! This crate has one responsibility: map byte offsets to `(line, column)`
//! and back using cached line starts.

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]

/// Line index for byte <-> (line, col) mapping.
#[derive(Clone, Debug)]
pub struct LineIndex {
    /// Byte offset of each line start.
    line_starts: Vec<usize>,
    /// Total UTF-8 byte length of the indexed text.
    text_len: usize,
}

impl LineIndex {
    /// Build a line index from UTF-8 text.
    #[must_use]
    pub fn new(text: &str) -> Self {
        let mut line_starts = vec![0];
        for (idx, ch) in text.char_indices() {
            if ch == '\n' {
                line_starts.push(idx + 1);
            }
        }
        Self { line_starts, text_len: text.len() }
    }

    /// Convert a byte offset to `(line, column)` using byte columns.
    #[must_use]
    pub fn byte_to_position(&self, byte: usize) -> (usize, usize) {
        let line = self.line_starts.binary_search(&byte).unwrap_or_else(|i| i.saturating_sub(1));
        let column = byte - self.line_starts[line];
        (line, column)
    }

    /// Convert `(line, column)` back to byte offset.
    ///
    /// Returns `None` when the line is out of range or when the column extends
    /// past the end of the line (including the newline character, but not the
    /// start of the next line).
    #[must_use]
    pub fn position_to_byte(&self, line: usize, column: usize) -> Option<usize> {
        let start = *self.line_starts.get(line)?;
        // line_end is the last addressable byte on this line (the newline char for
        // non-final lines, or text_len for the final line).  next_line_start itself
        // belongs to the *next* line, so we subtract one.
        let line_end = self
            .line_starts
            .get(line + 1)
            .map_or(self.text_len, |next_start| next_start.saturating_sub(1));
        let max_column = line_end.saturating_sub(start);

        if column > max_column {
            return None;
        }

        Some(start + column)
    }

    /// Convert `(line, column)` back to byte offset, returning `None` when
    /// the column crosses the line boundary.
    ///
    /// The newline character at the end of a line is the last addressable
    /// column on that line.  The byte at `next_line_start` belongs to the
    /// *next* line and is therefore out of range.
    #[must_use]
    pub fn position_to_byte_checked(&self, line: usize, column: usize) -> Option<usize> {
        let start = *self.line_starts.get(line)?;
        // Subtract one from next_line_start so the newline byte is reachable
        // but the first byte of the next line is not.
        let line_end = self
            .line_starts
            .get(line + 1)
            .map_or(self.text_len, |next_start| next_start.saturating_sub(1));
        let max_column = line_end.saturating_sub(start);

        if column > max_column {
            return None;
        }

        Some(start + column)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_string_has_one_line() {
        let idx = LineIndex::new("");
        assert_eq!(idx.byte_to_position(0), (0, 0));
        assert_eq!(idx.position_to_byte(0, 0), Some(0));
        assert_eq!(idx.position_to_byte(1, 0), None);
    }

    #[test]
    fn single_line_no_newline() {
        let idx = LineIndex::new("hello");
        assert_eq!(idx.byte_to_position(0), (0, 0));
        assert_eq!(idx.byte_to_position(4), (0, 4));
        assert_eq!(idx.position_to_byte(0, 0), Some(0));
        assert_eq!(idx.position_to_byte(0, 4), Some(4));
        assert_eq!(idx.position_to_byte(0, 5), Some(5));
        assert_eq!(idx.position_to_byte(0, 6), None);
    }

    #[test]
    fn two_lines_byte_to_position() {
        // "ab\ncd"  bytes: a=0, b=1, \n=2, c=3, d=4
        let idx = LineIndex::new("ab\ncd");
        assert_eq!(idx.byte_to_position(0), (0, 0));
        assert_eq!(idx.byte_to_position(1), (0, 1));
        assert_eq!(idx.byte_to_position(2), (0, 2)); // the newline is on line 0
        assert_eq!(idx.byte_to_position(3), (1, 0));
        assert_eq!(idx.byte_to_position(4), (1, 1));
    }

    #[test]
    fn two_lines_position_to_byte() {
        let idx = LineIndex::new("ab\ncd");
        assert_eq!(idx.position_to_byte(0, 0), Some(0));
        assert_eq!(idx.position_to_byte(0, 2), Some(2)); // newline byte
        assert_eq!(idx.position_to_byte(1, 0), Some(3));
        assert_eq!(idx.position_to_byte(1, 1), Some(4));
        assert_eq!(idx.position_to_byte(1, 2), Some(5)); // last line, end of text
        assert_eq!(idx.position_to_byte(1, 3), None); // beyond text
        assert_eq!(idx.position_to_byte(2, 0), None); // no third line
    }

    #[test]
    fn position_to_byte_checked_excludes_newline_as_next_line_start() {
        // "ab\ncd"
        let idx = LineIndex::new("ab\ncd");
        // Line 0 ends at the newline (byte 2); col 2 = newline byte is still on line 0
        assert_eq!(idx.position_to_byte_checked(0, 2), Some(2));
        // col 3 is the first byte of line 1 — out of range for line 0
        assert_eq!(idx.position_to_byte_checked(0, 3), None);
        assert_eq!(idx.position_to_byte_checked(1, 0), Some(3));
        assert_eq!(idx.position_to_byte_checked(2, 0), None);
    }

    #[test]
    fn trailing_newline_creates_empty_last_line() {
        // "foo\n" — line 1 starts at byte 4 and is empty
        let idx = LineIndex::new("foo\n");
        assert_eq!(idx.byte_to_position(3), (0, 3)); // newline
        assert_eq!(idx.byte_to_position(4), (1, 0)); // empty last line start
        assert_eq!(idx.position_to_byte(1, 0), Some(4));
    }

    #[test]
    fn multiple_lines_roundtrip() {
        let text = "line0\nline1\nline2";
        let idx = LineIndex::new(text);
        for (byte, _) in text.char_indices() {
            let (line, col) = idx.byte_to_position(byte);
            assert_eq!(idx.position_to_byte(line, col), Some(byte));
        }
    }

    // -----------------------------------------------------------------------
    // position_to_byte_checked: empty document
    // -----------------------------------------------------------------------

    #[test]
    fn checked_empty_string_origin_is_addressable() -> Result<(), Box<dyn std::error::Error>> {
        let idx = LineIndex::new("");
        // (0, 0) is the only valid position in an empty document
        assert_eq!(idx.position_to_byte_checked(0, 0), Some(0));
        Ok(())
    }

    #[test]
    fn checked_empty_string_col_one_out_of_range() -> Result<(), Box<dyn std::error::Error>> {
        let idx = LineIndex::new("");
        // text_len = 0, max_column = 0, so col 1 is beyond range
        assert_eq!(idx.position_to_byte_checked(0, 1), None);
        Ok(())
    }

    #[test]
    fn checked_empty_string_line_one_out_of_range() -> Result<(), Box<dyn std::error::Error>> {
        let idx = LineIndex::new("");
        assert_eq!(idx.position_to_byte_checked(1, 0), None);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // position_to_byte_checked: single line, no newline
    // -----------------------------------------------------------------------

    #[test]
    fn checked_single_line_all_columns_in_range() -> Result<(), Box<dyn std::error::Error>> {
        // "hello" = 5 bytes; text_len = 5; max_column on line 0 = 5 - 0 = 5
        let idx = LineIndex::new("hello");
        assert_eq!(idx.position_to_byte_checked(0, 0), Some(0));
        assert_eq!(idx.position_to_byte_checked(0, 4), Some(4)); // last char 'o'
        assert_eq!(idx.position_to_byte_checked(0, 5), Some(5)); // one past last char (end-of-file position)
        Ok(())
    }

    #[test]
    fn checked_single_line_col_beyond_text_len_is_none() -> Result<(), Box<dyn std::error::Error>> {
        let idx = LineIndex::new("hello");
        // max_column = 5 (text_len - line_start = 5 - 0 = 5); col 6 is out of range
        assert_eq!(idx.position_to_byte_checked(0, 6), None);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // position_to_byte_checked: non-final line newline boundary
    // -----------------------------------------------------------------------

    #[test]
    fn checked_newline_byte_is_last_addressable_on_its_line()
    -> Result<(), Box<dyn std::error::Error>> {
        // "abc\ndef": line 0 bytes 0-3 ('\n' at 3), line 1 bytes 4-6
        let idx = LineIndex::new("abc\ndef");
        // column 3 = newline byte — the last addressable byte on line 0
        assert_eq!(idx.position_to_byte_checked(0, 3), Some(3));
        Ok(())
    }

    #[test]
    fn checked_next_line_start_is_not_addressable_on_current_line()
    -> Result<(), Box<dyn std::error::Error>> {
        let idx = LineIndex::new("abc\ndef");
        // column 4 would be 'd' (first byte of line 1) — out of range for line 0
        assert_eq!(idx.position_to_byte_checked(0, 4), None);
        assert_eq!(idx.position_to_byte_checked(0, 100), None);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // position_to_byte_checked: trailing newline (empty final line)
    // -----------------------------------------------------------------------

    #[test]
    fn checked_trailing_newline_empty_final_line_origin() -> Result<(), Box<dyn std::error::Error>>
    {
        // "foo\n": line 0 = "foo\n" (0-3), line 1 starts at 4 (empty)
        let idx = LineIndex::new("foo\n");
        // line 1 is empty; text_len = 4, line_start = 4 → max_column = 0
        assert_eq!(idx.position_to_byte_checked(1, 0), Some(4));
        Ok(())
    }

    #[test]
    fn checked_trailing_newline_empty_final_line_col_one_is_none()
    -> Result<(), Box<dyn std::error::Error>> {
        let idx = LineIndex::new("foo\n");
        assert_eq!(idx.position_to_byte_checked(1, 1), None);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // position_to_byte_checked: CRLF sequences
    // -----------------------------------------------------------------------

    #[test]
    fn checked_crlf_cr_byte_addressable_on_its_line() -> Result<(), Box<dyn std::error::Error>> {
        // "ab\r\ncd": '\r' at byte 2, '\n' at byte 3 → line 1 starts at 4
        // Line 0 ends at byte 3 (the '\n'); max_column = 3 - 0 = 3
        let idx = LineIndex::new("ab\r\ncd");
        // '\r' (col 2) is within line 0
        assert_eq!(idx.position_to_byte_checked(0, 2), Some(2));
        // '\n' (col 3) is the last addressable byte on line 0
        assert_eq!(idx.position_to_byte_checked(0, 3), Some(3));
        // col 4 is the start of line 1 — out of range for line 0
        assert_eq!(idx.position_to_byte_checked(0, 4), None);
        Ok(())
    }

    #[test]
    fn checked_crlf_second_line() -> Result<(), Box<dyn std::error::Error>> {
        // "ab\r\ncd": line 1 starts at byte 4; text_len = 6; max_column = 6 - 4 = 2
        let idx = LineIndex::new("ab\r\ncd");
        assert_eq!(idx.position_to_byte_checked(1, 0), Some(4)); // 'c'
        assert_eq!(idx.position_to_byte_checked(1, 1), Some(5)); // 'd'
        assert_eq!(idx.position_to_byte_checked(1, 2), Some(6)); // end-of-file position
        assert_eq!(idx.position_to_byte_checked(1, 3), None); // beyond text
        Ok(())
    }

    // -----------------------------------------------------------------------
    // position_to_byte_checked: Unicode multi-byte chars
    // -----------------------------------------------------------------------

    #[test]
    fn checked_unicode_two_byte_char_boundary() -> Result<(), Box<dyn std::error::Error>> {
        // "caf\u{00e9}" = c(0) a(1) f(2) é(3,4) — 5 bytes total
        // Single line; text_len = 5; max_column = 5
        let idx = LineIndex::new("caf\u{00e9}");
        assert_eq!(idx.position_to_byte_checked(0, 3), Some(3)); // start of é
        assert_eq!(idx.position_to_byte_checked(0, 4), Some(4)); // second byte of é
        assert_eq!(idx.position_to_byte_checked(0, 5), Some(5)); // end-of-file position
        assert_eq!(idx.position_to_byte_checked(0, 6), None); // beyond text
        Ok(())
    }

    #[test]
    fn checked_unicode_multiline_second_line_boundary() -> Result<(), Box<dyn std::error::Error>> {
        // "a\u{00e9}\nb" — a(0) é(1,2) \n(3) b(4) = 5 bytes
        // Line 0: bytes 0-3 (max_column = (3+1-1) - 0 = 3)
        // Line 1: bytes 4-4 (max_column = 5 - 4 = 1)
        let idx = LineIndex::new("a\u{00e9}\nb");
        assert_eq!(idx.position_to_byte_checked(0, 3), Some(3)); // '\n'
        assert_eq!(idx.position_to_byte_checked(0, 4), None); // next line start
        assert_eq!(idx.position_to_byte_checked(1, 0), Some(4)); // 'b'
        assert_eq!(idx.position_to_byte_checked(1, 1), Some(5)); // end-of-file position
        assert_eq!(idx.position_to_byte_checked(1, 2), None); // beyond text
        Ok(())
    }

    // -----------------------------------------------------------------------
    // position_to_byte vs position_to_byte_checked equivalence on final line
    // -----------------------------------------------------------------------

    #[test]
    fn checked_and_unchecked_agree_on_final_line() -> Result<(), Box<dyn std::error::Error>> {
        // For the final line (no next-line-start constraint), both methods should
        // produce identical results for any column value.
        let idx = LineIndex::new("abc\nxyz");
        for col in 0..=5 {
            assert_eq!(
                idx.position_to_byte(1, col),
                idx.position_to_byte_checked(1, col),
                "methods diverged at col {col} on final line"
            );
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // byte_to_position: offset at text_len (one past end)
    // -----------------------------------------------------------------------

    #[test]
    fn byte_to_position_at_text_len_single_line() -> Result<(), Box<dyn std::error::Error>> {
        // "hi" has text_len = 2; byte 2 is one past the end, still maps to line 0
        let idx = LineIndex::new("hi");
        assert_eq!(idx.byte_to_position(2), (0, 2));
        Ok(())
    }

    #[test]
    fn byte_to_position_at_text_len_multiline() -> Result<(), Box<dyn std::error::Error>> {
        // "a\nb": line 1 starts at byte 2 ('b' at byte 2); text_len = 3
        // byte 3 is one past the end — still on line 1, column = 3 - 2 = 1
        let idx = LineIndex::new("a\nb");
        assert_eq!(idx.byte_to_position(3), (1, 1));
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Clone and Debug derives
    // -----------------------------------------------------------------------

    #[test]
    fn clone_preserves_index_state() -> Result<(), Box<dyn std::error::Error>> {
        let idx = LineIndex::new("foo\nbar");
        let cloned = idx.clone();
        // Both originals and clones should produce the same result
        assert_eq!(idx.byte_to_position(4), cloned.byte_to_position(4));
        assert_eq!(idx.position_to_byte(1, 0), cloned.position_to_byte(1, 0));
        Ok(())
    }

    #[test]
    fn debug_format_is_non_empty() -> Result<(), Box<dyn std::error::Error>> {
        let idx = LineIndex::new("test\ndata");
        let debug_str = format!("{idx:?}");
        assert!(!debug_str.is_empty());
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Consecutive newlines (empty lines in sequence)
    // -----------------------------------------------------------------------

    #[test]
    fn consecutive_newlines_produce_empty_lines() -> Result<(), Box<dyn std::error::Error>> {
        // "\n\n" → line 0 starts at 0, line 1 at 1, line 2 at 2
        let idx = LineIndex::new("\n\n");
        assert_eq!(idx.byte_to_position(0), (0, 0)); // first '\n'
        assert_eq!(idx.byte_to_position(1), (1, 0)); // second '\n'
        assert_eq!(idx.byte_to_position(2), (2, 0)); // past end

        assert_eq!(idx.position_to_byte(0, 0), Some(0));
        assert_eq!(idx.position_to_byte(1, 0), Some(1));
        assert_eq!(idx.position_to_byte(2, 0), Some(2));
        // line 3 does not exist
        assert_eq!(idx.position_to_byte(3, 0), None);
        Ok(())
    }
}
