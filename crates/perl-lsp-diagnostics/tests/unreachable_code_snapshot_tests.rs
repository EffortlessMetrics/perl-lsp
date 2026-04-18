//! Snapshot tests for unreachable code detection (PL406)
//!
//! These tests complement code-level assertions by snapshotting normalized
//! diagnostics (code, severity, range, message) for representative
//! snippets involving continue blocks. This catches regressions in message
//! text and location metadata.

use insta::assert_snapshot;
use perl_lsp_diagnostics::unreachable_code::check_unreachable_code;
use perl_parser::Parser;
use perl_parser_core::ast::Node;

fn parse(source: &str) -> Node {
    let output = Parser::new(source).parse_with_recovery();
    output.ast
}

fn unreachable_code_for(source: &str) -> Vec<perl_lsp_diagnostics::Diagnostic> {
    let ast = parse(source);
    let mut diagnostics = vec![];
    check_unreachable_code(&ast, &mut diagnostics);
    diagnostics
}

fn severity_name(severity: perl_diagnostics::codes::DiagnosticSeverity) -> &'static str {
    match severity {
        perl_diagnostics::codes::DiagnosticSeverity::Error => "Error",
        perl_diagnostics::codes::DiagnosticSeverity::Warning => "Warning",
        perl_diagnostics::codes::DiagnosticSeverity::Information => "Information",
        perl_diagnostics::codes::DiagnosticSeverity::Hint => "Hint",
    }
}

fn normalize(diags: Vec<perl_lsp_diagnostics::Diagnostic>) -> String {
    let mut normalized: Vec<_> = diags
        .into_iter()
        .map(|diag| {
            let code = diag.code.unwrap_or_else(|| "<none>".to_string());
            format!(
                "{} | {} | {:?} | {}",
                code,
                severity_name(diag.severity),
                diag.range,
                diag.message
            )
        })
        .collect();

    normalized.sort_unstable();
    normalized.join("\n")
}

// ===========================================================================
// AC-1: Continue block with die followed by statement
// "while (1) { } continue { die 'err'; print 'dead'; }"
// expect: exactly 1 PL406 diagnostic on the print statement
// ===========================================================================

#[test]
fn snapshot_continue_block_die_followed_by_statement() {
    let source = "while (1) { } continue { die 'err'; print 'dead'; }";
    let snapshot = normalize(unreachable_code_for(source));
    assert_snapshot!("continue_block_die_followed_by_statement", snapshot);
}

// ===========================================================================
// AC-2: Continue block with exit followed by statement
// "while (1) { } continue { exit(0); print 'dead'; }"
// expect: exactly 1 PL406 diagnostic on the print statement
// ===========================================================================

#[test]
fn snapshot_continue_block_exit_followed_by_statement() {
    let source = "while (1) { } continue { exit(0); print 'dead'; }";
    let snapshot = normalize(unreachable_code_for(source));
    assert_snapshot!("continue_block_exit_followed_by_statement", snapshot);
}

// ===========================================================================
// AC-3: Continue block with croak followed by statement
// "while (1) { } continue { croak 'err'; print 'dead'; }"
// expect: exactly 1 PL406 diagnostic on the print statement
// ===========================================================================

#[test]
fn snapshot_continue_block_croak_followed_by_statement() {
    let source = "while (1) { } continue { croak 'err'; print 'dead'; }";
    let snapshot = normalize(unreachable_code_for(source));
    assert_snapshot!("continue_block_croak_followed_by_statement", snapshot);
}

// ===========================================================================
// AC-4: Continue block with last followed by statement
// "while (1) { } continue { last; print 'dead'; }"
// expect: exactly 1 PL406 diagnostic on the print statement
// ===========================================================================

#[test]
fn snapshot_continue_block_last_followed_by_statement() {
    let source = "while (1) { } continue { last; print 'dead'; }";
    let snapshot = normalize(unreachable_code_for(source));
    assert_snapshot!("continue_block_last_followed_by_statement", snapshot);
}

// ===========================================================================
// AC-6: next in continue block followed by statement — NO false positive
// "while (1) { } continue { next; print 'reachable'; }"
// expect: 0 PL406 diagnostics (next jumps to next iteration, continue block re-runs)
// ===========================================================================

#[test]
fn snapshot_continue_block_next_no_false_positive() {
    let source = "while (1) { } continue { next; print 'reachable'; }";
    let snapshot = normalize(unreachable_code_for(source));
    assert_snapshot!("continue_block_next_no_false_positive", snapshot);
}

// ===========================================================================
// AC-7: redo in continue block followed by statement — NO false positive
// "while (1) { } continue { redo; print 'reachable'; }"
// expect: 0 PL406 diagnostics (redo re-runs the continue block)
// ===========================================================================

#[test]
fn snapshot_continue_block_redo_no_false_positive() {
    let source = "while (1) { } continue { redo; print 'reachable'; }";
    let snapshot = normalize(unreachable_code_for(source));
    assert_snapshot!("continue_block_redo_no_false_positive", snapshot);
}

// ===========================================================================
// AC-8: Multiple unreachable statements in continue block
// "while (1) { } continue { die 'err'; my $x = 1; my $y = 2; print 'dead'; }"
// expect: 3 PL406 diagnostics (one each for $x, $y, and print)
// ===========================================================================

#[test]
fn snapshot_continue_block_multiple_unreachable() {
    let source = "while (1) { } continue { die 'err'; my $x = 1; my $y = 2; print 'dead'; }";
    let snapshot = normalize(unreachable_code_for(source));
    assert_snapshot!("continue_block_multiple_unreachable", snapshot);
}

// ===========================================================================
// AC-9: Loop body unreachable detection unchanged
// "while (1) { die 'err'; print 'dead'; }"
// expect: 1 PL406 diagnostic on print in the loop body (not in continue block)
// ===========================================================================

#[test]
fn snapshot_loop_body_unreachable_unchanged() {
    let source = "while (1) { die 'err'; print 'dead'; }";
    let snapshot = normalize(unreachable_code_for(source));
    assert_snapshot!("loop_body_unreachable_unchanged", snapshot);
}

// ===========================================================================
// AC-10: All four loop types covered — while with continue block
// ===========================================================================

#[test]
fn snapshot_while_loop_with_continue_block() {
    let source = "while (1) { } continue { die 'err'; print 'dead'; }";
    let snapshot = normalize(unreachable_code_for(source));
    assert_snapshot!("while_loop_with_continue_block", snapshot);
}

#[test]
fn snapshot_until_loop_with_continue_block() {
    let source = "until (1) { } continue { die 'err'; print 'dead'; }";
    let snapshot = normalize(unreachable_code_for(source));
    assert_snapshot!("until_loop_with_continue_block", snapshot);
}

#[test]
fn snapshot_for_loop_with_continue_block() {
    let source = "for (1;;) { } continue { die 'err'; print 'dead'; }";
    let snapshot = normalize(unreachable_code_for(source));
    assert_snapshot!("for_loop_with_continue_block", snapshot);
}

#[test]
fn snapshot_foreach_loop_with_continue_block() {
    let source = "foreach my $x (1..3) { } continue { die 'err'; print 'dead'; }";
    let snapshot = normalize(unreachable_code_for(source));
    assert_snapshot!("foreach_loop_with_continue_block", snapshot);
}

// ===========================================================================
// Confess in continue block
// ===========================================================================

#[test]
fn snapshot_continue_block_confess_followed_by_statement() {
    let source = "while (1) { } continue { confess 'err'; print 'dead'; }";
    let snapshot = normalize(unreachable_code_for(source));
    assert_snapshot!("continue_block_confess_followed_by_statement", snapshot);
}

// ===========================================================================
// Carp::croak and Carp::confess in continue block
// ===========================================================================

#[test]
fn snapshot_continue_block_carp_croak_followed_by_statement() {
    let source = "while (1) { } continue { Carp::croak 'err'; print 'dead'; }";
    let snapshot = normalize(unreachable_code_for(source));
    assert_snapshot!("continue_block_carp_croak_followed_by_statement", snapshot);
}

// ===========================================================================
// Happy path: no unreachable code
// ===========================================================================

#[test]
fn snapshot_no_unreachable_code() {
    let source = "while (1) { print 'alive'; } continue { next; print 'still alive'; }";
    let snapshot = normalize(unreachable_code_for(source));
    assert_snapshot!("no_unreachable_code", snapshot);
}
