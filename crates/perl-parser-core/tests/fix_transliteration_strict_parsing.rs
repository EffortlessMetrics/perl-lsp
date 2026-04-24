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
    assert_clean_parse(r#"$x =~ y /αβ/γδ/c;"#);
    assert_clean_parse(r#"$x =~ tr///;"#);
}

#[test]
fn transliteration_rejects_invalid_modifiers() {
    assert_has_error(r#"$x =~ tr/a-z/A-Z/z;"#, "invalid transliteration modifier");
    assert_has_error(r#"$x =~ y/a-z/A-Z/1;"#, "invalid transliteration modifier");
}

#[test]
fn transliteration_accepts_all_valid_modifiers() {
    assert_clean_parse(r#"$x =~ tr/a-z/A-Z/c;"#);
    assert_clean_parse(r#"$x =~ tr/a-z/A-Z/d;"#);
    assert_clean_parse(r#"$x =~ tr/a-z/A-Z/s;"#);
    assert_clean_parse(r#"$x =~ tr/a-z/A-Z/r;"#);
}

#[test]
fn transliteration_reports_malformed_delimiter_closures() {
    assert_has_error(r#"$x =~ tr/abc/xyz;"#, "missing closing delimiter in transliteration");
    assert_has_error(r#"$x =~ tr{abc}{xyz;"#, "missing closing delimiter in transliteration");
}
