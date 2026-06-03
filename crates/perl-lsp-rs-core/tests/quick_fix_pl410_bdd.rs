//! BDD tests for the PL410 quick-fix handler:
//!   PL410 — `next LABEL`/`last LABEL`/`redo LABEL` with an undefined label
//!           (`fix_loop_control_undefined_label`)
//!
//! Each scenario follows the pattern:
//!   GIVEN  source with a loop-control statement that targets an undefined label
//!   WHEN   a PL410 diagnostic is produced and code actions are requested
//!   THEN   exactly one preferred action is returned that drops the label,
//!          leaving the bare operator to target the innermost enclosing loop

use perl_lsp_rs_core::providers::code_actions::{CodeAction, CodeActionKind, CodeActionsProvider};
use perl_lsp_rs_core::providers::diagnostics::{Diagnostic, DiagnosticSeverity};
use perl_parser::Parser;
use perl_tdd_support::must;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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

fn actions_for(source: &str, diags: &[Diagnostic]) -> Vec<CodeAction> {
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CodeActionsProvider::new(source.to_string());
    provider.get_code_actions(&ast, (0, source.len()), diags)
}

/// Apply all edits from an action and return the resulting source.
fn edited(source: &str, action: &CodeAction) -> String {
    let mut edits = action.edit.changes.clone();
    edits.sort_by(|a, b| b.location.start.cmp(&a.location.start));
    let mut out = source.to_string();
    for edit in edits {
        out.replace_range(edit.location.start..edit.location.end, &edit.new_text);
    }
    out
}

fn find_action<'a>(
    actions: &'a [CodeAction],
    pred: impl Fn(&str) -> bool,
) -> Option<&'a CodeAction> {
    actions.iter().find(|a| pred(&a.title))
}

// ===========================================================================
// PL410 — next LABEL with undefined label
// ===========================================================================

#[test]
fn pl410_next_with_undefined_label_produces_remove_action() -> Result<(), Box<dyn std::error::Error>>
{
    // GIVEN a loop body that uses `next INNER` where INNER is not defined
    let source = "while (1) {\n    next INNER;\n}\n";
    let next_start = source.find("next INNER").ok_or("marker not found")?;
    let next_end = next_start + "next INNER".len();

    let diag = make_diag(
        next_start,
        next_end,
        "PL410",
        "`next INNER` references a label that is not defined in this file",
    );

    // WHEN code actions are requested for the diagnostic range
    let actions = actions_for(source, &[diag]);

    // THEN there is an action offering to remove the undefined label
    let action = find_action(&actions, |t| t.contains("INNER"))
        .ok_or_else(|| format!("no remove-label action in: {actions:?}"))?;

    assert_eq!(action.title, "Remove undefined label 'INNER' from 'next INNER'");
    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred, "the remove-label action should be preferred");

    Ok(())
}

#[test]
fn pl410_last_with_undefined_label_produces_remove_action() -> Result<(), Box<dyn std::error::Error>>
{
    // GIVEN a loop body that uses `last OUTER` where OUTER is not defined
    let source = "while (1) {\n    last OUTER;\n}\n";
    let last_start = source.find("last OUTER").ok_or("marker not found")?;
    let last_end = last_start + "last OUTER".len();

    let diag = make_diag(
        last_start,
        last_end,
        "PL410",
        "`last OUTER` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN there is an action offering to remove the label
    let action = find_action(&actions, |t| t.contains("OUTER"))
        .ok_or_else(|| format!("no remove-label action in: {actions:?}"))?;

    assert_eq!(action.title, "Remove undefined label 'OUTER' from 'last OUTER'");
    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred);

    Ok(())
}

#[test]
fn pl410_redo_with_undefined_label_produces_remove_action() -> Result<(), Box<dyn std::error::Error>>
{
    // GIVEN a loop body that uses `redo LOOP` where LOOP is not defined
    let source = "for my $i (1..10) {\n    redo LOOP;\n}\n";
    let redo_start = source.find("redo LOOP").ok_or("marker not found")?;
    let redo_end = redo_start + "redo LOOP".len();

    let diag = make_diag(
        redo_start,
        redo_end,
        "PL410",
        "`redo LOOP` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN there is an action offering to remove the label
    let action = find_action(&actions, |t| t.contains("LOOP"))
        .ok_or_else(|| format!("no remove-label action in: {actions:?}"))?;

    assert_eq!(action.title, "Remove undefined label 'LOOP' from 'redo LOOP'");
    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred);

    Ok(())
}

#[test]
fn pl410_edit_strips_label_leaving_bare_operator() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a source with an undefined-label next
    let source = "while (1) {\n    next MISSING;\n}\n";
    let next_start = source.find("next MISSING").ok_or("marker not found")?;
    let next_end = next_start + "next MISSING".len();

    let diag = make_diag(
        next_start,
        next_end,
        "PL410",
        "`next MISSING` references a label that is not defined in this file",
    );

    let actions = actions_for(source, &[diag]);
    let action = find_action(&actions, |t| t.contains("MISSING"))
        .ok_or_else(|| format!("no remove-label action in: {actions:?}"))?;

    // WHEN the edit is applied
    let result = edited(source, action);

    // THEN the label is stripped and the bare operator remains, leaving valid Perl
    assert_eq!(result, "while (1) {\n    next;\n}\n");

    Ok(())
}

#[test]
fn pl410_invalid_range_produces_no_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a PL410 diagnostic whose range falls inside a multi-byte character
    // (simulates a misaligned or synthetic diagnostic)
    let source = "while (1) {\n    next INNER;\n}\nmy $x = \"\u{e9}\";\n";
    let char_start = source.find('\u{e9}').ok_or("marker not found")?;
    // Deliberately point into the interior of the multi-byte UTF-8 sequence
    let diag = make_diag(
        char_start + 1,
        char_start + 2,
        "PL410",
        "`next INNER` references a label that is not defined in this file",
    );

    let actions = actions_for(source, &[diag]);

    // THEN no remove-label action is produced
    assert!(
        !actions.iter().any(|a| a.title.contains("Remove undefined label")),
        "expected no PL410 action for non-char-boundary range, got: {actions:?}"
    );

    Ok(())
}

// ===========================================================================
// Cross-cutting: dispatch-table smoke test
// ===========================================================================

#[test]
fn pl410_dispatch_smoke_test() -> Result<(), Box<dyn std::error::Error>> {
    // Smoke test: PL410 diagnostics reach the fix_loop_control_undefined_label handler,
    // confirming the routing in diagnostic_routes.rs is wired up correctly.
    let source = "while (1) {\n    next FOO;\n    last BAR;\n}\n";
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CodeActionsProvider::new(source.to_string());

    let next_start = source.find("next FOO").ok_or("next FOO not found")?;
    let next_end = next_start + "next FOO".len();
    let last_start = source.find("last BAR").ok_or("last BAR not found")?;
    let last_end = last_start + "last BAR".len();

    let diags = vec![
        make_diag(
            next_start,
            next_end,
            "PL410",
            "`next FOO` references a label that is not defined in this file",
        ),
        make_diag(
            last_start,
            last_end,
            "PL410",
            "`last BAR` references a label that is not defined in this file",
        ),
    ];

    let actions = provider.get_code_actions(&ast, (0, source.len()), &diags);

    let has_next_fix = actions.iter().any(|a| a.title.contains("'FOO'"));
    let has_last_fix = actions.iter().any(|a| a.title.contains("'BAR'"));

    assert!(has_next_fix, "PL410 route not producing action for 'next FOO'; actions: {actions:?}");
    assert!(has_last_fix, "PL410 route not producing action for 'last BAR'; actions: {actions:?}");

    Ok(())
}
