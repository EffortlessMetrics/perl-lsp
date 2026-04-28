//! Property-based tests for UTF-8 ↔ UTF-16 position roundtrip.
//!
//! Verifies that `offset_to_utf16_line_col` and `utf16_line_col_to_offset`
//! form a stable roundtrip for arbitrary Unicode strings.

use perl_position_tracking::{offset_to_utf16_line_col, utf16_line_col_to_offset};
use perl_test_generators::unicode_string;
use proptest::prelude::*;

proptest! {
    /// For any string and any byte offset within it, converting to
    /// UTF-16 line/col and back yields the same offset (or the nearest
    /// valid position).
    #[test]
    fn roundtrip_offset_to_utf16_and_back(
        s in unicode_string(),
        offset in 0usize..200usize,
    ) {
        let offset = offset.min(s.len());
        let (line, col) = offset_to_utf16_line_col(&s, offset);
        let back = utf16_line_col_to_offset(&s, line, col);
        // The roundtrip must land on the same line (may differ by ≤ 1 col
        // due to multi-byte char boundary snapping).
        let (line2, _col2) = offset_to_utf16_line_col(&s, back);
        prop_assert_eq!(line, line2, "line mismatch: {} → ({},{}) → {} → ({},{})", offset, line, col, back, line2, _col2);
    }

    /// Every UTF-8 character boundary should roundtrip exactly.
    #[test]
    fn char_boundary_roundtrip_is_exact(s in unicode_string()) {
        let mut boundaries: Vec<usize> = s.char_indices().map(|(idx, _)| idx).collect();
        boundaries.push(s.len());

        for offset in boundaries {
            let (line, col) = offset_to_utf16_line_col(&s, offset);
            let back = utf16_line_col_to_offset(&s, line, col);
            prop_assert_eq!(
                back,
                offset,
                "char-boundary roundtrip mismatch: {} → ({},{}) → {} for {:?}",
                offset,
                line,
                col,
                back,
                s
            );
        }
    }

    /// For any string, every valid UTF-16 column on every line maps
    /// to a byte offset within bounds.
    #[test]
    fn utf16_col_stays_in_bounds(
        s in unicode_string(),
    ) {
        for (line, line_text) in s.split_inclusive('\n').enumerate() {
            let line = line as u32;
            let utf16_len = line_text.trim_end_matches('\n').encode_utf16().count() as u32;
            // Test col = 0, midpoint, and end
            for col in [0, utf16_len / 2, utf16_len, utf16_len + 1] {
                let offset = utf16_line_col_to_offset(&s, line, col);
                prop_assert!(offset <= s.len(), "offset {} out of bounds for len {} (line={}, col={})", offset, s.len(), line, col);
            }
        }
    }

    /// Offsets must be monotonic as UTF-16 columns increase within a line.
    #[test]
    fn utf16_columns_map_to_monotonic_offsets(s in unicode_string()) {
        let mut line_start = 0usize;

        for (line, line_text) in s.split_inclusive('\n').enumerate() {
            let line = line as u32;
            let line_content = line_text.trim_end_matches('\n');
            let utf16_len = line_content.encode_utf16().count() as u32;
            let line_end = line_start + line_content.len();

            let mut previous = utf16_line_col_to_offset(&s, line, 0);
            for col in 1..=(utf16_len + 2) {
                let current = utf16_line_col_to_offset(&s, line, col);
                prop_assert!(
                    current >= previous,
                    "non-monotonic offset mapping on line {} at col {}: {} -> {}",
                    line,
                    col,
                    previous,
                    current
                );
                prop_assert!(
                    current >= line_start && current <= line_end,
                    "offset {} escaped line bounds [{}, {}] on line {} col {}",
                    current,
                    line_start,
                    line_end,
                    line,
                    col
                );
                previous = current;
            }

            line_start += line_text.len();
        }
    }

    /// Empty string edge case.
    #[test]
    fn empty_string(s in Just(String::new())) {
        let (line, col) = offset_to_utf16_line_col(&s, 0);
        prop_assert_eq!((line, col), (0, 0));
        let back = utf16_line_col_to_offset(&s, line, col);
        prop_assert_eq!(back, 0);
    }
}
