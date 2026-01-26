use lsp_types::DocumentDiagnosticReport;
use perl_lsp::features::diagnostics::PullDiagnosticsProvider;

#[test]
fn pull_diagnostics_full_then_unchanged() -> Result<(), Box<dyn std::error::Error>> {
    let provider = PullDiagnosticsProvider::new();
    let uri = "file:///test.pl".parse()?;
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
            report.result_id.clone().ok_or("result id missing")?
        }
        DocumentDiagnosticReport::Unchanged(_) => {
            return Err("expected full diagnostics report for initial request".into());
        }
    };

    let second = provider.get_document_diagnostics(&uri, content, Some(result_id));
    assert!(
        matches!(second, DocumentDiagnosticReport::Unchanged(_)),
        "expected unchanged diagnostics report on identical content"
    );

    Ok(())
}
