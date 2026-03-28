//! Generators for Unicode strings exercising various edge cases.
//!
//! Produces strings with BMP characters, supplementary planes, surrogate
//! boundaries, combining marks, and RTL scripts — useful for testing
//! UTF-8/UTF-16 position mappers and LSP protocol encoding.

use proptest::prelude::*;

/// Generate a Unicode string suitable for testing UTF-8/UTF-16 conversion.
///
/// Mixes ASCII, BMP non-ASCII, supplementary plane characters, and
/// combining marks.
pub fn unicode_string() -> impl Strategy<Value = String> {
    prop_oneof![
        // Pure ASCII
        prop::string::string_regex("[a-zA-Z0-9 \\t]*").unwrap(),
        // BMP with non-ASCII (Latin-1 supplement, CJK, etc.)
        prop::collection::vec(prop::char::range('\u{00C0}', '\u{FFFF}'), 0..=50_usize)
            .prop_map(|chars| chars.into_iter().collect()),
        // Supplementary plane characters (emoji, rare scripts)
        prop::collection::vec(prop::char::range('\u{10000}', '\u{10FFFF}'), 0..=30_usize)
            .prop_map(|chars| chars.into_iter().collect()),
        // Mix of all ranges
        prop::collection::vec(
            prop_oneof![
                prop::char::range('a', 'z'),
                prop::char::range('A', 'Z'),
                prop::char::range('0', '9'),
                prop::char::range('\u{00C0}', '\u{024F}'),
                prop::char::range('\u{4E00}', '\u{9FFF}'),
                prop::char::range('\u{10000}', '\u{10FFFF}'),
                Just('\t'),
                Just('\n'),
            ],
            0..=100_usize,
        )
        .prop_map(|chars| chars.into_iter().collect()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    proptest! {
        #[test]
        fn unicode_string_is_valid_utf8(s in unicode_string()) {
            // If it compiled as String, it's valid UTF-8. Verify round-trip.
            let bytes = s.as_bytes();
            let roundtrip = std::str::from_utf8(bytes).unwrap();
            prop_assert_eq!(s.as_str(), roundtrip);
        }

        #[test]
        fn utf16_len_agrees_with_encode_utf16(s in unicode_string()) {
            let encoded: Vec<u16> = s.encode_utf16().collect();
            let from_utf16 = String::from_utf16_lossy(&encoded);
            prop_assert_eq!(s, from_utf16);
        }
    }
}
