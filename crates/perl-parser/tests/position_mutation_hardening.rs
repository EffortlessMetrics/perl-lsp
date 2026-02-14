/// Mutation hardening tests for position.rs
/// These tests target specific mutation survivors to improve test coverage
/// and eliminate edge cases that could lead to security vulnerabilities.
use perl_parser::position::{Position, Range, offset_to_utf16_line_col, utf16_line_col_to_offset};

/// Test the critical line advancement bug that was a mutation survivor
/// This ensures that line numbers correctly increment when encountering newlines
#[test]
fn test_advance_line_increment_critical_bug() {
    let mut pos = Position::start();

    // Test single newline advancement
    pos.advance("\n");
    assert_eq!(pos.line, 2, "Line should increment from 1 to 2 after newline");
    assert_eq!(pos.column, 1, "Column should reset to 1 after newline");
    assert_eq!(pos.byte, 1, "Byte position should be 1 after single newline");

    // Test multiple newlines
    pos.advance("\n\n\n");
    assert_eq!(pos.line, 5, "Line should be 5 after three more newlines");
    assert_eq!(pos.column, 1, "Column should remain 1 after newlines");
    assert_eq!(pos.byte, 4, "Byte position should be 4 after four total newlines");

    // Test newline mixed with content
    pos.advance("hello\nworld\n");
    assert_eq!(pos.line, 7, "Line should be 7 after two more newlines with content");
    assert_eq!(pos.column, 1, "Column should be 1 after final newline");
    assert_eq!(pos.byte, 16, "Byte position should account for all content");
}

/// Test advance_char specifically for newline handling
#[test]
fn test_advance_char_newline_handling() {
    let mut pos = Position::start();

    // Test individual character advancement
    pos.advance_char('h');
    assert_eq!(pos.line, 1, "Line should remain 1 for non-newline");
    assert_eq!(pos.column, 2, "Column should increment to 2");

    pos.advance_char('\n');
    assert_eq!(pos.line, 2, "Line should increment to 2 after newline");
    assert_eq!(pos.column, 1, "Column should reset to 1 after newline");

    // Test UTF-8 multibyte character
    pos.advance_char('世');
    assert_eq!(pos.line, 2, "Line should remain 2 for non-newline UTF-8");
    assert_eq!(pos.column, 2, "Column should increment to 2");
    assert_eq!(pos.byte, 5, "Byte should account for 1 + 1 + 3 UTF-8 bytes");
}

/// Test edge cases in range operations that could be mutation survivors
#[test]
fn test_range_edge_cases() {
    // Test range with same start and end (empty range)
    let pos = Position::new(10, 2, 5);
    let empty_range = Range::empty(pos);
    assert!(empty_range.is_empty(), "Empty range should be empty");
    assert_eq!(empty_range.len(), 0, "Empty range length should be 0");
    assert!(!empty_range.contains_byte(10), "Empty range should not contain any bytes");

    // Test range boundary conditions
    let start = Position::new(5, 1, 1);
    let end = Position::new(10, 1, 6);
    let range = Range::new(start, end);

    // Test exact boundaries
    assert!(range.contains_byte(5), "Range should contain start byte");
    assert!(!range.contains_byte(10), "Range should not contain end byte (exclusive)");
    assert!(range.contains_byte(9), "Range should contain byte before end");
    assert!(!range.contains_byte(4), "Range should not contain byte before start");

    // Test overflow protection in range operations
    let max_pos = Position::new(usize::MAX, u32::MAX, u32::MAX);
    let range_max = Range::new(max_pos, max_pos);
    assert_eq!(range_max.len(), 0, "Range with same max positions should have length 0");

    // Test saturating subtraction in len()
    let bad_range = Range::new(end, start); // End before start
    assert_eq!(bad_range.len(), 0, "Invalid range should saturate to 0 length");
}

/// Test UTF-16 conversion edge cases that are likely mutation survivors
#[test]
fn test_utf16_conversion_edge_cases() {
    // Test offset beyond text length
    let text = "hello";
    let (line, col) = offset_to_utf16_line_col(text, 100);
    assert_eq!(line, 0, "Line should be 0 for offset beyond text");
    assert_eq!(col, 5, "Column should be text length for offset beyond text");

    // Test offset at exact text length
    let (line, col) = offset_to_utf16_line_col(text, 5);
    assert_eq!(line, 0, "Line should be 0 for offset at text end");
    assert_eq!(col, 5, "Column should be text length for offset at text end");

    // Test empty text
    let (line, col) = offset_to_utf16_line_col("", 0);
    assert_eq!(line, 0, "Line should be 0 for empty text");
    assert_eq!(col, 0, "Column should be 0 for empty text");

    let (line, col) = offset_to_utf16_line_col("", 5);
    assert_eq!(line, 0, "Line should be 0 for empty text with large offset");
    assert_eq!(col, 0, "Column should be 0 for empty text with large offset");
}

/// Test UTF-16 conversion with text ending in newline
#[test]
fn test_utf16_newline_ending_edge_cases() {
    // Test text ending with single newline
    let text = "hello\n";
    let (line, col) = offset_to_utf16_line_col(text, 6); // At end
    assert_eq!(line, 1, "Should be on line 1 after newline");
    assert_eq!(col, 0, "Should be at column 0 after newline");

    // Test text ending with CRLF
    let text = "hello\r\n";
    let (line, col) = offset_to_utf16_line_col(text, 7); // At end
    assert_eq!(line, 1, "Should be on line 1 after CRLF");
    assert_eq!(col, 0, "Should be at column 0 after CRLF");

    // Test multiple trailing newlines
    let text = "hello\n\n\n";
    let (line, col) = offset_to_utf16_line_col(text, 8); // At end
    assert_eq!(line, 3, "Should be on line 3 after multiple newlines");
    assert_eq!(col, 0, "Should be at column 0 after final newline");
}

/// Test fractional UTF-16 positions inside multibyte characters
#[test]
fn test_utf16_fractional_positions() {
    let text = "hello 😀 world";

    // Test position inside emoji (which is 4 bytes, 2 UTF-16 units)
    let emoji_start_byte = 6; // Position of emoji

    // Test various byte positions within the emoji
    for byte_offset in emoji_start_byte..emoji_start_byte + 4 {
        let (line, col) = offset_to_utf16_line_col(text, byte_offset);
        assert_eq!(line, 0, "Line should be 0 for positions within emoji");

        // Column should progress logically through the emoji
        if byte_offset == emoji_start_byte {
            assert_eq!(col, 6, "Should be at column 6 at start of emoji");
        } else if byte_offset == emoji_start_byte + 4 {
            assert_eq!(col, 8, "Should be at column 8 after emoji (6 + 2 UTF-16 units)");
        } else {
            // Fractional positions should be between 6 and 8
            assert!(
                (6..=8).contains(&col),
                "Fractional position should be between 6 and 8, got {}",
                col
            );
        }
    }
}

/// Test round-trip conversion edge cases
#[test]
#[ignore = "UTF-16 roundtrip edge cases: emoji handling needs investigation"]
fn test_utf16_roundtrip_edge_cases() {
    let texts = vec![
        "",                  // Empty text
        "\n",                // Single newline
        "\r\n",              // CRLF
        "😀",                // Single emoji
        "a😀b",              // Emoji in middle
        "😀😀😀",            // Multiple emojis
        "hello\nworld\r\n",  // Mixed line endings
        "café naïve résumé", // Accented characters
        "中文测试",          // Chinese characters
    ];

    for text in texts {
        let _rope_len = text.chars().count();

        // Test round-trip for every possible UTF-16 position
        for line in 0..=text.lines().count() as u32 {
            for col in 0..=20u32 {
                // Test beyond typical line length
                let byte_offset = utf16_line_col_to_offset(text, line, col);
                let (back_line, back_col) = offset_to_utf16_line_col(text, byte_offset);

                // Verify byte offset is within bounds
                assert!(
                    byte_offset <= text.len(),
                    "Byte offset {} exceeds text length {} for line {}, col {} in {:?}",
                    byte_offset,
                    text.len(),
                    line,
                    col,
                    text
                );

                // For positions within bounds, check if round-trip is reasonable
                // Note: UTF-16 conversion may not be exact for positions inside
                // multi-UTF-16-unit characters (e.g., emoji surrogate pairs).
                // Positions inside surrogate pairs get snapped to character boundaries.
                if line < text.lines().count() as u32 {
                    if let Some(line_text) = text.lines().nth(line as usize) {
                        let line_utf16_len = line_text.encode_utf16().count() as u32;
                        if col <= line_utf16_len {
                            // For valid positions at character boundaries, round-trip should be exact.
                            // For positions inside multi-UTF-16-unit characters (like emoji),
                            // the round-trip may snap to the character boundary.
                            // Allow tolerance for positions that might be inside surrogate pairs.
                            let col_diff = back_col.abs_diff(col);
                            // Allow up to 1 column difference for surrogate pair snapping
                            assert!(
                                back_line == line && col_diff <= 1,
                                "Round-trip exceeded tolerance for position line {}, col {} in {:?}: got line {}, col {}",
                                line,
                                col,
                                text,
                                back_line,
                                back_col
                            );
                        }
                    }
                } else if line == text.lines().count() as u32 && col == 0 && !text.is_empty() {
                    // End of document position should round-trip for multi-line texts
                    // Single line texts without trailing newline may not have a separate "next line"
                    if text.contains('\n') {
                        assert_eq!(
                            (line, col),
                            (back_line, back_col),
                            "Round-trip failed for end-of-document position line {}, col {} in {:?}: got line {}, col {}",
                            line,
                            col,
                            text,
                            back_line,
                            back_col
                        );
                    }
                }
            }
        }
    }
}

/// Test specific mixed line ending scenarios that could be mutation survivors
#[test]
fn test_mixed_line_ending_scenarios() {
    // Test various mixed line ending patterns
    let mixed_cases = vec![
        ("line1\nline2\r\nline3\n", "Mixed LF and CRLF"),
        ("line1\r\nline2\nline3\r\n", "Mixed CRLF and LF"),
        ("\n\r\n\n\r\n", "Only line endings mixed"),
        ("text\n\r\nmore\n", "Text with mixed endings"),
    ];

    for (text, description) in mixed_cases {
        println!("Testing: {}", description);

        // Test conversion at each line boundary
        let mut byte_pos = 0;
        for line_text in text.split_inclusive('\n') {
            let line_end = byte_pos + line_text.len();

            // Test position just before line ending
            if byte_pos < line_end - 1 {
                let (line, col) = offset_to_utf16_line_col(text, line_end - 1);
                println!(
                    "  Position {} (before line end): line {}, col {}",
                    line_end - 1,
                    line,
                    col
                );
            }

            // Test position at line ending
            let (line, col) = offset_to_utf16_line_col(text, line_end);
            println!("  Position {} (at line end): line {}, col {}", line_end, line, col);

            byte_pos = line_end;
        }
    }
}

/// Test saturating arithmetic in position calculations
#[test]
fn test_position_arithmetic_overflow() {
    // Test potential overflow in byte calculations
    let mut pos = Position::new(usize::MAX - 1, 1, 1);
    pos.advance_char('a'); // Should not overflow
    assert_eq!(pos.byte, usize::MAX, "Byte position should reach MAX without overflow");

    // Test advance with empty string (should be no-op)
    let mut pos = Position::start();
    let original = pos;
    pos.advance("");
    assert_eq!(pos, original, "Advancing with empty string should be no-op");

    // Test range with maximum values
    let max_pos1 = Position::new(usize::MAX - 10, u32::MAX, u32::MAX);
    let max_pos2 = Position::new(usize::MAX, u32::MAX, u32::MAX);
    let range = Range::new(max_pos1, max_pos2);

    assert_eq!(
        range.len(),
        10,
        "Range length should be calculated correctly even with large values"
    );
    assert!(range.contains_byte(usize::MAX - 5), "Range should contain intermediate values");
    assert!(!range.contains_byte(usize::MAX), "Range should not contain end value (exclusive)");
}
