//! Sub::Exporter goto-definition tests for perl-semantic-analyzer.
//!
//! Tests cover:
//! - Simple exports: goto-definition on foo imported via `{ exports => [qw(foo)] }`
//! - Groups/tag resolution: goto-definition on symbol imported via `:default` tag
//! - Renaming: goto-definition on renamed symbol
//! - -setup syntax: `use Sub::Exporter -setup => { exports => [...] }`

use perl_semantic_analyzer::Parser;
use perl_semantic_analyzer::analysis::declaration::{DeclarationProvider, ParentMap};
use perl_tdd_support::must;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Helper utilities
// ---------------------------------------------------------------------------

fn parse_and_provider(code: &str) -> (DeclarationProvider, Arc<perl_ast::Node>, String) {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let ast_arc = Arc::new(ast);
    let mut parent_map = ParentMap::default();
    DeclarationProvider::build_parent_map(&ast_arc, &mut parent_map, None);
    let provider =
        DeclarationProvider::new(ast_arc.clone(), code.to_string(), "file:///test.pl".to_string())
            .with_parent_map(&parent_map);
    (provider, ast_arc, code.to_string())
}

fn find_declaration(code: &str, symbol: &str) -> Option<String> {
    let (provider, _, _) = parse_and_provider(code);
    let offset = must(code.find(symbol)) + 1; // skip sigil
    provider
        .find_declaration(offset, 0)
        .map(|links| {
            links
                .first()
                .map(|link| {
                    let target =
                        &code[link.target_selection_range.0..link.target_selection_range.1];
                    target.to_string()
                })
                .unwrap_or_default()
        })
        .unwrap_or(None)
}

// ---------------------------------------------------------------------------
// Test 1: Simple exports array - foo resolves to MyModule
// ---------------------------------------------------------------------------

#[test]
fn sub_exporter_goto_simple_exports_resolves_foo() {
    let code = "package MyModule;\nuse MyModule { exports => [qw(foo bar)] };\nfoo();";
    let result = find_declaration(code, "foo");
    assert!(result.is_some(), "foo should resolve to MyModule; got: {:?}", result);
}

// ---------------------------------------------------------------------------
// Test 2: Simple exports array - bar resolves to MyModule
// ---------------------------------------------------------------------------

#[test]
fn sub_exporter_goto_simple_exports_resolves_bar() {
    let code = "package MyModule;\nuse MyModule { exports => [qw(foo bar)] };\nbar();";
    let result = find_declaration(code, "bar");
    assert!(result.is_some(), "bar should resolve to MyModule; got: {:?}", result);
}

// ---------------------------------------------------------------------------
// Test 3: -setup syntax exports
// ---------------------------------------------------------------------------

#[test]
fn sub_exporter_goto_setup_syntax_exports() {
    let code = "package MyModule;\nuse Sub::Exporter -setup => { exports => [qw(foo)] };\nfoo();";
    let result = find_declaration(code, "foo");
    assert!(result.is_some(), "foo should resolve with -setup syntax; got: {:?}", result);
}

// ---------------------------------------------------------------------------
// Test 4: Groups - :default tag resolves to symbol
// ---------------------------------------------------------------------------

#[test]
fn sub_exporter_goto_groups_default_tag() {
    let code = r#"package MyModule;
use MyModule {
    exports => [qw(foo bar baz)],
    groups => { default => [qw(foo bar)] },
};
foo();"#;
    let result = find_declaration(code, "foo");
    assert!(result.is_some(), "foo via :default should resolve; got: {:?}", result);
}

// ---------------------------------------------------------------------------
// Test 5: Renaming with -as - goto on renamed symbol
// ---------------------------------------------------------------------------

#[test]
fn sub_exporter_goto_renaming_as() {
    let code = "package MyModule;\nuse Module func1 => { -as => 'my_func1' };\nmy_func1();";
    let result = find_declaration(code, "my_func1");
    assert!(result.is_some(), "my_func1 should resolve to original func1; got: {:?}", result);
}

// ---------------------------------------------------------------------------
// Test 6: MethodCall with HashLiteral
// ---------------------------------------------------------------------------

#[test]
fn sub_exporter_goto_method_call_hash_literal() {
    let code =
        "package MyModule;\nuse MyModule;\nMyModule->import({ exports => [qw(foo)] });\nfoo();";
    let result = find_declaration(code, "foo");
    assert!(result.is_some(), "foo from MethodCall HashLiteral should resolve; got: {:?}", result);
}

// ---------------------------------------------------------------------------
// Test 7: No regression - standard Exporter still works
// ---------------------------------------------------------------------------

#[test]
fn sub_exporter_no_regression_standard_exporter() {
    let code = "package My::Loader;\nour @EXPORT = qw(load_data);\nsub load_data { }\n\npackage main;\nuse My::Loader qw(load_data);\nload_data();";
    let result = find_declaration(code, "load_data");
    assert!(
        result.is_some(),
        "load_data should still resolve with standard Exporter; got: {:?}",
        result
    );
}

// ---------------------------------------------------------------------------
// Test 8: Multiple Sub::Exporter uses
// ---------------------------------------------------------------------------

#[test]
fn sub_exporter_goto_multiple_uses() {
    let code = "package MyModule;\nuse MyModule { exports => [qw(foo)] };\nuse MyModule { exports => [qw(bar)] };\nbar();";
    let result = find_declaration(code, "bar");
    assert!(
        result.is_some(),
        "bar from second Sub::Exporter use should resolve; got: {:?}",
        result
    );
}

// ---------------------------------------------------------------------------
// Test 9: baz symbol (not in default group but in exports)
// ---------------------------------------------------------------------------

#[test]
fn sub_exporter_goto_baz_not_in_default_group() {
    let code = "package MyModule;\nuse MyModule {\n    exports => [qw(foo bar baz)],\n    groups => { default => [qw(foo)] },\n};\nbaz();";
    let result = find_declaration(code, "baz");
    assert!(
        result.is_some(),
        "baz should resolve even though it's not in default group; got: {:?}",
        result
    );
}

// ---------------------------------------------------------------------------
// Test 10: Multiple renaming
// ---------------------------------------------------------------------------

#[test]
fn sub_exporter_goto_multiple_renaming() {
    let code = "package MyModule;\nuse Module\n    func1 => { -as => 'my_func1' },\n    func2 => { -as => 'my_func2' };\nmy_func2();";
    let result = find_declaration(code, "my_func2");
    assert!(result.is_some(), "my_func2 should resolve to func2; got: {:?}", result);
}

// ---------------------------------------------------------------------------
// Test 11: Sub::Exporter used as setup in exporter module
// ---------------------------------------------------------------------------

#[test]
fn sub_exporter_goto_setup_exports_in_module() {
    let code = "package Exporter::Sub;\nuse Sub::Exporter -setup => {\n    exports => [qw(exported1 exported2)],\n};\nsub exported1 { }\nsub exported2 { }\n\npackage main;\nuse Exporter::Sub { exports => [qw(exported1)] };\nexported1();";
    let result = find_declaration(code, "exported1");
    assert!(
        result.is_some(),
        "exported1 should resolve in module using Sub::Exporter -setup; got: {:?}",
        result
    );
}

// ---------------------------------------------------------------------------
// Test 12: Empty exports (no symbols available)
// ---------------------------------------------------------------------------

#[test]
fn sub_exporter_goto_empty_exports_no_resolution() {
    let code = "package MyModule;\nuse MyModule { exports => [] };\nfoo();";
    let result = find_declaration(code, "foo");
    // foo should NOT resolve since exports is empty
    assert!(result.is_none(), "foo should NOT resolve when exports is empty; got: {:?}", result);
}

// ---------------------------------------------------------------------------
// Test 13: Nested hash in groups (simplified)
// ---------------------------------------------------------------------------

#[test]
fn sub_exporter_goto_nested_groups_structure() {
    let code = "package MyModule;\nuse MyModule {\n    exports => [qw(alpha beta)],\n    groups => {\n        default => [qw(alpha)],\n        all => [qw(alpha beta)],\n    },\n};\nbeta();";
    let result = find_declaration(code, "beta");
    assert!(
        result.is_some(),
        "beta should resolve from exports even with nested groups; got: {:?}",
        result
    );
}

// ---------------------------------------------------------------------------
// Test 14: Standard qw import still works alongside Sub::Exporter
// ---------------------------------------------------------------------------

#[test]
fn sub_exporter_goto_mixed_standard_and_sub_exporter() {
    let code = "package My::Loader;\nour @EXPORT = qw(standard_func);\nsub standard_func { }\n\npackage main;\nuse My::Loader qw(standard_func);\nstandard_func();";
    let result = find_declaration(code, "standard_func");
    assert!(
        result.is_some(),
        "standard_func should resolve with standard Exporter; got: {:?}",
        result
    );
}

// ---------------------------------------------------------------------------
// Test 15: Symbol not in exports (should not resolve)
// ---------------------------------------------------------------------------

#[test]
fn sub_exporter_goto_symbol_not_in_exports() {
    let code = "package MyModule;\nuse MyModule { exports => [qw(foo)] };\nbar();";
    let result = find_declaration(code, "bar");
    // bar should NOT resolve since it's not in exports
    assert!(result.is_none(), "bar should NOT resolve when not in exports; got: {:?}", result);
}

// ---------------------------------------------------------------------------
// Test 16: All groups resolve to all exports
// ---------------------------------------------------------------------------

#[test]
fn sub_exporter_goto_all_group_resolves() {
    let code = "package MyModule;\nuse MyModule {\n    exports => [qw(all_sym)],\n    groups => { all => [qw(all_sym)] },\n};\nall_sym();";
    let result = find_declaration(code, "all_sym");
    assert!(result.is_some(), "all_sym should resolve via :all group; got: {:?}", result);
}
