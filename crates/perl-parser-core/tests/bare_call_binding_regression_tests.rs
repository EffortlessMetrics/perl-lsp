use perl_parser_core::Parser;
use perl_tdd_support::must;

fn parse_ok(src: &str) -> String {
    let mut parser = Parser::new(src);
    let ast = must(parser.parse());
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("ERROR"), "parse should succeed without errors for: {src}\ngot: {sexp}");
    sexp
}

#[test]
fn bare_call_sigil_arg_does_not_absorb_ternary() {
    let sexp = parse_ok("sub is_ready { 1 }; my $x = is_ready $obj ? 1 : 0;");
    assert!(sexp.contains("(ternary"), "expected ternary expression, got: {sexp}");
    assert!(
        !sexp.contains("ambiguous_function_call_expression (function) (ternary")
            && !sexp.contains("(call is_ready ((ternary"),
        "bare call greedily absorbed ternary: {sexp}"
    );
}

#[test]
fn bare_call_sigil_arg_does_not_absorb_word_or() {
    let sexp = parse_ok("do_thing @args or die;");
    assert!(sexp.contains("(binary_or"), "expected top-level word-or binding, got: {sexp}");
    assert!(
        !sexp.contains("(call do_thing ((binary_or")
            && !sexp.contains("ambiguous_function_call_expression (function) (binary_or"),
        "bare call greedily absorbed word-or: {sexp}"
    );
}

#[test]
fn bare_call_sigil_arg_does_not_absorb_word_and() {
    let sexp = parse_ok("do_thing $x and return;");
    assert!(sexp.contains("(binary_and"), "expected top-level word-and binding, got: {sexp}");
    assert!(
        !sexp.contains("(call do_thing ((binary_and")
            && !sexp.contains("ambiguous_function_call_expression (function) (binary_and"),
        "bare call greedily absorbed word-and: {sexp}"
    );
}

#[test]
fn bare_call_sigil_arg_does_not_absorb_defined_or() {
    let sexp = parse_ok("my $v = transform $x // $fallback;");
    assert!(
        sexp.contains("binary_//") || sexp.contains("binary_defined_or"),
        "expected defined-or expression, got: {sexp}"
    );
    assert!(
        !sexp.contains("(call transform ((binary_//")
            && !sexp.contains("(call transform ((binary_defined_or")
            && !sexp.contains("ambiguous_function_call_expression (function) (binary_//")
            && !sexp.contains("ambiguous_function_call_expression (function) (binary_defined_or"),
        "bare call greedily absorbed defined-or: {sexp}"
    );
}

#[test]
fn nested_bare_call_condition_still_allows_outer_ternary() {
    let sexp = parse_ok("my $y = (is_ready $obj ? 1 : 0) + 2;");
    assert!(sexp.contains("(ternary"), "expected nested ternary, got: {sexp}");
    assert!(sexp.contains("(binary_+"), "expected outer addition expression, got: {sexp}");
    assert!(
        !sexp.contains("(call is_ready ((ternary"),
        "nested bare call greedily absorbed ternary: {sexp}"
    );
}
