//! Tests for issue #2395: s///e modifier with expressions containing `/` in replacement
//!
//! Root cause: the lexer's parse_substitution() stops scanning the replacement at the first
//! unescaped `/`, even when that `/` is inside a string literal like `"foo/bar"` or `'/'`.
//! This breaks `s/foo/sprintf("%s/%s", $a, $b)/e` and similar patterns.

mod cpan_test_helpers;
use cpan_test_helpers::*;

/// Replacement contains `/` inside a double-quoted string literal
#[test]
fn test_subst_e_slash_in_double_quoted_replacement() {
    let source = r#"$str =~ s/foo/sprintf("%s/%s", $a, $b)/e;"#;
    assert_clean_parse(source);
}

/// Replacement contains `/` inside a single-quoted string literal
#[test]
fn test_subst_e_slash_in_single_quoted_replacement() {
    let source = r#"$str =~ s/([A-Za-z]+)/join('/', @parts)/ge;"#;
    assert_clean_parse(source);
}

/// Replacement is a ternary expression with `/` in one branch
#[test]
fn test_subst_e_ternary_with_slash_in_replacement() {
    let source = r#"s/$MATCH/defined($map{$MATCH}) ? $map{$MATCH} : "default/$MATCH"/ge;"#;
    assert_clean_parse(source);
}

/// Replacement uses sprintf with a format string containing `/`
#[test]
fn test_subst_e_sprintf_format_slash() {
    let source = r#"$path =~ s|(\w+)|sprintf("%s/%s", $base, $1)|ge;"#;
    assert_clean_parse(source);
}

/// Simple /e cases that should already work (regression guard)
#[test]
fn test_subst_e_simple_lc() {
    let source = r#"$str =~ s/([A-Z])/lc($1)/ge;"#;
    assert_clean_parse(source);
}

#[test]
fn test_subst_e_simple_arithmetic() {
    let source = r#"$x =~ s/(\d+)/$1 * 2/e;"#;
    assert_clean_parse(source);
}

#[test]
fn test_subst_e_hash_lookup() {
    let source = r#"$str =~ s/\b($word)\b/$map{$1}/ge;"#;
    assert_clean_parse(source);
}

/// Template Toolkit style: replacement is a full conditional expression
#[test]
fn test_subst_e_template_toolkit_style() {
    let source = r#"s/$pattern/$replacement/ge;"#;
    assert_clean_parse(source);
}
