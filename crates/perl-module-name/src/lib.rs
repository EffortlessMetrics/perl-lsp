//! Perl module-name separator normalization and variant helpers.
//!
//! This crate has a single responsibility: normalize and project Perl module
//! names across canonical (`::`) and legacy (`'`) package separator forms.

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

use std::borrow::Cow;

/// Normalize legacy package separator `'` to canonical `::`.
///
/// # Examples
///
/// ```
/// use perl_module_name::normalize_package_separator;
///
/// assert_eq!(normalize_package_separator("Foo'Bar"), "Foo::Bar");
/// assert_eq!(normalize_package_separator("Foo::Bar"), "Foo::Bar");
/// ```
#[must_use]
pub fn normalize_package_separator(module_name: &str) -> Cow<'_, str> {
    if module_name.contains('\'') {
        Cow::Owned(module_name.replace('\'', "::"))
    } else {
        Cow::Borrowed(module_name)
    }
}

/// Project canonical package separator `::` to legacy `'`.
///
/// # Examples
///
/// ```
/// use perl_module_name::legacy_package_separator;
///
/// assert_eq!(legacy_package_separator("Foo::Bar"), "Foo'Bar");
/// assert_eq!(legacy_package_separator("Foo'Bar"), "Foo'Bar");
/// ```
#[must_use]
pub fn legacy_package_separator(module_name: &str) -> Cow<'_, str> {
    if module_name.contains("::") {
        Cow::Owned(module_name.replace("::", "'"))
    } else {
        Cow::Borrowed(module_name)
    }
}

/// Build canonical + legacy module rename pairs.
///
/// The returned vector always includes the canonical `::` pair. It also
/// includes the legacy `'` pair when it differs.
///
/// # Examples
///
/// ```
/// use perl_module_name::module_variant_pairs;
///
/// let variants = module_variant_pairs("Foo::Bar", "New::Path");
/// assert_eq!(
///     variants,
///     vec![
///         ("Foo::Bar".to_string(), "New::Path".to_string()),
///         ("Foo'Bar".to_string(), "New'Path".to_string()),
///     ]
/// );
/// ```
#[must_use]
pub fn module_variant_pairs(old_module: &str, new_module: &str) -> Vec<(String, String)> {
    let canonical_old = normalize_package_separator(old_module).into_owned();
    let canonical_new = normalize_package_separator(new_module).into_owned();

    let canonical = (canonical_old.clone(), canonical_new.clone());
    let legacy = (
        legacy_package_separator(&canonical_old).into_owned(),
        legacy_package_separator(&canonical_new).into_owned(),
    );

    if legacy == canonical { vec![canonical] } else { vec![canonical, legacy] }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::{legacy_package_separator, module_variant_pairs, normalize_package_separator};

    #[test]
    fn normalizes_legacy_separator() {
        assert_eq!(normalize_package_separator("Foo'Bar"), "Foo::Bar");
        assert_eq!(normalize_package_separator("Foo::Bar"), "Foo::Bar");
    }

    #[test]
    fn projects_canonical_separator_to_legacy() {
        assert_eq!(legacy_package_separator("Foo::Bar"), "Foo'Bar");
        assert_eq!(legacy_package_separator("Foo'Bar"), "Foo'Bar");
    }

    #[test]
    fn builds_canonical_and_legacy_variant_pairs() {
        assert_eq!(
            module_variant_pairs("Foo::Bar", "New::Path"),
            vec![
                ("Foo::Bar".to_string(), "New::Path".to_string()),
                ("Foo'Bar".to_string(), "New'Path".to_string()),
            ]
        );
    }

    #[test]
    fn deduplicates_pair_when_no_separator_variants_exist() {
        assert_eq!(module_variant_pairs("strict", "warnings").len(), 1);
    }

    // ── Simple module names ──────────────────────────────────────

    #[test]
    fn normalize_simple_bare_name_foo() {
        let result = normalize_package_separator("Foo");
        assert_eq!(result, "Foo");
        assert!(matches!(result, Cow::Borrowed(_)));
    }

    #[test]
    fn normalize_simple_two_segment_foo_bar() {
        assert_eq!(normalize_package_separator("Foo'Bar"), "Foo::Bar");
    }

    #[test]
    fn legacy_simple_two_segment_foo_bar() {
        assert_eq!(legacy_package_separator("Foo::Bar"), "Foo'Bar");
    }

    #[test]
    fn legacy_simple_bare_name_foo() {
        let result = legacy_package_separator("Foo");
        assert_eq!(result, "Foo");
        assert!(matches!(result, Cow::Borrowed(_)));
    }

    // ── Deeply nested: Foo::Bar::Baz::Qux ───────────────────────

    #[test]
    fn normalize_four_segments_deeply_nested() {
        assert_eq!(normalize_package_separator("Foo'Bar'Baz'Qux"), "Foo::Bar::Baz::Qux");
    }

    #[test]
    fn legacy_four_segments_deeply_nested() {
        assert_eq!(legacy_package_separator("Foo::Bar::Baz::Qux"), "Foo'Bar'Baz'Qux");
    }

    #[test]
    fn variant_pairs_four_segments_produces_both_forms() {
        let pairs = module_variant_pairs("Foo::Bar::Baz::Qux", "One::Two::Three::Four");
        assert_eq!(pairs.len(), 2);
        assert_eq!(
            pairs[0],
            ("Foo::Bar::Baz::Qux".to_string(), "One::Two::Three::Four".to_string())
        );
        assert_eq!(pairs[1], ("Foo'Bar'Baz'Qux".to_string(), "One'Two'Three'Four".to_string()));
    }

    #[test]
    fn roundtrip_four_segment_normalize_then_legacy() {
        let input = "Foo'Bar'Baz'Qux";
        let canonical = normalize_package_separator(input);
        assert_eq!(canonical, "Foo::Bar::Baz::Qux");
        let back = legacy_package_separator(&canonical);
        assert_eq!(back, input);
    }

    #[test]
    fn roundtrip_four_segment_legacy_then_normalize() {
        let input = "Foo::Bar::Baz::Qux";
        let legacy = legacy_package_separator(input);
        assert_eq!(legacy, "Foo'Bar'Baz'Qux");
        let back = normalize_package_separator(&legacy);
        assert_eq!(back, input);
    }

    // ── Module name validation behavior ──────────────────────────

    #[test]
    fn normalize_preserves_content_with_no_separators() {
        // Names without separators pass through unchanged
        for name in &["DBI", "strict", "warnings", "utf8", "POSIX", "Carp"] {
            let result = normalize_package_separator(name);
            assert_eq!(result.as_ref(), *name);
            assert!(matches!(result, Cow::Borrowed(_)));
        }
    }

    #[test]
    fn legacy_preserves_content_with_no_separators() {
        for name in &["DBI", "strict", "warnings", "utf8", "POSIX", "Carp"] {
            let result = legacy_package_separator(name);
            assert_eq!(result.as_ref(), *name);
            assert!(matches!(result, Cow::Borrowed(_)));
        }
    }

    #[test]
    fn normalize_handles_only_identifier_chars() {
        // Underscores and digits are valid Perl identifier chars
        assert_eq!(normalize_package_separator("_Private'_Internal"), "_Private::_Internal");
    }

    #[test]
    fn normalize_handles_numeric_only_segments() {
        assert_eq!(normalize_package_separator("V5'V6"), "V5::V6");
    }

    // ── Module name components extraction ────────────────────────
    // These tests verify that separator normalization correctly
    // enables component extraction via split.

    #[test]
    fn components_after_normalize_two_segments() {
        let canonical = normalize_package_separator("Foo'Bar");
        let components: Vec<&str> = canonical.split("::").collect();
        assert_eq!(components, vec!["Foo", "Bar"]);
    }

    #[test]
    fn components_after_normalize_four_segments() {
        let canonical = normalize_package_separator("Foo'Bar'Baz'Qux");
        let components: Vec<&str> = canonical.split("::").collect();
        assert_eq!(components, vec!["Foo", "Bar", "Baz", "Qux"]);
    }

    #[test]
    fn components_after_normalize_single_segment() {
        let canonical = normalize_package_separator("strict");
        let components: Vec<&str> = canonical.split("::").collect();
        assert_eq!(components, vec!["strict"]);
    }

    #[test]
    fn components_after_normalize_already_canonical() {
        let canonical = normalize_package_separator("File::Spec::Functions");
        let components: Vec<&str> = canonical.split("::").collect();
        assert_eq!(components, vec!["File", "Spec", "Functions"]);
    }

    #[test]
    fn components_after_normalize_mixed_separators() {
        let canonical = normalize_package_separator("A'B::C'D");
        let components: Vec<&str> = canonical.split("::").collect();
        assert_eq!(components, vec!["A", "B", "C", "D"]);
    }

    // ── Module name to file path conversion ──────────────────────
    // These tests verify the separator-to-path relationship:
    // Foo::Bar::Baz -> Foo/Bar/Baz.pm

    #[test]
    fn normalized_name_converts_to_file_path_two_segments() {
        let canonical = normalize_package_separator("Foo'Bar");
        let path = canonical.replace("::", "/") + ".pm";
        assert_eq!(path, "Foo/Bar.pm");
    }

    #[test]
    fn normalized_name_converts_to_file_path_four_segments() {
        let canonical = normalize_package_separator("Foo'Bar'Baz'Qux");
        let path = canonical.replace("::", "/") + ".pm";
        assert_eq!(path, "Foo/Bar/Baz/Qux.pm");
    }

    #[test]
    fn normalized_name_converts_to_file_path_single_segment() {
        let canonical = normalize_package_separator("strict");
        let path = canonical.replace("::", "/") + ".pm";
        assert_eq!(path, "strict.pm");
    }

    #[test]
    fn normalized_name_converts_to_file_path_cpan_style() {
        let canonical = normalize_package_separator("File'Spec'Functions");
        let path = canonical.replace("::", "/") + ".pm";
        assert_eq!(path, "File/Spec/Functions.pm");
    }

    #[test]
    fn file_path_roundtrip_through_normalize() {
        // Given a file path, derive module name, then convert back
        let module = "DateTime::Format::Strptime";
        let path = module.replace("::", "/") + ".pm";
        assert_eq!(path, "DateTime/Format/Strptime.pm");

        // Reverse: path components to module name
        let recovered = path.trim_end_matches(".pm").replace('/', "::");
        assert_eq!(recovered, module);
    }

    // ── Edge cases: single word ──────────────────────────────────

    #[test]
    fn single_word_normalize_is_identity() {
        let result = normalize_package_separator("Moose");
        assert_eq!(result, "Moose");
        assert!(matches!(result, Cow::Borrowed(_)));
    }

    #[test]
    fn single_word_legacy_is_identity() {
        let result = legacy_package_separator("Moose");
        assert_eq!(result, "Moose");
        assert!(matches!(result, Cow::Borrowed(_)));
    }

    #[test]
    fn single_word_variant_pairs_produces_one_pair() {
        let pairs = module_variant_pairs("Moose", "Moo");
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0], ("Moose".to_string(), "Moo".to_string()));
    }

    // ── Edge cases: trailing :: ──────────────────────────────────

    #[test]
    fn normalize_trailing_double_colon_passthrough() {
        // Trailing :: has no legacy quote to replace, so borrowed
        let result = normalize_package_separator("Foo::Bar::");
        assert_eq!(result, "Foo::Bar::");
        assert!(matches!(result, Cow::Borrowed(_)));
    }

    #[test]
    fn legacy_trailing_double_colon_converts() {
        assert_eq!(legacy_package_separator("Foo::Bar::"), "Foo'Bar'");
    }

    #[test]
    fn variant_pairs_trailing_separator() {
        let pairs = module_variant_pairs("Foo::Bar::", "Baz::Qux::");
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0], ("Foo::Bar::".to_string(), "Baz::Qux::".to_string()));
        assert_eq!(pairs[1], ("Foo'Bar'".to_string(), "Baz'Qux'".to_string()));
    }

    // ── Edge cases: leading :: ───────────────────────────────────

    #[test]
    fn normalize_leading_double_colon_passthrough() {
        let result = normalize_package_separator("::Foo::Bar");
        assert_eq!(result, "::Foo::Bar");
        assert!(matches!(result, Cow::Borrowed(_)));
    }

    #[test]
    fn legacy_leading_double_colon_converts() {
        assert_eq!(legacy_package_separator("::Foo::Bar"), "'Foo'Bar");
    }

    // ── CPAN real-world deeply nested modules ────────────────────

    #[test]
    fn normalize_cpan_deeply_nested_catalyst() {
        assert_eq!(
            normalize_package_separator("Catalyst'Plugin'Authentication'Store"),
            "Catalyst::Plugin::Authentication::Store"
        );
    }

    #[test]
    fn variant_pairs_cpan_moosex_types_rename() {
        let pairs = module_variant_pairs("MooseX::Types::Structured", "Type::Tiny::Structured");
        assert_eq!(pairs.len(), 2);
        assert_eq!(
            pairs[0],
            ("MooseX::Types::Structured".to_string(), "Type::Tiny::Structured".to_string())
        );
    }

    #[test]
    fn variant_pairs_cpan_deeply_nested_six_segments() {
        let pairs = module_variant_pairs(
            "App::Prove::State::Result::Test",
            "TAP::Harness::Result::Test::V2",
        );
        assert_eq!(pairs.len(), 2);
        // Canonical form
        assert!(!pairs[0].0.contains('\''));
        assert!(!pairs[0].1.contains('\''));
        // Legacy form
        assert!(!pairs[1].0.contains("::"));
        assert!(!pairs[1].1.contains("::"));
    }
}
