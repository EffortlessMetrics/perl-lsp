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
//!
//! Perl spec reference (perlsub):
//!   `+` in a prototype means "scalar, or a reference to an array or hash".
//!   Example: `sub myfunc (+) { ... }` — accepts anything that would satisfy
//!   the scalar-or-reftype check.  Used heavily in List::Util, CPAN modules.

mod cpan_test_helpers;
use cpan_test_helpers::*;

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
/// Attribute parsing must not consume the `(` opening the prototype.
#[test]
fn test_named_sub_attr_then_plus_proto() {
    assert_clean_parse("sub foo : lvalue (+) { 1 }");
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
