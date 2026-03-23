//! Tests for issue #2750 Pattern B: qualified `Pkg::func "literal"` with non-variable arg
//! inside parenthesized expressions.
//!
//! Root cause: In paren-expression context, the parser calls `parse_assignment_or_declaration()`
//! for each element. For `(Carp::croak "err")`, it parses `Carp::croak` as a complete
//! qualified identifier expression, then expects `)` but finds the string `"err"`.
//! Unqualified calls (`croak "err"`) work because they are handled as bareword function
//! calls with argument parsing. `(Carp::croak $x)` also works (variable arg).
//! The bug: qualified calls (`Pkg::func`) in expression context do not get the same
//! space-separated argument treatment as unqualified bareword calls.
//!
//! Fix: When parsing a paren-list element and the result is a qualified identifier
//! (`Foo::bar`) in expression context, check if the next token is a non-comma, non-`)`
//! term (string, number). If so, parse it as arguments like an unqualified bareword call does.
//!
//! Affected corpus files: `IO/Compress/Base.pm`

mod cpan_test_helpers;
use cpan_test_helpers::*;

// ---- Failing cases: qualified call with string/number literal arg in paren context ----

#[test]
fn test_qualified_call_string_arg_in_paren() {
    // Primary reproducer: (Carp::croak "error message")
    assert_clean_parse(r#"(Carp::croak "error message");"#);
}

#[test]
fn test_qualified_call_in_boolean_paren() {
    // Conditional: (x or Carp::croak "err")
    assert_clean_parse(r#"($x or Carp::croak "err");"#);
}

#[test]
fn test_qualified_call_or_boolean_paren() {
    // Method call result or die pattern in paren
    assert_clean_parse(r#"(utf8::downgrade($$buf, 1) or Carp::croak "Wide char in write");"#);
}

#[test]
fn test_qualified_call_number_arg_in_paren() {
    // Qualified call with numeric literal arg
    assert_clean_parse(r#"(Carp::croak 1);"#);
}

#[test]
fn test_qualified_call_multiple_string_args_in_paren() {
    // Multiple string args to a qualified call in paren
    assert_clean_parse(r#"(Foo::bar "x", "y");"#);
}

// ---- Regression: existing valid patterns must still work ----

#[test]
fn test_qualified_call_variable_arg_regression() {
    // (Carp::croak $x) — variable arg already worked, must not regress
    assert_clean_parse(r#"(Carp::croak $x);"#);
}

#[test]
fn test_qualified_call_with_explicit_parens_regression() {
    // (Carp::croak("error")) — explicit parens already worked
    assert_clean_parse(r#"(Carp::croak("error"));"#);
}

#[test]
fn test_qualified_call_at_statement_level_regression() {
    // Carp::croak "error" at statement level — already worked
    assert_clean_parse(r#"Carp::croak "error";"#);
}

#[test]
fn test_qualified_const_in_hash_regression() {
    // (Foo::BAR => 1) — fat arrow must autoquote, not parse as call
    assert_clean_parse(r#"my %h = (Foo::BAR => 1);"#);
}

#[test]
fn test_unqualified_call_string_arg_regression() {
    // Unqualified (croak "err") already worked, must not regress
    assert_clean_parse(r#"(croak "error");"#);
}

// ---- Edge cases: less-common but reachable patterns ----

#[test]
fn test_qualified_call_interpolated_string_arg() {
    // Interpolated string (double-quoted with variable) — TokenKind::String
    assert_clean_parse(r#"(Carp::croak "Error: $msg");"#);
}

#[test]
fn test_qualified_call_single_quoted_arg() {
    // Single-quoted string '...' — also TokenKind::String
    assert_clean_parse(r#"(Carp::croak 'hard error');"#);
}

#[test]
fn test_qualified_call_no_args_regression() {
    // (Foo::bar) with no args — must not fire Pattern B, must stay as identifier
    assert_clean_parse(r#"(Foo::bar);"#);
}

#[test]
fn test_deeply_qualified_call_in_paren() {
    // Three-segment qualified name
    assert_clean_parse(r#"(Foo::Bar::baz "arg");"#);
}
