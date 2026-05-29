//! BDD tests for the PL410 quick-fix: drop undefined loop-control label.
//!
//! Each scenario follows the pattern:
//!   GIVEN  source with a `next`/`last`/`redo LABEL` where LABEL is not defined
//!   WHEN   a PL410 diagnostic is produced and code actions are requested
//!   THEN   exactly one preferred action drops the label from the statement

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
fn next_undefined_label_produces_drop_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a loop body that uses `next OUTER` where OUTER is not defined
    let source = "for my $i (1..10) { next OUTER; }\n";
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

    // THEN there is an action to drop the label
    let action = find_action(&actions, |t| t.contains("OUTER") && t.contains("next"))
        .ok_or_else(|| format!("no drop-label action in: {:?}", actions))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred, "drop-label action must be preferred");

    Ok(())
}

// ===========================================================================
// PL410 — last LABEL
// ===========================================================================

#[test]
fn last_undefined_label_produces_drop_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a loop body that uses `last MISSING` where MISSING is not defined
    let source = "while (1) { last MISSING; }\n";
    let stmt_start = source.find("last MISSING").ok_or("marker not found")?;
    let stmt_end = stmt_start + "last MISSING".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`last MISSING` references a label that is not defined in this file",
    );

    let actions = actions_for(source, &[diag]);

    let action = find_action(&actions, |t| t.contains("MISSING") && t.contains("last"))
        .ok_or_else(|| format!("no drop-label action in: {:?}", actions))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred);

    Ok(())
}

// ===========================================================================
// PL410 — redo LABEL
// ===========================================================================

#[test]
fn redo_undefined_label_produces_drop_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a loop body that uses `redo NOWHERE` where NOWHERE is not defined
    let source = "for my $i (1..5) { redo NOWHERE; }\n";
    let stmt_start = source.find("redo NOWHERE").ok_or("marker not found")?;
    let stmt_end = stmt_start + "redo NOWHERE".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`redo NOWHERE` references a label that is not defined in this file",
    );

    let actions = actions_for(source, &[diag]);

    let action = find_action(&actions, |t| t.contains("NOWHERE") && t.contains("redo"))
        .ok_or_else(|| format!("no drop-label action in: {:?}", actions))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred);

    Ok(())
}

// ===========================================================================
// PL410 — action title naming
// ===========================================================================

#[test]
fn pl410_drop_action_title_names_label_and_operator() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a `last PHANTOM` diagnostic
    let source = "while (1) { last PHANTOM; }\n";
    let stmt_start = source.find("last PHANTOM").ok_or("marker not found")?;
    let stmt_end = stmt_start + "last PHANTOM".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`last PHANTOM` references a label that is not defined in this file",
    );

    let actions = actions_for(source, &[diag]);

    // THEN the action title mentions both the label and the operator
    let action = find_action(&actions, |t| t.contains("PHANTOM") && t.contains("last"))
        .ok_or_else(|| format!("no drop-label action in: {:?}", actions))?;

    assert!(action.title.contains("PHANTOM"), "title should name the label, got: {}", action.title);
    assert!(action.title.contains("last"), "title should name the operator, got: {}", action.title);

    Ok(())
}

// ===========================================================================
// PL410 — edit correctness
// ===========================================================================

#[test]
fn pl410_drop_edit_removes_label_leaving_bare_operator() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN `next OUTER` inside a loop
    let source = "for my $i (1..3) { next OUTER; }\n";
    let stmt_start = source.find("next OUTER").ok_or("marker not found")?;
    let stmt_end = stmt_start + "next OUTER".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`next OUTER` references a label that is not defined in this file",
    );

    let actions = actions_for(source, &[diag]);
    let action = find_action(&actions, |t| t.contains("OUTER") && t.contains("next"))
        .ok_or_else(|| format!("no drop-label action in: {:?}", actions))?;

    // WHEN the edit is applied
    let result = edited(source, action);

    // THEN only the label is removed — the operator and surrounding code are intact
    assert!(
        result.contains("next;") || result.contains("next ") || result.contains("next\n"),
        "bare operator must remain after edit, got: {result:?}"
    );
    assert!(!result.contains("OUTER"), "label OUTER must be removed, got: {result:?}");

    Ok(())
}

// ===========================================================================
// PL410 — invalid-range guard
// ===========================================================================

#[test]
fn pl410_out_of_bounds_range_produces_no_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a PL410 diagnostic whose byte range is beyond the source length
    let source = "for my $i (1..3) { next GHOST; }\n";
    let diag = make_diag(
        source.len() + 1,
        source.len() + 10,
        "PL410",
        "`next GHOST` references a label that is not defined in this file",
    );

    let actions = actions_for(source, &[diag]);

    // THEN no drop-label action is produced
    assert!(
        !actions.iter().any(|a| a.title.contains("GHOST")),
        "expected no action for out-of-bounds range, got: {actions:?}"
    );

    Ok(())
}

// ===========================================================================
// Cross-cutting: dispatch-table smoke test
// ===========================================================================

#[test]
fn pl410_dispatch_smoke_all_three_operators_reach_handler() -> Result<(), Box<dyn std::error::Error>>
{
    // Smoke test: each operator form produces at least one drop-label action,
    // confirming the PL410 arm is wired correctly in the dispatch table.
    let source =
        "for my $i (1..3) { next ALPHA; }\nwhile (1) { last BETA; }\nfor (1..3) { redo GAMMA; }\n";

    let next_start = source.find("next ALPHA").ok_or("next ALPHA not found")?;
    let last_start = source.find("last BETA").ok_or("last BETA not found")?;
    let redo_start = source.find("redo GAMMA").ok_or("redo GAMMA not found")?;

    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CodeActionsProvider::new(source.to_string());

    let diags = vec![
        make_diag(
            next_start,
            next_start + "next ALPHA".len(),
            "PL410",
            "`next ALPHA` references a label that is not defined in this file",
        ),
        make_diag(
            last_start,
            last_start + "last BETA".len(),
            "PL410",
            "`last BETA` references a label that is not defined in this file",
        ),
        make_diag(
            redo_start,
            redo_start + "redo GAMMA".len(),
            "PL410",
            "`redo GAMMA` references a label that is not defined in this file",
        ),
    ];

    let actions = provider.get_code_actions(&ast, (0, source.len()), &diags);

    let has_next = actions.iter().any(|a| a.title.contains("ALPHA") && a.title.contains("next"));
    let has_last = actions.iter().any(|a| a.title.contains("BETA") && a.title.contains("last"));
    let has_redo = actions.iter().any(|a| a.title.contains("GAMMA") && a.title.contains("redo"));

    assert!(has_next, "PL410 next-arm not producing action; actions: {:?}", actions);
    assert!(has_last, "PL410 last-arm not producing action; actions: {:?}", actions);
    assert!(has_redo, "PL410 redo-arm not producing action; actions: {:?}", actions);

    Ok(())
}
