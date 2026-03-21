//! Regression tests: qr<> angle-bracket delimiter (corpus: Template-Toolkit)
//! Issue #2406: Template/Parser.pm was filed under unclosed_angle error bucket (stale sweep data).
//! Plan-review confirmed the parser already handles qr<...> correctly at all layers.
//! Template/Parser.pm line 391: my $tags_dir = $self->{ANYCASE} ? qr<TAGS>i : qr<TAGS>;
//! These tests lock in the correct behaviour and prevent future regressions.

mod cpan_test_helpers;
use cpan_test_helpers::*;

// --- qr<...> basic patterns ---

#[test]
fn qr_angle_basic() {
    assert_clean_parse(r#"my $re = qr<TAGS>;"#);
}

#[test]
fn qr_angle_with_i_modifier() {
    assert_clean_parse(r#"my $re = qr<TAGS>i;"#);
}

#[test]
fn qr_angle_multiple_modifiers() {
    assert_clean_parse(r#"my $re = qr<foo\s+bar>xi;"#);
}

#[test]
fn qr_angle_in_ternary() {
    // The exact pattern from Template::Parser line 391
    assert_clean_parse(r#"my $tags_dir = $self->{ANYCASE} ? qr<TAGS>i : qr<TAGS>;"#);
}

#[test]
fn qr_angle_nested_delimiters() {
    // Nested angle brackets — depth counter must handle these
    assert_clean_parse(r#"my $re = qr<a<b>c>;"#);
}

#[test]
fn qr_angle_special_chars() {
    assert_clean_parse(r#"my $re = qr<\d+\s*>;"#);
}

#[test]
fn qr_angle_empty() {
    assert_clean_parse(r#"my $re = qr<>;"#);
}

// --- Other quote-like operators with angle brackets ---

#[test]
fn m_angle_delimiter() {
    // m<> uses the same delimiter path as qr<>
    assert_clean_parse(r#"if ($x =~ m<pattern>i) { }"#);
}

#[test]
fn s_angle_angle() {
    assert_clean_parse(r#"$x =~ s<foo><bar>;"#);
}

#[test]
fn tr_angle_angle() {
    assert_clean_parse(r#"$x =~ tr<a-z><A-Z>;"#);
}

#[test]
fn y_angle_angle() {
    assert_clean_parse(r#"$x =~ y<abc><def>;"#);
}

// --- qr with other paired delimiters (regression guard) ---

#[test]
fn qr_parens() {
    assert_clean_parse(r#"my $re = qr(foo)i;"#);
}

#[test]
fn qr_brackets() {
    assert_clean_parse(r#"my $re = qr[foo]i;"#);
}

#[test]
fn qr_braces() {
    assert_clean_parse(r#"my $re = qr{foo}i;"#);
}

// --- Real-world patterns from CPAN ---

#[test]
fn qr_angle_complex_pattern() {
    // Complex nested angle-bracket pattern
    assert_clean_parse(r#"my $re = qr<(?:foo|bar)<baz>>xi;"#);
}
