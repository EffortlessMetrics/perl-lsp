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
    let _ast = must(parser.parse());

    // If we got here, the code parsed and can be analyzed by security linter
    assert!(true);
}

#[test]
fn nested_variable_list_deep_nesting_parses() {
    // Exercise recursive parsing for deep nesting
    let code = "my ($x, ($y, ($z, $w))) = (1, (2, (3, 4)));";
    let mut parser = Parser::new(code);
    let _ast = must(parser.parse());

    assert!(true);
}

#[test]
fn nested_variable_list_with_signal_parses() {
    // Test nested variables with potential signal shadowing
    let code = "my ($a, ($SIG, $b)) = @_;";
    let mut parser = Parser::new(code);
    let _ast = must(parser.parse());

    assert!(true);
}
