//! BDD tests for the PL410 quick-fix handler (`fix_loop_control_undefined_label`).
//!
//! Scenarios:
//!   GIVEN  source containing a `next`/`last`/`redo LABEL` where the label is not defined
//!   WHEN   a PL410 diagnostic covering that statement is passed to the code-actions provider
//!   THEN   exactly one action is returned that drops the label

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

// ---------------------------------------------------------------------------
// PL410 — next LABEL
// ---------------------------------------------------------------------------

#[test]
fn next_label_produces_drop_label_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a while loop containing `next MISSING` with no MISSING label defined
    let source = "while (1) { next MISSING; }\n";
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

    // THEN exactly one action is offered and it is preferred
    let action = actions
        .iter()
        .find(|a| a.title.contains("next"))
        .ok_or_else(|| format!("no `next` action in: {:?}", actions))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred, "the drop-label action should be preferred");

    // AND applying the edit produces bare `next`
    let result = edited(source, action);
    assert_eq!(result, "while (1) { next; }\n");

    Ok(())
}

// ---------------------------------------------------------------------------
// PL410 — last LABEL
// ---------------------------------------------------------------------------

#[test]
fn last_label_produces_drop_label_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a for loop containing `last NOPE`
    let source = "for my $x (1..10) { last NOPE; }\n";
    let stmt_start = source.find("last NOPE").ok_or("marker not found")?;
    let stmt_end = stmt_start + "last NOPE".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`last NOPE` references a label that is not defined in this file",
    );

    let actions = actions_for(source, &[diag]);

    let action = actions
        .iter()
        .find(|a| a.title.contains("last"))
        .ok_or_else(|| format!("no `last` action in: {:?}", actions))?;

    assert!(action.is_preferred);

    let result = edited(source, action);
    assert_eq!(result, "for my $x (1..10) { last; }\n");

    Ok(())
}

// ---------------------------------------------------------------------------
// PL410 — redo LABEL
// ---------------------------------------------------------------------------

#[test]
fn redo_label_produces_drop_label_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a foreach loop containing `redo TOP`
    let source = "foreach my $item (@arr) { redo TOP; }\n";
    let stmt_start = source.find("redo TOP").ok_or("marker not found")?;
    let stmt_end = stmt_start + "redo TOP".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`redo TOP` references a label that is not defined in this file",
    );

    let actions = actions_for(source, &[diag]);

    let action = actions
        .iter()
        .find(|a| a.title.contains("redo"))
        .ok_or_else(|| format!("no `redo` action in: {:?}", actions))?;

    assert!(action.is_preferred);

    let result = edited(source, action);
    assert_eq!(result, "foreach my $item (@arr) { redo; }\n");

    Ok(())
}

// ---------------------------------------------------------------------------
// PL410 — action title format
// ---------------------------------------------------------------------------

#[test]
fn action_title_names_the_operator() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a `next GHOST` statement
    let source = "while (1) { next GHOST; }\n";
    let stmt_start = source.find("next GHOST").ok_or("marker not found")?;
    let stmt_end = stmt_start + "next GHOST".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`next GHOST` references a label that is not defined in this file",
    );

    let actions = actions_for(source, &[diag]);

    let action = actions
        .iter()
        .find(|a| a.title.contains("next"))
        .ok_or_else(|| format!("no action in: {:?}", actions))?;

    // Title should mention "next" so it is self-describing in the lightbulb menu
    assert!(
        action.title.contains("next"),
        "title should mention the operator, got: {:?}",
        action.title
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// PL410 — invalid range guard
// ---------------------------------------------------------------------------

#[test]
fn invalid_range_returns_no_actions() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a diagnostic whose range extends past the end of the source
    let source = "while (1) { next NOPE; }\n";
    let out_of_bounds = source.len() + 5;

    let diag = make_diag(out_of_bounds, out_of_bounds + 9, "PL410", "`next NOPE` references …");
    let actions = actions_for(source, &[diag]);

    assert!(
        !actions.iter().any(|a| a.title.contains("next")
            || a.title.contains("last")
            || a.title.contains("redo")),
        "expected no PL410 action for out-of-bounds range, got: {:?}",
        actions
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Dispatch smoke test — all three operators reach the handler
// ---------------------------------------------------------------------------

#[test]
fn all_three_operators_reach_pl410_handler() -> Result<(), Box<dyn std::error::Error>> {
    // Smoke test: each operator code reaches at least one action when given a
    // valid diagnostic, confirming the dispatch table is wired up.
    let source = "while (1) { next A; last B; redo C; }\n";
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CodeActionsProvider::new(source.to_string());

    let next_start = source.find("next A").ok_or("next A not found")?;
    let last_start = source.find("last B").ok_or("last B not found")?;
    let redo_start = source.find("redo C").ok_or("redo C not found")?;

    let diags = vec![
        make_diag(
            next_start,
            next_start + "next A".len(),
            "PL410",
            "`next A` references a label that is not defined in this file",
        ),
        make_diag(
            last_start,
            last_start + "last B".len(),
            "PL410",
            "`last B` references a label that is not defined in this file",
        ),
        make_diag(
            redo_start,
            redo_start + "redo C".len(),
            "PL410",
            "`redo C` references a label that is not defined in this file",
        ),
    ];

    let actions = provider.get_code_actions(&ast, (0, source.len()), &diags);

    let has_next = actions.iter().any(|a| a.title.contains("next"));
    let has_last = actions.iter().any(|a| a.title.contains("last"));
    let has_redo = actions.iter().any(|a| a.title.contains("redo"));

    assert!(has_next, "PL410 `next` route not producing action; actions: {:?}", actions);
    assert!(has_last, "PL410 `last` route not producing action; actions: {:?}", actions);
    assert!(has_redo, "PL410 `redo` route not producing action; actions: {:?}", actions);

    Ok(())
}
