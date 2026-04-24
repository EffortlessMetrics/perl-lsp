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
fn transliteration_rejects_invalid_delimiter_forms() {
    assert_has_error(r#"$x =~ tr\a-z\A-Z\;"#, "Missing delimiter after transliteration operator");
}

#[test]
fn transliteration_handles_mixed_replacement_delimiters_and_empty_bodies() {
    assert_clean_parse(r#"$x =~ tr{abc}/xyz/r;"#);
    assert_clean_parse(r#"$x =~ tr/🦀π/🐪λ/cdsr;"#);
    assert_has_error(r#"$x =~ tr///;"#, "Missing search list in transliteration");
}

#[test]
fn transliteration_rejects_malformed_closures() {
    assert_has_error(r#"$x =~ tr/abc/xyz;"#, "Missing closing delimiter in transliteration");
    assert_has_error(r#"$x =~ tr{abc}{xyz;"#, "Missing closing delimiter in transliteration");
}
