//! Edge-case tests for PR #1456 (#1352 recovery fix) — reviewer-deep additions.
//!
//! These cover scenarios the builder's 12-test grid did not explicitly exercise:
//!   E1. Empty unclosed hash `{ ; sub foo {}` — nothing between { and ;
//!   E2. Unclosed hash at true EOF (no trailing ; or sub)
//!   E3. Nested unclosed inside a valid sub body (sub body uses parse_block, not this path)
//!   E4. Multiple key=>val pairs then semicolon — `{ a=>1, b=>2; sub foo {}`
//!   E5. `{ }` (empty braces) — healthy case, must remain a hash, 0 errors
//!   E6. `{ a => 1 }` (closed, single-pair hash) — no errors
//!   E7. Back-to-back hash recovery: two consecutive unclosed hashes, then a sub
//!   E8. Unclosed after heredoc context (heredoc parsing doesn't interfere)

mod cpan_test_helpers;
use cpan_test_helpers::*;
use perl_parser_core::{NodeKind, Parser};
use perl_tdd_support::must;

fn statement_count(src: &str) -> usize {
    let mut parser = Parser::new(src);
    let ast = must(parser.parse());
    match &ast.kind {
        NodeKind::Program { statements } => statements.len(),
        _ => 0,
    }
}

fn error_count(src: &str) -> usize {
    let mut parser = Parser::new(src);
    let _ast = must(parser.parse());
    parser.get_errors().len()
}

fn has_subroutine(src: &str, expected_name: &str) -> bool {
    let mut parser = Parser::new(src);
    let ast = must(parser.parse());
    fn find_sub(node: &perl_parser_core::Node, name: &str) -> bool {
        match &node.kind {
            NodeKind::Program { statements } => statements.iter().any(|s| find_sub(s, name)),
            NodeKind::Subroutine { name: n, .. } => n.as_deref() == Some(name),
            _ => false,
        }
    }
    find_sub(&ast, expected_name)
}

/// E1: `{ ; sub foo {}` — nothing between outer `{` and the boundary `;`.
/// parse_expression() fails immediately (`;` is not an expression start).
/// Falls into the Err branch, is_delimiter_recovery_point() true → guard fires.
/// sub foo must be recovered as separate statement.
#[test]
fn e1_empty_unclosed_hash_with_semicolon() {
    let src = "my $x = {; sub foo {}";
    let has_foo = has_subroutine(src, "foo");
    assert!(has_foo, "E1: sub after `{{;` must be recovered");
    let errs = error_count(src);
    assert!(errs >= 1, "E1: must record at least one error");
}

/// E2: `my $x = { a => 1` — unclosed hash at true EOF, no trailing semicolon.
/// Recovery must not hang or panic; Program root is still produced.
#[test]
fn e2_unclosed_hash_at_eof_no_semicolon() {
    let src = "my $x = { a => 1";
    let count = statement_count(src);
    assert!(count >= 1, "E2: EOF unclosed hash must produce >=1 statements, got {}", count);
    let errs = error_count(src);
    assert!(errs >= 1, "E2: EOF unclosed hash must record an error");
}

/// E3: Unclosed bracket inside a valid named sub body — uses parse_block(), not this path.
/// Belt-and-suspenders: confirms sub scope is unaffected by the fix.
#[test]
fn e3_unclosed_inside_named_sub_body() {
    let src = "sub broken { my $x = [1, 2, 3 } sub clean {}";
    let has_clean = has_subroutine(src, "clean");
    assert!(has_clean, "E3: sub after error in sub body must be recovered");
    let errs = error_count(src);
    assert!(errs >= 1, "E3: unclosed bracket inside sub must record an error");
}

/// E4: Multiple key=>val pairs then semicolon: `my $x = { a=>1, b=>2; sub foo {}`.
/// parse_expression() returns HashLiteral (from `a=>1, b=>2`), peek `;`.
/// unclosed_hash guard fires. sub foo must be recovered.
#[test]
fn e4_multi_pair_unclosed_hash_recovery() {
    let src = "my $x = { a => 1, b => 2; sub foo {}";
    let has_foo = has_subroutine(src, "foo");
    assert!(has_foo, "E4: sub after multi-pair unclosed hash must be recovered");
    let errs = error_count(src);
    assert!(errs >= 1, "E4: multi-pair unclosed hash must record an error");
}

/// E5: Empty braces `{}` — must parse as empty HashLiteral, 0 errors.
/// This takes the early-exit empty path (before any fix code runs).
#[test]
fn e5_empty_braces_healthy() {
    let src = "my %h = {};";
    let errs = error_count(src);
    assert_eq!(errs, 0, "E5: empty braces must have 0 errors");
    let count = statement_count(src);
    assert_eq!(count, 1, "E5: must be 1 statement");
}

/// E6: `{ a => 1 }` — properly closed single-pair hash, 0 errors.
/// Takes the single-expression path (peek `}` after parse_expression).
#[test]
fn e6_closed_single_pair_hash_no_error() {
    let src = "my %h = { a => 1 };";
    let errs = error_count(src);
    assert_eq!(errs, 0, "E6: closed single-pair hash must have 0 errors");
}

/// E7: Back-to-back unclosed hashes then a sub.
/// `my $x = { a=>1; my $y = { b=>2; sub end {}` — two unclosed, one sub.
/// Both declarations must be present, sub must be recovered.
#[test]
fn e7_back_to_back_unclosed_hashes() {
    let src = "my $x = { a=>1; my $y = { b=>2; sub end {}";
    let has_end = has_subroutine(src, "end");
    assert!(has_end, "E7: sub after two consecutive unclosed hashes must be recovered");
    let errs = error_count(src);
    assert!(errs >= 2, "E7: two unclosed hashes must record at least 2 errors, got {}", errs);
    let count = statement_count(src);
    assert!(count >= 3, "E7: must have >=3 statements ($x, $y, sub end), got {}", count);
}

/// E8: Boundary — `errors_before` counter must not mis-count across nested calls.
/// This tests that errors_before is initialized correctly inside parse_hash_or_block_inner
/// even when there are pre-existing errors from earlier in the same parse.
#[test]
fn e8_errors_before_not_stale_after_prior_errors() {
    // First statement has an error, second statement has an unclosed hash with trailing sub.
    // The errors_before for the second parse_hash_or_block_inner call must NOT include
    // the first statement's error — otherwise had_inner_errors would be wrong.
    let src = "my $bad = [; my $x = { a => 1; sub after {}";
    let has_after = has_subroutine(src, "after");
    assert!(has_after, "E8: sub must be recovered even when prior errors exist");
    let errs = error_count(src);
    assert!(errs >= 2, "E8: must have at least 2 errors (one per unclosed), got {}", errs);
}
