use std::sync::Arc;

use perl_lsp_diagnostics::{Diagnostic, DiagnosticsProvider};
use perl_parser::Parser;

fn diagnostics_for(source: &str) -> Vec<Diagnostic> {
    let output = Parser::new(source).parse_with_recovery();
    let ast = Arc::new(output.ast);
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    provider.get_diagnostics(&ast, &output.diagnostics, source, None)
}

fn pl409_messages(source: &str) -> Vec<String> {
    diagnostics_for(source)
        .into_iter()
        .filter(|d| d.code.as_deref() == Some("PL409"))
        .map(|d| d.message)
        .collect()
}

#[test]
fn goto_to_missing_label_warns() {
    let source = "use v5.40;\ngoto MISSING;\n";
    let messages = pl409_messages(source);
    assert!(
        messages.iter().any(|m| m.contains("not defined in this file")),
        "expected PL409 for missing goto label, got: {messages:?}"
    );
}

#[test]
fn goto_to_existing_label_is_allowed() {
    let source = "use v5.40;\nSTART: goto START;\n";
    let messages = pl409_messages(source);
    assert!(messages.is_empty(), "goto to an existing label should not warn, got: {messages:?}");
}

#[test]
fn dynamic_goto_forms_are_ignored() {
    let source = "use v5.40;\ngoto &target;\ngoto $label;\n";
    let messages = pl409_messages(source);
    assert!(
        messages.is_empty(),
        "dynamic goto forms should not be validated as labels, got: {messages:?}"
    );
}
