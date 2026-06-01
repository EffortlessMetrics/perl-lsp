//! BDD tests for `fix_loop_control_undefined_label` (PL410).
//!
//! PL410 fires when `next`, `last`, or `redo` names a label that does not exist
//! anywhere in the current file — a runtime-fatal condition in Perl.  The only
//! sensible automated fix is to drop the label token so the statement targets
//! the innermost enclosing loop instead.
//!
//! Scenarios:
//!   1. `next LABEL`  — action is offered and drops the label
//!   2. `last LABEL`  — action is offered and drops the label
//!   3. `redo LABEL`  — action is offered and drops the label
//!   4. Title naming  — exact title includes the operator keyword
//!   5. Invalid-range guard — out-of-bounds range returns no actions
//!   6. Dispatch smoke test — routing through CodeActionsProvider works end-to-end

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

fn find_action<'a>(
    actions: &'a [CodeAction],
    pred: impl Fn(&str) -> bool,
) -> Option<&'a CodeAction> {
    actions.iter().find(|a| pred(&a.title))
}

/// Apply all edits in an action and return the resulting source string.
fn apply(source: &str, action: &CodeAction) -> String {
    let mut edits = action.edit.changes.clone();
    edits.sort_by(|a, b| b.location.start.cmp(&a.location.start));
    let mut out = source.to_string();
    for edit in edits {
        out.replace_range(edit.location.start..edit.location.end, &edit.new_text);
    }
    out
}

// ===========================================================================
// Scenario 1: next LABEL — fix drops the label
// ===========================================================================

#[test]
fn next_undefined_label_fix_drops_label() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a source where `next OUTER` references an undefined label
    let source = "for my $i (1..10) { next OUTER; }";

    let stmt = "next OUTER";
    let start = source.find(stmt).ok_or("marker not found")?;
    let end = start + stmt.len();

    let diag = make_diag(
        start,
        end,
        "PL410",
        "`next OUTER` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN the fix replaces `next OUTER` with `next`
    let action = find_action(&actions, |t| t.contains("next"))
        .ok_or_else(|| format!("no PL410 action in: {:?}", actions))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred, "PL410 fix should be preferred");

    let result = apply(source, action);
    assert_eq!(result, "for my $i (1..10) { next; }");

    Ok(())
}

// ===========================================================================
// Scenario 2: last LABEL — fix drops the label
// ===========================================================================

#[test]
fn last_undefined_label_fix_drops_label() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a source where `last LOOP` references an undefined label
    let source = "while (1) { last LOOP; }";

    let stmt = "last LOOP";
    let start = source.find(stmt).ok_or("marker not found")?;
    let end = start + stmt.len();

    let diag = make_diag(
        start,
        end,
        "PL410",
        "`last LOOP` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN the fix replaces `last LOOP` with `last`
    let action = find_action(&actions, |t| t.contains("last"))
        .ok_or_else(|| format!("no PL410 action in: {:?}", actions))?;

    assert!(action.is_preferred);
    let result = apply(source, action);
    assert_eq!(result, "while (1) { last; }");

    Ok(())
}

// ===========================================================================
// Scenario 3: redo LABEL — fix drops the label
// ===========================================================================

#[test]
fn redo_undefined_label_fix_drops_label() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a source where `redo ITER` references an undefined label
    let source = "for my $i (1..5) { redo ITER; }";

    let stmt = "redo ITER";
    let start = source.find(stmt).ok_or("marker not found")?;
    let end = start + stmt.len();

    let diag = make_diag(
        start,
        end,
        "PL410",
        "`redo ITER` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN the fix replaces `redo ITER` with `redo`
    let action = find_action(&actions, |t| t.contains("redo"))
        .ok_or_else(|| format!("no PL410 action in: {:?}", actions))?;

    assert!(action.is_preferred);
    let result = apply(source, action);
    assert_eq!(result, "for my $i (1..5) { redo; }");

    Ok(())
}

// ===========================================================================
// Scenario 4: Title naming — title contains the operator keyword
// ===========================================================================

#[test]
fn action_title_includes_operator_keyword() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a PL410 diagnostic for `next GHOST`
    let source = "for my $x (1..3) { next GHOST; }";

    let stmt = "next GHOST";
    let start = source.find(stmt).ok_or("marker not found")?;
    let end = start + stmt.len();

    let diag = make_diag(
        start,
        end,
        "PL410",
        "`next GHOST` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN the title mentions the operator
    let action = actions.first().ok_or_else(|| format!("no actions: {:?}", actions))?;
    assert!(
        action.title.contains("next"),
        "title should mention the operator 'next': {}",
        action.title
    );

    Ok(())
}

// ===========================================================================
// Scenario 5: Invalid-range guard — out-of-bounds range returns no actions
// ===========================================================================

#[test]
fn invalid_range_returns_no_actions() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a range that exceeds the source length
    let source = "for my $i (1..3) { next GHOST; }";
    let beyond = source.len() + 5;

    let diag = make_diag(
        beyond,
        beyond + 4,
        "PL410",
        "`next GHOST` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN no actions are returned — invalid range is silently rejected
    let pl410_actions: Vec<_> =
        actions.iter().filter(|a| a.diagnostics.iter().any(|d| d == "PL410")).collect();
    assert!(
        pl410_actions.is_empty(),
        "out-of-bounds range should return no PL410 actions: {:?}",
        pl410_actions
    );

    Ok(())
}

// ===========================================================================
// Scenario 6: Dispatch smoke test — routing via CodeActionsProvider
// ===========================================================================

#[test]
fn pl410_dispatch_routes_to_fix() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a minimal source with a PL410 diagnostic routed through the provider
    let source = "while (1) { next PHANTOM; }";

    let stmt = "next PHANTOM";
    let start = source.find(stmt).ok_or("marker not found")?;
    let end = start + stmt.len();

    let diag = make_diag(
        start,
        end,
        "PL410",
        "`next PHANTOM` references a label that is not defined in this file",
    );

    // WHEN code actions are requested via the provider
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CodeActionsProvider::new(source.to_string());
    let actions = provider.get_code_actions(&ast, (0, source.len()), &[diag]);

    // THEN at least one quick-fix action is returned for PL410
    let has_pl410_fix = actions
        .iter()
        .any(|a| a.kind == CodeActionKind::QuickFix && a.diagnostics.iter().any(|d| d == "PL410"));
    assert!(has_pl410_fix, "CodeActionsProvider should route PL410 to a quick-fix: {:?}", actions);

    Ok(())
}
