//! BDD tests for the PL410 quick-fix handler (`fix_loop_control_undefined_label`).
//!
//! Each scenario follows the pattern:
//!   GIVEN  source with a `next`/`last`/`redo LABEL` where the label is undefined
//!   WHEN   a PL410 diagnostic is produced and code actions are requested
//!   THEN   exactly one action is returned that removes the label and leaves the bare op

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

/// Apply all edits from an action (sorted descending by offset) and return the
/// resulting source so tests can verify the exact text produced.
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
fn pl410_next_undefined_label_produces_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN source where `next OUTER` references a label that is not defined
    let source = "for my $i (1..10) { next OUTER; }\n";
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

    // THEN there is an action that removes the label
    let action = find_action(&actions, |t| t.contains("next"))
        .ok_or_else(|| format!("no PL410 action for `next OUTER` in: {actions:?}"))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred, "PL410 fix should be preferred — it is the only sensible fix");

    // Applying the edit rewrites `next OUTER` → `next`
    let result = edited(source, action);
    assert!(
        result.contains("next;")
            || result.contains("next ")
            || result.contains("next\n")
            || result.ends_with("next"),
        "edit should replace `next OUTER` with `next`: {result:?}"
    );
    assert!(!result.contains("OUTER"), "label name should be removed: {result:?}");

    Ok(())
}

// ===========================================================================
// PL410 — last LABEL with undefined label
// ===========================================================================

#[test]
fn pl410_last_undefined_label_produces_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN source where `last MISSING` references a label that is not defined
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

    // THEN there is an action for `last`
    let action = find_action(&actions, |t| t.contains("last"))
        .ok_or_else(|| format!("no PL410 action for `last MISSING` in: {actions:?}"))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred);

    // The edit removes the label name
    let result = edited(source, action);
    assert!(!result.contains("MISSING"), "label name should be removed: {result:?}");

    Ok(())
}

// ===========================================================================
// PL410 — redo LABEL with undefined label
// ===========================================================================

#[test]
fn pl410_redo_undefined_label_produces_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN source where `redo NOWHERE` references a label that is not defined
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

    // THEN there is an action for `redo`
    let action = find_action(&actions, |t| t.contains("redo"))
        .ok_or_else(|| format!("no PL410 action for `redo NOWHERE` in: {actions:?}"))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred);

    let result = edited(source, action);
    assert!(!result.contains("NOWHERE"), "label name should be removed: {result:?}");

    Ok(())
}

// ===========================================================================
// Title naming convention
// ===========================================================================

#[test]
fn pl410_action_title_names_the_op() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a PL410 diagnostic on `next GHOST`
    let source = "for my $x (1..3) { next GHOST; }\n";
    let stmt_start = source.find("next GHOST").ok_or("marker not found")?;
    let stmt_end = stmt_start + "next GHOST".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`next GHOST` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN the title mentions both the fix concept and the operator name
    let action = find_action(&actions, |t| t.contains("next"))
        .ok_or_else(|| format!("no action found, got: {actions:?}"))?;

    assert!(action.title.contains("next"), "title should mention the op 'next': {}", action.title);
    assert!(
        action.title.to_lowercase().contains("label")
            || action.title.to_lowercase().contains("innermost"),
        "title should describe the fix intent: {}",
        action.title
    );

    Ok(())
}

// ===========================================================================
// Invalid range guard
// ===========================================================================

#[test]
fn pl410_invalid_range_returns_no_actions() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a PL410 diagnostic whose range is beyond the end of the source
    let source = "for my $i (1..10) { next OUTER; }\n";

    let diag = make_diag(
        source.len() + 1,
        source.len() + 10,
        "PL410",
        "`next OUTER` references a label that is not defined in this file",
    );

    // WHEN code actions are requested for an out-of-bounds range
    let actions = actions_for(source, &[diag]);

    // THEN no PL410 action is produced (range guard must reject it)
    let has_pl410_action =
        actions.iter().any(|a| a.title.contains("next") || a.title.contains("innermost"));
    assert!(!has_pl410_action, "expected no PL410 action for invalid range, got: {actions:?}");

    Ok(())
}

// ===========================================================================
// Dispatch smoke test
// ===========================================================================

#[test]
fn pl410_dispatch_smoke_reaches_handler() -> Result<(), Box<dyn std::error::Error>> {
    // Smoke test: PL410 reaches the fix handler when given a well-formed diagnostic,
    // confirming the dispatch table is wired correctly.
    let source = "for my $i (1..10) { next PHANTOM; }\n";
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CodeActionsProvider::new(source.to_string());

    let stmt_start = source.find("next PHANTOM").ok_or("marker not found")?;
    let stmt_end = stmt_start + "next PHANTOM".len();

    let diags = vec![make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`next PHANTOM` references a label that is not defined in this file",
    )];

    let actions = provider.get_code_actions(&ast, (0, source.len()), &diags);

    let has_pl410 = actions.iter().any(|a| a.title.contains("next"));
    assert!(has_pl410, "PL410 dispatch route not producing an action; actions: {actions:?}");

    Ok(())
}
