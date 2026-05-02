//! Diagnostics integration tests for dynamic import boundaries.

use std::sync::Arc;

use perl_diagnostics::codes::DiagnosticCode;
use perl_lsp_rs_core::providers::diagnostics::{Diagnostic, DiagnosticsProvider};
use perl_parser::Parser;

fn diagnostics_for(source: &str) -> Vec<Diagnostic> {
    let output = Parser::new(source).parse_with_recovery();
    let ast = Arc::new(output.ast);
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    provider.get_diagnostics(&ast, &output.diagnostics, source, None)
}

fn unquoted_bareword_diagnostics(source: &str) -> Vec<Diagnostic> {
    diagnostics_for(source)
        .into_iter()
        .filter(|diag| diag.code.as_deref() == Some(DiagnosticCode::UnquotedBareword.as_str()))
        .collect()
}

#[test]
fn dynamic_require_import_suppresses_unquoted_bareword_diagnostic()
-> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
use strict 'subs';
my $module = 'Dynamic::Loader';
require $module;
$module->import(qw(dynamic_func));
print dynamic_func;
"#;

    let diags = unquoted_bareword_diagnostics(source);

    assert!(
        diags.is_empty(),
        "dynamic require/import boundary should suppress exact PL109 for imported bareword: {diags:?}"
    );
    Ok(())
}

#[test]
fn runtime_import_symbol_list_keeps_unquoted_bareword_diagnostic()
-> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
use strict 'subs';
my $module = 'Dynamic::Loader';
my @names = ('dynamic_func');
require $module;
$module->import(@names);
print dynamic_func;
"#;

    let diags = unquoted_bareword_diagnostics(source);

    assert_eq!(
        diags.len(),
        1,
        "runtime-computed import lists should not claim exact imported barewords: {diags:?}"
    );
    Ok(())
}

#[test]
fn ordinary_missing_bareword_still_warns() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
use strict 'subs';
print still_missing;
"#;

    let diags = unquoted_bareword_diagnostics(source);

    assert_eq!(diags.len(), 1, "normal strict-subs bareword should still emit PL109");
    Ok(())
}
