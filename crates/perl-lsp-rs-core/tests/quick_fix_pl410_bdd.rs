//! BDD tests for the PL410 quick-fix handler:
//!   PL410 — `next`/`last`/`redo LABEL` where the label is not defined
//!            (`fix_loop_control_undefined_label`)
//!
//! Each scenario follows the pattern:
//!   GIVEN  source containing a loop-control statement with an undefined label
//!   WHEN   diagnostics are produced and code actions are requested
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
// PL410 — next/last/redo to undefined label
// ===========================================================================

#[test]
fn loop_control_pl410_next_produces_drop_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a while loop containing `next OUTER` where OUTER is not defined
    let source = "while (1) { next OUTER; }\n";
    let stmt_start = source.find("next OUTER").ok_or("marker not found")?;
    let stmt_end = stmt_start + "next OUTER".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`next OUTER` references a label that is not defined in this file",
    );

    // WHEN code actions are requested for the diagnostic range
    let actions = actions_for(source, &[diag]);

    // THEN there is a drop-label action for 'next'
    let action = find_action(&actions, |t| t.contains("'next'"))
        .ok_or_else(|| format!("no PL410 drop action in: {:?}", actions))?;

    assert_eq!(action.title, "Remove undefined label from 'next'");
    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred, "the drop-label action should be preferred");

    Ok(())
}

#[test]
fn loop_control_pl410_last_produces_drop_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a for loop containing `last DONE` where DONE is not defined
    let source = "for my $i (1..10) { last DONE; }\n";
    let stmt_start = source.find("last DONE").ok_or("marker not found")?;
    let stmt_end = stmt_start + "last DONE".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`last DONE` references a label that is not defined in this file",
    );

    let actions = actions_for(source, &[diag]);

    let action = find_action(&actions, |t| t.contains("'last'"))
        .ok_or_else(|| format!("no PL410 drop action for 'last' in: {:?}", actions))?;

    assert_eq!(action.title, "Remove undefined label from 'last'");
    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred);

    Ok(())
}

#[test]
fn loop_control_pl410_redo_produces_drop_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a foreach loop containing `redo RETRY` where RETRY is not defined
    let source = "foreach my $x (@items) { redo RETRY; }\n";
    let stmt_start = source.find("redo RETRY").ok_or("marker not found")?;
    let stmt_end = stmt_start + "redo RETRY".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`redo RETRY` references a label that is not defined in this file",
    );

    let actions = actions_for(source, &[diag]);

    let action = find_action(&actions, |t| t.contains("'redo'"))
        .ok_or_else(|| format!("no PL410 drop action for 'redo' in: {:?}", actions))?;

    assert_eq!(action.title, "Remove undefined label from 'redo'");
    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred);

    Ok(())
}

#[test]
fn loop_control_pl410_edit_drops_the_label() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN source with `next MISSING` where the label does not exist
    let source = "while (1) { next MISSING; }\n";
    let stmt_start = source.find("next MISSING").ok_or("marker not found")?;
    let stmt_end = stmt_start + "next MISSING".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`next MISSING` references a label that is not defined in this file",
    );
    let actions = actions_for(source, &[diag]);

    let action = find_action(&actions, |t| t.contains("'next'"))
        .ok_or_else(|| format!("no PL410 action in: {:?}", actions))?;
    let result = edited(source, action);

    // THEN the label is dropped; only the bare operator remains
    assert_eq!(result, "while (1) { next; }\n");

    Ok(())
}

#[test]
fn loop_control_pl410_invalid_range_is_ignored() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a PL410 diagnostic whose range straddles a multi-byte character boundary
    let source = "while (1) { next OUTER; }\nmy $x = \"\u{e9}\";\n";
    let char_start = source.find('\u{e9}').ok_or("marker not found")?;
    // Point into the middle of the two-byte UTF-8 sequence — not a char boundary.
    let diag = make_diag(
        char_start + 1,
        char_start + 2,
        "PL410",
        "`next OUTER` references a label that is not defined in this file",
    );

    let actions = actions_for(source, &[diag]);

    // THEN no drop-label action is produced for the invalid range
    assert!(
        !actions.iter().any(|a| a.title.contains("Remove undefined label")),
        "expected no PL410 action for non-char-boundary range, got: {actions:?}"
    );

    Ok(())
}

#[test]
fn loop_control_pl410_dispatch_smoke_test() -> Result<(), Box<dyn std::error::Error>> {
    // Smoke test: confirms the dispatch table in diagnostic_routes.rs routes
    // PL410 to the handler and produces at least one action for a valid diagnostic.
    let source = "while (1) { next NOWHERE; }\n";
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CodeActionsProvider::new(source.to_string());

    let stmt_start = source.find("next NOWHERE").ok_or("marker not found")?;
    let stmt_end = stmt_start + "next NOWHERE".len();

    let diags = vec![make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`next NOWHERE` references a label that is not defined in this file",
    )];

    let actions = provider.get_code_actions(&ast, (0, source.len()), &diags);

    assert!(
        actions.iter().any(|a| a.title.contains("Remove undefined label")),
        "PL410 dispatch route not producing an action; actions: {actions:?}"
    );

    Ok(())
}
