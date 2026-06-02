//! BDD tests for the PL410 quick-fix handler: `fix_loop_control_undefined_label`.
//!
//! PL410 fires when `next LABEL`, `last LABEL`, or `redo LABEL` references a
//! label that does not exist in the enclosing scope.  The fix drops the label
//! so the statement targets the innermost enclosing loop.
//!
//! Each scenario follows the pattern:
//!   GIVEN  source containing a loop-control statement with an undefined label
//!   WHEN   a PL410 diagnostic is supplied and code actions are requested
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
        severity: DiagnosticSeverity::Error,
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

fn apply_first_edit(source: &str, action: &CodeAction) -> String {
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
// PL410 — next LABEL: undefined label
// ===========================================================================

#[test]
fn next_undefined_label_produces_remove_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a loop that uses `next OUTER` where OUTER is not defined
    let source = "for my $i (1..10) {\n    next OUTER;\n}\n";
    let stmt_start = source.find("next OUTER").ok_or("marker not found")?;
    let stmt_end = stmt_start + "next OUTER".len();

    let diag = make_diag(stmt_start, stmt_end, "PL410", "Label 'OUTER' not found for 'next'");

    // WHEN code actions are requested for the diagnostic range
    let actions = actions_for(source, &[diag]);

    // THEN there is an action offering to remove the undefined label
    let action = find_action(&actions, |t| t.contains("next"))
        .ok_or_else(|| format!("no 'next' action in: {:?}", actions))?;

    assert_eq!(action.title, "Remove undefined label (use 'next' without label)");
    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(
        action.is_preferred,
        "the fix should be preferred — there is exactly one sensible option"
    );

    Ok(())
}

#[test]
fn next_undefined_label_edit_drops_label_text() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN source with `next OUTER;`
    let source = "for my $i (1..5) {\n    next OUTER;\n}\n";
    let stmt_start = source.find("next OUTER").ok_or("marker not found")?;
    let stmt_end = stmt_start + "next OUTER".len();

    let diag = make_diag(stmt_start, stmt_end, "PL410", "Label 'OUTER' not found for 'next'");
    let actions = actions_for(source, &[diag]);

    let action = find_action(&actions, |t| t.contains("next"))
        .ok_or_else(|| format!("no 'next' action in: {:?}", actions))?;
    let result = apply_first_edit(source, action);

    // THEN `next OUTER` is replaced by `next`; semicolon and surrounding code intact
    assert!(result.contains("next;"), "expected 'next;' in result, got: {result:?}");
    assert!(!result.contains("OUTER"), "expected label 'OUTER' to be removed, got: {result:?}");

    Ok(())
}

// ===========================================================================
// PL410 — last LABEL: undefined label
// ===========================================================================

#[test]
fn last_undefined_label_produces_remove_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a loop that uses `last DONE` where DONE is not defined
    let source = "while (1) {\n    last DONE;\n}\n";
    let stmt_start = source.find("last DONE").ok_or("marker not found")?;
    let stmt_end = stmt_start + "last DONE".len();

    let diag = make_diag(stmt_start, stmt_end, "PL410", "Label 'DONE' not found for 'last'");

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN there is a 'last' action
    let action = find_action(&actions, |t| t.contains("last"))
        .ok_or_else(|| format!("no 'last' action in: {:?}", actions))?;

    assert_eq!(action.title, "Remove undefined label (use 'last' without label)");
    assert!(action.is_preferred);

    Ok(())
}

// ===========================================================================
// PL410 — redo LABEL: undefined label
// ===========================================================================

#[test]
fn redo_undefined_label_produces_remove_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a loop that uses `redo RETRY` where RETRY is not defined
    let source = "for (1..3) {\n    redo RETRY;\n}\n";
    let stmt_start = source.find("redo RETRY").ok_or("marker not found")?;
    let stmt_end = stmt_start + "redo RETRY".len();

    let diag = make_diag(stmt_start, stmt_end, "PL410", "Label 'RETRY' not found for 'redo'");

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN there is a 'redo' action
    let action = find_action(&actions, |t| t.contains("redo"))
        .ok_or_else(|| format!("no 'redo' action in: {:?}", actions))?;

    assert_eq!(action.title, "Remove undefined label (use 'redo' without label)");
    assert!(action.is_preferred);

    Ok(())
}

// ===========================================================================
// Invalid-range guard
// ===========================================================================

#[test]
fn invalid_range_produces_no_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a PL410 diagnostic whose range is out-of-bounds or mid-char
    let source = "for (1..3) {\n    next OUTER;\n}\n";

    // Out-of-bounds end
    let bad_end = source.len() + 5;
    let diag_oob = make_diag(0, bad_end, "PL410", "Label 'OUTER' not found for 'next'");
    let actions_oob = actions_for(source, &[diag_oob]);
    assert!(
        !actions_oob.iter().any(|a| a.title.contains("next")),
        "expected no action for out-of-bounds range, got: {:?}",
        actions_oob
    );

    // Non-char-boundary range (inside a multi-byte character)
    let multibyte_source = "my $x = \"\u{e9}\";\nnext OUTER;\n";
    let char_start = multibyte_source.find('\u{e9}').ok_or("char not found")?;
    let diag_mid = make_diag(char_start + 1, char_start + 2, "PL410", "Label 'OUTER' not found");
    let actions_mid = actions_for(multibyte_source, &[diag_mid]);
    assert!(
        !actions_mid.iter().any(|a| a.title.contains("next")),
        "expected no action for mid-char-boundary range, got: {:?}",
        actions_mid
    );

    Ok(())
}

// ===========================================================================
// Dispatch smoke test
// ===========================================================================

#[test]
fn pl410_dispatch_reaches_handler_for_all_three_operators() -> Result<(), Box<dyn std::error::Error>>
{
    // Smoke test: each operator code reaches the fix handler when the diagnostic
    // code is PL410, confirming the dispatch table is wired correctly.

    let source = "for my $i (1..10) {\n    next OUTER;\n    last OUTER;\n    redo OUTER;\n}\n";
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CodeActionsProvider::new(source.to_string());

    let next_start = source.find("next OUTER").ok_or("next not found")?;
    let last_start = source.find("last OUTER").ok_or("last not found")?;
    let redo_start = source.find("redo OUTER").ok_or("redo not found")?;

    let diags = vec![
        make_diag(
            next_start,
            next_start + "next OUTER".len(),
            "PL410",
            "Label 'OUTER' not found for 'next'",
        ),
        make_diag(
            last_start,
            last_start + "last OUTER".len(),
            "PL410",
            "Label 'OUTER' not found for 'last'",
        ),
        make_diag(
            redo_start,
            redo_start + "redo OUTER".len(),
            "PL410",
            "Label 'OUTER' not found for 'redo'",
        ),
    ];

    let actions = provider.get_code_actions(&ast, (0, source.len()), &diags);

    let has_next = actions.iter().any(|a| a.title.contains("'next'"));
    let has_last = actions.iter().any(|a| a.title.contains("'last'"));
    let has_redo = actions.iter().any(|a| a.title.contains("'redo'"));

    assert!(has_next, "PL410 'next' route not producing action; actions: {:?}", actions);
    assert!(has_last, "PL410 'last' route not producing action; actions: {:?}", actions);
    assert!(has_redo, "PL410 'redo' route not producing action; actions: {:?}", actions);

    Ok(())
}
