//! Tests for cascade parse-error suppression in the diagnostics pipeline.
//!
//! Cascade suppression groups adjacent parse-error diagnostics by byte proximity
//! and keeps only the first in each cluster, reducing noise for users.

use std::sync::Arc;

use perl_lsp_diagnostics::{Diagnostic, DiagnosticSeverity, DiagnosticsProvider};
use perl_parser::Parser;
use perl_parser_core::error::ParseError;
use perl_parser_core::{Node, NodeKind, SourceLocation};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn empty_program(source_len: usize) -> Arc<Node> {
    Arc::new(Node::new(
        NodeKind::Program { statements: vec![] },
        SourceLocation {
            start: 0,
            end: source_len,
        },
    ))
}

fn parse_errors_only(diags: &[Diagnostic]) -> Vec<&Diagnostic> {
    diags
        .iter()
        .filter(|d| {
            matches!(
                d.code.as_deref(),
                Some("PL001") | Some("PL002") | Some("PL003")
            )
        })
        .collect()
}

fn run_diagnostics(source: &str, errors: Vec<ParseError>) -> Vec<Diagnostic> {
    let ast = empty_program(source.len());
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    provider.get_diagnostics(&ast, &errors, source, None)
}

/// Run the full parser + diagnostics pipeline on real Perl source.
fn parse_and_diagnose(source: &str) -> Vec<Diagnostic> {
    let output = Parser::new(source).parse_with_recovery();
    let ast = Arc::new(output.ast);
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    provider.get_diagnostics(&ast, &output.diagnostics, source, None)
}

/// Filter to Error-severity parse-error diagnostics only.
///
/// Excludes non-Error-severity diagnostics even if they carry a parse-error
/// code.  Use this when you want to count "alarming" diagnostics shown in red
/// in the gutter — the key user-visible noise metric.
fn error_level_parse_diags(diags: &[Diagnostic]) -> Vec<&Diagnostic> {
    diags
        .iter()
        .filter(|d| {
            d.severity == DiagnosticSeverity::Error
                && matches!(
                    d.code.as_deref(),
                    Some("PL001") | Some("PL002") | Some("PL003")
                )
        })
        .collect()
}

// ---------------------------------------------------------------------------
// 1. Single parse error — no suppression applied
// ---------------------------------------------------------------------------

#[test]
fn single_parse_error_is_preserved() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = ;";
    let errors = vec![ParseError::UnexpectedToken {
        location: 8,
        expected: "expression".to_string(),
        found: ";".to_string(),
    }];

    let diags = run_diagnostics(source, errors);
    let parse_diags = parse_errors_only(&diags);

    assert_eq!(
        parse_diags.len(),
        1,
        "Single parse error must not be suppressed"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 2. Two parse errors far apart — both preserved
// ---------------------------------------------------------------------------

#[test]
fn two_distant_parse_errors_both_preserved() -> Result<(), Box<dyn std::error::Error>> {
    // Errors at offset 0 and offset 50 — well beyond the 10-byte threshold.
    let source = "x x x x x x x x x x x x x x x x x x x x x x x x x x x";
    let errors = vec![
        ParseError::UnexpectedToken {
            location: 0,
            expected: "statement".to_string(),
            found: "x".to_string(),
        },
        ParseError::UnexpectedToken {
            location: 50,
            expected: "statement".to_string(),
            found: "x".to_string(),
        },
    ];

    let diags = run_diagnostics(source, errors);
    let parse_diags = parse_errors_only(&diags);

    assert!(
        parse_diags.len() >= 2,
        "Parse errors 50 bytes apart must both be preserved, got {}",
        parse_diags.len()
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 3. Three cascade errors adjacent (within 10 bytes) — only first kept
// ---------------------------------------------------------------------------

#[test]
fn adjacent_cascade_errors_suppressed_to_one() -> Result<(), Box<dyn std::error::Error>> {
    // Simulate a cascade: errors at offsets 5, 8, 10 — all within 10 bytes of each other.
    // Only the first (offset 5) should survive after cascade suppression.
    let source = "my $x = foo(1, 2, 3\nmy $y = 2;\nmy $z = 3;\n";
    let errors = vec![
        ParseError::UnexpectedToken {
            location: 5,
            expected: "expression".to_string(),
            found: "x".to_string(),
        },
        ParseError::UnexpectedToken {
            location: 8,
            expected: "expression".to_string(),
            found: "foo".to_string(),
        },
        ParseError::UnexpectedToken {
            location: 10,
            expected: "expression".to_string(),
            found: "1".to_string(),
        },
    ];

    let diags = run_diagnostics(source, errors);
    let parse_diags = parse_errors_only(&diags);

    assert_eq!(
        parse_diags.len(),
        1,
        "Three parse errors within 10 bytes should be collapsed to one, got {}",
        parse_diags.len()
    );
    assert_eq!(
        parse_diags[0].range.0, 5,
        "The first (lowest-offset) error should be preserved"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 4. Two clusters — one primary from each cluster preserved
// ---------------------------------------------------------------------------

#[test]
fn two_separate_clusters_one_primary_each() -> Result<(), Box<dyn std::error::Error>> {
    // Cluster A: errors at 0, 3, 7 (all within 10 bytes of first → same cluster)
    // Cluster B: errors at 40, 44, 48 (all within 10 bytes of first → same cluster)
    let source = "x x x x x x x x x x x x x x x x x x x x x x x x x x x x";
    let errors = vec![
        ParseError::UnexpectedToken {
            location: 0,
            expected: "statement".to_string(),
            found: "x".to_string(),
        },
        ParseError::UnexpectedToken {
            location: 3,
            expected: "statement".to_string(),
            found: "x".to_string(),
        },
        ParseError::UnexpectedToken {
            location: 7,
            expected: "statement".to_string(),
            found: "x".to_string(),
        },
        ParseError::UnexpectedToken {
            location: 40,
            expected: "statement".to_string(),
            found: "x".to_string(),
        },
        ParseError::UnexpectedToken {
            location: 44,
            expected: "statement".to_string(),
            found: "x".to_string(),
        },
        ParseError::UnexpectedToken {
            location: 48,
            expected: "statement".to_string(),
            found: "x".to_string(),
        },
    ];

    let diags = run_diagnostics(source, errors);
    let parse_diags = parse_errors_only(&diags);

    assert_eq!(
        parse_diags.len(),
        2,
        "Six errors in two clusters should yield exactly 2 primaries, got {}",
        parse_diags.len()
    );
    // The two survivors must be the cluster heads
    let starts: Vec<usize> = parse_diags.iter().map(|d| d.range.0).collect();
    assert!(
        starts.contains(&0),
        "First cluster head (offset 0) must be preserved"
    );
    assert!(
        starts.contains(&40),
        "Second cluster head (offset 40) must be preserved"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 5. Boundary: exactly 10 bytes apart — still same cluster (within threshold)
// ---------------------------------------------------------------------------

#[test]
fn errors_at_exactly_threshold_boundary_suppressed() -> Result<(), Box<dyn std::error::Error>> {
    // Two errors exactly 10 bytes apart from the same cluster head.
    // They must be in the same cluster (within 10 bytes ≡ distance ≤ 10).
    let source = "x x x x x x x x x x x x x x x x x x x x x x x x x x x x";
    let errors = vec![
        ParseError::UnexpectedToken {
            location: 0,
            expected: "statement".to_string(),
            found: "x".to_string(),
        },
        ParseError::UnexpectedToken {
            location: 10,
            expected: "statement".to_string(),
            found: "x".to_string(),
        },
    ];

    let diags = run_diagnostics(source, errors);
    let parse_diags = parse_errors_only(&diags);

    assert_eq!(
        parse_diags.len(),
        1,
        "Errors 10 bytes apart (at threshold) should collapse to one, got {}",
        parse_diags.len()
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 5b. Boundary-plus-one: exactly 11 bytes apart — treated as new cluster
//
// gap > CASCADE_THRESHOLD_BYTES (10) must create a fresh cluster head.
// This is the complement of test 5: at exactly threshold the error is
// suppressed, but at threshold+1 it becomes a new primary.
// ---------------------------------------------------------------------------

#[test]
fn errors_at_threshold_plus_one_start_new_cluster() -> Result<(), Box<dyn std::error::Error>> {
    // Two errors: head at offset 0, second at offset 11 (= threshold + 1).
    // gap = 11 > 10 → second error starts a new cluster and must be preserved.
    let source = "x x x x x x x x x x x x x x x x x x x x x x x x x x x x";
    let errors = vec![
        ParseError::UnexpectedToken {
            location: 0,
            expected: "statement".to_string(),
            found: "x".to_string(),
        },
        ParseError::UnexpectedToken {
            location: 11,
            expected: "statement".to_string(),
            found: "x".to_string(),
        },
    ];

    let diags = run_diagnostics(source, errors);
    let parse_diags = parse_errors_only(&diags);

    assert_eq!(
        parse_diags.len(),
        2,
        "Errors 11 bytes apart (threshold+1) must both survive as separate cluster heads, got {}",
        parse_diags.len()
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 6. Non-parse-error diagnostics (warnings/hints) are unaffected by suppression
// ---------------------------------------------------------------------------

#[test]
fn non_parse_error_diagnostics_not_suppressed() -> Result<(), Box<dyn std::error::Error>> {
    // Submit no parse errors, just let scope/lint analysis run.
    // Even if scope/lint produces diagnostics close together, they should not
    // be suppressed (cascade suppression only targets parse-error codes).
    let source = "use strict;\nuse warnings;\nmy $x = 1;\nmy $y = 1;\n";
    let ast = empty_program(source.len());
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    let diags = provider.get_diagnostics(&ast, &[], source, None);

    // All diagnostics should have a code (regression check)
    for d in &diags {
        assert!(
            d.code.is_some(),
            "Every diagnostic should carry a code: {d:?}"
        );
    }
    // No parse-error codes should appear (no parse errors submitted)
    let parse_diags = parse_errors_only(&diags);
    assert!(
        parse_diags.is_empty(),
        "No parse errors submitted, none should appear: {parse_diags:?}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 7. Empty input — empty output
// ---------------------------------------------------------------------------

#[test]
fn empty_error_list_produces_empty_parse_diagnostics() -> Result<(), Box<dyn std::error::Error>> {
    let diags = run_diagnostics("", vec![]);
    let parse_diags = parse_errors_only(&diags);
    assert!(
        parse_diags.is_empty(),
        "No errors in, no parse diagnostics out"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 8. Exact duplicates and cascades — both removed
// ---------------------------------------------------------------------------

#[test]
fn exact_duplicate_cascades_all_removed() -> Result<(), Box<dyn std::error::Error>> {
    // Two exact-duplicate errors at offset 5, plus a cascade at offset 7.
    // After dedup: exact-dup collapse to one, then cascade-suppress → one total.
    let source = "my $x = foo(1, 2, 3\nmy $y = 2;\n";
    let errors = vec![
        ParseError::UnexpectedToken {
            location: 5,
            expected: "expression".to_string(),
            found: "foo".to_string(),
        },
        // exact duplicate
        ParseError::UnexpectedToken {
            location: 5,
            expected: "expression".to_string(),
            found: "foo".to_string(),
        },
        // cascade at 7 (within 10 bytes of 5)
        ParseError::UnexpectedToken {
            location: 7,
            expected: "expression".to_string(),
            found: "1".to_string(),
        },
    ];

    let diags = run_diagnostics(source, errors);
    let parse_diags = parse_errors_only(&diags);

    assert_eq!(
        parse_diags.len(),
        1,
        "Exact duplicate + cascade should collapse to a single diagnostic, got {}",
        parse_diags.len()
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 9. Real-world: single missing semicolon → at most 2 error-level diagnostics
//
// Regression guard: the parser's error-recovery is good enough that a missing
// semicolon between two simple assignments produces at most a handful of
// Error-level parse markers.  The v3 recursive descent parser currently emits
// zero PL-code errors for this exact source (it recovers silently), so this
// test primarily guards against parser regressions that would suddenly flood
// the gutter.  See test 9b for a synthetic test that directly exercises the
// cascade suppression path.
// ---------------------------------------------------------------------------

#[test]
fn single_missing_semicolon_produces_at_most_two_parse_errors()
-> Result<(), Box<dyn std::error::Error>> {
    // The v3 parser recovers from a missing semicolon without emitting
    // PL-code errors for this simple two-statement case.  This test guards
    // against future parser regressions that might produce noisy output.
    let source = "my $x = 42\nmy $y = 43;\n";

    let diags = parse_and_diagnose(source);
    let error_diags = error_level_parse_diags(&diags);

    assert!(
        error_diags.len() <= 2,
        "A single missing semicolon should produce at most 2 Error-level parse diagnostics, \
         got {}: {:?}",
        error_diags.len(),
        error_diags.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 9b. Cascade suppression reduces multiple synthetic adjacent errors to one
//
// This directly exercises the suppress_cascades path: we inject 3 errors
// within 10 bytes via run_diagnostics and verify that only the first survives.
// Using error_level_parse_diags (the same filter as the real-world tests)
// confirms the suppression result is visible at the user-facing layer.
// ---------------------------------------------------------------------------

#[test]
fn cascade_suppression_reduces_adjacent_errors_to_one_at_error_level()
-> Result<(), Box<dyn std::error::Error>> {
    // Three errors tightly clustered at offsets 5, 8, 10 — all within the
    // 10-byte threshold of the cluster head (offset 5).  After cascade
    // suppression only the head (offset 5) should remain.
    let source = "my $x = foo(1, 2, 3\nmy $y = 2;\nmy $z = 3;\n";
    let errors = vec![
        ParseError::UnexpectedToken {
            location: 5,
            expected: "expression".to_string(),
            found: "x".to_string(),
        },
        ParseError::UnexpectedToken {
            location: 8,
            expected: "expression".to_string(),
            found: "foo".to_string(),
        },
        ParseError::UnexpectedToken {
            location: 10,
            expected: "expression".to_string(),
            found: "1".to_string(),
        },
    ];

    let diags = run_diagnostics(source, errors);
    let error_diags = error_level_parse_diags(&diags);

    assert_eq!(
        error_diags.len(),
        1,
        "Three cascade errors within 10 bytes should collapse to one at Error level, got {}",
        error_diags.len()
    );
    assert_eq!(
        error_diags[0].range.0, 5,
        "The cluster head (offset 5) should be the survivor"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 10. Real-world: unclosed delimiter causes cascade — only primary survives
//
// An unclosed parenthesis may cause the parser to emit multiple downstream
// errors.  After cascade suppression the user should see at most a small
// number of Error-level diagnostics, not an explosion of them.
// ---------------------------------------------------------------------------

#[test]
fn unclosed_paren_does_not_produce_error_explosion() -> Result<(), Box<dyn std::error::Error>> {
    // An unclosed paren can make the parser confused about every subsequent
    // statement.  Cascade suppression should prevent more than a handful of
    // Error-level markers from reaching the user.
    let source = "my $result = foo(1, 2, 3;\nmy $y = 42;\nmy $z = 99;\n";

    let diags = parse_and_diagnose(source);
    let error_diags = error_level_parse_diags(&diags);

    // Upper bound: even a nasty delimiter cascade should not produce more than
    // 5 Error markers.  The exact count may vary as the parser improves, but
    // the user should never see an explosion.
    assert!(
        error_diags.len() <= 5,
        "Unclosed paren should produce at most 5 Error-level diagnostics after cascade \
         suppression, got {}: {:?}",
        error_diags.len(),
        error_diags.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 11. Real-world: valid Perl produces zero parse-error diagnostics
//
// Regression guard: cascade suppression must not accidentally suppress
// valid-code diagnostics or misfire on clean source.
// ---------------------------------------------------------------------------

#[test]
fn valid_perl_produces_no_parse_error_markers() -> Result<(), Box<dyn std::error::Error>> {
    let source = "use strict;\nuse warnings;\n\nmy $x = 42;\nmy $y = $x + 1;\nprint \"$y\\n\";\n";

    let diags = parse_and_diagnose(source);
    let error_diags = error_level_parse_diags(&diags);

    assert!(
        error_diags.is_empty(),
        "Valid Perl should produce zero Error-level parse diagnostics, got {}: {:?}",
        error_diags.len(),
        error_diags.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 12. Real-world: multiple unrelated syntax errors both reported
//
// Two syntax errors separated by many bytes (different lines) should each
// generate an Error-level diagnostic.  Cascade suppression must not over-
// suppress genuinely independent errors.
// ---------------------------------------------------------------------------

#[test]
fn two_independent_syntax_errors_both_reported() -> Result<(), Box<dyn std::error::Error>> {
    // Synthetic errors at offsets 0 and 60 — well beyond the cascade threshold.
    // Both should survive suppression.
    let source =
        "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx";
    let errors = vec![
        ParseError::UnexpectedToken {
            location: 0,
            expected: "statement".to_string(),
            found: "x".to_string(),
        },
        ParseError::UnexpectedToken {
            location: 60,
            expected: "statement".to_string(),
            found: "x".to_string(),
        },
    ];

    let diags = run_diagnostics(source, errors);
    let error_diags = error_level_parse_diags(&diags);

    assert!(
        error_diags.len() >= 2,
        "Two errors 60 bytes apart should both survive cascade suppression, got {}",
        error_diags.len()
    );
    Ok(())
}
