//! BDD tests for the PL410 quick-fix handler:
//!   PL410 - `next`/`last`/`redo LABEL` references an undefined label
//!           (`fix_loop_control_undefined_label`)
//!
//! Each scenario follows the pattern:
//!   GIVEN  source containing a loop-control statement with an undefined label
//!   WHEN   a PL410 diagnostic is produced and code actions are requested
//!   THEN   exactly one preferred action drops the label (bare op form)

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

/// Apply edits from an action (sorted descending by start offset) to source.
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
// PL410 - next LABEL with undefined label
// ===========================================================================

#[test]
fn next_undefined_label_produces_drop_label_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a `next OUTER` statement where OUTER is not defined
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

    // THEN there is one preferred action dropping the label
    let action = find_action(&actions, |t| t.contains("next"))
        .ok_or_else(|| format!("no 'next' action in: {:?}", actions))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred, "drop-label action must be preferred");
    assert_eq!(action.edit.changes.len(), 1);
    assert_eq!(action.edit.changes[0].new_text, "next");

    Ok(())
}

#[test]
fn next_drop_label_edit_removes_label_from_source() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN source with `next OUTER` inside a nested loop
    let source = "OUTER: for my $i (1..3) { for my $j (1..3) { next GHOST; } }\n";
    let stmt_start = source.find("next GHOST").ok_or("marker not found")?;
    let stmt_end = stmt_start + "next GHOST".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`next GHOST` references a label that is not defined in this file",
    );
    let actions = actions_for(source, &[diag]);
    let action = find_action(&actions, |t| t.contains("next"))
        .ok_or_else(|| format!("no action in: {:?}", actions))?;

    // WHEN the action is applied
    let result = edited(source, action);

    // THEN the label is gone and the operator remains
    assert!(
        result.contains("next;")
            || result.contains("next ")
            || result.contains("next\n")
            || result.contains("{ next }"),
        "expected bare 'next' in result, got: {result:?}"
    );
    assert!(!result.contains("GHOST"), "label 'GHOST' should be removed; got: {result:?}");

    Ok(())
}

// ===========================================================================
// PL410 - last LABEL with undefined label
// ===========================================================================

#[test]
fn last_undefined_label_produces_drop_label_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a `last MISSING` statement where MISSING is not defined
    let source = "for my $i (1..10) { last MISSING; }\n";
    let stmt_start = source.find("last MISSING").ok_or("marker not found")?;
    let stmt_end = stmt_start + "last MISSING".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`last MISSING` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN there is one preferred action dropping the label
    let action = find_action(&actions, |t| t.contains("last"))
        .ok_or_else(|| format!("no 'last' action in: {:?}", actions))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred, "drop-label action must be preferred");
    assert_eq!(action.edit.changes[0].new_text, "last");

    Ok(())
}

// ===========================================================================
// PL410 - redo LABEL with undefined label
// ===========================================================================

#[test]
fn redo_undefined_label_produces_drop_label_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a `redo NOWHERE` statement where NOWHERE is not defined
    let source = "for my $i (1..5) { redo NOWHERE; }\n";
    let stmt_start = source.find("redo NOWHERE").ok_or("marker not found")?;
    let stmt_end = stmt_start + "redo NOWHERE".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`redo NOWHERE` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN there is one preferred action dropping the label
    let action = find_action(&actions, |t| t.contains("redo"))
        .ok_or_else(|| format!("no 'redo' action in: {:?}", actions))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred, "drop-label action must be preferred");
    assert_eq!(action.edit.changes[0].new_text, "redo");

    Ok(())
}

// ===========================================================================
// PL410 - action title names the operator
// ===========================================================================

#[test]
fn action_title_names_the_operator() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN three distinct operators each with an undefined label
    let cases: &[(&str, &str)] = &[
        ("for my $i (1..10) { next GHOST; }\n", "next GHOST"),
        ("while (1) { last PHANTOM; }\n", "last PHANTOM"),
        ("for my $i (1..5) { redo NOWHERE; }\n", "redo NOWHERE"),
    ];

    for (source, needle) in cases {
        let stmt_start = source.find(needle).ok_or("marker not found")?;
        let stmt_end = stmt_start + needle.len();
        let op = needle.split_ascii_whitespace().next().unwrap();

        let diag = make_diag(
            stmt_start,
            stmt_end,
            "PL410",
            &format!("`{needle}` references a label that is not defined in this file"),
        );
        let actions = actions_for(source, &[diag]);

        let action = find_action(&actions, |t| t.contains(op))
            .ok_or_else(|| format!("no '{op}' action for source {source:?}; got: {:?}", actions))?;

        // THEN the title mentions the operator
        assert!(
            action.title.contains(op),
            "title should mention operator '{op}'; got: {:?}",
            action.title
        );
    }

    Ok(())
}

// ===========================================================================
// PL410 - guard: invalid range returns no action
// ===========================================================================

#[test]
fn invalid_range_returns_no_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a PL410 diagnostic with an out-of-bounds byte range
    let source = "for my $i (1..10) { next OUTER; }\n";
    let beyond = source.len() + 10;

    let diag = make_diag(beyond, beyond + 5, "PL410", "`next OUTER` references undefined label");
    let actions = actions_for(source, &[diag]);

    // THEN no action is produced
    let has_pl410_action = actions
        .iter()
        .any(|a| a.title.contains("next") || a.title.contains("last") || a.title.contains("redo"));
    assert!(!has_pl410_action, "expected no PL410 action for invalid range, got: {:?}", actions);

    Ok(())
}

// ===========================================================================
// Cross-cutting: dispatch smoke test — all three operators fire
// ===========================================================================

#[test]
fn all_three_operators_reach_pl410_handler() -> Result<(), Box<dyn std::error::Error>> {
    // Smoke test: each operator fires its action when the range is correct.
    let source =
        "for my $i (1..3) { for my $j (1..3) { next GHOST; last PHANTOM; redo NOWHERE; } }\n";
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CodeActionsProvider::new(source.to_string());

    let next_start = source.find("next GHOST").ok_or("next GHOST not found")?;
    let last_start = source.find("last PHANTOM").ok_or("last PHANTOM not found")?;
    let redo_start = source.find("redo NOWHERE").ok_or("redo NOWHERE not found")?;

    let diags = vec![
        make_diag(
            next_start,
            next_start + "next GHOST".len(),
            "PL410",
            "`next GHOST` references a label that is not defined in this file",
        ),
        make_diag(
            last_start,
            last_start + "last PHANTOM".len(),
            "PL410",
            "`last PHANTOM` references a label that is not defined in this file",
        ),
        make_diag(
            redo_start,
            redo_start + "redo NOWHERE".len(),
            "PL410",
            "`redo NOWHERE` references a label that is not defined in this file",
        ),
    ];

    let actions = provider.get_code_actions(&ast, (0, source.len()), &diags);

    let has_next = actions.iter().any(|a| a.title.contains("next"));
    let has_last = actions.iter().any(|a| a.title.contains("last"));
    let has_redo = actions.iter().any(|a| a.title.contains("redo"));

    assert!(has_next, "PL410 'next' route not firing; actions: {:?}", actions);
    assert!(has_last, "PL410 'last' route not firing; actions: {:?}", actions);
    assert!(has_redo, "PL410 'redo' route not firing; actions: {:?}", actions);

    Ok(())
}
