//! Tests for #752 finding 8: variable-declaration attributes with parenthesized args.
//!
//! Before the fix, the inline `while Colon` loops in `variables.rs` only consumed
//! the attribute name and dropped `(arg)` parenthesized argument lists entirely.
//! After the fix they delegate to `parse_declaration_attributes()`, which properly
//! collects the parenthesized content.

mod cpan_test_helpers;
use cpan_test_helpers::*;

// ---------------------------------------------------------------------------
// Parenthesized attribute args — the failing cases before the fix
// ---------------------------------------------------------------------------

#[test]
fn my_scalar_custom_attr_with_paren_arg() {
    // my $x :custom(arg);  — paren arg must not be dropped or error
    assert_clean_parse("my $x :custom(arg);");
}

#[test]
fn my_hash_attr_with_colon_prefixed_inner_attrs() {
    // my %h :ATTR(:get<title> :set<title>);  — inner ':' must not fail parse
    assert_clean_parse("my %h :ATTR(:get<title> :set<title>);");
}

#[test]
fn my_scalar_attr_with_numeric_args() {
    // my $x :Foo(1,2);  — comma-separated args inside parens
    assert_clean_parse("my $x :Foo(1,2);");
}

#[test]
fn my_array_attr_with_bareword_arg() {
    // my @a :Bar(x);
    assert_clean_parse("my @a :Bar(x);");
}

#[test]
fn my_scalar_attr_arg_captured_in_sexp() {
    // The attribute string including the paren args must appear in the sexp.
    let ast = parse("my $x :custom(arg);");
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("ERROR"), "Parse of `my $x :custom(arg);` produced ERROR nodes: {sexp}",);
    // attributes should include "custom(arg)" — the full paren-arg form
    assert!(sexp.contains("custom(arg)"), "Expected `custom(arg)` in sexp attributes, got: {sexp}",);
}

#[test]
fn my_hash_attr_arg_captured_in_sexp() {
    let ast = parse("my %h :ATTR(:get<title>);");
    let sexp = ast.to_sexp();
    assert!(
        !sexp.contains("ERROR"),
        "Parse of `my %h :ATTR(:get<title>);` produced ERROR nodes: {sexp}",
    );
    // The attribute should be captured with its args
    assert!(sexp.contains("ATTR("), "Expected `ATTR(` in sexp attributes, got: {sexp}",);
}

// ---------------------------------------------------------------------------
// Regression guard: simple (no-paren) variable attributes still work
// ---------------------------------------------------------------------------

#[test]
fn regression_my_scalar_shared_attr() {
    // my $x :shared;  — plain attribute without args, must still work
    assert_clean_parse("my $x :shared;");
}

#[test]
fn regression_my_array_shared_attr() {
    assert_clean_parse("my @arr :shared;");
}

#[test]
fn regression_my_hash_shared_attr() {
    assert_clean_parse("my %h :shared;");
}

#[test]
fn regression_my_scalar_multiple_attrs() {
    // my $x :A :B;  — multiple attributes, no parens
    assert_clean_parse("my $x :A :B;");
}

#[test]
fn regression_my_scalar_with_init() {
    // my $x :shared = 1;  — attribute + initializer
    assert_clean_parse("my $x :shared = 1;");
}

// ---------------------------------------------------------------------------
// Regression guard: sub and class attributes unchanged
// ---------------------------------------------------------------------------

#[test]
fn regression_sub_lvalue_attr() {
    assert_clean_parse("sub f :lvalue {}");
}

#[test]
fn regression_class_isa_attr() {
    assert_clean_parse("class C :isa(Base) {}");
}
