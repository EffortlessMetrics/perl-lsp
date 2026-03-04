use perl_lsp_diagnostic_dedup::deduplicate_diagnostics;
use perl_lsp_diagnostic_types::{Diagnostic, DiagnosticSeverity};

fn mk(
    range: (usize, usize),
    severity: DiagnosticSeverity,
    code: Option<&str>,
    message: &str,
) -> Diagnostic {
    Diagnostic {
        range,
        severity,
        code: code.map(ToString::to_string),
        message: message.to_string(),
        related_information: vec![],
        tags: vec![],
    }
}

#[test]
fn removes_exact_duplicates() {
    let mut diagnostics = vec![
        mk((1, 2), DiagnosticSeverity::Warning, Some("W1"), "hello"),
        mk((1, 2), DiagnosticSeverity::Warning, Some("W1"), "hello"),
        mk((1, 2), DiagnosticSeverity::Warning, Some("W2"), "hello"),
    ];

    deduplicate_diagnostics(&mut diagnostics);

    assert_eq!(diagnostics.len(), 2);
}

#[test]
fn sorts_diagnostics_before_dedup() {
    let mut diagnostics = vec![
        mk((10, 12), DiagnosticSeverity::Warning, None, "b"),
        mk((2, 4), DiagnosticSeverity::Error, None, "a"),
    ];

    deduplicate_diagnostics(&mut diagnostics);

    assert_eq!(diagnostics[0].range, (2, 4));
    assert_eq!(diagnostics[1].range, (10, 12));
}
