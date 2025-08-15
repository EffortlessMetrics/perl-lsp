#[cfg(test)]
mod line_cache_property_tests {
    use perl_parser::positions::LineStartsCache;

    /// Test that cached position conversions match the slow path exactly
    #[test]
    fn cache_matches_slow_path() {
        // Test various edge cases of line endings and UTF-16
        let test_cases = vec![
            // Simple LF
            ("hello\nworld\n", vec![(0, 0), (5, 5), (6, 0), (11, 5)]),
            // CRLF
            ("hello\r\nworld\r\n", vec![(0, 0), (5, 5), (7, 0), (12, 5)]),
            // Mixed line endings
            ("a\nb\r\nc\r\n", vec![(0, 0), (1, 1), (2, 0), (3, 1), (5, 0), (6, 1)]),
            // UTF-16 with emoji (surrogate pairs)
            (
                "👋 hello\n🌍 world",
                vec![
                    (0, 0),  // Start of first line
                    (5, 3),  // After emoji+space (emoji=2 UTF-16, space=1)
                    (10, 8), // After "hello"
                    (11, 0), // Start of second line
                    (16, 3), // After emoji+space
                    (21, 8), // Last char of "world"
                ],
            ),
            // BMP characters (single UTF-16 units)
            (
                "καλημέρα\nκόσμος",
                vec![
                    (0, 0),  // Start
                    (16, 8), // End of first word (8 Greek chars)
                    (17, 0), // Start of second line
                    (29, 6), // End of second word
                ],
            ),
            // Empty lines
            ("\n\n\n", vec![(0, 0), (1, 0), (2, 0), (3, 0)]),
            // No trailing newline
            ("hello", vec![(0, 0), (5, 5)]),
        ];

        for (content, positions) in test_cases {
            let cache = LineStartsCache::new(content);

            for &(offset, expected_col) in &positions {
                // Test offset to position
                let (line, col) = cache.offset_to_position(content, offset);

                // Verify against slow path
                let (slow_line, slow_col) = slow_offset_to_position(content, offset);
                assert_eq!(
                    (line, col),
                    (slow_line, slow_col),
                    "Cache mismatch for offset {} in {:?}",
                    offset,
                    content
                );

                // Verify expected column
                assert_eq!(
                    col, expected_col as u32,
                    "Column mismatch for offset {} in {:?}",
                    offset, content
                );

                // Test round-trip
                let round_trip_offset = cache.position_to_offset(content, line, col);
                assert_eq!(
                    round_trip_offset, offset,
                    "Round-trip failed for ({}, {}) in {:?}",
                    line, col, content
                );
            }
        }
    }

    /// Slow but correct implementation for testing
    fn slow_offset_to_position(content: &str, offset: usize) -> (u32, u32) {
        let mut line = 0u32;
        let mut col_utf16 = 0u32;
        let mut byte_offset = 0;

        for ch in content.chars() {
            if byte_offset >= offset {
                break;
            }

            if ch == '\n' {
                line += 1;
                col_utf16 = 0;
            } else if ch != '\r' {
                col_utf16 += if ch.len_utf16() == 2 { 2 } else { 1 };
            }

            byte_offset += ch.len_utf8();
        }

        (line, col_utf16)
    }

    /// Test ZWJ sequences, BOM, and other edge cases
    #[test]
    fn unicode_edge_cases() {
        // Test 1: ZWJ sequence (family emoji - grapheme cluster with >2 UTF-16 units)
        let zwj_content = "A👨‍👩‍👧‍👦Z\n";
        let cache = LineStartsCache::new(zwj_content);
        let test_points = vec![
            (0, 0, 0), // 'A'
            (1, 0, 1), // After 'A', before emoji
            // The family emoji is: 👨 (2) + ZWJ (1) + 👩 (2) + ZWJ (1) + 👧 (2) + ZWJ (1) + 👦 (2) = 11 UTF-16 units
            (26, 0, 12), // After full emoji, before 'Z' (25 UTF-8 bytes for emoji, +1 for A)
            (27, 0, 13), // After 'Z'
            (28, 1, 0),  // After newline
        ];

        for (offset, expected_line, expected_col) in test_points {
            let (line, col) = cache.offset_to_position(zwj_content, offset);
            assert_eq!(
                (line, col),
                (expected_line, expected_col),
                "Failed at offset {} in ZWJ test",
                offset
            );
        }

        // Test 2: BOM at start of file
        let bom_content = "\u{FEFF}print 1;\n";
        let cache = LineStartsCache::new(bom_content);
        let test_points = vec![
            (0, 0, 0),  // Start (before BOM)
            (3, 0, 1),  // After BOM (3 bytes UTF-8, 1 UTF-16 unit)
            (8, 0, 6),  // After "print"
            (11, 0, 9), // End of line
            (12, 1, 0), // Start of next line
        ];

        for (offset, expected_line, expected_col) in test_points {
            let (line, col) = cache.offset_to_position(bom_content, offset);
            assert_eq!(
                (line, col),
                (expected_line, expected_col),
                "Failed at offset {} in BOM test",
                offset
            );
        }

        // Test 3: Column clamping on very long lines
        let mut long_line = String::with_capacity(20_000);

        // 10k ASCII chars
        for _ in 0..10_000 {
            long_line.push('x');
        }

        // 5k surrogate pairs (each is 4 UTF-8 bytes, 2 UTF-16 units)
        for _ in 0..5_000 {
            long_line.push('𝐀'); // Mathematical bold A
        }

        long_line.push('\n');
        let cache = LineStartsCache::new(&long_line);

        // Test that we can convert positions at various points
        let test_points = vec![
            (0, 0, 0),           // Start
            (5_000, 0, 5_000),   // Middle of ASCII
            (10_000, 0, 10_000), // End of ASCII
            (10_004, 0, 10_002), // After first surrogate pair (4 UTF-8 bytes = 2 UTF-16 units)
            (30_000, 0, 20_000), // After 5k surrogates (20k UTF-8 bytes = 10k UTF-16 units)
        ];

        for (offset, expected_line, expected_col) in test_points {
            let (line, col) = cache.offset_to_position(&long_line, offset);
            assert_eq!(
                (line, col),
                (expected_line, expected_col),
                "Failed at offset {} in long line test",
                offset
            );

            // Test round-trip
            let rt_offset = cache.position_to_offset(&long_line, line, col);
            assert_eq!(rt_offset, offset, "Round-trip failed for long line");
        }

        // Test clamping: Ask for column way beyond line end
        let clamped = cache.position_to_offset(&long_line, 0, 100_000);
        assert_eq!(clamped, 30_000, "Should clamp to end of line");

        // Verify it round-trips correctly
        let (rt_line, rt_col) = cache.offset_to_position(&long_line, clamped);
        assert_eq!((rt_line, rt_col), (0, 20_000), "Clamped position should round-trip");
    }

    /// Test that cache handles all Unicode planes correctly
    #[test]
    fn unicode_planes() {
        // Test characters from different Unicode planes
        let content = concat!(
            "ASCII\n",     // Basic Latin
            "Café\n",      // Latin-1 Supplement
            "Привет\n",    // Cyrillic
            "你好\n",      // CJK
            "𝐇𝐞𝐥𝐥𝐨\n",     // Mathematical Alphanumeric
            "👨‍👩‍👧‍👦 Family\n", // Emoji with ZWJ sequences
        );

        let cache = LineStartsCache::new(content);

        // Test that we can convert positions on each line
        let test_points = vec![
            (0, 0, 0),  // Start of ASCII
            (6, 1, 0),  // Start of Café
            (12, 2, 0), // Start of Cyrillic
            (25, 3, 0), // Start of CJK (after "Привет\n")
            (32, 4, 0), // Start of Mathematical (after "你好\n")
            (53, 5, 0), // Start of Emoji line (after "𝐇𝐞𝐥𝐥𝐨\n")
        ];

        for (offset, expected_line, expected_col) in test_points {
            let (line, col) = cache.offset_to_position(content, offset);
            assert_eq!(
                (line, col),
                (expected_line, expected_col),
                "Failed at offset {} in Unicode test",
                offset
            );

            // Test round-trip
            let rt_offset = cache.position_to_offset(content, line, col);
            assert_eq!(rt_offset, offset, "Round-trip failed for Unicode");
        }
    }

    /// Stress test with large file
    #[test]
    fn large_file_performance() {
        // Generate a large file with mixed content
        let mut content = String::new();
        for i in 0..10000 {
            if i % 3 == 0 {
                content.push_str(&format!("Line {} with emoji 🎉\n", i));
            } else if i % 3 == 1 {
                content.push_str(&format!("Line {} with Greek καλημέρα\r\n", i));
            } else {
                content.push_str(&format!("Simple line {}\n", i));
            }
        }

        let cache = LineStartsCache::new(&content);

        // Test some random positions
        let positions = vec![
            (0, 0),    // Start
            (1000, 0), // Somewhere in middle
            (5000, 0), // Another middle point
            (9999, 0), // Near end
        ];

        for (line, _) in positions {
            let offset = cache.position_to_offset(&content, line, 0);
            let (rt_line, rt_col) = cache.offset_to_position(&content, offset);
            assert_eq!(rt_line, line, "Round-trip failed for line {}", line);
            assert_eq!(rt_col, 0, "Column should be 0 at line start");
        }
    }
}
