//! Generators for Perl module paths.
//!
//! Produces syntactically valid module names like `Foo`, `Foo::Bar`, `Foo::Bar::Baz`.

use proptest::prelude::*;

/// Single identifier segment (capitalized PascalCase or `UPPER_CASE`).
fn module_segment() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("Foo".to_string()),
        Just("Bar".to_string()),
        Just("Baz".to_string()),
        Just("HTTP".to_string()),
        Just("IO".to_string()),
        (
            prop::char::range('A', 'Z'),
            prop::collection::vec(
                prop_oneof![prop::char::range('A', 'Z'), prop::char::range('0', '9'), Just('_'),],
                0..=7_usize,
            ),
        )
            .prop_map(|(first, rest)| std::iter::once(first).chain(rest).collect::<String>()),
        (
            prop::char::range('A', 'Z'),
            prop::collection::vec(prop::char::range('a', 'z'), 0..=7_usize),
        )
            .prop_map(|(first, rest)| std::iter::once(first).chain(rest).collect::<String>()),
    ]
}

/// Generate a full module path (1–5 segments joined by `::`).
pub fn module_path() -> impl Strategy<Value = String> {
    prop::collection::vec(module_segment(), 1..=5_usize).prop_map(|segs| segs.join("::"))
}

/// Generate module path segments as a `Vec<String>`.
pub fn module_path_segments() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(module_segment(), 1..=5_usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    proptest! {
        #[test]
        fn module_path_no_empty_segments(path in module_path()) {
            for seg in path.split("::") {
                assert!(!seg.is_empty(), "empty segment in {}", path);
            }
        }

        #[test]
        fn module_path_starts_uppercase(path in module_path()) {
            prop_assert!(!path.is_empty(), "module path must not be empty");
            let first = path.chars().next().unwrap_or_default();
            prop_assert!(first.is_ascii_uppercase(), "first char not uppercase: {}", path);
        }

        #[test]
        fn segments_non_empty(segs in module_path_segments()) {
            prop_assert!(!segs.is_empty());
            for s in &segs {
                prop_assert!(!s.is_empty());
            }
        }

        #[test]
        fn segments_use_module_identifier_charset(segs in module_path_segments()) {
            for seg in &segs {
                let mut chars = seg.chars();
                let Some(first) = chars.next() else {
                    prop_assert!(false, "segment cannot be empty");
                    continue;
                };
                prop_assert!(first.is_ascii_uppercase(), "segment must start uppercase: {seg}");
                for ch in chars {
                    prop_assert!(
                        ch.is_ascii_alphanumeric() || ch == '_',
                        "invalid module segment char '{ch}' in {seg}",
                    );
                }
            }
        }
    }
}
