//! Tests for issue #2896: s/''/'/g single-quote in substitution replacement
//! with slash delimiter — the string-skip arm incorrectly treats `'` as a
//! string opener, consuming past the closing `/`.

mod cpan_test_helpers;
use cpan_test_helpers::*;

/// Core failing case: empty pattern, single-quote replacement, slash delimiter.
/// From TAP/Parser/YAMLish/Reader.pm and Log/Log4perl/DateFormat.pm.
#[test]
fn subst_slash_delim_squote_replacement() {
    // $literal =~ s/''/'/g
    assert_clean_parse(r#"$literal =~ s/''/'/g;"#);
}

/// Variant: single-quote pattern with double-single-quote replacement.
/// From PgCommon.pm — this may already pass; kept as a regression guard.
#[test]
fn subst_slash_delim_squote_pattern_squote_replacement() {
    // $value =~ s/'/''/g
    assert_clean_parse(r#"$value =~ s/'/''/g;"#);
}

/// Double-quote variant: same structure with `"` as the replacement content.
#[test]
fn subst_slash_delim_dquote_replacement() {
    assert_clean_parse(r#"$x =~ s/""/"/g;"#);
}

/// Single literal quote as the full replacement with /e modifier.
#[test]
fn subst_slash_delim_squote_replacement_e_modifier() {
    assert_clean_parse(r#"$x =~ s/pat/'/e;"#);
}

/// Regression: s///e with string containing closing delimiter must still work.
/// This exercises the string-skip logic that was added for issue #2395.
#[test]
fn subst_e_string_containing_closing_delim_regression() {
    assert_clean_parse(r#"$str =~ s/([A-Za-z]+)/join('/', @parts)/ge;"#);
}

/// Regression: s///e with double-quoted string containing /.
#[test]
fn subst_e_dquoted_string_containing_slash_regression() {
    assert_clean_parse(r#"$str =~ s/foo/sprintf("%s/%s", $a, $b)/e;"#);
}

/// Regression: plain s/// with slash delimiter still works.
#[test]
fn subst_slash_delim_basic_regression() {
    assert_clean_parse(r#"$x =~ s/foo/bar/g;"#);
}
