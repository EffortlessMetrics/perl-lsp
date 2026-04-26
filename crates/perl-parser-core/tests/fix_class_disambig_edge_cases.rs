//! Edge-case regression tests for the class-keyword and signature-attribute
//! disambiguation fix.
//!
//! Covers scenarios that the primary test (`bareword_class_call_is_not_forced_into_native_class_declaration`)
//! does not exercise:
//!   - `class->method()` method call on a bareword class name
//!   - Multiple `:attr` on one signature parameter
//!   - `:attr(args)` with nested parentheses
//!   - Mixed `:attr` + default value on one parameter

mod cpan_test_helpers;
use cpan_test_helpers::assert_clean_parse;

#[test]
fn class_method_call_as_bareword_parses_cleanly() {
    // `class->new()` is common Moose-style code where `class` is used as a
    // bareword package name.  Before the guard, the parser attempted to parse
    // `->new()` as the class name and failed.
    assert_clean_parse(r#"my $obj = class->new();"#);
}

#[test]
fn class_hash_key_as_fat_arrow_parses_cleanly() {
    // `class =>` autoquotes the keyword — should not trigger native-class parsing.
    assert_clean_parse(r#"my %h = (class => "MyClass");"#);
}

#[test]
fn signature_param_multiple_attrs_no_error() {
    // Two plain attributes on a single parameter.
    assert_clean_parse(r#"sub build ($x :param :required) { }"#);
}

#[test]
fn signature_param_attr_with_nested_parens_no_error() {
    // Attribute with parenthesised argument containing nested parens.
    // `consume_signature_param_attributes` must balance all paren levels.
    assert_clean_parse(r#"sub build ($x :reader(get_x())) { }"#);
}

#[test]
fn signature_param_attr_then_default_no_error() {
    // Attribute followed by a default value — both must coexist.
    // The span's `end` must reflect the default, not the attribute.
    assert_clean_parse(r#"sub build ($x :param = 42) { }"#);
}

#[test]
fn signature_param_attr_with_args_then_default_no_error() {
    // Attribute with parens AND a default value.
    assert_clean_parse(r#"sub build ($x :reader(get_x) = "default") { }"#);
}

#[test]
fn native_class_with_package_qualified_name_parses_cleanly() {
    // `class Foo::Bar { }` — DoubleColon in the class name must still trigger
    // native-class parsing (via the peek_second == DoubleColon guard arm).
    // Before the guard the DoubleColon was handled but the guard being removed
    // could accidentally break this. Lock it explicitly.
    assert_clean_parse(r#"class Foo::Bar { }"#);
}

#[test]
fn signature_named_param_colon_dollar_not_treated_as_attr() {
    // `:$name` is a named parameter, not a trailing attribute.
    // The `named` check at the top of parse_signature_param consumes the leading `:`,
    // so consume_signature_param_attributes must not see a `:` for a named param
    // that has already been consumed.
    assert_clean_parse(r#"sub build (:$host, :$port = 80) { }"#);
}
