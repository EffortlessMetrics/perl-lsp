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

        #[test]
        fn module_path_segment_count_is_at_least_one(path in module_path()) {
            let count = path.split("::").count();
            prop_assert!(count >= 1, "module path must have at least one segment: {path}");
        }

        #[test]
        fn module_path_segment_count_is_at_most_five(path in module_path()) {
            let count = path.split("::").count();
            prop_assert!(count <= 5, "module path must have at most 5 segments: {path}");
        }

        #[test]
        fn module_path_is_valid_perl_use_statement(path in module_path()) {
            // A module path joined with :: should be usable in `use Foo::Bar;`
            let use_stmt = format!("use {};", path);
            prop_assert!(use_stmt.starts_with("use "), "use statement malformed: {use_stmt}");
            prop_assert!(use_stmt.ends_with(';'), "use statement must end with semicolon");
        }

        #[test]
        fn module_path_segments_count_bounded(segs in module_path_segments()) {
            prop_assert!(!segs.is_empty(), "must produce at least one segment");
            prop_assert!(segs.len() <= 5, "must produce at most 5 segments");
        }

        #[test]
        fn module_path_segments_are_ascii(segs in module_path_segments()) {
            for seg in &segs {
                prop_assert!(seg.is_ascii(), "module segments must be ASCII: {seg}");
            }
        }

        #[test]
        fn module_path_segments_match_joined_path(segs in module_path_segments()) {
            // The segments joined with :: must parse back to the same segments
            let joined = segs.join("::");
            let re_split: Vec<&str> = joined.split("::").collect();
            prop_assert_eq!(segs.len(), re_split.len());
            for (orig, recovered) in segs.iter().zip(re_split.iter()) {
                prop_assert_eq!(orig.as_str(), *recovered);
            }
        }

        #[test]
        fn module_path_does_not_start_or_end_with_colon_colon(path in module_path()) {
            prop_assert!(!path.starts_with("::"), "path must not start with '::': {path}");
            prop_assert!(!path.ends_with("::"), "path must not end with '::': {path}");
        }

        #[test]
        fn module_path_contains_no_triple_colons(path in module_path()) {
            prop_assert!(!path.contains(":::"), "path must not contain ':::': {path}");
        }
    }
}
