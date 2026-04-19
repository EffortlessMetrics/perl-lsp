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
    let end = text[cursor..]
        .find('\n')
        .map_or(text.len(), |idx| cursor + idx);
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

/// Advance `idx` while bytes at the cursor are ASCII whitespace.
#[must_use]
pub fn skip_ascii_whitespace(bytes: &[u8], mut idx: usize) -> usize {
    while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
        idx += 1;
    }
    idx
}
