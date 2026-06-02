//! BDD tests for PL410 quick-fix: undefined loop-control label.
//!
//! PL410 fires when `next LABEL`, `last LABEL`, or `redo LABEL` references a
//! label that does not wrap any enclosing loop. The only safe mechanical fix is
//! to drop the label so the statement targets the innermost enclosing loop.
//!
//! Each scenario follows the pattern:
//!   GIVEN  source with a named loop-control statement
//!   WHEN   a PL410 diagnostic is supplied and code actions are requested
//!   THEN   exactly the expected action is returned with correct edits

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
// PL410 - Loop control with undefined label
// ===========================================================================

#[test]
fn next_undefined_label_produces_drop_label_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a `next OUTER` where OUTER is not a defined enclosing label
    let source = "while (1) { next OUTER; }\n";
    let stmt_start = source.find("next OUTER").ok_or("marker not found")?;
    let stmt_end = stmt_start + "next OUTER".len();

    let diag = make_diag(stmt_start, stmt_end, "PL410", "Label 'OUTER' not found for 'next'");

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN there is an action that drops the label
    let action = find_action(&actions, |t| t.contains("OUTER"))
        .ok_or_else(|| format!("no PL410 action in: {:?}", actions))?;

    assert_eq!(action.title, "Remove undefined label 'OUTER' from 'next OUTER'");
    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred, "the drop-label action should be preferred");

    Ok(())
}

#[test]
fn last_undefined_label_edit_removes_label() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a `last DONE` where DONE is undefined
    let source = "for my $i (1..10) { last DONE; }\n";
    let stmt_start = source.find("last DONE").ok_or("marker not found")?;
    let stmt_end = stmt_start + "last DONE".len();

    let diag = make_diag(stmt_start, stmt_end, "PL410", "Label 'DONE' not found for 'last'");

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);
    let action = find_action(&actions, |t| t.contains("DONE"))
        .ok_or_else(|| format!("no PL410 action in: {:?}", actions))?;

    // THEN the edit replaces `last DONE` with bare `last`
    let result = edited(source, action);
    assert!(result.contains("last;") || result.contains("last "), "expected bare 'last' in: {result:?}");
    assert!(!result.contains("DONE"), "label should be removed from: {result:?}");

    Ok(())
}

#[test]
fn redo_undefined_label_produces_drop_label_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a `redo RETRY` where RETRY is not an enclosing label
    let source = "while (1) { redo RETRY; }\n";
    let stmt_start = source.find("redo RETRY").ok_or("marker not found")?;
    let stmt_end = stmt_start + "redo RETRY".len();

    let diag = make_diag(stmt_start, stmt_end, "PL410", "Label 'RETRY' not found for 'redo'");

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN there is an action for redo
    let action = find_action(&actions, |t| t.contains("RETRY"))
        .ok_or_else(|| format!("no PL410 redo action in: {:?}", actions))?;

    assert_eq!(action.title, "Remove undefined label 'RETRY' from 'redo RETRY'");
    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred);

    Ok(())
}

#[test]
fn pl410_action_title_uses_operator_and_label_from_source() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN three different operators — verify title always names both
    let cases = [
        ("while (1) { next LOOP; }\n", "next LOOP", "next", "LOOP"),
        ("while (1) { last DONE; }\n", "last DONE", "last", "DONE"),
        ("while (1) { redo START; }\n", "redo START", "redo", "START"),
    ];

    for (source, needle, op, label) in cases {
        let stmt_start = source.find(needle).ok_or("marker not found")?;
        let stmt_end = stmt_start + needle.len();
        let diag = make_diag(stmt_start, stmt_end, "PL410", &format!("Label '{label}' not found"));
        let actions = actions_for(source, &[diag]);

        let action = find_action(&actions, |t| t.contains(label))
            .ok_or_else(|| format!("no action for '{needle}' in: {:?}", actions))?;

        let expected = format!("Remove undefined label '{label}' from '{op} {label}'");
        assert_eq!(action.title, expected, "title mismatch for {needle}");
    }

    Ok(())
}

#[test]
fn pl410_invalid_range_returns_no_actions() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a diagnostic whose byte range lands inside a multi-byte character
    let source = "while (1) { next OUTER; }\nmy $x = \"\u{00e9}\";\n";
    let char_start = source.find('\u{00e9}').ok_or("marker not found")?;
    // Point into the middle of the two-byte UTF-8 sequence — not a char boundary
    let diag = make_diag(char_start + 1, char_start + 2, "PL410", "Label 'OUTER' not found");

    let actions = actions_for(source, &[diag]);

    // THEN no PL410 action is produced for the invalid range
    assert!(
        !actions.iter().any(|a| a.title.contains("Remove undefined label")),
        "expected no PL410 action for non-char-boundary range, got: {:?}",
        actions
    );

    Ok(())
}

#[test]
fn pl410_dispatch_smoke_test_reaches_handler() -> Result<(), Box<dyn std::error::Error>> {
    // Smoke test: PL410 reaches fix_loop_control_undefined_label via dispatch table.
    // Alongside PL602 to confirm no cross-routing.
    let source = "use strict;\nwhile (1) { next OUTER; }\n$SIG{__DIE__} = sub {};\n";

    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CodeActionsProvider::new(source.to_string());

    let next_start = source.find("next OUTER").ok_or("next OUTER not found")?;
    let next_end = next_start + "next OUTER".len();
    let sig_start = source.find("$SIG{__DIE__}").ok_or("$SIG not found")?;
    let sig_end = source[sig_start..].find(";\n").ok_or("sig end not found")? + sig_start;

    let diags = vec![
        make_diag(next_start, next_end, "PL410", "Label 'OUTER' not found for 'next'"),
        make_diag(sig_start, sig_end, "PL602", "Global $SIG assignment"),
    ];

    let actions = provider.get_code_actions(&ast, (0, source.len()), &diags);

    let has_pl410 = actions.iter().any(|a| a.title.contains("OUTER"));
    let has_pl602 = actions.iter().any(|a| a.title.contains("local"));

    assert!(has_pl410, "PL410 route not producing action; actions: {:?}", actions);
    assert!(has_pl602, "PL602 route not producing action; actions: {:?}", actions);

    Ok(())
}
