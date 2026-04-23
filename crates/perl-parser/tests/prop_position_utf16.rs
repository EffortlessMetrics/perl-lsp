use perl_parser::position::{offset_to_utf16_line_col, utf16_line_col_to_offset};
/// Property-based tests for UTF-16 position conversion
/// Uses proptest to generate randomized inputs and verify invariants
/// that should hold for all valid UTF-16 position conversions.
use proptest::prelude::*;

/// Generate reasonable text content for testing
fn arb_text() -> impl Strategy<Value = String> {
    prop_oneof![
        // ASCII content
        "[a-zA-Z0-9 \t]*",
        // ASCII with newlines
        prop::collection::vec("[ a-zA-Z0-9]*", 1..10).prop_map(|lines| lines.join("\n")),
        // Unicode content
        "[\u{0080}-\u{00FF} \t\n]*", // Latin-1 Supplement
        // Mixed content with emojis
        prop::collection::vec(
            prop_oneof![
                "[a-zA-Z0-9 ]*",
                "[\u{1F600}-\u{1F64F}]", // Emoticons
                "[\u{4E00}-\u{9FFF}]",   // Chinese characters
            ],
            1..5
        )
        .prop_map(|parts| parts.join(""))
    ]
}

/// Generate valid line/column positions
fn arb_position() -> impl Strategy<Value = (u32, u32)> {
    (0u32..100, 0u32..200)
}

proptest! {
    /// Property: Round-trip conversion should preserve positions for valid inputs
    #[test]
    fn prop_utf16_roundtrip_preserves_valid_positions(
        text in arb_text(),
        (line, col) in arb_position()
    ) {
        // Only test positions that could be valid
        if text.is_empty() && (line > 0 || col > 0) {
            return Ok(()); // Skip invalid positions for empty text
        }

        let line_count = text.lines().count() as u32;
        if line >= line_count {
            return Ok(()); // Skip positions beyond text
        }

        // Get the line at the specified line number
        if let Some(line_text) = text.lines().nth(line as usize) {
            let line_char_count = line_text.chars().count() as u32;
            if col > line_char_count {
                return Ok(()); // Skip positions beyond line end
            }
        }

        // Perform round-trip conversion
        let byte_offset = utf16_line_col_to_offset(&text, line, col);
        let (back_line, back_col) = offset_to_utf16_line_col(&text, byte_offset);

        // Round-trip should preserve the position exactly for valid inputs
        prop_assert_eq!((line, col), (back_line, back_col),
            "Round-trip failed for text: {:?}, position: ({}, {}), byte_offset: {}, back: ({}, {})",
            text, line, col, byte_offset, back_line, back_col);
    }

    /// Property: Byte offset should always be within text bounds
    #[test]
    fn prop_utf16_byte_offset_within_bounds(
        text in arb_text(),
        (line, col) in arb_position()
    ) {
        let byte_offset = utf16_line_col_to_offset(&text, line, col);

        prop_assert!(
            byte_offset <= text.len(),
            "Byte offset {} exceeds text length {} for position ({}, {}) in text: {:?}",
            byte_offset, text.len(), line, col, text
        );
    }

    /// Property: Converting any byte offset should produce valid line/column
    #[test]
    fn prop_offset_to_utf16_produces_valid_position(
        text in arb_text(),
        offset in any::<usize>()
    ) {
        // Clamp offset to reasonable range to avoid excessive test times
        let offset = offset % (text.len() + 100);

        let (line, col) = offset_to_utf16_line_col(&text, offset);

        // Line should be within document bounds
        let line_count = text.lines().count() as u32;
        prop_assert!(
            line <= line_count,
            "Line {} exceeds document line count {} for offset {} in text: {:?}",
            line, line_count, offset, text
        );

        // If line is within bounds, column should be reasonable
        if line < line_count
            && let Some(line_text) = text.lines().nth(line as usize) {
                let line_utf16_len = line_text.encode_utf16().count() as u32;
                prop_assert!(
                    col <= line_utf16_len,
                    "Column {} exceeds line UTF-16 length {} for line {} at offset {} in text: {:?}",
                    col, line_utf16_len, line, offset, text
                );
            }
    }

    /// Property: Monotonicity - later byte offsets should not produce earlier positions
    #[test]
    fn prop_offset_monotonicity(
        text in arb_text(),
        offset1 in any::<usize>(),
        offset2 in any::<usize>()
    ) {
        if text.is_empty() {
            return Ok(());
        }

        let offset1 = offset1 % text.len();
        let offset2 = offset2 % text.len();

        if offset1 > offset2 {
            return Ok(()); // Only test when offset1 <= offset2
        }

        let (line1, col1) = offset_to_utf16_line_col(&text, offset1);
        let (line2, col2) = offset_to_utf16_line_col(&text, offset2);

        // Later offset should produce later or equal position
        prop_assert!(
            line1 < line2 || (line1 == line2 && col1 <= col2),
            "Position monotonicity violated: offset {} -> ({}, {}), offset {} -> ({}, {}) in text: {:?}",
            offset1, line1, col1, offset2, line2, col2, text
        );
    }

    /// Property: Line boundaries should be consistent
    #[test]
    fn prop_line_boundaries_consistent(
        text in arb_text(),
        line_num in 0u32..50
    ) {
        if text.is_empty() || line_num >= text.lines().count() as u32 {
            return Ok(());
        }

        // Get the start of the line
        let (_, _col_start) = offset_to_utf16_line_col(&text, 0);
        let line_start_offset = utf16_line_col_to_offset(&text, line_num, 0);
        let (back_line, back_col) = offset_to_utf16_line_col(&text, line_start_offset);

        prop_assert_eq!(back_line, line_num,
            "Line start conversion inconsistent for line {} in text: {:?}",
            line_num, text);
        prop_assert_eq!(back_col, 0,
            "Line start should have column 0 for line {} in text: {:?}",
            line_num, text);

        // Test line end boundary if line exists
        if let Some(line_text) = text.lines().nth(line_num as usize) {
            let line_utf16_len = line_text.encode_utf16().count() as u32;
            let line_end_offset = utf16_line_col_to_offset(&text, line_num, line_utf16_len);
            let (end_line, end_col) = offset_to_utf16_line_col(&text, line_end_offset);

            // Line end should be at line boundary or next line start
            prop_assert!(
                end_line == line_num || (end_line == line_num + 1 && end_col == 0),
                "Line end conversion inconsistent: line {}, utf16_len {}, offset {}, result ({}, {}) in text: {:?}",
                line_num, line_utf16_len, line_end_offset, end_line, end_col, text
            );
        }
    }

    /// Property: UTF-16 position should handle multibyte characters correctly
    #[test]
    fn prop_multibyte_character_handling(
        prefix in "[a-zA-Z0-9]*",
        multibyte_char in "[\u{1F600}-\u{1F64F}]", // Emoji range
        suffix in "[a-zA-Z0-9]*"
    ) {
        let text = format!("{}{}{}", prefix, multibyte_char, suffix);
        let emoji_byte_start = prefix.len();
        let emoji_utf16_start = prefix.encode_utf16().count() as u32;

        // Position just before emoji
        let (_line_before, col_before) = offset_to_utf16_line_col(&text, emoji_byte_start);
        prop_assert_eq!(col_before, emoji_utf16_start,
            "Position before emoji should match UTF-16 prefix length");

        // Position just after emoji
        let emoji_byte_len = multibyte_char.len();
        let emoji_utf16_len = multibyte_char.encode_utf16().count() as u32;
        let (line_after, col_after) = offset_to_utf16_line_col(&text, emoji_byte_start + emoji_byte_len);
        prop_assert_eq!(col_after, emoji_utf16_start + emoji_utf16_len,
            "Position after emoji should account for UTF-16 length");

        // Round-trip through the emoji position
        let roundtrip_offset = utf16_line_col_to_offset(&text, line_after, col_after);
        prop_assert_eq!(roundtrip_offset, emoji_byte_start + emoji_byte_len,
            "Round-trip through emoji position should be exact");
    }

    /// Property: Position clamping should work correctly for out-of-bounds positions
    #[test]
    fn prop_position_clamping(
        text in arb_text(),
        large_line in 1000u32..2000u32,
        large_col in 1000u32..2000u32
    ) {
        let byte_offset = utf16_line_col_to_offset(&text, large_line, large_col);

        // Clamped position should be at or near text end
        prop_assert!(
            byte_offset == text.len(),
            "Out-of-bounds position should clamp to text end: got offset {} for text length {} with position ({}, {})",
            byte_offset, text.len(), large_line, large_col
        );

        // Converting back should give reasonable position
        let (clamped_line, _clamped_col) = offset_to_utf16_line_col(&text, byte_offset);
        let line_count = text.lines().count() as u32;

        prop_assert!(
            clamped_line <= line_count,
            "Clamped line {} should not exceed line count {} for text: {:?}",
            clamped_line, line_count, text
        );
    }

    /// Property: Empty lines should be handled correctly
    #[test]
    fn prop_empty_lines_handling(
        lines_before in prop::collection::vec("[a-zA-Z0-9 ]*", 0..5),
        empty_line_count in 1usize..5,
        lines_after in prop::collection::vec("[a-zA-Z0-9 ]*", 0..5)
    ) {
        let mut all_lines = lines_before;
        for _ in 0..empty_line_count {
            all_lines.push("".to_string());
        }
        all_lines.extend(lines_after);

        let text = all_lines.join("\n");

        // Test positions at empty line boundaries
        let mut byte_offset = 0;
        for (line_idx, line_content) in all_lines.iter().enumerate() {
            let (line, col) = offset_to_utf16_line_col(&text, byte_offset);
            prop_assert_eq!(line, line_idx as u32,
                "Line number should match for byte offset {} in text with empty lines: {:?}",
                byte_offset, text);
            prop_assert_eq!(col, 0,
                "Column should be 0 at start of line {} in text with empty lines: {:?}",
                line_idx, text);

            // Move to next line
            byte_offset += line_content.len();
            if line_idx < all_lines.len() - 1 {
                byte_offset += 1; // Account for newline
            }
        }
    }
}
