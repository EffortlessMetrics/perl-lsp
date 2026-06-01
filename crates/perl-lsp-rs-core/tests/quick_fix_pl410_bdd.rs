//! BDD tests for the PL410 quick-fix handler (`fix_loop_control_undefined_label`).
//!
//! PL410 fires when `next LABEL`, `last LABEL`, or `redo LABEL` references a label
//! that is not defined anywhere in the file. The fix drops the label so the statement
//! targets the innermost enclosing loop instead.
//!
//! Scenarios
//! ---------
//! 1. `next LABEL` diagnostic produces a drop-label action
//! 2. `last LABEL` diagnostic produces a drop-label action
//! 3. `redo LABEL` diagnostic produces a drop-label action
//! 4. Action title names the operator correctly
//! 5. Out-of-bounds range returns no actions (guard)
//! 6. PL410 code reaches the handler via the dispatch table (smoke)

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
// Scenario 1 — next LABEL
// ===========================================================================

#[test]
fn next_undefined_label_produces_drop_label_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a for-loop body with `next OUTER` where OUTER is not defined
    let source = "for my $i (1..10) { next OUTER; }";

    let ctrl_start = source.find("next OUTER").ok_or("marker not found")?;
    let ctrl_end = ctrl_start + "next OUTER".len();

    let diag = make_diag(
        ctrl_start,
        ctrl_end,
        "PL410",
        "`next OUTER` references a label that is not defined in this file",
    );

    // WHEN code actions are requested for the diagnostic range
    let actions = actions_for(source, &[diag]);

    // THEN there is exactly one action offering to drop the label
    let action = find_action(&actions, |t| t.contains("next"))
        .ok_or_else(|| format!("no drop-label action for 'next' in: {:?}", actions))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred, "the drop-label action should be preferred");

    // AND applying the edit replaces 'next OUTER' with 'next'
    let result = edited(source, action);
    assert_eq!(result, "for my $i (1..10) { next; }");

    Ok(())
}

// ===========================================================================
// Scenario 2 — last LABEL
// ===========================================================================

#[test]
fn last_undefined_label_produces_drop_label_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a while-loop body with `last MISSING` where MISSING is not defined
    let source = "while (1) { last MISSING; }";

    let ctrl_start = source.find("last MISSING").ok_or("marker not found")?;
    let ctrl_end = ctrl_start + "last MISSING".len();

    let diag = make_diag(
        ctrl_start,
        ctrl_end,
        "PL410",
        "`last MISSING` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN there is an action offering to drop the label
    let action = find_action(&actions, |t| t.contains("last"))
        .ok_or_else(|| format!("no drop-label action for 'last' in: {:?}", actions))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred);

    // AND the edit replaces 'last MISSING' with 'last'
    let result = edited(source, action);
    assert_eq!(result, "while (1) { last; }");

    Ok(())
}

// ===========================================================================
// Scenario 3 — redo LABEL
// ===========================================================================

#[test]
fn redo_undefined_label_produces_drop_label_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a for-loop body with `redo NOWHERE` where NOWHERE is not defined
    let source = "for my $i (1..5) { redo NOWHERE; }";

    let ctrl_start = source.find("redo NOWHERE").ok_or("marker not found")?;
    let ctrl_end = ctrl_start + "redo NOWHERE".len();

    let diag = make_diag(
        ctrl_start,
        ctrl_end,
        "PL410",
        "`redo NOWHERE` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN there is an action offering to drop the label
    let action = find_action(&actions, |t| t.contains("redo"))
        .ok_or_else(|| format!("no drop-label action for 'redo' in: {:?}", actions))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred);

    // AND the edit replaces 'redo NOWHERE' with 'redo'
    let result = edited(source, action);
    assert_eq!(result, "for my $i (1..5) { redo; }");

    Ok(())
}

// ===========================================================================
// Scenario 4 — Title names the operator
// ===========================================================================

#[test]
fn action_title_names_the_operator() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a diagnostic for `next GHOST`
    let source = "for my $x (1..3) { next GHOST; }";
    let ctrl_start = source.find("next GHOST").ok_or("marker not found")?;
    let ctrl_end = ctrl_start + "next GHOST".len();

    let diag = make_diag(
        ctrl_start,
        ctrl_end,
        "PL410",
        "`next GHOST` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN the action title mentions both the fix operation and the operator
    let action = find_action(&actions, |t| t.contains("next"))
        .ok_or_else(|| format!("no action for 'next GHOST' in: {:?}", actions))?;

    assert_eq!(action.title, "Drop label: use bare 'next'");

    Ok(())
}

// ===========================================================================
// Scenario 5 — Invalid-range guard
// ===========================================================================

#[test]
fn out_of_bounds_range_returns_no_actions() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a diagnostic whose range extends beyond the end of the source
    let source = "for my $i (1..5) { next OUTER; }";
    let out_of_bounds_end = source.len() + 5;

    let diag = make_diag(
        source.len(),
        out_of_bounds_end,
        "PL410",
        "`next OUTER` references a label that is not defined in this file",
    );

    // WHEN code actions are requested for the invalid range
    let actions = actions_for(source, &[diag]);

    // THEN no drop-label action is produced
    let has_pl410_action = actions.iter().any(|a| a.title.contains("Drop label"));
    assert!(
        !has_pl410_action,
        "expected no PL410 action for out-of-bounds range, got: {:?}",
        actions
    );

    Ok(())
}

// ===========================================================================
// Scenario 6 — Dispatch smoke test
// ===========================================================================

#[test]
fn pl410_code_reaches_handler_via_dispatch_table() -> Result<(), Box<dyn std::error::Error>> {
    // Smoke test: a PL410 diagnostic given to the provider produces at least
    // one code action, confirming the dispatch table is wired correctly.
    let source = "while (1) { next PHANTOM; }";

    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CodeActionsProvider::new(source.to_string());

    let ctrl_start = source.find("next PHANTOM").ok_or("marker not found")?;
    let ctrl_end = ctrl_start + "next PHANTOM".len();

    let diags = vec![make_diag(
        ctrl_start,
        ctrl_end,
        "PL410",
        "`next PHANTOM` references a label that is not defined in this file",
    )];

    let actions = provider.get_code_actions(&ast, (0, source.len()), &diags);

    assert!(
        actions.iter().any(|a| a.title.contains("Drop label")),
        "PL410 dispatch route did not produce a drop-label action; actions: {:?}",
        actions
    );

    Ok(())
}
