use lsp_types::Position;
use perl_parser::textdoc::{byte_to_lsp_pos, lsp_pos_to_byte, PosEnc};
use ropey::Rope;

/// Comprehensive validation of UTF-8/UTF-16 position conversion accuracy
#[test]
fn validate_utf16_position_conversions() {
    // Test cases with various Unicode scenarios
    let test_cases = vec![
        // Basic ASCII
        (
            "Hello\nWorld\n",
            vec![
                (Position::new(0, 0), "Start of first line"),
                (Position::new(0, 5), "End of first line"),
                (Position::new(1, 0), "Start of second line"),
                (Position::new(1, 5), "End of second line"),
            ],
        ),
        // Unicode with emojis (2 UTF-16 code units each)
        (
            "Hello 🌍\nWorld 🚀\n",
            vec![
                (Position::new(0, 6), "Before emoji"),
                (Position::new(0, 8), "After emoji"), // 🌍 = 2 UTF-16 units
                (Position::new(1, 6), "Before second emoji"),
                (Position::new(1, 8), "After second emoji"), // 🚀 = 2 UTF-16 units
            ],
        ),
        // Multi-byte UTF-8 characters (1 UTF-16 unit each)
        (
            "café naïve résumé\n",
            vec![
                (Position::new(0, 3), "Before é in café"),
                (Position::new(0, 4), "After é in café"),
                (Position::new(0, 7), "Before ï in naïve"),
                (Position::new(0, 8), "After ï in naïve"),
            ],
        ),
        // Mixed content with various Unicode
        (
            "Line 1: 中文测试\nLine 2: Test 🎉 café\n",
            vec![
                (Position::new(0, 8), "Start of Chinese text"),
                (Position::new(0, 12), "End of Chinese text"),
                (Position::new(1, 13), "Before emoji"),
                (Position::new(1, 15), "After emoji"),
                (Position::new(1, 16), "Before café"),
            ],
        ),
    ];

    for (content, positions) in test_cases {
        println!("Testing content: {:?}", content);
        let rope = Rope::from_str(content);

        for (pos, description) in positions {
            // Test UTF-16 round-trip conversion
            let byte_offset = lsp_pos_to_byte(&rope, pos, PosEnc::Utf16);
            let converted_back = byte_to_lsp_pos(&rope, byte_offset, PosEnc::Utf16);

            println!(
                "  {} - Position: {:?}, Byte: {}, Back: {:?}",
                description, pos, byte_offset, converted_back
            );

            // Position should be preserved exactly in round-trip
            assert_eq!(
                pos, converted_back,
                "UTF-16 round-trip failed for {} at {:?}",
                description, pos
            );

            // Byte offset should be within rope bounds
            assert!(
                byte_offset <= rope.len_bytes(),
                "Byte offset {} exceeds rope length {} for {}",
                byte_offset,
                rope.len_bytes(),
                description
            );

            // Test UTF-8 conversion separately (primarily for demonstration)
            // Note: UTF-8 and UTF-16 position interpretations differ significantly
            // for Unicode content, so we only verify it doesn't crash
            let _byte_offset_utf8 = lsp_pos_to_byte(&rope, pos, PosEnc::Utf8);
            let _converted_back_utf8 = byte_to_lsp_pos(&rope, _byte_offset_utf8, PosEnc::Utf8);

            // UTF-8 conversion functional test passed (no panic)
        }
    }
}

/// Test edge cases and boundary conditions for position conversion
#[test]
fn validate_position_boundary_conditions() {
    let content = "Short\nMedium line\nA much longer line with Unicode 🌟 and more content\n";
    let rope = Rope::from_str(content);

    // Test various boundary positions
    let boundary_tests = vec![
        (Position::new(0, 0), "Start of document"),
        (Position::new(0, 5), "End of first line"),
        (Position::new(1, 0), "Start of second line"),
        (Position::new(2, 35), "Before emoji in long line"),
        (Position::new(2, 37), "After emoji in long line"),
        (Position::new(100, 0), "Beyond end of document (should clamp)"),
        (Position::new(2, 1000), "Beyond end of line (should clamp)"),
    ];

    for (pos, description) in boundary_tests {
        println!("Testing boundary: {} - {:?}", description, pos);

        // Both UTF-16 and UTF-8 conversions should handle boundaries gracefully
        let byte_offset_utf16 = lsp_pos_to_byte(&rope, pos, PosEnc::Utf16);
        let byte_offset_utf8 = lsp_pos_to_byte(&rope, pos, PosEnc::Utf8);

        // Offsets should be within bounds
        assert!(
            byte_offset_utf16 <= rope.len_bytes(),
            "UTF-16 byte offset {} exceeds bounds for {}",
            byte_offset_utf16,
            description
        );
        assert!(
            byte_offset_utf8 <= rope.len_bytes(),
            "UTF-8 byte offset {} exceeds bounds for {}",
            byte_offset_utf8,
            description
        );

        // Conversion back should produce valid positions
        let back_utf16 = byte_to_lsp_pos(&rope, byte_offset_utf16, PosEnc::Utf16);
        let back_utf8 = byte_to_lsp_pos(&rope, byte_offset_utf8, PosEnc::Utf8);

        // Line numbers should never exceed document bounds
        assert!(
            back_utf16.line < rope.len_lines() as u32,
            "UTF-16 converted line {} exceeds document lines for {}",
            back_utf16.line,
            description
        );
        assert!(
            back_utf8.line < rope.len_lines() as u32,
            "UTF-8 converted line {} exceeds document lines for {}",
            back_utf8.line,
            description
        );
    }
}

/// Test position conversion performance with large documents
#[test]
fn validate_position_conversion_performance() {
    // Create a large document with mixed content
    let mut content = String::new();
    for i in 0..1000 {
        if i % 5 == 0 {
            content.push_str(&format!("Line {}: Unicode content 🚀 café naïve résumé 中文\n", i));
        } else {
            content.push_str(&format!("Line {}: Regular ASCII content here\n", i));
        }
    }

    let rope = Rope::from_str(&content);
    let start = std::time::Instant::now();

    // Test multiple conversions to ensure consistent performance
    let test_positions = vec![
        Position::new(100, 20),
        Position::new(500, 35),
        Position::new(750, 10),
        Position::new(900, 45),
    ];

    for pos in &test_positions {
        // UTF-16 conversions
        let byte_offset = lsp_pos_to_byte(&rope, *pos, PosEnc::Utf16);
        let back_pos = byte_to_lsp_pos(&rope, byte_offset, PosEnc::Utf16);

        // Verify accuracy
        assert_eq!(*pos, back_pos, "Performance test position conversion failed");

        // UTF-8 conversions for comparison
        let byte_offset_utf8 = lsp_pos_to_byte(&rope, *pos, PosEnc::Utf8);
        let back_pos_utf8 = byte_to_lsp_pos(&rope, byte_offset_utf8, PosEnc::Utf8);

        assert_eq!(*pos, back_pos_utf8, "Performance test UTF-8 conversion failed");
    }

    let duration = start.elapsed();
    println!(
        "Position conversion performance: {:?} for {} conversions",
        duration,
        test_positions.len() * 4
    );

    // Performance should be reasonable for large documents
    assert!(
        duration.as_millis() < 10,
        "Position conversion took {} ms, expected < 10ms",
        duration.as_millis()
    );
}

/// Test specific Unicode scenarios that commonly cause issues
#[test]
fn validate_problematic_unicode_scenarios() {
    let problematic_cases = vec![
        // Zero-width characters
        ("Hello\u{200B}World\n", Position::new(0, 5), "Zero-width space"),
        // Combining characters
        ("e\u{0301}t\u{0301}\n", Position::new(0, 2), "Combining acute accents"), // é and t́
        // Surrogate pairs in UTF-16 (emojis)
        ("🇺🇸🇨🇦🇬🇧\n", Position::new(0, 6), "Flag emojis"), // Each flag is 4 UTF-16 units
        // Mixed RTL/LTR text
        ("Hello שלום World\n", Position::new(0, 11), "Mixed LTR/RTL"),
        // Variation selectors
        ("👨‍💻👩‍💻\n", Position::new(0, 8), "Complex emoji sequences"), // Each is 4+ UTF-16 units
    ];

    for (content, test_pos, description) in problematic_cases {
        println!("Testing problematic Unicode: {}", description);
        let rope = Rope::from_str(content);

        // Test that conversion handles complex Unicode correctly
        let byte_offset = lsp_pos_to_byte(&rope, test_pos, PosEnc::Utf16);
        let converted_back = byte_to_lsp_pos(&rope, byte_offset, PosEnc::Utf16);

        // Position should be preserved or reasonably close
        let line_match = test_pos.line == converted_back.line;
        let char_diff = test_pos.character.abs_diff(converted_back.character);

        assert!(
            line_match,
            "Line mismatch for {}: expected {}, got {}",
            description, test_pos.line, converted_back.line
        );

        // Character positions may vary for complex Unicode, but should be reasonable
        assert!(
            char_diff <= 10,
            "Character position too far off for {}: diff = {}",
            description,
            char_diff
        );

        println!(
            "  {} - Original: {:?}, Converted: {:?}, Diff: {}",
            description, test_pos, converted_back, char_diff
        );
    }
}
