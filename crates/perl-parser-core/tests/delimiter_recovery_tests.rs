//! Tests for improved delimiter recovery (#1649).

mod cpan_test_helpers;
use cpan_test_helpers::*;
use perl_parser_core::{NodeKind, Parser};
use perl_tdd_support::must;

fn parse_with_errors(src: &str) -> (perl_parser_core::Node, usize) {
    let mut parser = Parser::new(src);
    let ast = must(parser.parse());
    let n = parser.errors().len();
    (ast, n)
}

fn statement_count(ast: &perl_parser_core::Node) -> usize {
    match &ast.kind {
        NodeKind::Program { statements } => statements.len(),
        _ => 0,
    }
}

#[test]
fn unclosed_paren_does_not_cascade_to_next_statement() {
    let code = "my $x = (1 + 2; print 42;";
    let (ast, _) = parse_with_errors(code);
    let count = statement_count(&ast);
    assert!(
        count >= 2,
        "Unclosed paren should not swallow following statement. Got {} stmts",
        count,
    );
}

#[test]
fn unclosed_paren_in_if_condition_recovers() {
    let code = "if ($x == 1 { print 1; } print 2;";
    let (ast, errs) = parse_with_errors(code);
    assert!(errs > 0);
    let count = statement_count(&ast);
    assert!(count >= 2, "Missing ) in if should not prevent rest. Got {} stmts", count,);
}

#[test]
fn unclosed_bracket_does_not_cascade() {
    let code = "my @x = [1, 2, 3; my $y = 42;";
    let (ast, _) = parse_with_errors(code);
    let count = statement_count(&ast);
    assert!(count >= 2, "Unclosed bracket should not swallow next stmt. Got {} stmts", count,);
}

#[test]
fn extra_closing_paren_does_not_cascade() {
    let code = "my $x = 1); my $y = 2;";
    let (ast, _) = parse_with_errors(code);
    let count = statement_count(&ast);
    assert!(count >= 2, "Extra ) should not prevent parsing remaining code. Got {} stmts", count,);
}

#[test]
fn unclosed_paren_produces_bounded_errors() {
    let code = "my $x = (1 + 2;\nmy $y = 3;\nmy $z = 4;\nprint $y;\n";
    let (ast, error_count) = parse_with_errors(code);
    assert!(error_count <= 4, "Unclosed paren should produce bounded errors, got {}", error_count);
    let count = statement_count(&ast);
    assert!(count >= 3, "Should parse most stmts. Got {}", count);
}

#[test]
fn unclosed_paren_in_function_call_recovers() {
    let code = "foo(1, 2; bar(3);";
    let (ast, errs) = parse_with_errors(code);
    assert!(errs > 0);
    let count = statement_count(&ast);
    assert!(count >= 2, "Unclosed paren in foo() should not prevent bar(). Got {} stmts", count,);
}

#[test]
fn nested_unclosed_paren_recovers() {
    let code = "my $x = ((1 + 2); my $y = 3;";
    let (ast, _) = parse_with_errors(code);
    let count = statement_count(&ast);
    assert!(count >= 2, "Nested unclosed paren should allow next stmt. Got {} stmts", count,);
}

#[test]
fn clean_paren_expression_still_works() {
    assert_clean_parse("my $x = (1 + 2);");
}
#[test]
fn clean_bracket_literal_still_works() {
    assert_clean_parse("my @x = [1, 2, 3];");
}
#[test]
fn clean_if_condition_still_works() {
    assert_clean_parse("if ($x == 1) { print 1; }");
}
#[test]
fn clean_nested_parens_still_works() {
    assert_clean_parse("my $x = ((1 + 2) * 3);");
}
#[test]
fn clean_while_condition_still_works() {
    assert_clean_parse("while ($i < 10) { $i = $i + 1; }");
}
#[test]
fn clean_for_loop_still_works() {
    assert_clean_parse("for my $i (1..10) { print $i; }");
}

#[test]
fn semicolon_inside_parens_triggers_recovery() {
    let code = "my $x = ($a + $b; my $y = $c;";
    let (ast, errs) = parse_with_errors(code);
    assert!(errs > 0);
    let count = statement_count(&ast);
    assert!(count >= 2, "Semicolon should trigger recovery. Got {} stmts", count,);
}

#[test]
fn while_missing_close_paren_recovers() {
    let code = "while ($x < 10 { $x = $x + 1; } print 'done';";
    let (ast, errs) = parse_with_errors(code);
    assert!(errs > 0);
    let count = statement_count(&ast);
    assert!(count >= 2, "Missing ) in while should not prevent rest. Got {} stmts", count,);
}
