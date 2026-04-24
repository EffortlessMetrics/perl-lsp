//! Regression tests for strict transliteration parsing.
//!
//! Ensures `tr///` and `y///` parsing supports optional whitespace before
//! delimiters and rejects invalid modifier characters with diagnostics.

mod cpan_test_helpers;
use cpan_test_helpers::*;

#[test]
fn transliteration_allows_whitespace_before_delimiter() {
    assert_clean_parse(r#"$x =~ tr /a-z/A-Z/;"#);
    assert_clean_parse(r#"$x =~ y  {abc}{xyz}r;"#);
}

#[test]
fn transliteration_rejects_invalid_modifiers() {
    assert_has_error(r#"$x =~ tr/a-z/A-Z/z;"#, "invalid transliteration modifier");
    assert_has_error(r#"$x =~ y/a-z/A-Z/1;"#, "invalid transliteration modifier");
}

#[test]
fn transliteration_accepts_valid_modifier_set() {
    assert_clean_parse(r#"$x =~ tr/a-z/A-Z/c;"#);
    assert_clean_parse(r#"$x =~ tr/a-z/A-Z/d;"#);
    assert_clean_parse(r#"$x =~ tr/a-z/A-Z/s;"#);
    assert_clean_parse(r#"$x =~ tr/a-z/A-Z/r;"#);
}

#[test]
fn transliteration_rejects_invalid_delimiter_and_malformed_closures() {
    assert_has_error(r#"$x =~ tr/a-z/A-Z;"#, "missing closing delimiter");
    assert_has_error(r#"$x =~ tr{a-z}{A-Z;"#, "missing closing delimiter");
}

#[test]
fn transliteration_supports_empty_bodies_without_panicking() {
    assert_has_error(r#"$x =~ tr//A-Z/;"#, "missing search list");
    assert_clean_parse(r#"$x =~ tr/a-z//;"#);
}

#[test]
fn transliteration_angle_bracket_and_paren_delimiters() {
    // All four ASCII bracket pairs are valid delimiters for tr/y
    assert_clean_parse(r#"$x =~ tr<a-z><A-Z>;"#);
    assert_clean_parse(r#"$x =~ tr(a-z)(A-Z)s;"#);
    assert_clean_parse(r#"$x =~ tr[a-z][A-Z]d;"#);
}
