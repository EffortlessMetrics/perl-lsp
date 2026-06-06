//! BDD tests for the PL410 quick-fix handler (`fix_loop_control_undefined_label`).
//!
//! Each scenario follows:
//!   GIVEN  source with a `next`/`last`/`redo LABEL` targeting an undefined label
//!   WHEN   a PL410 diagnostic is supplied and code actions are requested
//!   THEN   exactly one preferred action is returned that drops the label

use perl_lsp_rs_core::providers::code_actions::{CodeAction, CodeActionKind, CodeActionsProvider};
use perl_lsp_rs_core::providers::diagnostics::{Diagnostic, DiagnosticSeverity};
use perl_parser::Parser;
use perl_tdd_support::must;

// ---------------------------------------------------------------------------
// Helpers (mirrors the pattern from quick_fix_new_codes_bdd.rs)
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
// PL410 — next LABEL → next
// ===========================================================================

#[test]
fn next_undefined_label_produces_drop_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a `next OUTER` where OUTER is not defined in the file
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

    // THEN there is exactly one action offering to drop the label
    let action = find_action(&actions, |t| t.contains("next"))
        .ok_or_else(|| format!("no 'next' action in: {:?}", actions))?;

    assert!(
        action.title.contains("next OUTER"),
        "title should include original text: {}",
        action.title
    );
    assert!(
        action.title.contains("bare `next`"),
        "title should mention bare form: {}",
        action.title
    );
    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred, "the drop-label action should be preferred");

    Ok(())
}

#[test]
fn next_undefined_label_edit_replaces_with_bare_next() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN source with `next MISSING`
    let source = "for my $i (1..5) { next MISSING; }\n";
    let stmt_start = source.find("next MISSING").ok_or("marker not found")?;
    let stmt_end = stmt_start + "next MISSING".len();

    let diag =
        make_diag(stmt_start, stmt_end, "PL410", "`next MISSING` references undefined label");
    let actions = actions_for(source, &[diag]);

    let action = find_action(&actions, |t| t.contains("next"))
        .ok_or_else(|| format!("no 'next' action in: {actions:?}"))?;
    let result = edited(source, action);

    // THEN `next MISSING` becomes `next`
    assert_eq!(result, "for my $i (1..5) { next; }\n");

    Ok(())
}

// ===========================================================================
// PL410 — last LABEL → last
// ===========================================================================

#[test]
fn last_undefined_label_produces_drop_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN `last PHANTOM` inside a while loop where PHANTOM is not defined
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
    let action = find_action(&actions, |t| t.contains("last"))
        .ok_or_else(|| format!("no 'last' action in: {actions:?}"))?;

    assert!(
        action.title.contains("last PHANTOM"),
        "title should include original: {}",
        action.title
    );
    assert!(action.is_preferred);

    let result = edited(source, action);
    assert_eq!(result, "while (1) { last; }\n");

    Ok(())
}

// ===========================================================================
// PL410 — redo LABEL → redo
// ===========================================================================

#[test]
fn redo_undefined_label_produces_drop_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN `redo NOWHERE` where NOWHERE is not defined
    let source = "for my $i (1..5) { redo NOWHERE; }\n";
    let stmt_start = source.find("redo NOWHERE").ok_or("marker not found")?;
    let stmt_end = stmt_start + "redo NOWHERE".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`redo NOWHERE` references a label that is not defined in this file",
    );

    let actions = actions_for(source, &[diag]);
    let action = find_action(&actions, |t| t.contains("redo"))
        .ok_or_else(|| format!("no 'redo' action in: {actions:?}"))?;

    assert!(
        action.title.contains("redo NOWHERE"),
        "title should include original: {}",
        action.title
    );
    assert!(action.is_preferred);

    let result = edited(source, action);
    assert_eq!(result, "for my $i (1..5) { redo; }\n");

    Ok(())
}

// ===========================================================================
// Guard: invalid / non-char-boundary range returns no action
// ===========================================================================

#[test]
fn invalid_range_returns_no_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a PL410 diagnostic whose byte range falls on a non-char boundary
    let source = "for my $i (1..3) { next GHOST; }\nmy $x = \"\u{e9}\";\n";
    let char_start = source.find('\u{e9}').ok_or("marker not found")?;

    // Range deliberately split inside the multi-byte character
    let diag = make_diag(
        char_start + 1,
        char_start + 2,
        "PL410",
        "`next GHOST` references undefined label",
    );
    let actions = actions_for(source, &[diag]);

    assert!(
        !actions.iter().any(|a| a.title.contains("Drop undefined label")),
        "expected no PL410 action for non-char-boundary range, got: {actions:?}"
    );

    Ok(())
}

// ===========================================================================
// Guard: non-loop-control operator does not produce an action
// ===========================================================================

#[test]
fn non_loop_control_op_range_returns_no_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a PL410 diagnostic pointing at source text that is NOT next/last/redo
    let source = "print \"hello\";\n";
    let diag = make_diag(0, 5, "PL410", "`print LABEL` — should never fire, just a guard test");
    let actions = actions_for(source, &[diag]);

    assert!(
        !actions.iter().any(|a| a.title.contains("Drop undefined label")),
        "expected no PL410 action when text is not a loop-control op, got: {actions:?}"
    );

    Ok(())
}

// ===========================================================================
// Dispatch smoke test: PL410 route is wired up
// ===========================================================================

#[test]
fn pl410_dispatch_route_is_wired() -> Result<(), Box<dyn std::error::Error>> {
    // Smoke test: a correctly-positioned PL410 diagnostic produces at least one action.
    let source = "for my $i (1..10) { next OUTER; }\n";
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CodeActionsProvider::new(source.to_string());

    let stmt_start = source.find("next OUTER").ok_or("marker not found")?;
    let stmt_end = stmt_start + "next OUTER".len();

    let diags = vec![make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`next OUTER` references a label that is not defined in this file",
    )];

    let actions = provider.get_code_actions(&ast, (0, source.len()), &diags);

    assert!(
        actions.iter().any(|a| a.title.contains("next")),
        "PL410 dispatch route did not produce an action; actions: {actions:?}"
    );

    Ok(())
}
