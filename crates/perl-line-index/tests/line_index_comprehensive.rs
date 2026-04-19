//! Comprehensive test coverage for `perl-line-index` byte/line/column conversions.
//!
//! Covers: empty documents, single-line, multi-line, CRLF, Unicode,
//! very long lines, boundary conditions, and roundtrip invariants.

use perl_line_index::LineIndex;

// ---------------------------------------------------------------------------
// Empty document
// ---------------------------------------------------------------------------

#[test]
fn test_empty_document_byte_to_position_at_zero() -> Result<(), Box<dyn std::error::Error>> {
    let index = LineIndex::new("");
    let (line, col) = index.byte_to_position(0);
    assert_eq!(line, 0, "line should be 0 for empty doc offset 0");
    assert_eq!(col, 0, "col should be 0 for empty doc offset 0");
    Ok(())
}

#[test]
fn test_empty_document_position_to_byte_origin() -> Result<(), Box<dyn std::error::Error>> {
    let index = LineIndex::new("");
    let byte = index.position_to_byte(0, 0);
    assert_eq!(byte, Some(0));
    Ok(())
}

#[test]
fn test_empty_document_position_to_byte_out_of_range_line() -> Result<(), Box<dyn std::error::Error>>
{
    let index = LineIndex::new("");
    let byte = index.position_to_byte(1, 0);
    assert_eq!(byte, None, "line 1 should be out of range for empty doc");
    Ok(())
}

// ---------------------------------------------------------------------------
// Single-line document (no newlines)
// ---------------------------------------------------------------------------

#[test]
fn test_single_line_first_byte() -> Result<(), Box<dyn std::error::Error>> {
    let index = LineIndex::new("hello");
    let (line, col) = index.byte_to_position(0);
    assert_eq!((line, col), (0, 0));
    Ok(())
}

#[test]
fn test_single_line_middle_byte() -> Result<(), Box<dyn std::error::Error>> {
    let index = LineIndex::new("hello");
    let (line, col) = index.byte_to_position(3);
    assert_eq!((line, col), (0, 3), "byte 3 of 'hello' is column 3");
    Ok(())
}

#[test]
fn test_single_line_last_byte() -> Result<(), Box<dyn std::error::Error>> {
    let index = LineIndex::new("hello");
    let (line, col) = index.byte_to_position(4);
    assert_eq!(
        (line, col),
        (0, 4),
        "byte 4 of 'hello' is column 4 (last char 'o')"
    );
    Ok(())
}

#[test]
fn test_single_line_roundtrip_every_offset() -> Result<(), Box<dyn std::error::Error>> {
    let text = "hello";
    let index = LineIndex::new(text);
    for offset in 0..text.len() {
        let (line, col) = index.byte_to_position(offset);
        let back = index.position_to_byte(line, col);
        assert_eq!(back, Some(offset), "roundtrip failed for offset {offset}");
    }
    Ok(())
}

#[test]
fn test_single_line_position_to_byte_returns_some() -> Result<(), Box<dyn std::error::Error>> {
    let index = LineIndex::new("abcde");
    assert_eq!(index.position_to_byte(0, 0), Some(0));
    assert_eq!(index.position_to_byte(0, 2), Some(2));
    assert_eq!(index.position_to_byte(0, 5), Some(5));
    Ok(())
}

// ---------------------------------------------------------------------------
// Multi-line document with LF
// ---------------------------------------------------------------------------

#[test]
fn test_multiline_lf_first_line_start() -> Result<(), Box<dyn std::error::Error>> {
    let index = LineIndex::new("abc\ndef\nghi");
    let (line, col) = index.byte_to_position(0);
    assert_eq!((line, col), (0, 0));
    Ok(())
}

#[test]
fn test_multiline_lf_newline_byte_is_end_of_line() -> Result<(), Box<dyn std::error::Error>> {
    // "abc\n" -> the \n is at byte 3
    let index = LineIndex::new("abc\ndef\nghi");
    let (line, col) = index.byte_to_position(3);
    // byte 3 is '\n' which is still on line 0 (column 3)
    assert_eq!((line, col), (0, 3));
    Ok(())
}

#[test]
fn test_multiline_lf_second_line_start() -> Result<(), Box<dyn std::error::Error>> {
    // "abc\n" is 4 bytes, so "def" starts at byte 4
    let index = LineIndex::new("abc\ndef\nghi");
    let (line, col) = index.byte_to_position(4);
    assert_eq!((line, col), (1, 0));
    Ok(())
}

#[test]
fn test_multiline_lf_third_line_middle() -> Result<(), Box<dyn std::error::Error>> {
    // "abc\ndef\n" is 8 bytes, "ghi" starts at byte 8, 'h' at byte 9
    let index = LineIndex::new("abc\ndef\nghi");
    let (line, col) = index.byte_to_position(9);
    assert_eq!((line, col), (2, 1));
    Ok(())
}

#[test]
fn test_multiline_lf_roundtrip_all_offsets() -> Result<(), Box<dyn std::error::Error>> {
    let text = "abc\ndef\nghi";
    let index = LineIndex::new(text);
    for offset in 0..text.len() {
        let (line, col) = index.byte_to_position(offset);
        let back = index.position_to_byte(line, col);
        assert_eq!(back, Some(offset), "roundtrip failed at offset {offset}");
    }
    Ok(())
}

#[test]
fn test_multiline_lf_position_to_byte_each_line_start() -> Result<(), Box<dyn std::error::Error>> {
    let index = LineIndex::new("abc\ndef\nghi");
    assert_eq!(index.position_to_byte(0, 0), Some(0));
    assert_eq!(index.position_to_byte(1, 0), Some(4));
    assert_eq!(index.position_to_byte(2, 0), Some(8));
    Ok(())
}

// ---------------------------------------------------------------------------
// Multi-line with CRLF
// ---------------------------------------------------------------------------

#[test]
fn test_crlf_line_starts() -> Result<(), Box<dyn std::error::Error>> {
    // "ab\r\ncd\r\nef"
    // Line 0: bytes 0..3 ("ab\r"), \n at byte 3 -> line 1 starts at byte 4
    // Line 1: bytes 4..7 ("cd\r"), \n at byte 7 -> line 2 starts at byte 8
    let index = LineIndex::new("ab\r\ncd\r\nef");
    let (line, col) = index.byte_to_position(4);
    assert_eq!(line, 1, "byte 4 should be line 1 (start of 'cd')");
    assert_eq!(col, 0);
    Ok(())
}

#[test]
fn test_crlf_cr_byte_is_still_previous_line() -> Result<(), Box<dyn std::error::Error>> {
    // In this implementation, only \n triggers a new line.
    // So \r at byte 2 is still line 0.
    let index = LineIndex::new("ab\r\ncd");
    let (line, col) = index.byte_to_position(2);
    assert_eq!(line, 0, "\\r at byte 2 is still line 0");
    assert_eq!(col, 2, "\\r at byte 2 is column 2");
    Ok(())
}

#[test]
fn test_crlf_second_line_char() -> Result<(), Box<dyn std::error::Error>> {
    // "ab\r\ncd" -> 'c' at byte 4, 'd' at byte 5
    let index = LineIndex::new("ab\r\ncd");
    let (line, col) = index.byte_to_position(5);
    assert_eq!((line, col), (1, 1), "'d' is at line 1, col 1");
    Ok(())
}

#[test]
fn test_crlf_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let text = "line1\r\nline2\r\nline3";
    let index = LineIndex::new(text);
    for offset in 0..text.len() {
        let (line, col) = index.byte_to_position(offset);
        let back = index.position_to_byte(line, col);
        assert_eq!(
            back,
            Some(offset),
            "roundtrip failed at offset {offset} in CRLF text"
        );
    }
    Ok(())
}

#[test]
fn test_crlf_three_lines_positions() -> Result<(), Box<dyn std::error::Error>> {
    // "x\r\ny\r\nz"
    // This implementation splits only on \n (not \r\n), so:
    //   bytes: x(0) \r(1) \n(2) y(3) \r(4) \n(5) z(6)
    //   line 0 starts at 0, line 1 at 3, line 2 at 6
    let index = LineIndex::new("x\r\ny\r\nz");
    assert_eq!(index.byte_to_position(0), (0, 0)); // 'x'
    assert_eq!(index.byte_to_position(3), (1, 0)); // 'y'
    assert_eq!(index.byte_to_position(6), (2, 0)); // 'z'
    Ok(())
}

// ---------------------------------------------------------------------------
// Unicode character offsets
// ---------------------------------------------------------------------------

#[test]
fn test_unicode_two_byte_char_offset() -> Result<(), Box<dyn std::error::Error>> {
    // 'e' with acute: U+00E9, encoded as 2 bytes in UTF-8 (0xC3, 0xA9)
    // "cafe\u{0301}" has 'c'(1) + 'a'(1) + 'f'(1) + 'e'(1) + combining accent(2) = 6 bytes
    // Actually let's use a simpler example:
    // "caf\u{00e9}" = "cafe" where e-acute is 2 bytes
    let text = "caf\u{00e9}";
    assert_eq!(text.len(), 5, "caf + 2-byte e-acute = 5 bytes");
    let index = LineIndex::new(text);

    // 'c' at byte 0
    assert_eq!(index.byte_to_position(0), (0, 0));
    // 'a' at byte 1
    assert_eq!(index.byte_to_position(1), (0, 1));
    // 'f' at byte 2
    assert_eq!(index.byte_to_position(2), (0, 2));
    // e-acute starts at byte 3
    assert_eq!(index.byte_to_position(3), (0, 3));
    // byte 4 is second byte of e-acute (mid-character byte offset)
    assert_eq!(index.byte_to_position(4), (0, 4));
    Ok(())
}

#[test]
fn test_unicode_three_byte_char() -> Result<(), Box<dyn std::error::Error>> {
    // CJK character U+4E16 (world) is 3 bytes in UTF-8
    let text = "a\u{4E16}b";
    assert_eq!(text.len(), 5, "'a' + 3-byte CJK + 'b' = 5");
    let index = LineIndex::new(text);

    assert_eq!(index.byte_to_position(0), (0, 0)); // 'a'
    assert_eq!(index.byte_to_position(1), (0, 1)); // start of CJK char
    assert_eq!(index.byte_to_position(4), (0, 4)); // 'b'
    Ok(())
}

#[test]
fn test_unicode_four_byte_char() -> Result<(), Box<dyn std::error::Error>> {
    // Emoji U+1F600 is 4 bytes in UTF-8
    let text = "a\u{1F600}b";
    assert_eq!(text.len(), 6, "'a' + 4-byte emoji + 'b' = 6");
    let index = LineIndex::new(text);

    assert_eq!(index.byte_to_position(0), (0, 0)); // 'a'
    assert_eq!(index.byte_to_position(1), (0, 1)); // start of emoji
    assert_eq!(index.byte_to_position(5), (0, 5)); // 'b'
    Ok(())
}

#[test]
fn test_unicode_multiline() -> Result<(), Box<dyn std::error::Error>> {
    // "h\u{00e9}llo\nw\u{00f6}rld"
    // Line 0: h(1) + e-acute(2) + l(1) + l(1) + o(1) + \n(1) = 7 bytes -> line 1 starts at 7
    // Line 1: w(1) + o-umlaut(2) + r(1) + l(1) + d(1) = 6 bytes
    let text = "h\u{00e9}llo\nw\u{00f6}rld";
    assert_eq!(text.len(), 13);
    let index = LineIndex::new(text);

    // First line: 'h' at 0, e-acute at 1-2, 'l' at 3, 'l' at 4, 'o' at 5, '\n' at 6
    assert_eq!(index.byte_to_position(0), (0, 0)); // 'h'
    assert_eq!(index.byte_to_position(1), (0, 1)); // e-acute start
    assert_eq!(index.byte_to_position(3), (0, 3)); // first 'l'

    // Second line starts at byte 7
    assert_eq!(index.byte_to_position(7), (1, 0)); // 'w'
    assert_eq!(index.byte_to_position(8), (1, 1)); // o-umlaut start

    // Roundtrip line starts
    assert_eq!(index.position_to_byte(0, 0), Some(0));
    assert_eq!(index.position_to_byte(1, 0), Some(7));
    Ok(())
}

// ---------------------------------------------------------------------------
// Very long lines
// ---------------------------------------------------------------------------

#[test]
fn test_very_long_single_line() -> Result<(), Box<dyn std::error::Error>> {
    let long_line: String = "x".repeat(10_000);
    let index = LineIndex::new(&long_line);

    // First byte
    assert_eq!(index.byte_to_position(0), (0, 0));
    // Middle
    assert_eq!(index.byte_to_position(5_000), (0, 5_000));
    // Last byte
    assert_eq!(index.byte_to_position(9_999), (0, 9_999));

    // Roundtrip
    let back = index.position_to_byte(0, 5_000);
    assert_eq!(back, Some(5_000));
    Ok(())
}

#[test]
fn test_very_long_line_with_trailing_newline() -> Result<(), Box<dyn std::error::Error>> {
    let mut text = "a".repeat(100_000);
    text.push('\n');
    text.push_str("short");
    let index = LineIndex::new(&text);

    // Last char of long line
    assert_eq!(index.byte_to_position(99_999), (0, 99_999));
    // Newline itself
    assert_eq!(index.byte_to_position(100_000), (0, 100_000));
    // Start of second line
    assert_eq!(index.byte_to_position(100_001), (1, 0));
    // 's' of "short"
    assert_eq!(index.byte_to_position(100_003), (1, 2));
    Ok(())
}

// ---------------------------------------------------------------------------
// Trailing newline / empty last line
// ---------------------------------------------------------------------------

#[test]
fn test_trailing_newline_creates_extra_line() -> Result<(), Box<dyn std::error::Error>> {
    // "abc\n" -> line 0 = "abc\n", line 1 starts at byte 4 (empty)
    let index = LineIndex::new("abc\n");
    assert_eq!(index.byte_to_position(4), (1, 0));
    assert_eq!(index.position_to_byte(1, 0), Some(4));
    Ok(())
}

#[test]
fn test_multiple_trailing_newlines() -> Result<(), Box<dyn std::error::Error>> {
    // "a\n\n\n" -> lines: 0 starts at 0, 1 at 2, 2 at 3, 3 at 4
    let index = LineIndex::new("a\n\n\n");
    assert_eq!(index.byte_to_position(2), (1, 0)); // first empty line
    assert_eq!(index.byte_to_position(3), (2, 0)); // second empty line
    assert_eq!(index.byte_to_position(4), (3, 0)); // after last \n
    Ok(())
}

#[test]
fn test_only_newlines() -> Result<(), Box<dyn std::error::Error>> {
    let index = LineIndex::new("\n\n\n");
    assert_eq!(index.byte_to_position(0), (0, 0)); // first \n
    assert_eq!(index.byte_to_position(1), (1, 0)); // second \n
    assert_eq!(index.byte_to_position(2), (2, 0)); // third \n
    assert_eq!(index.byte_to_position(3), (3, 0)); // past end
    Ok(())
}

// ---------------------------------------------------------------------------
// Position to byte: out-of-bounds
// ---------------------------------------------------------------------------

#[test]
fn test_position_to_byte_line_out_of_range() -> Result<(), Box<dyn std::error::Error>> {
    let index = LineIndex::new("abc\ndef");
    assert_eq!(index.position_to_byte(5, 0), None);
    assert_eq!(index.position_to_byte(100, 0), None);
    Ok(())
}

#[test]
fn test_position_to_byte_large_column_returns_some() -> Result<(), Box<dyn std::error::Error>> {
    // The implementation returns Some(start + column) without bounds checking
    // the column against line length, so large columns just produce large offsets.
    let index = LineIndex::new("abc\ndef");
    let result = index.position_to_byte(0, 100);
    // start of line 0 is 0, so 0 + 100 = 100
    assert_eq!(result, Some(100));
    Ok(())
}

// ---------------------------------------------------------------------------
// Byte to position: past end of text
// ---------------------------------------------------------------------------

#[test]
fn test_byte_past_end_maps_to_last_line() -> Result<(), Box<dyn std::error::Error>> {
    let text = "abc\ndef";
    let index = LineIndex::new(text);
    // byte 7 = text.len(), which is one past the last char
    let (line, col) = index.byte_to_position(7);
    assert_eq!(line, 1, "past-end byte should map to last line");
    assert_eq!(
        col, 3,
        "past-end byte should have col = bytes past line start"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Roundtrip invariants across diverse documents
// ---------------------------------------------------------------------------

#[test]
fn test_roundtrip_mixed_content() -> Result<(), Box<dyn std::error::Error>> {
    let text = "use strict;\nmy $x = 42;\nprint $x;\n";
    let index = LineIndex::new(text);
    for offset in 0..text.len() {
        let (line, col) = index.byte_to_position(offset);
        let back = index.position_to_byte(line, col);
        assert_eq!(
            back,
            Some(offset),
            "roundtrip failed at offset {offset} (line={line}, col={col})"
        );
    }
    Ok(())
}

#[test]
fn test_roundtrip_perl_subroutine() -> Result<(), Box<dyn std::error::Error>> {
    let text = "sub greet {\n    my ($name) = @_;\n    print \"Hello, $name!\\n\";\n}\n";
    let index = LineIndex::new(text);
    for offset in 0..text.len() {
        let (line, col) = index.byte_to_position(offset);
        let back = index.position_to_byte(line, col);
        assert_eq!(
            back,
            Some(offset),
            "roundtrip failed at offset {offset} (line={line}, col={col})"
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Single character document
// ---------------------------------------------------------------------------

#[test]
fn test_single_char_document() -> Result<(), Box<dyn std::error::Error>> {
    let index = LineIndex::new("x");
    assert_eq!(index.byte_to_position(0), (0, 0));
    assert_eq!(index.position_to_byte(0, 0), Some(0));
    Ok(())
}

#[test]
fn test_single_newline_document() -> Result<(), Box<dyn std::error::Error>> {
    let index = LineIndex::new("\n");
    assert_eq!(index.byte_to_position(0), (0, 0)); // the \n char
    assert_eq!(index.byte_to_position(1), (1, 0)); // after the \n
    assert_eq!(index.position_to_byte(0, 0), Some(0));
    assert_eq!(index.position_to_byte(1, 0), Some(1));
    Ok(())
}

// ---------------------------------------------------------------------------
// Lines of varying lengths
// ---------------------------------------------------------------------------

#[test]
fn test_varying_line_lengths() -> Result<(), Box<dyn std::error::Error>> {
    // Lines: "a"(1), "bb"(2), "ccc"(3), "d"(1)
    let text = "a\nbb\nccc\nd";
    let index = LineIndex::new(text);

    // Line 0: "a" at byte 0
    assert_eq!(index.byte_to_position(0), (0, 0));
    // Line 1: "bb" starts at byte 2
    assert_eq!(index.byte_to_position(2), (1, 0));
    assert_eq!(index.byte_to_position(3), (1, 1));
    // Line 2: "ccc" starts at byte 5
    assert_eq!(index.byte_to_position(5), (2, 0));
    assert_eq!(index.byte_to_position(7), (2, 2));
    // Line 3: "d" starts at byte 9
    assert_eq!(index.byte_to_position(9), (3, 0));

    // Verify position_to_byte for each line start
    assert_eq!(index.position_to_byte(0, 0), Some(0));
    assert_eq!(index.position_to_byte(1, 0), Some(2));
    assert_eq!(index.position_to_byte(2, 0), Some(5));
    assert_eq!(index.position_to_byte(3, 0), Some(9));
    Ok(())
}

// ---------------------------------------------------------------------------
// Empty lines in the middle
// ---------------------------------------------------------------------------

#[test]
fn test_empty_lines_in_middle() -> Result<(), Box<dyn std::error::Error>> {
    // "a\n\nb" -> line 0: "a\n" (bytes 0-1), line 1: "\n" (byte 2), line 2: "b" (byte 3)
    let index = LineIndex::new("a\n\nb");
    assert_eq!(index.byte_to_position(0), (0, 0)); // 'a'
    assert_eq!(index.byte_to_position(2), (1, 0)); // empty line's \n
    assert_eq!(index.byte_to_position(3), (2, 0)); // 'b'
    Ok(())
}

// ---------------------------------------------------------------------------
// Tab characters (treated as single byte)
// ---------------------------------------------------------------------------

#[test]
fn test_tab_characters() -> Result<(), Box<dyn std::error::Error>> {
    // Tabs are single bytes, so byte offset = column
    let index = LineIndex::new("\thello");
    assert_eq!(index.byte_to_position(0), (0, 0)); // tab at col 0
    assert_eq!(index.byte_to_position(1), (0, 1)); // 'h' at col 1
    Ok(())
}

// ---------------------------------------------------------------------------
// Position to byte with column offset
// ---------------------------------------------------------------------------

#[test]
fn test_position_to_byte_with_column() -> Result<(), Box<dyn std::error::Error>> {
    let index = LineIndex::new("abc\ndef\nghi");
    // Line 1, col 2 -> byte offset = 4 (line start) + 2 = 6
    assert_eq!(index.position_to_byte(1, 2), Some(6));
    // Line 2, col 1 -> byte offset = 8 (line start) + 1 = 9
    assert_eq!(index.position_to_byte(2, 1), Some(9));
    Ok(())
}

// ---------------------------------------------------------------------------
// Stress test: many short lines
// ---------------------------------------------------------------------------

#[test]
fn test_many_short_lines() -> Result<(), Box<dyn std::error::Error>> {
    // 1000 lines of "x\n"
    let text: String = (0..1000).map(|_| "x\n").collect();
    let index = LineIndex::new(&text);

    // Each "x\n" is 2 bytes, so line N starts at byte 2*N
    for line_num in 0..1000 {
        let expected_offset = line_num * 2;
        assert_eq!(
            index.byte_to_position(expected_offset),
            (line_num, 0),
            "line {line_num} should start at byte {expected_offset}"
        );
        assert_eq!(
            index.position_to_byte(line_num, 0),
            Some(expected_offset),
            "position_to_byte for line {line_num} should return {expected_offset}"
        );
    }
    Ok(())
}
