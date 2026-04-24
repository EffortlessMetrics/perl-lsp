//! Tests for unused import detection lint

use perl_lsp_diagnostics::unused_imports::check_unused_imports;
use perl_lsp_diagnostics::{Diagnostic, DiagnosticSeverity, DiagnosticTag};
use perl_parser_core::{Node, NodeKind, SourceLocation};

fn loc(start: usize, end: usize) -> SourceLocation {
    SourceLocation { start, end }
}

fn use_node(module: &str, start: usize, end: usize) -> Node {
    Node::new(
        NodeKind::Use {
            module: module.to_string(),
            args: vec![],
            has_filter_risk: false,
            has_explicit_import_list: false,
        },
        loc(start, end),
    )
}

fn use_node_with_args(module: &str, args: Vec<&str>, start: usize, end: usize) -> Node {
    Node::new(
        NodeKind::Use {
            module: module.to_string(),
            args: args.into_iter().map(|s| s.to_string()).collect(),
            has_filter_risk: false,
            has_explicit_import_list: false,
        },
        loc(start, end),
    )
}

fn program(stmts: Vec<Node>) -> Node {
    Node::new(NodeKind::Program { statements: stmts }, loc(0, 500))
}

#[test]
fn unused_module_is_flagged() {
    let source = "use Foo::Bar;\nmy $x = 1;\n";
    let ast = program(vec![use_node("Foo::Bar", 0, 13)]);
    let mut diags: Vec<Diagnostic> = Vec::new();
    check_unused_imports(&ast, source, &mut diags);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code.as_deref(), Some("PL700"));
    assert!(diags[0].message.contains("Foo::Bar"));
}

#[test]
fn used_module_is_not_flagged() {
    let source = "use Foo::Bar;\nFoo::Bar->new();\n";
    let ast = program(vec![use_node("Foo::Bar", 0, 13)]);
    let mut diags: Vec<Diagnostic> = Vec::new();
    check_unused_imports(&ast, source, &mut diags);
    assert!(diags.is_empty());
}

#[test]
fn method_call_target_not_flagged() {
    let source = "use Some::Module;\nSome::Module::do_thing();\n";
    let ast = program(vec![use_node("Some::Module", 0, 17)]);
    let mut diags: Vec<Diagnostic> = Vec::new();
    check_unused_imports(&ast, source, &mut diags);
    assert!(diags.is_empty());
}

#[test]
fn pragma_strict_not_flagged() {
    let source = "use strict;\nmy $x = 1;\n";
    let ast = program(vec![use_node("strict", 0, 11)]);
    let mut diags: Vec<Diagnostic> = Vec::new();
    check_unused_imports(&ast, source, &mut diags);
    assert!(diags.is_empty());
}

#[test]
fn pragma_warnings_not_flagged() {
    let source = "use warnings;\nmy $x = 1;\n";
    let ast = program(vec![use_node("warnings", 0, 13)]);
    let mut diags: Vec<Diagnostic> = Vec::new();
    check_unused_imports(&ast, source, &mut diags);
    assert!(diags.is_empty());
}

#[test]
fn moose_not_flagged() {
    let source = "use Moose;\nhas 'name' => (is => 'ro');\n";
    let ast = program(vec![use_node("Moose", 0, 10)]);
    let mut diags: Vec<Diagnostic> = Vec::new();
    check_unused_imports(&ast, source, &mut diags);
    assert!(diags.is_empty());
}

#[test]
fn test_more_not_flagged() {
    let source = "use Test::More;\nok(1, 'works');\n";
    let ast = program(vec![use_node("Test::More", 0, 15)]);
    let mut diags: Vec<Diagnostic> = Vec::new();
    check_unused_imports(&ast, source, &mut diags);
    assert!(diags.is_empty());
}

#[test]
fn module_with_import_args_not_flagged() {
    let source = "use Foo::Bar qw(baz);\nmy $x = baz();\n";
    let ast = program(vec![use_node_with_args("Foo::Bar", vec!["baz"], 0, 21)]);
    let mut diags: Vec<Diagnostic> = Vec::new();
    check_unused_imports(&ast, source, &mut diags);
    assert!(diags.is_empty());
}

#[test]
fn diagnostic_has_hint_severity_and_unnecessary_tag() {
    let source = "use Unused::Mod;\nmy $x = 1;\n";
    let ast = program(vec![use_node("Unused::Mod", 0, 16)]);
    let mut diags: Vec<Diagnostic> = Vec::new();
    check_unused_imports(&ast, source, &mut diags);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].severity, DiagnosticSeverity::Hint);
    assert!(diags[0].tags.contains(&DiagnosticTag::Unnecessary));
}

#[test]
fn multiple_unused_all_flagged() {
    let source = "use Alpha;\nuse Beta;\nmy $x = 1;\n";
    let ast = program(vec![use_node("Alpha", 0, 10), use_node("Beta", 11, 20)]);
    let mut diags: Vec<Diagnostic> = Vec::new();
    check_unused_imports(&ast, source, &mut diags);
    assert_eq!(diags.len(), 2);
}

#[test]
fn mixed_used_and_unused() {
    let source = "use Alpha;\nuse Beta;\nAlpha->new();\n";
    let ast = program(vec![use_node("Alpha", 0, 10), use_node("Beta", 11, 20)]);
    let mut diags: Vec<Diagnostic> = Vec::new();
    check_unused_imports(&ast, source, &mut diags);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("Beta"));
}

#[test]
fn empty_program_no_diagnostics() {
    let source = "";
    let ast = program(vec![]);
    let mut diags: Vec<Diagnostic> = Vec::new();
    check_unused_imports(&ast, source, &mut diags);
    assert!(diags.is_empty());
}

#[test]
fn version_import_not_flagged() {
    let source = "use 5.010;\nmy $x = 1;\n";
    let ast = program(vec![use_node("5.010", 0, 10)]);
    let mut diags: Vec<Diagnostic> = Vec::new();
    check_unused_imports(&ast, source, &mut diags);
    assert!(diags.is_empty());
}

#[test]
fn substring_match_does_not_count() {
    let source = "use Foo;\nmy $x = FooBar->new();\n";
    let ast = program(vec![use_node("Foo", 0, 8)]);
    let mut diags: Vec<Diagnostic> = Vec::new();
    check_unused_imports(&ast, source, &mut diags);
    assert_eq!(diags.len(), 1, "substring match should not suppress diagnostic");
}
