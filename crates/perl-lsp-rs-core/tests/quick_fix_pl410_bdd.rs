//! BDD tests for PL410 quick-fix handler: `fix_loop_control_undefined_label`
//!
//! Each scenario follows:
//!   GIVEN  source with a `next`/`last`/`redo LABEL` whose label is not defined
//!   WHEN   a PL410 diagnostic is passed to the code-actions provider
//!   THEN   exactly one action is returned that drops the label and is `is_preferred`

use perl_lsp_rs_core::providers::code_actions::{CodeAction, CodeActionKind, CodeActionsProvider};
use perl_lsp_rs_core::providers::diagnostics::{Diagnostic, DiagnosticSeverity};
use perl_parser::Parser;
use perl_tdd_support::must;

// ---------------------------------------------------------------------------
// Helpers (mirrors quick_fix_new_codes_bdd.rs)
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
// Scenario 1: `next LABEL` — label dropped, innermost loop targeted
// ===========================================================================

#[test]
fn pl410_next_undefined_label_produces_drop_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a `next OUTER` inside a loop where OUTER is not defined
    let source = "for my $i (1..10) { next OUTER; }\n";
    let stmt_start = source.find("next OUTER").ok_or("stmt not found")?;
    let stmt_end = stmt_start + "next OUTER".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`next OUTER` references a label that is not defined in this file",
    );

    // WHEN code actions are requested for the diagnostic range
    let actions = actions_for(source, &[diag]);

    // THEN exactly one action is returned; it drops the label and is preferred
    let action = find_action(&actions, |t| t.contains("Drop") && t.contains("OUTER"))
        .ok_or_else(|| format!("no drop-label action in: {actions:?}"))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred, "drop-label action must be is_preferred");

    // AND the edit replaces `next OUTER` with bare `next`
    let result = edited(source, action);
    assert!(
        result.contains("next;")
            || result.contains("next ")
            || result == source.replace("next OUTER", "next"),
        "edit should reduce 'next OUTER' to 'next', got: {result:?}"
    );
    assert!(!result.contains("OUTER"), "label should be removed from source, got: {result:?}");

    Ok(())
}

// ===========================================================================
// Scenario 2: `last LABEL` — label dropped
// ===========================================================================

#[test]
fn pl410_last_undefined_label_produces_drop_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a `last MISSING` inside a while loop where MISSING is not defined
    let source = "while (1) { last MISSING; }\n";
    let stmt_start = source.find("last MISSING").ok_or("stmt not found")?;
    let stmt_end = stmt_start + "last MISSING".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`last MISSING` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN there is a drop-label action
    let action = find_action(&actions, |t| t.contains("Drop") && t.contains("MISSING"))
        .ok_or_else(|| format!("no drop-label action in: {actions:?}"))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred);

    // AND applying the edit removes the label
    let result = edited(source, action);
    assert!(!result.contains("MISSING"), "label should be removed, got: {result:?}");
    assert!(result.contains("last"), "bare 'last' should remain, got: {result:?}");

    Ok(())
}

// ===========================================================================
// Scenario 3: `redo LABEL` — label dropped
// ===========================================================================

#[test]
fn pl410_redo_undefined_label_produces_drop_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a `redo NOWHERE` inside a for loop where NOWHERE is not defined
    let source = "for my $i (1..5) { redo NOWHERE; }\n";
    let stmt_start = source.find("redo NOWHERE").ok_or("stmt not found")?;
    let stmt_end = stmt_start + "redo NOWHERE".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`redo NOWHERE` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN there is a drop-label action
    let action = find_action(&actions, |t| t.contains("Drop") && t.contains("NOWHERE"))
        .ok_or_else(|| format!("no drop-label action in: {actions:?}"))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred);

    // AND applying the edit removes the label
    let result = edited(source, action);
    assert!(!result.contains("NOWHERE"), "label should be removed, got: {result:?}");
    assert!(result.contains("redo"), "bare 'redo' should remain, got: {result:?}");

    Ok(())
}

// ===========================================================================
// Scenario 4: Action title contains the label name
// ===========================================================================

#[test]
fn pl410_action_title_includes_label_name() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a diagnostic for `next PHANTOM`
    let source = "for (1..3) { next PHANTOM; }\n";
    let stmt_start = source.find("next PHANTOM").ok_or("stmt not found")?;
    let stmt_end = stmt_start + "next PHANTOM".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`next PHANTOM` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN the action title names the offending label so the user can identify it
    let action = find_action(&actions, |t| t.contains("PHANTOM"))
        .ok_or_else(|| format!("no action naming label in: {actions:?}"))?;

    assert!(
        action.title.contains("PHANTOM"),
        "expected title to contain 'PHANTOM', got: {:?}",
        action.title
    );

    Ok(())
}

// ===========================================================================
// Scenario 5: Invalid-range guard — out-of-bounds range returns no actions
// ===========================================================================

#[test]
fn pl410_out_of_bounds_range_returns_no_actions() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a PL410 diagnostic whose range is beyond the source length
    let source = "for (1..3) { next FOO; }\n";
    let beyond = source.len() + 5;

    let diag = make_diag(beyond, beyond + 8, "PL410", "`next FOO` references an undefined label");

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN no drop-label action is produced (guard prevented out-of-bounds access)
    let has_drop = actions.iter().any(|a| a.title.contains("Drop"));
    assert!(!has_drop, "expected no PL410 action for out-of-bounds range, got: {actions:?}");

    Ok(())
}

// ===========================================================================
// Scenario 6: Dispatch smoke test — PL410 reaches its handler
// ===========================================================================

#[test]
fn pl410_dispatch_smoke_test_handler_is_wired() -> Result<(), Box<dyn std::error::Error>> {
    // Smoke test: a trivially-correct PL410 diagnostic reaches the handler
    // and produces at least one action, confirming the dispatch table is wired.
    let source = "while (1) { next GHOST; }\n";
    let stmt_start = source.find("next GHOST").ok_or("stmt not found")?;
    let stmt_end = stmt_start + "next GHOST".len();

    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CodeActionsProvider::new(source.to_string());

    let diags = vec![make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`next GHOST` references a label that is not defined in this file",
    )];

    let actions = provider.get_code_actions(&ast, (0, source.len()), &diags);

    let has_pl410_action =
        actions.iter().any(|a| a.title.contains("Drop") && a.title.contains("GHOST"));
    assert!(
        has_pl410_action,
        "PL410 route not producing action — handler may not be wired; actions: {actions:?}"
    );

    Ok(())
}
