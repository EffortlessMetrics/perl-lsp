//! Tests for quick fixes added in issue #3469
//!
//! Covers the 6 new diagnostic quick fixes:
//! - PL200: MissingPackageDeclaration — add `package main;`
//! - PL105: VariableRedeclaration     — remove duplicate `my`
//! - PL111: MisspelledPragma          — fix pragma spelling
//! - PL406: UnreachableCode           — remove unreachable statement
//! - PL300: DuplicateSubroutine       — rename second definition
//! - PL301: MissingReturn             — add explicit return

use perl_lsp_code_actions::{CodeAction, CodeActionKind, CodeActionsProvider};
use perl_lsp_diagnostics::{Diagnostic, DiagnosticSeverity};
use perl_parser_core::Parser;
use perl_tdd_support::must;

fn make_diag(start: usize, end: usize, code: &str, msg: &str) -> Diagnostic {
    Diagnostic {
        range: (start, end),
        severity: DiagnosticSeverity::Warning,
        code: Some(code.to_string()),
        message: msg.to_string(),
        related_information: Vec::new(),
        tags: Vec::new(),
        suggestion: None,
    }
}

fn parse_and_get_actions(source: &str, diagnostics: &[Diagnostic]) -> Vec<CodeAction> {
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CodeActionsProvider::new(source.to_string());
    provider.get_code_actions(&ast, (0, source.len()), diagnostics)
}

fn has_action_with_title(actions: &[CodeAction], title_fragment: &str) -> bool {
    actions.iter().any(|a| a.title.contains(title_fragment))
}

fn find_action<'a>(actions: &'a [CodeAction], title_fragment: &str) -> Option<&'a CodeAction> {
    actions.iter().find(|a| a.title.contains(title_fragment))
}

// ===========================================================================
// PL200 — MissingPackageDeclaration
// ===========================================================================

#[test]
fn missing_package_declaration_fix_inserts_package_main() {
    let src = "sub greet { \"Hello\" }\n";
    let diags = [make_diag(0, 1, "PL200", "Missing package declaration")];
    let actions = parse_and_get_actions(src, &diags);

    assert!(
        has_action_with_title(&actions, "package main"),
        "Expected action to add package main, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

#[test]
fn missing_package_declaration_fix_is_quick_fix_kind() {
    let src = "my $x = 1;\n";
    let diags = [make_diag(0, 1, "PL200", "Missing package declaration")];
    let actions = parse_and_get_actions(src, &diags);

    let action = find_action(&actions, "package main");
    assert!(action.is_some(), "Expected a 'package main' action");
    assert_eq!(action.unwrap().kind, CodeActionKind::QuickFix);
}

#[test]
fn missing_package_declaration_fix_inserts_after_shebang() {
    let src = "#!/usr/bin/env perl\nmy $x = 1;\n";
    let diags = [make_diag(0, 1, "PL200", "Missing package declaration")];
    let actions = parse_and_get_actions(src, &diags);

    let action = find_action(&actions, "package main");
    assert!(action.is_some(), "Expected a 'package main' action");

    let edit = &action.unwrap().edit.changes[0];
    assert!(
        edit.location.start > 0,
        "With shebang, insertion should be after the first line, not at position 0"
    );
    assert_eq!(edit.new_text, "package main;\n");
}

#[test]
fn missing_package_declaration_fix_inserts_at_top_without_shebang() {
    let src = "my $x = 1;\n";
    let diags = [make_diag(0, 1, "PL200", "Missing package declaration")];
    let actions = parse_and_get_actions(src, &diags);

    let action = find_action(&actions, "package main");
    assert!(action.is_some(), "Expected a 'package main' action");

    let edit = &action.unwrap().edit.changes[0];
    assert_eq!(edit.location.start, 0, "Without shebang, insertion should be at position 0");
    assert_eq!(edit.new_text, "package main;\n");
}

// ===========================================================================
// PL105 — VariableRedeclaration
// ===========================================================================

#[test]
fn variable_redeclaration_fix_removes_my_keyword() {
    let src = "my $x = 1;\nmy $x = 2;\n";
    let second_var_start = src.rfind("$x").unwrap_or(0);
    let diags = [make_diag(
        second_var_start,
        second_var_start + 2,
        "PL105",
        "Variable '$x' is declared again in the same scope -- remove the duplicate 'my'",
    )];
    let actions = parse_and_get_actions(src, &diags);

    assert!(
        has_action_with_title(&actions, "Remove duplicate"),
        "Expected action to remove duplicate my, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

#[test]
fn variable_redeclaration_fix_deletes_my_prefix() {
    let src = "my $x = 1;\nmy $x = 2;\n";
    let second_my_start = src.find("my $x = 2").unwrap_or(0);
    let second_var_start = second_my_start + 3;
    let diags = [make_diag(
        second_var_start,
        second_var_start + 2,
        "PL105",
        "Variable '$x' is declared again in the same scope -- remove the duplicate 'my'",
    )];
    let actions = parse_and_get_actions(src, &diags);

    let action = find_action(&actions, "Remove duplicate");
    assert!(action.is_some(), "Expected remove duplicate action");

    let edit = &action.unwrap().edit.changes[0];
    assert_eq!(edit.new_text, "", "Edit should delete 'my '");
    assert_eq!(edit.location.start, second_my_start, "Edit should start at the duplicate 'my'");
    assert_eq!(
        edit.location.end - edit.location.start,
        3,
        "Edit should span exactly 3 bytes ('my ')"
    );
}

#[test]
fn variable_redeclaration_fix_is_preferred() {
    let src = "my $y = 1;\nmy $y = 2;\n";
    let second_var_start = src.rfind("$y").unwrap_or(0);
    let diags = [make_diag(
        second_var_start,
        second_var_start + 2,
        "PL105",
        "Variable '$y' is declared again in the same scope",
    )];
    let actions = parse_and_get_actions(src, &diags);

    let action = find_action(&actions, "Remove duplicate");
    assert!(action.is_some(), "Expected remove duplicate action");
    assert!(action.unwrap().is_preferred, "Remove duplicate 'my' should be the preferred action");
}

// ===========================================================================
// PL111 — MisspelledPragma
// ===========================================================================

#[test]
fn misspelled_pragma_fix_suggests_correct_spelling() {
    let src = "use warning;\nmy $x = 1;\n";
    let diags = [make_diag(
        0,
        12,
        "PL111",
        "Did you mean 'use warnings;'? 'warning' is not a known pragma",
    )];
    let actions = parse_and_get_actions(src, &diags);

    assert!(
        has_action_with_title(&actions, "warnings"),
        "Expected action to fix to 'warnings', got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

#[test]
fn misspelled_pragma_fix_replaces_full_range() {
    let src = "use strict;\nuse warning;\n";
    let use_warning_start = src.find("use warning;").unwrap_or(0);
    let diags = [make_diag(
        use_warning_start,
        use_warning_start + 12,
        "PL111",
        "Did you mean 'use warnings;'? 'warning' is not a known pragma",
    )];
    let actions = parse_and_get_actions(src, &diags);

    let action = find_action(&actions, "warnings");
    assert!(action.is_some(), "Expected a 'warnings' fix action");

    let edit = &action.unwrap().edit.changes[0];
    assert_eq!(edit.new_text, "use warnings;", "Should replace with 'use warnings;'");
    assert_eq!(edit.location.start, use_warning_start, "Should start at diagnostic range start");
}

#[test]
fn misspelled_pragma_fix_strict_typo() {
    let src = "use structs;\nmy $x = 1;\n";
    let diags =
        [make_diag(0, 12, "PL111", "Did you mean 'use strict;'? 'structs' is not a known pragma")];
    let actions = parse_and_get_actions(src, &diags);

    assert!(
        has_action_with_title(&actions, "strict"),
        "Expected action to fix to 'strict', got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );

    let action = find_action(&actions, "strict");
    assert!(action.is_some());
    let edit = &action.unwrap().edit.changes[0];
    assert_eq!(edit.new_text, "use strict;");
}

#[test]
fn misspelled_pragma_fix_is_preferred() {
    let src = "use warning;\nmy $x = 1;\n";
    let diags = [make_diag(
        0,
        12,
        "PL111",
        "Did you mean 'use warnings;'? 'warning' is not a known pragma",
    )];
    let actions = parse_and_get_actions(src, &diags);

    let action = find_action(&actions, "warnings");
    assert!(action.is_some(), "Expected a 'warnings' fix action");
    assert!(action.unwrap().is_preferred, "Pragma spelling fix should be preferred");
}

// ===========================================================================
// PL406 — UnreachableCode
// ===========================================================================

#[test]
fn unreachable_code_fix_removes_statement() {
    let src = "sub foo {\n    return 1;\n    my $dead = 2;\n}\n";
    let dead_start = src.find("my $dead").unwrap_or(0);
    let dead_end = dead_start + "my $dead = 2;".len();
    let diags = [make_diag(
        dead_start,
        dead_end,
        "PL406",
        "Unreachable code: this statement cannot be executed",
    )];
    let actions = parse_and_get_actions(src, &diags);

    assert!(
        has_action_with_title(&actions, "unreachable"),
        "Expected action to remove unreachable code, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

#[test]
fn unreachable_code_fix_is_quick_fix_kind() {
    let src = "sub foo {\n    return 1;\n    my $dead = 2;\n}\n";
    let dead_start = src.find("my $dead").unwrap_or(0);
    let dead_end = dead_start + "my $dead = 2;".len();
    let diags = [make_diag(
        dead_start,
        dead_end,
        "PL406",
        "Unreachable code: this statement cannot be executed",
    )];
    let actions = parse_and_get_actions(src, &diags);

    let action = find_action(&actions, "unreachable");
    assert!(action.is_some(), "Expected an unreachable code action");
    assert_eq!(action.unwrap().kind, CodeActionKind::QuickFix);
}

#[test]
fn unreachable_code_fix_deletes_entire_line() {
    let src = "sub foo {\n    return 1;\n    my $dead = 2;\n}\n";
    let dead_start = src.find("my $dead").unwrap_or(0);
    let dead_end = dead_start + "my $dead = 2;".len();
    let diags = [make_diag(
        dead_start,
        dead_end,
        "PL406",
        "Unreachable code: this statement cannot be executed",
    )];
    let actions = parse_and_get_actions(src, &diags);

    let action = find_action(&actions, "unreachable");
    assert!(action.is_some(), "Expected an unreachable code action");

    let edit = &action.unwrap().edit.changes[0];
    assert_eq!(edit.new_text, "", "Should delete the line completely");
    assert!(edit.location.end > edit.location.start, "Should span at least the statement");
}

// ===========================================================================
// PL300 — DuplicateSubroutine
// ===========================================================================

#[test]
fn duplicate_subroutine_fix_offers_rename() {
    let src = "sub greet { 'hi' }\nsub greet { 'hello' }\n";
    let second_greet = src.rfind("greet").unwrap_or(0);
    let diags = [make_diag(
        second_greet,
        second_greet + 5,
        "PL300",
        "Subroutine 'greet' is defined more than once",
    )];
    let actions = parse_and_get_actions(src, &diags);

    assert!(
        has_action_with_title(&actions, "greet"),
        "Expected action mentioning the sub name, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

#[test]
fn duplicate_subroutine_fix_is_quick_fix_kind() {
    let src = "sub foo { 1 }\nsub foo { 2 }\n";
    let second_foo = src.rfind("foo").unwrap_or(0);
    let diags = [make_diag(
        second_foo,
        second_foo + 3,
        "PL300",
        "Subroutine 'foo' is defined more than once",
    )];
    let actions = parse_and_get_actions(src, &diags);

    let action = find_action(&actions, "foo");
    assert!(action.is_some(), "Expected a duplicate subroutine action");
    assert_eq!(action.unwrap().kind, CodeActionKind::QuickFix);
}

// ===========================================================================
// PL301 — MissingReturn
// ===========================================================================

#[test]
fn missing_return_fix_adds_return_statement() {
    let src = "sub compute {\n    my $x = 1;\n}\n";
    let close_brace = src.rfind('}').unwrap_or(0);
    let diags = [make_diag(
        close_brace,
        close_brace + 1,
        "PL301",
        "Subroutine 'compute' has no explicit return statement",
    )];
    let actions = parse_and_get_actions(src, &diags);

    assert!(
        has_action_with_title(&actions, "return"),
        "Expected action to add return statement, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

#[test]
fn missing_return_fix_is_quick_fix_kind() {
    let src = "sub process {\n    my $y = 2;\n}\n";
    let close_brace = src.rfind('}').unwrap_or(0);
    let diags = [make_diag(
        close_brace,
        close_brace + 1,
        "PL301",
        "Subroutine 'process' has no explicit return statement",
    )];
    let actions = parse_and_get_actions(src, &diags);

    let action = find_action(&actions, "return");
    assert!(action.is_some(), "Expected a missing return action");
    assert_eq!(action.unwrap().kind, CodeActionKind::QuickFix);
}

#[test]
fn missing_return_fix_is_preferred() {
    let src = "sub calc {\n    my $z = 3;\n}\n";
    let close_brace = src.rfind('}').unwrap_or(0);
    let diags = [make_diag(
        close_brace,
        close_brace + 1,
        "PL301",
        "Subroutine 'calc' has no explicit return statement",
    )];
    let actions = parse_and_get_actions(src, &diags);

    let action = find_action(&actions, "return");
    assert!(action.is_some(), "Expected a missing return action");
    assert!(action.unwrap().is_preferred, "Add return should be the preferred action");
}
