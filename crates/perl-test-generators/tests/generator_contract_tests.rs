//! Public generator contract tests for reusable Perl proptest strategies.
//!
//! These integration tests exercise the exported strategies the same way
//! downstream crates consume them, complementing the module-private unit tests.

use perl_test_generators::{
    module_path, module_path_segments, non_empty_unicode_string, unicode_string, variable,
};
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig {
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    #[test]
    fn variable_package_qualifiers_never_have_empty_segments(generated in variable()) {
        let body = &generated[1..];

        prop_assert!(!body.starts_with("::"), "variable has leading package separator: {generated}");
        prop_assert!(!body.ends_with("::"), "variable has trailing package separator: {generated}");
        prop_assert!(!body.contains(":::"), "variable has overlapping package separators: {generated}");

        for segment in body.split("::") {
            prop_assert!(!segment.is_empty(), "variable has empty package segment: {generated}");
        }
    }

    #[test]
    fn numeric_special_variables_are_scalar_digits(generated in variable()) {
        let body = &generated[1..];
        if body.chars().all(|ch| ch.is_ascii_digit()) {
            prop_assert!(generated.starts_with('$'), "numeric special variable must be scalar: {generated}");
            prop_assert_eq!(body.len(), 1, "numeric special variable should be one digit: {}", generated);
        }
    }

    #[test]
    fn module_paths_have_bounded_valid_segments(path in module_path()) {
        let segments: Vec<&str> = path.split("::").collect();
        prop_assert!((1..=5).contains(&segments.len()), "module path segment count out of range: {path}");

        for segment in segments {
            let mut chars = segment.chars();
            let Some(first) = chars.next() else {
                prop_assert!(false, "module path has empty segment: {path}");
                continue;
            };
            prop_assert!(first.is_ascii_uppercase(), "module segment must start uppercase: {segment}");
            prop_assert!(segment.len() <= 8, "module segment exceeds documented generator bound: {segment}");
            for ch in chars {
                prop_assert!(ch.is_ascii_alphanumeric() || ch == '_', "invalid char '{ch}' in module segment {segment}");
            }
        }
    }

    #[test]
    fn module_path_segments_can_round_trip_through_canonical_separator(segments in module_path_segments()) {
        prop_assert!((1..=5).contains(&segments.len()), "segment vector length out of range: {segments:?}");
        let joined = segments.join("::");
        prop_assert!(!joined.contains("::::"), "joined module path has an empty segment: {joined}");
        prop_assert_eq!(joined.split("::").count(), segments.len());
    }

    #[test]
    fn unicode_strings_round_trip_through_utf16(text in unicode_string()) {
        let encoded: Vec<u16> = text.encode_utf16().collect();
        let decoded = String::from_utf16_lossy(&encoded);
        prop_assert_eq!(decoded, text);
    }

    #[test]
    fn non_empty_unicode_strings_have_at_least_one_scalar_value(text in non_empty_unicode_string()) {
        prop_assert!(!text.is_empty());
        prop_assert!(text.chars().next().is_some(), "non-empty string should contain a Unicode scalar value");
    }
}
