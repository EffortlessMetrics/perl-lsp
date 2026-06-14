//! Red TDD tests for issue #1352 — parser recovery premature bail fix.
//!
//! Issue: When an unclosed delimiter (e.g., `[` without `]`) appears in a nested
//! hash/array initializer at/near EOF, the parser swallows the ENTIRE rest of the file
//! into one ERROR node, losing subsequent valid `sub` declarations and other statements.
//!
//! Expected behavior: After an unclosed-delimiter error, the parser should recover
//! and continue parsing trailing statements (including `sub` declarations) as separate
//! AST nodes, not swallow them into an ERROR node.
//!
//! These tests are RED against the current parser and MUST become GREEN after the fix.

mod cpan_test_helpers;
use cpan_test_helpers::*;
use perl_parser_core::{NodeKind, Parser};
use perl_tdd_support::must;

/// Parse source and extract top-level statement kinds.
/// Useful for verifying that statements are present in the AST (not swallowed).
fn statement_kinds(src: &str) -> Vec<String> {
    let mut parser = Parser::new(src);
    let ast = must(parser.parse());
    match &ast.kind {
        NodeKind::Program { statements } => {
            statements.iter().map(|stmt| format!("{:?}", stmt.kind)).collect()
        }
        _ => vec![],
    }
}

/// Count how many statements are present at the top level.
fn statement_count(src: &str) -> usize {
    let mut parser = Parser::new(src);
    let ast = must(parser.parse());
    match &ast.kind {
        NodeKind::Program { statements } => statements.len(),
        _ => 0,
    }
}

/// Find a subroutine by name in the AST (case-insensitive).
/// Returns true if found, false otherwise.
fn has_subroutine(src: &str, expected_name: &str) -> bool {
    let mut parser = Parser::new(src);
    let ast = must(parser.parse());

    fn find_sub(node: &perl_parser_core::Node, name: &str) -> bool {
        match &node.kind {
            NodeKind::Program { statements } => statements.iter().any(|stmt| find_sub(stmt, name)),
            NodeKind::Subroutine { name: sub_name, .. } => {
                sub_name.as_ref().map_or(false, |n| n == name)
            }
            _ => false,
        }
    }

    find_sub(&ast, expected_name)
}

/// Parse and return the error count.
fn error_count(src: &str) -> usize {
    let mut parser = Parser::new(src);
    let _ast = must(parser.parse());
    parser.get_errors().len()
}

// ============================================================================
// Test Grid — 8 acceptance cases from spec
// ============================================================================

/// **Test-Grid Row 1: Positive case — no errors, healthy parse**
///
/// Input: `my %x = { a => 1, b => 2 }; sub foo {}`
/// Expected: 2 statements, subroutine recovered, no errors
#[test]
fn test_healthy_hash_and_sub() {
    let src = "my %x = { a => 1, b => 2 }; sub foo {}";

    let count = statement_count(src);
    assert_eq!(count, 2, "Healthy case must have 2 statements (decl + sub), got {}", count);

    let has_foo = has_subroutine(src, "foo");
    assert!(has_foo, "Healthy case must have subroutine 'foo'");

    let errors = error_count(src);
    assert_eq!(errors, 0, "Healthy case must have 0 errors, got {}", errors);
}

/// **Test-Grid Row 2: Positive case — deeply nested valid delimiters**
///
/// Input: `my $x = [[[1, 2, 3]]]; sub bar {}`
/// Expected: All statements present, no ERROR nodes in structure, no regression
#[test]
fn test_nested_arrays_deep_valid() {
    let src = "my $x = [[[1, 2, 3]]]; sub bar {}";

    let count = statement_count(src);
    assert_eq!(count, 2, "Deeply nested valid code must have 2 statements, got {}", count);

    let has_bar = has_subroutine(src, "bar");
    assert!(has_bar, "Deeply nested valid code must have subroutine 'bar'");

    // Ensure no errors for valid code
    let errors = error_count(src);
    assert_eq!(errors, 0, "Deeply nested valid code must have 0 errors, got {}", errors);
}

/// **Test-Grid Row 3: Negative case — unclosed bracket in hash**
///
/// Input: `my %x = [1, 2, 3 };`
/// Expected: VariableDeclaration with ERROR, not entire file swallowed
/// Currently RED: The parser swallows the whole thing.
#[test]
fn test_unclosed_bracket_in_hash() {
    let src = "my %x = [1, 2, 3 };";

    // The file should still produce at least a Program node.
    let count = statement_count(src);
    assert!(
        count >= 1,
        "Unclosed bracket must still produce at least one statement, got {}",
        count
    );

    // With the trailing sub, the parser should recover the sub.
    let src_with_sub = "my %x = [1, 2, 3 }; sub valid_after {}";
    let count_with_sub = statement_count(src_with_sub);
    assert!(
        count_with_sub >= 2,
        "Unclosed bracket + sub must produce 2+ statements, got {}",
        count_with_sub
    );

    // The subroutine should be recovered separately, not swallowed into the error.
    let has_sub = has_subroutine(src_with_sub, "valid_after");
    assert!(has_sub, "Subroutine after unclosed bracket must be recovered; not found");
}

/// **Test-Grid Row 4: Negative case — unclosed hash recovery**
///
/// Input: `my $x = { a => 1; sub foo {}`
/// Expected: VariableDeclaration + Subroutine recovered separately
/// Currently RED: Parser swallows the sub into the error.
#[test]
fn test_unclosed_hash_recovery() {
    let src = "my $x = { a => 1; sub foo {}";

    let count = statement_count(src);
    // Should have at least declaration and subroutine (not all swallowed)
    assert!(
        count >= 2,
        "Unclosed hash must recover sub as separate statement, got {} statements",
        count
    );

    let has_foo = has_subroutine(src, "foo");
    assert!(has_foo, "Subroutine 'foo' must be recovered after unclosed hash");

    // Should have at least one error (the unclosed delimiter).
    let errors = error_count(src);
    assert!(errors >= 1, "Unclosed hash must record an error; got 0");
}

/// **Test-Grid Row 5: Boundary case — EOF in middle of unclosed array**
///
/// Input: `my $x = [1, 2, 3` (no closing at EOF)
/// Expected: VariableDeclaration with ERROR, Program root still valid
/// Currently RED: May swallow trailing statements if present.
#[test]
fn test_eof_in_unclosed_array() {
    let src = "my $x = [1, 2, 3";

    let count = statement_count(src);
    assert!(
        count >= 1,
        "EOF in unclosed array must still have Program root, got {} statements",
        count
    );

    let errors = error_count(src);
    assert!(errors >= 1, "EOF in unclosed array must record an error; got 0");

    // With a trailing sub, should be recovered.
    let src_with_sub = "my $x = [1, 2, 3; sub baz {}";
    let count_with_sub = statement_count(src_with_sub);
    assert!(
        count_with_sub >= 2,
        "EOF in unclosed array + sub must recover both, got {} statements",
        count_with_sub
    );

    let has_baz = has_subroutine(src_with_sub, "baz");
    assert!(has_baz, "Subroutine 'baz' must be recovered after EOF in array");
}

/// **Test-Grid Row 6: Boundary case — 5-level unclosed nesting**
///
/// Input: `my $x = { { { { [1` (5 levels unclosed)
/// Expected: ERROR node for var, file still parseable
/// Currently RED: Parser may cascade error and swallow trailing code.
#[test]
fn test_5_level_unclosed_nesting() {
    let src = "my $x = { { { { [1";

    let count = statement_count(src);
    assert!(
        count >= 1,
        "5-level unclosed nesting must still have Program, got {} statements",
        count
    );

    // With trailing sub, should recover.
    let src_with_sub = "my $x = { { { { [1; sub qux {}";
    let count_with_sub = statement_count(src_with_sub);
    assert!(
        count_with_sub >= 2,
        "5-level unclosed nesting + sub must produce 2+ statements, got {}",
        count_with_sub
    );

    let has_qux = has_subroutine(src_with_sub, "qux");
    assert!(has_qux, "Subroutine 'qux' must be recovered after 5-level unclosed nesting");
}

/// **Test-Grid Row 7: Adversarial case — multiple separate unclosed errors**
///
/// Input: `my $x = [; my $y = {` (two separate unclosed)
/// Expected: Both statements parsed (with errors), multiple errors recorded
/// Currently RED: May cascade one error into the other.
#[test]
fn test_multiple_unclosed_errors() {
    let src = "my $x = [; my $y = {";

    let count = statement_count(src);
    assert!(
        count >= 2,
        "Multiple unclosed errors must parse both declarations, got {} statements",
        count
    );

    // Should have at least 2 errors (one per unclosed).
    let errors = error_count(src);
    assert!(errors >= 2, "Multiple unclosed errors must record at least 2 errors; got {}", errors);

    // With a trailing sub, should be recovered.
    let src_with_sub = "my $x = [; my $y = {; sub zap {}";
    let count_with_sub = statement_count(src_with_sub);
    assert!(
        count_with_sub >= 3,
        "Multiple unclosed + sub must produce 3+ statements, got {}",
        count_with_sub
    );

    let has_zap = has_subroutine(src_with_sub, "zap");
    assert!(has_zap, "Subroutine 'zap' must be recovered after multiple errors");
}

/// **Test-Grid Row 8: Regression case — error in if branch**
///
/// Input: `if ($x) { my $y = [1 } else { print 1; }`
/// Expected: If statement parsed (with error), else recovered, following recovered
/// Currently RED: May cascade error and lose the else branch.
#[test]
fn test_error_in_if_branch_recovery() {
    let src = "if ($x) { my $y = [1 } else { print 1; }";

    // Should produce at least one statement (the if).
    let count = statement_count(src);
    assert!(
        count >= 1,
        "Error in if branch must still parse the if statement, got {} statements",
        count
    );

    // Should have at least one error (unclosed bracket in if body).
    let errors = error_count(src);
    assert!(errors >= 1, "Error in if branch must record an error; got 0");

    // With a trailing sub, should be recovered.
    let src_with_sub = "if ($x) { my $y = [1 } else { print 1; } sub after {}";
    let count_with_sub = statement_count(src_with_sub);
    assert!(
        count_with_sub >= 2,
        "Error in if + trailing sub must produce 2+ statements, got {}",
        count_with_sub
    );

    let has_after = has_subroutine(src_with_sub, "after");
    assert!(has_after, "Subroutine 'after' must be recovered after error in if branch");
}

// ============================================================================
// Core bug reproduction — the main issue
// ============================================================================

/// **CORE BUG: Trailing subroutine swallowed by unclosed-delimiter error**
///
/// This is the main issue from #1352. When an unclosed delimiter appears
/// near EOF, the parser swallows the entire rest of the file into one ERROR node,
/// losing subsequent valid `sub` declarations.
///
/// Example:
/// ```perl
/// my %config = { key => { nested => [1, 2, 3   # missing ] and }
/// };
/// sub valid_after_unclosed {}   # currently SWALLOWED — not in AST
/// ```
///
/// Currently RED: The subroutine is not in the AST (swallowed into error).
/// After fix: The subroutine should be present as a separate Subroutine node.
#[test]
fn test_core_bug_unclosed_delimiter_swallows_trailing_sub() {
    let src = "my %config = { key => { nested => [1, 2, 3   # missing ] and }
};
sub valid_after_unclosed {}";

    // The parser must produce at least 2 statements:
    // 1. The VariableDeclaration (with an ERROR node inside)
    // 2. The Subroutine (recovered as a separate statement)
    let count = statement_count(src);
    assert!(
        count >= 2,
        "CORE BUG: Trailing sub must not be swallowed; expected 2+ statements, got {}",
        count
    );

    // The subroutine must be findable by name, not lost in an ERROR node.
    let has_sub = has_subroutine(src, "valid_after_unclosed");
    assert!(
        has_sub,
        "CORE BUG: Subroutine 'valid_after_unclosed' must be recovered; not found in AST"
    );
}

/// **Variant: Multiple trailing statements after unclosed delimiter**
///
/// Verifies that all trailing code is recovered, not just the first subroutine.
#[test]
fn test_unclosed_delimiter_with_multiple_trailing_statements() {
    let src = "my $x = [1, 2, 3; sub foo {} sub bar {} my $y = 42;";

    let count = statement_count(src);
    // Should have at least: declaration, sub foo, sub bar, declaration of $y
    assert!(
        count >= 4,
        "Unclosed + multiple trailing must recover all; expected 4+ statements, got {}",
        count
    );

    let has_foo = has_subroutine(src, "foo");
    let has_bar = has_subroutine(src, "bar");
    assert!(has_foo, "Subroutine 'foo' must be recovered");
    assert!(has_bar, "Subroutine 'bar' must be recovered");
}

/// **Test: Deeply nested but valid code is not broken by recovery changes**
///
/// Ensures that fixing the recovery issue doesn't break valid deeply-nested code.
#[test]
fn test_regression_deeply_nested_valid_code() {
    let src = "my $x = {
    level1 => {
        level2 => {
            level3 => {
                level4 => {
                    level5 => [1, 2, 3, 4, 5]
                }
            }
        }
    }
};
sub after_deep_nesting {}";

    let count = statement_count(src);
    assert_eq!(
        count, 2,
        "Deeply nested valid code must have 2 statements (decl + sub), got {}",
        count
    );

    let has_sub = has_subroutine(src, "after_deep_nesting");
    assert!(has_sub, "Subroutine after deep nesting must be present");

    let errors = error_count(src);
    assert_eq!(errors, 0, "Deeply nested valid code must have 0 errors, got {}", errors);
}

/// **Test: Error is still recorded (don't silently accept unclosed delimiters)**
///
/// Ensures that fixing the recovery issue doesn't hide the actual error.
/// The parser must still report that there was an unclosed delimiter.
#[test]
fn test_error_is_recorded_after_recovery() {
    let src = "my $x = [1, 2, 3; sub foo {}";

    let errors = error_count(src);
    assert!(errors >= 1, "Parser must still record an error for unclosed bracket; got 0");

    // But the subroutine must still be recovered.
    let has_foo = has_subroutine(src, "foo");
    assert!(has_foo, "Error must be recorded, but subroutine must still be recovered");
}

// ============================================================================
// Reviewer-deep regression guards — added by deep review of PR #1456
// ============================================================================

/// **Regression: valid multi-statement block (non-hash first stmt) must not be truncated**
///
/// The `unclosed_after_inner_error` guard in parse_hash_or_block_inner must NOT fire
/// for a valid multi-statement block where the first statement does NOT produce inner
/// errors during parse_expression().
///
/// Example: `transaction { my $x = 1; do_work(); }` — the block's first expression
/// `my $x = 1` succeeds without errors, so had_inner_errors=false and the guard does
/// not fire. The multi-statement block loop runs correctly.
#[test]
fn test_valid_multi_stmt_block_not_truncated() {
    // A bare-function-style call where the {} argument is a multi-statement block
    let src = "transaction { my $x = 1; do_work($x); commit(); };";
    let errs = error_count(src);
    assert_eq!(errs, 0, "Valid multi-stmt block must have 0 errors, got {}", errs);
    let count = statement_count(src);
    assert_eq!(count, 1, "Must parse as 1 statement (the transaction call), got {}", count);
}

/// **Regression: sub body with multiple statements must not be truncated**
///
/// Sub bodies use parse_block(), not parse_hash_or_block_inner(), so this is a
/// belt-and-suspenders regression guard that also confirms the scoping of the fix.
#[test]
fn test_sub_body_multi_stmt_regression() {
    let src = "sub compute { my $x = 1; my $y = 2; return $x + $y; }";
    let errs = error_count(src);
    assert_eq!(errs, 0, "Sub with multi-stmt body must have 0 errors, got {}", errs);
    let count = statement_count(src);
    assert_eq!(count, 1, "Sub must be 1 top-level statement, got {}", count);
    let has_compute = has_subroutine(src, "compute");
    assert!(has_compute, "Subroutine 'compute' must be in AST");
}

/// **Regression: valid block with inner expression error still recovers gracefully**
///
/// Verifies the `unclosed_after_inner_error` arm: when inner errors DID occur
/// (e.g., nested unclosed `[`) AND peek is `;`, the guard fires correctly and
/// the following declarations appear as separate top-level statements.
#[test]
fn test_inner_error_plus_semicolon_triggers_recovery() {
    // my $x = { { { { [1   — 5 levels deep unclosed, then ; sub
    // The unclosed_after_inner_error guard must fire here (not unclosed_hash)
    let src = "my $x = { { { { [1; sub recovered {}";
    let has_sub = has_subroutine(src, "recovered");
    assert!(has_sub, "'recovered' sub must appear as separate statement after deeply-nested error");
    let errs = error_count(src);
    assert!(errs >= 1, "Deeply nested unclosed must record at least one error");
}
