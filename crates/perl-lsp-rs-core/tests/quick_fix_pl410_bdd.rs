//! BDD tests for the PL410 quick-fix handler.
//!
//!   PL410 - `next`/`last`/`redo LABEL` references a label not defined in the file.
//!   Fix: drop the label so the statement targets the innermost enclosing loop.
//!
//! Each scenario follows the pattern:
//!   GIVEN  source containing a loop-control statement with an undefined label
//!   WHEN   a PL410 diagnostic is produced and code actions are requested
//!   THEN   exactly one preferred action is returned that strips the label

use perl_lsp_rs_core::providers::code_actions::{CodeAction, CodeActionKind, CodeActionsProvider};
use perl_lsp_rs_core::providers::diagnostics::{Diagnostic, DiagnosticSeverity};
use perl_parser::Parser;
use perl_tdd_support::must;

// ---------------------------------------------------------------------------
// Helpers (mirror the pattern used in quick_fix_new_codes_bdd.rs)
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

/// Apply every edit from an action (largest offset first) and return the result.
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
fn next_label_produces_drop_label_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a `next OUTER` where OUTER is not defined
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

    // THEN there is a preferred action offering to drop the label
    let action = find_action(&actions, |t| t.contains("next"))
        .ok_or_else(|| format!("no 'next' action in: {actions:?}"))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred, "drop-label action must be preferred");

    Ok(())
}

#[test]
fn next_label_edit_leaves_bare_next() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a `next MISSING` in a for-loop
    let source = "for my $i (1..10) { next MISSING; }\n";
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
        .ok_or_else(|| format!("no 'next' action in: {actions:?}"))?;
    let result = edited(source, action);

    // THEN the label is removed; `next` becomes bare
    assert!(result.contains("next;"), "expected bare 'next;' after edit, got: {result:?}");
    assert!(!result.contains("MISSING"), "label name should be gone after edit");

    Ok(())
}

// ===========================================================================
// PL410 — last LABEL
// ===========================================================================

#[test]
fn last_label_edit_leaves_bare_last() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a `last PHANTOM` in a while-loop
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
    let result = edited(source, action);

    assert!(result.contains("last;"), "expected bare 'last;', got: {result:?}");
    assert!(!result.contains("PHANTOM"), "label should be gone");

    Ok(())
}

// ===========================================================================
// PL410 — redo LABEL
// ===========================================================================

#[test]
fn redo_label_edit_leaves_bare_redo() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a `redo NOWHERE` inside a loop body
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
    let result = edited(source, action);

    assert!(result.contains("redo;"), "expected bare 'redo;', got: {result:?}");
    assert!(!result.contains("NOWHERE"), "label should be gone");

    Ok(())
}

// ===========================================================================
// PL410 — title naming
// ===========================================================================

#[test]
fn action_title_names_the_operator() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a PL410 diagnostic for `last GHOST`
    let source = "while (1) { last GHOST; }\n";
    let stmt_start = source.find("last GHOST").ok_or("marker not found")?;
    let stmt_end = stmt_start + "last GHOST".len();

    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`last GHOST` references a label that is not defined in this file",
    );
    let actions = actions_for(source, &[diag]);

    // THEN the action title mentions 'last' so the user knows which operator is fixed
    let action = find_action(&actions, |t| t.contains("last"))
        .ok_or_else(|| format!("no 'last' action in: {actions:?}"))?;
    assert!(
        action.title.contains("last"),
        "title should name the operator, got: {:?}",
        action.title
    );

    Ok(())
}

// ===========================================================================
// PL410 — invalid-range guard
// ===========================================================================

#[test]
fn invalid_range_produces_no_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a diagnostic whose byte range points past the end of the source
    let source = "for my $i (1..3) { next OUTER; }\n";
    let out_of_bounds_start = source.len() + 1;
    let out_of_bounds_end = source.len() + 10;

    let diag = make_diag(
        out_of_bounds_start,
        out_of_bounds_end,
        "PL410",
        "`next OUTER` references a label that is not defined in this file",
    );
    let actions = actions_for(source, &[diag]);

    // THEN no action is produced — invalid ranges must be silently ignored
    assert!(
        !actions.iter().any(|a| a.title.contains("next")
            || a.title.contains("last")
            || a.title.contains("redo")),
        "expected no PL410 action for out-of-bounds range, got: {actions:?}"
    );

    Ok(())
}

// ===========================================================================
// PL410 — dispatch smoke test
// ===========================================================================

#[test]
fn pl410_dispatch_reaches_handler() -> Result<(), Box<dyn std::error::Error>> {
    // Smoke test: the routing table dispatches PL410 to the handler.
    // Mix PL410 with a known-good PL700 diagnostic; both must produce actions.
    let source = "use Foo;\nfor my $i (1..3) { next GHOST; }\n";
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CodeActionsProvider::new(source.to_string());

    let foo_start = source.find("use Foo;").ok_or("use Foo not found")?;
    let next_start = source.find("next GHOST").ok_or("next GHOST not found")?;
    let next_end = next_start + "next GHOST".len();

    let diags = vec![
        make_diag(
            foo_start,
            foo_start + "use Foo;".len(),
            "PL700",
            "Module 'Foo' appears to be unused",
        ),
        make_diag(
            next_start,
            next_end,
            "PL410",
            "`next GHOST` references a label that is not defined in this file",
        ),
    ];

    let actions = provider.get_code_actions(&ast, (0, source.len()), &diags);

    let has_pl700 = actions.iter().any(|a| a.title.contains("use Foo"));
    let has_pl410 = actions.iter().any(|a| a.title.contains("next"));

    assert!(has_pl700, "PL700 route must produce an action; got: {actions:?}");
    assert!(has_pl410, "PL410 route must produce an action; got: {actions:?}");

    Ok(())
}
