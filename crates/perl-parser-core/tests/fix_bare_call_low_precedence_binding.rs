mod cpan_test_helpers;
use cpan_test_helpers::*;

fn parse_ok(src: &str) -> String {
    let ast = parse(src);
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("ERROR"), "parse should succeed without errors for: {src}\ngot: {sexp}");
    sexp
}

#[test]
fn bare_call_sigil_arg_keeps_ternary_outside_call() {
    let sexp = parse_ok("sub is_ready { 1 }; my $x = is_ready $obj ? 1 : 0;");
    assert!(sexp.contains("(ternary"), "expected ternary in AST, got: {sexp}");
    assert!(
        sexp.contains("(ambiguous_function_call_expression (function) (variable $ obj))"),
        "expected bare unary-style call node in AST, got: {sexp}"
    );
    assert!(
        !sexp.contains("(ambiguous_function_call_expression (function) (ternary"),
        "is_ready bare call swallowed ternary argument: {sexp}"
    );
}

#[test]
fn bare_call_list_arg_keeps_word_or_outside_call() {
    let sexp = parse_ok("do_thing @args or die;");
    assert!(
        sexp.contains("(ambiguous_function_call_expression"),
        "expected bare unary-style call node in AST, got: {sexp}"
    );
    assert!(
        !sexp.contains("(ambiguous_function_call_expression (function) (binary"),
        "do_thing bare call swallowed low-precedence operator: {sexp}"
    );
}

#[test]
fn bare_call_scalar_arg_keeps_word_and_outside_call() {
    let sexp = parse_ok("do_thing $x and return;");
    assert!(
        sexp.contains("(ambiguous_function_call_expression"),
        "expected bare unary-style call node in AST, got: {sexp}"
    );
    assert!(
        !sexp.contains("(ambiguous_function_call_expression (function) (binary"),
        "do_thing bare call swallowed low-precedence operator: {sexp}"
    );
}

#[test]
fn bare_call_scalar_arg_keeps_defined_or_outside_call() {
    let sexp = parse_ok("my $v = transform $x // $fallback;");
    assert!(
        sexp.contains("(ambiguous_function_call_expression"),
        "expected bare unary-style call node in AST, got: {sexp}"
    );
    assert!(
        !sexp.contains("(ambiguous_function_call_expression (function) (binary"),
        "transform bare call swallowed low-precedence operator: {sexp}"
    );
}

#[test]
fn nested_bare_call_ternary_in_larger_expression() {
    let sexp = parse_ok("my $n = (is_ready $obj ? 1 : 0) + 1;");
    assert!(sexp.contains("(ternary"), "expected ternary in AST, got: {sexp}");
    assert!(
        sexp.contains("(ambiguous_function_call_expression (function) (variable $ obj))"),
        "expected bare unary-style call node in AST, got: {sexp}"
    );
    assert!(
        !sexp.contains("(ambiguous_function_call_expression (function) (ternary"),
        "is_ready bare call swallowed ternary inside larger expression: {sexp}"
    );
}
