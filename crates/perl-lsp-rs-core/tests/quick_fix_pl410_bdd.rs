//! BDD tests for the PL410 quick-fix handler: drop undefined loop-control label.
//!
//! Each scenario follows the pattern:
//!   GIVEN  source containing a loop-control statement with an undefined label
//!   WHEN   a PL410 diagnostic is passed and code actions are requested
//!   THEN   exactly the expected action is returned with the correct edit

use perl_lsp_rs_core::providers::code_actions::{CodeAction, CodeActionKind, CodeActionsProvider};
use perl_lsp_rs_core::providers::diagnostics::{Diagnostic, DiagnosticSeverity};
use perl_parser::Parser;
use perl_tdd_support::must;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_pl410(start: usize, end: usize, op: &str, label: &str) -> Diagnostic {
    Diagnostic {
        range: (start, end),
        severity: DiagnosticSeverity::Warning,
        code: Some("PL410".to_string()),
        message: format!("`{op} {label}` references a label that is not defined in this file"),
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
fn next_undefined_label_produces_drop_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a loop-control statement `next OUTER` with an undefined label
    let source = "for my $i (1..10) { next OUTER; }\n";
    let stmt_start = source.find("next OUTER").ok_or("marker not found")?;
    let stmt_end = stmt_start + "next OUTER".len();

    let diag = make_pl410(stmt_start, stmt_end, "next", "OUTER");

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN there is exactly one action, offering to drop the label
    let action = find_action(&actions, |t| t.contains("OUTER"))
        .ok_or_else(|| format!("no OUTER action in: {:?}", actions))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred, "the drop-label action should be preferred");

    // AND the edit removes ' OUTER' leaving 'next;' intact
    let result = edited(source, action);
    assert!(result.contains("next;"), "expected 'next;' after label removal, got: {result:?}");
    assert!(!result.contains("OUTER"), "label 'OUTER' should be gone, got: {result:?}");

    Ok(())
}

// ===========================================================================
// PL410 — last LABEL
// ===========================================================================

#[test]
fn last_undefined_label_produces_drop_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a loop-control statement `last INNER` with an undefined label
    let source = "while (1) { last INNER; }\n";
    let stmt_start = source.find("last INNER").ok_or("marker not found")?;
    let stmt_end = stmt_start + "last INNER".len();

    let diag = make_pl410(stmt_start, stmt_end, "last", "INNER");

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN there is an action for the `last` statement
    let action = find_action(&actions, |t| t.contains("INNER"))
        .ok_or_else(|| format!("no INNER action in: {:?}", actions))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred);

    Ok(())
}

// ===========================================================================
// PL410 — redo LABEL
// ===========================================================================

#[test]
fn redo_undefined_label_produces_drop_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a loop-control statement `redo LOOP` with an undefined label
    let source = "for my $x (1..5) { redo LOOP; }\n";
    let stmt_start = source.find("redo LOOP").ok_or("marker not found")?;
    let stmt_end = stmt_start + "redo LOOP".len();

    let diag = make_pl410(stmt_start, stmt_end, "redo", "LOOP");

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN there is an action for the `redo` statement
    let action = find_action(&actions, |t| t.contains("LOOP"))
        .ok_or_else(|| format!("no LOOP action in: {:?}", actions))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred);

    Ok(())
}

// ===========================================================================
// Title contains op and label name
// ===========================================================================

#[test]
fn title_names_the_op_and_label() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a next OUTER diagnostic
    let source = "for my $i (1..3) { next OUTER; }\n";
    let stmt_start = source.find("next OUTER").ok_or("marker not found")?;
    let stmt_end = stmt_start + "next OUTER".len();

    let diag = make_pl410(stmt_start, stmt_end, "next", "OUTER");
    let actions = actions_for(source, &[diag]);

    // THEN the action title contains both the label 'OUTER' and the op 'next'
    let action = find_action(&actions, |t| t.contains("OUTER") && t.contains("next"))
        .ok_or_else(|| format!("expected title with OUTER+next, got: {:?}", actions))?;

    // Title should NOT be generic — it should mention the actual label
    assert!(
        action.title.contains("OUTER"),
        "title should name the label 'OUTER', got: {:?}",
        action.title
    );
    assert!(
        action.title.contains("next"),
        "title should name the op 'next', got: {:?}",
        action.title
    );

    Ok(())
}

// ===========================================================================
// Invalid-range guard: no action for non-char-boundary range
// ===========================================================================

#[test]
fn invalid_range_guard_returns_no_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN source with a multi-byte character; a diagnostic pointing into its
    // interior (not a char boundary) should produce no actions.
    let source = "for my $i (1..3) { next OUTER; }\nmy $x = \"\u{e9}\";\n";
    let char_start = source.find('\u{e9}').ok_or("marker not found")?;

    // Point the diagnostic at a non-char-boundary offset (middle of 2-byte char)
    let diag = make_pl410(char_start + 1, char_start + 2, "next", "OUTER");
    let actions = actions_for(source, &[diag]);

    // THEN no PL410 drop-label action is produced
    assert!(
        !actions.iter().any(|a| a.title.contains("OUTER")),
        "expected no PL410 action for non-char-boundary range, got: {:?}",
        actions
    );

    Ok(())
}

// ===========================================================================
// Dispatch smoke test: PL410 route is wired in diagnostic_routes.rs
// ===========================================================================

#[test]
fn dispatch_smoke_test_pl410_route_is_wired() -> Result<(), Box<dyn std::error::Error>> {
    // Smoke test confirming the dispatch table routes PL410 to the handler.
    let source = "for my $i (1..10) { next MISSING; }\n";
    let stmt_start = source.find("next MISSING").ok_or("marker not found")?;
    let stmt_end = stmt_start + "next MISSING".len();

    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CodeActionsProvider::new(source.to_string());

    let diags = vec![make_pl410(stmt_start, stmt_end, "next", "MISSING")];
    let actions = provider.get_code_actions(&ast, (0, source.len()), &diags);

    // THEN at least one action is produced (route is live)
    assert!(
        !actions.is_empty(),
        "PL410 dispatch route produced no actions; dispatch table may be unwired"
    );
    assert!(
        actions.iter().any(|a| a.title.contains("MISSING")),
        "expected an action mentioning 'MISSING', got: {:?}",
        actions
    );

    Ok(())
}
