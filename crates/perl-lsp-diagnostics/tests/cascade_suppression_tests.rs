//! Tests for cascade parse-error suppression in the diagnostics pipeline.
//!
//! Cascade suppression groups adjacent parse-error diagnostics by byte proximity
//! and keeps only the first in each cluster, reducing noise for users.

use std::sync::Arc;

use perl_lsp_diagnostics::{Diagnostic, DiagnosticsProvider};
use perl_parser_core::error::ParseError;
use perl_parser_core::{Node, NodeKind, SourceLocation};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn empty_program(source_len: usize) -> Arc<Node> {
    Arc::new(Node::new(
        NodeKind::Program { statements: vec![] },
        SourceLocation { start: 0, end: source_len },
    ))
}

fn parse_errors_only(diags: &[Diagnostic]) -> Vec<&Diagnostic> {
    diags
        .iter()
        .filter(|d| matches!(d.code.as_deref(), Some("PL001") | Some("PL002") | Some("PL003")))
        .collect()
}

fn run_diagnostics(source: &str, errors: Vec<ParseError>) -> Vec<Diagnostic> {
    let ast = empty_program(source.len());
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    provider.get_diagnostics(&ast, &errors, source, None)
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

    assert_eq!(parse_diags.len(), 1, "Single parse error must not be suppressed");
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
    assert_eq!(parse_diags[0].range.0, 5, "The first (lowest-offset) error should be preserved");
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
    assert!(starts.contains(&0), "First cluster head (offset 0) must be preserved");
    assert!(starts.contains(&40), "Second cluster head (offset 40) must be preserved");
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
        assert!(d.code.is_some(), "Every diagnostic should carry a code: {d:?}");
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
    assert!(parse_diags.is_empty(), "No errors in, no parse diagnostics out");
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
