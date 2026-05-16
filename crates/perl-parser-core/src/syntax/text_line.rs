//! Text-line cursor helpers.
//!
//! This crate has a single responsibility: map cursor offsets to line
//! boundaries and provide conservative token-boundary primitives for
//! single-line scanning.

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]

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

/// Advance `idx` while bytes at the cursor are ASCII whitespace.
#[must_use]
pub fn skip_ascii_whitespace(bytes: &[u8], mut idx: usize) -> usize {
    while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
        idx += 1;
    }
    idx
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- line_bounds_at ---

    #[test]
    fn line_bounds_empty_input() {
        assert_eq!(line_bounds_at("", 0), (0, 0));
    }

    #[test]
    fn line_bounds_single_line_cursor_at_start() {
        assert_eq!(line_bounds_at("hello", 0), (0, 5));
    }

    #[test]
    fn line_bounds_single_line_cursor_at_mid() {
        assert_eq!(line_bounds_at("hello", 2), (0, 5));
    }

    #[test]
    fn line_bounds_single_line_cursor_at_end() {
        assert_eq!(line_bounds_at("hello", 5), (0, 5));
    }

    #[test]
    fn line_bounds_multiline_cursor_on_first_line() {
        let text = "foo\nbar\nbaz";
        // cursor at 'f' → first line is [0, 3)
        assert_eq!(line_bounds_at(text, 0), (0, 3));
    }

    #[test]
    fn line_bounds_multiline_cursor_on_second_line() {
        let text = "foo\nbar\nbaz";
        // cursor at 'b' of "bar" (index 4)
        assert_eq!(line_bounds_at(text, 4), (4, 7));
    }

    #[test]
    fn line_bounds_multiline_cursor_on_last_line() {
        let text = "foo\nbar\nbaz";
        // cursor at 'b' of "baz" (index 8)
        assert_eq!(line_bounds_at(text, 8), (8, 11));
    }

    #[test]
    fn line_bounds_cursor_on_newline_itself() {
        let text = "foo\nbar";
        // cursor on the '\n' at index 3:
        // start = rfind('\n') in "foo" → None → 0
        // end   = find('\n') in "\nbar" starting at 3 → idx 0 → cursor+0 = 3
        assert_eq!(line_bounds_at(text, 3), (0, 3));
    }

    #[test]
    fn line_bounds_cursor_past_end() {
        let text = "hello";
        // cursor_pos is clamped to text.len() (5) before use
        assert_eq!(line_bounds_at(text, 100), (0, 5));
    }

    #[test]
    fn line_bounds_crlf_cursor_on_cr() {
        let text = "foo\r\nbar";
        // cursor on '\r' at index 3
        // start = rfind('\n') in "foo\r" → None → 0
        // end   = find('\n') in "\r\nbar" → index 1 → cursor+1 = 4
        assert_eq!(line_bounds_at(text, 3), (0, 4));
    }

    #[test]
    fn line_bounds_crlf_cursor_after_lf() {
        let text = "foo\r\nbar";
        // cursor on 'b' at index 5
        // start = rfind('\n') in "foo\r\n" → index 4 → start = 5
        // end   = find('\n') in "bar" → None → text.len() = 8
        assert_eq!(line_bounds_at(text, 5), (5, 8));
    }

    // --- is_identifier_byte ---

    #[test]
    fn identifier_byte_lowercase_letters() {
        for b in b'a'..=b'z' {
            assert!(is_identifier_byte(b), "expected true for '{}'", b as char);
        }
    }

    #[test]
    fn identifier_byte_uppercase_letters() {
        for b in b'A'..=b'Z' {
            assert!(is_identifier_byte(b), "expected true for '{}'", b as char);
        }
    }

    #[test]
    fn identifier_byte_digits() {
        for b in b'0'..=b'9' {
            assert!(is_identifier_byte(b), "expected true for '{}'", b as char);
        }
    }

    #[test]
    fn identifier_byte_underscore() {
        assert!(is_identifier_byte(b'_'));
    }

    #[test]
    fn identifier_byte_space_is_false() {
        assert!(!is_identifier_byte(b' '));
    }

    #[test]
    fn identifier_byte_punctuation_is_false() {
        for b in [b'!', b'@', b'#', b'$', b'%', b'^', b'&', b'*', b'(', b')', b'-', b'+'] {
            assert!(!is_identifier_byte(b), "expected false for '{}'", b as char);
        }
    }

    #[test]
    fn identifier_byte_control_char_is_false() {
        assert!(!is_identifier_byte(b'\t'));
        assert!(!is_identifier_byte(b'\n'));
        assert!(!is_identifier_byte(0x00));
    }

    #[test]
    fn identifier_byte_high_bit_is_false() {
        // High-bit bytes are not ASCII alphanumeric and not '_'
        assert!(!is_identifier_byte(0x80));
        assert!(!is_identifier_byte(0xFF));
    }

    // --- is_keyword_boundary ---

    #[test]
    fn keyword_boundary_at_index_zero_start() {
        let bytes = b"if foo";
        // "if" at start (index 0, len 2): no preceding byte → left bound ok
        // bytes[2] == b' ' → not identifier → right bound ok
        assert!(is_keyword_boundary(bytes, 0, 2));
    }

    #[test]
    fn keyword_boundary_false_when_start_past_end() {
        let bytes = b"hi";
        assert!(!is_keyword_boundary(bytes, 5, 2));
    }

    #[test]
    fn keyword_boundary_false_when_token_runs_past_end() {
        let bytes = b"hi";
        assert!(!is_keyword_boundary(bytes, 0, 10));
    }

    #[test]
    fn keyword_boundary_false_when_preceded_by_identifier_byte() {
        // "if" with a letter immediately before it: "xif "
        let bytes = b"xif bar";
        // start=1, len=2 → bytes[0] = b'x' → identifier → false
        assert!(!is_keyword_boundary(bytes, 1, 2));
    }

    #[test]
    fn keyword_boundary_false_when_followed_by_identifier_byte() {
        // "if" followed immediately by a letter: "iffoo"
        let bytes = b"iffoo";
        // start=0, len=2 → bytes[2] = b'f' → identifier → false
        assert!(!is_keyword_boundary(bytes, 0, 2));
    }

    #[test]
    fn keyword_boundary_true_at_end_of_input() {
        // "if" at the very end of the buffer with preceding space
        let bytes = b" if";
        // start=1, len=2, end=3 == bytes.len() → right bound ok
        // bytes[0] = b' ' → not identifier → left bound ok
        assert!(is_keyword_boundary(bytes, 1, 2));
    }

    #[test]
    fn keyword_boundary_true_surrounded_by_whitespace() {
        let bytes = b" if ";
        assert!(is_keyword_boundary(bytes, 1, 2));
    }

    #[test]
    fn keyword_boundary_true_surrounded_by_punctuation() {
        let bytes = b";if;";
        assert!(is_keyword_boundary(bytes, 1, 2));
    }

    // --- skip_ascii_whitespace ---

    #[test]
    fn skip_whitespace_empty_input() {
        assert_eq!(skip_ascii_whitespace(b"", 0), 0);
    }

    #[test]
    fn skip_whitespace_no_whitespace_at_index() {
        assert_eq!(skip_ascii_whitespace(b"hello", 0), 0);
    }

    #[test]
    fn skip_whitespace_space() {
        assert_eq!(skip_ascii_whitespace(b"   x", 0), 3);
    }

    #[test]
    fn skip_whitespace_tab() {
        assert_eq!(skip_ascii_whitespace(b"\t\tx", 0), 2);
    }

    #[test]
    fn skip_whitespace_newline() {
        assert_eq!(skip_ascii_whitespace(b"\nx", 0), 1);
    }

    #[test]
    fn skip_whitespace_carriage_return() {
        assert_eq!(skip_ascii_whitespace(b"\rx", 0), 1);
    }

    #[test]
    fn skip_whitespace_mixed_whitespace() {
        assert_eq!(skip_ascii_whitespace(b" \t\n\r!", 0), 4);
    }

    #[test]
    fn skip_whitespace_all_whitespace_advances_to_end() {
        assert_eq!(skip_ascii_whitespace(b"   ", 0), 3);
    }

    #[test]
    fn skip_whitespace_index_already_past_whitespace() {
        // idx starts after the spaces
        assert_eq!(skip_ascii_whitespace(b"   hello", 3), 3);
    }

    #[test]
    fn skip_whitespace_index_mid_whitespace() {
        assert_eq!(skip_ascii_whitespace(b"x  y", 1), 3);
    }
}
