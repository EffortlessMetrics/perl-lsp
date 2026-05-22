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
        prop::collection::vec(
            prop_oneof![
                prop::char::range('a', 'z'),
                prop::char::range('A', 'Z'),
                prop::char::range('0', '9'),
                Just(' '),
                Just('\t'),
            ],
            0..=50_usize,
        )
        .prop_map(|chars| chars.into_iter().collect()),
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

/// Generate a non-empty Unicode string.
///
/// Useful when call sites need at least one code point and should avoid
/// separate assumptions/filters in their property tests.
pub fn non_empty_unicode_string() -> impl Strategy<Value = String> {
    unicode_string().prop_filter("string must be non-empty", |value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    proptest! {
        #[test]
        fn unicode_string_is_valid_utf8(s in unicode_string()) {
            // If it compiled as String, it's valid UTF-8. Verify round-trip.
            let bytes = s.as_bytes();
            match std::str::from_utf8(bytes) {
                Ok(roundtrip) => prop_assert_eq!(s.as_str(), roundtrip),
                Err(err) => prop_assert!(false, "generated string was not valid UTF-8: {err}"),
            }
        }

        #[test]
        fn utf16_len_agrees_with_encode_utf16(s in unicode_string()) {
            let encoded: Vec<u16> = s.encode_utf16().collect();
            let from_utf16 = String::from_utf16_lossy(&encoded);
            prop_assert_eq!(s, from_utf16);
        }

        #[test]
        fn non_empty_unicode_string_never_empty(s in non_empty_unicode_string()) {
            prop_assert!(!s.is_empty(), "non-empty strategy produced an empty string");
        }

        #[test]
        fn unicode_string_char_count_bounded(s in unicode_string()) {
            // Each arm produces at most 100 chars
            let count = s.chars().count();
            prop_assert!(count <= 100, "string has more chars than expected: {count}");
        }

        #[test]
        fn unicode_string_chars_are_not_surrogates(s in unicode_string()) {
            // Rust chars are guaranteed to not be surrogate code points;
            // this test documents and enforces that invariant explicitly.
            for ch in s.chars() {
                let cp = ch as u32;
                prop_assert!(
                    !(0xD800..=0xDFFF).contains(&cp),
                    "surrogate code point U+{cp:04X} found in generated string"
                );
            }
        }

        #[test]
        fn non_empty_unicode_string_has_positive_byte_length(s in non_empty_unicode_string()) {
            prop_assert!(!s.is_empty(), "non-empty string must have at least one byte");
        }

        #[test]
        fn unicode_string_utf8_byte_len_is_at_least_char_count(s in unicode_string()) {
            // Every char encodes to at least 1 byte in UTF-8.
            let char_count = s.chars().count();
            let byte_len = s.len();
            prop_assert!(
                byte_len >= char_count,
                "byte length {byte_len} < char count {char_count}"
            );
        }

        #[test]
        fn non_empty_unicode_string_first_char_is_valid(s in non_empty_unicode_string()) {
            let first = s.chars().next();
            prop_assert!(first.is_some(), "non-empty string must have a first char");
        }

        #[test]
        fn unicode_string_all_chars_are_valid_scalar_values(s in unicode_string()) {
            // Every char produced by the strategy must be a valid Unicode scalar value.
            for ch in s.chars() {
                let cp = ch as u32;
                prop_assert!(
                    cp <= 0x10_FFFF,
                    "code point U+{cp:X} exceeds Unicode maximum"
                );
            }
        }

        #[test]
        fn unicode_string_is_not_null_terminated(s in unicode_string()) {
            // The string must not contain embedded NUL bytes (which would break
            // null-terminated C-string assumptions in FFI-adjacent code).
            // Note: Rust Strings *can* contain NUL chars legally, but none of our
            // generation arms produce them, so this is a documentation test.
            // The generator only uses char::range arms that exclude '\0'.
            for ch in s.chars() {
                prop_assert!(ch != '\0', "unexpected NUL char in generated string");
            }
        }
    }
}
