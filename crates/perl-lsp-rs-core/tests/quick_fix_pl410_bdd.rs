//! BDD tests for PL410 quick-fix handler: `fix_loop_control_undefined_label`
//!
//! PL410 fires when `next LABEL`, `last LABEL`, or `redo LABEL` targets a label
//! that is not defined in the current file. The fix drops the undefined label so
//! the statement targets the innermost enclosing loop instead.
//!
//! Each scenario follows the pattern:
//!   GIVEN  source containing a loop-control statement with an undefined label
//!   WHEN   a PL410 diagnostic is produced and code actions are requested
//!   THEN   exactly the expected action is returned with a correct edit

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

/// Apply all edits from the action and return the resulting source.
fn edited(source: &str, action: &CodeAction) -> String {
    let mut edits = action.edit.changes.clone();
    edits.sort_by_key(|e| std::cmp::Reverse(e.location.start));
    let mut out = source.to_string();
    for edit in edits {
        out.replace_range(edit.location.start..edit.location.end, &edit.new_text);
    }
    out
}

fn find_action(actions: &[CodeAction], pred: impl Fn(&str) -> bool) -> Option<&CodeAction> {
    actions.iter().find(|a| pred(&a.title))
}

// ===========================================================================
// PL410 — `next LABEL` with undefined label
// ===========================================================================

#[test]
fn next_undefined_label_produces_drop_label_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a `next MISSING` statement where MISSING is not defined
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

    // THEN there is an action offering to drop the label
    let action = find_action(&actions, |t| t.contains("next"))
        .ok_or_else(|| format!("no PL410 action in: {:?}", actions))?;

    assert_eq!(action.title, "Remove undefined label (use bare 'next')");
    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred, "drop-label action should be is_preferred");

    Ok(())
}

#[test]
fn next_undefined_label_edit_strips_label_from_source() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN `next MISSING` in a while loop
    let source = "while (1) {\n    next MISSING;\n}\n";
    let stmt_start = source.find("next MISSING").ok_or("marker not found")?;
    let stmt_end = stmt_start + "next MISSING".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`next MISSING` references a label that is not defined in this file",
    );
    let actions = actions_for(source, &[diag]);

    let action = find_action(&actions, |t| t.contains("next"))
        .ok_or_else(|| format!("no PL410 action in: {:?}", actions))?;
    let result = edited(source, action);

    // THEN the label is stripped; the semicolon and surrounding code are intact
    assert_eq!(result, "while (1) {\n    next;\n}\n");

    Ok(())
}

// ===========================================================================
// PL410 — `last LABEL` with undefined label
// ===========================================================================

#[test]
fn last_undefined_label_edit_strips_label_from_source() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN `last NOPE` in a for loop
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

    let action = find_action(&actions, |t| t.contains("last"))
        .ok_or_else(|| format!("no PL410 action in: {:?}", actions))?;
    let result = edited(source, action);

    // THEN `last NOPE` is replaced with `last`
    assert_eq!(result, "for my $x (1..10) {\n    last;\n}\n");

    Ok(())
}

// ===========================================================================
// PL410 — `redo LABEL` with undefined label
// ===========================================================================

#[test]
fn redo_undefined_label_edit_strips_label_from_source() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN `redo TOP` in a foreach loop
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

    let action = find_action(&actions, |t| t.contains("redo"))
        .ok_or_else(|| format!("no PL410 action in: {:?}", actions))?;
    let result = edited(source, action);

    // THEN `redo TOP` is replaced with `redo`
    assert_eq!(result, "foreach my $item (@items) {\n    redo;\n}\n");

    Ok(())
}

// ===========================================================================
// PL410 — guard: invalid range returns no actions
// ===========================================================================

#[test]
fn pl410_out_of_bounds_range_produces_no_actions() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a diagnostic range that extends beyond the source
    let source = "while (1) {\n    next MISSING;\n}\n";
    let out_of_bounds_start = source.len() + 1;
    let out_of_bounds_end = source.len() + 10;

    let diag = make_diag(
        out_of_bounds_start,
        out_of_bounds_end,
        "PL410",
        "`next MISSING` references a label that is not defined in this file",
    );
    let actions = actions_for(source, &[diag]);

    // THEN no action is produced
    assert!(
        !actions.iter().any(|a| a.title.contains("next")),
        "expected no PL410 action for out-of-bounds range, got: {actions:?}"
    );

    Ok(())
}

// ===========================================================================
// Cross-cutting: dispatch-table smoke test
// ===========================================================================

#[test]
fn pl410_dispatch_smoke_test_all_three_operators() -> Result<(), Box<dyn std::error::Error>> {
    // Smoke test: verify the PL410 route in diagnostic_routes.rs dispatches
    // correctly for next, last, and redo in a single pass.
    let source = "while (1) {\n    next MISSING;\n    last GONE;\n    redo AWAY;\n}\n";
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CodeActionsProvider::new(source.to_string());

    let next_start = source.find("next MISSING").ok_or("next marker")?;
    let last_start = source.find("last GONE").ok_or("last marker")?;
    let redo_start = source.find("redo AWAY").ok_or("redo marker")?;

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
        make_diag(
            redo_start,
            redo_start + "redo AWAY".len(),
            "PL410",
            "`redo AWAY` references a label that is not defined in this file",
        ),
    ];

    let actions = provider.get_code_actions(&ast, (0, source.len()), &diags);

    let has_next = actions.iter().any(|a| a.title.contains("'next'"));
    let has_last = actions.iter().any(|a| a.title.contains("'last'"));
    let has_redo = actions.iter().any(|a| a.title.contains("'redo'"));

    assert!(has_next, "PL410 route not producing next action; actions: {:?}", actions);
    assert!(has_last, "PL410 route not producing last action; actions: {:?}", actions);
    assert!(has_redo, "PL410 route not producing redo action; actions: {:?}", actions);

    Ok(())
}
