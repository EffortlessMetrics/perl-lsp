//! Tests for named builtin calls followed by a ternary operator.
//!
//! Bug: `defined($x) ? $x : "d"` and `ref($x) ? 1 : 0` were mis-parsed —
//! the ternary was absorbed INTO the builtin's argument list instead of being
//! applied to the call's RESULT.
//!
//! `foo($x) ? 1 : 0` (user-defined function) always parsed correctly.
//! This regression ensures named builtins in `is_optional_arg_builtin` behave
//! the same way when called with explicit parentheses.
//!
//! Note on sexp format:
//! - `defined` and `ref` use `(call defined (...))` / `(call ref (...))`
//! - other builtins like `chr`, `length` use `(ambiguous_function_call_expression ...)`

use perl_parser_core::Parser;
use perl_tdd_support::must;

fn parse_sexp(src: &str) -> String {
    let mut parser = Parser::new(src);
    let ast = must(parser.parse());
    ast.to_sexp()
}

/// defined($x) ? $x : "d"  — ternary must be at the TOP (defined call is the condition)
#[test]
fn test_defined_paren_arg_ternary_top_level() {
    let sexp = parse_sexp("defined($x) ? $x : \"d\";");
    // The ternary should wrap the call, not be inside it
    assert!(
        sexp.contains("(ternary"),
        "expected ternary at top level for 'defined($x) ? $x : \"d\"', got: {sexp}"
    );
    // The call should NOT contain the ternary as an argument
    assert!(
        !sexp.contains("(call defined ((ternary"),
        "defined should NOT absorb the ternary into its args, got: {sexp}"
    );
}

/// ref($x) ? 1 : 0  — ternary must be at the TOP (ref call is the condition)
#[test]
fn test_ref_paren_arg_ternary_top_level() {
    let sexp = parse_sexp("ref($x) ? 1 : 0;");
    assert!(
        sexp.contains("(ternary"),
        "expected ternary at top level for 'ref($x) ? 1 : 0', got: {sexp}"
    );
    assert!(
        !sexp.contains("(call ref ((ternary"),
        "ref should NOT absorb the ternary into its args, got: {sexp}"
    );
}

/// chr($x) ? 1 : 0  — same structural expectation (uses ambiguous_function_call_expression format)
#[test]
fn test_chr_paren_arg_ternary_top_level() {
    let sexp = parse_sexp("chr($x) ? 1 : 0;");
    assert!(
        sexp.contains("(ternary"),
        "expected ternary at top level for 'chr($x) ? 1 : 0', got: {sexp}"
    );
    // chr uses ambiguous_function_call_expression format, not (call chr ...)
    assert!(
        !sexp.contains("(ambiguous_function_call_expression (function) (ternary"),
        "chr should NOT absorb the ternary into its args, got: {sexp}"
    );
}

/// length($s) ? "a" : "b"  — same structural expectation
#[test]
fn test_length_paren_arg_ternary_top_level() {
    let sexp = parse_sexp("length($s) ? \"a\" : \"b\";");
    assert!(
        sexp.contains("(ternary"),
        "expected ternary at top level for 'length($s) ? \"a\" : \"b\"', got: {sexp}"
    );
    assert!(
        !sexp.contains("(ambiguous_function_call_expression (function) (ternary"),
        "length should NOT absorb the ternary into its args, got: {sexp}"
    );
}

/// Regression: defined($x) without ternary must still parse cleanly
#[test]
fn test_defined_paren_arg_no_ternary_clean() {
    let sexp = parse_sexp("defined($x);");
    assert!(
        !sexp.contains("ERROR"),
        "defined($x) should parse cleanly without ternary, got: {sexp}"
    );
    assert!(
        sexp.contains("(call defined"),
        "defined call should appear in the parse output, got: {sexp}"
    );
}

/// Regression: chr($x) without ternary must still parse cleanly
#[test]
fn test_chr_paren_arg_no_ternary_clean() {
    let sexp = parse_sexp("chr($x);");
    assert!(!sexp.contains("ERROR"), "chr($x) should parse cleanly without ternary, got: {sexp}");
    assert!(
        sexp.contains("(ambiguous_function_call_expression (function)"),
        "chr call should appear in the parse output, got: {sexp}"
    );
}

/// Regression: user-defined foo($x) ? 1 : 0 must still be correct
#[test]
fn test_user_defined_func_paren_arg_ternary_top_level() {
    let sexp = parse_sexp("foo($x) ? 1 : 0;");
    assert!(
        sexp.contains("(ternary"),
        "expected ternary at top level for 'foo($x) ? 1 : 0', got: {sexp}"
    );
}

/// Regression: substr($s, 0, 1) ? 1 : 0 — builtin with multiple paren args
#[test]
fn test_substr_multi_arg_ternary_top_level() {
    let sexp = parse_sexp("substr($s, 0, 1) ? 1 : 0;");
    assert!(
        sexp.contains("(ternary"),
        "expected ternary at top level for 'substr($s, 0, 1) ? 1 : 0', got: {sexp}"
    );
}
