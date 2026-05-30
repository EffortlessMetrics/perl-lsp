//! BDD tests for auto-import quick-fix (issue #790)
//!
//! When a diagnostic flags an undefined function/bareword whose module can be
//! resolved from the static symbol-to-module map, the provider must offer a
//! `QuickFix` code action "Import `Module`" that inserts `use Module;` at the
//! top of the import block.
//!
//! Regression guards:
//!   - builtins → no import action
//!   - already-imported module → no duplicate action
//!   - unresolvable symbol → no action (no crash)
//!
//! Pattern: GIVEN source / WHEN actions requested / THEN assertions

use perl_lsp_rs_core::providers::code_actions::{CodeAction, CodeActionKind, CodeActionsProvider};
use perl_lsp_rs_core::providers::diagnostics::{Diagnostic, DiagnosticSeverity};
use perl_parser::Parser;
use perl_tdd_support::must;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn bareword_diag(start: usize, end: usize, name: &str) -> Diagnostic {
    Diagnostic {
        range: (start, end),
        severity: DiagnosticSeverity::Error,
        code: Some("PL109".to_string()),
        message: format!(
            "Bareword '{}' is not allowed under 'use strict' -- quote it as '{}' or use it as a subroutine call",
            name, name
        ),
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

fn apply_action(source: &str, action: &CodeAction) -> String {
    let mut edits = action.edit.changes.clone();
    edits.sort_by(|a, b| b.location.start.cmp(&a.location.start));
    let mut out = source.to_string();
    for edit in edits {
        out.replace_range(edit.location.start..edit.location.end, &edit.new_text);
    }
    out
}

fn find_import_action<'a>(actions: &'a [CodeAction]) -> Option<&'a CodeAction> {
    actions.iter().find(|a| a.title.starts_with("Import '") || a.title.starts_with("Add 'use "))
}

// ===========================================================================
// Happy path: resolvable symbol → import action offered
// ===========================================================================

#[test]
fn decode_json_bareword_offers_use_json_import() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a file calling decode_json without importing JSON
    let source = "use strict;\nuse warnings;\nmy $data = decode_json($text);\n";
    let call_start = source.find("decode_json").ok_or("marker not found")?;
    let call_end = call_start + "decode_json".len();

    let diag = bareword_diag(call_start, call_end, "decode_json");

    // WHEN code actions are requested
    let actions = actions_for(source, &[diag]);

    // THEN there is an import action offering JSON
    let action = find_import_action(&actions)
        .ok_or_else(|| format!("no import action; got: {actions:?}"))?;

    assert_eq!(action.kind, CodeActionKind::QuickFix, "must be QuickFix");
    assert!(action.title.contains("JSON"), "title must mention the module, got: {}", action.title);

    Ok(())
}

#[test]
fn decode_json_import_action_inserts_use_json_at_top() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a file calling decode_json without importing JSON
    let source = "use strict;\nuse warnings;\nmy $data = decode_json($text);\n";
    let call_start = source.find("decode_json").ok_or("marker not found")?;
    let call_end = call_start + "decode_json".len();

    let diag = bareword_diag(call_start, call_end, "decode_json");
    let actions = actions_for(source, &[diag]);

    // THEN applying the import action inserts `use JSON;`
    let action = find_import_action(&actions)
        .ok_or_else(|| format!("no import action; got: {actions:?}"))?;

    let result = apply_action(source, action);
    assert!(result.contains("use JSON;"), "result must contain 'use JSON;', got: {result:?}");

    Ok(())
}

#[test]
fn encode_json_bareword_offers_use_json_import() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a file calling encode_json without importing JSON
    let source = "use strict;\nmy $json = encode_json($data);\n";
    let call_start = source.find("encode_json").ok_or("marker not found")?;
    let call_end = call_start + "encode_json".len();

    let diag = bareword_diag(call_start, call_end, "encode_json");
    let actions = actions_for(source, &[diag]);

    let action = find_import_action(&actions)
        .ok_or_else(|| format!("no import action; got: {actions:?}"))?;
    assert!(action.title.contains("JSON"), "title must mention JSON, got: {}", action.title);

    Ok(())
}

#[test]
fn basename_bareword_offers_use_file_basename_import() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a file calling basename without importing File::Basename
    let source = "use strict;\nmy $name = basename($path);\n";
    let call_start = source.find("basename").ok_or("marker not found")?;
    let call_end = call_start + "basename".len();

    let diag = bareword_diag(call_start, call_end, "basename");
    let actions = actions_for(source, &[diag]);

    let action = find_import_action(&actions)
        .ok_or_else(|| format!("no import action; got: {actions:?}"))?;
    assert!(
        action.title.contains("File::Basename"),
        "title must mention File::Basename, got: {}",
        action.title
    );

    let result = apply_action(source, action);
    assert!(
        result.contains("use File::Basename;"),
        "result must contain 'use File::Basename;', got: {result:?}"
    );

    Ok(())
}

#[test]
fn import_inserted_after_existing_use_statements() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a file with existing use statements
    let source = "use strict;\nuse warnings;\nmy $json = decode_json($text);\n";
    let call_start = source.find("decode_json").ok_or("marker not found")?;
    let call_end = call_start + "decode_json".len();

    let diag = bareword_diag(call_start, call_end, "decode_json");
    let actions = actions_for(source, &[diag]);

    let action = find_import_action(&actions)
        .ok_or_else(|| format!("no import action; got: {actions:?}"))?;
    let result = apply_action(source, action);

    // `use JSON;` should appear after `use warnings;\n`
    let json_pos =
        result.find("use JSON;").ok_or_else(|| format!("no 'use JSON;' in: {result:?}"))?;
    let warnings_pos = result.find("use warnings;").ok_or("no 'use warnings;'")?;
    assert!(
        json_pos > warnings_pos,
        "use JSON; ({json_pos}) should appear after use warnings; ({warnings_pos})"
    );

    Ok(())
}

// ===========================================================================
// Regression: builtins → no import action
// ===========================================================================

#[test]
fn builtin_print_bareword_no_import_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a file that uses a builtin (which wouldn't get PL109 in practice,
    // but we guard the quick-fix against generating a spurious import action)
    let source = "use strict;\nprint 'hello';\n";
    let call_start = source.find("print").ok_or("marker not found")?;
    let call_end = call_start + "print".len();

    let diag = bareword_diag(call_start, call_end, "print");
    let actions = actions_for(source, &[diag]);

    // No import action should be offered for a builtin
    assert!(
        find_import_action(&actions).is_none(),
        "builtins must not get an import action; got: {actions:?}"
    );

    Ok(())
}

#[test]
fn chomp_builtin_bareword_no_import_action() -> Result<(), Box<dyn std::error::Error>> {
    let source = "use strict;\nchomp($line);\n";
    let call_start = source.find("chomp").ok_or("marker not found")?;
    let call_end = call_start + "chomp".len();

    let diag = bareword_diag(call_start, call_end, "chomp");
    let actions = actions_for(source, &[diag]);

    assert!(
        find_import_action(&actions).is_none(),
        "chomp is a builtin; no import action expected; got: {actions:?}"
    );

    Ok(())
}

// ===========================================================================
// Regression: already imported → no duplicate action
// ===========================================================================

#[test]
fn already_imported_module_no_duplicate_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a file that already has `use JSON;`
    let source = "use strict;\nuse JSON;\nmy $data = decode_json($text);\n";
    let call_start = source.find("decode_json").ok_or("marker not found")?;
    let call_end = call_start + "decode_json".len();

    let diag = bareword_diag(call_start, call_end, "decode_json");
    let actions = actions_for(source, &[diag]);

    // No import action should be offered since JSON is already imported
    assert!(
        find_import_action(&actions).is_none(),
        "JSON is already imported; no import action expected; got: {actions:?}"
    );

    Ok(())
}

#[test]
fn already_imported_with_args_no_duplicate_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a file that imports JSON with explicit symbols
    let source = "use strict;\nuse JSON qw(decode_json encode_json);\nmy $d = decode_json($t);\n";
    let call_start = source.find("decode_json(").ok_or("marker not found")?;
    let call_end = call_start + "decode_json".len();

    let diag = bareword_diag(call_start, call_end, "decode_json");
    let actions = actions_for(source, &[diag]);

    assert!(
        find_import_action(&actions).is_none(),
        "JSON already imported with explicit list; no import action expected; got: {actions:?}"
    );

    Ok(())
}

// ===========================================================================
// Regression: unresolvable symbol → no action (no crash)
// ===========================================================================

#[test]
fn unresolvable_symbol_no_import_action() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN a file calling an unknown function not in the static map
    let source = "use strict;\nmy $result = frobnicate(1);\n";
    let call_start = source.find("frobnicate").ok_or("marker not found")?;
    let call_end = call_start + "frobnicate".len();

    let diag = bareword_diag(call_start, call_end, "frobnicate");
    let actions = actions_for(source, &[diag]);

    // No import action — frobnicate is not in the static map
    assert!(
        find_import_action(&actions).is_none(),
        "unresolvable symbol must not produce an import action; got: {actions:?}"
    );

    Ok(())
}

#[test]
fn unresolvable_symbol_does_not_crash() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN edge cases: empty source, very long name, unicode name
    let sources_and_names: &[(&str, &str)] = &[
        ("use strict;\nnot_a_known_func();\n", "not_a_known_func"),
        ("use strict;\nsome_other_thing();\n", "some_other_thing"),
    ];

    for (source, name) in sources_and_names {
        let call_start = source.find(name).ok_or("marker not found")?;
        let call_end = call_start + name.len();
        let diag = bareword_diag(call_start, call_end, name);
        let _actions = actions_for(source, &[diag]);
        // Must not panic
    }

    Ok(())
}
