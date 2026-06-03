//! BDD tests for the PL410 quick-fix handler (`fix_loop_control_undefined_label`).
//!
//! Each scenario follows:
//!   GIVEN  source containing a `next`/`last`/`redo LABEL` with an undefined label
//!   WHEN   a PL410 diagnostic is supplied and code actions are requested
//!   THEN   exactly one action is returned that drops the label from the statement

use perl_lsp_rs_core::providers::code_actions::{CodeAction, CodeActionKind, CodeActionsProvider};
use perl_lsp_rs_core::providers::diagnostics::{Diagnostic, DiagnosticSeverity};
use perl_parser::Parser;
use perl_tdd_support::must;

// ---------------------------------------------------------------------------
// Helpers (mirror quick_fix_new_codes_bdd.rs)
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

/// Apply edits from an action (sorted reverse by start to avoid offset drift).
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
// Scenario 1 — next with undefined label
// ===========================================================================

#[test]
fn next_undefined_label_produces_drop_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a loop using `next OUTER` where OUTER is not defined
    let source = "for my $i (1..10) { next OUTER; }";
    let stmt_start = source.find("next OUTER").ok_or("marker not found")?;
    let stmt_end = stmt_start + "next OUTER".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`next OUTER` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN there is an action to remove the label
    let action = find_action(&actions, |t| t.contains("OUTER"))
        .ok_or_else(|| format!("no label-drop action in: {:?}", actions))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred, "label-drop action should be preferred");

    // AND applying the edit produces bare `next`
    let result = edited(source, action);
    assert!(
        result.contains("next;") || result.contains("next }"),
        "expected bare next, got: {result}"
    );

    Ok(())
}

// ===========================================================================
// Scenario 2 — last with undefined label
// ===========================================================================

#[test]
fn last_undefined_label_edit_produces_bare_last() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a loop using `last MISSING` where MISSING is not defined
    let source = "while (1) { last MISSING; }";
    let stmt_start = source.find("last MISSING").ok_or("marker not found")?;
    let stmt_end = stmt_start + "last MISSING".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`last MISSING` references a label that is not defined in this file",
    );

    // WHEN code actions are requested and the edit is applied
    let actions = actions_for(source, &[diag]);
    let action = find_action(&actions, |t| t.contains("MISSING"))
        .ok_or_else(|| format!("no label-drop action in: {:?}", actions))?;

    let result = edited(source, action);

    // THEN the label is gone and only `last` remains in the statement
    assert!(
        result.contains("last;") || result.contains("last }"),
        "expected bare last in result, got: {result}"
    );
    assert!(!result.contains("MISSING"), "MISSING label should be removed from result");

    Ok(())
}

// ===========================================================================
// Scenario 3 — redo with undefined label
// ===========================================================================

#[test]
fn redo_undefined_label_edit_produces_bare_redo() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a loop using `redo NOWHERE` where NOWHERE is not defined
    let source = "for my $i (1..5) { redo NOWHERE; }";
    let stmt_start = source.find("redo NOWHERE").ok_or("marker not found")?;
    let stmt_end = stmt_start + "redo NOWHERE".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`redo NOWHERE` references a label that is not defined in this file",
    );

    // WHEN code actions are requested and the edit is applied
    let actions = actions_for(source, &[diag]);
    let action = find_action(&actions, |t| t.contains("NOWHERE"))
        .ok_or_else(|| format!("no label-drop action in: {:?}", actions))?;

    let result = edited(source, action);

    // THEN the label is gone and only `redo` remains in the statement
    assert!(!result.contains("NOWHERE"), "NOWHERE label should be removed from result");

    Ok(())
}

// ===========================================================================
// Scenario 4 — title naming
// ===========================================================================

#[test]
fn action_title_includes_label_name_and_operator() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a `next GHOST` diagnostic
    let source = "for my $x (1..3) { next GHOST; }";
    let stmt_start = source.find("next GHOST").ok_or("marker not found")?;
    let stmt_end = stmt_start + "next GHOST".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`next GHOST` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN the action title names both the label and the operator
    let action = find_action(&actions, |t| t.contains("GHOST"))
        .ok_or_else(|| format!("no GHOST action in: {:?}", actions))?;

    assert_eq!(action.title, "Remove undefined label 'GHOST' from 'next'");

    Ok(())
}

// ===========================================================================
// Scenario 5 — invalid-range guard
// ===========================================================================

#[test]
fn out_of_bounds_range_produces_no_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a diagnostic range that extends beyond the end of the source
    let source = "for my $i (1..5) { next LABEL; }";
    let oob_start = source.len() + 1;
    let oob_end = source.len() + 10;

    let diag = make_diag(oob_start, oob_end, "PL410", "`next LABEL` undefined label");

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN no action is produced (range is invalid)
    assert!(
        !actions.iter().any(|a| a.title.contains("LABEL")),
        "expected no action for out-of-bounds range, got: {:?}",
        actions
    );

    Ok(())
}

// ===========================================================================
// Scenario 6 — dispatch smoke test
// ===========================================================================

#[test]
fn pl410_code_routes_to_fix_handler() -> Result<(), Box<dyn std::error::Error>> {
    // Smoke test: PL410 in the dispatch table reaches the quick-fix handler.
    let source = "while (1) { next PHANTOM; }";
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CodeActionsProvider::new(source.to_string());

    let stmt_start = source.find("next PHANTOM").ok_or("marker not found")?;
    let stmt_end = stmt_start + "next PHANTOM".len();

    let diags = vec![make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`next PHANTOM` references a label that is not defined in this file",
    )];

    let actions = provider.get_code_actions(&ast, (0, source.len()), &diags);

    // THEN at least one action targets the undefined label
    assert!(
        actions.iter().any(|a| a.title.contains("PHANTOM")),
        "PL410 route not producing action; actions: {:?}",
        actions
    );

    Ok(())
}
