//! BDD tests for PL410 quick-fix: drop undefined loop-control label.
//!
//!   PL410 - `next`/`last`/`redo LABEL` where LABEL is not defined in this file
//!
//! Fix: drop the label so the statement targets the innermost enclosing loop.
//!
//! Each scenario follows:
//!   GIVEN  source containing a loop-control statement with an undefined label
//!   WHEN   a PL410 diagnostic is presented and code actions are requested
//!   THEN   a single preferred action is returned that strips the label

use perl_lsp_rs_core::providers::code_actions::{CodeAction, CodeActionKind, CodeActionsProvider};
use perl_lsp_rs_core::providers::diagnostics::{Diagnostic, DiagnosticSeverity};
use perl_parser::Parser;
use perl_tdd_support::must;

// ---------------------------------------------------------------------------
// Helpers (same pattern as quick_fix_new_codes_bdd.rs)
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
// PL410 — next LABEL
// ===========================================================================

#[test]
fn next_undefined_label_produces_drop_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a for-loop body that uses `next FOO` where FOO is not defined
    let source = "for my $i (1..10) { next FOO; }";
    let range_start = source.find("next FOO").ok_or("marker not found")?;
    let range_end = range_start + "next FOO".len();

    let diag = make_diag(
        range_start,
        range_end,
        "PL410",
        "`next FOO` references a label that is not defined in this file",
    );

    // WHEN code actions are requested for the diagnostic range
    let actions = actions_for(source, &[diag]);

    // THEN there is a preferred action that drops the label
    let action = find_action(&actions, |t| t.contains("next"))
        .ok_or_else(|| format!("no drop-label action in: {actions:?}"))?;

    assert_eq!(action.title, "Drop undefined label: write `next` to target innermost loop");
    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred, "drop-label action should be preferred");

    Ok(())
}

#[test]
fn next_undefined_label_edit_strips_label_from_source() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN source where `next FOO` appears between other statements
    let source = "for my $i (1..10) { next FOO; }";
    let range_start = source.find("next FOO").ok_or("marker not found")?;
    let range_end = range_start + "next FOO".len();

    let diag = make_diag(
        range_start,
        range_end,
        "PL410",
        "`next FOO` references a label that is not defined in this file",
    );
    let actions = actions_for(source, &[diag]);

    let action = find_action(&actions, |t| t.contains("next"))
        .ok_or_else(|| format!("no drop-label action in: {actions:?}"))?;
    let result = edited(source, action);

    // THEN `next FOO` becomes `next` — the label is gone, semicolon is untouched
    assert_eq!(result, "for my $i (1..10) { next; }");

    Ok(())
}

// ===========================================================================
// PL410 — last LABEL
// ===========================================================================

#[test]
fn last_undefined_label_produces_drop_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a while-loop with `last MISSING` where MISSING is not defined
    let source = "while (1) { last MISSING; }";
    let range_start = source.find("last MISSING").ok_or("marker not found")?;
    let range_end = range_start + "last MISSING".len();

    let diag = make_diag(
        range_start,
        range_end,
        "PL410",
        "`last MISSING` references a label that is not defined in this file",
    );

    let actions = actions_for(source, &[diag]);

    let action = find_action(&actions, |t| t.contains("last"))
        .ok_or_else(|| format!("no drop-label action in: {actions:?}"))?;

    assert_eq!(action.title, "Drop undefined label: write `last` to target innermost loop");
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
fn redo_undefined_label_produces_drop_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a for-loop with `redo NOWHERE` where NOWHERE is not defined
    let source = "for my $i (1..5) { redo NOWHERE; }";
    let range_start = source.find("redo NOWHERE").ok_or("marker not found")?;
    let range_end = range_start + "redo NOWHERE".len();

    let diag = make_diag(
        range_start,
        range_end,
        "PL410",
        "`redo NOWHERE` references a label that is not defined in this file",
    );

    let actions = actions_for(source, &[diag]);

    let action = find_action(&actions, |t| t.contains("redo"))
        .ok_or_else(|| format!("no drop-label action in: {actions:?}"))?;

    assert_eq!(action.title, "Drop undefined label: write `redo` to target innermost loop");
    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred);

    let result = edited(source, action);
    assert_eq!(result, "for my $i (1..5) { redo; }");

    Ok(())
}

// ===========================================================================
// Guard: invalid diagnostic range produces no actions
// ===========================================================================

#[test]
fn invalid_range_produces_no_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a PL410 diagnostic with an out-of-bounds byte range
    let source = "for my $i (1..10) { next FOO; }";
    let beyond = source.len() + 10;

    let diag = make_diag(
        beyond,
        beyond + 8,
        "PL410",
        "`next FOO` references a label that is not defined in this file",
    );
    let actions = actions_for(source, &[diag]);

    // THEN no code action is produced (safe guard)
    assert!(
        !actions.iter().any(|a| a.title.contains("innermost loop")),
        "expected no PL410 action for out-of-bounds range, got: {actions:?}"
    );

    Ok(())
}

// ===========================================================================
// Dispatch smoke test: all three ops reach their handler
// ===========================================================================

#[test]
fn all_three_loop_ops_reach_pl410_handler() -> Result<(), Box<dyn std::error::Error>> {
    // Smoke test: next, last, and redo each produce at least one action when given
    // a correctly-formed PL410 diagnostic, confirming the dispatch route is wired up.
    let source = "for my $i (1..5) { next FOO; last BAR; redo BAZ; }";

    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CodeActionsProvider::new(source.to_string());

    let next_start = source.find("next FOO").ok_or("next not found")?;
    let last_start = source.find("last BAR").ok_or("last not found")?;
    let redo_start = source.find("redo BAZ").ok_or("redo not found")?;

    let diags = vec![
        make_diag(
            next_start,
            next_start + "next FOO".len(),
            "PL410",
            "`next FOO` references a label that is not defined in this file",
        ),
        make_diag(
            last_start,
            last_start + "last BAR".len(),
            "PL410",
            "`last BAR` references a label that is not defined in this file",
        ),
        make_diag(
            redo_start,
            redo_start + "redo BAZ".len(),
            "PL410",
            "`redo BAZ` references a label that is not defined in this file",
        ),
    ];

    let actions = provider.get_code_actions(&ast, (0, source.len()), &diags);

    let has_next = actions.iter().any(|a| a.title.contains("`next`"));
    let has_last = actions.iter().any(|a| a.title.contains("`last`"));
    let has_redo = actions.iter().any(|a| a.title.contains("`redo`"));

    assert!(has_next, "PL410 route not producing action for `next`; actions: {actions:?}");
    assert!(has_last, "PL410 route not producing action for `last`; actions: {actions:?}");
    assert!(has_redo, "PL410 route not producing action for `redo`; actions: {actions:?}");

    Ok(())
}
