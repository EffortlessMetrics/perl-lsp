//! BDD-style behavior specification tests for `perl-quote`.
//!
//! These scenarios exercise the crate from a consumer perspective and lock
//! parsing behavior for regex, substitution, and transliteration operators.

use perl_quote::{
    SubstitutionError, extract_regex_parts, extract_substitution_parts,
    extract_substitution_parts_strict, extract_transliteration_parts,
};
use perl_tdd_support::must;

#[test]
fn when_parsing_qr_with_paired_delimiters_then_body_and_modifiers_are_extracted() {
    let (pattern, body, modifiers) = extract_regex_parts("qr{foo{bar}baz}ix");

    assert_eq!(pattern, "{foo{bar}baz}");
    assert_eq!(body, "foo{bar}baz");
    assert_eq!(modifiers, "ix");
}

#[test]
fn when_parsing_match_operator_then_prefix_is_removed_but_delimiters_are_preserved() {
    let (pattern, body, modifiers) = extract_regex_parts("m!foo\\!bar!ms");

    assert_eq!(pattern, "!foo\\!bar!");
    assert_eq!(body, "foo\\!bar");
    assert_eq!(modifiers, "ms");
}

#[test]
fn when_lenient_substitution_has_invalid_modifiers_then_invalid_flags_are_filtered() {
    let (pattern, replacement, modifiers) = extract_substitution_parts("s/foo/bar/gizQ");

    assert_eq!(pattern, "foo");
    assert_eq!(replacement, "bar");
    assert_eq!(modifiers, "gi");
}

#[test]
fn when_strict_substitution_has_invalid_modifier_then_error_identifies_first_invalid_flag() {
    let error = extract_substitution_parts_strict("s/foo/bar/gizQ").unwrap_err();

    assert_eq!(error, SubstitutionError::InvalidModifier('z'));
}

#[test]
fn when_strict_substitution_uses_whitespace_and_mixed_paired_delimiters_then_it_still_parses() {
    let (pattern, replacement, modifiers) =
        must(extract_substitution_parts_strict("s [foo(bar)] {baz}g"));

    assert_eq!(pattern, "foo(bar)");
    assert_eq!(replacement, "baz");
    assert_eq!(modifiers, "g");
}

#[test]
fn when_strict_substitution_is_missing_replacement_then_error_is_reported() {
    let error = extract_substitution_parts_strict("s{foo}").unwrap_err();

    assert_eq!(error, SubstitutionError::MissingReplacement);
}

#[test]
fn when_parsing_transliteration_alias_then_search_replacement_and_modifiers_are_extracted() {
    let (search, replacement, modifiers) = extract_transliteration_parts("y(a-z)(A-Z)cdr");

    assert_eq!(search, "a-z");
    assert_eq!(replacement, "A-Z");
    assert_eq!(modifiers, "cdr");
}

#[test]
fn when_transliteration_contains_nested_paired_delimiters_then_balanced_content_is_preserved() {
    let (search, replacement, modifiers) = extract_transliteration_parts("tr{[a{b}c]}{[x{y}z]}s");

    assert_eq!(search, "[a{b}c]");
    assert_eq!(replacement, "[x{y}z]");
    assert_eq!(modifiers, "s");
}
