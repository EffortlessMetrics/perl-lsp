//! BDD tests for the PL410 quick-fix handler (`fix_loop_control_undefined_label`).
//!
//! PL410 fires when `next LABEL`, `last LABEL`, or `redo LABEL` references a
//! label that is not defined in any enclosing loop. The only safe mechanical fix
//! is to drop the label so the statement targets the innermost enclosing loop.
//!
//! Each scenario follows the pattern:
//!   GIVEN  source containing a loop-control statement with an undefined label
//!   WHEN   a PL410 diagnostic is produced and code actions are requested
//!   THEN   exactly one preferred action is returned that replaces the statement
//!          with the bare operator

use perl_lsp_rs_core::providers::code_actions::{CodeAction, CodeActionKind, CodeActionsProvider};
use perl_lsp_rs_core::providers::diagnostics::{Diagnostic, DiagnosticSeverity};
use perl_parser::Parser;
use perl_tdd_support::must;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_diag(start: usize, end: usize, message: &str) -> Diagnostic {
    Diagnostic {
        range: (start, end),
        severity: DiagnosticSeverity::Error,
        code: Some("PL410".to_string()),
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

// ===========================================================================
// Scenario 1: `next LABEL` — drop label, leaving bare `next`
// ===========================================================================

#[test]
fn next_undefined_label_produces_drop_label_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a loop body that uses `next OUTER` where OUTER is not defined
    let source = "for my $i (1..10) { next OUTER; }\n";
    let stmt_start = source.find("next OUTER").ok_or("marker not found")?;
    let stmt_end = stmt_start + "next OUTER".len();

    let diag = make_diag(stmt_start, stmt_end, "Label 'OUTER' is not defined");

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN there is exactly one preferred action that drops the label
    let action = actions
        .iter()
        .find(|a| a.title.contains("next"))
        .ok_or_else(|| format!("no 'next' action in: {:?}", actions))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred, "drop-label action should be preferred");

    let result = edited(source, action);
    assert!(
        result.contains("next;") || result.contains("next "),
        "expected bare 'next', got: {result:?}"
    );
    assert!(!result.contains("next OUTER"), "label should be removed, got: {result:?}");

    Ok(())
}

// ===========================================================================
// Scenario 2: `last LABEL` — drop label, leaving bare `last`
// ===========================================================================

#[test]
fn last_undefined_label_produces_drop_label_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a loop that uses `last LOOP` where LOOP is undefined
    let source = "while (1) { last LOOP; }\n";
    let stmt_start = source.find("last LOOP").ok_or("marker not found")?;
    let stmt_end = stmt_start + "last LOOP".len();

    let diag = make_diag(stmt_start, stmt_end, "Label 'LOOP' is not defined");

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN there is an action that removes the label from `last`
    let action = actions
        .iter()
        .find(|a| a.title.contains("last"))
        .ok_or_else(|| format!("no 'last' action in: {:?}", actions))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred);

    let result = edited(source, action);
    assert!(!result.contains("last LOOP"), "label should be removed, got: {result:?}");
    assert!(result.contains("last"), "bare 'last' should remain, got: {result:?}");

    Ok(())
}

// ===========================================================================
// Scenario 3: `redo LABEL` — drop label, leaving bare `redo`
// ===========================================================================

#[test]
fn redo_undefined_label_produces_drop_label_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a loop that uses `redo RETRY` where RETRY is undefined
    let source = "for my $n (1..5) { redo RETRY; }\n";
    let stmt_start = source.find("redo RETRY").ok_or("marker not found")?;
    let stmt_end = stmt_start + "redo RETRY".len();

    let diag = make_diag(stmt_start, stmt_end, "Label 'RETRY' is not defined");

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN there is an action that removes the label from `redo`
    let action = actions
        .iter()
        .find(|a| a.title.contains("redo"))
        .ok_or_else(|| format!("no 'redo' action in: {:?}", actions))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred);

    let result = edited(source, action);
    assert!(!result.contains("redo RETRY"), "label should be removed, got: {result:?}");
    assert!(result.contains("redo"), "bare 'redo' should remain, got: {result:?}");

    Ok(())
}

// ===========================================================================
// Scenario 4: Title naming — action title includes the operator name
// ===========================================================================

#[test]
fn action_title_names_the_operator() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a `next MISSING` diagnostic
    let source = "for (1..3) { next MISSING; }\n";
    let stmt_start = source.find("next MISSING").ok_or("marker not found")?;
    let stmt_end = stmt_start + "next MISSING".len();

    let diag = make_diag(stmt_start, stmt_end, "Label 'MISSING' is not defined");
    let actions = actions_for(source, &[diag]);

    let action = actions
        .iter()
        .find(|a| a.title.contains("next"))
        .ok_or_else(|| format!("no 'next' action in: {:?}", actions))?;

    // THEN the title explicitly mentions removing the label and names the operator
    assert!(
        action.title.contains("next"),
        "title should name the operator, got: {:?}",
        action.title
    );
    assert!(
        action.title.to_lowercase().contains("label") || action.title.contains("without"),
        "title should mention label removal, got: {:?}",
        action.title
    );

    Ok(())
}

// ===========================================================================
// Scenario 5: Invalid-range guard — out-of-bounds range returns no actions
// ===========================================================================

#[test]
fn invalid_range_returns_no_actions() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a diagnostic whose range extends past the end of the source
    let source = "for (1..3) { next OUTER; }\n";
    let beyond_end = source.len() + 5;

    let diag = make_diag(beyond_end, beyond_end + 10, "Label 'OUTER' is not defined");
    let actions = actions_for(source, &[diag]);

    // THEN no PL410 action is produced
    assert!(
        !actions.iter().any(|a| a.title.contains("without label")),
        "expected no action for out-of-bounds range, got: {:?}",
        actions
    );

    Ok(())
}

// ===========================================================================
// Scenario 6: Dispatch smoke test — PL410 route reaches the handler
// ===========================================================================

#[test]
fn pl410_dispatch_smoke_test() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a source with a `last UNDEFINED` in a loop
    let source = "while (1) { last UNDEFINED; }\n";
    let stmt_start = source.find("last UNDEFINED").ok_or("marker not found")?;
    let stmt_end = stmt_start + "last UNDEFINED".len();

    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CodeActionsProvider::new(source.to_string());

    let diags = vec![make_diag(stmt_start, stmt_end, "Label 'UNDEFINED' is not defined")];

    // WHEN code actions are requested via the provider's dispatch table
    let actions = provider.get_code_actions(&ast, (0, source.len()), &diags);

    // THEN at least one action is produced confirming the PL410 route is wired
    assert!(
        actions.iter().any(|a| a.title.contains("last")),
        "PL410 route not producing action; actions: {:?}",
        actions
    );

    Ok(())
}
