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
}
