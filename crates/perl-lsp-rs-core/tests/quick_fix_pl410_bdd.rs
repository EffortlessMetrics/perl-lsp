//! BDD tests for PL410 quick-fix — undefined loop-control label.
//!
//! Each scenario follows the pattern:
//!   GIVEN  source with a `next`/`last`/`redo LABEL` where the label is not defined
//!   WHEN   a PL410 diagnostic covers that statement and code actions are requested
//!   THEN   exactly one action is returned that drops the label and leaves a bare op

use perl_lsp_rs_core::providers::code_actions::{CodeAction, CodeActionKind, CodeActionsProvider};
use perl_lsp_rs_core::providers::diagnostics::{Diagnostic, DiagnosticSeverity};
use perl_parser::Parser;
use perl_tdd_support::must;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_diag(start: usize, end: usize, message: &str) -> Diagnostic {
    Diagnostic {
        range: (start, end),
        severity: DiagnosticSeverity::Warning,
        code: Some("PL410".to_string()),
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

/// Apply all edits from an action in reverse order and return the resulting source.
fn edited(source: &str, action: &CodeAction) -> String {
    let mut edits = action.edit.changes.clone();
    edits.sort_by(|a, b| b.location.start.cmp(&a.location.start));
    let mut out = source.to_string();
    for edit in edits {
        out.replace_range(edit.location.start..edit.location.end, &edit.new_text);
    }
    out
}

fn pl410_action(actions: &[CodeAction]) -> Option<&CodeAction> {
    actions.iter().find(|a| a.diagnostics.iter().any(|d| d == "PL410"))
}

// ===========================================================================
// next LABEL
// ===========================================================================

#[test]
fn next_undefined_label_produces_drop_label_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a `next OUTER` where OUTER is not defined
    let source = "for my $i (1..10) { next OUTER; }\n";
    let stmt_start = source.find("next OUTER").ok_or("marker not found")?;
    let stmt_end = stmt_start + "next OUTER".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "`next OUTER` references a label that is not defined in this file",
    );

    // WHEN code actions are requested for the diagnostic range
    let actions = actions_for(source, &[diag]);

    // THEN a single action drops the label
    let action =
        pl410_action(&actions).ok_or_else(|| format!("no PL410 action in: {actions:?}"))?;
    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred, "drop-label action should be preferred");

    // AND the resulting source replaces `next OUTER` with `next`
    let result = edited(source, action);
    assert_eq!(result, "for my $i (1..10) { next; }\n");

    Ok(())
}

// ===========================================================================
// last LABEL
// ===========================================================================

#[test]
fn last_undefined_label_produces_drop_label_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a `last MISSING` where MISSING is not defined
    let source = "while (1) { last MISSING; }\n";
    let stmt_start = source.find("last MISSING").ok_or("marker not found")?;
    let stmt_end = stmt_start + "last MISSING".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "`last MISSING` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN the action replaces `last MISSING` with bare `last`
    let action =
        pl410_action(&actions).ok_or_else(|| format!("no PL410 action in: {actions:?}"))?;
    assert!(action.is_preferred);
    let result = edited(source, action);
    assert_eq!(result, "while (1) { last; }\n");

    Ok(())
}

// ===========================================================================
// redo LABEL
// ===========================================================================

#[test]
fn redo_undefined_label_produces_drop_label_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a `redo NOWHERE` where NOWHERE is not defined
    let source = "for my $i (1..5) { redo NOWHERE; }\n";
    let stmt_start = source.find("redo NOWHERE").ok_or("marker not found")?;
    let stmt_end = stmt_start + "redo NOWHERE".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "`redo NOWHERE` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN the action replaces `redo NOWHERE` with bare `redo`
    let action =
        pl410_action(&actions).ok_or_else(|| format!("no PL410 action in: {actions:?}"))?;
    assert!(action.is_preferred);
    let result = edited(source, action);
    assert_eq!(result, "for my $i (1..5) { redo; }\n");

    Ok(())
}

// ===========================================================================
// Title naming
// ===========================================================================

#[test]
fn action_title_names_the_operator() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a `next GHOST` diagnostic
    let source = "for my $x (1..3) { next GHOST; }\n";
    let stmt_start = source.find("next GHOST").ok_or("marker not found")?;
    let stmt_end = stmt_start + "next GHOST".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "`next GHOST` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN the action title includes the operator name
    let action =
        pl410_action(&actions).ok_or_else(|| format!("no PL410 action in: {actions:?}"))?;
    assert!(
        action.title.contains("next"),
        "title should name the operator 'next', got: {}",
        action.title
    );

    Ok(())
}

// ===========================================================================
// Invalid-range guard
// ===========================================================================

#[test]
fn out_of_bounds_range_returns_no_action() {
    // GIVEN a diagnostic range that exceeds the source length
    let source = "for my $i (1..10) { next OUTER; }\n";
    let beyond = source.len() + 10;

    let diag = make_diag(
        beyond,
        beyond + 5,
        "`next OUTER` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN no PL410 action is produced
    assert!(
        pl410_action(&actions).is_none(),
        "out-of-bounds range must not produce an action, got: {actions:?}"
    );
}

// ===========================================================================
// Dispatch smoke test
// ===========================================================================

#[test]
fn dispatch_routes_pl410_through_provider() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN any PL410 diagnostic on a valid range
    let source = "while (1) { last PHANTOM; }\n";
    let stmt_start = source.find("last PHANTOM").ok_or("marker not found")?;
    let stmt_end = stmt_start + "last PHANTOM".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "`last PHANTOM` references a label that is not defined in this file",
    );

    // WHEN the code-actions provider handles the diagnostic
    let actions = actions_for(source, &[diag]);

    // THEN the dispatcher routes to the PL410 handler and returns an action
    assert!(
        pl410_action(&actions).is_some(),
        "dispatcher must route PL410 to fix_loop_control_undefined_label, got: {actions:?}"
    );

    Ok(())
}
