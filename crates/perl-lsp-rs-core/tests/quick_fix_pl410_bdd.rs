//! BDD tests for the PL410 quick-fix handler (`fix_loop_control_undefined_label`).
//!
//! Each scenario follows the pattern:
//!   GIVEN  source with a `next`/`last`/`redo LABEL` that references an undefined label
//!   WHEN   a PL410 diagnostic is produced and code actions are requested
//!   THEN   exactly one action is returned that drops the label

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
// PL410 — next LABEL
// ===========================================================================

#[test]
fn pl410_next_undefined_label_produces_drop_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a loop-control statement `next OUTER` where OUTER is not defined
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

    // THEN there is exactly one action and it drops the undefined label
    let action = find_action(&actions, |t| t.contains("next OUTER"))
        .ok_or_else(|| format!("no drop-label action in: {actions:?}"))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred, "the only sensible fix must be preferred");

    let result = edited(source, action);
    assert_eq!(result, "for my $i (1..10) { next; }");

    Ok(())
}

// ===========================================================================
// PL410 — last LABEL
// ===========================================================================

#[test]
fn pl410_last_undefined_label_produces_drop_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a loop-control statement `last MISSING` where MISSING is not defined
    let source = "while (1) { last MISSING; }";
    let stmt_start = source.find("last MISSING").ok_or("marker not found")?;
    let stmt_end = stmt_start + "last MISSING".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`last MISSING` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN the action strips the label and leaves bare `last`
    let action = find_action(&actions, |t| t.contains("last MISSING"))
        .ok_or_else(|| format!("no drop-label action in: {actions:?}"))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred);

    let result = edited(source, action);
    assert_eq!(result, "while (1) { last; }");

    Ok(())
}

// ===========================================================================
// PL410 — redo LABEL
// ===========================================================================

#[test]
fn pl410_redo_undefined_label_produces_drop_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a loop-control statement `redo NOWHERE` where NOWHERE is not defined
    let source = "for my $i (1..5) { redo NOWHERE; }";
    let stmt_start = source.find("redo NOWHERE").ok_or("marker not found")?;
    let stmt_end = stmt_start + "redo NOWHERE".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`redo NOWHERE` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN the action strips the label and leaves bare `redo`
    let action = find_action(&actions, |t| t.contains("redo NOWHERE"))
        .ok_or_else(|| format!("no drop-label action in: {actions:?}"))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred);

    let result = edited(source, action);
    assert_eq!(result, "for my $i (1..5) { redo; }");

    Ok(())
}

// ===========================================================================
// PL410 — title naming
// ===========================================================================

#[test]
fn pl410_action_title_names_both_original_and_replacement() -> Result<(), Box<dyn std::error::Error>>
{
    // GIVEN `next GHOST` triggers PL410
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

    // THEN the action title includes both the original expression and the fix
    let action = find_action(&actions, |t| t.contains("next GHOST"))
        .ok_or_else(|| format!("no action in: {actions:?}"))?;

    assert!(
        action.title.contains("next GHOST"),
        "title should name the original expression; got: {:?}",
        action.title
    );
    assert!(
        action.title.contains("next"),
        "title should name the replacement keyword; got: {:?}",
        action.title
    );
    assert_eq!(action.title, "Drop undefined label: change `next GHOST` to `next`");

    Ok(())
}

// ===========================================================================
// PL410 — invalid-range guard
// ===========================================================================

#[test]
fn pl410_out_of_bounds_range_produces_no_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a PL410 diagnostic whose range extends beyond the source length
    let source = "for my $i (1..5) { next X; }";
    let diag = make_diag(
        999,
        1000,
        "PL410",
        "`next X` references a label that is not defined in this file",
    );

    // WHEN code actions are requested with the bogus range
    let actions = actions_for(source, &[diag]);

    // THEN no drop-label action is produced
    assert!(
        !actions.iter().any(|a| a.title.contains("Drop undefined label")),
        "expected no action for out-of-bounds range, got: {actions:?}"
    );

    Ok(())
}

// ===========================================================================
// PL410 — dispatch smoke test
// ===========================================================================

#[test]
fn pl410_code_is_routed_to_fix_handler() -> Result<(), Box<dyn std::error::Error>> {
    // Smoke test: the "PL410" code reaches the fix handler and produces an
    // action, confirming the route is wired up in diagnostic_routes.rs.
    let source = "for my $i (1..10) { next OUTER; last MISSING; redo NOWHERE; }";
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CodeActionsProvider::new(source.to_string());

    let next_start = source.find("next OUTER").ok_or("next OUTER not found")?;
    let last_start = source.find("last MISSING").ok_or("last MISSING not found")?;
    let redo_start = source.find("redo NOWHERE").ok_or("redo NOWHERE not found")?;

    let diags = vec![
        make_diag(
            next_start,
            next_start + "next OUTER".len(),
            "PL410",
            "`next OUTER` references a label that is not defined in this file",
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

    let has_next_fix = actions.iter().any(|a| a.title.contains("next OUTER"));
    let has_last_fix = actions.iter().any(|a| a.title.contains("last MISSING"));
    let has_redo_fix = actions.iter().any(|a| a.title.contains("redo NOWHERE"));

    assert!(
        has_next_fix,
        "PL410 route did not produce action for 'next OUTER'; actions: {actions:?}"
    );
    assert!(
        has_last_fix,
        "PL410 route did not produce action for 'last MISSING'; actions: {actions:?}"
    );
    assert!(
        has_redo_fix,
        "PL410 route did not produce action for 'redo NOWHERE'; actions: {actions:?}"
    );

    Ok(())
}
