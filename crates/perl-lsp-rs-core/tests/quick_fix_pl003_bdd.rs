//! BDD tests for PL003 (UnexpectedEof) quick-fix routing.
//!
//! PL003 is emitted when the parser reaches end-of-file without completing a
//! syntactic unit.  Before this change there was no quick-fix handler for it,
//! so the dispatch table silently dropped the code.
//!
//! After this change:
//!
//!   1. `pl003_missing_semicolon_message_adds_semicolon` — when the diagnostic
//!      message mentions "missing semicolon", the fix inserts `;` at the end of
//!      the offending line (shared arm with PL001/PL002).
//!
//!   2. `pl003_generic_eof_offers_closing_brace` — for any other PL003 message
//!      (the typical "Unexpected end of file" case), the fix appends `\n}` at
//!      the end of the source.
//!
//!   3. `pl003_dispatch_smoke_test` — verifies that PL003 reaches a handler
//!      and produces at least one action (regression guard for the routing
//!      table entry).

use perl_lsp_rs_core::providers::code_actions::{CodeAction, CodeActionKind, CodeActionsProvider};
use perl_lsp_rs_core::providers::diagnostics::{Diagnostic, DiagnosticSeverity};
use perl_parser::Parser;
use perl_tdd_support::must;

// ---------------------------------------------------------------------------
// Helpers (mirrors quick_fix_new_codes_bdd.rs)
// ---------------------------------------------------------------------------

fn make_diag(start: usize, end: usize, code: &str, message: &str) -> Diagnostic {
    Diagnostic {
        range: (start, end),
        severity: DiagnosticSeverity::Error,
        code: Some(code.to_string()),
        message: message.to_string(),
        related_information: Vec::new(),
        tags: Vec::new(),
        suggestion: None,
    }
}

fn actions_for(source: &str, diags: &[Diagnostic]) -> Vec<CodeAction> {
    let mut parser = Parser::new(source);
    // Parse regardless of errors — code action provider works on best-effort ASTs.
    let ast = parser.parse().unwrap_or_else(|_| {
        let mut p = Parser::new("");
        must(p.parse())
    });
    let provider = CodeActionsProvider::new(source.to_string());
    provider.get_code_actions(&ast, (0, source.len()), diags)
}

fn apply_edits(source: &str, action: &CodeAction) -> String {
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
// PL003 — missing-semicolon message path
// ===========================================================================

#[test]
fn pl003_missing_semicolon_message_adds_semicolon() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a source that ends without a semicolon
    let source = "my $x = 1\n";

    // WHEN a PL003 diagnostic is raised with a "missing semicolon" message
    let diag = make_diag(0, source.len(), "PL003", "Unexpected end of file - missing semicolon");
    let actions = actions_for(source, &[diag]);

    // THEN the fix offers to add a semicolon
    let action = find_action(&actions, |t| t.contains("semicolon"))
        .ok_or_else(|| format!("no semicolon action in: {actions:?}"))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred, "semicolon fix should be preferred");

    let result = apply_edits(source, action);
    assert!(result.contains(';'), "semicolon must appear in result: {result:?}");

    Ok(())
}

// ===========================================================================
// PL003 — generic unexpected-EOF path
// ===========================================================================

#[test]
fn pl003_generic_eof_offers_closing_brace() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN an unclosed subroutine
    let source = "sub greet {\n    print \"hello\";\n";

    // WHEN a PL003 diagnostic fires with the standard "Unexpected end of file" message
    let diag =
        make_diag(source.len().saturating_sub(1), source.len(), "PL003", "Unexpected end of file");
    let actions = actions_for(source, &[diag]);

    // THEN the fix offers to append a closing brace
    let action = find_action(&actions, |t| t.contains("closing brace"))
        .ok_or_else(|| format!("no closing-brace action in: {actions:?}"))?;

    assert_eq!(action.title, "Add missing closing brace '}'");
    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred);

    let result = apply_edits(source, action);
    assert!(result.ends_with("\n}"), "result must end with newline + '}}'; got: {result:?}");

    Ok(())
}

#[test]
fn pl003_closing_brace_appended_at_eof() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN source with no trailing newline
    let source = "sub foo { my $x = 1;";
    let diag =
        make_diag(source.len().saturating_sub(1), source.len(), "PL003", "Unexpected end of file");
    let actions = actions_for(source, &[diag]);

    let action = find_action(&actions, |t| t.contains("closing brace"))
        .ok_or_else(|| format!("no closing-brace action in: {actions:?}"))?;

    let result = apply_edits(source, action);
    assert_eq!(result, "sub foo { my $x = 1;\n}", "brace must be appended at EOF");

    Ok(())
}

// ===========================================================================
// Dispatch smoke test
// ===========================================================================

#[test]
fn pl003_dispatch_smoke_test() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN any PL003 diagnostic
    let source = "sub broken {\n";
    let diag =
        make_diag(source.len().saturating_sub(1), source.len(), "PL003", "Unexpected end of file");
    let actions = actions_for(source, &[diag]);

    // THEN at least one QuickFix action is produced for PL003 — confirming the
    // routing table entry exists and the handler fires.  (Other action kinds,
    // such as SourceModernize, may also be present and are not checked here.)
    let has_quick_fix = actions.iter().any(|a| a.kind == CodeActionKind::QuickFix);
    assert!(
        has_quick_fix,
        "PL003 must produce at least one QuickFix action; routing table may be missing the entry. Got: {actions:?}"
    );

    Ok(())
}

// ===========================================================================
// Comment-correctness regression guards
// ===========================================================================
//
// These tests encode the correct PL-code → DiagnosticCode mapping so that
// a future renumbering which breaks a comment will also break a test.

#[test]
fn diagnostic_code_numbers_match_enum_strings() {
    use perl_diagnostics::codes::DiagnosticCode;

    // Previously the comments in diagnostic_routes.rs had these wrong:
    //   "PL107: Duplicate parameter"  (should be PL106)
    //   "PL110: Parameter shadows..."  (should be PL107)
    assert_eq!(DiagnosticCode::DuplicateParameter.as_str(), "PL106");
    assert_eq!(DiagnosticCode::ParameterShadowsGlobal.as_str(), "PL107");
    assert_eq!(DiagnosticCode::UninitializedVariable.as_str(), "PL110");
    assert_eq!(DiagnosticCode::UnexpectedEof.as_str(), "PL003");
}
