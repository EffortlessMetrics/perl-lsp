//! BDD tests for the PL304 quick-fix handler: add POD documentation stub
//!
//! PL304 fires when an exported subroutine has no corresponding `=head2` or
//! `=item` POD documentation.  The quick-fix inserts a `=head2 name` skeleton
//! immediately before the `sub` line so the developer only has to fill in the
//! description.
//!
//! Each scenario follows the pattern:
//!   GIVEN  source with an exported sub that lacks POD documentation
//!   WHEN   a PL304 diagnostic is produced and code actions are requested
//!   THEN   the expected action(s) are returned with correct edits

use perl_lsp_rs_core::providers::code_actions::{CodeAction, CodeActionKind, CodeActionsProvider};
use perl_lsp_rs_core::providers::diagnostics::{Diagnostic, DiagnosticSeverity};
use perl_parser::Parser;
use perl_tdd_support::must;

// ---------------------------------------------------------------------------
// Helpers  (identical structure to quick_fix_new_codes_bdd.rs)
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
// PL304 — Missing POD coverage
// ===========================================================================

#[test]
fn pl304_exported_sub_produces_add_pod_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN an exported subroutine that has no POD documentation
    let source = "package MyMod;\nuse Exporter 'import';\nour @EXPORT = qw(frobnicate);\n\nsub frobnicate { 1 }\n";

    let sub_start = source.find("sub frobnicate").ok_or("sub not found")?;
    let sub_end = source.find('}').ok_or("closing brace not found")? + 1;

    let diag = make_diag(
        sub_start,
        sub_end,
        "PL304",
        "Exported subroutine 'frobnicate' has no POD documentation",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN an action is returned that offers to add a POD stub
    let action = find_action(&actions, |t| t.contains("frobnicate"))
        .ok_or_else(|| format!("no PL304 action in: {:?}", actions))?;

    assert!(
        action.title.contains("=head2"),
        "title should reference =head2, got: {}",
        action.title
    );
    assert_eq!(action.kind, CodeActionKind::QuickFix);
    assert!(action.is_preferred, "adding POD is the only fix, so it should be preferred");

    Ok(())
}

#[test]
fn pl304_edit_inserts_pod_stub_before_sub_line() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a module with an undocumented exported sub
    let source =
        "package MyMod;\nuse Exporter 'import';\nour @EXPORT = qw(run);\n\nsub run { 42 }\n";

    let sub_start = source.find("sub run").ok_or("sub not found")?;
    let sub_end = sub_start + "sub run { 42 }".len();

    let diag = make_diag(
        sub_start,
        sub_end,
        "PL304",
        "Exported subroutine 'run' has no POD documentation",
    );
    let actions = actions_for(source, &[diag]);

    let action = find_action(&actions, |t| t.contains("run"))
        .ok_or_else(|| format!("no action in: {:?}", actions))?;

    // WHEN the edit is applied
    let result = edited(source, action);

    // THEN =head2 appears on the line immediately before `sub run`
    let pod_pos = result.find("=head2 run").ok_or("=head2 run not found in result")?;
    let sub_pos = result.find("sub run").ok_or("sub run not found in result")?;
    assert!(pod_pos < sub_pos, "=head2 should appear before sub run");

    // AND the stub ends with =cut before the sub
    let between = &result[pod_pos..sub_pos];
    assert!(between.contains("=cut"), "stub should end with =cut before the sub");

    Ok(())
}

#[test]
fn pl304_pod_stub_format_is_correct() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a minimal source with an exported sub
    let source = "package M;\nuse Exporter;\nour @EXPORT = qw(greet);\nsub greet { 'hello' }\n";

    let sub_start = source.find("sub greet").ok_or("sub not found")?;
    let sub_end = source.rfind('}').ok_or("} not found")? + 1;

    let diag = make_diag(
        sub_start,
        sub_end,
        "PL304",
        "Exported subroutine 'greet' has no POD documentation",
    );
    let actions = actions_for(source, &[diag]);

    let action = find_action(&actions, |t| t.contains("greet"))
        .ok_or_else(|| format!("no action in: {:?}", actions))?;
    let result = edited(source, action);

    // THEN the stub has the exact expected format
    assert!(
        result.contains("=head2 greet\n\nDescription.\n\n=cut\n"),
        "stub format wrong; result:\n{result}"
    );

    Ok(())
}

#[test]
fn pl304_sub_name_appears_in_action_title() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a diagnostic for a sub named process_data
    let source = "package Proc;\nuse Exporter 'import';\nour @EXPORT = qw(process_data);\nsub process_data { }\n";

    let sub_start = source.find("sub process_data").ok_or("sub not found")?;
    let sub_end = source.rfind('}').ok_or("} not found")? + 1;

    let diag = make_diag(
        sub_start,
        sub_end,
        "PL304",
        "Exported subroutine 'process_data' has no POD documentation",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN the action title contains the exact sub name
    let action = find_action(&actions, |t| t.contains("process_data"))
        .ok_or_else(|| format!("no action in: {:?}", actions))?;

    assert_eq!(action.title, "Add '=head2 process_data' POD documentation stub");

    Ok(())
}

#[test]
fn pl304_invalid_range_returns_no_actions() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a diagnostic with an out-of-bounds byte range
    let source = "sub foo { }\n";
    let oob_start = source.len() + 10;
    let oob_end = oob_start + 5;

    let diag = make_diag(
        oob_start,
        oob_end,
        "PL304",
        "Exported subroutine 'foo' has no POD documentation",
    );

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN no actions are returned (range guard fires)
    let pl304_actions: Vec<_> = actions.iter().filter(|a| a.title.contains("=head2")).collect();
    assert!(
        pl304_actions.is_empty(),
        "OOB range should produce no PL304 actions, got: {:?}",
        pl304_actions
    );

    Ok(())
}

#[test]
fn pl304_dispatch_smoke_test_reaches_handler() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a valid PL304 diagnostic delivered through the full provider pipeline
    let source =
        "package Smoke;\nuse Exporter 'import';\nour @EXPORT = qw(smoke_fn);\nsub smoke_fn { 1 }\n";

    let sub_start = source.find("sub smoke_fn").ok_or("sub not found")?;
    let sub_end = source.rfind('}').ok_or("} not found")? + 1;

    let diag = make_diag(
        sub_start,
        sub_end,
        "PL304",
        "Exported subroutine 'smoke_fn' has no POD documentation",
    );

    // WHEN code actions are collected via the full provider
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CodeActionsProvider::new(source.to_string());
    let actions = provider.get_code_actions(&ast, (sub_start, sub_end), &[diag]);

    // THEN at least one QuickFix action is returned — the routing table wires up the handler
    let quick_fix_actions: Vec<_> =
        actions.iter().filter(|a| a.kind == CodeActionKind::QuickFix).collect();
    assert!(
        !quick_fix_actions.is_empty(),
        "PL304 dispatch must reach fix_missing_pod_coverage; got actions: {:?}",
        actions
    );

    Ok(())
}
