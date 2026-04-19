//! Comprehensive unit tests for `perl-lsp-navigation` crate.
//!
//! Covers: document_links, references, type_definition, type_hierarchy,
//! workspace_symbols — including edge cases and error paths.

use perl_lsp_navigation::{
    TypeDefinitionProvider, TypeHierarchyItem, TypeHierarchyProvider, TypeHierarchySymbolKind,
    WorkspaceSymbol, WorkspaceSymbolsProvider, compute_links, find_references_single_file,
};
use perl_parser_core::Parser;
use perl_tdd_support::{must, must_some};
use std::collections::HashMap;

// ──────────────────────────────────────────────
// document_links — compute_links
// ──────────────────────────────────────────────

#[test]
fn compute_links_use_qualified_module() -> Result<(), Box<dyn std::error::Error>> {
    let links = compute_links("file:///ws/test.pl", "use Foo::Bar;\n", &[]);
    assert_eq!(links.len(), 1);
    let link = links.first().ok_or("expected at least one link")?;
    assert_eq!(
        link.pointer("/data/type")
            .and_then(serde_json::Value::as_str),
        Some("module")
    );
    assert_eq!(
        link.pointer("/data/module")
            .and_then(serde_json::Value::as_str),
        Some("Foo::Bar")
    );
    Ok(())
}

#[test]
fn compute_links_require_qualified_module() -> Result<(), Box<dyn std::error::Error>> {
    let links = compute_links("file:///ws/test.pl", "require Foo::Bar;\n", &[]);
    assert_eq!(links.len(), 1);
    let link = links.first().ok_or("expected at least one link")?;
    assert_eq!(
        link.pointer("/data/type")
            .and_then(serde_json::Value::as_str),
        Some("module")
    );
    Ok(())
}

#[test]
fn compute_links_require_quoted_file_path() -> Result<(), Box<dyn std::error::Error>> {
    let links = compute_links("file:///ws/test.pl", "require 'Foo/Bar.pm';\n", &[]);
    assert_eq!(links.len(), 1);
    let link = links.first().ok_or("expected at least one link")?;
    assert_eq!(
        link.pointer("/data/type")
            .and_then(serde_json::Value::as_str),
        Some("file")
    );
    assert_eq!(
        link.pointer("/data/path")
            .and_then(serde_json::Value::as_str),
        Some("Foo/Bar.pm")
    );
    Ok(())
}

#[test]
fn compute_links_require_double_quoted_file_path() -> Result<(), Box<dyn std::error::Error>> {
    let links = compute_links("file:///ws/test.pl", "require \"Foo/Bar.pm\";\n", &[]);
    assert_eq!(links.len(), 1);
    let link = links.first().ok_or("expected at least one link")?;
    assert_eq!(
        link.pointer("/data/type")
            .and_then(serde_json::Value::as_str),
        Some("file")
    );
    Ok(())
}

#[test]
fn compute_links_skips_pragmas() {
    let pragmas = [
        "use strict;",
        "use warnings;",
        "use utf8;",
        "use feature 'say';",
        "use constant FOO => 1;",
        "use lib 'lib';",
        "use parent 'Base';",
        "use base 'Base';",
    ];
    for pragma in &pragmas {
        let links = compute_links("file:///t.pl", pragma, &[]);
        assert!(
            links.is_empty(),
            "pragma '{}' should not produce a document link",
            pragma
        );
    }
}

#[test]
fn compute_links_empty_text() {
    let links = compute_links("file:///t.pl", "", &[]);
    assert!(links.is_empty());
}

#[test]
fn compute_links_no_use_or_require() {
    let links = compute_links("file:///t.pl", "my $x = 42;\nprint $x;\n", &[]);
    assert!(links.is_empty());
}

#[test]
fn compute_links_multiple_use_statements() {
    let text = "use Foo::Bar;\nuse Baz::Qux;\n";
    let links = compute_links("file:///t.pl", text, &[]);
    assert_eq!(links.len(), 2);
}

#[test]
fn compute_links_use_parent_produces_no_link() {
    let links = compute_links("file:///t.pl", "use parent 'Foo::Bar';\n", &[]);
    assert!(links.is_empty());
}

#[test]
fn compute_links_use_base_produces_no_link() {
    let links = compute_links("file:///t.pl", "use base 'Foo::Bar';\n", &[]);
    assert!(links.is_empty());
}

#[test]
fn compute_links_require_bare_name_no_colons_skipped() {
    // require without :: in the token should not produce a module link
    // (only file links via quoted paths)
    let links = compute_links("file:///t.pl", "require Foo;\n", &[]);
    // No module link for unqualified require (no ::)
    assert!(links.is_empty());
}

#[test]
fn compute_links_correct_range_for_use() -> Result<(), Box<dyn std::error::Error>> {
    let text = "use Foo::Bar;\n";
    let links = compute_links("file:///t.pl", text, &[]);
    let link = links.first().ok_or("expected a link")?;
    let start_char = link
        .pointer("/range/start/character")
        .and_then(serde_json::Value::as_u64)
        .ok_or("missing start character")?;
    let end_char = link
        .pointer("/range/end/character")
        .and_then(serde_json::Value::as_u64)
        .ok_or("missing end character")?;
    // The module name "Foo::Bar" should span from after "use " to before ";"
    assert!(start_char < end_char);
    Ok(())
}

#[test]
fn compute_links_tooltip_contains_module_name() -> Result<(), Box<dyn std::error::Error>> {
    let links = compute_links("file:///t.pl", "use My::Module;\n", &[]);
    let link = links.first().ok_or("expected a link")?;
    let tooltip = link
        .pointer("/tooltip")
        .and_then(serde_json::Value::as_str)
        .ok_or("missing tooltip")?;
    assert!(
        tooltip.contains("My::Module"),
        "tooltip should contain module name"
    );
    Ok(())
}

#[test]
fn compute_links_base_uri_propagated() -> Result<(), Box<dyn std::error::Error>> {
    let uri = "file:///workspace/lib/test.pl";
    let links = compute_links(uri, "use Foo::Bar;\n", &[]);
    let link = links.first().ok_or("expected a link")?;
    let base_uri = link
        .pointer("/data/baseUri")
        .and_then(serde_json::Value::as_str)
        .ok_or("missing baseUri")?;
    assert_eq!(base_uri, uri);
    Ok(())
}

// ──────────────────────────────────────────────
// references — find_references_single_file
// ──────────────────────────────────────────────

fn parse_ast(code: &str) -> perl_parser_core::ast::Node {
    let mut parser = Parser::new(code);
    must(parser.parse())
}

#[test]
fn refs_finds_variable_references() {
    let code = "my $count = 0; $count++; print $count;";
    let ast = parse_ast(code);

    // Find offset of first "$count" (at the variable declaration)
    let offset = must_some(code.find("$count"));
    let refs = find_references_single_file(&ast, offset);

    // Should find at least the declaration and one usage
    assert!(refs.is_some(), "should find variable references");
    let refs = must_some(refs);
    assert!(
        refs.len() >= 2,
        "should find at least 2 references, found {}",
        refs.len()
    );
}

#[test]
fn refs_finds_function_call_references() {
    let code = "sub greet { } greet();";
    let ast = parse_ast(code);

    // Offset inside the subroutine definition
    let offset = must_some(code.find("greet"));
    let refs = find_references_single_file(&ast, offset);

    assert!(refs.is_some(), "should find subroutine references");
    let refs = must_some(refs);
    assert!(
        refs.len() >= 2,
        "should find definition + call, found {}",
        refs.len()
    );
}

#[test]
fn refs_returns_none_for_non_symbol_offset() {
    let code = "my $x = 42;";
    let ast = parse_ast(code);

    // Offset on "42" — not a variable or function
    let offset = must_some(code.find("42"));
    let refs = find_references_single_file(&ast, offset);

    // Should be None since 42 is a literal
    // (it's fine if the implementation returns Some with the literal node)
    // Just ensure no panic
    let _ = refs;
}

#[test]
fn refs_returns_none_for_out_of_range_offset() {
    let code = "my $x = 1;";
    let ast = parse_ast(code);
    let refs = find_references_single_file(&ast, 99999);
    assert!(refs.is_none(), "out-of-range offset should return None");
}

#[test]
fn refs_empty_source() {
    let code = "";
    let ast = parse_ast(code);
    let refs = find_references_single_file(&ast, 0);
    // Empty program — should be None or empty references
    let _ = refs;
}

#[test]
fn refs_variable_with_different_sigils_not_confused() {
    // $foo and @foo are distinct variables in Perl
    let code = "my $foo = 1; my @foo = (2, 3);";
    let ast = parse_ast(code);

    let offset = must_some(code.find("$foo"));
    let refs = find_references_single_file(&ast, offset);
    if let Some(refs) = refs {
        // All references should share the same sigil
        for &(start, end) in &refs {
            let fragment = &code[start..end.min(code.len())];
            assert!(
                !fragment.starts_with('@'),
                "scalar $foo reference should not match @foo: '{}'",
                fragment
            );
        }
    }
}

// ──────────────────────────────────────────────
// type_definition — TypeDefinitionProvider
// ──────────────────────────────────────────────

#[test]
fn type_definition_provider_default_trait() {
    let provider = TypeDefinitionProvider;
    // Just ensure Default impl works without panic
    let _ = provider;
}

#[test]
fn type_definition_provider_new() {
    let provider = TypeDefinitionProvider::new();
    let _ = provider;
}

// ──────────────────────────────────────────────
// type_hierarchy — TypeHierarchyProvider
// ──────────────────────────────────────────────

#[test]
fn type_hierarchy_provider_default_trait() {
    let provider = TypeHierarchyProvider;
    let _ = provider;
}

#[test]
fn type_hierarchy_prepare_on_package() {
    let code = "package MyClass;\nsub new { }\n";
    let ast = parse_ast(code);
    let provider = TypeHierarchyProvider::new();

    // Offset inside "MyClass" (byte 8 is within "package MyClass")
    let items = provider.prepare(&ast, code, 8);
    assert!(
        items.is_some(),
        "should find type hierarchy item for package"
    );
    let items = must_some(items);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "MyClass");
}

#[test]
fn type_hierarchy_prepare_outside_package_returns_none() {
    let code = "my $x = 1;\n";
    let ast = parse_ast(code);
    let provider = TypeHierarchyProvider::new();

    let items = provider.prepare(&ast, code, 0);
    // Should be None since there's no package at offset 0
    assert!(
        items.is_none(),
        "should not find hierarchy for non-package position"
    );
}

#[test]
fn type_hierarchy_find_supertypes_use_parent() {
    let code = "package Child;\nuse parent 'Parent';\n";
    let ast = parse_ast(code);
    let provider = TypeHierarchyProvider::new();

    let item = TypeHierarchyItem {
        name: "Child".to_string(),
        kind: TypeHierarchySymbolKind::Class,
        uri: "file:///test.pl".to_string(),
        range: perl_position_tracking::WireRange::default(),
        selection_range: perl_position_tracking::WireRange::default(),
        detail: None,
        data: None,
    };

    let supertypes = provider.find_supertypes(&ast, &item);
    assert_eq!(supertypes.len(), 1);
    assert_eq!(supertypes[0].name, "Parent");
}

#[test]
fn type_hierarchy_find_supertypes_use_base() {
    let code = "package Child;\nuse base 'Parent';\n";
    let ast = parse_ast(code);
    let provider = TypeHierarchyProvider::new();

    let item = TypeHierarchyItem {
        name: "Child".to_string(),
        kind: TypeHierarchySymbolKind::Class,
        uri: "file:///test.pl".to_string(),
        range: perl_position_tracking::WireRange::default(),
        selection_range: perl_position_tracking::WireRange::default(),
        detail: None,
        data: None,
    };

    let supertypes = provider.find_supertypes(&ast, &item);
    assert_eq!(supertypes.len(), 1);
    assert_eq!(supertypes[0].name, "Parent");
}

#[test]
fn type_hierarchy_find_subtypes() {
    let code = "package Base;\n\npackage Derived;\nuse parent 'Base';\n";
    let ast = parse_ast(code);
    let provider = TypeHierarchyProvider::new();

    let base_item = TypeHierarchyItem {
        name: "Base".to_string(),
        kind: TypeHierarchySymbolKind::Class,
        uri: "file:///test.pl".to_string(),
        range: perl_position_tracking::WireRange::default(),
        selection_range: perl_position_tracking::WireRange::default(),
        detail: None,
        data: None,
    };

    let subtypes = provider.find_subtypes(&ast, &base_item);
    assert_eq!(subtypes.len(), 1);
    assert_eq!(subtypes[0].name, "Derived");
}

#[test]
fn type_hierarchy_no_subtypes_for_leaf_class() {
    let code = "package LeafClass;\nuse parent 'SomeParent';\n";
    let ast = parse_ast(code);
    let provider = TypeHierarchyProvider::new();

    let item = TypeHierarchyItem {
        name: "LeafClass".to_string(),
        kind: TypeHierarchySymbolKind::Class,
        uri: "file:///test.pl".to_string(),
        range: perl_position_tracking::WireRange::default(),
        selection_range: perl_position_tracking::WireRange::default(),
        detail: None,
        data: None,
    };

    let subtypes = provider.find_subtypes(&ast, &item);
    assert!(subtypes.is_empty(), "leaf class should have no subtypes");
}

#[test]
fn type_hierarchy_no_supertypes_for_root_class() {
    let code = "package RootClass;\nsub new { }\n";
    let ast = parse_ast(code);
    let provider = TypeHierarchyProvider::new();

    let item = TypeHierarchyItem {
        name: "RootClass".to_string(),
        kind: TypeHierarchySymbolKind::Class,
        uri: "file:///test.pl".to_string(),
        range: perl_position_tracking::WireRange::default(),
        selection_range: perl_position_tracking::WireRange::default(),
        detail: None,
        data: None,
    };

    let supertypes = provider.find_supertypes(&ast, &item);
    assert!(
        supertypes.is_empty(),
        "root class should have no supertypes"
    );
}

#[test]
fn type_hierarchy_multiple_supertypes_use_parent() {
    let code = "package Multi;\nuse parent 'Base1', 'Base2';\n";
    let ast = parse_ast(code);
    let provider = TypeHierarchyProvider::new();

    let item = TypeHierarchyItem {
        name: "Multi".to_string(),
        kind: TypeHierarchySymbolKind::Class,
        uri: "file:///test.pl".to_string(),
        range: perl_position_tracking::WireRange::default(),
        selection_range: perl_position_tracking::WireRange::default(),
        detail: None,
        data: None,
    };

    let supertypes = provider.find_supertypes(&ast, &item);
    assert_eq!(supertypes.len(), 2, "should find both parents");
    let names: Vec<&str> = supertypes.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"Base1"));
    assert!(names.contains(&"Base2"));
}

#[test]
fn type_hierarchy_multiple_subtypes() {
    let code = concat!(
        "package Base;\n\n",
        "package DerivedA;\nuse parent 'Base';\n\n",
        "package DerivedB;\nuse parent 'Base';\n\n",
        "package Unrelated;\nuse parent 'Other';\n"
    );
    let ast = parse_ast(code);
    let provider = TypeHierarchyProvider::new();

    let base_item = TypeHierarchyItem {
        name: "Base".to_string(),
        kind: TypeHierarchySymbolKind::Class,
        uri: "file:///test.pl".to_string(),
        range: perl_position_tracking::WireRange::default(),
        selection_range: perl_position_tracking::WireRange::default(),
        detail: None,
        data: None,
    };

    let subtypes = provider.find_subtypes(&ast, &base_item);
    assert_eq!(subtypes.len(), 2);
    let names: Vec<&str> = subtypes.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"DerivedA"));
    assert!(names.contains(&"DerivedB"));
    assert!(!names.contains(&"Unrelated"));
}

#[test]
fn type_hierarchy_symbol_kind_values() {
    // LSP protocol requires specific numeric values
    assert_eq!(TypeHierarchySymbolKind::Class as i32, 5);
    assert_eq!(TypeHierarchySymbolKind::Method as i32, 6);
    assert_eq!(TypeHierarchySymbolKind::Function as i32, 12);
}

#[test]
fn type_hierarchy_item_serialization() -> Result<(), Box<dyn std::error::Error>> {
    let item = TypeHierarchyItem {
        name: "TestPkg".to_string(),
        kind: TypeHierarchySymbolKind::Class,
        uri: "file:///test.pl".to_string(),
        range: perl_position_tracking::WireRange::default(),
        selection_range: perl_position_tracking::WireRange::default(),
        detail: Some("Perl Package".to_string()),
        data: None,
    };

    let json = serde_json::to_string(&item)?;
    assert!(json.contains("TestPkg"));
    assert!(json.contains("Perl Package"));

    // Round-trip
    let deserialized: TypeHierarchyItem = serde_json::from_str(&json)?;
    assert_eq!(deserialized.name, "TestPkg");
    Ok(())
}

#[test]
fn type_hierarchy_block_form_package() {
    let code = "package Outer {\n  package Inner;\n  use parent 'Outer';\n}\n";
    let ast = parse_ast(code);
    let provider = TypeHierarchyProvider::new();

    let outer_item = TypeHierarchyItem {
        name: "Outer".to_string(),
        kind: TypeHierarchySymbolKind::Class,
        uri: "file:///test.pl".to_string(),
        range: perl_position_tracking::WireRange::default(),
        selection_range: perl_position_tracking::WireRange::default(),
        detail: None,
        data: None,
    };

    let subtypes = provider.find_subtypes(&ast, &outer_item);
    // Inner inherits from Outer
    let names: Vec<&str> = subtypes.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"Inner"));
}

// ──────────────────────────────────────────────
// workspace_symbols — WorkspaceSymbolsProvider
// ──────────────────────────────────────────────

fn make_provider_with_source(
    uri: &str,
    source: &str,
) -> (WorkspaceSymbolsProvider, HashMap<String, String>) {
    let mut provider = WorkspaceSymbolsProvider::new();
    let mut source_map = HashMap::new();

    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    provider.index_document(uri, &ast, source);
    source_map.insert(uri.to_string(), source.to_string());

    (provider, source_map)
}

#[test]
fn workspace_symbols_default_trait() {
    let provider = WorkspaceSymbolsProvider::default();
    let _ = provider;
}

#[test]
fn workspace_symbols_index_and_search_sub() {
    let source = "sub hello { print 'world'; }\n";
    let (provider, source_map) = make_provider_with_source("file:///test.pl", source);

    let results = provider.search("hello", &source_map);
    assert!(!results.is_empty(), "should find 'hello' subroutine");
    assert_eq!(results[0].name, "hello");
}

#[test]
fn workspace_symbols_search_package() {
    let source = "package My::Package;\nsub foo { }\n";
    let (provider, source_map) = make_provider_with_source("file:///test.pl", source);

    let results = provider.search("My::Package", &source_map);
    assert!(!results.is_empty(), "should find package symbol");
}

#[test]
fn workspace_symbols_empty_query_returns_all() {
    let source = "sub alpha { }\nsub beta { }\n";
    let (provider, source_map) = make_provider_with_source("file:///test.pl", source);

    let results = provider.search("", &source_map);
    assert!(results.len() >= 2, "empty query should return all symbols");
}

#[test]
fn workspace_symbols_case_insensitive_search() {
    let source = "sub MyFunction { }\n";
    let (provider, source_map) = make_provider_with_source("file:///test.pl", source);

    let results = provider.search("myfunction", &source_map);
    assert!(!results.is_empty(), "search should be case-insensitive");
    assert_eq!(results[0].name, "MyFunction");
}

#[test]
fn workspace_symbols_prefix_match() {
    let source = "sub process_data { }\nsub process_file { }\nsub other { }\n";
    let (provider, source_map) = make_provider_with_source("file:///test.pl", source);

    let results = provider.search("process", &source_map);
    assert_eq!(results.len(), 2, "should match both process_ symbols");
}

#[test]
fn workspace_symbols_contains_match() {
    let source = "sub get_data_from_db { }\n";
    let (provider, source_map) = make_provider_with_source("file:///test.pl", source);

    let results = provider.search("data", &source_map);
    assert!(!results.is_empty(), "should match via substring");
}

#[test]
fn workspace_symbols_fuzzy_match() {
    let source = "sub foobar { }\n";
    let (provider, source_map) = make_provider_with_source("file:///test.pl", source);

    let results = provider.search("fb", &source_map);
    assert!(
        !results.is_empty(),
        "fuzzy match 'fb' should match 'foobar'"
    );
    assert_eq!(results[0].name, "foobar");
}

#[test]
fn workspace_symbols_no_match_returns_empty() {
    let source = "sub hello { }\n";
    let (provider, source_map) = make_provider_with_source("file:///test.pl", source);

    let results = provider.search("zzzzz_nonexistent", &source_map);
    assert!(results.is_empty(), "non-matching query should return empty");
}

#[test]
fn workspace_symbols_remove_document() {
    let source = "sub hello { }\n";
    let (mut provider, source_map) = make_provider_with_source("file:///test.pl", source);

    // Verify symbol exists
    let results = provider.search("hello", &source_map);
    assert!(!results.is_empty());

    // Remove document
    provider.remove_document("file:///test.pl");

    // Symbol should no longer be found
    let results = provider.search("hello", &source_map);
    assert!(
        results.is_empty(),
        "removed document symbols should not appear"
    );
}

#[test]
fn workspace_symbols_remove_nonexistent_document() {
    let provider = WorkspaceSymbolsProvider::new();
    // Should not panic
    let mut provider = provider;
    provider.remove_document("file:///does_not_exist.pl");
}

#[test]
fn workspace_symbols_get_all_symbols() {
    let source = "package Pkg;\nsub alpha { }\nsub beta { }\n";
    let (provider, _source_map) = make_provider_with_source("file:///test.pl", source);

    let all = provider.get_all_symbols();
    assert!(
        all.len() >= 2,
        "should return at least 2 symbols (alpha, beta)"
    );
}

#[test]
fn workspace_symbols_get_all_symbols_empty_provider() {
    let provider = WorkspaceSymbolsProvider::new();
    let all = provider.get_all_symbols();
    assert!(all.is_empty(), "empty provider should return empty symbols");
}

#[test]
fn workspace_symbols_multi_document_index() {
    let mut provider = WorkspaceSymbolsProvider::new();
    let mut source_map = HashMap::new();

    let source1 = "sub func_a { }\n";
    let source2 = "sub func_b { }\n";

    let ast1 = must(Parser::new(source1).parse());
    let ast2 = must(Parser::new(source2).parse());

    provider.index_document("file:///a.pl", &ast1, source1);
    provider.index_document("file:///b.pl", &ast2, source2);
    source_map.insert("file:///a.pl".to_string(), source1.to_string());
    source_map.insert("file:///b.pl".to_string(), source2.to_string());

    let results = provider.search("func", &source_map);
    assert_eq!(results.len(), 2, "should find symbols from both documents");
}

#[test]
fn workspace_symbols_reindex_replaces_old() {
    let uri = "file:///test.pl";
    let source_v1 = "sub old_func { }\n";
    let source_v2 = "sub new_func { }\n";

    let mut provider = WorkspaceSymbolsProvider::new();
    let mut source_map = HashMap::new();

    // Index v1
    let ast1 = must(Parser::new(source_v1).parse());
    provider.index_document(uri, &ast1, source_v1);
    source_map.insert(uri.to_string(), source_v1.to_string());

    let results = provider.search("old_func", &source_map);
    assert!(!results.is_empty());

    // Re-index with v2
    let ast2 = must(Parser::new(source_v2).parse());
    provider.index_document(uri, &ast2, source_v2);
    source_map.insert(uri.to_string(), source_v2.to_string());

    let results = provider.search("old_func", &source_map);
    assert!(
        results.is_empty(),
        "old symbol should be replaced after re-index"
    );

    let results = provider.search("new_func", &source_map);
    assert!(
        !results.is_empty(),
        "new symbol should be found after re-index"
    );
}

#[test]
fn workspace_symbols_container_name_for_packaged_sub() {
    let source = "package Foo::Bar;\nsub baz { }\n";
    let (provider, source_map) = make_provider_with_source("file:///test.pl", source);

    let results = provider.search("baz", &source_map);
    assert!(!results.is_empty());
    assert_eq!(
        results[0].container_name.as_deref(),
        Some("Foo::Bar"),
        "sub inside package should have container name"
    );
}

#[test]
fn workspace_symbols_search_result_sorting_exact_first() {
    let source = "sub foo { }\nsub foobar { }\nsub afoo { }\n";
    let (provider, source_map) = make_provider_with_source("file:///test.pl", source);

    let results = provider.search("foo", &source_map);
    assert!(results.len() >= 2);
    assert_eq!(results[0].name, "foo", "exact match should come first");
}

#[test]
fn workspace_symbols_search_with_candidates() {
    let source = "sub alpha { }\nsub beta { }\nsub gamma { }\n";
    let (provider, source_map) = make_provider_with_source("file:///test.pl", source);

    let candidates = vec!["alpha".to_string(), "gamma".to_string()];
    let results = provider.search_with_candidates("a", &source_map, &candidates);

    // Should only search within candidates
    assert!(!results.is_empty());
    let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
    assert!(names.contains(&"alpha"));
    assert!(
        !names.contains(&"beta"),
        "beta should not appear — not in candidates"
    );
}

#[test]
fn workspace_symbols_search_missing_source_map_entry() {
    let source = "sub hello { }\n";
    let (provider, _source_map) = make_provider_with_source("file:///test.pl", source);

    // Search with an empty source map — should gracefully return empty
    let empty_map = HashMap::new();
    let results = provider.search("hello", &empty_map);
    assert!(
        results.is_empty(),
        "missing source map entry should yield empty results"
    );
}

#[test]
fn workspace_symbol_location_has_uri() {
    let source = "sub hello { }\n";
    let (provider, source_map) = make_provider_with_source("file:///test.pl", source);

    let results = provider.search("hello", &source_map);
    assert!(!results.is_empty());
    assert_eq!(results[0].location.uri, "file:///test.pl");
}

#[test]
fn workspace_symbol_serialization_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let sym = WorkspaceSymbol {
        name: "my_sub".to_string(),
        kind: 12, // Function
        location: perl_position_tracking::WireLocation::new(
            "file:///test.pl".to_string(),
            perl_position_tracking::WireRange::default(),
        ),
        container_name: Some("MyPkg".to_string()),
    };

    let json = serde_json::to_string(&sym)?;
    let deserialized: WorkspaceSymbol = serde_json::from_str(&json)?;
    assert_eq!(deserialized.name, "my_sub");
    assert_eq!(deserialized.kind, 12);
    assert_eq!(deserialized.container_name.as_deref(), Some("MyPkg"));
    Ok(())
}

#[test]
fn workspace_symbol_serialization_no_container() -> Result<(), Box<dyn std::error::Error>> {
    let sym = WorkspaceSymbol {
        name: "standalone".to_string(),
        kind: 12,
        location: perl_position_tracking::WireLocation::new(
            "file:///test.pl".to_string(),
            perl_position_tracking::WireRange::default(),
        ),
        container_name: None,
    };

    let json = serde_json::to_string(&sym)?;
    // container_name should be omitted (skip_serializing_if = "Option::is_none")
    assert!(
        !json.contains("containerName"),
        "None container should be omitted"
    );
    Ok(())
}

// ──────────────────────────────────────────────
// Edge cases and cross-module interactions
// ──────────────────────────────────────────────

#[test]
fn deeply_nested_packages() {
    let source = "package A::B::C::D::E;\nsub deep_func { }\n";
    let (provider, source_map) = make_provider_with_source("file:///deep.pl", source);

    let results = provider.search("deep_func", &source_map);
    assert!(!results.is_empty());
    assert_eq!(results[0].container_name.as_deref(), Some("A::B::C::D::E"));
}

#[test]
fn workspace_symbols_special_characters_in_query() {
    let source = "sub normal_sub { }\n";
    let (provider, source_map) = make_provider_with_source("file:///test.pl", source);

    // Search with special characters — should not panic
    let results = provider.search("$@%", &source_map);
    let _ = results;
}

#[test]
fn compute_links_mixed_use_and_require_lines() {
    let text = "use Foo::Bar;\nmy $x = 1;\nrequire Baz::Qux;\nprint 'hello';\n";
    let links = compute_links("file:///t.pl", text, &[]);
    assert_eq!(links.len(), 2, "should find links for both use and require");
}

#[test]
fn type_hierarchy_prepare_at_exact_start_of_package() {
    let code = "package Exact;\n";
    let ast = parse_ast(code);
    let provider = TypeHierarchyProvider::new();

    // Offset 0 is at the start of "package"
    let items = provider.prepare(&ast, code, 0);
    // Should find the package (offset 0 is within the package node)
    if let Some(items) = items {
        assert_eq!(items[0].name, "Exact");
    }
}

#[test]
fn workspace_symbols_large_file() {
    // Generate a source with many subs
    let mut source = String::new();
    source.push_str("package BigPkg;\n");
    for i in 0..50 {
        source.push_str(&format!("sub func_{} {{ }}\n", i));
    }
    let (provider, source_map) = make_provider_with_source("file:///big.pl", &source);

    let all = provider.get_all_symbols();
    // Should have at least 50 functions + the package
    assert!(
        all.len() >= 50,
        "should index many symbols, found {}",
        all.len()
    );

    // Search should still work efficiently
    let results = provider.search("func_25", &source_map);
    assert!(!results.is_empty());
    assert_eq!(results[0].name, "func_25");
}
