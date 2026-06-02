//! BDD tests for the PL410 quick-fix handler:
//!   PL410 — `next`/`last`/`redo LABEL` references an undefined label
//!           (`fix_loop_control_undefined_label`)
//!
//! Each scenario follows the pattern:
//!   GIVEN  source containing a loop-control statement with an undefined label
//!   WHEN   a PL410 diagnostic is present and code actions are requested
//!   THEN   the handler offers exactly one preferred action that drops the label

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

/// Apply the edits from an action and return the modified source.
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
// next LABEL — PL410
// ===========================================================================

#[test]
fn next_undefined_label_produces_remove_label_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a `next OUTER` where OUTER is not defined
    let source = "for my $i (1..10) { next OUTER; }\n";

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

    // THEN there is a preferred action offering to drop the label
    let action = find_action(&actions, |t| t.contains("next"))
        .ok_or_else(|| format!("no `next` action in: {actions:?}"))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred, "PL410 fix should be marked preferred");

    // AND the edit replaces `next OUTER` with bare `next`
    let result = edited(source, action);
    assert!(
        result.contains("next;") || result.contains("next "),
        "expected bare `next` after edit, got: {result:?}"
    );
    assert!(!result.contains("OUTER"), "label should be removed after edit, got: {result:?}");

    Ok(())
}

// ===========================================================================
// last LABEL — PL410
// ===========================================================================

#[test]
fn last_undefined_label_produces_remove_label_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a `last MISSING` where MISSING is not defined
    let source = "for my $i (1..10) { last MISSING; }\n";

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

    // THEN there is a preferred action offering to drop the label
    let action = find_action(&actions, |t| t.contains("last"))
        .ok_or_else(|| format!("no `last` action in: {actions:?}"))?;

    assert!(action.is_preferred, "PL410 fix should be marked preferred");
    assert_eq!(action.kind, CodeActionKind::QuickFix);

    let result = edited(source, action);
    assert!(!result.contains("MISSING"), "label should be removed after edit, got: {result:?}");

    Ok(())
}

// ===========================================================================
// redo LABEL — PL410
// ===========================================================================

#[test]
fn redo_undefined_label_produces_remove_label_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a `redo NOWHERE` where NOWHERE is not defined
    let source = "for my $i (1..5) { redo NOWHERE; }\n";

    let ctrl_start = source.find("redo NOWHERE").ok_or("marker not found")?;
    let ctrl_end = ctrl_start + "redo NOWHERE".len();

    let diag = make_diag(
        ctrl_start,
        ctrl_end,
        "PL410",
        "`redo NOWHERE` references a label that is not defined in this file",
    );

    let actions = actions_for(source, &[diag]);

    let action = find_action(&actions, |t| t.contains("redo"))
        .ok_or_else(|| format!("no `redo` action in: {actions:?}"))?;

    assert!(action.is_preferred);
    assert_eq!(action.kind, CodeActionKind::QuickFix);

    let result = edited(source, action);
    assert!(!result.contains("NOWHERE"), "label should be removed after edit, got: {result:?}");

    Ok(())
}

// ===========================================================================
// Action title includes the operator name
// ===========================================================================

#[test]
fn pl410_action_title_names_the_operator() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a next statement with an undefined label
    let source = "while (1) { next GHOST; }\n";

    let ctrl_start = source.find("next GHOST").ok_or("marker not found")?;
    let ctrl_end = ctrl_start + "next GHOST".len();

    let diag = make_diag(
        ctrl_start,
        ctrl_end,
        "PL410",
        "`next GHOST` references a label that is not defined in this file",
    );

    let actions = actions_for(source, &[diag]);

    // THEN the action title mentions the `next` operator by name
    let action = find_action(&actions, |t| t.contains("next"))
        .ok_or_else(|| format!("no action with 'next' in title: {actions:?}"))?;

    assert!(
        action.title.contains("next"),
        "title should name the operator 'next', got: {:?}",
        action.title
    );

    Ok(())
}

// ===========================================================================
// Invalid-range guard
// ===========================================================================

#[test]
fn pl410_non_char_boundary_range_produces_no_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a source with a multi-byte character and a PL410 diagnostic whose
    // range falls inside the multi-byte sequence (not on a char boundary)
    let source = "for my $i (1..10) { next OUTER; }\nmy $s = \"\u{e9}\";\n";

    let char_pos = source.find('\u{e9}').ok_or("marker not found")?;
    // char_pos+1 is inside the 2-byte UTF-8 sequence — not a valid char boundary
    let diag = make_diag(char_pos + 1, char_pos + 2, "PL410", "`next OUTER` undefined label");

    let actions = actions_for(source, &[diag]);

    assert!(
        !actions.iter().any(|a| a.title.contains("next") || a.title.contains("last") || a.title.contains("redo")),
        "expected no PL410 action for invalid range, got: {actions:?}"
    );

    Ok(())
}

// ===========================================================================
// Dispatch smoke test — PL410 reaches the handler
// ===========================================================================

#[test]
fn pl410_dispatch_reaches_handler() -> Result<(), Box<dyn std::error::Error>> {
    // Smoke test: a correctly positioned PL410 diagnostic produces at least
    // one action, confirming the dispatch table routes PL410 to its handler.
    let source = "for my $i (1..10) { next OUTER; }\n";

    let ctrl_start = source.find("next OUTER").ok_or("marker not found")?;
    let ctrl_end = ctrl_start + "next OUTER".len();

    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CodeActionsProvider::new(source.to_string());

    let diags = vec![make_diag(
        ctrl_start,
        ctrl_end,
        "PL410",
        "`next OUTER` references a label that is not defined in this file",
    )];

    let actions = provider.get_code_actions(&ast, (0, source.len()), &diags);

    assert!(
        actions.iter().any(|a| a.title.contains("next")),
        "PL410 route should produce a loop-control action; got: {actions:?}"
    );

    Ok(())
}
