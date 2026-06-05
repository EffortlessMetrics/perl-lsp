//! BDD integration tests for the PL410 quick-fix handler.
//!
//! `fix_loop_control_undefined_label` drops the undefined label from a
//! `next LABEL`, `last LABEL`, or `redo LABEL` statement so the statement
//! targets the innermost enclosing loop instead.
//!
//! Each scenario follows the pattern:
//!   GIVEN  source with a loop-control statement targeting an undefined label
//!   WHEN   a PL410 diagnostic is present and code actions are requested
//!   THEN   exactly one preferred quick-fix action is returned with the
//!          correct edit

use perl_lsp_rs_core::providers::code_actions::{CodeAction, CodeActionKind, CodeActionsProvider};
use perl_lsp_rs_core::providers::diagnostics::{Diagnostic, DiagnosticSeverity};
use perl_parser::Parser;
use perl_tdd_support::must;

// ---------------------------------------------------------------------------
// Helpers (identical pattern to quick_fix_new_codes_bdd.rs)
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

/// Apply all edits from an action in reverse-start-position order and return the result.
fn edited(source: &str, action: &CodeAction) -> String {
    let mut edits = action.edit.changes.clone();
    edits.sort_by(|a, b| b.location.start.cmp(&a.location.start));
    let mut out = source.to_string();
    for edit in edits {
        out.replace_range(edit.location.start..edit.location.end, &edit.new_text);
    }
    out
}

/// Find the first action whose title satisfies the predicate.
fn find_action<'a>(
    actions: &'a [CodeAction],
    pred: impl Fn(&str) -> bool,
) -> Option<&'a CodeAction> {
    actions.iter().find(|a| pred(&a.title))
}

// ===========================================================================
// Scenario 1 — `next LABEL` drops to bare `next`
// ===========================================================================

#[test]
fn pl410_next_undefined_label_drops_to_bare_next() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a source where `next OUTER` targets a label that does not exist
    let source = "for my $i (1..10) { next OUTER; }";

    let stmt_start = source.find("next OUTER").ok_or("next OUTER not found")?;
    let stmt_end = stmt_start + "next OUTER".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`next OUTER` references a label that is not defined in this file",
    );

    // WHEN code actions are requested for the PL410 diagnostic
    let actions = actions_for(source, &[diag]);

    // THEN a preferred quick-fix action is returned
    let action = find_action(&actions, |t| t.contains("OUTER"))
        .ok_or_else(|| format!("no action containing 'OUTER' in: {actions:?}"))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred, "PL410 fix should be preferred");

    // AND applying the edit leaves `next` without the label
    let result = edited(source, action);
    assert!(result.contains("next;"), "bare next should follow after label removal, got: {result}");
    assert!(!result.contains("next OUTER"), "undefined label should be removed, got: {result}");

    Ok(())
}

// ===========================================================================
// Scenario 2 — `last LABEL` drops to bare `last`
// ===========================================================================

#[test]
fn pl410_last_undefined_label_drops_to_bare_last() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a source where `last MISSING` targets an undefined label
    let source = "while (1) { last MISSING; }";

    let stmt_start = source.find("last MISSING").ok_or("last MISSING not found")?;
    let stmt_end = stmt_start + "last MISSING".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`last MISSING` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN a quick-fix is produced
    let action = find_action(&actions, |t| t.contains("MISSING"))
        .ok_or_else(|| format!("no action containing 'MISSING' in: {actions:?}"))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred);

    // AND the edit strips the label, leaving `last`
    let result = edited(source, action);
    assert!(result.contains("last;"), "bare last should follow after label removal, got: {result}");
    assert!(!result.contains("last MISSING"), "label should be gone, got: {result}");

    Ok(())
}

// ===========================================================================
// Scenario 3 — `redo LABEL` drops to bare `redo`
// ===========================================================================

#[test]
fn pl410_redo_undefined_label_drops_to_bare_redo() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a source where `redo NOWHERE` targets an undefined label
    let source = "for my $i (1..5) { redo NOWHERE; }";

    let stmt_start = source.find("redo NOWHERE").ok_or("redo NOWHERE not found")?;
    let stmt_end = stmt_start + "redo NOWHERE".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`redo NOWHERE` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN a quick-fix is produced
    let action = find_action(&actions, |t| t.contains("NOWHERE"))
        .ok_or_else(|| format!("no action containing 'NOWHERE' in: {actions:?}"))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred);

    // AND the edit leaves `redo` without the label
    let result = edited(source, action);
    assert!(result.contains("redo;"), "bare redo should follow after label removal, got: {result}");
    assert!(!result.contains("redo NOWHERE"), "label should be gone, got: {result}");

    Ok(())
}

// ===========================================================================
// Scenario 4 — action title includes both the operator and the label name
// ===========================================================================

#[test]
fn pl410_action_title_names_op_and_label() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a PL410 diagnostic for `next PHANTOM`
    let source = "for my $x (1..3) { next PHANTOM; }";

    let stmt_start = source.find("next PHANTOM").ok_or("next PHANTOM not found")?;
    let stmt_end = stmt_start + "next PHANTOM".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`next PHANTOM` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN the action title contains both the operator ("next") and the label ("PHANTOM")
    let action = find_action(&actions, |t| t.contains("next") && t.contains("PHANTOM"))
        .ok_or_else(|| format!("no action with both 'next' and 'PHANTOM' in: {actions:?}"))?;

    assert!(
        action.title.contains("next"),
        "title should name the loop-control operator: {}",
        action.title
    );
    assert!(
        action.title.contains("PHANTOM"),
        "title should name the undefined label: {}",
        action.title
    );

    Ok(())
}

// ===========================================================================
// Scenario 5 — invalid (out-of-bounds) diagnostic range produces no actions
// ===========================================================================

#[test]
fn pl410_invalid_range_produces_no_actions() {
    // GIVEN a source and a diagnostic whose range extends past the end of the source
    let source = "for my $i (1..10) { next GHOST; }";

    let out_of_bounds_start = source.len() + 1;
    let out_of_bounds_end = source.len() + 10;

    let diag = make_diag(
        out_of_bounds_start,
        out_of_bounds_end,
        "PL410",
        "`next GHOST` references a label that is not defined in this file",
    );

    // WHEN code actions are requested with the bogus range
    let actions = actions_for(source, &[diag]);

    // THEN no PL410 quick-fix action is returned (invalid range is silently ignored)
    assert!(
        !actions.iter().any(|a| a.diagnostics.iter().any(|c| c == "PL410")),
        "invalid range should produce no PL410 actions, got: {actions:?}"
    );
}

// ===========================================================================
// Scenario 6 — dispatch smoke: full routing pipeline delivers a PL410 fix
// ===========================================================================

#[test]
fn pl410_dispatch_smoke_routes_via_code_actions_provider() -> Result<(), Box<dyn std::error::Error>>
{
    // GIVEN a PL410 diagnostic constructed the same way the diagnostic engine would
    let source = "while (1) { last PHANTOM; }";

    let stmt_start = source.find("last PHANTOM").ok_or("last PHANTOM not found")?;
    let stmt_end = stmt_start + "last PHANTOM".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`last PHANTOM` references a label that is not defined in this file",
    );

    // WHEN the full CodeActionsProvider pipeline processes it
    let actions = actions_for(source, &[diag]);

    // THEN at least one QuickFix action is returned for the PL410 code
    let pl410_actions: Vec<&CodeAction> =
        actions.iter().filter(|a| a.diagnostics.iter().any(|c| c == "PL410")).collect();

    assert!(
        !pl410_actions.is_empty(),
        "dispatch should route PL410 to fix_loop_control_undefined_label, got: {actions:?}"
    );
    assert_eq!(
        pl410_actions[0].kind,
        CodeActionKind::QuickFix,
        "routed action should be a QuickFix"
    );

    Ok(())
}
