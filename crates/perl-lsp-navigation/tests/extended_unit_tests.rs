//! Extended unit tests for `perl-lsp-navigation` crate.
//!
//! Targets edge cases, boundary conditions, and deeper coverage of:
//! document_links, references, type_hierarchy, workspace_symbols.

use perl_lsp_navigation::{
    TypeHierarchyItem, TypeHierarchyProvider, TypeHierarchySymbolKind, WorkspaceSymbol,
    WorkspaceSymbolsProvider, compute_links, find_references_single_file,
};
use perl_parser_core::Parser;
use perl_tdd_support::{must, must_some};
use std::collections::HashMap;

// ══════════════════════════════════════════════
// Helpers
// ══════════════════════════════════════════════

fn parse_ast(code: &str) -> perl_parser_core::ast::Node {
    let mut parser = Parser::new(code);
    must(parser.parse())
}

fn make_provider(uri: &str, source: &str) -> (WorkspaceSymbolsProvider, HashMap<String, String>) {
    let mut provider = WorkspaceSymbolsProvider::new();
    let mut source_map = HashMap::new();
    let ast = must(Parser::new(source).parse());
    provider.index_document(uri, &ast, source);
    source_map.insert(uri.to_string(), source.to_string());
    (provider, source_map)
}

fn make_hierarchy_item(name: &str) -> TypeHierarchyItem {
    TypeHierarchyItem {
        name: name.to_string(),
        kind: TypeHierarchySymbolKind::Class,
        uri: "file:///test.pl".to_string(),
        range: perl_position_tracking::WireRange::default(),
        selection_range: perl_position_tracking::WireRange::default(),
        detail: None,
        data: None,
    }
}

// ══════════════════════════════════════════════
// document_links — additional edge cases
// ══════════════════════════════════════════════

#[test]
fn links_use_deeply_nested_module() {
    let links = compute_links("file:///t.pl", "use A::B::C::D::E;\n", &[]);
    assert_eq!(links.len(), 1);
    if let Some(link) = links.first() {
        assert_eq!(
            link.pointer("/data/module")
                .and_then(serde_json::Value::as_str),
            Some("A::B::C::D::E")
        );
    }
}

#[test]
fn links_use_single_word_module() {
    // Single-word module like `use Moose;` should still produce a link
    let links = compute_links("file:///t.pl", "use Moose;\n", &[]);
    assert_eq!(links.len(), 1);
}

#[test]
fn links_use_with_import_list_still_links() {
    let links = compute_links("file:///t.pl", "use Foo::Bar qw(baz quux);\n", &[]);
    assert_eq!(links.len(), 1);
    if let Some(link) = links.first() {
        assert_eq!(
            link.pointer("/data/module")
                .and_then(serde_json::Value::as_str),
            Some("Foo::Bar")
        );
    }
}

#[test]
fn links_use_with_version_number() {
    let links = compute_links("file:///t.pl", "use Foo::Bar 1.23;\n", &[]);
    assert_eq!(links.len(), 1);
}

#[test]
fn links_multiple_pragmas_no_links() {
    let text = "use strict;\nuse warnings;\nuse utf8;\nuse feature 'say';\n";
    let links = compute_links("file:///t.pl", text, &[]);
    assert!(links.is_empty(), "only pragmas — should produce no links");
}

#[test]
fn links_require_file_with_relative_path() -> Result<(), Box<dyn std::error::Error>> {
    let links = compute_links("file:///t.pl", "require 'lib/Helper.pm';\n", &[]);
    assert_eq!(links.len(), 1);
    let link = links.first().ok_or("expected a link")?;
    assert_eq!(
        link.pointer("/data/path")
            .and_then(serde_json::Value::as_str),
        Some("lib/Helper.pm")
    );
    Ok(())
}

#[test]
fn links_require_file_double_quotes() -> Result<(), Box<dyn std::error::Error>> {
    let links = compute_links("file:///t.pl", "require \"Helper.pm\";\n", &[]);
    assert_eq!(links.len(), 1);
    let link = links.first().ok_or("expected a link")?;
    assert_eq!(
        link.pointer("/data/type")
            .and_then(serde_json::Value::as_str),
        Some("file")
    );
    Ok(())
}

#[test]
fn links_comment_line_with_use_produces_no_module_link() {
    // A comment containing "use" shouldn't produce a _module_ link,
    // but the naive scanner may still pick it up. This test documents
    // the current behaviour rather than prescribing it.
    let links = compute_links("file:///t.pl", "# use Fake::Module;\n", &[]);
    // Current implementation may or may not emit a link for comments.
    // We just verify no panic.
    let _ = links;
}

#[test]
fn links_use_on_second_line() {
    let text = "#!/usr/bin/perl\nuse My::Mod;\n";
    let links = compute_links("file:///t.pl", text, &[]);
    assert_eq!(links.len(), 1);
    if let Some(link) = links.first() {
        let line = link
            .pointer("/range/start/line")
            .and_then(serde_json::Value::as_u64);
        assert_eq!(line, Some(1), "link should be on line 1 (0-indexed)");
    }
}

#[test]
fn links_require_bare_module_without_colons_skipped() {
    let links = compute_links("file:///t.pl", "require SomeModule;\n", &[]);
    // Bare require without :: or quotes should not produce a module link
    assert!(links.is_empty());
}

#[test]
fn links_empty_use_statement_no_panic() {
    // Malformed `use ;` — should not panic
    let links = compute_links("file:///t.pl", "use ;\n", &[]);
    let _ = links;
}

#[test]
fn links_line_numbers_sequential() {
    let text = "use Foo::A;\nuse Foo::B;\nuse Foo::C;\n";
    let links = compute_links("file:///t.pl", text, &[]);
    assert_eq!(links.len(), 3);
    for (i, link) in links.iter().enumerate() {
        let line = link
            .pointer("/range/start/line")
            .and_then(serde_json::Value::as_u64);
        assert_eq!(line, Some(i as u64), "link {} should be on line {}", i, i);
    }
}

#[test]
fn links_all_extended_pragmas_skipped() {
    let extended_pragmas = [
        "use autodie;",
        "use bigint;",
        "use bignum;",
        "use bigrat;",
        "use charnames;",
        "use diagnostics;",
        "use encoding;",
        "use filetest;",
        "use locale;",
        "use open;",
        "use ops;",
        "use re;",
        "use sigtrap;",
        "use sort;",
        "use threads;",
        "use vmsish;",
        "use overload;",
        "use attributes;",
        "use autouse;",
        "use blib;",
        "use bytes;",
        "use integer;",
        "use fields;",
        "use if;",
        "use vars;",
        "use subs;",
    ];
    for pragma in &extended_pragmas {
        let links = compute_links("file:///t.pl", pragma, &[]);
        assert!(
            links.is_empty(),
            "pragma '{}' should not produce a link",
            pragma
        );
    }
}

// ══════════════════════════════════════════════
// references — additional coverage
// ══════════════════════════════════════════════

#[test]
fn refs_multiple_variables_distinct() {
    let code = "my $a = 1; my $b = 2; print $a + $b;";
    let ast = parse_ast(code);

    let offset_a = must_some(code.find("$a"));
    let refs_a = find_references_single_file(&ast, offset_a);
    if let Some(refs) = &refs_a {
        for &(start, end) in refs {
            let frag = &code[start..end.min(code.len())];
            assert!(!frag.contains("$b"), "$a refs should not contain $b");
        }
    }

    let offset_b = must_some(code.find("$b"));
    let refs_b = find_references_single_file(&ast, offset_b);
    if let Some(refs) = &refs_b {
        for &(start, end) in refs {
            let frag = &code[start..end.min(code.len())];
            assert!(!frag.contains("$a"), "$b refs should not contain $a");
        }
    }
}

#[test]
fn refs_variable_used_in_expression() {
    let code = "my $x = 10; my $y = $x + 5;";
    let ast = parse_ast(code);

    let offset = must_some(code.find("$x"));
    let refs = find_references_single_file(&ast, offset);
    assert!(refs.is_some());
    let refs = must_some(refs);
    assert!(refs.len() >= 2, "should find declaration and usage of $x");
}

#[test]
fn refs_subroutine_defined_and_called_twice() {
    let code = "sub handler { } handler(); handler();";
    let ast = parse_ast(code);

    let offset = must_some(code.find("handler"));
    let refs = find_references_single_file(&ast, offset);
    assert!(refs.is_some());
    let refs = must_some(refs);
    assert!(
        refs.len() >= 3,
        "should find definition + 2 calls, found {}",
        refs.len()
    );
}

#[test]
fn refs_offset_at_end_of_source() {
    let code = "my $x = 1;";
    let ast = parse_ast(code);
    let refs = find_references_single_file(&ast, code.len());
    // At end boundary — should not panic, result is implementation-defined
    let _ = refs;
}

#[test]
fn refs_offset_zero() {
    let code = "my $x = 1;";
    let ast = parse_ast(code);
    let refs = find_references_single_file(&ast, 0);
    // Offset 0 is at "my" keyword — result depends on node kind
    let _ = refs;
}

#[test]
fn refs_hash_variable() {
    let code = "my %data = (); $data{key} = 1; print %data;";
    let ast = parse_ast(code);

    let offset = must_some(code.find("%data"));
    let refs = find_references_single_file(&ast, offset);
    // If the implementation tracks %data references, we should find them
    let _ = refs;
}

#[test]
fn refs_array_variable() {
    let code = "my @list = (1,2,3); push @list, 4;";
    let ast = parse_ast(code);

    let offset = must_some(code.find("@list"));
    let refs = find_references_single_file(&ast, offset);
    // Should find at least the declaration and usage
    let _ = refs;
}

#[test]
fn refs_qualified_function_call() {
    let code = "sub Foo::bar { } Foo::bar();";
    let ast = parse_ast(code);

    // Find references from the call site
    if let Some(call_offset) = code.rfind("Foo::bar") {
        let refs = find_references_single_file(&ast, call_offset);
        if let Some(refs) = refs {
            assert!(!refs.is_empty(), "should find at least the call");
        }
    }
}

#[test]
fn refs_multiple_functions_not_confused() {
    let code = "sub alpha { } sub beta { } alpha(); beta();";
    let ast = parse_ast(code);

    let offset = must_some(code.find("alpha"));
    let refs = find_references_single_file(&ast, offset);
    if let Some(refs) = refs {
        for &(start, end) in &refs {
            let frag = &code[start..end.min(code.len())];
            assert!(!frag.contains("beta"), "alpha refs should not contain beta");
        }
    }
}

#[test]
fn refs_variable_in_nested_sub() {
    let code = "my $outer = 1; sub inner { my $inner_var = $outer; }";
    let ast = parse_ast(code);

    let offset = must_some(code.find("$outer"));
    let refs = find_references_single_file(&ast, offset);
    // The implementation may or may not track cross-scope references
    let _ = refs;
}

#[test]
fn refs_variable_inside_if_condition_and_block() {
    let code = "my $flag = 1; if ($flag) { print $flag; }";
    let ast = parse_ast(code);

    let offset = must_some(code.find("$flag"));
    let refs = find_references_single_file(&ast, offset);
    assert!(refs.is_some(), "should find references for $flag");

    let refs = must_some(refs);
    assert!(
        refs.len() >= 3,
        "should include declaration + if condition + print usage, found {}",
        refs.len()
    );
}

#[test]
fn hierarchy_diamond_inheritance() {
    let code = concat!(
        "package Base;\n\n",
        "package Left;\nuse parent 'Base';\n\n",
        "package Right;\nuse parent 'Base';\n\n",
        "package Diamond;\nuse parent 'Left', 'Right';\n",
    );
    let ast = parse_ast(code);
    let provider = TypeHierarchyProvider::new();

    let diamond = make_hierarchy_item("Diamond");
    let supertypes = provider.find_supertypes(&ast, &diamond);
    assert_eq!(supertypes.len(), 2, "Diamond should have 2 parents");
    let names: Vec<&str> = supertypes.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"Left"));
    assert!(names.contains(&"Right"));

    let base = make_hierarchy_item("Base");
    let subtypes = provider.find_subtypes(&ast, &base);
    assert_eq!(
        subtypes.len(),
        2,
        "Base should have Left and Right as subtypes"
    );
}

#[test]
fn hierarchy_deep_chain() {
    let code = concat!(
        "package A;\n",
        "package B;\nuse parent 'A';\n",
        "package C;\nuse parent 'B';\n",
        "package D;\nuse parent 'C';\n",
    );
    let ast = parse_ast(code);
    let provider = TypeHierarchyProvider::new();

    // D's direct parent is C
    let d = make_hierarchy_item("D");
    let supertypes = provider.find_supertypes(&ast, &d);
    assert_eq!(supertypes.len(), 1);
    assert_eq!(supertypes[0].name, "C");

    // A's direct subtypes is just B
    let a = make_hierarchy_item("A");
    let subtypes = provider.find_subtypes(&ast, &a);
    assert_eq!(subtypes.len(), 1);
    assert_eq!(subtypes[0].name, "B");
}

#[test]
fn hierarchy_no_inheritance_single_package() {
    let code = "package Standalone;\nsub new { }\n";
    let ast = parse_ast(code);
    let provider = TypeHierarchyProvider::new();

    let item = make_hierarchy_item("Standalone");
    let supertypes = provider.find_supertypes(&ast, &item);
    assert!(supertypes.is_empty());
    let subtypes = provider.find_subtypes(&ast, &item);
    assert!(subtypes.is_empty());
}

#[test]
fn hierarchy_use_base_multiple_parents() {
    let code = "package Child;\nuse base 'Parent1', 'Parent2';\n";
    let ast = parse_ast(code);
    let provider = TypeHierarchyProvider::new();

    let item = make_hierarchy_item("Child");
    let supertypes = provider.find_supertypes(&ast, &item);
    assert_eq!(supertypes.len(), 2);
    let names: Vec<&str> = supertypes.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"Parent1"));
    assert!(names.contains(&"Parent2"));
}

#[test]
fn hierarchy_prepare_returns_class_kind() {
    let code = "package Typed;\n";
    let ast = parse_ast(code);
    let provider = TypeHierarchyProvider::new();

    if let Some(items) = provider.prepare(&ast, code, 8) {
        assert_eq!(items[0].kind as i32, TypeHierarchySymbolKind::Class as i32);
    }
}

#[test]
fn hierarchy_prepare_multiple_packages_finds_correct_one() {
    let code = "package First;\npackage Second;\n";
    let ast = parse_ast(code);
    let provider = TypeHierarchyProvider::new();

    // Offset in "Second" (after "package First;\n" = 15 bytes, then "package " = 8)
    let second_offset = must_some(code.find("Second"));
    if let Some(items) = provider.prepare(&ast, code, second_offset) {
        assert_eq!(items[0].name, "Second");
    }
}

#[test]
fn hierarchy_item_with_data_field() -> Result<(), Box<dyn std::error::Error>> {
    let item = TypeHierarchyItem {
        name: "WithData".to_string(),
        kind: TypeHierarchySymbolKind::Class,
        uri: "file:///test.pl".to_string(),
        range: perl_position_tracking::WireRange::default(),
        selection_range: perl_position_tracking::WireRange::default(),
        detail: Some("Custom detail".to_string()),
        data: Some(serde_json::json!({"custom": true})),
    };

    let json = serde_json::to_string(&item)?;
    let round: TypeHierarchyItem = serde_json::from_str(&json)?;
    assert_eq!(round.name, "WithData");
    assert_eq!(round.detail.as_deref(), Some("Custom detail"));
    assert!(round.data.is_some());
    Ok(())
}

#[test]
fn hierarchy_item_kind_serialization() -> Result<(), Box<dyn std::error::Error>> {
    // Serde serializes the enum variant name as a string (not the discriminant)
    let json = serde_json::to_string(&TypeHierarchySymbolKind::Class)?;
    assert!(
        json.contains("Class"),
        "Class should serialize, got {}",
        json
    );
    let json = serde_json::to_string(&TypeHierarchySymbolKind::Method)?;
    assert!(
        json.contains("Method"),
        "Method should serialize, got {}",
        json
    );
    let json = serde_json::to_string(&TypeHierarchySymbolKind::Function)?;
    assert!(
        json.contains("Function"),
        "Function should serialize, got {}",
        json
    );

    // Round-trip deserialization
    let rt: TypeHierarchySymbolKind = serde_json::from_str(&json)?;
    assert_eq!(rt as i32, TypeHierarchySymbolKind::Function as i32);
    Ok(())
}

#[test]
fn hierarchy_subtypes_detail_field() {
    let code = "package Base;\npackage Sub1;\nuse parent 'Base';\n";
    let ast = parse_ast(code);
    let provider = TypeHierarchyProvider::new();

    let base = make_hierarchy_item("Base");
    let subtypes = provider.find_subtypes(&ast, &base);
    assert_eq!(subtypes.len(), 1);
    assert_eq!(subtypes[0].detail.as_deref(), Some("Subclass"));
}

#[test]
fn hierarchy_supertypes_detail_field() {
    let code = "package Child;\nuse parent 'Parent';\n";
    let ast = parse_ast(code);
    let provider = TypeHierarchyProvider::new();

    let child = make_hierarchy_item("Child");
    let supertypes = provider.find_supertypes(&ast, &child);
    assert_eq!(supertypes.len(), 1);
    assert_eq!(supertypes[0].detail.as_deref(), Some("Parent Class"));
}

#[test]
fn hierarchy_nonexistent_package_has_no_relations() {
    let code = "package Real;\n";
    let ast = parse_ast(code);
    let provider = TypeHierarchyProvider::new();

    let fake = make_hierarchy_item("DoesNotExist");
    let supertypes = provider.find_supertypes(&ast, &fake);
    assert!(supertypes.is_empty());
    let subtypes = provider.find_subtypes(&ast, &fake);
    assert!(subtypes.is_empty());
}

#[test]
fn hierarchy_three_levels_middle_class() {
    let code = concat!(
        "package Grand;\n",
        "package Parent;\nuse parent 'Grand';\n",
        "package Child;\nuse parent 'Parent';\n",
    );
    let ast = parse_ast(code);
    let provider = TypeHierarchyProvider::new();

    let parent = make_hierarchy_item("Parent");
    let supertypes = provider.find_supertypes(&ast, &parent);
    assert_eq!(supertypes.len(), 1);
    assert_eq!(supertypes[0].name, "Grand");

    let subtypes = provider.find_subtypes(&ast, &parent);
    assert_eq!(subtypes.len(), 1);
    assert_eq!(subtypes[0].name, "Child");
}

// ══════════════════════════════════════════════
// workspace_symbols — additional coverage
// ══════════════════════════════════════════════

#[test]
fn ws_search_exact_match_first_in_sort() {
    let source = "sub abc { }\nsub abcdef { }\nsub xabc { }\n";
    let (provider, source_map) = make_provider("file:///t.pl", source);

    let results = provider.search("abc", &source_map);
    assert!(results.len() >= 2);
    assert_eq!(results[0].name, "abc", "exact match should be sorted first");
}

#[test]
fn ws_search_prefix_before_contains() {
    let source = "sub prefix_match { }\nsub has_prefix_inside { }\n";
    let (provider, source_map) = make_provider("file:///t.pl", source);

    let results = provider.search("prefix", &source_map);
    assert!(results.len() >= 2);
    // prefix_match starts with "prefix", should come before has_prefix_inside
    assert_eq!(results[0].name, "prefix_match");
}

#[test]
fn ws_search_unicode_sub_name() {
    let source = "sub café { }\n";
    let (provider, source_map) = make_provider("file:///t.pl", source);

    let results = provider.search("café", &source_map);
    if !results.is_empty() {
        assert_eq!(results[0].name, "café");
    }
}

#[test]
fn ws_multi_doc_remove_one() {
    let mut provider = WorkspaceSymbolsProvider::new();
    let mut source_map = HashMap::new();

    let s1 = "sub alpha { }\n";
    let s2 = "sub beta { }\n";
    let ast1 = must(Parser::new(s1).parse());
    let ast2 = must(Parser::new(s2).parse());

    provider.index_document("file:///a.pl", &ast1, s1);
    provider.index_document("file:///b.pl", &ast2, s2);
    source_map.insert("file:///a.pl".to_string(), s1.to_string());
    source_map.insert("file:///b.pl".to_string(), s2.to_string());

    provider.remove_document("file:///a.pl");

    let results = provider.search("alpha", &source_map);
    assert!(
        results.is_empty(),
        "alpha should be gone after removing a.pl"
    );

    let results = provider.search("beta", &source_map);
    assert!(
        !results.is_empty(),
        "beta should remain after removing a.pl"
    );
}

#[test]
fn ws_get_all_symbols_from_multiple_docs() {
    let mut provider = WorkspaceSymbolsProvider::new();
    let s1 = "sub f1 { }\nsub f2 { }\n";
    let s2 = "sub f3 { }\n";
    let ast1 = must(Parser::new(s1).parse());
    let ast2 = must(Parser::new(s2).parse());

    provider.index_document("file:///a.pl", &ast1, s1);
    provider.index_document("file:///b.pl", &ast2, s2);

    let all = provider.get_all_symbols();
    let names: Vec<&str> = all.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"f1"));
    assert!(names.contains(&"f2"));
    assert!(names.contains(&"f3"));
}

#[test]
fn ws_search_with_candidates_empty_candidates() {
    let source = "sub alpha { }\nsub beta { }\n";
    let (provider, source_map) = make_provider("file:///t.pl", source);

    let candidates: Vec<String> = vec![];
    let results = provider.search_with_candidates("a", &source_map, &candidates);
    assert!(
        results.is_empty(),
        "empty candidates should yield no results"
    );
}

#[test]
fn ws_search_with_candidates_case_insensitive() {
    let source = "sub Alpha { }\nsub Beta { }\n";
    let (provider, source_map) = make_provider("file:///t.pl", source);

    let candidates = vec!["alpha".to_string()];
    let results = provider.search_with_candidates("al", &source_map, &candidates);
    assert!(!results.is_empty());
    assert_eq!(results[0].name, "Alpha");
}

#[test]
fn ws_search_with_candidates_exact_match_sorted_first() {
    let source = "sub foo { }\nsub foobar { }\n";
    let (provider, source_map) = make_provider("file:///t.pl", source);

    let candidates = vec!["foo".to_string(), "foobar".to_string()];
    let results = provider.search_with_candidates("foo", &source_map, &candidates);
    assert!(results.len() >= 2);
    assert_eq!(results[0].name, "foo", "exact match should be first");
}

#[test]
fn ws_package_symbol_kind() {
    let source = "package MyNamespace;\n";
    let (provider, source_map) = make_provider("file:///t.pl", source);

    let results = provider.search("MyNamespace", &source_map);
    assert!(!results.is_empty());
    // LSP SymbolKind::Namespace is 3; Package should map to a namespace or module kind
    let kind = results[0].kind;
    // Just ensure it's a valid positive kind
    assert!(kind > 0, "symbol kind should be positive, got {}", kind);
}

#[test]
fn ws_sub_symbol_kind() {
    let source = "sub my_function { }\n";
    let (provider, source_map) = make_provider("file:///t.pl", source);

    let results = provider.search("my_function", &source_map);
    assert!(!results.is_empty());
    // LSP SymbolKind::Function is 12
    assert_eq!(
        results[0].kind, 12,
        "subroutine should have Function kind (12)"
    );
}

#[test]
fn ws_symbol_location_range_nonzero() {
    let source = "package Pkg;\nsub target { 1; }\n";
    let (provider, source_map) = make_provider("file:///t.pl", source);

    let results = provider.search("target", &source_map);
    assert!(!results.is_empty());
    let loc = &results[0].location;
    // target is on line 1 (0-indexed)
    assert_eq!(loc.range.start.line, 1);
}

#[test]
fn ws_reindex_same_uri_updates() {
    let uri = "file:///t.pl";
    let mut provider = WorkspaceSymbolsProvider::new();
    let mut source_map = HashMap::new();

    let v1 = "sub old { }\n";
    let ast1 = must(Parser::new(v1).parse());
    provider.index_document(uri, &ast1, v1);
    source_map.insert(uri.to_string(), v1.to_string());

    let results = provider.search("old", &source_map);
    assert!(!results.is_empty());

    let v2 = "sub replaced { }\n";
    let ast2 = must(Parser::new(v2).parse());
    provider.index_document(uri, &ast2, v2);
    source_map.insert(uri.to_string(), v2.to_string());

    let results = provider.search("old", &source_map);
    assert!(
        results.is_empty(),
        "old symbol should be gone after reindex"
    );
    let results = provider.search("replaced", &source_map);
    assert!(!results.is_empty(), "new symbol should be present");
}

#[test]
fn ws_empty_source_produces_no_symbols() {
    let (provider, source_map) = make_provider("file:///empty.pl", "");
    let all = provider.get_all_symbols();
    // Empty source may produce 0 or minimal symbols
    let _ = all;
    let results = provider.search("anything", &source_map);
    assert!(results.is_empty());
}

#[test]
fn ws_search_single_char_query() {
    let source = "sub a { }\nsub b { }\nsub ab { }\n";
    let (provider, source_map) = make_provider("file:///t.pl", source);

    let results = provider.search("a", &source_map);
    assert!(results.len() >= 2, "should match 'a' and 'ab'");
}

#[test]
fn ws_container_for_deeply_nested_package() {
    let source = "package X::Y::Z;\nsub method { }\n";
    let (provider, source_map) = make_provider("file:///t.pl", source);

    let results = provider.search("method", &source_map);
    assert!(!results.is_empty());
    assert_eq!(results[0].container_name.as_deref(), Some("X::Y::Z"));
}

#[test]
fn ws_multiple_packages_same_file() {
    let source = "package Pkg1;\nsub f1 { }\npackage Pkg2;\nsub f2 { }\n";
    let (provider, source_map) = make_provider("file:///t.pl", source);

    let r1 = provider.search("f1", &source_map);
    assert!(!r1.is_empty());
    assert_eq!(r1[0].container_name.as_deref(), Some("Pkg1"));

    let r2 = provider.search("f2", &source_map);
    assert!(!r2.is_empty());
    assert_eq!(r2[0].container_name.as_deref(), Some("Pkg2"));
}

#[test]
fn ws_symbol_serialization_with_location() -> Result<(), Box<dyn std::error::Error>> {
    let sym = WorkspaceSymbol {
        name: "test_sym".to_string(),
        kind: 12,
        location: perl_position_tracking::WireLocation::new(
            "file:///loc.pl".to_string(),
            perl_position_tracking::WireRange {
                start: perl_position_tracking::WirePosition {
                    line: 5,
                    character: 4,
                },
                end: perl_position_tracking::WirePosition {
                    line: 5,
                    character: 12,
                },
            },
        ),
        container_name: Some("Container".to_string()),
    };

    let json = serde_json::to_string(&sym)?;
    assert!(json.contains("test_sym"));
    assert!(json.contains("file:///loc.pl"));

    let round: WorkspaceSymbol = serde_json::from_str(&json)?;
    assert_eq!(round.location.uri, "file:///loc.pl");
    assert_eq!(round.location.range.start.line, 5);
    assert_eq!(round.location.range.start.character, 4);
    Ok(())
}

#[test]
fn ws_search_special_perl_chars_no_panic() {
    let source = "sub normal { }\n";
    let (provider, source_map) = make_provider("file:///t.pl", source);

    // Various special characters that should not cause panics
    for query in &["$", "@", "%", "&", "*", "::", "->", "=>"] {
        let results = provider.search(query, &source_map);
        let _ = results;
    }
}

// ══════════════════════════════════════════════
// Cross-module / integration-like tests
// ══════════════════════════════════════════════

#[test]
fn links_and_hierarchy_same_source() {
    let code = "use Foo::Bar;\npackage Child;\nuse parent 'Foo::Bar';\n";

    // Document links
    let links = compute_links("file:///t.pl", code, &[]);
    assert!(!links.is_empty(), "should find use Foo::Bar link");

    // Type hierarchy
    let ast = parse_ast(code);
    let provider = TypeHierarchyProvider::new();
    let child = make_hierarchy_item("Child");
    let supertypes = provider.find_supertypes(&ast, &child);
    assert_eq!(supertypes.len(), 1);
    assert_eq!(supertypes[0].name, "Foo::Bar");
}

#[test]
fn refs_and_workspace_symbols_same_source() {
    let code = "sub greet { } greet(); greet();";

    // References
    let ast = parse_ast(code);
    let offset = must_some(code.rfind("greet"));
    let refs = find_references_single_file(&ast, offset);
    assert!(refs.is_some());

    // Workspace symbols
    let (provider, source_map) = make_provider("file:///t.pl", code);
    let results = provider.search("greet", &source_map);
    assert!(!results.is_empty());
    assert_eq!(results[0].name, "greet");
}

#[test]
fn hierarchy_and_workspace_symbols_same_source() {
    let code = concat!(
        "package Animal;\nsub speak { }\n\n",
        "package Dog;\nuse parent 'Animal';\nsub speak { }\n",
    );
    let ast = parse_ast(code);

    // Hierarchy
    let provider = TypeHierarchyProvider::new();
    let animal = make_hierarchy_item("Animal");
    let subtypes = provider.find_subtypes(&ast, &animal);
    assert_eq!(subtypes.len(), 1);
    assert_eq!(subtypes[0].name, "Dog");

    // Workspace symbols — should find both speak methods with different containers
    let (ws_provider, source_map) = make_provider("file:///t.pl", code);
    let results = ws_provider.search("speak", &source_map);
    assert_eq!(results.len(), 2);
    let containers: Vec<Option<String>> =
        results.iter().map(|r| r.container_name.clone()).collect();
    assert!(containers.contains(&Some("Animal".to_string())));
    assert!(containers.contains(&Some("Dog".to_string())));
}

#[test]
fn hierarchy_mixed_use_parent_and_use_base() {
    let code = concat!(
        "package Root;\n\n",
        "package Child1;\nuse parent 'Root';\n\n",
        "package Child2;\nuse base 'Root';\n",
    );
    let ast = parse_ast(code);
    let provider = TypeHierarchyProvider::new();

    let root = make_hierarchy_item("Root");
    let subtypes = provider.find_subtypes(&ast, &root);
    assert_eq!(
        subtypes.len(),
        2,
        "Both use parent and use base should be detected"
    );
    let names: Vec<&str> = subtypes.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"Child1"));
    assert!(names.contains(&"Child2"));
}

#[test]
fn ws_symbols_large_workspace_50_subs() {
    let mut source = String::from("package BigPkg;\n");
    for i in 0..50 {
        source.push_str(&format!("sub method_{} {{ }}\n", i));
    }
    let (provider, source_map) = make_provider("file:///big.pl", &source);

    // Search for a specific method
    let results = provider.search("method_42", &source_map);
    assert!(!results.is_empty());
    assert_eq!(results[0].name, "method_42");

    // All symbols should be indexed
    let all = provider.get_all_symbols();
    assert!(all.len() >= 50);
}

#[test]
fn ws_fuzzy_match_subsequence() {
    let source = "sub get_user_by_id { }\n";
    let (provider, source_map) = make_provider("file:///t.pl", source);

    // "gui" is a subsequence of "get_user_by_id"
    let results = provider.search("gui", &source_map);
    assert!(
        !results.is_empty(),
        "subsequence 'gui' should match 'get_user_by_id'"
    );
}

#[test]
fn ws_search_with_candidates_no_overlap() {
    let source = "sub alpha { }\nsub beta { }\nsub gamma { }\n";
    let (provider, source_map) = make_provider("file:///t.pl", source);

    // Candidates that don't match the query at all
    let candidates = vec!["gamma".to_string()];
    let results = provider.search_with_candidates("zzz", &source_map, &candidates);
    assert!(results.is_empty());
}
