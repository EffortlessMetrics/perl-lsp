//! Boundary-safe Perl module token replacement helpers.
//!
//! This crate provides a small, focused API used by module-rename workflows.
//! It handles canonical (`Foo::Bar`) and legacy (`Foo'Bar`) separator variants
//! and delegates standalone token scanning to `perl-module-boundary`.

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

use perl_module_boundary::{contains_standalone_module_token, find_standalone_module_token_ranges};

/// Build canonical + legacy module rename pairs.
///
/// The returned vector always includes the canonical `::` pair. It also
/// includes the legacy `'` pair when it differs.
///
/// # Examples
///
/// ```
/// use perl_module_token::module_variant_pairs;
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
pub use perl_module_name::module_variant_pairs;

/// Returns `true` when `line` contains `module_name` as a standalone module
/// token, respecting module boundaries.
#[must_use]
pub fn contains_module_token(line: &str, module_name: &str) -> bool {
    contains_standalone_module_token(line, module_name)
}

/// Replace standalone `from` module token occurrences in `line` with `to`.
///
/// Returns `(rewritten_line, changed)`.
#[must_use]
pub fn replace_module_token(line: &str, from: &str, to: &str) -> (String, bool) {
    if from.is_empty() || line.is_empty() {
        return (line.to_string(), false);
    }

    let mut ranges = find_standalone_module_token_ranges(line, from).peekable();
    if ranges.peek().is_none() {
        return (line.to_string(), false);
    }

    let mut out = String::with_capacity(line.len());
    let mut cursor = 0usize;

    for range in ranges {
        out.push_str(&line[cursor..range.start]);
        out.push_str(to);
        cursor = range.end;
    }

    out.push_str(&line[cursor..]);
    (out, true)
}

#[cfg(test)]
mod tests {
    use super::{contains_module_token, module_variant_pairs, replace_module_token};

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
    fn canonicalizes_legacy_inputs_for_variant_pairs() {
        assert_eq!(
            module_variant_pairs("Foo'Bar", "New'Path"),
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

    #[test]
    fn replaces_only_standalone_module_tokens() {
        let (rewritten, changed) = replace_module_token("use Foo::Bar;", "Foo::Bar", "X::Y");
        assert_eq!(rewritten, "use X::Y;");
        assert!(changed);

        let (rewritten, changed) = replace_module_token("use Foo::Barista;", "Foo::Bar", "X::Y");
        assert_eq!(rewritten, "use Foo::Barista;");
        assert!(!changed);
    }

    #[test]
    fn treats_legacy_separator_as_module_character_boundary() {
        let (rewritten, changed) = replace_module_token("use Foo'Bar'Baz;", "Foo'Bar", "X'Y");
        assert_eq!(rewritten, "use Foo'Bar'Baz;");
        assert!(!changed);
    }

    #[test]
    fn contains_matches_boundary_aware_token_presence() {
        assert!(contains_module_token("use parent 'Foo::Bar';", "Foo::Bar"));
        assert!(!contains_module_token("use Foo::Barista;", "Foo::Bar"));
    }

    // ── Simple module names ──────────────────────────────────────

    #[test]
    fn contains_simple_bare_name_foo() {
        assert!(contains_module_token("use Foo;", "Foo"));
    }

    #[test]
    fn contains_simple_two_segment_foo_bar() {
        assert!(contains_module_token("use Foo::Bar;", "Foo::Bar"));
    }

    #[test]
    fn replace_simple_bare_name_foo() {
        let (out, changed) = replace_module_token("use Foo;", "Foo", "Bar");
        assert!(changed);
        assert_eq!(out, "use Bar;");
    }

    #[test]
    fn replace_simple_two_segment_foo_bar() {
        let (out, changed) = replace_module_token("use Foo::Bar;", "Foo::Bar", "Baz::Qux");
        assert!(changed);
        assert_eq!(out, "use Baz::Qux;");
    }

    #[test]
    fn contains_rejects_simple_name_as_substring() {
        assert!(!contains_module_token("use Foobar;", "Foo"));
    }

    // ── Deeply nested: Foo::Bar::Baz::Qux ───────────────────────

    #[test]
    fn contains_four_segment_deeply_nested() {
        assert!(contains_module_token("use Foo::Bar::Baz::Qux;", "Foo::Bar::Baz::Qux"));
    }

    #[test]
    fn replace_four_segment_deeply_nested() {
        let (out, changed) = replace_module_token(
            "use Foo::Bar::Baz::Qux;",
            "Foo::Bar::Baz::Qux",
            "One::Two::Three::Four",
        );
        assert!(changed);
        assert_eq!(out, "use One::Two::Three::Four;");
    }

    #[test]
    fn contains_rejects_prefix_of_deeply_nested() {
        // "Foo::Bar" is a prefix of "Foo::Bar::Baz::Qux", not standalone
        assert!(!contains_module_token("use Foo::Bar::Baz::Qux;", "Foo::Bar"));
    }

    #[test]
    fn contains_rejects_suffix_of_deeply_nested() {
        // "Baz::Qux" is embedded in the larger token
        assert!(!contains_module_token("use Foo::Bar::Baz::Qux;", "Baz::Qux"));
    }

    #[test]
    fn replace_does_not_modify_prefix_of_deeply_nested() {
        let (out, changed) = replace_module_token("use Foo::Bar::Baz::Qux;", "Foo::Bar", "X::Y");
        assert!(!changed);
        assert_eq!(out, "use Foo::Bar::Baz::Qux;");
    }

    #[test]
    fn contains_deeply_nested_in_method_call() {
        assert!(contains_module_token("Foo::Bar::Baz::Qux->new()", "Foo::Bar::Baz::Qux"));
    }

    #[test]
    fn replace_deeply_nested_in_method_call() {
        let (out, changed) =
            replace_module_token("Foo::Bar::Baz::Qux->new()", "Foo::Bar::Baz::Qux", "A::B::C::D");
        assert!(changed);
        assert_eq!(out, "A::B::C::D->new()");
    }

    #[test]
    fn variant_pairs_deeply_nested_four_segments() {
        let pairs = module_variant_pairs("Foo::Bar::Baz::Qux", "One::Two::Three::Four");
        assert_eq!(pairs.len(), 2);
        assert_eq!(
            pairs[0],
            ("Foo::Bar::Baz::Qux".to_string(), "One::Two::Three::Four".to_string())
        );
        assert_eq!(pairs[1], ("Foo'Bar'Baz'Qux".to_string(), "One'Two'Three'Four".to_string()));
    }

    // ── Module name validation via boundary checks ───────────────

    #[test]
    fn contains_rejects_empty_module_name() {
        assert!(!contains_module_token("use Foo::Bar;", ""));
    }

    #[test]
    fn contains_rejects_empty_line() {
        assert!(!contains_module_token("", "Foo::Bar"));
    }

    #[test]
    fn replace_empty_from_is_noop() {
        let (out, changed) = replace_module_token("use Foo::Bar;", "", "X::Y");
        assert!(!changed);
        assert_eq!(out, "use Foo::Bar;");
    }

    #[test]
    fn replace_empty_line_is_noop() {
        let (out, changed) = replace_module_token("", "Foo::Bar", "X::Y");
        assert!(!changed);
        assert_eq!(out, "");
    }

    #[test]
    fn contains_validates_boundary_before_token() {
        // Digit before module name means it's part of a larger identifier
        assert!(!contains_module_token("use 1Foo::Bar;", "Foo::Bar"));
    }

    #[test]
    fn contains_validates_boundary_after_token() {
        // Letter after module name means it's part of a larger identifier
        assert!(!contains_module_token("use Foo::BarX;", "Foo::Bar"));
    }

    // ── Module name components via rename workflow ────────────────

    #[test]
    fn rename_workflow_four_segment_canonical() {
        let pairs = module_variant_pairs("Foo::Bar::Baz::Qux", "A::B::C::D");
        let line = "use Foo::Bar::Baz::Qux;";

        let mut result = line.to_string();
        for (old, new) in &pairs {
            let (candidate, changed) = replace_module_token(&result, old, new);
            if changed {
                result = candidate;
            }
        }
        assert_eq!(result, "use A::B::C::D;");
    }

    #[test]
    fn rename_workflow_four_segment_legacy() {
        let pairs = module_variant_pairs("Foo::Bar::Baz::Qux", "A::B::C::D");
        let line = "use Foo'Bar'Baz'Qux;";

        let mut result = line.to_string();
        for (old, new) in &pairs {
            let (candidate, changed) = replace_module_token(&result, old, new);
            if changed {
                result = candidate;
            }
        }
        assert_eq!(result, "use A'B'C'D;");
    }

    // ── Edge cases: single word module ───────────────────────────

    #[test]
    fn contains_single_word_module() {
        assert!(contains_module_token("use strict;", "strict"));
        assert!(contains_module_token("use warnings;", "warnings"));
        assert!(contains_module_token("use DBI;", "DBI"));
    }

    #[test]
    fn replace_single_word_module() {
        let (out, changed) = replace_module_token("use strict;", "strict", "warnings");
        assert!(changed);
        assert_eq!(out, "use warnings;");
    }

    #[test]
    fn single_word_not_matched_as_substring() {
        assert!(!contains_module_token("use strictures;", "strict"));
        assert!(!contains_module_token("use astrict;", "strict"));
    }

    #[test]
    fn variant_pairs_single_word_produces_one_pair() {
        let pairs = module_variant_pairs("DBI", "DBIx");
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0], ("DBI".to_string(), "DBIx".to_string()));
    }

    // ── Edge cases: trailing :: in search context ────────────────

    #[test]
    fn contains_does_not_match_when_followed_by_colon_colon() {
        // Foo::Bar is followed by :: making it part of Foo::Bar::Baz
        assert!(!contains_module_token("use Foo::Bar::Baz;", "Foo::Bar"));
    }

    #[test]
    fn replace_does_not_modify_when_followed_by_colon_colon() {
        let (out, changed) = replace_module_token("use Foo::Bar::Baz;", "Foo::Bar", "X::Y");
        assert!(!changed);
        assert_eq!(out, "use Foo::Bar::Baz;");
    }

    // ── CPAN real-world deeply nested modules ────────────────────

    #[test]
    fn contains_cpan_catalyst_plugin() {
        assert!(contains_module_token(
            "use Catalyst::Plugin::Authentication;",
            "Catalyst::Plugin::Authentication"
        ));
    }

    #[test]
    fn replace_cpan_catalyst_plugin() {
        let (out, changed) = replace_module_token(
            "use Catalyst::Plugin::Authentication;",
            "Catalyst::Plugin::Authentication",
            "Catalyst::Plugin::AuthN",
        );
        assert!(changed);
        assert_eq!(out, "use Catalyst::Plugin::AuthN;");
    }

    #[test]
    fn replace_cpan_app_prove_five_segments() {
        let (out, changed) = replace_module_token(
            "use App::Prove::State::Result::Test;",
            "App::Prove::State::Result::Test",
            "TAP::Result::Test",
        );
        assert!(changed);
        assert_eq!(out, "use TAP::Result::Test;");
    }

    #[test]
    fn contains_and_replace_agree_on_deeply_nested() {
        let line = "my $obj = Foo::Bar::Baz::Qux->new;";
        let module = "Foo::Bar::Baz::Qux";
        let present = contains_module_token(line, module);
        let (_, changed) = replace_module_token(line, module, "X");
        assert_eq!(present, changed);
    }

    #[test]
    fn contains_and_replace_agree_on_prefix_of_deeply_nested() {
        let line = "my $obj = Foo::Bar::Baz::Qux->new;";
        let module = "Foo::Bar";
        let present = contains_module_token(line, module);
        let (_, changed) = replace_module_token(line, module, "X");
        assert_eq!(present, changed);
    }

    // ── Underscore-only and special identifier modules ───────────

    #[test]
    fn contains_underscore_prefixed_module() {
        assert!(contains_module_token("use _Private::Module;", "_Private::Module"));
    }

    #[test]
    fn replace_underscore_prefixed_module() {
        let (out, changed) =
            replace_module_token("use _Private::Module;", "_Private::Module", "Public::Module");
        assert!(changed);
        assert_eq!(out, "use Public::Module;");
    }

    #[test]
    fn contains_all_underscore_segments() {
        assert!(contains_module_token("use _::_::_;", "_::_::_"));
    }

    // ── Multiple occurrences with deeply nested ──────────────────

    #[test]
    fn replace_multiple_occurrences_deeply_nested() {
        let line = "use Foo::Bar::Baz; my $x = Foo::Bar::Baz->new;";
        let (out, changed) = replace_module_token(line, "Foo::Bar::Baz", "A::B::C");
        assert!(changed);
        assert_eq!(out, "use A::B::C; my $x = A::B::C->new;");
    }
}
