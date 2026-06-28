//! Tests for nested variable list security analysis.
//!
//! Exercises the security linter path for nested variable lists
//! in lexical declarations.

use perl_semantic_analyzer::Parser;
use perl_tdd_support::must;

#[test]
fn nested_variable_list_parses_for_security_analysis() {
    // This exercises the NestedVariableList path in security analysis
    // by ensuring the code parses correctly (the security analyzer
    // runs on the AST, not directly via the test)
    let code = "my ($a, ($b, $c)) = (1, (2, 3));";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let sexp = ast.to_sexp();

    assert!(sexp.contains("nested_variable_list"), "expected nested variable list: {sexp}");
}

#[test]
fn nested_variable_list_deep_nesting_parses() {
    // Exercise recursive parsing for deep nesting
    let code = "my ($x, ($y, ($z, $w))) = (1, (2, (3, 4)));";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let sexp = ast.to_sexp();

    assert!(
        sexp.matches("nested_variable_list").count() >= 2,
        "expected recursive nested variable lists: {sexp}"
    );
}

#[test]
fn nested_variable_list_with_signal_parses() {
    // Test nested variables with potential signal shadowing
    let code = "my ($a, ($SIG, $b)) = @_;";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let sexp = ast.to_sexp();

    assert!(sexp.contains("nested_variable_list"), "expected nested variable list: {sexp}");
    assert!(sexp.contains("SIG"), "expected signal variable to remain visible: {sexp}");
}
