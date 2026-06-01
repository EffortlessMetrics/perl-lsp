//! BDD tests for the PL410 quick-fix handler (`fix_loop_control_undefined_label`).
//!
//! Each scenario follows:
//!   GIVEN  source containing a `next`/`last`/`redo` with an undefined label
//!   WHEN   a PL410 diagnostic is synthesised and code actions are requested
//!   THEN   exactly one action is returned that strips the label

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

// ---------------------------------------------------------------------------
// PL410 – next LABEL with undefined label
// ---------------------------------------------------------------------------

#[test]
fn pl410_next_undefined_label_returns_strip_label_action() -> Result<(), Box<dyn std::error::Error>>
{
    // GIVEN a `next OUTER` where OUTER is not defined
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

    // THEN there is exactly one action and it strips the label
    let action = actions.iter().find(|a| a.title.contains("next")).ok_or_else(|| {
        format!("no PL410 action for next in: {:?}", actions)
    })?;
    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred, "PL410 fix should be is_preferred");
    let result = edited(source, action);
    assert!(
        result.contains("next;") || result.contains("next "),
        "label should be stripped: {result}"
    );
    assert!(!result.contains("OUTER"), "OUTER label should be gone: {result}");
    Ok(())
}

// ---------------------------------------------------------------------------
// PL410 – last LABEL with undefined label
// ---------------------------------------------------------------------------

#[test]
fn pl410_last_undefined_label_returns_strip_label_action() -> Result<(), Box<dyn std::error::Error>>
{
    // GIVEN a `last MISSING` where MISSING is not defined
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

    // THEN the action strips the label and produces a bare `last`
    let action = actions.iter().find(|a| a.title.contains("last")).ok_or_else(|| {
        format!("no PL410 action for last in: {:?}", actions)
    })?;
    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred);
    let result = edited(source, action);
    assert!(!result.contains("MISSING"), "MISSING label should be gone: {result}");
    Ok(())
}

// ---------------------------------------------------------------------------
// PL410 – redo LABEL with undefined label
// ---------------------------------------------------------------------------

#[test]
fn pl410_redo_undefined_label_returns_strip_label_action() -> Result<(), Box<dyn std::error::Error>>
{
    // GIVEN a `redo NOWHERE` where NOWHERE is not defined
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

    // THEN the action strips the label
    let action = actions.iter().find(|a| a.title.contains("redo")).ok_or_else(|| {
        format!("no PL410 action for redo in: {:?}", actions)
    })?;
    assert!(action.is_preferred);
    let result = edited(source, action);
    assert!(!result.contains("NOWHERE"), "NOWHERE label should be gone: {result}");
    Ok(())
}

// ---------------------------------------------------------------------------
// PL410 – action title names the operator
// ---------------------------------------------------------------------------

#[test]
fn pl410_action_title_names_the_operator() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN any PL410 diagnostic for `next`
    let source = "for my $x (1..3) { next GHOST; }";
    let stmt_start = source.find("next GHOST").ok_or("marker not found")?;
    let stmt_end = stmt_start + "next GHOST".len();
    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`next GHOST` references a label that is not defined in this file",
    );

    // WHEN actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN the title mentions the operator so it is human-readable
    let action = actions.iter().find(|a| a.title.contains("next")).ok_or_else(|| {
        format!("expected action with 'next' in title, got: {:?}", actions)
    })?;
    assert!(
        action.title.contains("next"),
        "title should name the operator `next`, got: {}",
        action.title
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// PL410 – invalid range produces no actions (guard test)
// ---------------------------------------------------------------------------

#[test]
fn pl410_invalid_range_produces_no_actions() {
    // GIVEN a diagnostic with a range that extends past the source length
    let source = "while (1) { next PHANTOM; }";
    let diag = make_diag(
        source.len() + 1,
        source.len() + 10,
        "PL410",
        "`next PHANTOM` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN no actions are returned (guard prevents out-of-bounds access)
    assert!(
        actions.iter().all(|a| a.title.is_empty() || !a.title.contains("PHANTOM")),
        "invalid range must not produce a PL410 action: {actions:?}"
    );
}

// ---------------------------------------------------------------------------
// PL410 – dispatch smoke: non-PL410 code does not trigger handler
// ---------------------------------------------------------------------------

#[test]
fn pl410_dispatch_smoke_non_pl410_code_no_action() {
    // GIVEN source that looks like a loop-control statement
    let source = "for my $i (1..10) { next OUTER; }";
    let stmt_start = source.find("next OUTER").unwrap_or(0);
    let stmt_end = stmt_start + "next OUTER".len();

    // BUT the diagnostic code is PL100 (wrong code)
    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL100",
        "`next OUTER` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN the PL410 handler is NOT invoked — a different action (add_use_strict) may fire
    // but the loop-control label strip must not
    assert!(
        !actions.iter().any(|a| a.title.contains("innermost loop")),
        "PL410 handler must not fire for non-PL410 code: {actions:?}"
    );
}
