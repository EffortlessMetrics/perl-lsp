//! BDD tests for the PL410 quick-fix handler (`fix_loop_control_undefined_label`).
//!
//! `next LABEL`, `last LABEL`, and `redo LABEL` statements that reference an
//! undefined label receive a `PL410` diagnostic. The quick-fix removes the label
//! argument so the statement targets the innermost enclosing loop instead.
//!
//! Scenario index
//! 1. `next MISSING` → action offered with correct title
//! 2. `last MISSING` → action offered with correct title
//! 3. `redo MISSING` → action offered with correct title
//! 4. Title format: "Remove undefined label (use bare '<op>')"
//! 5. Invalid byte-range → no action returned (guard)
//! 6. Dispatch smoke: PL410 code routes to the fix via `CodeActionsProvider`

use perl_lsp_rs_core::providers::code_actions::{CodeAction, CodeActionKind, CodeActionsProvider};
use perl_lsp_rs_core::providers::diagnostics::{Diagnostic, DiagnosticSeverity};
use perl_parser::Parser;
use perl_tdd_support::must;

// ---------------------------------------------------------------------------
// Helpers (mirror the pattern in quick_fix_new_codes_bdd.rs)
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

/// Apply all edits from an action (sorted by descending start) and return the result.
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
// Scenario 1 — next LABEL produces an action
// ===========================================================================

#[test]
fn pl410_next_undefined_label_produces_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a source with `next OUTER` where OUTER is not defined
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

    // THEN there is a quick-fix action for the undefined label
    let action = find_action(&actions, |t| t.contains("next"))
        .ok_or_else(|| format!("no PL410 action in: {actions:?}"))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred, "the fix should be marked preferred (only one sensible option)");

    // AND applying the edit produces bare `next`
    let result = edited(source, action);
    assert!(result.contains("next;") || result.contains("next "), "result: {result}");
    assert!(!result.contains("OUTER"), "label OUTER must be removed: {result}");

    Ok(())
}

// ===========================================================================
// Scenario 2 — last LABEL produces an action
// ===========================================================================

#[test]
fn pl410_last_undefined_label_produces_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a source with `last LOOP` where LOOP is not defined
    let source = "while (1) { last LOOP; }";

    let stmt_start = source.find("last LOOP").ok_or("marker not found")?;
    let stmt_end = stmt_start + "last LOOP".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`last LOOP` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN there is a quick-fix action
    let action = find_action(&actions, |t| t.contains("last"))
        .ok_or_else(|| format!("no PL410 action for last in: {actions:?}"))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);

    // AND applying the edit removes the label
    let result = edited(source, action);
    assert!(!result.contains("LOOP"), "label LOOP must be removed: {result}");

    Ok(())
}

// ===========================================================================
// Scenario 3 — redo LABEL produces an action
// ===========================================================================

#[test]
fn pl410_redo_undefined_label_produces_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a source with `redo ITER` where ITER is not defined
    let source = "for my $i (1..5) { redo ITER; }";

    let stmt_start = source.find("redo ITER").ok_or("marker not found")?;
    let stmt_end = stmt_start + "redo ITER".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`redo ITER` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN there is a quick-fix action
    let action = find_action(&actions, |t| t.contains("redo"))
        .ok_or_else(|| format!("no PL410 action for redo in: {actions:?}"))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);

    // AND applying the edit removes the label
    let result = edited(source, action);
    assert!(!result.contains("ITER"), "label ITER must be removed: {result}");

    Ok(())
}

// ===========================================================================
// Scenario 4 — title format is "Remove undefined label (use bare '<op>')"
// ===========================================================================

#[test]
fn pl410_action_title_names_the_operator() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a `last GHOST` statement
    let source = "while (1) { last GHOST; }";

    let stmt_start = source.find("last GHOST").ok_or("marker not found")?;
    let stmt_end = stmt_start + "last GHOST".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`last GHOST` references a label that is not defined in this file",
    );

    let actions = actions_for(source, &[diag]);

    // THEN the action title follows the convention "Remove undefined label (use bare '<op>')"
    let action = find_action(&actions, |t| t.contains("last"))
        .ok_or_else(|| format!("no PL410 action in: {actions:?}"))?;

    assert_eq!(action.title, "Remove undefined label (use bare 'last')");

    Ok(())
}

// ===========================================================================
// Scenario 5 — invalid byte-range returns no action
// ===========================================================================

#[test]
fn pl410_invalid_range_returns_no_action() {
    // GIVEN a range that extends beyond the source
    let source = "while (1) { next GHOST; }";
    let out_of_bounds_start = source.len() + 1;
    let out_of_bounds_end = source.len() + 10;

    let diag = make_diag(
        out_of_bounds_start,
        out_of_bounds_end,
        "PL410",
        "`next GHOST` references a label that is not defined in this file",
    );

    let actions = actions_for(source, &[diag]);

    // THEN no action is offered — the range guard must reject it
    assert!(
        !actions.iter().any(|a| a.diagnostics.iter().any(|c| c == "PL410")),
        "no PL410 action should be offered for an invalid range, got: {actions:?}"
    );
}

// ===========================================================================
// Scenario 6 — dispatch smoke: PL410 code routes through CodeActionsProvider
// ===========================================================================

#[test]
fn pl410_dispatch_routes_through_provider() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a minimal source and a hand-crafted PL410 diagnostic
    let source = "for my $x (1..3) { next NOWHERE; }";

    let stmt_start = source.find("next NOWHERE").ok_or("marker not found")?;
    let stmt_end = stmt_start + "next NOWHERE".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`next NOWHERE` references a label that is not defined in this file",
    );

    // WHEN code actions are requested via the top-level provider
    let actions = actions_for(source, &[diag]);

    // THEN at least one action is returned (routing is wired)
    assert!(
        !actions.is_empty(),
        "PL410 code must route to fix_loop_control_undefined_label via the provider"
    );

    // AND it carries the expected diagnostic code tag
    let has_pl410_tag = actions.iter().any(|a| a.diagnostics.iter().any(|c| c == "PL410"));
    assert!(has_pl410_tag, "returned action must reference PL410: {actions:?}");

    Ok(())
}
