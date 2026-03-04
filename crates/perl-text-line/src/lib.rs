//! Text-line cursor helpers.
//!
//! This crate has a single responsibility: map cursor offsets to line
//! boundaries and provide conservative token-boundary primitives for
//! single-line scanning.

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

/// Return the byte span of the line containing `cursor_pos`.
///
/// The returned range is inclusive of the first line byte and exclusive of
/// one past the last byte, matching half-open Rust range conventions.
#[must_use]
pub fn line_bounds_at(text: &str, cursor_pos: usize) -> (usize, usize) {
    let cursor = cursor_pos.min(text.len());
    let start = text[..cursor].rfind('\n').map_or(0, |idx| idx + 1);
    let end = text[cursor..].find('\n').map_or(text.len(), |idx| cursor + idx);
    (start, end)
}

/// Return `true` when `byte` is an identifier character (`[A-Za-z0-9_]`).
#[must_use]
pub fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// Return `true` when token `keyword` bytes in `[start, start + len)` are
/// bounded on both sides by non-identifier bytes.
#[must_use]
pub fn is_keyword_boundary(bytes: &[u8], start: usize, len: usize) -> bool {
    if start > bytes.len() {
        return false;
    }

    let end = start.saturating_add(len);
    if end > bytes.len() {
        return false;
    }

    if start > 0 && is_identifier_byte(bytes[start - 1]) {
        return false;
    }

    if end < bytes.len() && is_identifier_byte(bytes[end]) {
        return false;
    }

    true
}

/// Advance `idx` while bytes at the cursor are horizontal ASCII whitespace (` ` or `\t`).
#[must_use]
pub fn skip_ascii_whitespace(bytes: &[u8], mut idx: usize) -> usize {
    while idx < bytes.len() && (bytes[idx] == b' ' || bytes[idx] == b'\t') {
        idx += 1;
    }
    idx
}

/// Return the insertion offset at the start of the statement containing `pos`.
///
/// A statement boundary is considered to be either `;` or a newline.
#[must_use]
pub fn statement_start_before(text: &str, pos: usize) -> usize {
    let cursor = pos.min(text.len());
    if cursor == 0 {
        return 0;
    }

    let bytes = text.as_bytes();
    let mut idx = cursor.saturating_sub(1);
    while idx > 0 {
        if bytes[idx] == b';' || bytes[idx] == b'\n' {
            return idx + 1;
        }
        idx = idx.saturating_sub(1);
    }

    0
}

/// Return the leading whitespace prefix for the line containing `pos`.
#[must_use]
pub fn leading_indent_at(text: &str, pos: usize) -> String {
    let (line_start, line_end) = line_bounds_at(text, pos);
    let line = &text[line_start..line_end];
    let indent_len = line.as_bytes().iter().take_while(|b| **b == b' ' || **b == b'\t').count();
    line[..indent_len].to_string()
}

#[cfg(test)]
mod tests {
    use super::{leading_indent_at, statement_start_before};

    #[test]
    fn statement_start_detects_statement_boundaries() {
        let source = "my $x = 1;\n  say $x;\n";
        assert_eq!(statement_start_before(source, source.len()), 21);
        assert_eq!(statement_start_before(source, 8), 0);
    }

    #[test]
    fn leading_indent_returns_spaces_and_tabs() {
        let source = "first\n\t  second\nthird";
        assert_eq!(leading_indent_at(source, 9), "\t  ");
        assert_eq!(leading_indent_at(source, source.len()), "");
    }
}
