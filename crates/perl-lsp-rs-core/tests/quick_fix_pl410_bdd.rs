//! BDD tests for the PL410 quick-fix: remove undefined loop-control label.
//!
//! `next LABEL`, `last LABEL`, and `redo LABEL` that reference a label not
//! defined in the current file cause a fatal runtime error. The fix drops the
//! label so the statement targets the innermost enclosing loop.
//!
//! Each scenario follows the pattern:
//!   GIVEN  source with a loop-control statement targeting an undefined label
//!   WHEN   a PL410 diagnostic is supplied and code actions are requested
//!   THEN   exactly one action is returned that removes the label

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

/// Apply every edit in the action and return the resulting source.
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
// PL410 — next with undefined label
// ===========================================================================

#[test]
fn pl410_next_undefined_label_produces_remove_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a for loop whose `next` targets a label that is never defined
    let source = "for my $i (1..5) { next OUTER; }";
    let stmt = "next OUTER";
    let start = source.find(stmt).ok_or("marker not found")?;
    let end = start + stmt.len();

    let diag = make_diag(
        start,
        end,
        "PL410",
        "`next OUTER` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN there is exactly one action, it removes the label, and it is preferred
    let action = find_action(&actions, |t| t.contains("next"))
        .ok_or_else(|| format!("no 'next' action in: {actions:?}"))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred, "the remove-label action should be preferred");

    // AND the edit strips the label, leaving bare `next`
    let result = edited(source, action);
    assert_eq!(result, "for my $i (1..5) { next; }");

    Ok(())
}

// ===========================================================================
// PL410 — last with undefined label
// ===========================================================================

#[test]
fn pl410_last_undefined_label_produces_remove_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a while loop whose `last` targets an undefined label
    let source = "while (1) { last MISSING; }";
    let stmt = "last MISSING";
    let start = source.find(stmt).ok_or("marker not found")?;
    let end = start + stmt.len();

    let diag = make_diag(
        start,
        end,
        "PL410",
        "`last MISSING` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN there is an action for `last` and the edit removes the label
    let action = find_action(&actions, |t| t.contains("last"))
        .ok_or_else(|| format!("no 'last' action in: {actions:?}"))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred);

    let result = edited(source, action);
    assert_eq!(result, "while (1) { last; }");

    Ok(())
}

// ===========================================================================
// PL410 — redo with undefined label
// ===========================================================================

#[test]
fn pl410_redo_undefined_label_produces_remove_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a for loop whose `redo` targets a label that does not exist
    let source = "for my $i (1..5) { redo NOWHERE; }";
    let stmt = "redo NOWHERE";
    let start = source.find(stmt).ok_or("marker not found")?;
    let end = start + stmt.len();

    let diag = make_diag(
        start,
        end,
        "PL410",
        "`redo NOWHERE` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN there is an action for `redo` and the edit removes the label
    let action = find_action(&actions, |t| t.contains("redo"))
        .ok_or_else(|| format!("no 'redo' action in: {actions:?}"))?;

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
fn pl410_action_title_names_the_operator() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a `next LABEL` diagnostic
    let source = "for my $x (1..3) { next GHOST; }";
    let stmt = "next GHOST";
    let start = source.find(stmt).ok_or("marker not found")?;
    let end = start + stmt.len();

    let diag = make_diag(
        start,
        end,
        "PL410",
        "`next GHOST` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN the title contains the operator name so the user knows which statement is being fixed
    let action = find_action(&actions, |t| t.contains("next"))
        .ok_or_else(|| format!("no 'next' action in: {actions:?}"))?;

    assert_eq!(action.title, "Remove undefined label from 'next'");

    Ok(())
}

// ===========================================================================
// PL410 — invalid-range guard
// ===========================================================================

#[test]
fn pl410_invalid_range_returns_no_actions() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a source with a multibyte character
    // WHEN the diagnostic range splits the multibyte sequence (non-char-boundary)
    let source = "for my $i (1..5) { next OUTER; }\nmy $s = \"\u{e9}\";\n";
    let char_start = source.find('\u{e9}').ok_or("marker not found")?;
    let diag = make_diag(char_start + 1, char_start + 2, "PL410", "invalid range");

    let actions = actions_for(source, &[diag]);

    // THEN no PL410 action is produced
    assert!(
        !actions.iter().any(|a| a.title.contains("undefined label")),
        "expected no action for non-char-boundary range, got: {actions:?}"
    );

    Ok(())
}

// ===========================================================================
// PL410 — dispatch-table smoke test
// ===========================================================================

#[test]
fn pl410_dispatch_table_route_is_wired() -> Result<(), Box<dyn std::error::Error>> {
    // Smoke test: confirm the route table entry for PL410 exists and produces
    // at least one action for a trivially valid diagnostic.
    let source = "while (1) { next PHANTOM; }";
    let stmt = "next PHANTOM";
    let start = source.find(stmt).ok_or("marker not found")?;
    let end = start + stmt.len();

    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CodeActionsProvider::new(source.to_string());

    let diags = vec![make_diag(
        start,
        end,
        "PL410",
        "`next PHANTOM` references a label that is not defined in this file",
    )];

    let actions = provider.get_code_actions(&ast, (0, source.len()), &diags);

    assert!(
        actions.iter().any(|a| a.title.contains("next")),
        "PL410 route not producing action; actions: {actions:?}"
    );

    Ok(())
}
