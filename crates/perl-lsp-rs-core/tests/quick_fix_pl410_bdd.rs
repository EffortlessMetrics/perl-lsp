//! BDD tests for the PL410 quick-fix handler:
//!   `fix_loop_control_undefined_label` — drop the undefined label from a
//!   `next`/`last`/`redo LABEL` statement so it targets the innermost loop.
//!
//! Each scenario follows the pattern:
//!   GIVEN  source with a loop-control statement whose label is not defined
//!   WHEN   a PL410 diagnostic is present and code actions are requested
//!   THEN   exactly the expected action is returned with the correct edit

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

/// Apply all edits from an action in reverse offset order and return the resulting source.
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
// PL410 – next LABEL with undefined label
// ===========================================================================

#[test]
fn next_undefined_label_produces_drop_label_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a file where `next OUTER` references a label that does not exist
    let source = "for my $i (1..10) { next OUTER; }\n";
    let stmt_start = source.find("next OUTER").ok_or("marker not found")?;
    let stmt_end = stmt_start + "next OUTER".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`next OUTER` references a label that is not defined in this file",
    );

    // WHEN code actions are requested for the PL410 diagnostic
    let actions = actions_for(source, &[diag]);

    // THEN exactly one action offering to drop the label is returned
    let action = find_action(&actions, |t| t.contains("OUTER"))
        .ok_or_else(|| format!("no drop-label action in: {:?}", actions))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred, "drop-label action should be marked preferred");

    // AND applying the fix converts `next OUTER` to bare `next`
    let result = edited(source, action);
    assert!(
        result.contains("next;"),
        "expected bare 'next;' after label drop, got: {result:?}"
    );

    Ok(())
}

// ===========================================================================
// PL410 – last LABEL with undefined label
// ===========================================================================

#[test]
fn last_undefined_label_produces_drop_label_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a file where `last MISSING` references an undefined label
    let source = "while (1) { last MISSING; }\n";
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

    // THEN a drop-label action is offered
    let action = find_action(&actions, |t| t.contains("MISSING"))
        .ok_or_else(|| format!("no drop-label action in: {:?}", actions))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred);

    // AND the edit converts `last MISSING` to bare `last`
    let result = edited(source, action);
    assert!(
        result.contains("last;"),
        "expected bare 'last;' after label drop, got: {result:?}"
    );

    Ok(())
}

// ===========================================================================
// PL410 – redo LABEL with undefined label
// ===========================================================================

#[test]
fn redo_undefined_label_produces_drop_label_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a file where `redo NOWHERE` references an undefined label
    let source = "for my $i (1..5) { redo NOWHERE; }\n";
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

    // THEN a drop-label action is offered
    let action = find_action(&actions, |t| t.contains("NOWHERE"))
        .ok_or_else(|| format!("no drop-label action in: {:?}", actions))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred);

    // AND the edit converts `redo NOWHERE` to bare `redo`
    let result = edited(source, action);
    assert!(
        result.contains("redo;"),
        "expected bare 'redo;' after label drop, got: {result:?}"
    );

    Ok(())
}

// ===========================================================================
// PL410 – title naming convention
// ===========================================================================

#[test]
fn drop_label_action_title_names_the_operator_and_label() -> Result<(), Box<dyn std::error::Error>>
{
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

    // THEN the action title contains both the label name and the operator keyword
    let action = find_action(&actions, |t| t.contains("PHANTOM"))
        .ok_or_else(|| format!("no action for PHANTOM in: {:?}", actions))?;

    assert!(
        action.title.contains("last"),
        "title should name the operator 'last', got: {:?}",
        action.title
    );
    assert!(
        action.title.contains("PHANTOM"),
        "title should name the label 'PHANTOM', got: {:?}",
        action.title
    );

    Ok(())
}

// ===========================================================================
// PL410 – invalid-range guard
// ===========================================================================

#[test]
fn drop_label_action_invalid_range_is_guarded() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a PL410 diagnostic whose byte range falls inside a multi-byte character
    let source = "for my $i (1..10) { next OUTER; }\nmy $s = \"\u{e9}\";\n";
    let char_start = source.find('\u{e9}').ok_or("UTF-8 marker not found")?;

    // Range straddling a non-char-boundary (char_start + 1 is mid-codepoint)
    let diag = make_diag(
        char_start + 1,
        char_start + 2,
        "PL410",
        "`next OUTER` references a label that is not defined in this file",
    );

    let actions = actions_for(source, &[diag]);

    // THEN no drop-label action is produced — the guard rejects the invalid range
    assert!(
        !actions.iter().any(|a| a.title.contains("OUTER")),
        "expected no PL410 action for non-char-boundary range, got: {:?}",
        actions
    );

    Ok(())
}

// ===========================================================================
// Cross-cutting: dispatch-table smoke test
// ===========================================================================

#[test]
fn dispatch_smoke_test_pl410_reaches_handler() -> Result<(), Box<dyn std::error::Error>> {
    // Smoke test: a PL410 diagnostic produces at least one action, confirming
    // the dispatch table in diagnostic_routes.rs is wired up correctly.
    let source = "for my $i (1..10) { next GHOST; }\n";

    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CodeActionsProvider::new(source.to_string());

    let stmt_start = source.find("next GHOST").ok_or("marker not found")?;
    let stmt_end = stmt_start + "next GHOST".len();

    let diags = vec![make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`next GHOST` references a label that is not defined in this file",
    )];

    let actions = provider.get_code_actions(&ast, (0, source.len()), &diags);

    assert!(
        actions.iter().any(|a| a.title.contains("GHOST")),
        "PL410 route not wired — no action for 'GHOST' in: {:?}",
        actions
    );

    Ok(())
}
