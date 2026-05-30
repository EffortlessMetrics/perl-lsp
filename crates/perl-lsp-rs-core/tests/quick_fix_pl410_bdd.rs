//! BDD tests for the PL410 quick-fix handler (`fix_loop_control_undefined_label`).
//!
//! PL410 fires when `next LABEL`, `last LABEL`, or `redo LABEL` references a
//! label that is not defined anywhere in the current file. The quick fix drops
//! the label so the statement targets the innermost enclosing loop — the only
//! safe mechanical transformation available without full AST rewrite support.
//!
//! Each scenario follows the pattern:
//!   GIVEN  source with an undefined label reference
//!   WHEN   a PL410 diagnostic is produced and code actions are requested
//!   THEN   exactly one preferred action is returned with the correct edit

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
// PL410 — next LABEL with undefined label
// ===========================================================================

#[test]
fn next_pl410_produces_drop_label_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a loop body with `next MISSING` where MISSING is not defined
    let source = "while (1) { next MISSING; }\n";
    let next_start = source.find("next MISSING").ok_or("marker not found")?;
    let next_end = next_start + "next MISSING".len();

    let diag = make_diag(
        next_start,
        next_end,
        "PL410",
        "`next MISSING` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN there is a preferred action to drop the label
    let action = find_action(&actions, |t| t.contains("next"))
        .ok_or_else(|| format!("no 'next' action in: {actions:?}"))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred, "PL410 fix must be marked preferred");
    assert!(
        action.title.contains("MISSING"),
        "title should name the removed label, got: {}",
        action.title
    );

    Ok(())
}

// ===========================================================================
// PL410 — last LABEL edit correctness
// ===========================================================================

#[test]
fn last_pl410_edit_removes_label_from_source() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a loop body with `last NOPE` where NOPE is not defined
    let source = "for my $i (1..10) { last NOPE; }\n";
    let last_start = source.find("last NOPE").ok_or("marker not found")?;
    let last_end = last_start + "last NOPE".len();

    let diag = make_diag(
        last_start,
        last_end,
        "PL410",
        "`last NOPE` references a label that is not defined in this file",
    );

    let actions = actions_for(source, &[diag]);
    let action = find_action(&actions, |t| t.contains("last"))
        .ok_or_else(|| format!("no 'last' action in: {actions:?}"))?;

    // WHEN the edit is applied
    let result = edited(source, action);

    // THEN the label is stripped and the operator remains
    assert!(
        result.contains("last;"),
        "edited source should contain 'last;' (label removed), got: {result:?}"
    );
    assert!(
        !result.contains("NOPE"),
        "edited source must not contain the removed label 'NOPE', got: {result:?}"
    );

    Ok(())
}

// ===========================================================================
// PL410 — redo LABEL produces action
// ===========================================================================

#[test]
fn redo_pl410_produces_drop_label_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a loop body with `redo NOWHERE` where NOWHERE is not defined
    let source = "for my $i (1..5) { redo NOWHERE; }\n";
    let redo_start = source.find("redo NOWHERE").ok_or("marker not found")?;
    let redo_end = redo_start + "redo NOWHERE".len();

    let diag = make_diag(
        redo_start,
        redo_end,
        "PL410",
        "`redo NOWHERE` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN there is a preferred action to drop the label
    let action = find_action(&actions, |t| t.contains("redo"))
        .ok_or_else(|| format!("no 'redo' action in: {actions:?}"))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred);

    // AND the edit produces `redo;`
    let result = edited(source, action);
    assert!(result.contains("redo;"), "edited source should contain 'redo;', got: {result:?}");

    Ok(())
}

// ===========================================================================
// PL410 — action title names the operator
// ===========================================================================

#[test]
fn pl410_action_title_names_the_operator() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a `last DEADLABEL` statement
    let source = "while (1) { last DEADLABEL; }\n";
    let last_start = source.find("last DEADLABEL").ok_or("marker not found")?;
    let last_end = last_start + "last DEADLABEL".len();

    let diag = make_diag(
        last_start,
        last_end,
        "PL410",
        "`last DEADLABEL` references a label that is not defined in this file",
    );

    let actions = actions_for(source, &[diag]);
    let action = find_action(&actions, |t| t.contains("last"))
        .ok_or_else(|| format!("no 'last' action in: {actions:?}"))?;

    // THEN the title contains the operator so the user understands what changes
    assert!(
        action.title.contains("last"),
        "title must name the operator 'last', got: {}",
        action.title
    );
    assert!(
        action.title.contains("DEADLABEL"),
        "title must name the removed label 'DEADLABEL', got: {}",
        action.title
    );

    Ok(())
}

// ===========================================================================
// PL410 — invalid range guard
// ===========================================================================

#[test]
fn pl410_non_char_boundary_range_produces_no_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a PL410 diagnostic whose byte range splits a multi-byte character
    let source = "while (1) { next LABEL; }\nmy $s = \"\u{e9}\";\n";
    let char_start = source.find('\u{e9}').ok_or("marker not found")?;

    // The range intentionally splits the two-byte UTF-8 sequence for é
    let diag = make_diag(
        char_start + 1,
        char_start + 2,
        "PL410",
        "`next LABEL` references a label that is not defined in this file",
    );

    let actions = actions_for(source, &[diag]);

    // THEN no action is produced — the guard rejects the invalid range
    assert!(
        !actions.iter().any(|a| a.kind == CodeActionKind::QuickFix && a.title.contains("next")),
        "expected no PL410 action for non-char-boundary range, got: {actions:?}"
    );

    Ok(())
}

// ===========================================================================
// PL410 — dispatch smoke test
// ===========================================================================

#[test]
fn pl410_dispatch_smoke_test() -> Result<(), Box<dyn std::error::Error>> {
    // Smoke test: PL410 code reaches its handler and produces at least one
    // action when given a valid diagnostic, confirming the route is wired up.
    let source = "while (1) { next GHOST; }\n";
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CodeActionsProvider::new(source.to_string());

    let next_start = source.find("next GHOST").ok_or("marker not found")?;
    let next_end = next_start + "next GHOST".len();

    let diags = vec![make_diag(
        next_start,
        next_end,
        "PL410",
        "`next GHOST` references a label that is not defined in this file",
    )];

    let actions = provider.get_code_actions(&ast, (0, source.len()), &diags);

    assert!(
        actions.iter().any(|a| a.title.contains("next")),
        "PL410 route must produce at least one action; actions: {actions:?}"
    );

    Ok(())
}
