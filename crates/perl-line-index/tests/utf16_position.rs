//! Tests for UTF-16-aware position_to_byte_utf16 conversion.
//!
//! LSP `Position.character` values are UTF-16 code unit offsets.
//! On lines containing multibyte UTF-8 characters, a raw byte add-offset
//! produces the wrong result and can land mid-codepoint.
//!
//! These tests pin the correct before/after behaviour for `position_to_byte_utf16`.

use perl_line_index::LineIndex;

// ---------------------------------------------------------------------------
// Core correctness: multibyte character on the line
// ---------------------------------------------------------------------------

/// `my $café = 1;`
///
/// UTF-8 bytes: m=0 y=1 ' '=2 $=3 c=4 a=5 f=6 é=(7,8) ' '=9 ==10 ' '=11 1=12 ;=13
/// UTF-16 cols: m=0 y=1 ' '=2 $=3 c=4 a=5 f=6  é=7    ' '=8 ==9  ' '=10 1=11 ;=12
///
/// `=` is at UTF-16 col 9 → byte offset 10.
/// Before the fix `position_to_byte(0, 9)` would return `Some(9)` (byte 9 = the
/// space before `=`), which is wrong.  `position_to_byte_utf16` returns `Some(10)`.
#[test]
fn test_cafe_equals_sign_utf16_col_9_maps_to_byte_10() -> Result<(), Box<dyn std::error::Error>> {
    let text = "my $caf\u{00e9} = 1;";
    let idx = LineIndex::new(text);

    // UTF-16 col 9 is the `=` character.
    // The é (U+00E9) is 2 UTF-8 bytes but 1 UTF-16 code unit, so the byte offset
    // of UTF-16 col 9 is 10.
    assert_eq!(
        idx.position_to_byte_utf16(text, 0, 9),
        Some(10),
        "UTF-16 col 9 on 'my $café = 1;' should be byte 10"
    );
    Ok(())
}

/// Demonstrate that the old position_to_byte returns the wrong answer for the
/// same query — this is the regression the new method fixes.
#[test]
fn test_old_position_to_byte_returns_wrong_byte_for_multibyte_line()
-> Result<(), Box<dyn std::error::Error>> {
    let text = "my $caf\u{00e9} = 1;";
    let idx = LineIndex::new(text);

    // position_to_byte treats column as a raw byte offset.
    // UTF-16 col 9 is passed but position_to_byte adds 9 bytes from line start,
    // landing at byte 9 (the space before `=`) instead of byte 10 (the `=` itself).
    assert_eq!(
        idx.position_to_byte(0, 9),
        Some(9),
        "position_to_byte(0, 9) on multibyte line returns byte 9 (wrong for LSP use)"
    );
    // The correct byte offset via the UTF-16-aware method is 10.
    assert_eq!(idx.position_to_byte_utf16(text, 0, 9), Some(10));
    Ok(())
}

/// Regression guard: pure ASCII lines must continue to work — byte offsets and
/// UTF-16 offsets are identical for ASCII, so the old numeric result must be preserved.
#[test]
fn test_ascii_line_utf16_col_equals_byte_offset() -> Result<(), Box<dyn std::error::Error>> {
    let text = "hello world";
    let idx = LineIndex::new(text);

    assert_eq!(idx.position_to_byte_utf16(text, 0, 0), Some(0));
    assert_eq!(idx.position_to_byte_utf16(text, 0, 5), Some(5));
    assert_eq!(idx.position_to_byte_utf16(text, 0, 10), Some(10));
    Ok(())
}

/// Out-of-range UTF-16 column beyond the end of the line → None.
#[test]
fn test_utf16_col_beyond_line_end_returns_none() -> Result<(), Box<dyn std::error::Error>> {
    let text = "hi";
    let idx = LineIndex::new(text);

    // "hi" has UTF-16 length 2; col 3 is out of range.
    assert_eq!(idx.position_to_byte_utf16(text, 0, 3), None);
    Ok(())
}

/// Out-of-range line → None (same as position_to_byte).
#[test]
fn test_utf16_out_of_range_line_returns_none() -> Result<(), Box<dyn std::error::Error>> {
    let text = "hello";
    let idx = LineIndex::new(text);

    assert_eq!(idx.position_to_byte_utf16(text, 5, 0), None);
    Ok(())
}

/// Multi-line document: multibyte character on line 1, query for col on line 1.
/// Line 0: "abc\n"  (bytes 0-3)
/// Line 1: "caf\u{e9}x"  (bytes 4-9: c=4 a=5 f=6 é=(7,8) x=9)
/// UTF-16 col of 'x' = 4 → byte 9.
#[test]
fn test_multiline_utf16_col_on_multibyte_line() -> Result<(), Box<dyn std::error::Error>> {
    let text = "abc\ncaf\u{00e9}x";
    let idx = LineIndex::new(text);

    // On line 1, 'x' is UTF-16 col 4 but byte offset 9.
    assert_eq!(idx.position_to_byte_utf16(text, 1, 4), Some(9));
    Ok(())
}

/// Surrogate-pair emoji (U+1F600 = 😀): 4 UTF-8 bytes, 2 UTF-16 code units.
/// "a😀b": a=UTF-16_col_0 😀=UTF-16_col_1 (2 units) b=UTF-16_col_3
/// Byte offsets: a=0 😀=(1..5) b=5
#[test]
fn test_emoji_surrogate_pair_utf16_two_units() -> Result<(), Box<dyn std::error::Error>> {
    let text = "a\u{1F600}b";
    let idx = LineIndex::new(text);

    // 'b' is at UTF-16 col 3 → byte 5.
    assert_eq!(idx.position_to_byte_utf16(text, 0, 3), Some(5));
    // 'a' is at UTF-16 col 0 → byte 0.
    assert_eq!(idx.position_to_byte_utf16(text, 0, 0), Some(0));
    Ok(())
}

/// UTF-16 col that falls in the middle of a surrogate pair should return None
/// (cannot address the interior of a surrogate pair).
#[test]
fn test_utf16_col_inside_surrogate_pair_returns_none() -> Result<(), Box<dyn std::error::Error>> {
    // U+1F600 is a supplementary character: 2 UTF-16 code units.
    let text = "a\u{1F600}b";
    let idx = LineIndex::new(text);

    // UTF-16 col 2 is the second unit of the emoji — mid-character.
    assert_eq!(idx.position_to_byte_utf16(text, 0, 2), None);
    Ok(())
}

/// Multiline: line 0 with leading ASCII, line 1 with accented character.
/// Confirm position_to_byte_utf16(0, ...) is not confused by line 1's content.
#[test]
fn test_utf16_first_line_unaffected_by_second_line() -> Result<(), Box<dyn std::error::Error>> {
    let text = "hello\ncaf\u{00e9}";
    let idx = LineIndex::new(text);

    // Line 0 is pure ASCII: UTF-16 col == byte offset.
    assert_eq!(idx.position_to_byte_utf16(text, 0, 0), Some(0));
    assert_eq!(idx.position_to_byte_utf16(text, 0, 4), Some(4));

    // Line 1 starts at byte 6.
    // 'c'=UTF16_col0=byte6, 'a'=col1=byte7, 'f'=col2=byte8, 'é'=col3=byte9, end=col4=byte11
    assert_eq!(idx.position_to_byte_utf16(text, 1, 0), Some(6));
    assert_eq!(idx.position_to_byte_utf16(text, 1, 3), Some(9));
    assert_eq!(idx.position_to_byte_utf16(text, 1, 4), Some(11));
    Ok(())
}
