//! Regression tests for barewords before `=>` in unparenthesized calls.
//!
//! `signature_for execute => (...)` is valid Perl: the fat comma auto-quotes
//! `execute`. The parser previously left it as an Identifier node, which made
//! the semantic analyzer emit a false-positive `unquoted-bareword` diagnostic.

use perl_parser_core::Parser;
use perl_tdd_support::must;

fn parse_sexp(src: &str) -> String {
    let mut parser = Parser::new(src);
    let ast = must(parser.parse());
    ast.to_sexp()
}

#[test]
fn bare_call_fat_arrow_first_arg_is_autoquoted() {
    let sexp = parse_sexp("signature_for execute => ( method => 1 );");
    assert!(sexp.contains("(string \"execute\")"), "fat-arrow key must be autoquoted, got: {sexp}");
    assert!(
        !sexp.contains("(identifier execute)"),
        "fat-arrow key must not remain an identifier, got: {sexp}"
    );
}

#[test]
fn bare_call_fat_arrow_between_args_is_autoquoted() {
    let sexp = parse_sexp("register thing => 1, other => 2;");
    assert!(sexp.contains("(string \"thing\")"), "first key must be autoquoted, got: {sexp}");
    assert!(sexp.contains("(string \"other\")"), "second key must be autoquoted, got: {sexp}");
}
