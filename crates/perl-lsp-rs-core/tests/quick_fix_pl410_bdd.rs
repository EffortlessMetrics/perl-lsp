//! BDD tests for PL410 quick-fix handler:
//!   PL410 - Loop-control statement references undefined label (`fix_loop_control_undefined_label`)
//!
//! Each scenario follows the pattern:
//!   GIVEN  source with `next/last/redo LABEL` where LABEL is not defined
//!   WHEN   code actions are requested
//!   THEN   an action is returned that drops the label, targeting the innermost loop

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

/// Apply the first matching edit from an action and return the resulting source.
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
// PL410 - Undefined loop-control label
// ===========================================================================

#[test]
fn next_undefined_label_produces_drop_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a loop body with `next OUTER` where OUTER is not defined
    let source = "for my $i (1..10) { next OUTER; }";
    let next_start = source.find("next OUTER").ok_or("marker not found")?;
    let next_end = next_start + "next OUTER".len();

    let diag = make_diag(
        next_start,
        next_end,
        "PL410",
        "`next OUTER` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN there is an action offering to drop the label
    let action = find_action(&actions, |t| t.contains("OUTER"))
        .ok_or_else(|| format!("no drop-label action in: {:?}", actions))?;

    assert_eq!(
        action.title,
        "Drop undefined label 'OUTER' — `next;` will target the innermost loop"
    );
    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred, "drop-label should be the preferred fix");

    Ok(())
}

#[test]
fn last_undefined_label_produces_drop_action() -> Result<(), Box<dyn std::error::Error>> {
    let source = "while (1) { last MISSING; }";
    let last_start = source.find("last MISSING").ok_or("marker not found")?;
    let last_end = last_start + "last MISSING".len();

    let diag = make_diag(
        last_start,
        last_end,
        "PL410",
        "`last MISSING` references a label that is not defined in this file",
    );
    let actions = actions_for(source, &[diag]);

    let action = find_action(&actions, |t| t.contains("MISSING"))
        .ok_or_else(|| format!("no drop-label action in: {:?}", actions))?;

    assert_eq!(
        action.title,
        "Drop undefined label 'MISSING' — `last;` will target the innermost loop"
    );
    assert!(action.is_preferred);

    Ok(())
}

#[test]
fn redo_undefined_label_edit_replaces_with_bare_op() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN `redo NOWHERE` inside a loop
    let source = "for my $i (1..5) { redo NOWHERE; }";
    let redo_start = source.find("redo NOWHERE").ok_or("marker not found")?;
    let redo_end = redo_start + "redo NOWHERE".len();

    let diag = make_diag(
        redo_start,
        redo_end,
        "PL410",
        "`redo NOWHERE` references a label that is not defined in this file",
    );
    let actions = actions_for(source, &[diag]);

    let action = find_action(&actions, |t| t.contains("NOWHERE"))
        .ok_or_else(|| format!("no drop-label action in: {:?}", actions))?;
    let result = edited(source, action);

    // THEN `redo NOWHERE` is replaced with bare `redo`
    assert_eq!(result, "for my $i (1..5) { redo; }");

    Ok(())
}

#[test]
fn action_title_names_the_label_from_range_text() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN next with a long multi-word-style label name
    let source = "for my $x (1..3) { next MY_CUSTOM_LABEL; }";
    let next_start = source.find("next MY_CUSTOM_LABEL").ok_or("marker not found")?;
    let next_end = next_start + "next MY_CUSTOM_LABEL".len();

    let diag = make_diag(
        next_start,
        next_end,
        "PL410",
        "`next MY_CUSTOM_LABEL` references a label that is not defined in this file",
    );
    let actions = actions_for(source, &[diag]);

    let action = find_action(&actions, |t| t.contains("MY_CUSTOM_LABEL"))
        .ok_or_else(|| format!("no action with label name in: {:?}", actions))?;

    assert!(
        action.title.contains("MY_CUSTOM_LABEL"),
        "title should contain the label name, got: {}",
        action.title
    );

    Ok(())
}

#[test]
fn invalid_range_does_not_produce_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a PL410 diagnostic whose range points to something that is NOT a loop-control op
    let source = "my $x = 1;\n";
    let diag = make_diag(
        3,
        5,
        "PL410",
        "`next GHOST` references a label that is not defined in this file",
    );
    let actions = actions_for(source, &[diag]);

    assert!(
        !actions.iter().any(|a| a.title.contains("innermost")),
        "expected no drop-label action for misaligned range, got: {:?}",
        actions
    );

    Ok(())
}

#[test]
fn pl410_dispatch_smoke_test() -> Result<(), Box<dyn std::error::Error>> {
    // Smoke test: PL410 reaches the handler in the routing table.
    let source = "for my $i (1..10) { next OUTER; }";
    let next_start = source.find("next OUTER").ok_or("marker not found")?;
    let next_end = next_start + "next OUTER".len();

    let diag = make_diag(
        next_start,
        next_end,
        "PL410",
        "`next OUTER` references a label that is not defined in this file",
    );
    let actions = actions_for(source, &[diag]);

    assert!(
        actions.iter().any(|a| a.title.contains("OUTER")),
        "PL410 route not producing action; actions: {:?}",
        actions
    );

    Ok(())
}
