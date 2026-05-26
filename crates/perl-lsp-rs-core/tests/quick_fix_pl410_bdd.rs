//! BDD tests for PL410 quick-fix: undefined loop-control label.
//!
//! `next LABEL`, `last LABEL`, and `redo LABEL` that reference a label not
//! defined in the file get a single quick-fix: drop the label so the statement
//! targets the innermost enclosing loop.
//!
//! Pattern for each scenario:
//!   GIVEN  source with an undefined label in a loop-control statement
//!   WHEN   a PL410 diagnostic is synthesised at the exact byte range and
//!          `CodeActionsProvider::get_code_actions` is called
//!   THEN   exactly one quick-fix action is returned, its edit replaces the
//!          labelled form with the bare operator, and the resulting source is
//!          syntactically correct

use perl_lsp_rs_core::providers::code_actions::{CodeAction, CodeActionKind, CodeActionsProvider};
use perl_lsp_rs_core::providers::diagnostics::{Diagnostic, DiagnosticSeverity};
use perl_parser::Parser;
use perl_tdd_support::must;

// ---------------------------------------------------------------------------
// Helpers (identical pattern to quick_fix_new_codes_bdd.rs)
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

/// Apply the first matching edit and return the resulting source.
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

const PL410: &str = "PL410";

// ---------------------------------------------------------------------------
// Scenario 1: `next GHOST` inside a for loop
// ---------------------------------------------------------------------------

/// GIVEN source with `next GHOST` where GHOST is not a defined label
/// WHEN  a PL410 diagnostic is raised at the `next GHOST` span
/// THEN  the quick-fix drops the label, producing `next`
#[test]
fn pl410_next_undefined_label_drops_label() {
    let source = "for my $x (1..3) { next GHOST; }";
    //                                    ^^^^^^^^^  "next GHOST" = bytes 20..29
    let start = source.find("next GHOST").expect("test source must contain 'next GHOST'");
    let end = start + "next GHOST".len();

    let diag = make_diag(
        start,
        end,
        PL410,
        "`next GHOST` references a label that is not defined in this file",
    );
    let actions = actions_for(source, &[diag]);

    let action = find_action(&actions, |t| t.contains("Remove undefined label"))
        .expect("expected a 'Remove undefined label' quick-fix");
    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred, "the label-removal fix should be is_preferred");

    let result = edited(source, action);
    assert_eq!(result, "for my $x (1..3) { next; }");
}

// ---------------------------------------------------------------------------
// Scenario 2: `last PHANTOM` inside a while loop
// ---------------------------------------------------------------------------

/// GIVEN source with `last PHANTOM` where PHANTOM is not defined
/// WHEN  a PL410 diagnostic is raised
/// THEN  the quick-fix produces `last`
#[test]
fn pl410_last_undefined_label_drops_label() {
    let source = "while (1) { last PHANTOM; }";
    let start = source.find("last PHANTOM").expect("test source must contain 'last PHANTOM'");
    let end = start + "last PHANTOM".len();

    let diag = make_diag(
        start,
        end,
        PL410,
        "`last PHANTOM` references a label that is not defined in this file",
    );
    let actions = actions_for(source, &[diag]);

    let action = find_action(&actions, |t| t.contains("Remove undefined label"))
        .expect("expected a 'Remove undefined label' quick-fix");

    let result = edited(source, action);
    assert_eq!(result, "while (1) { last; }");
}

// ---------------------------------------------------------------------------
// Scenario 3: `redo WISP` inside a loop
// ---------------------------------------------------------------------------

/// GIVEN source with `redo WISP` where WISP is not defined
/// WHEN  a PL410 diagnostic is raised
/// THEN  the quick-fix produces `redo`
#[test]
fn pl410_redo_undefined_label_drops_label() {
    let source = "for (1..5) { redo WISP; }";
    let start = source.find("redo WISP").expect("test source must contain 'redo WISP'");
    let end = start + "redo WISP".len();

    let diag = make_diag(
        start,
        end,
        PL410,
        "`redo WISP` references a label that is not defined in this file",
    );
    let actions = actions_for(source, &[diag]);

    let action = find_action(&actions, |t| t.contains("Remove undefined label"))
        .expect("expected a 'Remove undefined label' quick-fix");

    let result = edited(source, action);
    assert_eq!(result, "for (1..5) { redo; }");
}

// ---------------------------------------------------------------------------
// Scenario 4: action title names the operator
// ---------------------------------------------------------------------------

/// GIVEN a PL410 diagnostic for `last PHANTOM`
/// WHEN  code actions are requested
/// THEN  the title contains the operator name (`last`)
#[test]
fn pl410_action_title_names_operator() {
    let source = "while (1) { last PHANTOM; }";
    let start = source.find("last PHANTOM").expect("test source must contain 'last PHANTOM'");
    let end = start + "last PHANTOM".len();

    let diag = make_diag(
        start,
        end,
        PL410,
        "`last PHANTOM` references a label that is not defined in this file",
    );
    let actions = actions_for(source, &[diag]);

    let action = find_action(&actions, |t| t.contains("Remove undefined label"))
        .expect("expected quick-fix");
    assert!(
        action.title.contains("last"),
        "title should name the operator 'last'; got: {}",
        action.title
    );
}

// ---------------------------------------------------------------------------
// Scenario 5: invalid/empty range returns no actions
// ---------------------------------------------------------------------------

/// GIVEN a PL410 diagnostic with an out-of-bounds range
/// WHEN  code actions are requested
/// THEN  an empty Vec is returned (no panic)
#[test]
fn pl410_invalid_range_returns_empty() {
    let source = "while (1) { next GHOST; }";
    let bad_start = source.len() + 10;
    let bad_end = bad_start + 5;

    let diag = make_diag(
        bad_start,
        bad_end,
        PL410,
        "`next GHOST` references a label that is not defined in this file",
    );
    let actions = actions_for(source, &[diag]);
    let pl410_actions: Vec<_> =
        actions.iter().filter(|a| a.diagnostics.iter().any(|d| d == PL410)).collect();
    assert!(pl410_actions.is_empty(), "should return no PL410 actions for invalid range");
}

// ---------------------------------------------------------------------------
// Scenario 6: dispatch smoke test — routes through the full provider
// ---------------------------------------------------------------------------

/// GIVEN a PL410 diagnostic at a valid range
/// WHEN  CodeActionsProvider is called
/// THEN  at least one quick-fix action with PL410 in its diagnostics vec is returned
#[test]
fn pl410_dispatch_smoke_test() {
    let source = "for my $i (1..10) { next LOOP; }";
    let start = source.find("next LOOP").expect("test source must contain 'next LOOP'");
    let end = start + "next LOOP".len();

    let diag = make_diag(
        start,
        end,
        PL410,
        "`next LOOP` references a label that is not defined in this file",
    );
    let actions = actions_for(source, &[diag]);

    let has_pl410_action = actions.iter().any(|a| a.diagnostics.iter().any(|d| d == PL410));
    assert!(has_pl410_action, "expected at least one PL410 quick-fix action from the provider");
}
