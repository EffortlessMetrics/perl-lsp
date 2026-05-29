//! Property tests for parser-core text-line helpers.
//!
//! These guard cursor-line boundary calculations and byte-classification helpers
//! with bounded generators so the substrate invariants stay cheap enough for the
//! normal crate test lane.

use perl_parser_core::text_line::{
    is_identifier_byte, is_keyword_boundary, line_bounds_at, skip_ascii_whitespace,
};
use proptest::prelude::*;
use proptest::test_runner::Config as ProptestConfig;

fn text_fragments(max_fragments: usize) -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop_oneof![
            "[A-Za-z0-9_ ]{0,12}".prop_map(|s| s),
            Just("\n".to_string()),
            Just("\r\n".to_string()),
            Just("\t".to_string()),
            Just("".to_string()),
        ],
        0..=max_fragments,
    )
    .prop_map(|fragments| fragments.concat())
}

fn ascii_bytes(max_len: usize) -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(0u8..=127, 0..=max_len)
}

fn naive_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn naive_keyword_boundary(bytes: &[u8], start: usize, len: usize) -> bool {
    let Some(end) = start.checked_add(len) else {
        return false;
    };

    if start > bytes.len() || end > bytes.len() {
        return false;
    }

    let left_is_identifier = start > 0 && naive_identifier_byte(bytes[start - 1]);
    let right_is_identifier = end < bytes.len() && naive_identifier_byte(bytes[end]);

    !left_is_identifier && !right_is_identifier
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 128,
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    #[test]
    fn prop_line_bounds_contain_clamped_cursor(
        text in text_fragments(24),
        cursor in 0usize..512,
    ) {
        let (start, end) = line_bounds_at(&text, cursor);
        let clamped = cursor.min(text.len());

        prop_assert!(start <= clamped, "start {start} should be <= clamped cursor {clamped} for {text:?}");
        prop_assert!(clamped <= end, "clamped cursor {clamped} should be <= end {end} for {text:?}");
        prop_assert!(end <= text.len(), "end {end} should be within len {} for {text:?}", text.len());
    }

    #[test]
    fn prop_line_bounds_stop_at_newline_boundaries(
        text in text_fragments(24),
        cursor in 0usize..512,
    ) {
        let (start, end) = line_bounds_at(&text, cursor);
        let bytes = text.as_bytes();

        prop_assert!(start == 0 || bytes[start - 1] == b'\n', "line start {start} was not after newline in {text:?}");
        prop_assert!(end == text.len() || bytes[end] == b'\n', "line end {end} was not at newline in {text:?}");
        prop_assert!(!bytes[start..end].contains(&b'\n'), "line slice {start}..{end} contained newline in {text:?}");
    }

    #[test]
    fn prop_line_bounds_are_idempotent_within_line(
        text in text_fragments(24),
        cursor in 0usize..512,
    ) {
        let (start, end) = line_bounds_at(&text, cursor);
        for inner_cursor in start..=end {
            let inner_bounds = line_bounds_at(&text, inner_cursor);
            prop_assert_eq!(
                inner_bounds,
                (start, end),
                "line bounds changed inside same line at {} for {:?}",
                inner_cursor,
                text,
            );
        }
    }

    #[test]
    fn prop_identifier_byte_matches_ascii_definition(byte in any::<u8>()) {
        prop_assert_eq!(is_identifier_byte(byte), naive_identifier_byte(byte));
    }

    #[test]
    fn prop_keyword_boundary_matches_naive_ascii_model(
        bytes in ascii_bytes(96),
        start in 0usize..128,
        len in 0usize..128,
    ) {
        prop_assert_eq!(
            is_keyword_boundary(&bytes, start, len),
            naive_keyword_boundary(&bytes, start, len),
            "boundary mismatch for bytes={:?}, start={}, len={}",
            bytes,
            start,
            len,
        );
    }

    #[test]
    fn prop_skip_ascii_whitespace_stops_at_first_non_whitespace(
        bytes in ascii_bytes(128),
        start in 0usize..160,
    ) {
        let skipped = skip_ascii_whitespace(&bytes, start);

        if start >= bytes.len() {
            prop_assert_eq!(skipped, start);
            return Ok(());
        }

        prop_assert!(skipped >= start);
        prop_assert!(skipped <= bytes.len());

        for byte in &bytes[start..skipped] {
            prop_assert!(byte.is_ascii_whitespace(), "skipped non-whitespace byte {byte}");
        }

        if skipped < bytes.len() {
            prop_assert!(
                !bytes[skipped].is_ascii_whitespace(),
                "stopped on whitespace byte {} at index {skipped}",
                bytes[skipped],
            );
        }
    }
}
