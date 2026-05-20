//! Insta snapshots for full diagnostic payload coverage.
//!
//! These tests complement code-level assertions by snapshotting normalized
//! diagnostics (code, severity, range, message, suggestion) for representative
//! snippets. This catches regressions in message text and location metadata, not
//! just code presence.

use std::sync::Arc;

use insta::assert_snapshot;
use perl_lsp_rs_core::providers::diagnostics::{
    Diagnostic, DiagnosticSeverity, DiagnosticsProvider,
};
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

#[test]
fn snapshot_missing_module_import() {
    let source = concat!(
        "package Foo;\n",
        "use strict;\n",
        "use warnings;\n",
        "use Does::Not::Exist;\n",
        "1;\n",
    );
    let snapshot = normalize(diagnostics_for(source));
    assert_snapshot!("missing_module_import", snapshot);
}

#[test]
fn snapshot_syntax_error_with_follow_on_statement() {
    let source =
        concat!("use strict;\n", "use warnings;\n", "my $x = ;\n", "my $y = 2;\n", "print $y;\n");
    let snapshot = normalize(diagnostics_for(source));
    assert_snapshot!("syntax_error_with_follow_on_statement", snapshot);
}

#[test]
fn snapshot_multiple_missing_pragmas_and_eval() {
    let source = concat!(
        "package Legacy::Script;\n",
        "my $payload = q{print \"hello\\n\"};\n",
        "eval $payload;\n",
        "my $unused = 42;\n",
        "1;\n",
    );
    let snapshot = normalize(diagnostics_for(source));
    assert_snapshot!("multiple_missing_pragmas_and_eval", snapshot);
}

#[test]
fn snapshot_duplicate_declaration_and_shadowing() {
    let source = concat!(
        "use strict;\n",
        "use warnings;\n",
        "my $value = 1;\n",
        "my $value = 2;\n",
        "print $value;\n",
    );
    let snapshot = normalize(diagnostics_for(source));
    assert_snapshot!("duplicate_declaration_and_shadowing", snapshot);
}

#[test]
fn snapshot_unused_import_hint() {
    let source = concat!(
        "use strict;\n",
        "use warnings;\n",
        "use File::Spec;\n",
        "my $value = 1;\n",
        "print $value;\n",
    );
    let snapshot = normalize(diagnostics_for(source));
    assert_snapshot!("unused_import_hint", snapshot);
}

#[test]
fn snapshot_phase_scoped_pragma_warning() {
    let source = concat!(
        "BEGIN {\n",
        "    use strict;\n",
        "    use warnings;\n",
        "}\n",
        "my $value = 1;\n",
        "print $value;\n",
    );
    let snapshot = normalize(diagnostics_for(source));
    assert_snapshot!("phase_scoped_pragma_warning", snapshot);
}

#[test]
fn snapshot_suspicious_regex_and_tainted_system_call() {
    let source = concat!(
        "use strict;\n",
        "use warnings;\n",
        "my $input = <STDIN>;\n",
        "if ($input =~ /^(a+)+$/) {\n",
        "    system($input);\n",
        "}\n",
    );
    let snapshot = normalize(diagnostics_for(source));
    assert_snapshot!("suspicious_regex_and_tainted_system_call", snapshot);
}
