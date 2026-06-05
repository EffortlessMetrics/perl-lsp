//! BDD tests for PL410 quick fixes.
//!
//! PL410 already reports `next`/`last`/`redo LABEL` when the label is not
//! defined in the file. The quick fix must offer one deterministic edit:
//! remove the undefined label and leave the bare loop-control operator.

use std::cmp::Reverse;
use std::sync::Arc;

use perl_lsp_rs_core::providers::code_actions::{CodeAction, CodeActionKind, CodeActionsProvider};
use perl_lsp_rs_core::providers::diagnostics::{
    Diagnostic, DiagnosticSeverity, DiagnosticsProvider,
};
use perl_parser::Parser;
use perl_tdd_support::{must, must_some};

fn make_diag(start: usize, end: usize, code: &str, message: &str) -> Diagnostic {
    Diagnostic {
        range: (start, end),
        severity: DiagnosticSeverity::Warning,
        code: Some(code.to_string()),
        message: message.to_string(),
        related_information: Vec::new(),
        tags: Vec::new(),
        suggestion: None,
    }
}

fn diagnostics_for(source: &str) -> Vec<Diagnostic> {
    let output = Parser::new(source).parse_with_recovery();
    let ast = Arc::new(output.ast);
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    provider.get_diagnostics(&ast, &output.diagnostics, source, None)
}

fn actions_for(source: &str, diagnostics: &[Diagnostic]) -> Vec<CodeAction> {
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CodeActionsProvider::new(source.to_string());
    provider.get_code_actions(&ast, (0, source.len()), diagnostics)
}

fn pl410_actions(actions: &[CodeAction]) -> Vec<&CodeAction> {
    actions.iter().filter(|action| action.title == "Remove undefined label").collect()
}

fn edited(source: &str, action: &CodeAction) -> String {
    let mut edits = action.edit.changes.clone();
    edits.sort_by_key(|edit| Reverse(edit.location.start));

    let mut output = source.to_string();
    for edit in edits {
        output.replace_range(edit.location.start..edit.location.end, &edit.new_text);
    }
    output
}

fn first_pl410(source: &str) -> Option<Diagnostic> {
    diagnostics_for(source)
        .into_iter()
        .find(|diagnostic| diagnostic.code.as_deref() == Some("PL410"))
}

#[test]
fn code_action_pl410_next_missing_label_offers_remove_label_action() {
    let source = "for my $i (1..5) { next OUTER; }\n";
    let diagnostic = must_some(first_pl410(source));

    let actions = actions_for(source, &[diagnostic]);
    let pl410 = pl410_actions(&actions);

    assert_eq!(pl410.len(), 1, "expected one PL410 quick fix, got: {actions:?}");
    assert_eq!(pl410[0].kind, CodeActionKind::QuickFix);
    assert!(pl410[0].is_preferred);
    assert_eq!(edited(source, pl410[0]), "for my $i (1..5) { next; }\n");
}

#[test]
fn code_action_pl410_last_missing_label_offers_remove_label_action() {
    let source = "while (1) { last MISSING; }\n";
    let diagnostic = must_some(first_pl410(source));

    let actions = actions_for(source, &[diagnostic]);
    let pl410 = pl410_actions(&actions);

    assert_eq!(pl410.len(), 1, "expected one PL410 quick fix, got: {actions:?}");
    assert_eq!(edited(source, pl410[0]), "while (1) { last; }\n");
}

#[test]
fn code_action_pl410_redo_missing_label_offers_remove_label_action() {
    let source = "for my $i (1..5) { redo NOWHERE; }\n";
    let diagnostic = must_some(first_pl410(source));

    let actions = actions_for(source, &[diagnostic]);
    let pl410 = pl410_actions(&actions);

    assert_eq!(pl410.len(), 1, "expected one PL410 quick fix, got: {actions:?}");
    assert_eq!(edited(source, pl410[0]), "for my $i (1..5) { redo; }\n");
}

#[test]
fn code_action_pl410_range_without_semicolon_removes_label_before_semicolon() {
    let source = "while (1) { next MISSING; }\n";
    let statement_start = must_some(source.find("next"));
    let statement_end = statement_start + "next MISSING".len();
    let diagnostic = make_diag(
        statement_start,
        statement_end,
        "PL410",
        "`next MISSING` references a label that is not defined in this file",
    );

    let actions = actions_for(source, &[diagnostic]);
    let pl410 = pl410_actions(&actions);

    assert_eq!(pl410.len(), 1, "expected one PL410 quick fix, got: {actions:?}");
    assert_eq!(edited(source, pl410[0]), "while (1) { next; }\n");
}

#[test]
fn code_action_pl410_defined_label_has_no_remove_label_action() {
    let source = "OUTER: for my $i (1..5) { next OUTER; }\n";
    let diagnostics = diagnostics_for(source);

    let actions = actions_for(source, &diagnostics);

    assert!(
        pl410_actions(&actions).is_empty(),
        "defined label should not offer PL410 quick fix, got: {actions:?}"
    );
}

#[test]
fn code_action_pl410_bare_operator_diagnostic_returns_no_remove_label_action() {
    let source = "while (1) { next; }\n";
    let statement_start = must_some(source.find("next"));
    let diagnostic = make_diag(
        statement_start,
        statement_start + "next".len(),
        "PL410",
        "`next` references a label that is not defined in this file",
    );

    let actions = actions_for(source, &[diagnostic]);

    assert!(
        pl410_actions(&actions).is_empty(),
        "bare operator should not offer PL410 quick fix, got: {actions:?}"
    );
}

#[test]
fn code_action_pl410_bad_diagnostic_range_returns_no_remove_label_action() {
    let source = "for my $i (1..5) { next OUTER; }\nmy $s = \"\u{e9}\";\n";
    let char_start = must_some(source.find('\u{e9}'));
    let diagnostic = make_diag(
        char_start + 1,
        char_start + 2,
        "PL410",
        "`next OUTER` references a label that is not defined in this file",
    );

    let actions = actions_for(source, &[diagnostic]);

    assert!(
        pl410_actions(&actions).is_empty(),
        "bad diagnostic range should not offer PL410 quick fix, got: {actions:?}"
    );
}

#[test]
fn code_action_pl410_misrouted_diagnostic_text_returns_no_remove_label_action() {
    let source = "my $x = 1;\n";
    let var_start = must_some(source.find("$x"));
    let diagnostic = make_diag(
        var_start,
        var_start + "$x".len(),
        "PL410",
        "`next OUTER` references a label that is not defined in this file",
    );

    let actions = actions_for(source, &[diagnostic]);

    assert!(
        pl410_actions(&actions).is_empty(),
        "misrouted diagnostic should not offer PL410 quick fix, got: {actions:?}"
    );
}
