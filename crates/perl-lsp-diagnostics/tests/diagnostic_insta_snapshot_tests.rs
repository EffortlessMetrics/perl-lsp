//! Insta snapshots for full diagnostic payload coverage.
//!
//! These tests complement code-level assertions by snapshotting normalized
//! diagnostics (code, severity, range, message, suggestion) for representative
//! snippets. This catches regressions in message text and location metadata, not
//! just code presence.

use std::sync::Arc;

use insta::assert_snapshot;
use perl_lsp_diagnostics::{Diagnostic, DiagnosticSeverity, DiagnosticsProvider};
use perl_parser::Parser;

fn diagnostics_for(source: &str) -> Vec<Diagnostic> {
    let output = Parser::new(source).parse_with_recovery();
    let ast = Arc::new(output.ast);
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    provider.get_diagnostics(&ast, &output.diagnostics, source, None)
}

fn severity_name(severity: DiagnosticSeverity) -> &'static str {
    match severity {
        DiagnosticSeverity::Error => "Error",
        DiagnosticSeverity::Warning => "Warning",
        DiagnosticSeverity::Information => "Information",
        DiagnosticSeverity::Hint => "Hint",
    }
}

fn normalize(diags: Vec<Diagnostic>) -> String {
    let mut normalized: Vec<_> = diags
        .into_iter()
        .map(|diag| {
            let code = diag.code.unwrap_or_else(|| "<none>".to_string());
            let suggestion = diag.suggestion.unwrap_or_else(|| "<none>".to_string());
            format!(
                "{code} | {} | {:?} | {} | {suggestion}",
                severity_name(diag.severity),
                diag.range,
                diag.message
            )
        })
        .collect();

    normalized.sort_unstable();
    normalized.join("\n")
}

#[test]
fn snapshot_script_happy_path() {
    let source = "use strict;\nuse warnings;\nmy $x = 1;\nprint $x;\n";
    let snapshot = normalize(diagnostics_for(source));
    assert_snapshot!("script_happy_path", snapshot);
}

#[test]
fn snapshot_package_module_happy_path() {
    let source = concat!(
        "package Foo;\n",
        "use strict;\n",
        "use warnings;\n",
        "sub value { return 42 }\n",
        "1;\n",
    );
    let snapshot = normalize(diagnostics_for(source));
    assert_snapshot!("package_module_happy_path", snapshot);
}

#[test]
fn snapshot_missing_pragmas_and_unused_variable() {
    let source = "my $unused = 1;\n";
    let snapshot = normalize(diagnostics_for(source));
    assert_snapshot!("missing_pragmas_and_unused_variable", snapshot);
}

#[test]
fn snapshot_security_string_eval() {
    let source = concat!(
        "package Foo;\n",
        "use strict;\n",
        "use warnings;\n",
        "eval(\"system('rm -rf /')\");\n",
        "1;\n",
    );
    let snapshot = normalize(diagnostics_for(source));
    assert_snapshot!("security_string_eval", snapshot);
}
