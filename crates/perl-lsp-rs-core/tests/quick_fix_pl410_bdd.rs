//! BDD tests for the PL410 quick-fix handler (`fix_loop_control_undefined_label`).
//!
//! PL410 fires when a `next LABEL`, `last LABEL`, or `redo LABEL` statement
//! references a label that does not exist anywhere in the file. The only safe
//! automatic fix is to drop the label so the statement targets the innermost
//! enclosing loop.
//!
//! Scenarios
//! ---------
//!   1. `next LABEL`  — action is produced with correct title and kind
//!   2. `last LABEL`  — edit removes the label, leaving bare `last`
//!   3. `redo LABEL`  — action is produced
//!   4. Title naming  — title contains both op and label name
//!   5. Invalid-range guard — non-char-boundary range returns no actions
//!   6. Dispatch smoke — PL410 code reaches its handler end-to-end

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
// Scenario 1 — `next LABEL` produces a quick-fix action
// ===========================================================================

#[test]
fn next_undefined_label_produces_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a loop-control statement that targets an undefined label
    let source = "for my $i (1..10) { next OUTER; }\n";
    let stmt_start = source.find("next OUTER").ok_or("stmt not found")?;
    let stmt_end = stmt_start + "next OUTER".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`next OUTER` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN an action is produced that removes the label
    let action = find_action(&actions, |t| t.contains("OUTER"))
        .ok_or_else(|| format!("no OUTER action in: {:?}", actions))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred, "the label-removal action should be preferred");

    Ok(())
}

// ===========================================================================
// Scenario 2 — `last LABEL` edit removes the label, leaving bare `last`
// ===========================================================================

#[test]
fn last_undefined_label_edit_removes_label() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a `last MISSING` inside a while loop
    let source = "while (1) { last MISSING; }\n";
    let stmt_start = source.find("last MISSING").ok_or("stmt not found")?;
    let stmt_end = stmt_start + "last MISSING".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`last MISSING` references a label that is not defined in this file",
    );
    let actions = actions_for(source, &[diag]);

    let action = find_action(&actions, |t| t.contains("MISSING"))
        .ok_or_else(|| format!("no MISSING action in: {:?}", actions))?;
    let result = edited(source, action);

    // THEN the label is removed and `last;` targets the innermost loop
    assert_eq!(result, "while (1) { last; }\n");

    Ok(())
}

// ===========================================================================
// Scenario 3 — `redo LABEL` produces a quick-fix action
// ===========================================================================

#[test]
fn redo_undefined_label_produces_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a `redo NOWHERE` inside a for loop
    let source = "for my $j (1..5) { redo NOWHERE; }\n";
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

    // THEN an action is produced
    let action = find_action(&actions, |t| t.contains("NOWHERE"))
        .ok_or_else(|| format!("no NOWHERE action in: {:?}", actions))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred);

    Ok(())
}

// ===========================================================================
// Scenario 4 — Action title names both the operator and the label
// ===========================================================================

#[test]
fn action_title_names_op_and_label() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a `next GHOST` diagnostic
    let source = "for my $x (1..3) { next GHOST; }\n";
    let stmt_start = source.find("next GHOST").ok_or("stmt not found")?;
    let stmt_end = stmt_start + "next GHOST".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`next GHOST` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN the title follows the pattern "Remove undefined label 'X' from 'op'"
    let action = find_action(&actions, |t| t.contains("GHOST"))
        .ok_or_else(|| format!("no GHOST action in: {:?}", actions))?;

    assert_eq!(action.title, "Remove undefined label 'GHOST' from 'next'");

    Ok(())
}

// ===========================================================================
// Scenario 5 — Non-char-boundary range produces no action
// ===========================================================================

#[test]
fn non_char_boundary_range_is_ignored() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a range that falls inside a multi-byte UTF-8 character
    let source = "for my $i (1..10) { next OUTER; }\nmy $name = \"\u{e9}\";\n";
    let char_start = source.find('\u{e9}').ok_or("marker not found")?;
    // Offset by 1 to land inside the two-byte sequence — not a char boundary.
    let diag = make_diag(char_start + 1, char_start + 2, "PL410", "spurious");
    let actions = actions_for(source, &[diag]);

    // THEN no label-removal action is produced
    assert!(
        !actions.iter().any(|a| a.title.contains("Remove undefined label")),
        "expected no PL410 action for non-char-boundary range, got: {actions:?}"
    );

    Ok(())
}

// ===========================================================================
// Scenario 6 — Dispatch smoke: PL410 code is wired into the routing table
// ===========================================================================

#[test]
fn pl410_dispatch_reaches_handler() -> Result<(), Box<dyn std::error::Error>> {
    // Smoke test: confirm the routing table routes PL410 to the fix handler.
    let source = "while (1) { next PHANTOM; }\n";
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CodeActionsProvider::new(source.to_string());

    let stmt_start = source.find("next PHANTOM").ok_or("stmt not found")?;
    let stmt_end = stmt_start + "next PHANTOM".len();
    let diags = vec![make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`next PHANTOM` references a label that is not defined in this file",
    )];

    let actions = provider.get_code_actions(&ast, (0, source.len()), &diags);

    assert!(
        actions.iter().any(|a| a.title.contains("PHANTOM")),
        "PL410 route should produce an action; got: {:?}",
        actions
    );

    Ok(())
}
