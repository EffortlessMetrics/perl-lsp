//! Regression tests for issue #2835 — unexpected_rbrace_expr bucket.
//!
//! Root cause: `is_likely_prototype` did not recognise `+` (TokenKind::Plus)
//! as a valid prototype character, so `sub foo (+) { ... }` was misrouted
//! into signature parsing.  The signature parser then tried to parse `+` as
//! an expression, failed, and left a dangling `}` that became the
//! `unexpected_rbrace_expr` error node.
//!
//! Fix: add `TokenKind::Plus` to the unambiguous-prototype match arm in
//! `is_likely_prototype` and the explicit arm in `parse_prototype`.
#![allow(clippy::panic)]
//!
//! Perl spec reference (perlsub):
//!   `+` in a prototype means "scalar, or a reference to an array or hash".
//!   Example: `sub myfunc (+) { ... }` — accepts anything that would satisfy
//!   the scalar-or-reftype check.  Used heavily in List::Util, CPAN modules.

mod cpan_test_helpers;
use cpan_test_helpers::*;
use perl_parser_core::NodeKind;

// ---------------------------------------------------------------------------
// Core `+` prototype patterns
// ---------------------------------------------------------------------------

/// Simplest `+` prototype: `sub foo (+) { ... }`.
/// Was the primary reproducer producing unexpected_rbrace_expr.
#[test]
fn test_proto_plus_basic() {
    assert_clean_parse("sub foo (+) { 1 }");
}

/// `+` followed by a semicolon-separated optional arg: `(+;$)`.
#[test]
fn test_proto_plus_semicolon_optional() {
    assert_clean_parse("sub foo (+;$) { 1 }");
}

/// Multiple `+` positions: `(+$+)` — unusual but syntactically valid.
#[test]
fn test_proto_plus_multiple() {
    assert_clean_parse("sub foo (+$+) { 1 }");
}

/// `+` combined with `@`: `(+@)`.
#[test]
fn test_proto_plus_at() {
    assert_clean_parse("sub foo (+@) { 1 }");
}

/// Forward declaration with `+` prototype: `sub foo (+);`.
#[test]
fn test_proto_plus_forward_decl() {
    assert_clean_parse("sub foo (+);");
}

/// Forward declaration with `+;$` prototype.
#[test]
fn test_proto_plus_semicolon_forward() {
    assert_clean_parse("sub foo (+;$);");
}

// ---------------------------------------------------------------------------
// Real-world CPAN patterns using `+` prototype
// ---------------------------------------------------------------------------

/// List::Util-style `any`/`all`/`none` that takes a coderef + list.
/// These use `(&@)` but `+` is the modern alternative in some forks.
#[test]
fn test_list_util_style_plus_proto() {
    assert_clean_parse(
        r#"
sub myany (+@) {
    my $code = shift;
    for (@_) { return 1 if $code->() }
    return 0;
}
"#,
    );
}

/// POSIX-style accessor with `+` prototype (scalar-or-ref).
#[test]
fn test_plus_proto_accessor() {
    assert_clean_parse(
        r#"
sub safe_value (+) {
    my $val = shift;
    return ref($val) ? $$val : $val;
}
"#,
    );
}

/// Named sub with attribute AND `+` prototype (attribute comes before).
/// Tests parser tolerance: `sub foo : lvalue (+)` is not valid Perl (Perl requires
/// prototype before attributes), but the parser should not crash on it.
#[test]
fn test_named_sub_attr_then_plus_proto() {
    assert_clean_parse("sub foo : lvalue (+) { 1 }");
}

#[test]
fn test_proto_then_attr_correct_order() {
    let ast = parse("sub foo (+) : lvalue { $_[0] }");
    let ast_kind = format!("{:?}", ast.kind);
    let NodeKind::Program { statements } = &ast.kind else {
        panic!("expected program node, got: {}", ast_kind);
    };
    let Some(first) = statements.first() else {
        panic!("expected a subroutine statement");
    };
    let first_kind = format!("{:?}", first.kind);
    let NodeKind::Subroutine { prototype, attributes, .. } = &first.kind else {
        panic!("expected subroutine node, got: {}", first_kind);
    };
    assert!(prototype.is_some(), "expected prototype for sub foo (+)");
    assert_eq!(attributes, &vec!["lvalue".to_string()]);
}

// ---------------------------------------------------------------------------
// Regression guard: previously-passing prototype patterns must still pass
// ---------------------------------------------------------------------------

#[test]
fn test_proto_dollar_guard() {
    assert_clean_parse("sub foo ($) { 1 }");
}

#[test]
fn test_proto_at_guard() {
    assert_clean_parse("sub foo (@) { 1 }");
}

#[test]
fn test_proto_amp_guard() {
    assert_clean_parse("sub foo (&) { 1 }");
}

#[test]
fn test_proto_star_guard() {
    assert_clean_parse("sub foo (*) { 1 }");
}

#[test]
fn test_proto_empty_guard() {
    assert_clean_parse("sub foo () { 1 }");
}

#[test]
fn test_proto_backslash_percent_guard() {
    assert_clean_parse(r#"sub foo (\%) { 1 }"#);
}

#[test]
fn test_proto_semicolon_guard() {
    assert_clean_parse("sub foo ($$;@) { 1 }");
}

#[test]
fn test_proto_amp_semicolon_guard() {
    assert_clean_parse(r#"sub foo (&;@) { 1 }"#);
}

#[test]
fn test_proto_backslash_bracket_guard() {
    assert_clean_parse(r#"sub foo (\[$@%]) { 1 }"#);
}

// ---------------------------------------------------------------------------
// Edge case: `++` in prototype — lexer merges two `+` into Increment token
// ---------------------------------------------------------------------------

/// `sub foo(++$)` — Perl allows `++` in prototypes (two consecutive `+` chars).
/// The lexer produces `Increment` (not two `Plus` tokens), so `is_likely_prototype`
/// and `parse_prototype` must handle `TokenKind::Increment` explicitly.
#[test]
fn test_proto_double_plus() {
    assert_clean_parse("sub foo(++$) { 1 }");
}

/// `++` prototype with no other args.
#[test]
fn test_proto_double_plus_only() {
    assert_clean_parse("sub foo(++) { 1 }");
}

// ---------------------------------------------------------------------------
// Non-regression: `+` in sub BODY must not be confused with a prototype `+`
// ---------------------------------------------------------------------------

/// `+` inside the body of a sub (as an arithmetic operator) must parse cleanly.
/// is_likely_prototype is only called at sub-declaration time, never inside a body,
/// so this is verifying the parser's overall structural correctness for `+` as operator.
#[test]
fn test_sub_body_plus_expression() {
    assert_clean_parse(
        r#"
sub add_one {
    my ($x) = @_;
    return $x + 1;
}
"#,
    );
}

/// Anonymous sub with `+` in body — ensure no false prototype detection.
#[test]
fn test_anon_sub_body_plus_expression() {
    assert_clean_parse("my $f = sub { $_[0] + $_[1] };");
}
