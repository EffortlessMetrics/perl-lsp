//! Integration tests for Perl modernization code actions

use perl_lsp_code_actions::{CodeActionKind, CodeActionsProvider};
use perl_parser_core::Parser;
use perl_tdd_support::must;

fn get_actions(source: &str) -> Vec<perl_lsp_code_actions::CodeAction> {
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let provider = CodeActionsProvider::new(source.to_string());
    provider.get_code_actions(&ast, (0, source.len()), &[])
}

fn modernize_actions(source: &str) -> Vec<perl_lsp_code_actions::CodeAction> {
    get_actions(source)
        .into_iter()
        .filter(|a| a.kind == CodeActionKind::SourceModernize)
        .collect()
}

#[test]
fn two_arg_open_suggests_three_arg() {
    let actions = modernize_actions(r#"open(FILE, ">output.txt");"#);
    assert!(
        actions.iter().any(|a| a.title.contains("three-arg open")),
        "Expected three-arg open suggestion, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

#[test]
fn three_arg_open_not_flagged() {
    let actions = modernize_actions(r#"open(my $fh, ">", "output.txt");"#);
    assert!(
        !actions.iter().any(|a| a.title.contains("three-arg open")),
        "Three-arg open should not trigger modernization"
    );
}

#[test]
fn two_arg_open_edit_contains_error_handling() {
    let actions = modernize_actions(r#"open(FILE, ">output.txt");"#);
    let open_action = actions.iter().find(|a| a.title.contains("three-arg open"));
    assert!(open_action.is_some(), "Expected three-arg open action");
    let edit = &open_action.map(|a| &a.edit);
    assert!(
        edit.is_some_and(|e| e.changes.iter().any(|c| c.new_text.contains("or die"))),
        "Modern open should include error handling"
    );
}

#[test]
fn deprecated_defined_array_detected() {
    let actions = modernize_actions("if (defined(@array)) { }");
    assert!(
        actions
            .iter()
            .any(|a| a.title.contains("deprecated defined(@")),
        "Expected deprecated defined(@) suggestion, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

#[test]
fn deprecated_defined_hash_detected() {
    let actions = modernize_actions("if (defined(%hash)) { }");
    assert!(
        actions
            .iter()
            .any(|a| a.title.contains("deprecated defined(%")),
        "Expected deprecated defined(%) suggestion"
    );
}

#[test]
fn defined_scalar_not_flagged() {
    let actions = modernize_actions("if (defined($x)) { }");
    assert!(
        !actions
            .iter()
            .any(|a| a.title.contains("deprecated defined")),
        "defined($scalar) should NOT be flagged"
    );
}

#[test]
fn deprecated_defined_edit_removes_defined() {
    let actions = modernize_actions("if (defined(@array)) { }");
    let action = actions.iter().find(|a| a.title.contains("deprecated"));
    assert!(action.is_some());
    let edit = &action.map(|a| &a.edit);
    assert!(
        edit.is_some_and(|e| e.changes.iter().any(|c| c.new_text == "@array")),
        "Edit should replace with bare @array"
    );
}

#[test]
fn require_version_suggests_use_v() {
    let actions = modernize_actions("require 5.006;");
    assert!(
        actions.iter().any(|a| a.title.contains("use v5.6")),
        "Expected 'use v5.6' suggestion, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

#[test]
fn require_5010_suggests_v510() {
    let actions = modernize_actions("require 5.010;");
    assert!(
        actions.iter().any(|a| a.title.contains("use v5.10")),
        "Expected 'use v5.10' suggestion"
    );
}

#[test]
fn require_module_not_flagged() {
    let actions = modernize_actions("require Foo::Bar;");
    assert!(
        !actions.iter().any(|a| a.title.contains("require")),
        "require Module should NOT trigger modernization"
    );
}

#[test]
fn missing_strict_warnings_suggests_add() {
    let actions = modernize_actions("print 'hello';");
    assert!(
        actions.iter().any(|a| a.title.contains("use strict")),
        "Expected strict suggestion, got: {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>()
    );
}

#[test]
fn strict_warnings_present_no_modernize_suggestion() {
    let actions = modernize_actions("use strict;\nuse warnings;\nprint 'hello';");
    assert!(
        !actions.iter().any(|a| a.title.contains("Modernize: add")),
        "Should not suggest when both strict/warnings present"
    );
}

#[test]
fn moose_implies_strict_no_suggestion() {
    let actions = modernize_actions("use Moose;\nprint 'hello';");
    assert!(
        !actions
            .iter()
            .any(|a| a.title.contains("Modernize: add use strict")),
        "Moose implies strict/warnings"
    );
}

#[test]
fn all_modernize_actions_have_correct_kind() {
    let source = "require 5.006;\nopen(FILE, \">foo\");\nif (defined(@arr)) {}";
    let actions = modernize_actions(source);
    assert!(
        !actions.is_empty(),
        "Expected at least one modernize action"
    );
    for action in &actions {
        assert_eq!(
            action.kind,
            CodeActionKind::SourceModernize,
            "Action '{}' should have SourceModernize kind",
            action.title
        );
    }
}

#[test]
fn modernize_actions_have_non_empty_edits() {
    let source = "require 5.006;\nopen(FILE, \">foo\");\nif (defined(@arr)) {}";
    let actions = modernize_actions(source);
    for action in &actions {
        assert!(
            !action.edit.changes.is_empty(),
            "Action '{}' should have at least one edit",
            action.title
        );
    }
}
