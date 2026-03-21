use lsp_types::{DocumentDiagnosticReport, NumberOrString};
use perl_lsp::features::diagnostics::PullDiagnosticsProvider;

/// Extract items from a full diagnostic report, returning an error if it is Unchanged.
fn items_from_report(
    report: DocumentDiagnosticReport,
) -> Result<Vec<lsp_types::Diagnostic>, Box<dyn std::error::Error>> {
    match report {
        DocumentDiagnosticReport::Full(full) => Ok(full.full_document_diagnostic_report.items),
        DocumentDiagnosticReport::Unchanged(_) => {
            Err("expected Full diagnostic report, got Unchanged".into())
        }
    }
}

/// Returns true when a diagnostic has the given code string (e.g. "PL102").
fn has_code(diag: &lsp_types::Diagnostic, code: &str) -> bool {
    matches!(&diag.code, Some(NumberOrString::String(s)) if s == code)
}

#[test]
fn pull_diagnostics_unused_variable_emits_pl102() -> Result<(), Box<dyn std::error::Error>> {
    let provider = PullDiagnosticsProvider::new();
    let uri = "file:///test_unused.pl".parse()?;
    // $used is referenced; $unused is not — should produce PL102 for $unused only.
    let content = "use strict;\nuse warnings;\nsub foo {\n    my $used = 123;\n    my $unused = 456;\n    return $used;\n}\n";

    let items = items_from_report(provider.get_document_diagnostics(&uri, content, None))?;

    let pl102_diags: Vec<_> = items.iter().filter(|d| has_code(d, "PL102")).collect();
    if pl102_diags.is_empty() {
        return Err(format!(
            "Expected at least one PL102 (unused variable) diagnostic, got none.\nAll diagnostics: {items:#?}"
        )
        .into());
    }

    // At least one PL102 must mention $unused
    let mentions_unused =
        pl102_diags.iter().any(|d| d.message.contains("$unused") || d.message.contains("unused"));
    if !mentions_unused {
        return Err(format!(
            "Expected a PL102 diagnostic mentioning '$unused', got: {pl102_diags:#?}"
        )
        .into());
    }

    // No PL102 should mention $used (it is referenced)
    let false_positive =
        pl102_diags.iter().any(|d| d.message.contains("$used") && !d.message.contains("$unused"));
    if false_positive {
        return Err(format!(
            "Unexpected PL102 diagnostic for '$used' (it is referenced): {pl102_diags:#?}"
        )
        .into());
    }

    Ok(())
}

#[test]
fn pull_diagnostics_unused_variable_severity_is_warning() -> Result<(), Box<dyn std::error::Error>>
{
    let provider = PullDiagnosticsProvider::new();
    let uri = "file:///test_unused_sev.pl".parse()?;
    let content = "sub bar {\n    my $never_used = 1;\n}\n";

    let items = items_from_report(provider.get_document_diagnostics(&uri, content, None))?;

    let pl102 = items
        .iter()
        .find(|d| has_code(d, "PL102"))
        .ok_or("Expected PL102 diagnostic for unused variable $never_used")?;

    let severity = pl102.severity.ok_or("PL102 diagnostic must have a severity")?;
    if severity != lsp_types::DiagnosticSeverity::WARNING {
        return Err(format!("Expected WARNING severity for PL102, got {:?}", severity).into());
    }

    Ok(())
}

#[test]
fn pull_diagnostics_underscore_prefix_suppresses_unused_warning()
-> Result<(), Box<dyn std::error::Error>> {
    let provider = PullDiagnosticsProvider::new();
    let uri = "file:///test_underscore.pl".parse()?;
    // _intentionally_unused should NOT produce PL102 (underscore prefix = intentionally unused)
    let content = "sub baz {\n    my $_intentionally_unused = 1;\n    return 42;\n}\n";

    let items = items_from_report(provider.get_document_diagnostics(&uri, content, None))?;

    let pl102_for_underscore =
        items.iter().find(|d| has_code(d, "PL102") && d.message.contains("_intentionally_unused"));

    if pl102_for_underscore.is_some() {
        return Err(
            "PL102 must NOT be emitted for underscore-prefixed variable $_intentionally_unused"
                .into(),
        );
    }

    Ok(())
}

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
