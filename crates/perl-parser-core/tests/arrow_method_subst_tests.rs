mod cpan_test_helpers;
use cpan_test_helpers::*;

// Tests for -> method calls with names that are also regex/substitution keywords.
// These must parse as method calls, not as s///, y///, tr/// operators.

#[test]
fn arrow_method_s_with_parens() {
    assert_clean_parse(r#"$obj->s("foo", "bar");"#);
}

#[test]
fn arrow_method_y_with_parens() {
    assert_clean_parse(r#"$obj->y("foo");"#);
}

#[test]
fn arrow_method_tr_with_parens() {
    assert_clean_parse(r#"$obj->tr("old", "new");"#);
}

#[test]
fn arrow_method_m_with_parens() {
    assert_clean_parse(r#"$obj->m("pattern");"#);
}

#[test]
fn arrow_method_q_with_parens() {
    assert_clean_parse(r#"$obj->q("value");"#);
}

#[test]
fn arrow_method_s_no_parens() {
    // $obj->s followed by non-delimiter should be a method call with no args
    assert_clean_parse(r#"my $result = $obj->s;"#);
}

#[test]
fn arrow_method_chained() {
    assert_clean_parse(r#"$obj->s("x")->y("z");"#);
}

// Regression tests: s///, y///, tr/// must still work in normal contexts

#[test]
fn subst_basic() {
    assert_clean_parse(r#"$x =~ s/foo/bar/;"#);
}

#[test]
fn subst_standalone() {
    assert_clean_parse(r#"s/foo/bar/;"#);
}

#[test]
fn subst_after_semicolon() {
    assert_clean_parse(r#"my $x = 1; s/foo/bar/;"#);
}

#[test]
fn subst_brace_delimiters() {
    assert_clean_parse(r#"s{foo}{bar};"#);
}

#[test]
fn transliteration_basic() {
    assert_clean_parse(r#"$x =~ tr/a-z/A-Z/;"#);
}

#[test]
fn y_transliteration_basic() {
    assert_clean_parse(r#"$x =~ y/a-z/A-Z/;"#);
}

#[test]
fn subst_in_condition() {
    assert_clean_parse(r#"if (s/foo/bar/) { 1; }"#);
}

#[test]
fn subst_global_flag() {
    assert_clean_parse(r#"s/foo/bar/g;"#);
}

// Edge cases: s/// after keywords that might not reset mode to ExpectTerm

#[test]
fn subst_after_die() {
    assert_clean_parse(r#"die s/foo/bar/;"#);
}

#[test]
fn subst_after_warn() {
    assert_clean_parse(r#"warn s/foo/bar/;"#);
}

#[test]
fn subst_after_return() {
    assert_clean_parse(r#"sub f { return s/foo/bar/; }"#);
}

#[test]
fn subst_after_or() {
    assert_clean_parse(r#"$x or s/foo/bar/;"#);
}

#[test]
fn subst_after_and() {
    assert_clean_parse(r#"$x and s/foo/bar/;"#);
}

#[test]
fn subst_after_chomp() {
    assert_clean_parse(r#"chomp s/foo/bar/;"#);
}

#[test]
fn subst_after_defined() {
    assert_clean_parse(r#"defined s/foo/bar/;"#);
}

#[test]
fn subst_paren_delimiters() {
    assert_clean_parse(r#"s(foo)(bar);"#);
}
