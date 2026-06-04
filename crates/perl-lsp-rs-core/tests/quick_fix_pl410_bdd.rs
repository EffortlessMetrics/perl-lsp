//! BDD tests for the PL410 quick-fix handler — `fix_loop_control_undefined_label`.
//!
//! `next LABEL`, `last LABEL`, and `redo LABEL` that reference an undefined
//! label produce a PL410 diagnostic. This handler offers a single preferred
//! action that removes the label so the statement targets the innermost loop.
//!
//! Each scenario follows the pattern:
//!   GIVEN  source with a loop-control statement referencing an undefined label
//!   WHEN   a PL410 diagnostic is supplied and code actions are requested
//!   THEN   exactly the expected "Remove label" action is returned

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
// PL410 — next LABEL
// ===========================================================================

#[test]
fn next_undefined_label_produces_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a while loop with `next MISSING` where MISSING is not defined
    let source = "while (1) {\n    next MISSING;\n}\n";
    let stmt_start = source.find("next MISSING").ok_or("marker not found")?;
    let stmt_end = stmt_start + "next MISSING".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`next MISSING` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN a preferred QuickFix action is offered
    let action = find_action(&actions, |t| t.contains("MISSING") && t.contains("next"))
        .ok_or_else(|| format!("no PL410 action for `next MISSING` in: {:?}", actions))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred, "PL410 fix must be is_preferred (single sensible fix)");

    let result = edited(source, action);
    assert_eq!(result, "while (1) {\n    next;\n}\n", "label should be stripped from `next`");

    Ok(())
}

// ===========================================================================
// PL410 — last LABEL
// ===========================================================================

#[test]
fn last_undefined_label_edit_removes_label() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a for loop with `last NOPE` where NOPE is not defined
    let source = "for my $x (1..10) {\n    last NOPE;\n}\n";
    let stmt_start = source.find("last NOPE").ok_or("marker not found")?;
    let stmt_end = stmt_start + "last NOPE".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`last NOPE` references a label that is not defined in this file",
    );

    let actions = actions_for(source, &[diag]);
    let action = find_action(&actions, |t| t.contains("NOPE") && t.contains("last"))
        .ok_or_else(|| format!("no PL410 action for `last NOPE` in: {:?}", actions))?;

    // THEN the edit strips the label, leaving just `last`
    let result = edited(source, action);
    assert_eq!(result, "for my $x (1..10) {\n    last;\n}\n");

    Ok(())
}

// ===========================================================================
// PL410 — redo LABEL
// ===========================================================================

#[test]
fn redo_undefined_label_edit_removes_label() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a foreach loop with `redo TOP` where TOP is not defined
    let source = "foreach my $item (@items) {\n    redo TOP;\n}\n";
    let stmt_start = source.find("redo TOP").ok_or("marker not found")?;
    let stmt_end = stmt_start + "redo TOP".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`redo TOP` references a label that is not defined in this file",
    );

    let actions = actions_for(source, &[diag]);
    let action = find_action(&actions, |t| t.contains("TOP") && t.contains("redo"))
        .ok_or_else(|| format!("no PL410 action for `redo TOP` in: {:?}", actions))?;

    let result = edited(source, action);
    assert_eq!(result, "foreach my $item (@items) {\n    redo;\n}\n");

    Ok(())
}

// ===========================================================================
// PL410 — title naming
// ===========================================================================

#[test]
fn action_title_includes_operator_and_label() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a PL410 diagnostic for `next OUTER`
    let source = "while (1) {\n    next OUTER;\n}\n";
    let stmt_start = source.find("next OUTER").ok_or("marker not found")?;
    let stmt_end = stmt_start + "next OUTER".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`next OUTER` references a label that is not defined in this file",
    );

    let actions = actions_for(source, &[diag]);
    let action = find_action(&actions, |t| t.contains("OUTER"))
        .ok_or_else(|| format!("no PL410 action for `next OUTER` in: {:?}", actions))?;

    // THEN the title names both the label and the operator
    assert_eq!(action.title, "Remove label 'OUTER' from 'next' (targets innermost loop)");

    Ok(())
}

// ===========================================================================
// Guard: invalid range returns no actions
// ===========================================================================

#[test]
fn invalid_range_produces_no_actions() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a PL410 diagnostic whose byte range falls in a multi-byte UTF-8
    // character (not on a char boundary) — simulates a malformed diagnostic
    let source = "while (1) {\n    next MISSING;\n}\nmy $s = \"\u{e9}\";\n";
    let char_pos = source.find('\u{e9}').ok_or("UTF-8 marker not found")?;

    let diag = make_diag(
        char_pos + 1, // splits the 2-byte é sequence — not a char boundary
        char_pos + 2,
        "PL410",
        "`next MISSING` references a label that is not defined in this file",
    );

    let actions = actions_for(source, &[diag]);

    assert!(
        !actions.iter().any(|a| a.title.contains("innermost loop")),
        "expected no PL410 action for non-char-boundary range, got: {:?}",
        actions
    );

    Ok(())
}

// ===========================================================================
// Dispatch smoke test
// ===========================================================================

#[test]
fn dispatch_table_routes_pl410() -> Result<(), Box<dyn std::error::Error>> {
    // Smoke test: confirm the diagnostic_routes dispatch table routes PL410 to
    // fix_loop_control_undefined_label and produces actions for multiple diagnostics.
    let source = "while (1) {\n    next MISSING;\n    last GONE;\n}\n";
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CodeActionsProvider::new(source.to_string());

    let next_start = source.find("next MISSING").ok_or("next MISSING not found")?;
    let last_start = source.find("last GONE").ok_or("last GONE not found")?;

    let diags = vec![
        make_diag(
            next_start,
            next_start + "next MISSING".len(),
            "PL410",
            "`next MISSING` references a label that is not defined in this file",
        ),
        make_diag(
            last_start,
            last_start + "last GONE".len(),
            "PL410",
            "`last GONE` references a label that is not defined in this file",
        ),
    ];

    let actions = provider.get_code_actions(&ast, (0, source.len()), &diags);

    let has_next_fix =
        actions.iter().any(|a| a.title.contains("MISSING") && a.title.contains("next"));
    let has_last_fix = actions.iter().any(|a| a.title.contains("GONE") && a.title.contains("last"));

    assert!(
        has_next_fix,
        "PL410 route not producing action for `next MISSING`; actions: {:?}",
        actions
    );
    assert!(
        has_last_fix,
        "PL410 route not producing action for `last GONE`; actions: {:?}",
        actions
    );

    Ok(())
}
