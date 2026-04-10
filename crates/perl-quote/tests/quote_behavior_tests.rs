//! BDD-style behavior specification tests for `perl-quote`.
//!
//! These tests describe the crate from a caller's perspective using
//! "given/when/then" semantics in the test names.

use perl_quote::{
    SubstitutionError, extract_regex_parts, extract_substitution_parts,
    extract_substitution_parts_strict, extract_transliteration_parts,
};

#[test]
fn given_qr_regex_when_extracting_parts_then_returns_body_and_modifiers() {
    let (pattern, body, modifiers) = extract_regex_parts("qr{foo|bar}ix");

    assert_eq!(pattern, "{foo|bar}");
    assert_eq!(body, "foo|bar");
    assert_eq!(modifiers, "ix");
}

#[test]
fn given_bare_regex_when_body_contains_escaped_delimiter_then_body_preserves_escape() {
    let (_pattern, body, modifiers) = extract_regex_parts(r"/a\/b/g");

    assert_eq!(body, r"a\/b");
    assert_eq!(modifiers, "g");
}

#[test]
fn given_non_paired_substitution_when_extracting_lenient_then_returns_pattern_replacement_and_modifiers()
 {
    let (pattern, replacement, modifiers) = extract_substitution_parts("s/foo/bar/g");

    assert_eq!(pattern, "foo");
    assert_eq!(replacement, "bar");
    assert_eq!(modifiers, "g");
}

#[test]
fn given_substitution_with_invalid_modifiers_when_extracting_lenient_then_invalid_modifiers_are_filtered()
 {
    let (pattern, replacement, modifiers) = extract_substitution_parts("s/foo/bar/gz");

    assert_eq!(pattern, "foo");
    assert_eq!(replacement, "bar");
    assert_eq!(modifiers, "g");
}

#[test]
fn given_substitution_with_invalid_modifiers_when_extracting_strict_then_returns_invalid_modifier_error()
 {
    let result = extract_substitution_parts_strict("s/foo/bar/gz");

    assert_eq!(result, Err(SubstitutionError::InvalidModifier('z')));
}

#[test]
fn given_substitution_without_replacement_when_extracting_strict_then_returns_missing_replacement()
{
    let result = extract_substitution_parts_strict("s/foo");

    assert_eq!(result, Err(SubstitutionError::MissingClosingDelimiter));
}

#[test]
fn given_paired_substitution_with_mixed_delimiters_when_extracting_strict_then_parses_both_sections()
 {
    let result = extract_substitution_parts_strict("s[foo]{bar}ge");

    assert_eq!(result, Ok(("foo".to_string(), "bar".to_string(), "ge".to_string())));
}

#[test]
fn given_unclosed_replacement_with_non_paired_delimiter_when_extracting_strict_then_returns_missing_closing_delimiter()
 {
    let result = extract_substitution_parts_strict("s/foo/bar");

    assert_eq!(result, Err(SubstitutionError::MissingClosingDelimiter));
}

#[test]
fn given_transliteration_operator_when_extracting_parts_then_returns_search_replace_and_modifiers()
{
    let (search, replacement, modifiers) = extract_transliteration_parts("tr/a-z/A-Z/d");

    assert_eq!(search, "a-z");
    assert_eq!(replacement, "A-Z");
    assert_eq!(modifiers, "d");
}

#[test]
fn given_transliteration_alias_when_extracting_parts_then_y_operator_behaves_like_tr() {
    let (search, replacement, modifiers) = extract_transliteration_parts("y{abc}{xyz}sr");

    assert_eq!(search, "abc");
    assert_eq!(replacement, "xyz");
    assert_eq!(modifiers, "sr");
}

#[test]
fn given_paired_regex_with_nested_delimiters_when_extracting_parts_then_nested_body_is_preserved() {
    let (pattern, body, modifiers) = extract_regex_parts("m{outer{inner}}ms");

    assert_eq!(pattern, "{outer{inner}}");
    assert_eq!(body, "outer{inner}");
    assert_eq!(modifiers, "ms");
}

#[test]
fn given_empty_input_when_extracting_regex_then_returns_empty_triplet() {
    let (pattern, body, modifiers) = extract_regex_parts("");

    assert_eq!(pattern, "");
    assert_eq!(body, "");
    assert_eq!(modifiers, "");
}
