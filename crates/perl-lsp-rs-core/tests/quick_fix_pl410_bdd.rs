//! BDD tests for the PL410 quick-fix handler (`fix_loop_control_undefined_label`).
//!
//! PL410 fires when `next LABEL`, `last LABEL`, or `redo LABEL` reference a
//! label that is not defined anywhere in the file. The only mechanical fix is
//! to drop the label so the statement targets the innermost enclosing loop.
//!
//! Each scenario follows the pattern:
//!   GIVEN  source with a loop-control statement referencing an undefined label
//!   WHEN   a PL410 diagnostic is produced and code actions are requested
//!   THEN   exactly one preferred action is returned that strips the label

use perl_lsp_rs_core::providers::code_actions::{CodeAction, CodeActionKind, CodeActionsProvider};
use perl_lsp_rs_core::providers::diagnostics::{Diagnostic, DiagnosticSeverity};
use perl_parser::Parser;
use perl_tdd_support::must;

// ---------------------------------------------------------------------------
// Helpers
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

fn find_action<'a>(
    actions: &'a [CodeAction],
    pred: impl Fn(&str) -> bool,
) -> Option<&'a CodeAction> {
    actions.iter().find(|a| pred(&a.title))
}

fn apply_first_edit(source: &str, action: &CodeAction) -> String {
    let mut edits = action.edit.changes.clone();
    edits.sort_by(|a, b| b.location.start.cmp(&a.location.start));
    let mut out = source.to_string();
    for edit in edits {
        out.replace_range(edit.location.start..edit.location.end, &edit.new_text);
    }
    out
}

// ===========================================================================
// PL410 — next LABEL
// ===========================================================================

#[test]
fn pl410_next_undefined_label_action_title_names_op() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a loop with `next OUTER` where OUTER is not defined
    let source = "for my $i (1..10) { next OUTER; }\n";
    let start = source.find("next OUTER").ok_or("marker not found")?;
    let end = start + "next OUTER".len();
    let diag = make_pl410(start, end, "next", "OUTER");

    // WHEN code actions are requested for the PL410 diagnostic
    let actions = actions_for(source, &[diag]);

    // THEN exactly one action is returned and its title names the op
    let action = find_action(&actions, |t| t.contains("next"))
        .ok_or_else(|| format!("no next action in: {:?}", actions))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred, "PL410 fix should be the preferred action");
    assert!(action.title.contains("next"), "title should name the op: {}", action.title);

    Ok(())
}

#[test]
fn pl410_next_label_edit_produces_bare_next() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN source with `next OUTER` where OUTER is not defined
    let source = "for my $i (1..10) { next OUTER; }\n";
    let start = source.find("next OUTER").ok_or("marker not found")?;
    let end = start + "next OUTER".len();
    let diag = make_pl410(start, end, "next", "OUTER");

    // WHEN the fix is applied
    let actions = actions_for(source, &[diag]);
    let action = find_action(&actions, |t| t.contains("next"))
        .ok_or_else(|| format!("no next action in: {:?}", actions))?;
    let result = apply_first_edit(source, action);

    // THEN the label is stripped and the bare operator remains
    assert!(result.contains("next;"), "expected bare 'next;' in result: {result:?}");
    assert!(!result.contains("OUTER"), "label should be gone from result: {result:?}");

    Ok(())
}

// ===========================================================================
// PL410 — last LABEL
// ===========================================================================

#[test]
fn pl410_last_label_edit_produces_bare_last() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN source with `last INNER` where INNER is not defined
    let source = "while (1) { last INNER; }\n";
    let start = source.find("last INNER").ok_or("marker not found")?;
    let end = start + "last INNER".len();
    let diag = make_pl410(start, end, "last", "INNER");

    // WHEN the fix is applied
    let actions = actions_for(source, &[diag]);
    let action = find_action(&actions, |t| t.contains("last"))
        .ok_or_else(|| format!("no last action in: {:?}", actions))?;
    let result = apply_first_edit(source, action);

    // THEN the label is stripped and the bare operator remains
    assert!(result.contains("last;"), "expected bare 'last;' in result: {result:?}");
    assert!(!result.contains("INNER"), "label should be gone from result: {result:?}");

    Ok(())
}

// ===========================================================================
// PL410 — redo LABEL
// ===========================================================================

#[test]
fn pl410_redo_label_edit_produces_bare_redo() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN source with `redo CYCLE` where CYCLE is not defined
    let source = "for my $i (1..5) { redo CYCLE; }\n";
    let start = source.find("redo CYCLE").ok_or("marker not found")?;
    let end = start + "redo CYCLE".len();
    let diag = make_pl410(start, end, "redo", "CYCLE");

    // WHEN the fix is applied
    let actions = actions_for(source, &[diag]);
    let action = find_action(&actions, |t| t.contains("redo"))
        .ok_or_else(|| format!("no redo action in: {:?}", actions))?;
    let result = apply_first_edit(source, action);

    // THEN the label is stripped and the bare operator remains
    assert!(result.contains("redo;"), "expected bare 'redo;' in result: {result:?}");
    assert!(!result.contains("CYCLE"), "label should be gone from result: {result:?}");

    Ok(())
}

// ===========================================================================
// PL410 — invalid-range guard
// ===========================================================================

#[test]
fn pl410_invalid_range_returns_no_actions() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a PL410 diagnostic whose byte range extends beyond the source
    let source = "for my $i (1..10) { next OUTER; }\n";
    let out_of_bounds_start = source.len() + 1;
    let out_of_bounds_end = source.len() + 5;
    let diag = make_pl410(out_of_bounds_start, out_of_bounds_end, "next", "OUTER");

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN no actions are produced (range guard fires)
    let pl410_actions: Vec<_> =
        actions.iter().filter(|a| a.diagnostics.iter().any(|d| d == "PL410")).collect();
    assert!(
        pl410_actions.is_empty(),
        "expected no PL410 action for out-of-bounds range, got: {:?}",
        pl410_actions
    );

    Ok(())
}

// ===========================================================================
// Dispatch smoke test
// ===========================================================================

#[test]
fn pl410_dispatch_smoke_test_reaches_handler() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a source with an undefined-label loop control statement
    let source = "for my $x (1..3) { next GHOST; }\n";
    let start = source.find("next GHOST").ok_or("marker not found")?;
    let end = start + "next GHOST".len();

    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CodeActionsProvider::new(source.to_string());

    let diag = make_pl410(start, end, "next", "GHOST");

    // WHEN code actions are fetched through the full provider dispatch table
    let actions = provider.get_code_actions(&ast, (0, source.len()), &[diag]);

    // THEN at least one PL410 quick-fix action is returned
    let has_pl410_action = actions.iter().any(|a| a.diagnostics.iter().any(|d| d == "PL410"));
    assert!(has_pl410_action, "PL410 route should produce at least one action; got: {:?}", actions);

    Ok(())
}
