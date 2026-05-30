//! BDD tests for the PL410 quick-fix: drop undefined loop-control label.
//!
//! Each scenario follows the pattern:
//!   GIVEN  source with a `next/last/redo LABEL` where the label is undefined
//!   WHEN   a PL410 diagnostic is provided and code actions are requested
//!   THEN   exactly the expected drop-label action is returned with correct edit

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

/// Apply the edits from an action and return the resulting source.
fn edited(source: &str, action: &CodeAction) -> String {
    let mut edits = action.edit.changes.clone();
    edits.sort_by(|a, b| b.location.start.cmp(&a.location.start));
    let mut out = source.to_string();
    for edit in edits {
        out.replace_range(edit.location.start..edit.location.end, &edit.new_text);
    }
    out
}

/// Find the first action whose title matches the predicate.
fn find_action<'a>(
    actions: &'a [CodeAction],
    pred: impl Fn(&str) -> bool,
) -> Option<&'a CodeAction> {
    actions.iter().find(|a| pred(&a.title))
}

// ===========================================================================
// PL410 - undefined loop-control label
// ===========================================================================

#[test]
fn next_undefined_label_produces_drop_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a while loop with `next MISSING` where MISSING is not defined
    let source = "while(1) { next MISSING; }";
    let stmt_start = source.find("next MISSING").ok_or("marker not found")?;
    let stmt_end = stmt_start + "next MISSING".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`next MISSING` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN there is a drop-label action mentioning "next" and "innermost"
    let action = find_action(&actions, |t| t.contains("next") && t.contains("innermost"))
        .ok_or_else(|| format!("no drop-label action for next in: {:?}", actions))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred, "the drop action should be preferred");

    Ok(())
}

#[test]
fn last_undefined_label_produces_drop_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a for loop with `last NOPE` where NOPE is not defined
    let source = "for my $i (1..10) { last NOPE; }";
    let stmt_start = source.find("last NOPE").ok_or("marker not found")?;
    let stmt_end = stmt_start + "last NOPE".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`last NOPE` references a label that is not defined in this file",
    );

    let actions = actions_for(source, &[diag]);

    // THEN there is a drop-label action mentioning "last"
    let action = find_action(&actions, |t| t.contains("last") && t.contains("innermost"))
        .ok_or_else(|| format!("no drop-label action for last in: {:?}", actions))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred);

    Ok(())
}

#[test]
fn redo_undefined_label_produces_drop_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a foreach loop with `redo TOP` where TOP is not defined
    let source = "foreach my $x (@a) { redo TOP; }";
    let stmt_start = source.find("redo TOP").ok_or("marker not found")?;
    let stmt_end = stmt_start + "redo TOP".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`redo TOP` references a label that is not defined in this file",
    );

    let actions = actions_for(source, &[diag]);

    // THEN there is a drop-label action mentioning "redo"
    let action = find_action(&actions, |t| t.contains("redo") && t.contains("innermost"))
        .ok_or_else(|| format!("no drop-label action for redo in: {:?}", actions))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred);

    Ok(())
}

#[test]
fn drop_label_edit_leaves_bare_operator() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a while loop with `next MISSING`
    let source = "while(1) { next MISSING; }";
    let stmt_start = source.find("next MISSING").ok_or("marker not found")?;
    let stmt_end = stmt_start + "next MISSING".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`next MISSING` references a label that is not defined in this file",
    );

    let actions = actions_for(source, &[diag]);
    let action = find_action(&actions, |t| t.contains("next") && t.contains("innermost"))
        .ok_or_else(|| format!("no drop-label action in: {:?}", actions))?;

    // WHEN the edit is applied
    let result = edited(source, action);

    // THEN `next MISSING` becomes bare `next` (the `;` was already in source outside the span)
    assert!(result.contains("next;"), "expected 'next MISSING' replaced with 'next', got: {result:?}");
    assert!(!result.contains("MISSING"), "label should be removed from the result: {result:?}");

    Ok(())
}

#[test]
fn action_title_names_the_operator() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a while loop with `next MISSING`
    let source = "while(1) { next MISSING; }";
    let stmt_start = source.find("next MISSING").ok_or("marker not found")?;
    let stmt_end = stmt_start + "next MISSING".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`next MISSING` references a label that is not defined in this file",
    );

    let actions = actions_for(source, &[diag]);
    let action = find_action(&actions, |t| t.contains("next") && t.contains("innermost"))
        .ok_or_else(|| format!("no drop-label action in: {:?}", actions))?;

    // THEN the title matches the expected format exactly
    assert_eq!(
        action.title,
        "Drop label \u{2014} write `next` to target the innermost enclosing loop"
    );

    Ok(())
}

#[test]
fn invalid_range_guard_suppresses_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a diagnostic with an out-of-bounds range
    let source = "while(1) { next MISSING; }";
    let diag = make_diag(
        9999,
        10000,
        "PL410",
        "`next MISSING` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN no PL410 drop-label action is produced (guard fires)
    let has_drop_action = actions.iter().any(|a| a.title.contains("innermost"));
    assert!(
        !has_drop_action,
        "expected no drop-label action for out-of-bounds range, got: {:?}",
        actions
    );

    Ok(())
}
