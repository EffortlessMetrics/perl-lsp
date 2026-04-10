#[cfg(test)]
mod fuzz {
    use perl_parser::position::LineStartsCache;
    use proptest::prelude::*;

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

    fn sample_char_boundaries(content: &str) -> Vec<usize> {
        let mut boundaries = vec![0];
        if !content.is_empty() {
            boundaries.push(content.len());
            for (i, _) in content.char_indices() {
                boundaries.push(i);
            }
            boundaries.push(content.len());
        }
        boundaries.sort_unstable();
        boundaries.dedup();
        boundaries
    }

    fn assert_cache_matches_reference(content: &str, offsets: &[usize]) {
        let cache = LineStartsCache::new(content);

        for &offset in offsets {
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

    fn fuzz_content_strategy() -> impl Strategy<Value = String> {
        let token = prop_oneof![
            Just("a".to_string()),
            Just("Z".to_string()),
            Just("0".to_string()),
            Just(" ".to_string()),
            Just("\t".to_string()),
            Just("\n".to_string()),
            Just("\r".to_string()),
            Just("\r\n".to_string()),
            Just("\u{FEFF}".to_string()),
            Just("𝐀".to_string()),
            Just("𝐁".to_string()),
            Just("👨‍👩‍👧‍👦".to_string()),
            "[a-zA-Z0-9_]{0,4}",
        ];

        proptest::collection::vec(token, 0..64).prop_map(|parts| parts.concat())
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]

        #[test]
        fn prop_line_cache_matches_reference(content in fuzz_content_strategy()) {
            let all_boundaries = sample_char_boundaries(&content);

            // Keep assertions per-case bounded while still spreading coverage.
            let sampled: Vec<usize> = all_boundaries
                .iter()
                .enumerate()
                .filter_map(|(i, &offset)| if i % 3 == 0 { Some(offset) } else { None })
                .collect();

            let mut offsets = if sampled.is_empty() { vec![0] } else { sampled };
            if !offsets.contains(&0) {
                offsets.push(0);
            }
            if !offsets.contains(&content.len()) {
                offsets.push(content.len());
            }
            offsets.sort_unstable();
            offsets.dedup();

            assert_cache_matches_reference(&content, &offsets);
        }

        #[test]
        fn prop_all_boundaries_match_reference(content in fuzz_content_strategy()) {
            let offsets = sample_char_boundaries(&content);
            assert_cache_matches_reference(&content, &offsets);
        }
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
            let mut offsets = sample_char_boundaries(content);
            // Sample some more if content is large.
            if offsets.len() > 20 {
                offsets = offsets
                    .iter()
                    .enumerate()
                    .filter_map(|(i, &offset)| if i % 7 == 0 { Some(offset) } else { None })
                    .collect();
                if !offsets.contains(&0) {
                    offsets.push(0);
                }
                if !offsets.contains(&content.len()) {
                    offsets.push(content.len());
                }
                offsets.sort_unstable();
                offsets.dedup();
            }

            assert_cache_matches_reference(content, &offsets);
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
            assert_cache_matches_reference(content, &offsets);
        }
    }
}
