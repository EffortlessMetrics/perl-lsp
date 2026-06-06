//! BDD tests for PL409 quick fixes.
//!
//! PL409 fires when a `goto LABEL` statement references a label that is not
//! defined anywhere in the current file. Since `goto` without a label target is
//! invalid Perl, the only sensible quick fix is to remove the entire
//! `goto LABEL;` statement line.

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

fn pl409_actions(actions: &[CodeAction]) -> Vec<&CodeAction> {
    actions
        .iter()
        .filter(|action| action.title == "Remove goto to undefined label")
        .collect()
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

fn first_pl409(source: &str) -> Option<Diagnostic> {
    diagnostics_for(source)
        .into_iter()
        .find(|diagnostic| diagnostic.code.as_deref() == Some("PL409"))
}

// --- Scenario 1: goto with undefined label emits a remove-statement action ---

#[test]
fn code_action_pl409_goto_missing_label_offers_remove_statement_action() {
    let source = "sub foo { goto MISSING; }\n";
    let diagnostic = must_some(first_pl409(source));

    let actions = actions_for(source, &[diagnostic]);
    let pl409 = pl409_actions(&actions);

    assert_eq!(pl409.len(), 1, "expected one PL409 quick fix, got: {actions:?}");
    assert_eq!(pl409[0].kind, CodeActionKind::QuickFix);
    assert!(pl409[0].is_preferred);
}

// --- Scenario 2: the edit removes the entire goto statement line ---

#[test]
fn code_action_pl409_edit_removes_goto_statement_line() {
    let source = "sub foo {\n    goto NOWHERE;\n    return 1;\n}\n";
    let diagnostic = must_some(first_pl409(source));

    let actions = actions_for(source, &[diagnostic]);
    let pl409 = pl409_actions(&actions);

    assert_eq!(pl409.len(), 1, "expected one PL409 quick fix, got: {actions:?}");
    let result = edited(source, pl409[0]);
    assert_eq!(result, "sub foo {\n    return 1;\n}\n");
}

// --- Scenario 3: action title is exactly "Remove goto to undefined label" ---

#[test]
fn code_action_pl409_action_title_is_remove_goto_to_undefined_label() {
    let source = "goto PHANTOM;\n";
    let diagnostic = must_some(first_pl409(source));

    let actions = actions_for(source, &[diagnostic]);
    let pl409 = pl409_actions(&actions);

    assert_eq!(
        pl409.len(),
        1,
        "expected one action titled 'Remove goto to undefined label', got: {actions:?}"
    );
    assert_eq!(pl409[0].title, "Remove goto to undefined label");
}

// --- Scenario 4: defined goto label does not produce an action ---

#[test]
fn code_action_pl409_defined_label_has_no_remove_statement_action() {
    let source = "FOUND: my $x = 1;\ngoto FOUND;\n";
    let diagnostics = diagnostics_for(source);

    let actions = actions_for(source, &diagnostics);

    assert!(
        pl409_actions(&actions).is_empty(),
        "defined label should not offer PL409 quick fix, got: {actions:?}"
    );
}

// --- Scenario 5: a bad (non-char-boundary) diagnostic range returns no action ---

#[test]
fn code_action_pl409_bad_diagnostic_range_returns_no_action() {
    let source = "goto MISSING;\nmy $s = \"\u{e9}\";\n";
    let char_start = must_some(source.find('\u{e9}'));
    let diagnostic = make_diag(
        char_start + 1,
        char_start + 2,
        "PL409",
        "Goto label 'MISSING' is not defined in this file",
    );

    let actions = actions_for(source, &[diagnostic]);

    assert!(
        pl409_actions(&actions).is_empty(),
        "bad diagnostic range should not offer PL409 quick fix, got: {actions:?}"
    );
}

// --- Scenario 6: dispatch smoke test — wrong diagnostic code produces no PL409 action ---

#[test]
fn code_action_pl409_wrong_diagnostic_code_produces_no_remove_goto_action() {
    let source = "goto MISSING;\n";
    let label_start = must_some(source.find("MISSING"));
    let diagnostic = make_diag(
        label_start,
        label_start + "MISSING".len(),
        "PL410",
        "Goto label 'MISSING' is not defined in this file",
    );

    let actions = actions_for(source, &[diagnostic]);

    assert!(
        pl409_actions(&actions).is_empty(),
        "PL410 diagnostic code should not produce 'Remove goto to undefined label' action, got: {actions:?}"
    );
}
