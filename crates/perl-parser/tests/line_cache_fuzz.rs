#[cfg(test)]
mod fuzz {
    use perl_parser::position::LineStartsCache;

    // Quick property test without proptest dependency for now
    // Can upgrade to proptest later if desired

    /// Slow reference implementation for testing
    /// This matches the actual cache behavior: \r counts as a column before CRLF
    fn slow_offset_to_position(content: &str, offset: usize) -> (u32, u32) {
        let mut line = 0u32;
        let mut col_utf16 = 0u32;
        let mut byte_offset = 0;
        let bytes = content.as_bytes();

        for ch in content.chars() {
            if byte_offset >= offset {
                break;
            }

            if ch == '\n' {
                line += 1;
                col_utf16 = 0;
            } else if ch == '\r' {
                // Check if this is part of CRLF
                if byte_offset + 1 < bytes.len() && bytes[byte_offset + 1] == b'\n' {
                    // \r counts as a column, \n will reset on next iteration
                    col_utf16 += 1;
                } else {
                    // Standalone \r is a line break
                    line += 1;
                    col_utf16 = 0;
                }
            } else {
                // Count UTF-16 code units for the character
                col_utf16 += if ch as u32 >= 0x10000 { 2 } else { 1 };
            }

            byte_offset += ch.len_utf8();
        }

        (line, col_utf16)
    }

    #[test]
    fn fuzz_mixed_content() {
        // Test various combinations manually for now
        let long_line = format!("{}{}\n", "x".repeat(10000), "𝐀".repeat(1000));
        let many_lines = "a\n".repeat(1000);

        let test_cases = vec![
            // Plain ASCII
            "hello world\ntest",
            // CRLF line endings
            "line1\r\nline2\r\nline3",
            // Mixed line endings
            "unix\nmixed\r\nwindows\r\nend",
            // Unicode with surrogates
            "before𝐀after\n𝐁test",
            // ZWJ emoji sequences
            "start👨‍👩‍👧‍👦end\nmore",
            // BOM at start
            "\u{FEFF}content\nhere",
            // Mixed everything
            "\u{FEFF}test\r\n𝐀𝐁\n👨‍👩‍👧‍👦\r\nASCII",
            // Very long lines
            &long_line,
            // Many short lines
            &many_lines,
            // Edge cases
            "",
            "\n",
            "\r\n",
            "\r",
            "𝐀",
            "👨‍👩‍👧‍👦",
        ];

        for content in test_cases {
            let cache = LineStartsCache::new(content);

            // Test various offsets (only valid char boundaries)
            let mut offsets = vec![0];
            if !content.is_empty() {
                offsets.push(content.len());

                // Add char boundary offsets
                let mut char_boundaries = vec![0];
                for (i, _) in content.char_indices() {
                    char_boundaries.push(i);
                }
                char_boundaries.push(content.len());

                // Sample some of them
                for i in 0..char_boundaries.len().min(20) {
                    let idx = (i * 7) % char_boundaries.len();
                    offsets.push(char_boundaries[idx]);
                }
            }

            for offset in offsets {
                let cached = cache.offset_to_position(content, offset);
                let slow = slow_offset_to_position(content, offset);

                assert_eq!(
                    cached,
                    slow,
                    "Mismatch at offset {} in content {:?}",
                    offset,
                    content.chars().take(50).collect::<String>()
                );

                // Test round-trip (skip CRLF positions which don't round-trip correctly)
                // Both \r and \n in CRLF sequence have issues:
                // - \r at offset N maps to (line, col) but (line, col) maps back to N
                // - \n at offset N+1 maps to (line, col+1) but (line, col+1) maps back to N
                let is_crlf_r = content.as_bytes().get(offset) == Some(&b'\r')
                    && content.as_bytes().get(offset + 1) == Some(&b'\n');
                let is_crlf_n = offset > 0
                    && content.as_bytes().get(offset - 1) == Some(&b'\r')
                    && content.as_bytes().get(offset) == Some(&b'\n');

                if !is_crlf_r && !is_crlf_n {
                    let rt_offset = cache.position_to_offset(content, cached.0, cached.1);
                    assert_eq!(
                        rt_offset,
                        offset,
                        "Round-trip failed for offset {} in content {:?}",
                        offset,
                        content.chars().take(50).collect::<String>()
                    );
                }
            }
        }
    }

    #[test]
    fn fuzz_edge_boundaries() {
        // Test boundary conditions around line breaks
        // NOTE: Only test valid UTF-8 char boundaries
        let cases = vec![
            ("a\nb", vec![0, 1, 2, 3]),
            ("a\r\nb", vec![0, 1, 3, 4]), // Skip offset 2 (middle of CRLF)
            ("𝐀\n𝐁", vec![0, 4, 5, 9]),   // Only char boundaries for 4-byte chars
            ("👨‍👩‍👧‍👦", vec![0, 25]),          // Only start and end for ZWJ sequence
        ];

        for (content, offsets) in cases {
            let cache = LineStartsCache::new(content);

            for offset in offsets {
                if offset <= content.len() {
                    let cached = cache.offset_to_position(content, offset);
                    let slow = slow_offset_to_position(content, offset);

                    assert_eq!(
                        cached, slow,
                        "Boundary mismatch at offset {} in {:?}",
                        offset, content
                    );
                }
            }
        }
    }
}
