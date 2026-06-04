//! BDD tests for the PL410 quick-fix handler.
//!
//! PL410 fires when `next LABEL`, `last LABEL`, or `redo LABEL` references a
//! label that is not defined anywhere in the current file.  The fix drops the
//! label so the operator targets the innermost enclosing loop instead.
//!
//! Each scenario follows the pattern:
//!   GIVEN  source containing a loop-control statement with an undefined label
//!   WHEN   a PL410 diagnostic is produced and code actions are requested
//!   THEN   exactly one action is returned that replaces `op LABEL` with `op`

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
fn next_undefined_label_produces_drop_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a `next OUTER` where OUTER is not defined
    let source = "for my $i (1..10) { next OUTER; }";
    let ctrl_start = source.find("next OUTER").ok_or("marker not found")?;
    let ctrl_end = ctrl_start + "next OUTER".len();

    let diag = make_diag(
        ctrl_start,
        ctrl_end,
        "PL410",
        "`next OUTER` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN one action is returned offering to drop the label
    let action = find_action(&actions, |t| t.contains("next OUTER"))
        .ok_or_else(|| format!("no drop-label action in: {:?}", actions))?;

    assert_eq!(action.title, "Drop label from 'next OUTER' (targets innermost loop)");
    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred, "the drop-label action should be preferred");

    // AND the edit replaces `next OUTER` with `next`
    let result = edited(source, action);
    assert_eq!(result, "for my $i (1..10) { next; }");

    Ok(())
}

// ===========================================================================
// PL410 — last LABEL with undefined label
// ===========================================================================

#[test]
fn last_undefined_label_produces_drop_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a `last MISSING` where MISSING is not defined
    let source = "while (1) { last MISSING; }";
    let ctrl_start = source.find("last MISSING").ok_or("marker not found")?;
    let ctrl_end = ctrl_start + "last MISSING".len();

    let diag = make_diag(
        ctrl_start,
        ctrl_end,
        "PL410",
        "`last MISSING` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN the action exists and edits the source correctly
    let action = find_action(&actions, |t| t.contains("last MISSING"))
        .ok_or_else(|| format!("no drop-label action in: {:?}", actions))?;

    let result = edited(source, action);
    assert_eq!(result, "while (1) { last; }");

    Ok(())
}

// ===========================================================================
// PL410 — redo LABEL with undefined label
// ===========================================================================

#[test]
fn redo_undefined_label_produces_drop_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a `redo NOWHERE` where NOWHERE is not defined
    let source = "for my $i (1..5) { redo NOWHERE; }";
    let ctrl_start = source.find("redo NOWHERE").ok_or("marker not found")?;
    let ctrl_end = ctrl_start + "redo NOWHERE".len();

    let diag = make_diag(
        ctrl_start,
        ctrl_end,
        "PL410",
        "`redo NOWHERE` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN the action exists and edits the source correctly
    let action = find_action(&actions, |t| t.contains("redo NOWHERE"))
        .ok_or_else(|| format!("no drop-label action in: {:?}", actions))?;

    let result = edited(source, action);
    assert_eq!(result, "for my $i (1..5) { redo; }");

    Ok(())
}

// ===========================================================================
// PL410 — title naming
// ===========================================================================

#[test]
fn title_names_the_op_and_label_from_source() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a diagnostic whose range covers `last LOOP_TOP`
    let source = "LOOP_TOP: while (1) { for my $x (1..3) { last PHANTOM; } }";
    let ctrl_start = source.find("last PHANTOM").ok_or("marker not found")?;
    let ctrl_end = ctrl_start + "last PHANTOM".len();

    let diag = make_diag(
        ctrl_start,
        ctrl_end,
        "PL410",
        "`last PHANTOM` references a label that is not defined in this file",
    );

    let actions = actions_for(source, &[diag]);

    // THEN the title embeds the exact `op LABEL` text from the source
    let action = find_action(&actions, |t| t.contains("PHANTOM"))
        .ok_or_else(|| format!("no PHANTOM action in: {:?}", actions))?;

    assert_eq!(action.title, "Drop label from 'last PHANTOM' (targets innermost loop)");

    Ok(())
}

// ===========================================================================
// PL410 — invalid-range guard
// ===========================================================================

#[test]
fn invalid_range_returns_no_actions() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a PL410 diagnostic whose byte range falls outside a multi-byte char boundary
    let source = "for my $x (1..5) { next \u{e9}label; }";
    let char_pos = source.find('\u{e9}').ok_or("marker not found")?;

    // A range that starts inside the multi-byte character is invalid
    let diag = make_diag(
        char_pos + 1,
        char_pos + 3,
        "PL410",
        "`next LABEL` references a label that is not defined in this file",
    );

    let actions = actions_for(source, &[diag]);

    // THEN no drop-label action is produced
    assert!(
        !actions.iter().any(|a| a.title.contains("Drop label")),
        "expected no drop-label action for invalid byte range, got: {:?}",
        actions
    );

    Ok(())
}

// ===========================================================================
// Cross-cutting: dispatch smoke test
// ===========================================================================

#[test]
fn pl410_dispatch_reaches_handler() -> Result<(), Box<dyn std::error::Error>> {
    // Smoke test: PL410 code reaches the fix_loop_control_undefined_label handler
    // and produces at least one action for a well-formed diagnostic.
    let source = "for my $i (1..10) { next GHOST; }";
    let ctrl_start = source.find("next GHOST").ok_or("marker not found")?;
    let ctrl_end = ctrl_start + "next GHOST".len();

    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CodeActionsProvider::new(source.to_string());

    let diags = vec![make_diag(
        ctrl_start,
        ctrl_end,
        "PL410",
        "`next GHOST` references a label that is not defined in this file",
    )];

    let actions = provider.get_code_actions(&ast, (0, source.len()), &diags);

    let has_pl410 = actions.iter().any(|a| a.title.contains("next GHOST"));
    assert!(has_pl410, "PL410 route not reaching handler; actions: {:?}", actions);

    Ok(())
}
