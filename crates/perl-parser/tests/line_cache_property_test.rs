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
            ("👋 hello\n🌍 world", vec![
                (0, 0),   // Start of first line
                (5, 3),   // After emoji+space (emoji=2 UTF-16, space=1)
                (10, 8),  // After "hello"
                (11, 0),  // Start of second line
                (16, 3),  // After emoji+space
                (21, 8),  // Last char of "world"
            ]),
            
            // BMP characters (single UTF-16 units)
            ("καλημέρα\nκόσμος", vec![
                (0, 0),   // Start
                (16, 8),  // End of first word (8 Greek chars)
                (17, 0),  // Start of second line
                (29, 6),  // End of second word
            ]),
            
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
                    (line, col), (slow_line, slow_col),
                    "Cache mismatch for offset {} in {:?}",
                    offset, content
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

    /// Test that cache handles all Unicode planes correctly
    #[test]
    fn unicode_planes() {
        // Test characters from different Unicode planes
        let content = concat!(
            "ASCII\n",                      // Basic Latin
            "Café\n",                        // Latin-1 Supplement
            "Привет\n",                      // Cyrillic
            "你好\n",                         // CJK
            "𝐇𝐞𝐥𝐥𝐨\n",                        // Mathematical Alphanumeric
            "👨‍👩‍👧‍👦 Family\n",                   // Emoji with ZWJ sequences
        );
        
        let cache = LineStartsCache::new(content);
        
        // Test that we can convert positions on each line
        let test_points = vec![
            (0, 0, 0),      // Start of ASCII
            (6, 1, 0),      // Start of Café
            (12, 2, 0),     // Start of Cyrillic  
            (25, 3, 0),     // Start of CJK (after "Привет\n")
            (32, 4, 0),     // Start of Mathematical (after "你好\n")
            (53, 5, 0),     // Start of Emoji line (after "𝐇𝐞𝐥𝐥𝐨\n")
        ];
        
        for (offset, expected_line, expected_col) in test_points {
            let (line, col) = cache.offset_to_position(content, offset);
            assert_eq!(
                (line, col), (expected_line, expected_col),
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
            (0, 0),               // Start
            (1000, 0),            // Somewhere in middle
            (5000, 0),            // Another middle point
            (9999, 0),            // Near end
        ];
        
        for (line, _) in positions {
            let offset = cache.position_to_offset(&content, line, 0);
            let (rt_line, rt_col) = cache.offset_to_position(&content, offset);
            assert_eq!(rt_line, line, "Round-trip failed for line {}", line);
            assert_eq!(rt_col, 0, "Column should be 0 at line start");
        }
    }
}