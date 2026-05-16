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
    fn line_bounds_empty_input() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(line_bounds_at("", 0), (0, 0));
        Ok(())
    }

    #[test]
    fn line_bounds_single_line_cursor_at_start() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(line_bounds_at("hello", 0), (0, 5));
        Ok(())
    }

    #[test]
    fn line_bounds_single_line_cursor_at_mid() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(line_bounds_at("hello", 2), (0, 5));
        Ok(())
    }

    #[test]
    fn line_bounds_single_line_cursor_at_end() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(line_bounds_at("hello", 5), (0, 5));
        Ok(())
    }

    #[test]
    fn line_bounds_multiline_cursor_on_first_line() -> Result<(), Box<dyn std::error::Error>> {
        let text = "foo\nbar\nbaz";
        // cursor at 'f' → first line is [0, 3)
        assert_eq!(line_bounds_at(text, 0), (0, 3));
        Ok(())
    }

    #[test]
    fn line_bounds_multiline_cursor_on_second_line() -> Result<(), Box<dyn std::error::Error>> {
        let text = "foo\nbar\nbaz";
        // cursor at 'b' of "bar" (index 4)
        assert_eq!(line_bounds_at(text, 4), (4, 7));
        Ok(())
    }

    #[test]
    fn line_bounds_multiline_cursor_on_last_line() -> Result<(), Box<dyn std::error::Error>> {
        let text = "foo\nbar\nbaz";
        // cursor at 'b' of "baz" (index 8)
        assert_eq!(line_bounds_at(text, 8), (8, 11));
        Ok(())
    }

    #[test]
    fn line_bounds_cursor_on_newline_itself() -> Result<(), Box<dyn std::error::Error>> {
        let text = "foo\nbar";
        // cursor on the '\n' at index 3:
        // start = rfind('\n') in "foo" → None → 0
        // end   = find('\n') in "\nbar" starting at 3 → idx 0 → cursor+0 = 3
        assert_eq!(line_bounds_at(text, 3), (0, 3));
        Ok(())
    }

    #[test]
    fn line_bounds_cursor_past_end() -> Result<(), Box<dyn std::error::Error>> {
        let text = "hello";
        // cursor_pos is clamped to text.len() (5) before use
        assert_eq!(line_bounds_at(text, 100), (0, 5));
        Ok(())
    }

    #[test]
    fn line_bounds_crlf_cursor_on_cr() -> Result<(), Box<dyn std::error::Error>> {
        let text = "foo\r\nbar";
        // cursor on '\r' at index 3
        // start = rfind('\n') in "foo\r" → None → 0
        // end   = find('\n') in "\r\nbar" → index 1 → cursor+1 = 4
        assert_eq!(line_bounds_at(text, 3), (0, 4));
        Ok(())
    }

    #[test]
    fn line_bounds_crlf_cursor_after_lf() -> Result<(), Box<dyn std::error::Error>> {
        let text = "foo\r\nbar";
        // cursor on 'b' at index 5
        // start = rfind('\n') in "foo\r\n" → index 4 → start = 5
        // end   = find('\n') in "bar" → None → text.len() = 8
        assert_eq!(line_bounds_at(text, 5), (5, 8));
        Ok(())
    }

    // --- is_identifier_byte ---

    #[test]
    fn identifier_byte_lowercase_letters() -> Result<(), Box<dyn std::error::Error>> {
        for b in b'a'..=b'z' {
            assert!(is_identifier_byte(b), "expected true for '{}'", b as char);
        }
        Ok(())
    }

    #[test]
    fn identifier_byte_uppercase_letters() -> Result<(), Box<dyn std::error::Error>> {
        for b in b'A'..=b'Z' {
            assert!(is_identifier_byte(b), "expected true for '{}'", b as char);
        }
        Ok(())
    }

    #[test]
    fn identifier_byte_digits() -> Result<(), Box<dyn std::error::Error>> {
        for b in b'0'..=b'9' {
            assert!(is_identifier_byte(b), "expected true for '{}'", b as char);
        }
        Ok(())
    }

    #[test]
    fn identifier_byte_underscore() -> Result<(), Box<dyn std::error::Error>> {
        assert!(is_identifier_byte(b'_'));
        Ok(())
    }

    #[test]
    fn identifier_byte_space_is_false() -> Result<(), Box<dyn std::error::Error>> {
        assert!(!is_identifier_byte(b' '));
        Ok(())
    }

    #[test]
    fn identifier_byte_punctuation_is_false() -> Result<(), Box<dyn std::error::Error>> {
        for b in [b'!', b'@', b'#', b'$', b'%', b'^', b'&', b'*', b'(', b')', b'-', b'+'] {
            assert!(!is_identifier_byte(b), "expected false for '{}'", b as char);
        }
        Ok(())
    }

    #[test]
    fn identifier_byte_control_char_is_false() -> Result<(), Box<dyn std::error::Error>> {
        assert!(!is_identifier_byte(b'\t'));
        assert!(!is_identifier_byte(b'\n'));
        assert!(!is_identifier_byte(0x00));
        Ok(())
    }

    #[test]
    fn identifier_byte_high_bit_is_false() -> Result<(), Box<dyn std::error::Error>> {
        // High-bit bytes are not ASCII alphanumeric and not '_'
        assert!(!is_identifier_byte(0x80));
        assert!(!is_identifier_byte(0xFF));
        Ok(())
    }

    // --- is_keyword_boundary ---

    #[test]
    fn keyword_boundary_at_index_zero_start() -> Result<(), Box<dyn std::error::Error>> {
        let bytes = b"if foo";
        // "if" at start (index 0, len 2): no preceding byte → left bound ok
        // bytes[2] == b' ' → not identifier → right bound ok
        assert!(is_keyword_boundary(bytes, 0, 2));
        Ok(())
    }

    #[test]
    fn keyword_boundary_false_when_start_past_end() -> Result<(), Box<dyn std::error::Error>> {
        let bytes = b"hi";
        assert!(!is_keyword_boundary(bytes, 5, 2));
        Ok(())
    }

    #[test]
    fn keyword_boundary_false_when_token_runs_past_end() -> Result<(), Box<dyn std::error::Error>> {
        let bytes = b"hi";
        assert!(!is_keyword_boundary(bytes, 0, 10));
        Ok(())
    }

    #[test]
    fn keyword_boundary_false_when_preceded_by_identifier_byte()
    -> Result<(), Box<dyn std::error::Error>> {
        // "if" with a letter immediately before it: "xif "
        let bytes = b"xif bar";
        // start=1, len=2 → bytes[0] = b'x' → identifier → false
        assert!(!is_keyword_boundary(bytes, 1, 2));
        Ok(())
    }

    #[test]
    fn keyword_boundary_false_when_followed_by_identifier_byte()
    -> Result<(), Box<dyn std::error::Error>> {
        // "if" followed immediately by a letter: "iffoo"
        let bytes = b"iffoo";
        // start=0, len=2 → bytes[2] = b'f' → identifier → false
        assert!(!is_keyword_boundary(bytes, 0, 2));
        Ok(())
    }

    #[test]
    fn keyword_boundary_true_at_end_of_input() -> Result<(), Box<dyn std::error::Error>> {
        // "if" at the very end of the buffer with preceding space
        let bytes = b" if";
        // start=1, len=2, end=3 == bytes.len() → right bound ok
        // bytes[0] = b' ' → not identifier → left bound ok
        assert!(is_keyword_boundary(bytes, 1, 2));
        Ok(())
    }

    #[test]
    fn keyword_boundary_true_surrounded_by_whitespace() -> Result<(), Box<dyn std::error::Error>> {
        let bytes = b" if ";
        assert!(is_keyword_boundary(bytes, 1, 2));
        Ok(())
    }

    #[test]
    fn keyword_boundary_true_surrounded_by_punctuation() -> Result<(), Box<dyn std::error::Error>> {
        let bytes = b";if;";
        assert!(is_keyword_boundary(bytes, 1, 2));
        Ok(())
    }

    // --- skip_ascii_whitespace ---

    #[test]
    fn skip_whitespace_empty_input() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(skip_ascii_whitespace(b"", 0), 0);
        Ok(())
    }

    #[test]
    fn skip_whitespace_no_whitespace_at_index() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(skip_ascii_whitespace(b"hello", 0), 0);
        Ok(())
    }

    #[test]
    fn skip_whitespace_space() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(skip_ascii_whitespace(b"   x", 0), 3);
        Ok(())
    }

    #[test]
    fn skip_whitespace_tab() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(skip_ascii_whitespace(b"\t\tx", 0), 2);
        Ok(())
    }

    #[test]
    fn skip_whitespace_newline() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(skip_ascii_whitespace(b"\nx", 0), 1);
        Ok(())
    }

    #[test]
    fn skip_whitespace_carriage_return() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(skip_ascii_whitespace(b"\rx", 0), 1);
        Ok(())
    }

    #[test]
    fn skip_whitespace_mixed_whitespace() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(skip_ascii_whitespace(b" \t\n\r!", 0), 4);
        Ok(())
    }

    #[test]
    fn skip_whitespace_all_whitespace_advances_to_end() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(skip_ascii_whitespace(b"   ", 0), 3);
        Ok(())
    }

    #[test]
    fn skip_whitespace_index_already_past_whitespace() -> Result<(), Box<dyn std::error::Error>> {
        // idx starts after the spaces
        assert_eq!(skip_ascii_whitespace(b"   hello", 3), 3);
        Ok(())
    }

    #[test]
    fn skip_whitespace_index_mid_whitespace() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(skip_ascii_whitespace(b"x  y", 1), 3);
        Ok(())
    }
}
