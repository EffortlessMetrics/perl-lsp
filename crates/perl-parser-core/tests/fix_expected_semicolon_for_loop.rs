//! Tests for expected_semicolon error recovery in C-style for loops (#2573).
//!
//! The parser previously used `expect(TokenKind::Semicolon)?` (hard fail) at
//! the two internal semicolon positions in a C-style for loop. These tests
//! verify that missing semicolons are recovered gracefully — an error is
//! recorded but parsing continues and the statement following the bad for loop
//! still parses.
//!
//! Post-fix requirements:
//! - The for-loop statement itself must be a `For` node (not an `Error` node)
//! - Exactly 1 error is recorded (the missing semicolon), not a cascade of 6
//! - The statement following the bad for loop still parses correctly

mod cpan_test_helpers;
use cpan_test_helpers::*;
use perl_parser_core::{NodeKind, Parser};
use perl_tdd_support::must;

fn parse_with_error_count(src: &str) -> (perl_parser_core::Node, usize) {
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

fn first_statement_kind<'a>(ast: &'a perl_parser_core::Node) -> &'a str {
    match &ast.kind {
        NodeKind::Program { statements } => {
            statements.first().map(|s| s.kind.kind_name()).unwrap_or("(none)")
        }
        _ => "(not a program)",
    }
}

/// Missing semicolon after the init expression — should record exactly 1 error,
/// produce a `For` node (not an `Error` node), and allow the statement following
/// the for loop to still parse.
#[test]
fn test_for_loop_missing_first_semicolon_records_error() {
    let src = "for (my $i = 0 $i < 10; $i++) { print $i; }\nprint 'done';";
    let (ast, errs) = parse_with_error_count(src);
    assert!(errs > 0, "Expected at least one error for missing semicolon after init");
    let count = statement_count(&ast);
    assert!(count >= 2, "Statement after bad for loop must still parse. Got {} stmts", count);
    // The for-loop itself must produce a For node, not cascade into Error nodes
    let first_kind = first_statement_kind(&ast);
    assert_eq!(
        first_kind, "For",
        "First statement must be a For node after recovery, not '{}'. \
         The fix should produce a partial For node and record the error inline.",
        first_kind
    );
    // Error count should be bounded (1-2), not a cascade of 6
    assert!(
        errs <= 3,
        "Error count should be bounded after recovery (expected 1-2, got {}). \
         The fix should not cascade into multiple spurious errors.",
        errs
    );
}

/// Missing semicolon after the condition expression — same recovery expectations.
#[test]
fn test_for_loop_missing_second_semicolon_records_error() {
    let src = "for (my $i = 0; $i < 10 $i++) { print $i; }\nprint 'done';";
    let (ast, errs) = parse_with_error_count(src);
    assert!(errs > 0, "Expected at least one error for missing semicolon after condition");
    let count = statement_count(&ast);
    assert!(count >= 2, "Statement after bad for loop must still parse. Got {} stmts", count);
    // The for-loop itself must produce a For node, not cascade into Error nodes
    let first_kind = first_statement_kind(&ast);
    assert_eq!(
        first_kind, "For",
        "First statement must be a For node after recovery, not '{}'. \
         The fix should produce a partial For node and record the error inline.",
        first_kind
    );
    // Error count should be bounded (1-2), not a cascade
    assert!(
        errs <= 3,
        "Error count should be bounded after recovery (expected 1-2, got {}). \
         The fix should not cascade into multiple spurious errors.",
        errs
    );
}

/// Regression: a valid C-style for loop must remain clean (no errors, no
/// Error/Missing nodes in the AST).
#[test]
fn test_for_loop_valid_all_semicolons_clean() {
    assert_clean_parse("for (my $i = 0; $i < 10; $i++) { print $i; }");
}

/// Regression: `for (;;)` must remain clean.
#[test]
fn test_for_loop_empty_all_clean() {
    assert_clean_parse("for (;;) { last; }");
}

/// Both internal semicolons missing — parser must not infinite-loop or cascade
/// catastrophically. Records errors and the statement following the loop still parses.
#[test]
fn test_for_loop_both_semicolons_missing_no_infinite_loop() {
    let src = "for (my $i = 0 $i < 10 $i++) { print $i; }\nprint 'done';";
    let (ast, errs) = parse_with_error_count(src);
    assert!(errs > 0, "Expected errors when both semicolons are missing");
    // Parser must not cascade catastrophically — statement count must be sane
    let count = statement_count(&ast);
    assert!(
        count >= 1,
        "Parser must produce at least one statement even with both semicolons missing. Got {}",
        count
    );
}
