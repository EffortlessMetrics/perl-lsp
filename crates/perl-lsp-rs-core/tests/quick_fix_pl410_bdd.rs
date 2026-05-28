//! BDD tests for the PL410 quick-fix handler:
//!   PL410 - `next`/`last`/`redo LABEL` targets an undefined label
//!           (`fix_loop_control_undefined_label`)
//!
//! Each scenario follows the pattern:
//!   GIVEN  source code containing a loop-control statement with an undefined label
//!   WHEN   a PL410 diagnostic is raised and code actions are requested
//!   THEN   exactly one action is returned offering to drop the undefined label

use perl_lsp_rs_core::providers::code_actions::{CodeAction, CodeActionKind, CodeActionsProvider};
use perl_lsp_rs_core::providers::diagnostics::{Diagnostic, DiagnosticSeverity};
use perl_parser::Parser;
use perl_tdd_support::must;

// ---------------------------------------------------------------------------
// Helpers (mirrored from quick_fix_new_codes_bdd.rs)
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

/// Apply all edits from an action (reverse order to preserve offsets) and return the result.
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
fn next_undefined_label_produces_drop_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a loop-control `next OUTER` where OUTER is not defined
    let source = "for my $i (1..10) { next OUTER; }";
    let start = source.find("next OUTER").ok_or("marker not found")?;
    let end = start + "next OUTER".len();

    let diag = make_diag(
        start,
        end,
        "PL410",
        "`next OUTER` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN there is exactly one action offering to drop the label
    let action = find_action(&actions, |t| t.contains("OUTER"))
        .ok_or_else(|| format!("no drop-label action in: {actions:?}"))?;

    assert_eq!(action.title, "Drop undefined label 'OUTER' from 'next'");
    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred, "drop-label action should be preferred");

    Ok(())
}

// ===========================================================================
// PL410 — last LABEL
// ===========================================================================

#[test]
fn last_undefined_label_produces_drop_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a loop-control `last MISSING` where MISSING is not defined
    let source = "while (1) { last MISSING; }";
    let start = source.find("last MISSING").ok_or("marker not found")?;
    let end = start + "last MISSING".len();

    let diag = make_diag(
        start,
        end,
        "PL410",
        "`last MISSING` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN the action offers to drop the label from `last`
    let action = find_action(&actions, |t| t.contains("MISSING"))
        .ok_or_else(|| format!("no drop-label action in: {actions:?}"))?;

    assert_eq!(action.title, "Drop undefined label 'MISSING' from 'last'");
    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred);

    // AND the edit produces `last` without the label
    let result = edited(source, action);
    assert_eq!(result, "while (1) { last; }");

    Ok(())
}

// ===========================================================================
// PL410 — redo LABEL
// ===========================================================================

#[test]
fn redo_undefined_label_produces_drop_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a loop-control `redo NOWHERE` where NOWHERE is not defined
    let source = "for my $i (1..5) { redo NOWHERE; }";
    let start = source.find("redo NOWHERE").ok_or("marker not found")?;
    let end = start + "redo NOWHERE".len();

    let diag = make_diag(
        start,
        end,
        "PL410",
        "`redo NOWHERE` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN the action offers to drop the label from `redo`
    let action = find_action(&actions, |t| t.contains("NOWHERE"))
        .ok_or_else(|| format!("no drop-label action in: {actions:?}"))?;

    assert_eq!(action.title, "Drop undefined label 'NOWHERE' from 'redo'");
    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred);

    Ok(())
}

// ===========================================================================
// PL410 — title includes both label name and operator
// ===========================================================================

#[test]
fn title_names_operator_and_label() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a `next GHOST` statement
    let source = "for my $x (1..3) { next GHOST; }";
    let start = source.find("next GHOST").ok_or("marker not found")?;
    let end = start + "next GHOST".len();

    let diag = make_diag(
        start,
        end,
        "PL410",
        "`next GHOST` references a label that is not defined in this file",
    );

    let actions = actions_for(source, &[diag]);

    let action = find_action(&actions, |t| t.contains("GHOST"))
        .ok_or_else(|| format!("no GHOST action in: {actions:?}"))?;

    // THEN the title contains both the label name and the operator
    assert!(
        action.title.contains("GHOST"),
        "title should include the label name; got: {:?}",
        action.title
    );
    assert!(
        action.title.contains("next"),
        "title should include the operator; got: {:?}",
        action.title
    );

    Ok(())
}

// ===========================================================================
// PL410 — invalid (non-char-boundary) range is rejected
// ===========================================================================

#[test]
fn non_char_boundary_range_produces_no_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN source with a multibyte character so we can manufacture an
    // invalid range that straddles a char boundary
    let source = "for my $i (1..3) { next LOOP; }\nmy $x = \"\u{e9}\";\n";
    let char_start = source.find('\u{e9}').ok_or("marker not found")?;

    // A range that starts inside the multibyte sequence is not a valid char boundary
    let diag = make_diag(
        char_start + 1,
        char_start + 2,
        "PL410",
        "`next LOOP` references a label that is not defined in this file",
    );

    let actions = actions_for(source, &[diag]);

    // THEN no drop-label action is produced
    assert!(
        !actions.iter().any(|a| a.title.contains("Drop undefined label")),
        "expected no PL410 action for non-char-boundary range, got: {actions:?}"
    );

    Ok(())
}

// ===========================================================================
// Cross-cutting: dispatch-table smoke test
// ===========================================================================

#[test]
fn pl410_dispatch_smoke_test() -> Result<(), Box<dyn std::error::Error>> {
    // Smoke test: one diagnostic for each of the three operators confirms
    // the dispatch table routes all three to the handler.
    let source = "for my $i (1..10) { next ALPHA; }\nwhile (1) { last BETA; }\nfor my $j (1..3) { redo GAMMA; }\n";

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

    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CodeActionsProvider::new(source.to_string());
    let actions = provider.get_code_actions(&ast, (0, source.len()), &diags);

    let has_next = actions.iter().any(|a| a.title.contains("ALPHA"));
    let has_last = actions.iter().any(|a| a.title.contains("BETA"));
    let has_redo = actions.iter().any(|a| a.title.contains("GAMMA"));

    assert!(has_next, "PL410 route not producing action for 'next ALPHA'; actions: {actions:?}");
    assert!(has_last, "PL410 route not producing action for 'last BETA'; actions: {actions:?}");
    assert!(has_redo, "PL410 route not producing action for 'redo GAMMA'; actions: {actions:?}");

    Ok(())
}
