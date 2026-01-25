#![allow(clippy::unwrap_used, clippy::expect_used)]
use lsp_types::DocumentDiagnosticReport;
use perl_lsp::features::diagnostics::PullDiagnosticsProvider;

#[test]
fn pull_diagnostics_full_then_unchanged() {
    let provider = PullDiagnosticsProvider::new();
    let uri = "file:///test.pl".parse().expect("valid uri");
    let content = "my $x = ;";

    let first = provider.get_document_diagnostics(&uri, content, None);
    let result_id = match &first {
        DocumentDiagnosticReport::Full(full) => {
            let report = &full.full_document_diagnostic_report;
            assert!(!report.items.is_empty(), "expected diagnostics for parse error");
            assert!(
                report.items.iter().all(|item| item.source.as_deref() == Some("perl-lsp")),
                "expected deterministic diagnostic source"
            );
            report.result_id.clone().expect("result id")
        }
        DocumentDiagnosticReport::Unchanged(_) => {
            panic!("expected full diagnostics report for initial request");
        }
    };

    let second = provider.get_document_diagnostics(&uri, content, Some(result_id));
    assert!(
        matches!(second, DocumentDiagnosticReport::Unchanged(_)),
        "expected unchanged diagnostics report on identical content"
    );
}
