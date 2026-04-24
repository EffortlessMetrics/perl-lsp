//! Regression tests for strict transliteration parsing.

mod cpan_test_helpers;
use cpan_test_helpers::*;

#[test]
fn transliteration_allows_whitespace_before_delimiter() {
    assert_clean_parse(r#"$x =~ tr /a-z/A-Z/;"#);
    assert_clean_parse(r#"$x =~ y  {abc}{xyz}r;"#);
}

#[test]
fn transliteration_rejects_invalid_delimiter_and_modifiers() {
    assert_has_error(r#"$x =~ tr/a-z/A-Z/z;"#, "invalid transliteration modifier");
    assert_has_error(r#"$x =~ y/a-z/A-Z/1;"#, "invalid transliteration modifier");
}

#[test]
fn transliteration_handles_empty_and_unicode_bodies() {
    assert_has_error(r#"$x =~ tr///;"#, "missing search list in transliteration");
    assert_clean_parse(r#"$x =~ tr/🦀α/🐪β/r;"#);
    assert_clean_parse(r#"$x =~ tr/a\\/b/c\\/d/;"#);
}

#[test]
fn transliteration_detects_malformed_closures() {
    assert_has_error(r#"$x =~ tr{abc}{xyz;"#, "missing closing delimiter in transliteration");
    assert_has_error(r#"$x =~ tr/a/b"#, "missing closing delimiter in transliteration");
}

#[test]
fn transliteration_accepts_only_valid_flags() {
    assert_clean_parse(r#"$x =~ tr/a/b/c;"#);
    assert_clean_parse(r#"$x =~ tr/a/b/d;"#);
    assert_clean_parse(r#"$x =~ tr/a/b/s;"#);
    assert_clean_parse(r#"$x =~ tr/a/b/r;"#);
    assert_has_error(r#"$x =~ tr/a/b/q;"#, "invalid transliteration modifier");
}
