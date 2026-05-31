//! BDD tests for the PL410 quick-fix handler: `fix_loop_control_undefined_label`.
//!
//! PL410 fires when `next LABEL`, `last LABEL`, or `redo LABEL` targets a
//! label that is not defined anywhere in the current file. The only safe
//! mechanical fix is to drop the label, turning the statement into its bare
//! form so it targets the innermost enclosing loop at runtime.
//!
//! Each scenario follows the pattern:
//!   GIVEN  source with a loop-control statement referencing an undefined label
//!   WHEN   a PL410 diagnostic is supplied and code actions are requested
//!   THEN   exactly one "remove label" action is returned with the correct edit

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

/// Apply all edits from an action (sorted descending by start to avoid offset shift)
/// and return the modified source.
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
// Scenario 1 — `next LABEL` with undefined label
// ===========================================================================

#[test]
fn next_undefined_label_produces_remove_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a while loop that uses `next MISSING` where MISSING is not defined
    let source = "while (1) {\n    next MISSING;\n}\n";

    let node_start = source.find("next MISSING").ok_or("marker not found")?;
    let node_end = node_start + "next MISSING".len();

    let diag = make_diag(
        node_start,
        node_end,
        "PL410",
        "`next MISSING` references a label that is not defined in this file",
    );

    // WHEN code actions are requested for the PL410 diagnostic
    let actions = actions_for(source, &[diag]);

    // THEN there is exactly one action, offering to remove the label
    let action = find_action(&actions, |t| t.contains("next"))
        .ok_or_else(|| format!("no 'next' action in: {:?}", actions))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred, "the label-removal action must be preferred");

    // AND the edit replaces the full `next MISSING` span with just `next`
    let result = edited(source, action);
    assert_eq!(result, "while (1) {\n    next;\n}\n");

    Ok(())
}

// ===========================================================================
// Scenario 2 — `last LABEL` with undefined label
// ===========================================================================

#[test]
fn last_undefined_label_produces_remove_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a for loop that uses `last NOPE` where NOPE is not defined
    let source = "for my $i (1..10) {\n    last NOPE;\n}\n";

    let node_start = source.find("last NOPE").ok_or("marker not found")?;
    let node_end = node_start + "last NOPE".len();

    let diag = make_diag(
        node_start,
        node_end,
        "PL410",
        "`last NOPE` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN there is a fix action for `last`
    let action = find_action(&actions, |t| t.contains("last"))
        .ok_or_else(|| format!("no 'last' action in: {:?}", actions))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred);

    // AND applying the edit produces bare `last`
    let result = edited(source, action);
    assert_eq!(result, "for my $i (1..10) {\n    last;\n}\n");

    Ok(())
}

// ===========================================================================
// Scenario 3 — `redo LABEL` with undefined label
// ===========================================================================

#[test]
fn redo_undefined_label_produces_remove_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a while loop that uses `redo OUTER` where OUTER is not defined
    let source = "while (1) {\n    redo OUTER;\n}\n";

    let node_start = source.find("redo OUTER").ok_or("marker not found")?;
    let node_end = node_start + "redo OUTER".len();

    let diag = make_diag(
        node_start,
        node_end,
        "PL410",
        "`redo OUTER` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN there is a fix action for `redo`
    let action = find_action(&actions, |t| t.contains("redo"))
        .ok_or_else(|| format!("no 'redo' action in: {:?}", actions))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred);

    // AND applying the edit produces bare `redo`
    let result = edited(source, action);
    assert_eq!(result, "while (1) {\n    redo;\n}\n");

    Ok(())
}

// ===========================================================================
// Scenario 4 — Title naming: exact title format
// ===========================================================================

#[test]
fn action_title_names_the_operator() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a `next GHOST` statement where GHOST is undefined
    let source = "while (1) { next GHOST; }\n";

    let node_start = source.find("next GHOST").ok_or("marker not found")?;
    let node_end = node_start + "next GHOST".len();

    let diag = make_diag(
        node_start,
        node_end,
        "PL410",
        "`next GHOST` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN the action title names the bare operator
    let action = find_action(&actions, |t| t.contains("next"))
        .ok_or_else(|| format!("no 'next' action in: {:?}", actions))?;

    assert_eq!(
        action.title,
        "Remove undefined label (use bare 'next' to target innermost loop)"
    );

    Ok(())
}

// ===========================================================================
// Scenario 5 — Invalid-range guard: out-of-bounds range → no actions
// ===========================================================================

#[test]
fn invalid_range_returns_no_actions() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN source and a PL410 diagnostic whose byte range is past the end of source
    let source = "while (1) { next FOO; }\n";
    let out_of_bounds_start = source.len() + 1;
    let out_of_bounds_end = source.len() + 10;

    let diag = make_diag(
        out_of_bounds_start,
        out_of_bounds_end,
        "PL410",
        "`next FOO` references a label that is not defined in this file",
    );

    // WHEN code actions are requested with the bad range
    let actions = actions_for(source, &[diag]);

    // THEN no PL410 action is produced — the guard prevents a panic
    let has_pl410_action = actions.iter().any(|a| {
        a.title.contains("innermost loop") || a.title.contains("next") && a.title.contains("label")
    });
    assert!(
        !has_pl410_action,
        "expected no PL410 action for out-of-bounds range, got: {:?}",
        actions
    );

    Ok(())
}

// ===========================================================================
// Scenario 6 — Dispatch smoke test: PL410 code reaches the handler
// ===========================================================================

#[test]
fn dispatch_smoke_pl410_reaches_handler() -> Result<(), Box<dyn std::error::Error>> {
    // Smoke test: confirms the PL410 code is wired into the dispatch table and
    // reaches fix_loop_control_undefined_label for each supported operator.
    let source =
        "while (1) {\n    next ALPHA;\n    last BETA;\n    redo GAMMA;\n}\n";

    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CodeActionsProvider::new(source.to_string());

    let next_start = source.find("next ALPHA").ok_or("next ALPHA not found")?;
    let last_start = source.find("last BETA").ok_or("last BETA not found")?;
    let redo_start = source.find("redo GAMMA").ok_or("redo GAMMA not found")?;

    let diags = vec![
        make_diag(
            next_start,
            next_start + "next ALPHA".len(),
            "PL410",
            "`next ALPHA` references a label that is not defined in this file",
        ),
        make_diag(
            last_start,
            last_start + "last BETA".len(),
            "PL410",
            "`last BETA` references a label that is not defined in this file",
        ),
        make_diag(
            redo_start,
            redo_start + "redo GAMMA".len(),
            "PL410",
            "`redo GAMMA` references a label that is not defined in this file",
        ),
    ];

    let actions = provider.get_code_actions(&ast, (0, source.len()), &diags);

    let has_next = actions.iter().any(|a| a.title.contains("'next'"));
    let has_last = actions.iter().any(|a| a.title.contains("'last'"));
    let has_redo = actions.iter().any(|a| a.title.contains("'redo'"));

    assert!(has_next, "PL410 next route not producing action; actions: {:?}", actions);
    assert!(has_last, "PL410 last route not producing action; actions: {:?}", actions);
    assert!(has_redo, "PL410 redo route not producing action; actions: {:?}", actions);

    Ok(())
}
