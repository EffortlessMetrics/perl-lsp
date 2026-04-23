mod cpan_test_helpers;
use cpan_test_helpers::*;

#[test]
fn not_after_logical_and() {
    assert_clean_parse(r#"my $x = $a && not $b;"#);
}

#[test]
fn not_after_method_logical_and() {
    assert_clean_parse(r#"my $x = $obj->flag && not $disabled;"#);
}

#[test]
fn not_after_word_and() {
    assert_clean_parse(r#"$x and not $y;"#);
}

#[test]
fn not_in_parenthesized_expr() {
    assert_clean_parse(r#"my $x = (not $flag);"#);
}

#[test]
fn not_in_complex_expr() {
    assert_clean_parse(r#"if ($a && not $b || $c) { 1; }"#);
}

#[test]
fn not_with_if_modifier() {
    assert_clean_parse(r#"die "error" if not $ok;"#);
}

#[test]
fn filetest_and_return() {
    assert_clean_parse(r#"sub f { -r and return $_; }"#);
}

#[test]
fn our_var_and_warn() {
    assert_clean_parse(r#"our $DEBUG and warn "debugging";"#);
}

#[test]
fn my_var_and_expr() {
    assert_clean_parse(r#"my $ok = 1 and warn "assigned";"#);
}

#[test]
fn method_and_return_in_sub() {
    assert_clean_parse(r#"sub check { $self->valid or return; }"#);
}

#[test]
fn use_filter_simple_sub() {
    assert_clean_parse(r#"use Filter::Simple sub { $_ = lc $_; };"#);
}

#[test]
fn use_module_sub_block() {
    assert_clean_parse(r#"use My::Module sub { my $x = 1; return $x; };"#);
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
fn basic_or_and_not() {
    assert_clean_parse(r#"$a or $b and not $c;"#);
}

#[test]
fn open_or_die() {
    assert_clean_parse(r#"open my $fh, '<', $file or die "Cannot open: $!";"#);
}
