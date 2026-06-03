//! BDD tests for PL410 quick-fix: remove undefined loop-control label.
//!
//! Each scenario follows the pattern:
//!   GIVEN  source containing a `next`/`last`/`redo LABEL` where LABEL is undefined
//!   WHEN   a PL410 diagnostic is passed to the code-actions provider
//!   THEN   exactly one preferred action is returned that strips the label

use perl_lsp_rs_core::providers::code_actions::{CodeAction, CodeActionKind, CodeActionsProvider};
use perl_lsp_rs_core::providers::diagnostics::{Diagnostic, DiagnosticSeverity};
use perl_parser::Parser;
use perl_tdd_support::must;

// ---------------------------------------------------------------------------
// Helpers (mirror the pattern from quick_fix_new_codes_bdd.rs)
// ---------------------------------------------------------------------------

fn make_pl410(start: usize, end: usize, op: &str, label: &str) -> Diagnostic {
    Diagnostic {
        range: (start, end),
        severity: DiagnosticSeverity::Warning,
        code: Some("PL410".to_string()),
        message: format!("`{op} {label}` references a label that is not defined in this file"),
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

/// Apply all edits from an action in reverse-offset order and return the result.
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
// PL410 — next LABEL with undefined label
// ===========================================================================

#[test]
fn next_undefined_label_produces_remove_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a loop that uses `next OUTER` where OUTER is not defined
    let source = "for my $i (1..10) { next OUTER; }";

    let stmt = "next OUTER";
    let start = source.find(stmt).ok_or("marker not found")?;
    let end = start + stmt.len();
    let diag = make_pl410(start, end, "next", "OUTER");

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN there is a preferred action to remove the label
    let action = find_action(&actions, |t| t.contains("next"))
        .ok_or_else(|| format!("no PL410 action in: {actions:?}"))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred, "PL410 fix should be preferred");

    Ok(())
}

#[test]
fn next_edit_drops_label_and_leaves_semicolon() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN the diagnostic range covers `next OUTER` (not the semicolon)
    let source = "for my $i (1..10) { next OUTER; }";

    let stmt = "next OUTER";
    let start = source.find(stmt).ok_or("marker not found")?;
    let end = start + stmt.len();
    let diag = make_pl410(start, end, "next", "OUTER");

    let actions = actions_for(source, &[diag]);
    let action = find_action(&actions, |t| t.contains("next"))
        .ok_or_else(|| format!("no PL410 action in: {actions:?}"))?;

    let result = edited(source, action);

    // THEN the label is removed; the loop keyword and semicolon are preserved
    assert_eq!(result, "for my $i (1..10) { next; }");

    Ok(())
}

// ===========================================================================
// PL410 — last LABEL with undefined label
// ===========================================================================

#[test]
fn last_undefined_label_produces_remove_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a loop that uses `last MISSING` where MISSING is not defined
    let source = "while (1) { last MISSING; }";

    let stmt = "last MISSING";
    let start = source.find(stmt).ok_or("marker not found")?;
    let end = start + stmt.len();
    let diag = make_pl410(start, end, "last", "MISSING");

    let actions = actions_for(source, &[diag]);

    let action = find_action(&actions, |t| t.contains("last"))
        .ok_or_else(|| format!("no PL410 action in: {actions:?}"))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred);

    let result = edited(source, action);
    assert_eq!(result, "while (1) { last; }");

    Ok(())
}

// ===========================================================================
// PL410 — redo LABEL with undefined label
// ===========================================================================

#[test]
fn redo_undefined_label_produces_remove_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a loop that uses `redo NOWHERE` where NOWHERE is not defined
    let source = "for my $i (1..5) { redo NOWHERE; }";

    let stmt = "redo NOWHERE";
    let start = source.find(stmt).ok_or("marker not found")?;
    let end = start + stmt.len();
    let diag = make_pl410(start, end, "redo", "NOWHERE");

    let actions = actions_for(source, &[diag]);

    let action = find_action(&actions, |t| t.contains("redo"))
        .ok_or_else(|| format!("no PL410 action in: {actions:?}"))?;

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
fn action_title_names_the_operator() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a `last GHOST` diagnostic
    let source = "while (1) { last GHOST; }";

    let stmt = "last GHOST";
    let start = source.find(stmt).ok_or("marker not found")?;
    let end = start + stmt.len();
    let diag = make_pl410(start, end, "last", "GHOST");

    let actions = actions_for(source, &[diag]);
    let action = find_action(&actions, |t| t.contains("last"))
        .ok_or_else(|| format!("no PL410 action in: {actions:?}"))?;

    // THEN the title includes the operator name so the user knows what will change
    assert!(
        action.title.contains("last"),
        "title should mention the loop-control operator; got: {:?}",
        action.title
    );

    Ok(())
}

// ===========================================================================
// PL410 — invalid-range guard
// ===========================================================================

#[test]
fn invalid_range_produces_no_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a PL410 diagnostic whose byte range exceeds the source length
    let source = "for my $i (1..3) { next FOO; }";
    let out_of_bounds_start = source.len() + 1;
    let out_of_bounds_end = source.len() + 10;

    let diag = Diagnostic {
        range: (out_of_bounds_start, out_of_bounds_end),
        severity: DiagnosticSeverity::Warning,
        code: Some("PL410".to_string()),
        message: "`next FOO` references a label that is not defined in this file".to_string(),
        related_information: Vec::new(),
        tags: Vec::new(),
        suggestion: None,
    };

    let actions = actions_for(source, &[diag]);

    // THEN no action is produced — the guard rejects the bad range
    assert!(
        !actions.iter().any(|a| a.title.contains("innermost")),
        "expected no PL410 action for out-of-bounds range; got: {actions:?}"
    );

    Ok(())
}

// ===========================================================================
// Dispatch smoke test
// ===========================================================================

#[test]
fn pl410_dispatch_reaches_handler() -> Result<(), Box<dyn std::error::Error>> {
    // Confirm the PL410 code is wired in diagnostic_routes and produces at
    // least one action for a well-formed diagnostic.
    let source = "for my $x (1..3) { next PHANTOM; }";

    let stmt = "next PHANTOM";
    let start = source.find(stmt).ok_or("stmt not found")?;
    let end = start + stmt.len();

    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CodeActionsProvider::new(source.to_string());

    let diags = vec![make_pl410(start, end, "next", "PHANTOM")];
    let actions = provider.get_code_actions(&ast, (0, source.len()), &diags);

    assert!(
        actions.iter().any(|a| a.title.contains("next")),
        "PL410 route not producing action; actions: {actions:?}"
    );

    Ok(())
}
