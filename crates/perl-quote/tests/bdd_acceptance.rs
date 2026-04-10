//! BDD-style acceptance coverage for the public `perl-quote` API.
//!
//! These scenarios are written from a consumer perspective using
//! Given/When/Then descriptions to capture expected behavior.

use perl_quote::{
    SubstitutionError, extract_regex_parts, extract_substitution_parts,
    extract_substitution_parts_strict, extract_transliteration_parts,
};
use perl_tdd_support::{must, must_err};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn scenario_regex_operator_with_nested_delimiters() -> TestResult {
    // Given a regex token that uses paired delimiters with nested content
    let input = "qr{foo{bar}baz}ix";

    // When we parse the regex parts
    let (pattern, body, modifiers) = extract_regex_parts(input);

    // Then the full pattern, body, and modifiers are preserved
    assert_eq!(pattern, "{foo{bar}baz}");
    assert_eq!(body, "foo{bar}baz");
    assert_eq!(modifiers, "ix");

    Ok(())
}

#[test]
fn scenario_lenient_substitution_filters_unknown_modifiers() -> TestResult {
    // Given a substitution with unsupported modifiers
    let input = "s/foo/bar/giz";

    // When the lenient parser extracts substitution parts
    let (pattern, replacement, modifiers) = extract_substitution_parts(input);

    // Then invalid modifiers are ignored instead of failing
    assert_eq!(pattern, "foo");
    assert_eq!(replacement, "bar");
    assert_eq!(modifiers, "gi");

    Ok(())
}

#[test]
fn scenario_strict_substitution_rejects_unknown_modifier() -> TestResult {
    // Given the same substitution token with an invalid modifier
    let input = "s/foo/bar/giz";

    // When strict parsing is used
    let err = must_err(extract_substitution_parts_strict(input));

    // Then the invalid modifier is reported explicitly
    assert_eq!(err, SubstitutionError::InvalidModifier('z'));

    Ok(())
}

#[test]
fn scenario_substitution_replacement_can_switch_paired_delimiter() -> TestResult {
    // Given a substitution where replacement switches paired delimiter style
    let input = "s[foo]{bar}g";

    // When both parsers consume it
    let lenient = extract_substitution_parts(input);
    let strict = must(extract_substitution_parts_strict(input));

    // Then both agree on the semantic parts
    assert_eq!(lenient, strict);
    assert_eq!(strict, ("foo".to_string(), "bar".to_string(), "g".to_string()));

    Ok(())
}

#[test]
fn scenario_strict_substitution_allows_whitespace_after_s() -> TestResult {
    // Given a substitution that includes Perl-style whitespace after `s`
    let input = "s {alpha} {beta}gr";

    // When strict parsing is used
    let (pattern, replacement, modifiers) = must(extract_substitution_parts_strict(input));

    // Then whitespace is tolerated and the token is still parsed correctly
    assert_eq!(pattern, "alpha");
    assert_eq!(replacement, "beta");
    assert_eq!(modifiers, "gr");

    Ok(())
}

#[test]
fn scenario_strict_substitution_reports_missing_closing_delimiter() -> TestResult {
    // Given a substitution where the replacement closing delimiter is missing
    let input = "s/foo";

    // When strict parsing is attempted
    let err = must_err(extract_substitution_parts_strict(input));

    // Then the failure mode identifies the missing closing delimiter
    assert_eq!(err, SubstitutionError::MissingClosingDelimiter);

    Ok(())
}

#[test]
fn scenario_transliteration_alias_y_behaves_like_tr() -> TestResult {
    // Given two transliteration forms that should be equivalent
    let tr = "tr/a-z/A-Z/c";
    let y = "y/a-z/A-Z/c";

    // When we parse both forms
    let tr_parts = extract_transliteration_parts(tr);
    let y_parts = extract_transliteration_parts(y);

    // Then their extracted pieces are identical
    assert_eq!(tr_parts, y_parts);
    assert_eq!(tr_parts.0, "a-z");
    assert_eq!(tr_parts.1, "A-Z");
    assert_eq!(tr_parts.2, "c");

    Ok(())
}
