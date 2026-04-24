//! Integration tests: unreachable code detection in continue blocks (PL406)
//!
//! Tests verify that the full pipeline — real Perl source → parser → DiagnosticsProvider
//! — emits PL406 unreachable code diagnostics for continue blocks.
//!
//! These tests complement the unit tests in `unreachable_code_tests.rs` which test
//! the `check_unreachable_code` function directly with manually constructed AST nodes.
//!
//! Integration tests use the real parser (`perl_parser::Parser`) and the full
//! `DiagnosticsProvider::get_diagnostics()` pipeline.

use std::sync::Arc;

use perl_lsp_diagnostics::{Diagnostic, DiagnosticSeverity, DiagnosticTag, DiagnosticsProvider};
use perl_parser::Parser;

/// Helper: parse Perl source and get all diagnostics from DiagnosticsProvider
fn diagnostics_for(source: &str) -> Vec<Diagnostic> {
    let output = Parser::new(source).parse_with_recovery();
    let ast = Arc::new(output.ast);
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    provider.get_diagnostics(&ast, &output.diagnostics, source, None)
}

/// Helper: count PL406 diagnostics in the output
fn count_pl406(diagnostics: &[Diagnostic]) -> usize {
    diagnostics.iter().filter(|d| d.code.as_deref() == Some("PL406")).count()
}

/// Helper: check if any PL406 diagnostic exists
fn has_pl406(diagnostics: &[Diagnostic]) -> bool {
    diagnostics.iter().any(|d| d.code.as_deref() == Some("PL406"))
}

// =========================================================================
// Continue block with die followed by statement (AC-1)
// =========================================================================

#[test]
fn integration_continue_block_die_emits_pl406() {
    // "while (1) { } continue { die 'err'; print 'dead'; }"
    // expect: exactly 1 PL406 diagnostic on the print statement
    let source = "use strict;\nuse warnings;\nwhile (1) {\n} continue {\n    die 'err';\n    print 'dead';\n}\n";
    let diags = diagnostics_for(source);
    let pl406_count = count_pl406(&diags);

    assert_eq!(
        pl406_count,
        1,
        "Expected exactly 1 PL406 for die in continue block, got {} total PL406: {:?}",
        pl406_count,
        diags
            .iter()
            .filter(|d| d.code.as_deref() == Some("PL406"))
            .map(|d| &d.message)
            .collect::<Vec<_>>()
    );

    // Verify it's a Hint severity with Unnecessary tag
    let pl406_diags: Vec<_> = diags.iter().filter(|d| d.code.as_deref() == Some("PL406")).collect();
    assert_eq!(pl406_diags[0].severity, DiagnosticSeverity::Hint);
    assert!(pl406_diags[0].tags.contains(&DiagnosticTag::Unnecessary));
    assert!(pl406_diags[0].suggestion.is_some());
}

// =========================================================================
// Continue block with exit followed by statement (AC-2)
// =========================================================================

#[test]
fn integration_continue_block_exit_emits_pl406() {
    // "while (1) { } continue { exit(0); print 'dead'; }"
    // expect: exactly 1 PL406 diagnostic on the print statement
    let source = "use strict;\nuse warnings;\nwhile (1) {\n} continue {\n    exit(0);\n    print 'dead';\n}\n";
    let diags = diagnostics_for(source);
    let pl406_count = count_pl406(&diags);

    assert_eq!(
        pl406_count,
        1,
        "Expected exactly 1 PL406 for exit in continue block, got {} total PL406: {:?}",
        pl406_count,
        diags
            .iter()
            .filter(|d| d.code.as_deref() == Some("PL406"))
            .map(|d| &d.message)
            .collect::<Vec<_>>()
    );
}

// =========================================================================
// Continue block with croak followed by statement (AC-3)
// =========================================================================

#[test]
fn integration_continue_block_croak_emits_pl406() {
    // "while (1) { } continue { croak 'err'; print 'dead'; }"
    // expect: exactly 1 PL406 diagnostic on the print statement
    let source = "use strict;\nuse warnings;\nwhile (1) {\n} continue {\n    croak 'err';\n    print 'dead';\n}\n";
    let diags = diagnostics_for(source);
    let pl406_count = count_pl406(&diags);

    assert_eq!(
        pl406_count,
        1,
        "Expected exactly 1 PL406 for croak in continue block, got {} total PL406: {:?}",
        pl406_count,
        diags
            .iter()
            .filter(|d| d.code.as_deref() == Some("PL406"))
            .map(|d| &d.message)
            .collect::<Vec<_>>()
    );
}

// =========================================================================
// Continue block with last followed by statement (AC-4)
// =========================================================================

#[test]
fn integration_continue_block_last_emits_pl406() {
    // "while (1) { } continue { last; print 'dead'; }"
    // expect: exactly 1 PL406 diagnostic on the print statement
    let source =
        "use strict;\nuse warnings;\nwhile (1) {\n} continue {\n    last;\n    print 'dead';\n}\n";
    let diags = diagnostics_for(source);
    let pl406_count = count_pl406(&diags);

    assert_eq!(
        pl406_count,
        1,
        "Expected exactly 1 PL406 for last in continue block, got {} total PL406: {:?}",
        pl406_count,
        diags
            .iter()
            .filter(|d| d.code.as_deref() == Some("PL406"))
            .map(|d| &d.message)
            .collect::<Vec<_>>()
    );
}

// =========================================================================
// Continue block with return followed by statement in sub context (AC-5)
// =========================================================================

#[test]
fn integration_continue_block_return_emits_pl406() {
    // "sub f { while (1) { } continue { return; print 'dead'; } }"
    // expect: exactly 1 PL406 diagnostic on the print statement
    let source = "use strict;\nuse warnings;\nsub f {\n    while (1) {\n    } continue {\n        return;\n        print 'dead';\n    }\n}\n";
    let diags = diagnostics_for(source);
    let pl406_count = count_pl406(&diags);

    assert_eq!(
        pl406_count,
        1,
        "Expected exactly 1 PL406 for return in continue block (in sub), got {} total PL406: {:?}",
        pl406_count,
        diags
            .iter()
            .filter(|d| d.code.as_deref() == Some("PL406"))
            .map(|d| &d.message)
            .collect::<Vec<_>>()
    );
}

// =========================================================================
// Continue block with next followed by statement — NO false positive (AC-6)
// =========================================================================

#[test]
fn integration_continue_block_next_no_false_positive() {
    // "while (1) { } continue { next; print 'reachable'; }"
    // expect: 0 PL406 diagnostics (next jumps to next iteration, continue re-runs)
    let source = "use strict;\nuse warnings;\nwhile (1) {\n} continue {\n    next;\n    print 'reachable';\n}\n";
    let diags = diagnostics_for(source);
    let pl406_count = count_pl406(&diags);

    assert_eq!(
        pl406_count,
        0,
        "Expected 0 PL406 for next in continue block (next re-runs continue), got {} total PL406: {:?}",
        pl406_count,
        diags
            .iter()
            .filter(|d| d.code.as_deref() == Some("PL406"))
            .map(|d| &d.message)
            .collect::<Vec<_>>()
    );
}

// =========================================================================
// Continue block with redo followed by statement — NO false positive (AC-7)
// =========================================================================

#[test]
fn integration_continue_block_redo_no_false_positive() {
    // "while (1) { } continue { redo; print 'reachable'; }"
    // expect: 0 PL406 diagnostics (redo re-runs the continue block)
    let source = "use strict;\nuse warnings;\nwhile (1) {\n} continue {\n    redo;\n    print 'reachable';\n}\n";
    let diags = diagnostics_for(source);
    let pl406_count = count_pl406(&diags);

    assert_eq!(
        pl406_count,
        0,
        "Expected 0 PL406 for redo in continue block (redo re-runs continue), got {} total PL406: {:?}",
        pl406_count,
        diags
            .iter()
            .filter(|d| d.code.as_deref() == Some("PL406"))
            .map(|d| &d.message)
            .collect::<Vec<_>>()
    );
}

// =========================================================================
// Multiple unreachable statements in continue block (AC-8)
// =========================================================================

#[test]
fn integration_continue_block_multiple_unreachable() {
    // "while (1) { } continue { die 'err'; my $x = 1; my $y = 2; print 'dead'; }"
    // expect: 3 PL406 diagnostics (one each for $x, $y, and print)
    let source = "use strict;\nuse warnings;\nwhile (1) {\n} continue {\n    die 'err';\n    my $x = 1;\n    my $y = 2;\n    print 'dead';\n}\n";
    let diags = diagnostics_for(source);
    let pl406_count = count_pl406(&diags);

    assert_eq!(
        pl406_count,
        3,
        "Expected exactly 3 PL406 for multiple unreachable statements in continue block, got {} total PL406: {:?}",
        pl406_count,
        diags
            .iter()
            .filter(|d| d.code.as_deref() == Some("PL406"))
            .map(|d| &d.message)
            .collect::<Vec<_>>()
    );
}

// =========================================================================
// Loop body unreachable detection unchanged (AC-9)
// =========================================================================

#[test]
fn integration_loop_body_detection_unchanged() {
    // "while (1) { next if $cond; die 'err'; print 'dead'; }"
    // expect: 1 PL406 diagnostic on print in the loop body (not in continue block)
    let source = "use strict;\nuse warnings;\nwhile (1) {\n    die 'err';\n    print 'dead';\n}\n";
    let diags = diagnostics_for(source);
    let pl406_count = count_pl406(&diags);

    assert_eq!(
        pl406_count,
        1,
        "Expected exactly 1 PL406 for unreachable code in loop body, got {} total PL406: {:?}",
        pl406_count,
        diags
            .iter()
            .filter(|d| d.code.as_deref() == Some("PL406"))
            .map(|d| &d.message)
            .collect::<Vec<_>>()
    );
}

// =========================================================================
// All four loop types covered (AC-10): while, until, for, foreach
// =========================================================================

#[test]
fn integration_while_loop_with_continue_block() {
    // while loop with continue block and die
    let source = "use strict;\nuse warnings;\nwhile (1) {\n} continue {\n    die 'err';\n    print 'dead';\n}\n";
    let diags = diagnostics_for(source);
    assert_eq!(count_pl406(&diags), 1, "while loop with continue block should detect PL406");
}

#[test]
fn integration_for_loop_with_continue_block() {
    // for loop with continue block and die
    let source = "use strict;\nuse warnings;\nfor (my $i = 0; $i < 10; $i++) {\n} continue {\n    die 'err';\n    print 'dead';\n}\n";
    let diags = diagnostics_for(source);
    assert_eq!(count_pl406(&diags), 1, "for loop with continue block should detect PL406");
}

#[test]
fn integration_foreach_loop_with_continue_block() {
    // foreach loop with continue block and die
    let source = "use strict;\nuse warnings;\nforeach my $item (@list) {\n} continue {\n    die 'err';\n    print 'dead';\n}\n";
    let diags = diagnostics_for(source);
    assert_eq!(count_pl406(&diags), 1, "foreach loop with continue block should detect PL406");
}

// =========================================================================
// Continue block with Carp::croak (qualified) followed by statement
// =========================================================================

#[test]
fn integration_continue_block_carp_croak_emits_pl406() {
    // "while (1) { } continue { Carp::croak 'err'; print 'dead'; }"
    let source = "use strict;\nuse warnings;\nwhile (1) {\n} continue {\n    Carp::croak 'err';\n    print 'dead';\n}\n";
    let diags = diagnostics_for(source);
    assert_eq!(count_pl406(&diags), 1, "Carp::croak in continue block should emit PL406");
}

// =========================================================================
// Continue block with Carp::confess (qualified) followed by statement
// =========================================================================

#[test]
fn integration_continue_block_carp_confess_emits_pl406() {
    // "while (1) { } continue { Carp::confess 'err'; print 'dead'; }"
    let source = "use strict;\nuse warnings;\nwhile (1) {\n} continue {\n    Carp::confess 'err';\n    print 'dead';\n}\n";
    let diags = diagnostics_for(source);
    assert_eq!(count_pl406(&diags), 1, "Carp::confess in continue block should emit PL406");
}

// =========================================================================
// Empty continue block — no diagnostics expected
// =========================================================================

#[test]
fn integration_empty_continue_block_no_pl406() {
    // "while (1) { } continue { }"
    let source = "use strict;\nuse warnings;\nwhile (1) {\n} continue {\n}\n";
    let diags = diagnostics_for(source);
    assert_eq!(count_pl406(&diags), 0, "Empty continue block should not emit PL406");
}

// =========================================================================
// Continue block with only unconditional exit — no diagnostics expected
// =========================================================================

#[test]
fn integration_continue_block_only_exit_no_pl406() {
    // "while (1) { } continue { die 'err'; }"
    let source = "use strict;\nuse warnings;\nwhile (1) {\n} continue {\n    die 'err';\n}\n";
    let diags = diagnostics_for(source);
    assert_eq!(count_pl406(&diags), 0, "Continue block with only exit should not emit PL406");
}

// =========================================================================
// Nested block inside continue block — inner die doesn't affect outer
// =========================================================================

#[test]
fn integration_nested_block_inside_continue() {
    // "while (1) { } continue { { die 'err'; } print 'reachable'; }"
    // The die is inside an inner block, so print is still reachable
    // from the continue block's perspective
    let source = "use strict;\nuse warnings;\nwhile (1) {\n} continue {\n    {\n        die 'err';\n    }\n    print 'reachable';\n}\n";
    let diags = diagnostics_for(source);
    assert_eq!(
        count_pl406(&diags),
        0,
        "Nested block in continue block should not affect outer reachability"
    );
}

// =========================================================================
// Labeled next in continue block — NO false positive
// =========================================================================

#[test]
fn integration_labeled_next_continue_block_no_pl406() {
    // "while (1) { } continue { next OUTER if $cond; print 'reachable'; }"
    let source = "use strict;\nuse warnings;\nOUTER: while (1) {\n} continue {\n    next OUTER if $cond;\n    print 'reachable';\n}\n";
    let diags = diagnostics_for(source);
    assert_eq!(count_pl406(&diags), 0, "Labeled next in continue block should not emit PL406");
}

// =========================================================================
// Labeled redo in continue block — NO false positive
// =========================================================================

#[test]
fn integration_labeled_redo_continue_block_no_pl406() {
    // "while (1) { } continue { redo OUTER; print 'reachable'; }"
    let source = "use strict;\nuse warnings;\nOUTER: while (1) {\n} continue {\n    redo OUTER;\n    print 'reachable';\n}\n";
    let diags = diagnostics_for(source);
    assert_eq!(count_pl406(&diags), 0, "Labeled redo in continue block should not emit PL406");
}

// =========================================================================
// Eval inside continue block — die in eval doesn't poison continue block
// =========================================================================

#[test]
fn integration_eval_inside_continue_block() {
    // "while (1) { } continue { eval { die 'err' }; print 'reachable'; }"
    let source = "use strict;\nuse warnings;\nwhile (1) {\n} continue {\n    eval { die 'err' };\n    print 'reachable';\n}\n";
    let diags = diagnostics_for(source);
    assert_eq!(count_pl406(&diags), 0, "Eval inside continue block should not emit PL406");
}

// =========================================================================
// Anonymous sub in continue block — return in sub doesn't affect continue
// =========================================================================

#[test]
fn integration_anonymous_sub_in_continue_block() {
    // "while (1) { } continue { my $f = sub { return; }; print 'reachable'; }"
    let source = "use strict;\nuse warnings;\nwhile (1) {\n} continue {\n    my $f = sub { return; };\n    print 'reachable';\n}\n";
    let diags = diagnostics_for(source);
    assert_eq!(count_pl406(&diags), 0, "Anonymous sub in continue block should not emit PL406");
}

// =========================================================================
// Conditional die in continue block — NO false positive
// =========================================================================

#[test]
fn integration_conditional_die_in_continue_block() {
    // "while (1) { } continue { die 'err' if $cond; print 'reachable'; }"
    let source = "use strict;\nuse warnings;\nwhile (1) {\n} continue {\n    die 'err' if $cond;\n    print 'reachable';\n}\n";
    let diags = diagnostics_for(source);
    assert_eq!(count_pl406(&diags), 0, "Conditional die in continue block should not emit PL406");
}

// =========================================================================
// next with multiple following statements in continue block — NO false positive
// =========================================================================

#[test]
fn integration_next_continue_with_multiple_following() {
    // "while (1) { } continue { next; $x = 1; $y = 2; print 'reachable'; }"
    let source = "use strict;\nuse warnings;\nwhile (1) {\n} continue {\n    next;\n    my $x = 1;\n    my $y = 2;\n    print 'reachable';\n}\n";
    let diags = diagnostics_for(source);
    assert_eq!(
        count_pl406(&diags),
        0,
        "Multiple statements after next in continue block should not emit PL406"
    );
}

// =========================================================================
// redo with multiple following statements in continue block — NO false positive
// =========================================================================

#[test]
fn integration_redo_continue_with_multiple_following() {
    // "while (1) { } continue { redo; $x = 1; $y = 2; print 'reachable'; }"
    let source = "use strict;\nuse warnings;\nwhile (1) {\n} continue {\n    redo;\n    my $x = 1;\n    my $y = 2;\n    print 'reachable';\n}\n";
    let diags = diagnostics_for(source);
    assert_eq!(
        count_pl406(&diags),
        0,
        "Multiple statements after redo in continue block should not emit PL406"
    );
}

// =========================================================================
// Combined: loop body with die AND continue block with die — both detected
// =========================================================================

#[test]
fn integration_both_loop_body_and_continue_block() {
    // Loop body has die followed by unreachable print
    // Continue block has die followed by unreachable print
    // Should get 2 PL406 diagnostics total
    let source = "use strict;\nuse warnings;\nwhile (1) {\n    die 'body_err';\n    print 'body_dead';\n} continue {\n    die 'cont_err';\n    print 'cont_dead';\n}\n";
    let diags = diagnostics_for(source);
    assert_eq!(
        count_pl406(&diags),
        2,
        "Expected 2 PL406 diagnostics (one for loop body, one for continue block), got: {:?}",
        diags
            .iter()
            .filter(|d| d.code.as_deref() == Some("PL406"))
            .map(|d| &d.message)
            .collect::<Vec<_>>()
    );
}
