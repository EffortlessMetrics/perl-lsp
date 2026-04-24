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
fn transliteration_strict_handles_escaped_unicode_and_empty_bodies() {
    assert_clean_parse(r#"$x =~ tr /a\/b/c\/d/;"#);
    assert_clean_parse(r#"$x =~ tr/αβγ/ΑΒΓ/r;"#);
    assert_clean_parse(r#"$x =~ tr/a//d;"#);
    assert_has_error(r#"$x =~ tr//x/;"#, "missing search list in transliteration");
}

#[test]
fn transliteration_strict_rejects_malformed_delimiters() {
    assert_has_error(r#"$x =~ tr/a-z/A-Z;"#, "missing closing delimiter in transliteration");
    assert_has_error(r#"$x =~ tr{a-z}{A-Z;"#, "missing closing delimiter in transliteration");
}
