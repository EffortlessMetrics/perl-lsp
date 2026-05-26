//! BDD tests for the PL410 quick-fix handler (`fix_loop_control_undefined_label`).
//!
//! Each scenario follows the pattern:
//!   GIVEN  source containing a loop-control statement with an undefined label
//!   WHEN   a PL410 diagnostic is supplied and code actions are requested
//!   THEN   exactly the expected action(s) are returned with correct edits

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

fn find_action<'a>(
    actions: &'a [CodeAction],
    pred: impl Fn(&str) -> bool,
) -> Option<&'a CodeAction> {
    actions.iter().find(|a| pred(&a.title))
}

// ===========================================================================
// PL410 — `next LABEL` with undefined label
// ===========================================================================

#[test]
fn pl410_next_undefined_label_produces_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a `next MISSING` statement where MISSING is not a defined loop label
    let source = "while (1) { next MISSING; }\n";
    let stmt_start = source.find("next MISSING").ok_or("marker not found")?;
    let stmt_end = stmt_start + "next MISSING".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "Label 'MISSING' is not defined in enclosing scope",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN there is a preferred quick-fix offering to drop the label
    let action = find_action(&actions, |t| t.contains("next"))
        .ok_or_else(|| format!("no PL410 action in: {actions:?}"))?;

    assert_eq!(action.title, "Remove undefined label (use 'next' without label)");
    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred, "the PL410 fix should be preferred");

    Ok(())
}

// ===========================================================================
// PL410 — `last LABEL` with undefined label
// ===========================================================================

#[test]
fn pl410_last_undefined_label_edit_strips_label() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a `last MISSING` statement
    let source = "for my $i (1..10) { last MISSING; }\n";
    let stmt_start = source.find("last MISSING").ok_or("marker not found")?;
    let stmt_end = stmt_start + "last MISSING".len();

    let diag = make_diag(stmt_start, stmt_end, "PL410", "Label 'MISSING' is not defined");

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);
    let action = find_action(&actions, |t| t.contains("last"))
        .ok_or_else(|| format!("no PL410 action in: {actions:?}"))?;

    // THEN applying the edit replaces `last MISSING` with just `last`
    let result = edited(source, action);
    assert_eq!(result, "for my $i (1..10) { last; }\n");

    Ok(())
}

// ===========================================================================
// PL410 — `redo LABEL` with undefined label
// ===========================================================================

#[test]
fn pl410_redo_undefined_label_edit_strips_label() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a `redo OUTER` statement where OUTER is not a surrounding label
    let source = "while (1) { redo OUTER; }\n";
    let stmt_start = source.find("redo OUTER").ok_or("marker not found")?;
    let stmt_end = stmt_start + "redo OUTER".len();

    let diag = make_diag(stmt_start, stmt_end, "PL410", "Label 'OUTER' is not defined");

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);
    let action = find_action(&actions, |t| t.contains("redo"))
        .ok_or_else(|| format!("no PL410 action in: {actions:?}"))?;

    // THEN applying the edit replaces `redo OUTER` with just `redo`
    let result = edited(source, action);
    assert_eq!(result, "while (1) { redo; }\n");

    Ok(())
}

// ===========================================================================
// PL410 — action title names the operator
// ===========================================================================

#[test]
fn pl410_action_title_names_the_operator() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a `last GONE` diagnostic
    let source = "while (1) { last GONE; }\n";
    let stmt_start = source.find("last GONE").ok_or("marker not found")?;
    let stmt_end = stmt_start + "last GONE".len();

    let diag = make_diag(stmt_start, stmt_end, "PL410", "Label 'GONE' is not defined");
    let actions = actions_for(source, &[diag]);

    let action = find_action(&actions, |t| t.contains("last"))
        .ok_or_else(|| format!("no PL410 action in: {actions:?}"))?;

    // THEN the title clearly names the operator so the user knows what they're accepting
    assert_eq!(action.title, "Remove undefined label (use 'last' without label)");

    Ok(())
}

// ===========================================================================
// PL410 — invalid (out-of-bounds) range produces no action
// ===========================================================================

#[test]
fn pl410_invalid_range_guard_returns_no_actions() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a PL410 diagnostic whose byte range is beyond the end of the source
    let source = "while (1) { next GONE; }\n";
    let out_of_bounds_start = source.len() + 1;
    let out_of_bounds_end = source.len() + 10;

    let diag =
        make_diag(out_of_bounds_start, out_of_bounds_end, "PL410", "Label 'GONE' is not defined");
    let actions = actions_for(source, &[diag]);

    // THEN no PL410-specific action is produced — the guard protects against OOB access
    // (unrelated source-level actions may still be returned by other providers)
    assert!(
        !actions.iter().any(|a| a.title.contains("undefined label")),
        "expected no PL410 action for out-of-bounds range, got: {actions:?}"
    );

    Ok(())
}

// ===========================================================================
// Cross-cutting: dispatch smoke test
// ===========================================================================

#[test]
fn pl410_dispatch_smoke_test_reaches_handler() -> Result<(), Box<dyn std::error::Error>> {
    // Smoke test: PL410 reaches its handler in the dispatch table.
    let source = "while (1) { next PHANTOM; }\n";
    let stmt_start = source.find("next PHANTOM").ok_or("marker not found")?;
    let stmt_end = stmt_start + "next PHANTOM".len();

    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CodeActionsProvider::new(source.to_string());

    let diags = vec![make_diag(stmt_start, stmt_end, "PL410", "Label 'PHANTOM' is not defined")];

    let actions = provider.get_code_actions(&ast, (0, source.len()), &diags);

    let has_pl410 = actions.iter().any(|a| a.title.contains("next"));
    assert!(has_pl410, "PL410 route not producing action; actions: {actions:?}");

    Ok(())
}
