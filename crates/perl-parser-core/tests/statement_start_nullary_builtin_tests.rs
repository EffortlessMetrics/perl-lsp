use perl_parser_core::Parser;
use perl_tdd_support::must;

fn parse_ok(src: &str) -> String {
    let mut parser = Parser::new(src);
    let ast = must(parser.parse());
    let sexp = ast.to_sexp();
    assert!(
        !sexp.contains("ERROR"),
        "parse should succeed without errors for: {src}\ngot: {sexp}"
    );
    sexp
}

#[test]
fn shift_statement_start_with_explicit_array_arg() {
    let sexp = parse_ok("shift @arr;");
    assert!(
        sexp.contains("(call shift ((variable @ arr)))"),
        "expected shift call with @arr arg, got: {sexp}"
    );
}

#[test]
fn shift_statement_start_method_chain_uses_nullary_call() {
    let sexp = parse_ok("shift->decode(@_);");
    assert!(
        sexp.contains("(method_call (identifier shift) decode"),
        "expected shift postfix method chain, got: {sexp}"
    );
    assert!(
        !sexp.contains("ERROR"),
        "shift postfix method chain should stay error-free: {sexp}"
    );
}

#[test]
fn shift_statement_start_arrow_hash_deref_uses_nullary_call() {
    let sexp = parse_ok("shift->{Name};");
    assert!(
        sexp.contains("(arrow_hash_deref (identifier shift)"),
        "expected shift postfix hash deref, got: {sexp}"
    );
    assert!(
        !sexp.contains("ERROR"),
        "shift postfix hash deref should stay error-free: {sexp}"
    );
}

#[test]
fn shift_statement_start_logical_or_stays_outside_call() {
    let sexp = parse_ok("shift @arr || die 'x';");
    assert!(
        sexp.contains("(binary_||"),
        "expected logical-or root, got: {sexp}"
    );
    assert!(
        sexp.contains("(call shift ((variable @ arr)))"),
        "expected shift call on left-hand side, got: {sexp}"
    );
    assert!(
        !sexp.contains("(call shift ((binary_||"),
        "shift swallowed the logical-or expression: {sexp}"
    );
}

#[test]
fn caller_statement_start_eq_comparison_stays_outside_call() {
    let sexp = parse_ok("caller 1 eq 'main';");
    assert!(
        sexp.contains("(binary_eq"),
        "expected binary_eq root, got: {sexp}"
    );
    assert!(
        sexp.contains("(ambiguous_function_call_expression (function) (number 1))"),
        "expected caller argument to stay inside the call, got: {sexp}"
    );
    assert!(
        !sexp.contains("(function_call_expression (function)) (identifier eq)"),
        "caller split into multiple statements: {sexp}"
    );
}

#[test]
fn caller_statement_start_logical_or_stays_outside_call() {
    let sexp = parse_ok("caller 1 || die 'x';");
    assert!(
        sexp.contains("(binary_||"),
        "expected logical-or root, got: {sexp}"
    );
    assert!(
        sexp.contains("(ambiguous_function_call_expression (function) (number 1))"),
        "expected caller argument to stay inside the call, got: {sexp}"
    );
    assert!(
        !sexp.contains("(function_call_expression (function)) (binary_||"),
        "caller still split before the logical-or: {sexp}"
    );
}

#[test]
fn localtime_statement_start_keeps_additive_expression_inside_arg() {
    let sexp = parse_ok("localtime 0 + 1;");
    assert!(
        sexp.contains(
            "(ambiguous_function_call_expression (function) (binary_+ (number 0) (number 1)))"
        ),
        "expected localtime to consume the additive expression as its single arg, got: {sexp}"
    );
}

#[test]
fn time_statement_start_division_stays_outside_call() {
    let sexp = parse_ok("time / 60;");
    assert!(
        sexp.contains("(binary_/"),
        "expected division root, got: {sexp}"
    );
}
