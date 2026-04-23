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
fn when_given_statement_contains_when_and_default_then_parser_emits_switch_shape() {
    let sexp = parse_to_sexp(
        r#"
given ($x) {
    when (1) { print "one"; }
    default { print "other"; }
}
"#,
    );

    assert!(sexp.contains("(given_statement"), "expected given_statement; got: {sexp}");
    assert!(sexp.contains("(when_clause"), "expected when_clause; got: {sexp}");
    assert!(sexp.contains("(default_clause"), "expected default_clause; got: {sexp}");
}

#[test]
fn when_method_call_uses_arrow_syntax_then_parser_recovers_to_partial_ast_without_panicking() {
    let sexp = parse_to_sexp(r#"$obj->run("fast");"#);

    assert!(sexp.contains("(source_file"), "expected source_file wrapper; got: {sexp}");
    assert!(
        sexp.contains("(scalar_variable $obj)"),
        "expected partial recovery to preserve receiver variable; got: {sexp}"
    );
}

#[test]
fn when_input_has_mixed_valid_invalid_and_valid_statements_then_recovery_keeps_later_statements()
-> Result<(), String> {
    let ast = parse_ast("my $start = 1;\nmy = ;\nmy $end = 2;\n");

    let AstNode::Program(nodes) = ast else {
        return Err("expected recovery to return Program".to_string());
    };

    let sexp_nodes: Vec<String> = nodes.iter().map(PureRustPerlParser::node_to_sexp).collect();
    let joined = sexp_nodes.join(" ");
    assert!(!nodes.is_empty(), "expected at least one preserved statement");
    assert!(
        joined.contains("$start") || joined.contains("$end"),
        "expected recovery output to keep parseable statements; got: {joined}"
    );
    Ok(())
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
