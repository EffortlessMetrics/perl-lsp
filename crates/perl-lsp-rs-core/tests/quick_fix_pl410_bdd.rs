//! BDD tests for the PL410 quick-fix handler: `fix_loop_control_undefined_label`.
//!
//! `PL410` fires when a `next LABEL`, `last LABEL`, or `redo LABEL` statement
//! references a label that is not defined anywhere in the current file.  The
//! quick fix drops the label so the statement targets the innermost enclosing
//! loop, which is always semantically valid.
//!
//! Each scenario follows the pattern:
//!   GIVEN  source containing a loop-control statement with an undefined label
//!   WHEN   a PL410 diagnostic is supplied and code actions are requested
//!   THEN   exactly one action is returned, dropping the label while leaving
//!          the operator and surrounding code intact

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
// PL410 — next LABEL with undefined label
// ===========================================================================

#[test]
fn next_undefined_label_produces_drop_label_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a loop that uses `next OUTER` where OUTER is not defined
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

    // THEN there is exactly one action and it removes the label
    let action = find_action(&actions, |t| t.contains("OUTER"))
        .ok_or_else(|| format!("no drop-label action in: {:?}", actions))?;

    assert_eq!(action.title, "Remove undefined label 'OUTER': target innermost enclosing loop");
    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred, "the drop-label action should be preferred");

    let result = edited(source, action);
    assert_eq!(result, "for my $i (1..10) { next; }\n");

    Ok(())
}

// ===========================================================================
// PL410 — last LABEL with undefined label
// ===========================================================================

#[test]
fn last_undefined_label_produces_drop_label_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a loop that uses `last MISSING` where MISSING is not defined
    let source = "while (1) { last MISSING; }\n";
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

    // THEN the fix drops the label, leaving bare `last`
    let action = find_action(&actions, |t| t.contains("MISSING"))
        .ok_or_else(|| format!("no drop-label action in: {:?}", actions))?;

    assert_eq!(action.title, "Remove undefined label 'MISSING': target innermost enclosing loop");
    assert!(action.is_preferred);

    let result = edited(source, action);
    assert_eq!(result, "while (1) { last; }\n");

    Ok(())
}

// ===========================================================================
// PL410 — redo LABEL with undefined label
// ===========================================================================

#[test]
fn redo_undefined_label_produces_drop_label_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a loop that uses `redo NOWHERE` where NOWHERE is not defined
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

    // THEN the fix drops the label, leaving bare `redo`
    let action = find_action(&actions, |t| t.contains("NOWHERE"))
        .ok_or_else(|| format!("no drop-label action in: {:?}", actions))?;

    assert_eq!(action.title, "Remove undefined label 'NOWHERE': target innermost enclosing loop");

    let result = edited(source, action);
    assert_eq!(result, "for my $i (1..5) { redo; }\n");

    Ok(())
}

// ===========================================================================
// PL410 — title uses label name from message
// ===========================================================================

#[test]
fn action_title_includes_label_name_from_message() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a diagnostic whose message names a specific label
    let source = "while (1) { next DEEPLY_NESTED_LABEL; }\n";
    let stmt_start = source.find("next DEEPLY_NESTED_LABEL").ok_or("marker not found")?;
    let stmt_end = stmt_start + "next DEEPLY_NESTED_LABEL".len();
    let diag = make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`next DEEPLY_NESTED_LABEL` references a label that is not defined in this file",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN the action title contains the label name extracted from the message
    let action = find_action(&actions, |t| t.contains("DEEPLY_NESTED_LABEL"))
        .ok_or_else(|| format!("no drop-label action in: {:?}", actions))?;

    assert_eq!(
        action.title,
        "Remove undefined label 'DEEPLY_NESTED_LABEL': target innermost enclosing loop"
    );

    Ok(())
}

// ===========================================================================
// PL410 — invalid byte-range guard
// ===========================================================================

#[test]
fn non_char_boundary_range_produces_no_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a source with a multi-byte character
    let source = "while (1) { next LOOP; }\nmy $name = \"\u{e9}\";\n";
    let char_start = source.find('\u{e9}').ok_or("marker not found")?;

    // A range that splits the multi-byte char is invalid
    let diag = make_diag(
        char_start + 1,
        char_start + 2,
        "PL410",
        "`next LOOP` references a label that is not defined in this file",
    );
    let actions = actions_for(source, &[diag]);

    // THEN no action is produced for the malformed range
    assert!(
        !actions.iter().any(|a| a.title.contains("LOOP")),
        "expected no PL410 action for non-char-boundary range, got: {actions:?}"
    );

    Ok(())
}

// ===========================================================================
// Cross-cutting: dispatch smoke test
// ===========================================================================

#[test]
fn pl410_reaches_its_handler_in_dispatch_table() -> Result<(), Box<dyn std::error::Error>> {
    // Smoke test: a well-formed PL410 diagnostic reaches the handler and
    // produces at least one action, confirming the routing table is wired.
    let source = "for my $i (1..3) { next GHOST; }\n";
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CodeActionsProvider::new(source.to_string());

    let stmt_start = source.find("next GHOST").ok_or("marker not found")?;
    let stmt_end = stmt_start + "next GHOST".len();

    let diags = vec![make_diag(
        stmt_start,
        stmt_end,
        "PL410",
        "`next GHOST` references a label that is not defined in this file",
    )];

    let actions = provider.get_code_actions(&ast, (0, source.len()), &diags);

    assert!(
        actions.iter().any(|a| a.title.contains("GHOST")),
        "PL410 route not producing action; actions: {:?}",
        actions
    );

    Ok(())
}
