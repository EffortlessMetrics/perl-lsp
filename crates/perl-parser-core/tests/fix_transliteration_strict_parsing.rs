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
fn transliteration_strict_handles_edge_cases() {
    assert_clean_parse(r#"$x =~ tr/a\/b/c\/d/;"#);
    assert_clean_parse(r#"$x =~ tr/αβγ/ΑΒΓ/;"#);
    assert_clean_parse(r#"$x =~ tr/a/b/cdsr;"#);
    assert_clean_parse(r#"$x =~ tr{abc}{xyz}r;"#);
    assert_clean_parse(r#"$x =~ tr[abc]{xyz}r;"#);

    assert_has_error(r#"$x =~ tr///;"#, "missing search list");
    assert_has_error(r#"$x =~ tr/a/b/z;"#, "invalid transliteration modifier");
    assert_has_error(r#"$x =~ tr/a/b;"#, "closing delimiter in transliteration");
    assert_has_error(r#"$x =~ tr{abc}{xyz;"#, "closing delimiter in transliteration");
}

#[test]
fn transliteration_supports_mixed_paired_delimiters() {
    assert_clean_parse(r#"$x =~ tr[a-z]{A-Z}d;"#);
    assert_clean_parse(r#"$x =~ y<abc>[xyz]r;"#);
}

#[test]
fn transliteration_reports_missing_replacement() {
    assert_has_error(
        r#"$x =~ tr{abc};"#,
        "missing replacement list in transliteration",
    );
}
