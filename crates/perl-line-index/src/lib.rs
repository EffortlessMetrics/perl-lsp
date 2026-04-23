//! Byte-oriented line/column indexing helpers.
//!
//! This crate has one responsibility: map byte offsets to `(line, column)`
//! and back using cached line starts.

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

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
    /// past the start of the next line (or the end of the document for the
    /// final line).
    #[must_use]
    pub fn position_to_byte(&self, line: usize, column: usize) -> Option<usize> {
        let start = *self.line_starts.get(line)?;
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
