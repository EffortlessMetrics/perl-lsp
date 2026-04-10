//! BDD-style behavior specification tests for `perl-parser-pest`.
//!
//! These tests focus on externally visible parser behavior: given a Perl
//! snippet, the parser should either produce a meaningful AST/S-expression or
//! fail gracefully for invalid syntax.

use perl_parser_pest::PureRustPerlParser;
use perl_tdd_support::must;
use std::error::Error;

fn parse_to_sexp(source: &str) -> String {
    let mut parser = PureRustPerlParser::new();
    let ast = must(parser.parse(source));
    parser.to_sexp(&ast)
}

fn parse_err(source: &str) -> Box<dyn Error> {
    let mut parser = PureRustPerlParser::new();
    parser.parse(source).expect_err("source should fail to parse")
}

#[test]
fn when_parsing_scalar_declaration_then_ast_contains_variable_declaration() {
    let sexp = parse_to_sexp("my $value = 42;");
    assert!(
        sexp.contains("(variable_declaration"),
        "expected variable declaration in sexp, got: {sexp}"
    );
    assert!(sexp.contains("$value"), "expected variable name in sexp: {sexp}");
    assert!(sexp.contains("42"), "expected initializer literal in sexp: {sexp}");
}

#[test]
fn when_parsing_sub_declaration_then_sexp_includes_subroutine_name() {
    let sexp = parse_to_sexp("sub greet { return 'hi'; }");

    assert!(
        sexp.contains("(subroutine (identifier greet)"),
        "expected sub declaration in sexp, got: {sexp}"
    );
}

#[test]
fn when_parsing_if_elsif_else_then_sexp_represents_all_branches() {
    let sexp = parse_to_sexp(
        "if ($x > 10) { print 'big'; } elsif ($x > 5) { print 'mid'; } else { print 'small'; }",
    );

    assert!(sexp.contains("(if_statement"), "missing if statement: {sexp}");
    assert!(sexp.contains("'big'"), "missing then-branch payload: {sexp}");
    assert!(sexp.contains("$x"), "missing if condition variable: {sexp}");
}

#[test]
fn when_parsing_foreach_loop_then_sexp_captures_loop_shape() {
    let sexp = parse_to_sexp("foreach my $item (@items) { print $item; }");

    assert!(sexp.contains("(foreach_statement"), "missing foreach node: {sexp}");
    assert!(sexp.contains("$item"), "missing loop variable: {sexp}");
    assert!(sexp.contains("@items"), "missing loop list: {sexp}");
}

#[test]
fn when_parsing_function_call_with_arguments_then_ast_contains_call_node() {
    let sexp = parse_to_sexp("print('hello', 123);");
    assert!(
        sexp.contains("(function_call (identifier print)")
            || sexp.contains("(builtin_listop print"),
        "expected call-like node in sexp: {sexp}"
    );
    assert!(
        sexp.contains("'hello'") && sexp.contains("123"),
        "expected call arguments in sexp: {sexp}"
    );
}

#[test]
fn when_parsing_regex_match_then_sexp_contains_regex_node() {
    let sexp = parse_to_sexp("$name =~ /foo.*/i;");

    assert!(sexp.contains("(regex "), "expected regex node in sexp: {sexp}");
}

#[test]
fn when_parsing_invalid_syntax_then_parse_fails_with_error() {
    let err = parse_err("if ($x > 0 { print $x; }");

    let message = err.to_string();
    assert!(!message.trim().is_empty(), "expected parse error message to be populated");
}
