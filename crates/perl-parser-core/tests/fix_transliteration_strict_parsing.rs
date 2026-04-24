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
fn transliteration_rejects_invalid_delimiters_and_malformed_closures() {
    assert_has_error(r#"$x =~ tr\abc\xyz\;"#, "missing delimiter after transliteration operator");
    assert_has_error(r#"$x =~ tr{abc}{xyz;"#, "missing closing delimiter in transliteration");
    assert_has_error(r#"$x =~ y(abc)(xyz;"#, "missing closing delimiter in transliteration");
}

#[test]
fn transliteration_accepts_supported_modifiers_and_edge_bodies() {
    assert_clean_parse(r#"$x =~ tr/a/b/c;"#);
    assert_clean_parse(r#"$x =~ tr/a/b/d;"#);
    assert_clean_parse(r#"$x =~ tr/a/b/s;"#);
    assert_clean_parse(r#"$x =~ tr/a/b/r;"#);
    assert_clean_parse(r#"$x =~ tr/a\//b\//;"#);
    assert_clean_parse(r#"$x =~ tr/αβγ/ΑΒΓ/;"#);
    assert_clean_parse(r#"$x =~ tr/abc//;"#);
    assert_has_error(r#"$x =~ tr//abc/;"#, "missing search list in transliteration");
}
