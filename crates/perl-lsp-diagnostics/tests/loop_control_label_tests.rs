use std::sync::Arc;

use perl_lsp_diagnostics::{Diagnostic, DiagnosticsProvider};
use perl_parser::Parser;

fn diagnostics_for(source: &str) -> Vec<Diagnostic> {
    let output = Parser::new(source).parse_with_recovery();
    let ast = Arc::new(output.ast);
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    provider.get_diagnostics(&ast, &output.diagnostics, source, None)
}

fn pl410_messages(source: &str) -> Vec<String> {
    diagnostics_for(source)
        .into_iter()
        .filter(|d| d.code.as_deref() == Some("PL410"))
        .map(|d| d.message)
        .collect()
}

#[test]
fn next_to_missing_label_warns() {
    let source = "use v5.40;\nwhile (1) { next MISSING; }\n";
    let messages = pl410_messages(source);
    assert!(
        messages.iter().any(|m| m.contains("not defined in this file")),
        "expected PL410 for missing next label, got: {messages:?}"
    );
    assert!(
        messages.iter().any(|m| m.contains("MISSING")),
        "expected the label name in the message, got: {messages:?}"
    );
}

#[test]
fn last_to_missing_label_warns() {
    let source = "use v5.40;\nwhile (1) { last MISSING; }\n";
    let messages = pl410_messages(source);
    assert_eq!(messages.len(), 1, "expected exactly one PL410, got: {messages:?}");
}

#[test]
fn redo_to_missing_label_warns() {
    let source = "use v5.40;\nwhile (1) { redo MISSING; }\n";
    let messages = pl410_messages(source);
    assert_eq!(messages.len(), 1, "expected exactly one PL410, got: {messages:?}");
}

#[test]
fn loop_control_to_existing_label_is_allowed() {
    let source = "use v5.40;\nOUTER: while (1) { next OUTER; }\n";
    let messages = pl410_messages(source);
    assert!(messages.is_empty(), "next to an existing label should not warn, got: {messages:?}");
}

#[test]
fn last_to_existing_outer_label_is_allowed() {
    let source = "use v5.40;\nOUTER: for my $i (1..10) {\n    INNER: for my $j (1..10) {\n        last OUTER if $i * $j > 50;\n    }\n}\n";
    let messages = pl410_messages(source);
    assert!(
        messages.is_empty(),
        "last to a defined outer label should not warn, got: {messages:?}"
    );
}

#[test]
fn unlabeled_loop_control_is_ignored() {
    // Bare `next` / `last` / `redo` with no label always target the innermost
    // enclosing loop and require no validation.
    let source = "use v5.40;\nwhile (1) { next; last; redo; }\n";
    let messages = pl410_messages(source);
    assert!(
        messages.is_empty(),
        "bare loop control without a label should not warn, got: {messages:?}"
    );
}

#[test]
fn label_visible_from_nested_block_is_allowed() {
    let source = "use v5.40;\nOUTER: while (1) {\n    if (1) {\n        last OUTER;\n    }\n}\n";
    let messages = pl410_messages(source);
    assert!(
        messages.is_empty(),
        "label visible from nested block should not warn, got: {messages:?}"
    );
}

#[test]
fn label_is_case_sensitive() {
    // Perl labels are case-sensitive; `outer` does not match `OUTER`.
    let source = "use v5.40;\nOUTER: while (1) { next outer; }\n";
    let messages = pl410_messages(source);
    assert!(
        messages.iter().any(|m| m.contains("'outer'")),
        "expected PL410 for case-mismatched label, got: {messages:?}"
    );
}

#[test]
fn diagnostic_carries_pl410_code_and_warning_severity() {
    use perl_lsp_diagnostics::DiagnosticSeverity;

    let source = "use v5.40;\nwhile (1) { next NOPE; }\n";
    let pl410: Vec<_> = diagnostics_for(source)
        .into_iter()
        .filter(|d| d.code.as_deref() == Some("PL410"))
        .collect();
    assert_eq!(pl410.len(), 1);
    assert_eq!(pl410[0].severity, DiagnosticSeverity::Warning);
    assert!(
        !pl410[0].related_information.is_empty(),
        "PL410 should include related information explaining the fix"
    );
    assert!(pl410[0].suggestion.is_some(), "PL410 should include a suggestion");
}

#[test]
fn forward_reference_to_label_in_same_file_is_allowed() {
    // The lint is intentionally file-scoped (matches PL409). Forward references
    // to a label declared later in the file should not warn — this avoids
    // false positives for codepaths where the loop is hoisted or organized
    // differently. Tighter scope-aware validation can layer on later.
    let source = "use v5.40;\nsub run { last DONE }\nDONE: while (0) {}\n";
    let messages = pl410_messages(source);
    assert!(
        messages.is_empty(),
        "forward-declared label in the same file should not warn, got: {messages:?}"
    );
}
