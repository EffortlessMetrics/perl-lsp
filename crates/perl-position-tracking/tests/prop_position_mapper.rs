//! Property tests for [`PositionMapper`].
//!
//! `PositionMapper` is the rope-backed primitive that every LSP
//! `textDocument/didChange` flows through. The existing
//! `line_starts_cache_fuzz.rs` proves the *cache* layer is sound on rope
//! offsets, but the higher-level mapper (with incremental edits, LSP
//! position conversion, and line-ending detection) had no property
//! coverage prior to this file.

use perl_position_tracking::{LineEnding, PositionMapper, WirePosition};
use perl_test_generators::unicode_string;
use proptest::prelude::*;

fn mixed_line_endings() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop_oneof![
            Just("a".to_string()),
            Just("Z".to_string()),
            Just(" ".to_string()),
            Just("\n".to_string()),
            Just("\r".to_string()),
            Just("\r\n".to_string()),
            Just("\u{00e9}".to_string()),
            Just("\u{4f60}".to_string()),
            Just("\u{1d400}".to_string()),
        ],
        0..96,
    )
    .prop_map(|parts| parts.concat())
}

fn char_boundary_offsets(s: &str) -> Vec<usize> {
    let mut offsets: Vec<usize> = s.char_indices().map(|(i, _)| i).collect();
    offsets.push(s.len());
    offsets
}

proptest! {
    #![proptest_config(ProptestConfig {
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    /// Round-trip: every byte offset on a char boundary maps to an LSP
    /// position whose inverse lands back on the same byte. This is the
    /// fundamental contract for `textDocument/definition`, `hover`, and
    /// every other request that travels byte -> LSP -> byte.
    #[test]
    fn prop_byte_lsp_byte_roundtrip_on_char_boundaries(s in unicode_string()) {
        let mapper = PositionMapper::new(&s);
        for offset in char_boundary_offsets(&s) {
            let pos = mapper.byte_to_lsp_pos(offset);
            let back = mapper.lsp_pos_to_byte(pos);
            prop_assert_eq!(
                back,
                Some(offset),
                "byte->LSP->byte mismatch at offset {} (pos={:?}) for {:?}",
                offset,
                pos,
                s
            );
        }
    }

    /// `lsp_pos_to_byte` must never panic and must return offsets that are
    /// either `None` (line out of range) or in-bounds and on a UTF-8 char
    /// boundary.
    #[test]
    fn prop_lsp_pos_to_byte_is_safe(
        s in mixed_line_endings(),
        line in 0u32..256,
        character in 0u32..256,
    ) {
        let mapper = PositionMapper::new(&s);
        if let Some(offset) = mapper.lsp_pos_to_byte(WirePosition { line, character }) {
            prop_assert!(offset <= s.len(), "offset {offset} > len {}", s.len());
            prop_assert!(
                s.is_char_boundary(offset),
                "offset {offset} is not on a UTF-8 boundary in {s:?}"
            );
        }
    }

    /// `byte_to_lsp_pos` clamps any out-of-bounds offset to the end of the
    /// document and never panics.
    #[test]
    fn prop_byte_to_lsp_pos_clamps_safely(
        s in mixed_line_endings(),
        offset in 0usize..1024,
    ) {
        let mapper = PositionMapper::new(&s);
        let pos = mapper.byte_to_lsp_pos(offset);
        // Position must reference a valid line.
        let line_count = mapper.len_lines();
        prop_assert!(
            (pos.line as usize) < line_count.max(1),
            "line {} out of range for len_lines() = {}",
            pos.line,
            line_count
        );
    }

    /// `apply_edit` is equivalent to splicing the replacement into the
    /// underlying text by byte range: the same operation a non-rope-backed
    /// implementation would perform.
    #[test]
    fn prop_apply_edit_matches_byte_splice(
        prefix in "[a-z ]{0,20}",
        middle in "[a-z ]{0,20}",
        suffix in "[a-z ]{0,20}",
        replacement in "[a-z ]{0,15}",
    ) {
        let original = format!("{prefix}{middle}{suffix}");
        let start = prefix.len();
        let end = prefix.len() + middle.len();

        let mut mapper = PositionMapper::new(&original);
        mapper.apply_edit(start, end, &replacement);

        let expected = format!("{prefix}{replacement}{suffix}");
        prop_assert_eq!(mapper.text(), expected.clone());
        prop_assert_eq!(mapper.len_bytes(), expected.len());
    }

    /// `apply_edit` with out-of-range byte indices must clamp rather than
    /// panic: LSP clients have been known to send positions past the end
    /// of the document during rapid edits.
    #[test]
    fn prop_apply_edit_clamps_out_of_range(
        initial in "[a-z]{0,20}",
        start in 0usize..64,
        end in 0usize..64,
        replacement in "[a-z]{0,10}",
    ) {
        let mut mapper = PositionMapper::new(&initial);
        let (lo, hi) = if start <= end { (start, end) } else { (end, start) };
        mapper.apply_edit(lo, hi, &replacement);

        // After the clamped edit the document is still valid UTF-8 ASCII
        // (the inputs are ASCII) and len_bytes matches text().len().
        prop_assert_eq!(mapper.len_bytes(), mapper.text().len());
    }

    /// Line-ending detection on a pure-style document returns that style;
    /// detection on an empty document falls back to `Lf` (the default).
    #[test]
    fn prop_line_ending_detection_pure(parts in 0usize..12) {
        for &(ending, expected) in &[
            ("\n", LineEnding::Lf),
            ("\r\n", LineEnding::CrLf),
            ("\r", LineEnding::Cr),
        ] {
            let text = std::iter::repeat_n("line", parts).collect::<Vec<_>>().join(ending);
            let text = if parts > 0 { format!("{text}{ending}") } else { String::new() };
            let mapper = PositionMapper::new(&text);
            if parts == 0 {
                prop_assert_eq!(mapper.line_ending(), LineEnding::Lf, "empty text should default to Lf");
            } else {
                prop_assert_eq!(
                    mapper.line_ending(),
                    expected,
                    "pure {:?} text misdetected: {:?}",
                    ending,
                    text
                );
            }
        }
    }

    /// `slice(start, end)` is equivalent to slicing the underlying text by
    /// the same byte range when both endpoints are on char boundaries.
    #[test]
    fn prop_slice_matches_str_slice(
        s in unicode_string(),
        a in 0usize..256,
        b in 0usize..256,
    ) {
        let mapper = PositionMapper::new(&s);
        // Snap each endpoint down to the nearest char boundary so that
        // slicing is well-defined.
        let mut start = a.min(s.len());
        while start > 0 && !s.is_char_boundary(start) {
            start -= 1;
        }
        let mut end = b.min(s.len());
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        let (start, end) = if start <= end { (start, end) } else { (end, start) };

        prop_assert_eq!(mapper.slice(start, end), s[start..end].to_string());
    }
}
