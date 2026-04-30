use perl_position_tracking::LineStartsCache;
use proptest::prelude::*;
use ropey::Rope;

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
        0..128,
    )
    .prop_map(|parts| parts.concat())
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
    fn prop_text_and_rope_offsets_agree(content in mixed_content_strategy(), offset in 0usize..512usize) {
        let rope = Rope::from_str(&content);
        let cache_text = LineStartsCache::new(&content);
        let cache_rope = LineStartsCache::new_rope(&rope);

        let bounded = offset.min(content.len());
        let text_pos = cache_text.offset_to_position(&content, bounded);
        let rope_pos = cache_rope.offset_to_position_rope(&rope, bounded);

        prop_assert_eq!(text_pos, rope_pos, "offset-to-position mismatch at {}", bounded);
    }

    #[test]
    fn prop_rope_position_lookup_is_bounded(content in mixed_content_strategy(), line in 0u32..128, col in 0u32..2048) {
        let rope = Rope::from_str(&content);
        let cache = LineStartsCache::new_rope(&rope);

        let offset = cache.position_to_offset_rope(&rope, line, col);
        prop_assert!(offset <= content.len());
        prop_assert!(content.is_char_boundary(offset));
    }

    #[test]
    fn prop_rope_roundtrip_on_char_boundaries(content in mixed_content_strategy()) {
        let rope = Rope::from_str(&content);
        let cache = LineStartsCache::new_rope(&rope);

        for offset in char_boundary_offsets(&content) {
            let (line, col) = cache.offset_to_position_rope(&rope, offset);
            let roundtrip_offset = cache.position_to_offset_rope(&rope, line, col);
            prop_assert_eq!(roundtrip_offset, offset, "roundtrip mismatch at offset {}", offset);
        }
    }
}
