//! Integration tests: PerformanceMonitor wired into PullDiagnosticsProvider.
//!
//! Verifies that timing is tracked for the diagnostic generation phases
//! (parse, scope_analysis, lint, deduplication) when pull diagnostics run.

use lsp_types::Uri;
use perl_lsp::features::diagnostics::pull::PullDiagnosticsProvider;

#[test]
fn test_pull_diagnostics_performance_tracking_returns_valid_diagnostics()
-> Result<(), Box<dyn std::error::Error>> {
    // The key acceptance criterion: get_document_diagnostics_with_metrics must
    // return the same diagnostics as get_document_diagnostics, and must also
    // return a non-empty metrics map.
    let provider = PullDiagnosticsProvider::new();
    let uri: Uri = "file:///test.pl".parse()?;
    let content = "print 'hello';\n";

    let (report, metrics) = provider.get_document_diagnostics_with_metrics(&uri, content, None);

    // Must return a valid diagnostic report (not crash or hang)
    match report {
        lsp_types::DocumentDiagnosticReport::Full(ref full) => {
            // parse + scope-analysis at minimum must be present
            assert!(
                !full.full_document_diagnostic_report.items.is_empty() || content.trim().is_empty(),
                "expected diagnostics for bare print without strict/warnings"
            );
        }
        lsp_types::DocumentDiagnosticReport::Unchanged(_) => {
            // acceptable if content was unchanged
        }
    }

    // Metrics must contain at least the "parse" operation
    assert!(
        metrics.contains_key("parse"),
        "metrics must record 'parse' phase; got: {:?}",
        metrics.keys().collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn test_pull_diagnostics_performance_metrics_contain_required_phases()
-> Result<(), Box<dyn std::error::Error>> {
    let provider = PullDiagnosticsProvider::new();
    let uri: Uri = "file:///test.pl".parse()?;
    // Content that exercises all phases: parse errors, unused vars, lint
    let content = "use strict; use warnings;\nmy $unused = 1;\n";

    let (_report, metrics) = provider.get_document_diagnostics_with_metrics(&uri, content, None);

    // All four phases from the acceptance criteria must be tracked
    let required_phases = ["parse", "scope_analysis", "lint", "deduplication"];
    for phase in &required_phases {
        assert!(
            metrics.contains_key(*phase),
            "metrics must contain '{}' phase; got keys: {:?}",
            phase,
            metrics.keys().collect::<Vec<_>>()
        );
    }
    Ok(())
}

#[test]
fn test_pull_diagnostics_performance_call_counts_are_one_per_request()
-> Result<(), Box<dyn std::error::Error>> {
    let provider = PullDiagnosticsProvider::new();
    let uri: Uri = "file:///test.pl".parse()?;
    let content = "my $x = 1;\n";

    let (_report, metrics) = provider.get_document_diagnostics_with_metrics(&uri, content, None);

    // Each phase must be called exactly once per diagnostic request
    for phase in &["parse", "scope_analysis", "lint", "deduplication"] {
        if let Some(entry) = metrics.get(*phase) {
            assert_eq!(
                entry.call_count, 1,
                "'{}' must have call_count=1 per request, got {}",
                phase, entry.call_count
            );
        }
    }
    Ok(())
}
