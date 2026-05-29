//! BDD tests for the PL410 (LoopControlUndefinedLabel) quick-fix handler:
//!   `fix_loop_control_undefined_label`
//!
//! Each scenario follows the pattern:
//!   GIVEN  source with a loop control statement targeting an undefined label
//!   WHEN   a PL410 diagnostic is provided and code actions are requested
//!   THEN   exactly the expected action(s) are returned with correct edits

use perl_lsp_rs_core::providers::code_actions::{CodeAction, CodeActionKind, CodeActionsProvider};
use perl_lsp_rs_core::providers::diagnostics::{Diagnostic, DiagnosticSeverity};
use perl_parser::Parser;
use perl_tdd_support::must;

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
// PL410 - Loop control statement targets undefined label
// ===========================================================================

#[test]
fn pl410_next_undefined_label_produces_drop_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a `next GHOST` statement where GHOST is not defined as a label
    let source = "for my $x (1..3) { next GHOST; }";
    let stmt_start = source.find("next GHOST").ok_or("marker not found")?;
    let stmt_end = stmt_start + "next GHOST".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`next GHOST` references a label that is not defined in this file",
    );

    // WHEN code actions are requested for the diagnostic
    let actions = actions_for(source, &[diag]);

    // THEN there is an action offering to drop the label
    let action = find_action(&actions, |t| t.contains("next"))
        .ok_or_else(|| format!("no drop-label action for 'next' in: {:?}", actions))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred, "drop-label action should be preferred");

    Ok(())
}

#[test]
fn pl410_next_edit_removes_label_from_statement() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a `next GHOST` where the label is undefined
    let source = "for my $x (1..3) { next GHOST; }";
    let stmt_start = source.find("next GHOST").ok_or("marker not found")?;
    let stmt_end = stmt_start + "next GHOST".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`next GHOST` references a label that is not defined in this file",
    );
    let actions = actions_for(source, &[diag]);

    let action = find_action(&actions, |t| t.contains("next"))
        .ok_or_else(|| format!("no next action in: {:?}", actions))?;
    let result = edited(source, action);

    // THEN the label is removed; `next` becomes bare
    assert_eq!(result, "for my $x (1..3) { next; }");

    Ok(())
}

#[test]
fn pl410_last_undefined_label_produces_drop_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a `last MISSING` statement
    let source = "for my $i (1..10) { last MISSING; }";
    let stmt_start = source.find("last MISSING").ok_or("marker not found")?;
    let stmt_end = stmt_start + "last MISSING".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`last MISSING` references a label that is not defined in this file",
    );
    let actions = actions_for(source, &[diag]);

    let action = find_action(&actions, |t| t.contains("last"))
        .ok_or_else(|| format!("no last action in: {:?}", actions))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred);

    let result = edited(source, action);
    assert_eq!(result, "for my $i (1..10) { last; }");

    Ok(())
}

#[test]
fn pl410_redo_undefined_label_produces_drop_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a `redo NOWHERE` statement
    let source = "for my $i (1..5) { redo NOWHERE; }";
    let stmt_start = source.find("redo NOWHERE").ok_or("marker not found")?;
    let stmt_end = stmt_start + "redo NOWHERE".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`redo NOWHERE` references a label that is not defined in this file",
    );
    let actions = actions_for(source, &[diag]);

    let action = find_action(&actions, |t| t.contains("redo"))
        .ok_or_else(|| format!("no redo action in: {:?}", actions))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred);

    let result = edited(source, action);
    assert_eq!(result, "for my $i (1..5) { redo; }");

    Ok(())
}

#[test]
fn pl410_action_title_names_the_operator() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a PL410 diagnostic for `next PHANTOM`
    let source = "while (1) { next PHANTOM; }";
    let stmt_start = source.find("next PHANTOM").ok_or("marker not found")?;
    let stmt_end = stmt_start + "next PHANTOM".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`next PHANTOM` references a label that is not defined in this file",
    );
    let actions = actions_for(source, &[diag]);

    // THEN the action title includes the operator name
    let action = find_action(&actions, |t| t.contains("next"))
        .ok_or_else(|| format!("no action naming 'next' in: {:?}", actions))?;

    assert!(
        action.title.contains("next"),
        "title should name the operator 'next', got: {:?}",
        action.title
    );

    Ok(())
}

#[test]
fn pl410_invalid_range_guard_returns_no_actions() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a PL410 diagnostic with an out-of-bounds range
    let source = "for my $x (1..3) { next GHOST; }";
    let diag = make_diag(
        source.len() + 1,
        source.len() + 10,
        "PL410",
        "`next GHOST` references a label that is not defined in this file",
    );

    let actions = actions_for(source, &[diag]);

    // THEN no PL410 drop-label action is returned (range guard fires)
    assert!(
        !actions.iter().any(|a| a.title.contains("Remove undefined label")),
        "expected no drop-label action for out-of-bounds range, got: {:?}",
        actions
    );

    Ok(())
}

// ===========================================================================
// Cross-cutting: dispatch-table smoke test
// ===========================================================================

#[test]
fn pl410_dispatch_smoke_test_all_three_ops_reach_handler() -> Result<(), Box<dyn std::error::Error>>
{
    // Smoke test: each of the three operators produces at least one action,
    // confirming the PL410 dispatch arm is wired up correctly.

    let source = "for my $x (1..3) { next GHOST; last MISSING; redo NOWHERE; }";
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CodeActionsProvider::new(source.to_string());

    let next_start = source.find("next GHOST").ok_or("next GHOST not found")?;
    let last_start = source.find("last MISSING").ok_or("last MISSING not found")?;
    let redo_start = source.find("redo NOWHERE").ok_or("redo NOWHERE not found")?;

    let diags = vec![
        make_diag(
            next_start,
            next_start + "next GHOST".len(),
            "PL410",
            "`next GHOST` references a label that is not defined in this file",
        ),
        make_diag(
            last_start,
            last_start + "last MISSING".len(),
            "PL410",
            "`last MISSING` references a label that is not defined in this file",
        ),
        make_diag(
            redo_start,
            redo_start + "redo NOWHERE".len(),
            "PL410",
            "`redo NOWHERE` references a label that is not defined in this file",
        ),
    ];

    let actions = provider.get_code_actions(&ast, (0, source.len()), &diags);

    let has_next = actions.iter().any(|a| a.title.contains("next"));
    let has_last = actions.iter().any(|a| a.title.contains("last"));
    let has_redo = actions.iter().any(|a| a.title.contains("redo"));

    assert!(has_next, "PL410 next route not producing action; actions: {:?}", actions);
    assert!(has_last, "PL410 last route not producing action; actions: {:?}", actions);
    assert!(has_redo, "PL410 redo route not producing action; actions: {:?}", actions);

    Ok(())
}
