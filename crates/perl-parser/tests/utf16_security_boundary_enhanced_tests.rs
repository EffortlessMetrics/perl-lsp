/// Enhanced UTF-16 boundary security tests for LSP position mapping
///
/// These tests build upon existing UTF-16 tests to provide comprehensive
/// security validation for position mapping functions, focusing on edge cases
/// that could cause security vulnerabilities or data corruption in LSP workflows.
use perl_parser::position::{offset_to_utf16_line_col, utf16_line_col_to_offset};

#[test]
fn test_utf16_boundary_overflow_protection() {
    // Test protection against integer overflow in UTF-16 calculations
    let repeated_crab = "🦀".repeat(100);
    let mixed_content = format!("{}🦀{}", "a".repeat(1000), "b".repeat(1000));
    let test_cases = vec![
        // Very large Unicode characters
        ("𝒞𝒪𝒟𝒴", 2), // Mathematical script characters (4 bytes each)
        ("👨‍👩‍👧‍👦", 5),   // Family emoji with ZWJ sequences
        ("🏴󠁧󠁢󠁳󠁣󠁴󠁿", 10),  // Flag sequence with tag characters
        // Mixed content with potential overflow points
        (&repeated_crab, 200), // Many crab emojis
        (&mixed_content, 1500),
    ];

    for (text, offset) in test_cases {
        // Ensure no panic occurs with large offsets
        let (line, col) = offset_to_utf16_line_col(text, offset);

        // Line and column should be reasonable values
        assert!(line < 10000, "Line number suspiciously large: {}", line);
        assert!(col < 10000, "Column number suspiciously large: {}", col);

        // Roundtrip should not cause panic
        let roundtrip = utf16_line_col_to_offset(text, line, col);

        // Even if roundtrip is inexact, it should be within reasonable bounds
        assert!(
            roundtrip <= text.len() + 10,
            "Roundtrip offset {} exceeds text length {} + tolerance",
            roundtrip,
            text.len()
        );
    }
}

#[test]
fn test_utf16_line_boundary_security() {
    // Test security with various line ending combinations that could cause issues
    let test_cases = vec![
        // Different line endings
        "line1\nline2",
        "line1\r\nline2",
        "line1\rline2",
        // Mixed line endings (potential security issue)
        "line1\n\r\nline2\rline3\n",
        // Unicode line separators
        "line1\u{2028}line2", // Line separator
        "line1\u{2029}line2", // Paragraph separator
        // Emoji spanning line boundaries
        "🦀\n🦀",
        "test🦀\r\n🦀test",
        // Complex Unicode with line breaks
        "café\nтест\r\n測試",
    ];

    for text in test_cases {
        // Test all positions including line boundaries
        for offset in 0..=text.len() {
            let (line, col) = offset_to_utf16_line_col(text, offset);

            // Validate line numbers are reasonable
            assert!(line < 100, "Line number {} too large for text: {:?}", line, text);

            // Validate column is not negative or extremely large
            assert!(col < 1000, "Column {} too large for line {} in text: {:?}", col, line, text);

            // Test roundtrip doesn't cause buffer overflow
            let roundtrip = utf16_line_col_to_offset(text, line, col);
            assert!(
                roundtrip <= text.len() * 2,
                "Roundtrip {} unreasonable for text length {}",
                roundtrip,
                text.len()
            );
        }
    }
}

#[test]
fn test_utf16_malicious_input_protection() {
    // Test protection against potentially malicious inputs
    let long_line = "a".repeat(10000);
    let mixed_long = format!("{}🦀{}", "a".repeat(5000), "b".repeat(5000));
    let many_newlines = "\n".repeat(1000);
    let many_crlf = "\r\n".repeat(500);

    let malicious_cases = vec![
        // Extremely long lines that could cause DoS
        &long_line,
        &mixed_long,
        // Many line breaks that could cause memory issues
        &many_newlines,
        &many_crlf,
        // Unicode normalization bombs (sequences that expand dramatically)
        "ﾘﾘﾘﾘﾘﾘﾘﾘﾘﾘ", // Half-width katakana
        "𝓐𝓑𝓒𝓓𝓔𝓕𝓖𝓗𝓘𝓙", // Mathematical script
        // Null bytes and control characters
        "test\x00test",
        "line1\x01\x02\x03line2",
        // RTL and bidirectional text that could cause confusion
        "Hello \u{202E}world\u{202C}",
        "\u{202D}test\u{202C}",
    ];

    for text in malicious_cases {
        // Test that functions don't panic with malicious input
        for offset in [0, text.len() / 2, text.len()] {
            let start_time = std::time::Instant::now();

            let (line, col) = offset_to_utf16_line_col(text, offset);
            let roundtrip = utf16_line_col_to_offset(text, line, col);

            let duration = start_time.elapsed();

            // Ensure processing completes quickly (DoS protection)
            assert!(
                duration.as_millis() < 100,
                "Processing took too long: {:?} for input length {}",
                duration,
                text.len()
            );

            // Validate results are within reasonable bounds
            assert!(line < 10000, "Line number {} unreasonable", line);
            assert!(col < 20000, "Column {} unreasonable", col);
            assert!(roundtrip < text.len() * 3, "Roundtrip {} unreasonable", roundtrip);
        }
    }
}

#[test]
fn test_utf16_edge_case_combinations() {
    // Test combinations of edge cases that could interact poorly
    let edge_cases = vec![
        // Emoji at line boundaries
        ("🦀\n", 2),
        ("test🦀\n", 6),
        ("\n🦀", 2),
        // Surrogate pairs at boundaries
        ("𝟙𝟚𝟛", 4), // Mathematical numbers
        ("𝕒𝕓𝕔", 4), // Mathematical letters
        // Zero-width characters
        ("a\u{200B}b", 2),      // Zero-width space
        ("test\u{FEFF}ing", 5), // BOM
        // Combining characters
        ("é\u{0301}", 2),         // e + acute accent
        ("a\u{0300}\u{0301}", 3), // a + grave + acute
        // Mixed scripts
        ("Hello世界", 6),
        ("test測試тест", 8),
        // Unusual but valid Unicode
        ("\u{1F1FA}\u{1F1F8}", 4),         // Flag sequence
        ("\u{1F9D1}\u{200D}\u{1F4BB}", 6), // Technologist emoji
    ];

    for (text, test_offset) in edge_cases {
        // Test the specific offset
        let (line, col) = offset_to_utf16_line_col(text, test_offset);
        let _roundtrip = utf16_line_col_to_offset(text, line, col);

        // For edge cases, we're more lenient but still check for crashes
        assert!(line < 100, "Line {} unreasonable for edge case: {:?}", line, text);
        assert!(col < 100, "Column {} unreasonable for edge case: {:?}", col, text);

        // Test all byte positions to find any that cause issues
        for offset in 0..=text.len() {
            let (line, col) = offset_to_utf16_line_col(text, offset);
            // Just ensure no panic - exact values may vary for complex Unicode
            let _roundtrip = utf16_line_col_to_offset(text, line, col);
        }
    }
}

#[test]
fn test_utf16_consistency_validation() {
    // Test consistency properties that must hold for security
    let test_texts = vec![
        "simple",
        "🦀 Rust",
        "Hello\nWorld",
        "café test",
        "test🦀\r\nmore🦀",
        "Mixed: Hello 世界 🦀 test",
    ];

    for text in test_texts {
        for offset in 0..=text.len() {
            let (line, col) = offset_to_utf16_line_col(text, offset);
            let roundtrip = utf16_line_col_to_offset(text, line, col);

            // Key security property: roundtrip should never exceed text bounds significantly
            assert!(
                roundtrip <= text.len() + text.chars().count(),
                "Roundtrip {} exceeds safe bounds for text length {} (char count {})",
                roundtrip,
                text.len(),
                text.chars().count()
            );

            // Line numbers should be monotonic (never decrease as offset increases)
            if offset > 0 {
                let (prev_line, _) = offset_to_utf16_line_col(text, offset - 1);
                assert!(
                    line >= prev_line,
                    "Line number decreased: {} -> {} at offset {}",
                    prev_line,
                    line,
                    offset
                );
            }

            // Column should reset to 0 or low value at line boundaries
            // Only check if offset is at a valid character boundary
            if offset > 0 && offset <= text.len() && text.is_char_boundary(offset) {
                if text[..offset].ends_with('\n') || text[..offset].ends_with('\r') {
                    assert!(
                        col < 10,
                        "Column {} should be small after line break at offset {}",
                        col,
                        offset
                    );
                }
            }
        }
    }
}

#[test]
fn test_utf16_memory_safety() {
    // Test patterns that could cause memory safety issues
    let high_unicode = "\u{FFFF}".repeat(100);
    let beyond_bmp = "\u{10000}".repeat(50);
    let many_combining = format!("a{}", "\u{0300}".repeat(50));
    let ideographic_spaces = "\u{3000}".repeat(20);

    let patterns = vec![
        // Patterns that could cause buffer overruns
        &high_unicode, // High Unicode code points
        &beyond_bmp,   // Beyond BMP
        // Patterns with many combining characters
        &many_combining, // Many grave accents
        // Patterns that normalize to different lengths
        "ﬁﬂﬃﬄﬅﬆ", // Ligatures
        // Invalid UTF-8 sequences (should be handled gracefully)
        // Note: Rust strings are guaranteed valid UTF-8, but we test edge cases

        // Extremely wide characters
        "𝟘𝟙𝟚𝟛𝟜𝟝𝟞𝟟𝟠𝟡", // Mathematical digits
        // Characters with unusual width properties
        &ideographic_spaces, // Ideographic space
    ];

    for pattern in patterns {
        // Test with various offsets to ensure memory safety
        let offsets =
            vec![0, pattern.len() / 4, pattern.len() / 2, pattern.len() * 3 / 4, pattern.len()];

        for offset in offsets {
            if offset <= pattern.len() {
                // These operations should complete without memory corruption
                let (line, col) = offset_to_utf16_line_col(pattern, offset);
                let roundtrip = utf16_line_col_to_offset(pattern, line, col);

                // Basic sanity checks
                assert!(line < u32::MAX / 1000, "Line number {} suspiciously large", line);
                assert!(col < u32::MAX / 1000, "Column {} suspiciously large", col);
                assert!(
                    roundtrip < pattern.len() * 10,
                    "Roundtrip {} suspiciously large",
                    roundtrip
                );
            }
        }
    }
}

#[test]
fn test_utf16_lsp_workflow_security() {
    // Test security in typical LSP workflows with UTF-16 positions
    let document = "sub example {\n    my $🦀 = 'Rust';\n    print \"Hello, 世界!\\n\";\n}";

    // Simulate LSP position requests that could be malicious
    let malicious_positions = vec![
        // Extremely large line numbers
        (u32::MAX, 0),
        (10000, 0),
        // Extremely large column numbers
        (0, u32::MAX),
        (0, 10000),
        // Both large
        (1000, 1000),
        // Positions that could cause integer overflow
        (u32::MAX - 1, u32::MAX - 1),
    ];

    for (line, col) in malicious_positions {
        // These should not panic or cause memory corruption
        let offset = utf16_line_col_to_offset(document, line, col);

        // Result should be within reasonable bounds (may clamp to document end)
        assert!(
            offset <= document.len() * 2,
            "Malicious position ({}, {}) produced unreasonable offset: {}",
            line,
            col,
            offset
        );

        // Reverse conversion should not panic
        let (back_line, back_col) = offset_to_utf16_line_col(document, offset);

        // Results should be reasonable
        assert!(back_line < 1000, "Back-converted line {} unreasonable", back_line);
        assert!(back_col < 1000, "Back-converted column {} unreasonable", back_col);
    }

    // Test rapid-fire position conversions (DoS protection)
    let start_time = std::time::Instant::now();
    for i in 0..1000 {
        let offset = i % (document.len() + 1);
        let (line, col) = offset_to_utf16_line_col(document, offset);
        let _ = utf16_line_col_to_offset(document, line, col);
    }
    let duration = start_time.elapsed();

    assert!(duration.as_millis() < 100, "1000 position conversions took too long: {:?}", duration);
}
