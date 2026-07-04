//! The one internal range format for the substrate.
//!
//! Core facts store **byte offsets plus 0-based UTF-8 line/column**. UTF-16 LSP
//! positions are never stored here — a consumer that needs them converts at the
//! LSP boundary from `start_byte`/`end_byte` and the source text (see
//! `NATIVE_STACK_POLICY.md`). Keeping one byte-and-UTF-8 format in the core
//! avoids re-deriving positions per consumer and keeps the substrate free of
//! `lsp-types`.

use serde::{Deserialize, Serialize};

/// A source span: byte offsets plus 0-based UTF-8 line/column endpoints.
///
/// `*_column_utf8` is the column measured in **UTF-8 code units (bytes)** from
/// the start of the line — unambiguous and cheap to compute. The LSP boundary
/// converts to UTF-16 when it needs to; the substrate never does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceRange {
    /// Inclusive start byte offset into the file.
    pub start_byte: u32,
    /// Exclusive end byte offset into the file.
    pub end_byte: u32,
    /// 0-based start line.
    pub start_line: u32,
    /// 0-based start column, in UTF-8 code units from the line start.
    pub start_column_utf8: u32,
    /// 0-based end line.
    pub end_line: u32,
    /// 0-based end column, in UTF-8 code units from the line start.
    pub end_column_utf8: u32,
}

/// Byte-offset → (line, UTF-8 column) index over a single file's text.
///
/// This is deliberately *not* `perl-position-tracking::LineIndex`, whose
/// `range()` yields UTF-16 columns. The substrate stores UTF-8 columns, so it
/// owns this small, allocation-light index instead.
pub struct Utf8LineIndex {
    /// Byte offset of each line start (index 0 is always byte 0).
    line_starts: Vec<u32>,
    /// Total length of the source in bytes.
    len: u32,
}

impl Utf8LineIndex {
    /// Build the index from source text.
    ///
    /// Handles `\n`, `\r\n`, and lone `\r` line endings (matching the parser's
    /// line index), so line numbers agree with the rest of the stack.
    #[must_use]
    pub fn new(text: &str) -> Self {
        let bytes = text.as_bytes();
        let mut line_starts = vec![0u32];
        let mut i = 0usize;
        while i < bytes.len() {
            match bytes[i] {
                b'\n' => line_starts.push(u32::try_from(i + 1).unwrap_or(u32::MAX)),
                b'\r' => {
                    let next = if i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
                        i += 1;
                        i + 1
                    } else {
                        i + 1
                    };
                    line_starts.push(u32::try_from(next).unwrap_or(u32::MAX));
                }
                _ => {}
            }
            i += 1;
        }
        Self { line_starts, len: u32::try_from(bytes.len()).unwrap_or(u32::MAX) }
    }

    /// Convert a byte offset to a 0-based `(line, utf8_column)`.
    ///
    /// Out-of-range offsets are clamped to the end of the text — the substrate
    /// never panics on a bad span.
    #[must_use]
    pub fn line_col(&self, byte: u32) -> (u32, u32) {
        let byte = byte.min(self.len);
        // Largest line_start <= byte.
        let line = match self.line_starts.binary_search(&byte) {
            Ok(exact) => exact,
            Err(insert) => insert.saturating_sub(1),
        };
        let line_start = self.line_starts.get(line).copied().unwrap_or(0);
        (u32::try_from(line).unwrap_or(u32::MAX), byte.saturating_sub(line_start))
    }

    /// Build a [`SourceRange`] from a `(start_byte, end_byte)` span.
    #[must_use]
    pub fn source_range(&self, start_byte: u32, end_byte: u32) -> SourceRange {
        let (start_line, start_column_utf8) = self.line_col(start_byte);
        let (end_line, end_column_utf8) = self.line_col(end_byte);
        SourceRange {
            start_byte,
            end_byte,
            start_line,
            start_column_utf8,
            end_line,
            end_column_utf8,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_col_on_first_line() {
        let idx = Utf8LineIndex::new("package App;\n");
        assert_eq!(idx.line_col(0), (0, 0));
        assert_eq!(idx.line_col(8), (0, 8), "still line 0, column 8 (`App`)");
    }

    #[test]
    fn line_col_across_lines() {
        let idx = Utf8LineIndex::new("package App;\nsub run { 1 }\n");
        // `sub` starts right after the first newline (byte 13).
        assert_eq!(idx.line_col(13), (1, 0));
    }

    #[test]
    fn utf8_columns_count_bytes_not_codepoints() {
        // "é" is 2 UTF-8 bytes; the `x` after it is at byte column 2.
        let idx = Utf8LineIndex::new("é x");
        assert_eq!(idx.line_col(2), (0, 2), "byte column after the 2-byte é");
    }

    #[test]
    fn crlf_line_endings() {
        let idx = Utf8LineIndex::new("a\r\nb");
        assert_eq!(idx.line_col(3), (1, 0), "b is the first byte of line 1");
    }

    #[test]
    fn out_of_range_offset_clamps() {
        let idx = Utf8LineIndex::new("abc");
        // Does not panic; clamps to end.
        let (line, col) = idx.line_col(9999);
        assert_eq!((line, col), (0, 3));
    }

    #[test]
    fn source_range_spans_both_endpoints() {
        let idx = Utf8LineIndex::new("package App;\nsub run { 1 }\n");
        let r = idx.source_range(0, 11);
        assert_eq!(r.start_byte, 0);
        assert_eq!(r.end_byte, 11);
        assert_eq!(r.start_line, 0);
        assert_eq!(r.end_line, 0);
    }
}
