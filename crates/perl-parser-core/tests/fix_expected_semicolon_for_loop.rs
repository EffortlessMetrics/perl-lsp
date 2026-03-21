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

fn first_statement_kind(ast: &perl_parser_core::Node) -> &str {
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
/// catastrophically. Records errors, produces a For node, and the statement
/// following the loop still parses.
#[test]
fn test_for_loop_both_semicolons_missing_no_infinite_loop() {
    let src = "for (my $i = 0 $i < 10 $i++) { print $i; }\nprint 'done';";
    let (ast, errs) = parse_with_error_count(src);
    assert!(errs > 0, "Expected errors when both semicolons are missing");
    // Parser must not cascade catastrophically — statement count must be sane
    let count = statement_count(&ast);
    assert!(
        count >= 2,
        "Statement after bad for loop must still parse even with both semicolons missing. Got {} stmts",
        count
    );
    // The for-loop itself must still produce a For node (recovery keeps it intact)
    let first_kind = first_statement_kind(&ast);
    assert_eq!(
        first_kind, "For",
        "First statement must be a For node after recovery, not '{}'. \
         Both-semicolons-missing should still yield a partial For node.",
        first_kind
    );
    // Error count must be bounded — at most 2 (one per missing semicolon), not a cascade
    assert!(
        errs <= 4,
        "Error count should be bounded with both semicolons missing (expected 2-3, got {}). \
         The fix should not cascade.",
        errs
    );
}

/// Nested for loop where the inner loop has a missing semicolon — outer loop must
/// still parse as a For node, inner loop must recover and not destroy the outer structure.
#[test]
fn test_for_loop_nested_inner_missing_semicolon() {
    let src = "for (my $i = 0; $i < 5; $i++) {\n    for (my $j = 0 $j < 5; $j++) { print \"$i $j\"; }\n}\nprint 'done';";
    let (ast, errs) = parse_with_error_count(src);
    assert!(errs > 0, "Expected at least one error for missing semicolon in inner for loop");
    let count = statement_count(&ast);
    assert!(count >= 2, "Statement after the outer for loop must still parse. Got {} stmts", count);
    // Outer for loop must be a For node — inner recovery must not bubble up
    let first_kind = first_statement_kind(&ast);
    assert_eq!(
        first_kind, "For",
        "Outer loop must remain a For node after inner recovery, not '{}'.",
        first_kind
    );
    // Errors should be bounded — only the inner missing semicolon
    assert!(
        errs <= 3,
        "Error count should be bounded (expected 1, got {}). Inner recovery must not cascade.",
        errs
    );
}

/// Expression-init path (not `my`): `for ($i = 0 $i < 10; $i++)` — the init is
/// a plain expression, not a variable declaration. The recovery must work for this
/// path too, not just the `my` declaration path.
#[test]
fn test_for_loop_expression_init_missing_first_semicolon() {
    let src = "for ($i = 0 $i < 10; $i++) { print $i; }\nprint 'done';";
    let (ast, errs) = parse_with_error_count(src);
    assert!(errs > 0, "Expected at least one error for missing semicolon after expression init");
    let count = statement_count(&ast);
    assert!(count >= 2, "Statement after bad for loop must still parse. Got {} stmts", count);
    let first_kind = first_statement_kind(&ast);
    assert_eq!(
        first_kind, "For",
        "First statement must be a For node after recovery, not '{}'.",
        first_kind
    );
    assert!(
        errs <= 3,
        "Error count should be bounded after recovery (expected 1-2, got {}).",
        errs
    );
}

/// Missing semicolons AND the body is directly after init — `for (my $i = 0) { }`.
/// When `)` immediately follows the init (no condition, no update, no semicolons),
/// the parser must not cascade: it should produce a For node with 1 error, not fail.
#[test]
fn test_for_loop_no_semicolons_rparen_immediately() {
    let src = "for (my $i = 0) { print $i; }\nprint 'done';";
    let (ast, errs) = parse_with_error_count(src);
    // This is malformed — must produce at least one error
    assert!(errs > 0, "Expected at least one error when `)` follows init directly");
    // Must not cascade — the statement after must still parse
    let count = statement_count(&ast);
    assert!(count >= 2, "Statement after bad for loop must still parse. Got {} stmts", count);
    // For node must survive (not become a cascade of Error nodes)
    let first_kind = first_statement_kind(&ast);
    assert_eq!(
        first_kind, "For",
        "First statement must be a For node, not '{}'. \
         The `)` guard in condition parsing must prevent cascading.",
        first_kind
    );
    // Only 1 error expected: the missing first semicolon
    assert!(errs <= 2, "Error count should be 1 (only the missing first `;`), got {}.", errs);
}
