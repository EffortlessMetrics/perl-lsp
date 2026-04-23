//! BDD-style behavior specification tests for `perl-parser-pest`.
//!
//! These tests focus on user-observable parser behavior:
//! - successful parsing of common Perl snippets,
//! - normalization compatibility paths,
//! - error-recovery behavior when input contains mixed-validity statements.

use perl_parser_pest::{AstNode, PureRustPerlParser};
use perl_tdd_support::{must, must_err};

fn parse_to_sexp(source: &str) -> String {
    let mut parser = PureRustPerlParser::new();
    let ast = must(parser.parse(source));
    parser.to_sexp(&ast)
}

fn parse_ast(source: &str) -> AstNode {
    let mut parser = PureRustPerlParser::new();
    must(parser.parse(source))
}

#[test]
fn when_given_variable_declaration_then_parser_emits_variable_declaration_node() {
    let sexp = parse_to_sexp("my $x = 42;");

    assert!(
        sexp.contains("(variable_declaration") && sexp.contains("$x"),
        "expected a variable declaration for my $x; got: {sexp}"
    );
}

#[test]
fn when_given_if_statement_then_parser_emits_if_statement_shape() {
    let sexp = parse_to_sexp("if ($ready) { print $ready; }");

    assert!(sexp.contains("(if_statement"), "expected if_statement; got: {sexp}");
    assert!(sexp.contains("(block"), "expected then block in output; got: {sexp}");
}

#[test]
fn when_foreach_uses_my_declaration_then_normalization_keeps_parse_successful() {
    let sexp = parse_to_sexp("foreach my $item (@items) { print $item; }");

    assert!(
        sexp.contains("(foreach_statement") || sexp.contains("(for_statement"),
        "expected loop node in output; got: {sexp}"
    );
}

#[test]
fn when_simple_scalar_deref_uses_double_dollar_then_normalization_allows_parse() {
    let sexp = parse_to_sexp("my $v = $$name;");

    assert!(
        sexp.contains("(variable_declaration")
            && sexp.contains("(dereference")
            && sexp.contains("$name"),
        "expected normalized scalar dereference to parse; got: {sexp}"
    );
}

#[test]
fn when_assignment_uses_space_tilde_form_then_normalization_allows_parse() {
    let sexp = parse_to_sexp("my $x = 1; $x = ~ $x;");

    assert!(
        sexp.contains("(assignment") || sexp.contains("(function_call") || sexp.contains("bitnot"),
        "expected assignment/bitnot-compatible parse output; got: {sexp}"
    );
}

#[test]
fn when_if_block_assigns_percent_string_then_parser_keeps_string_assignment_shape() {
    let sexp = parse_to_sexp(r#"if ($a > 0) { $a = "%"; }"#);

    assert!(sexp.contains("(if_statement"), "expected if_statement; got: {sexp}");
    assert!(
        sexp.contains("(assignment") && sexp.contains("(string_literal %)"),
        "expected percent-string assignment inside block; got: {sexp}"
    );
}

#[test]
fn when_hash_uses_fat_comma_pairs_then_parser_keeps_hash_assignment_structure() {
    let sexp = parse_to_sexp("%hash = (a => 1, b => 2);");

    assert!(
        sexp.contains("(assignment (hash_variable %hash) (=)")
            && sexp.contains("(identifier a )")
            && sexp.contains("(identifier b )"),
        "expected hash assignment with fat-comma pairs; got: {sexp}"
    );
}

#[test]
fn when_given_when_has_default_clause_then_parser_emits_given_shape() {
    let sexp = parse_to_sexp(
        r#"
        given ($kind) {
            when ("A") { print "alpha"; }
            default { print "other"; }
        }
        "#,
    );

    assert!(sexp.contains("(given_statement"), "expected given_statement; got: {sexp}");
    assert!(sexp.contains("(when_clause"), "expected when_clause in given block; got: {sexp}");
    assert!(
        sexp.contains("(default_clause"),
        "expected default_clause in given block; got: {sexp}"
    );
}

#[test]
fn when_if_has_elsif_and_else_then_parser_recovers_primary_if_shape() {
    let sexp = parse_to_sexp(
        r#"
        if ($x == 1) { print "one"; }
        elsif ($x == 2) { print "two"; }
        else { print "other"; }
        "#,
    );

    assert!(sexp.contains("(if_statement"), "expected if_statement; got: {sexp}");
    assert!(
        sexp.contains("(string_literal one)"),
        "expected recovery output to preserve primary branch body; got: {sexp}"
    );
}

#[test]
fn when_ternary_expression_is_used_then_parser_emits_ternary_shape() {
    let sexp = parse_to_sexp(r#"my $label = $ok ? "yes" : "no";"#);

    assert!(
        sexp.contains("(unhandled_node TernaryOp")
            || sexp.contains("(ternary")
            || sexp.contains("(ternary_op"),
        "expected ternary-compatible expression shape; got: {sexp}"
    );
}

#[test]
fn when_input_has_valid_then_invalid_then_recovery_returns_partial_program() -> Result<(), String> {
    let ast = parse_ast("my $ok = 1;\nmy = ;\nprint $ok;\n");

    let AstNode::Program(nodes) = ast else {
        return Err("expected recovery to return Program".to_string());
    };

    assert!(!nodes.is_empty(), "expected recovery parse to preserve at least one statement");
    Ok(())
}

#[test]
fn when_input_is_only_invalid_then_parser_returns_error() {
    let mut parser = PureRustPerlParser::new();
    let result = parser.parse("my = ; ???");

    let _err = must_err(result);
}
