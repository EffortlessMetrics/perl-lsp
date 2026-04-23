//! Tests for issue #2405: anonymous sub attribute syntax (`sub :lvalue { }`)
//! and statement label syntax (`OUTER: for ...`).
//!
//! Root cause: In expression context, `sub` followed by `:` was not recognised
//! as an anonymous subroutine — the parser only checked for `{` or `(` after
//! `sub`, ignoring the `:attr` case.

mod cpan_test_helpers;
use cpan_test_helpers::*;

// ---------------------------------------------------------------------------
// Anonymous sub with attributes — the main failing patterns from CPAN
// ---------------------------------------------------------------------------

#[test]
fn anon_sub_lvalue_attr_inline() {
    // `my $f = sub :lvalue { $_[0] };`
    let src = "my $f = sub :lvalue { $_[0] };";
    assert_clean_parse(src);
}

#[test]
fn anon_sub_lvalue_attr_with_space() {
    // `my $f = sub : lvalue { $_[0] };`
    let src = "my $f = sub : lvalue { $_[0] };";
    assert_clean_parse(src);
}

#[test]
fn anon_sub_lvalue_attr_typeglob_assign() {
    // Core CPAN pattern: *foo = sub :lvalue { $_[0] };
    let src = "*foo = sub :lvalue { $_[0] };";
    assert_clean_parse(src);
}

#[test]
fn anon_sub_lvalue_attr_multiline() {
    // XML::Twig style — attribute on the next line
    let src = "*{\"Foo::bar\"} = sub\n    :lvalue\n  { my $elt = shift; $elt->{x} };";
    assert_clean_parse(src);
}

#[test]
fn anon_sub_lvalue_attr_simple_multiline() {
    // sub on one line, :lvalue on the next
    let src = "my $f = sub\n    :lvalue\n{ $_[0] };";
    assert_clean_parse(src);
}

#[test]
fn anon_sub_method_and_lvalue_attrs() {
    // Multiple attributes on anon sub
    let src = "my $f = sub :lvalue :method { $_[0] };";
    assert_clean_parse(src);
}

#[test]
fn anon_sub_attr_in_hash_value() {
    // Anonymous sub with attribute as hash value
    let src = "my %h = (foo => sub :lvalue { $_[0] });";
    assert_clean_parse(src);
}

#[test]
fn anon_sub_attr_passed_as_arg() {
    // Anonymous sub with attribute as function argument
    let src = "install_method('foo', sub :lvalue { $_[0] });";
    assert_clean_parse(src);
}

// ---------------------------------------------------------------------------
// Named sub with attributes (already working but regression guard)
// ---------------------------------------------------------------------------

#[test]
fn named_sub_lvalue_attr() {
    let src = "sub rbuf : lvalue { (tied *${$_[0]})->[3] }";
    assert_clean_parse(src);
}

#[test]
fn named_sub_multiline_attr() {
    let src = "sub foo\n    :lvalue\n{ return 1 }";
    assert_clean_parse(src);
}

// ---------------------------------------------------------------------------
// Variable attributes: my $var : attrname (regression guard — already working)
// ---------------------------------------------------------------------------

#[test]
fn my_scalar_lvalue_attr() {
    let src = "my $x : lvalue;";
    assert_clean_parse(src);
}

#[test]
fn my_array_shared_attr() {
    let src = "my @arr : shared;";
    assert_clean_parse(src);
}

#[test]
fn my_scalar_lvalue_with_init() {
    let src = "my $y : lvalue = 1;";
    assert_clean_parse(src);
}

// ---------------------------------------------------------------------------
// Statement labels — all-caps identifiers (regression guard — already working)
// ---------------------------------------------------------------------------

#[test]
fn outer_label_for_loop() {
    let src = r#"OUTER: for my $i (1..10) { last OUTER; }"#;
    assert_clean_parse(src);
}

#[test]
fn loop_label_while() {
    let src = r#"LOOP: while (1) { last LOOP; }"#;
    assert_clean_parse(src);
}

// ---------------------------------------------------------------------------
// Real-world CPAN patterns from the issue
// ---------------------------------------------------------------------------

#[test]
fn xml_twig_style_accessor_pattern() {
    // From XML::Twig: typeglob = sub\n:lvalue\n{ ... }
    let src = r#"
*{"Foo::$att"} = sub
    :lvalue
  { my $elt = shift;
    if (@_) { $elt->{att}->{$att} = $_[0]; }
    $elt->{att}->{$att};
  };
"#;
    assert_clean_parse(src);
}

#[test]
fn thread_queue_limit_lvalue() {
    // From Thread::Queue: sub limit : lvalue { ... }
    let src = r#"
sub limit : lvalue {
    my $self = shift;
    $self->{LIMIT};
}
"#;
    assert_clean_parse(src);
}
