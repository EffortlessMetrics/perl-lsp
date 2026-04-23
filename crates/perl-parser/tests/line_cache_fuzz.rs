#[cfg(test)]
mod fuzz {
    use perl_parser::position::LineStartsCache;
    use proptest::prelude::*;

    fn mixed_content_strategy() -> impl Strategy<Value = String> {
        prop::collection::vec(
            prop_oneof![
                Just("a".to_string()),
                Just("Z".to_string()),
                Just("0".to_string()),
                Just(" ".to_string()),
                Just("\t".to_string()),
                Just("\n".to_string()),
                Just("\r".to_string()),
                Just("\r\n".to_string()),
                Just("\u{FEFF}".to_string()),
                Just("é".to_string()),
                Just("你".to_string()),
                Just("𝐀".to_string()),
                Just("👨\u{200D}👩\u{200D}👧\u{200D}👦".to_string()),
            ],
            0..96,
        )
        .prop_map(|parts| parts.concat())
    }

    /// Slow reference implementation for testing.
    /// Matches cache behavior where `\r` contributes one UTF-16 column in CRLF.
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
                if byte_offset + 1 < bytes.len() && bytes[byte_offset + 1] == b'\n' {
                    col_utf16 += 1;
                } else {
                    line += 1;
                    col_utf16 = 0;
                }
            } else {
                col_utf16 += ch.len_utf16() as u32;
            }

            byte_offset += ch.len_utf8();
        }

        (line, col_utf16)
    }

    fn char_boundary_offsets(content: &str) -> Vec<usize> {
        let mut offsets = Vec::with_capacity(content.chars().count() + 2);
        offsets.push(0);
        for (i, _) in content.char_indices() {
            offsets.push(i);
        }
        offsets.push(content.len());
        offsets.sort_unstable();
        offsets.dedup();
        offsets
    }

    proptest! {
        #[test]
        fn prop_cache_matches_reference(content in mixed_content_strategy()) {
            let cache = LineStartsCache::new(&content);

            for offset in char_boundary_offsets(&content) {
                let cached = cache.offset_to_position(&content, offset);
                let slow = slow_offset_to_position(&content, offset);
                prop_assert_eq!(cached, slow, "offset mismatch at {}", offset);
            }
        }

        #[test]
        fn prop_round_trip_for_non_crlf_byte_offsets(content in mixed_content_strategy()) {
            let cache = LineStartsCache::new(&content);
            let bytes = content.as_bytes();

            for offset in char_boundary_offsets(&content) {
                let is_crlf_r = bytes.get(offset) == Some(&b'\r') && bytes.get(offset + 1) == Some(&b'\n');
                let is_crlf_n = offset > 0
                    && bytes.get(offset - 1) == Some(&b'\r')
                    && bytes.get(offset) == Some(&b'\n');

                if !is_crlf_r && !is_crlf_n {
                    let (line, col) = cache.offset_to_position(&content, offset);
                    let rt_offset = cache.position_to_offset(&content, line, col);
                    prop_assert_eq!(rt_offset, offset, "round-trip mismatch at {}", offset);
                }
            }
        }

        #[test]
        fn prop_position_lookup_is_bounded(content in mixed_content_strategy(), line in 0u32..128, col in 0u32..2048) {
            let cache = LineStartsCache::new(&content);
            let offset = cache.position_to_offset(&content, line, col);

            prop_assert!(offset <= content.len());
            prop_assert!(content.is_char_boundary(offset));
        }
    }
}
