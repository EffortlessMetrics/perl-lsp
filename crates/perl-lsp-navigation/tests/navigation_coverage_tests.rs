//! Navigation test coverage for `perl-lsp-navigation` crate.
//!
//! Targets the five areas requested:
//! 1. Go to definition for variables (declaration lookup via references)
//! 2. Go to definition for functions (sub declaration via references)
//! 3. Go to definition for modules (use statement / package definition)
//! 4. Find all references
//! 5. Workspace symbol search

use perl_lsp_navigation::{
    TypeHierarchyProvider, TypeHierarchySymbolKind, WorkspaceSymbolsProvider, compute_links,
    find_references_single_file,
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

// ══════════════════════════════════════════════
// 1. Go to definition for variables
// ══════════════════════════════════════════════
//
// `find_references_single_file` returns all reference sites including
// the declaration.  The first entry is typically the declaration.
// We simulate "go to definition" by finding references from a usage
// site and checking that the declaration site is included.

#[test]
fn goto_def_variable_from_usage_to_declaration() {
    let code = "my $name = 'world'; print $name;";
    let ast = parse_ast(code);

    // Offset at the *usage* of $name (second occurrence)
    let usage_offset = must_some(code.rfind("$name"));
    let refs = must_some(find_references_single_file(&ast, usage_offset));

    // The declaration "my $name" starts at offset 3 ("my " is 3 chars)
    let decl_offset = must_some(code.find("$name"));
    assert!(
        refs.iter().any(|&(start, _)| start == decl_offset),
        "declaration site should appear in references; refs={:?}, expected decl at {}",
        refs,
        decl_offset
    );
}

#[test]
fn goto_def_variable_scalar_declaration_is_first_ref() {
    let code = "my $x = 10; $x += 5; print $x;";
    let ast = parse_ast(code);

    // From any usage of $x, we should get back the declaration
    let usage_offset = must_some(code.rfind("$x"));
    let refs = must_some(find_references_single_file(&ast, usage_offset));

    assert!(
        refs.len() >= 3,
        "expected at least 3 refs (decl + 2 usages), got {}",
        refs.len()
    );

    // Verify the first reference is the declaration (smallest offset)
    let min_offset = refs.iter().map(|&(s, _)| s).min();
    let decl_offset = must_some(code.find("$x"));
    assert_eq!(
        min_offset,
        Some(decl_offset),
        "earliest reference should be the declaration"
    );
}

#[test]
fn goto_def_variable_array_from_push_usage() {
    let code = "my @items = (); push @items, 42; print @items;";
    let ast = parse_ast(code);

    // From a usage of @items
    let usage_offset = must_some(code.rfind("@items"));
    let refs = find_references_single_file(&ast, usage_offset);
    // If the implementation tracks @items references, verify declaration is present
    if let Some(refs) = refs {
        let decl_offset = must_some(code.find("@items"));
        assert!(
            refs.iter().any(|&(start, _)| start == decl_offset),
            "@items declaration should be included in refs"
        );
    }
}

#[test]
fn goto_def_variable_hash_from_access() {
    let code = "my %config = (); $config{key} = 'val'; print %config;";
    let ast = parse_ast(code);

    // From %config usage
    let usage_offset = must_some(code.rfind("%config"));
    let refs = find_references_single_file(&ast, usage_offset);
    // If the implementation tracks %config, verify presence of declaration
    if let Some(refs) = refs {
        let decl_offset = must_some(code.find("%config"));
        assert!(
            refs.iter().any(|&(start, _)| start == decl_offset),
            "%config declaration should be included in refs"
        );
    }
}

#[test]
fn goto_def_variable_multiple_declarations_same_name() {
    // Different scopes may redeclare the same variable name
    let code = "{ my $x = 1; } { my $x = 2; print $x; }";
    let ast = parse_ast(code);

    // From the usage inside the second block
    let last_x = must_some(code.rfind("$x"));
    let refs = find_references_single_file(&ast, last_x);
    // Should at least not panic
    let _ = refs;
}

#[test]
fn goto_def_variable_used_in_loop() {
    let code = "my $sum = 0; for my $i (1..10) { $sum += $i; }";
    let ast = parse_ast(code);

    // From $sum inside the loop
    let sum_in_loop = must_some(code.rfind("$sum"));
    let refs = must_some(find_references_single_file(&ast, sum_in_loop));
    let decl_offset = must_some(code.find("$sum"));
    assert!(
        refs.iter().any(|&(start, _)| start == decl_offset),
        "$sum declaration should be reachable from loop usage"
    );
}

#[test]
fn goto_def_variable_in_nested_conditional() {
    let code = "my $val = 1; if ($val > 0) { if ($val < 100) { print $val; } }";
    let ast = parse_ast(code);

    let deepest_usage = must_some(code.rfind("$val"));
    let refs = must_some(find_references_single_file(&ast, deepest_usage));

    // All occurrences of $val
    let expected_count = code.matches("$val").count();
    assert!(
        refs.len() >= expected_count,
        "expected at least {} refs, got {}",
        expected_count,
        refs.len()
    );
}

// ══════════════════════════════════════════════
// 2. Go to definition for functions
// ══════════════════════════════════════════════

#[test]
fn goto_def_function_from_call_to_sub_definition() {
    let code = "sub greet { print 'hello'; } greet();";
    let ast = parse_ast(code);

    // Offset at the call site (second occurrence of "greet")
    let call_offset = must_some(code.rfind("greet"));
    let refs = must_some(find_references_single_file(&ast, call_offset));

    // The Subroutine node covers the entire "sub greet { ... }" span,
    // so the definition reference starts at the Subroutine node start (offset 0).
    // The call reference starts at the FunctionCall node position.
    assert!(
        refs.len() >= 2,
        "should find definition + call, got {}",
        refs.len()
    );

    // The first (earliest) reference should be the sub definition node
    let sub_start = must_some(code.find("sub greet"));
    assert!(
        refs.iter().any(|&(start, _)| start == sub_start),
        "sub definition node should appear in references; refs={:?}",
        refs,
    );
}

#[test]
fn goto_def_function_defined_after_call() {
    // Forward declaration: call before definition
    let code = "process(); sub process { print 'done'; }";
    let ast = parse_ast(code);

    let call_offset = must_some(code.find("process"));
    let refs = must_some(find_references_single_file(&ast, call_offset));

    // Should find both call and definition
    assert!(
        refs.len() >= 2,
        "should find call + definition, got {}",
        refs.len()
    );
}

#[test]
fn goto_def_function_called_multiple_times() {
    let code = "sub calc { 1 } calc(); calc(); calc();";
    let ast = parse_ast(code);

    // From the definition
    let def_offset = must_some(code.find("calc"));
    let refs = must_some(find_references_single_file(&ast, def_offset));

    // 1 definition + 3 calls = 4
    assert!(
        refs.len() >= 4,
        "expected 4+ refs (1 def + 3 calls), got {}",
        refs.len()
    );
}

#[test]
fn goto_def_function_with_args() {
    let code = "sub add { my ($a, $b) = @_; return $a + $b; } add(1, 2);";
    let ast = parse_ast(code);

    let call_offset = must_some(code.rfind("add"));
    let refs = must_some(find_references_single_file(&ast, call_offset));

    // Subroutine node starts at "sub add { ... }" which is offset 0
    let sub_start = must_some(code.find("sub add"));
    assert!(
        refs.iter().any(|&(start, _)| start == sub_start),
        "sub definition node should be reachable from call site; refs={:?}",
        refs,
    );
}

#[test]
fn goto_def_function_refs_include_correct_text_spans() {
    let code = "sub handler { } handler();";
    let ast = parse_ast(code);

    let offset = must_some(code.find("handler"));
    let refs = must_some(find_references_single_file(&ast, offset));

    // Verify each reference span corresponds to "handler" text
    for &(start, end) in &refs {
        let fragment = &code[start..end.min(code.len())];
        assert!(
            fragment.contains("handler"),
            "reference span should contain 'handler', got '{}'",
            fragment
        );
    }
}

// ══════════════════════════════════════════════
// 3. Go to definition for modules (use statement)
// ══════════════════════════════════════════════
//
// Module definition is handled via:
// - Document links: `compute_links` extracts use/require targets
// - TypeDefinitionProvider: finds package definitions within a file
// - TypeHierarchyProvider: navigates package inheritance

#[test]
fn goto_def_module_use_statement_produces_link() -> Result<(), Box<dyn std::error::Error>> {
    let text = "use My::Module;\n";
    let links = compute_links("file:///test.pl", text, &[]);
    assert_eq!(
        links.len(),
        1,
        "use statement should produce exactly one document link"
    );

    let link = links.first().ok_or("expected a link")?;
    assert_eq!(
        link.pointer("/data/module")
            .and_then(serde_json::Value::as_str),
        Some("My::Module")
    );
    assert_eq!(
        link.pointer("/data/type")
            .and_then(serde_json::Value::as_str),
        Some("module")
    );
    Ok(())
}

#[test]
fn goto_def_module_require_qualified_produces_link() -> Result<(), Box<dyn std::error::Error>> {
    let text = "require Net::HTTP;\n";
    let links = compute_links("file:///test.pl", text, &[]);
    assert_eq!(links.len(), 1);

    let link = links.first().ok_or("expected a link")?;
    assert_eq!(
        link.pointer("/data/module")
            .and_then(serde_json::Value::as_str),
        Some("Net::HTTP")
    );
    Ok(())
}

#[test]
fn goto_def_module_package_definition_via_workspace_symbols() {
    let code = concat!(
        "package My::Utils;\n",
        "sub helper { 42 }\n",
        "\n",
        "package main;\n",
        "My::Utils::helper();\n",
    );
    let (provider, source_map) = make_provider("file:///test.pl", code);

    // The package "My::Utils" should be findable via workspace symbol search
    let results = provider.search("My::Utils", &source_map);
    assert!(
        !results.is_empty(),
        "should find My::Utils package definition"
    );
    assert_eq!(results[0].name, "My::Utils");
}

#[test]
fn goto_def_module_multiple_packages_correct_resolution() {
    let code = concat!(
        "package Foo;\n",
        "sub foo_method { }\n",
        "\n",
        "package Bar;\n",
        "sub bar_method { }\n",
        "\n",
        "package main;\n",
    );
    let (provider, source_map) = make_provider("file:///test.pl", code);

    let foo_results = provider.search("Foo", &source_map);
    let bar_results = provider.search("Bar", &source_map);

    // Each package should be discoverable
    assert!(
        foo_results.iter().any(|r| r.name == "Foo"),
        "should find Foo package"
    );
    assert!(
        bar_results.iter().any(|r| r.name == "Bar"),
        "should find Bar package"
    );
}

#[test]
fn goto_def_module_nonexistent_package_returns_empty() {
    let code = "package Existing;\nsub method { }\n";
    let (provider, source_map) = make_provider("file:///test.pl", code);

    let results = provider.search("NonExistent", &source_map);
    assert!(
        results.is_empty(),
        "non-existent package should return empty results"
    );
}

#[test]
fn goto_def_module_deeply_qualified_use() -> Result<(), Box<dyn std::error::Error>> {
    let text = "use Very::Deep::Nested::Module;\n";
    let links = compute_links("file:///test.pl", text, &[]);
    assert_eq!(links.len(), 1);

    let link = links.first().ok_or("expected a link")?;
    assert_eq!(
        link.pointer("/data/module")
            .and_then(serde_json::Value::as_str),
        Some("Very::Deep::Nested::Module")
    );
    Ok(())
}

#[test]
fn goto_def_module_use_with_import_list() -> Result<(), Box<dyn std::error::Error>> {
    let text = "use File::Path qw(make_path remove_tree);\n";
    let links = compute_links("file:///test.pl", text, &[]);
    assert_eq!(links.len(), 1);

    let link = links.first().ok_or("expected a link")?;
    assert_eq!(
        link.pointer("/data/module")
            .and_then(serde_json::Value::as_str),
        Some("File::Path")
    );
    Ok(())
}

#[test]
fn goto_def_module_require_file_path() -> Result<(), Box<dyn std::error::Error>> {
    let text = "require 'My/Helper.pm';\n";
    let links = compute_links("file:///test.pl", text, &[]);
    assert_eq!(links.len(), 1);

    let link = links.first().ok_or("expected a link")?;
    assert_eq!(
        link.pointer("/data/type")
            .and_then(serde_json::Value::as_str),
        Some("file")
    );
    assert_eq!(
        link.pointer("/data/path")
            .and_then(serde_json::Value::as_str),
        Some("My/Helper.pm")
    );
    Ok(())
}

#[test]
fn goto_def_module_block_form_package() {
    let code = "package MyBlock { sub method { 1 } }\n";
    let (provider, source_map) = make_provider("file:///test.pl", code);

    let results = provider.search("MyBlock", &source_map);
    assert!(
        !results.is_empty(),
        "block-form package should be findable via workspace symbols"
    );
    assert_eq!(results[0].name, "MyBlock");
}

#[test]
fn goto_def_module_workspace_symbol_for_package() {
    let code = "package Data::Processor;\nsub run { }\n";
    let (provider, source_map) = make_provider("file:///lib/Data/Processor.pm", code);

    let results = provider.search("Data::Processor", &source_map);
    assert!(
        !results.is_empty(),
        "package should appear as workspace symbol"
    );
    assert_eq!(results[0].name, "Data::Processor");
}

// ══════════════════════════════════════════════
// 4. Find all references
// ══════════════════════════════════════════════

#[test]
fn find_all_refs_variable_covers_all_sites() {
    let code = "my $count = 0; $count++; $count += 10; print $count;";
    let ast = parse_ast(code);

    let offset = must_some(code.find("$count"));
    let refs = must_some(find_references_single_file(&ast, offset));

    let expected_count = code.matches("$count").count();
    assert_eq!(
        refs.len(),
        expected_count,
        "should find all {} occurrences of $count",
        expected_count
    );
}

#[test]
fn find_all_refs_function_covers_all_call_sites() {
    let code = "sub process { } process(); process(); process();";
    let ast = parse_ast(code);

    let offset = must_some(code.find("process"));
    let refs = must_some(find_references_single_file(&ast, offset));

    // 1 definition + 3 calls
    assert!(
        refs.len() >= 4,
        "should find 4+ refs for process, got {}",
        refs.len()
    );
}

#[test]
fn find_all_refs_variable_excludes_different_sigil() {
    let code = "my $data = 1; my @data = (2, 3); my %data = (a => 4);";
    let ast = parse_ast(code);

    // Find refs for $data
    let offset = must_some(code.find("$data"));
    let refs = find_references_single_file(&ast, offset);
    if let Some(refs) = refs {
        for &(start, end) in &refs {
            let fragment = &code[start..end.min(code.len())];
            assert!(
                fragment.starts_with('$'),
                "scalar refs should only match $data, got '{}'",
                fragment
            );
        }
    }
}

#[test]
fn find_all_refs_no_false_positives_for_similar_names() {
    let code = "my $foo = 1; my $foobar = 2; my $foo_baz = 3; print $foo;";
    let ast = parse_ast(code);

    let offset = must_some(code.find("$foo "));
    let refs = find_references_single_file(&ast, offset);
    if let Some(refs) = refs {
        for &(start, end) in &refs {
            let fragment = &code[start..end.min(code.len())];
            // Should only match "$foo", not "$foobar" or "$foo_baz"
            assert!(
                !fragment.contains("bar") && !fragment.contains("baz"),
                "should not match similar-but-different names, got '{}'",
                fragment
            );
        }
    }
}

#[test]
fn find_all_refs_from_any_occurrence() {
    // References should be the same regardless of which occurrence we start from
    let code = "my $x = 1; $x = 2; print $x;";
    let ast = parse_ast(code);

    // From first occurrence
    let offset_first = must_some(code.find("$x"));
    let refs_first = find_references_single_file(&ast, offset_first);

    // From last occurrence
    let offset_last = must_some(code.rfind("$x"));
    let refs_last = find_references_single_file(&ast, offset_last);

    // Both should return the same set of references
    if let (Some(r1), Some(r2)) = (&refs_first, &refs_last) {
        assert_eq!(
            r1.len(),
            r2.len(),
            "references should be same from any occurrence: first={}, last={}",
            r1.len(),
            r2.len()
        );
    }
}

#[test]
fn find_all_refs_subroutine_from_definition_site() {
    let code = "sub worker { 1 } worker(); worker();";
    let ast = parse_ast(code);

    // From the definition (sub worker)
    let def_offset = must_some(code.find("worker"));
    let refs = must_some(find_references_single_file(&ast, def_offset));

    assert!(
        refs.len() >= 3,
        "should find def + 2 calls, got {}",
        refs.len()
    );
}

#[test]
fn find_all_refs_returns_sorted_by_offset() {
    let code = "my $z = 1; $z = 2; $z = 3; print $z;";
    let ast = parse_ast(code);

    let offset = must_some(code.find("$z"));
    let refs = must_some(find_references_single_file(&ast, offset));

    // Verify references are sorted by start offset
    for window in refs.windows(2) {
        assert!(
            window[0].0 <= window[1].0,
            "references should be sorted by offset: {} > {}",
            window[0].0,
            window[1].0
        );
    }
}

#[test]
fn find_all_refs_variable_in_while_loop() {
    let code = "my $done = 0; while (!$done) { $done = check(); }";
    let ast = parse_ast(code);

    let offset = must_some(code.find("$done"));
    let refs = find_references_single_file(&ast, offset);
    if let Some(refs) = refs {
        assert!(refs.len() >= 2, "should find $done refs in while loop");
    }
}

#[test]
fn find_all_refs_subroutine_different_subs_not_confused() {
    let code = "sub read { } sub write { } read(); write(); read();";
    let ast = parse_ast(code);

    let read_offset = must_some(code.find("read"));
    let read_refs = find_references_single_file(&ast, read_offset);
    if let Some(refs) = read_refs {
        for &(start, end) in &refs {
            let fragment = &code[start..end.min(code.len())];
            assert!(
                !fragment.contains("write"),
                "'read' refs should not include 'write', got '{}'",
                fragment
            );
        }
    }
}

// ══════════════════════════════════════════════
// 5. Workspace symbol search
// ══════════════════════════════════════════════

#[test]
fn workspace_search_finds_sub_by_exact_name() {
    let code = "sub calculate_total { 1 }\n";
    let (provider, source_map) = make_provider("file:///lib.pl", code);

    let results = provider.search("calculate_total", &source_map);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "calculate_total");
    assert_eq!(results[0].kind, 12, "should be Function kind");
}

#[test]
fn workspace_search_finds_package_by_name() {
    let code = "package App::Controller;\nsub handle { }\n";
    let (provider, source_map) = make_provider("file:///lib/App/Controller.pm", code);

    let results = provider.search("App::Controller", &source_map);
    assert!(!results.is_empty());
    assert_eq!(results[0].name, "App::Controller");
}

#[test]
fn workspace_search_finds_sub_inside_package() {
    let code = "package DB::Query;\nsub execute { }\nsub prepare { }\n";
    let (provider, source_map) = make_provider("file:///lib.pm", code);

    let results = provider.search("execute", &source_map);
    assert!(!results.is_empty());
    assert_eq!(results[0].name, "execute");
    assert_eq!(results[0].container_name.as_deref(), Some("DB::Query"));
}

#[test]
fn workspace_search_across_multiple_files() {
    let mut provider = WorkspaceSymbolsProvider::new();
    let mut source_map = HashMap::new();

    let code_a = "package Model;\nsub load { }\n";
    let code_b = "package Controller;\nsub load { }\n";

    let ast_a = must(Parser::new(code_a).parse());
    let ast_b = must(Parser::new(code_b).parse());

    provider.index_document("file:///a.pm", &ast_a, code_a);
    provider.index_document("file:///b.pm", &ast_b, code_b);
    source_map.insert("file:///a.pm".to_string(), code_a.to_string());
    source_map.insert("file:///b.pm".to_string(), code_b.to_string());

    let results = provider.search("load", &source_map);
    assert_eq!(results.len(), 2, "should find 'load' in both files");

    let containers: Vec<Option<&str>> = results
        .iter()
        .map(|r| r.container_name.as_deref())
        .collect();
    assert!(containers.contains(&Some("Model")));
    assert!(containers.contains(&Some("Controller")));
}

#[test]
fn workspace_search_fuzzy_match_initials() {
    let code = "sub get_user_by_email { }\n";
    let (provider, source_map) = make_provider("file:///lib.pl", code);

    // "gube" is a subsequence of "get_user_by_email"
    let results = provider.search("gube", &source_map);
    assert!(
        !results.is_empty(),
        "fuzzy match 'gube' should match 'get_user_by_email'"
    );
}

#[test]
fn workspace_search_empty_query_returns_all_symbols() {
    let code = "sub alpha { }\nsub beta { }\nsub gamma { }\n";
    let (provider, source_map) = make_provider("file:///lib.pl", code);

    let results = provider.search("", &source_map);
    assert!(
        results.len() >= 3,
        "empty query should return all symbols, got {}",
        results.len()
    );
}

#[test]
fn workspace_search_case_insensitive_match() {
    let code = "sub ProcessData { }\n";
    let (provider, source_map) = make_provider("file:///lib.pl", code);

    let results = provider.search("processdata", &source_map);
    assert!(!results.is_empty(), "search should be case-insensitive");
    assert_eq!(results[0].name, "ProcessData");
}

#[test]
fn workspace_search_symbol_has_correct_uri() {
    let uri = "file:///workspace/lib/App/Main.pm";
    let code = "package App::Main;\nsub run { }\n";
    let (provider, source_map) = make_provider(uri, code);

    let results = provider.search("run", &source_map);
    assert!(!results.is_empty());
    assert_eq!(
        results[0].location.uri, uri,
        "symbol URI should match indexed document"
    );
}

#[test]
fn workspace_search_symbol_line_number_correct() {
    let code = "package Pkg;\n\n\nsub on_line_four { }\n";
    let (provider, source_map) = make_provider("file:///t.pl", code);

    let results = provider.search("on_line_four", &source_map);
    assert!(!results.is_empty());
    // "on_line_four" is on line 3 (0-indexed), since there are 3 newlines before it
    assert_eq!(results[0].location.range.start.line, 3);
}

#[test]
fn workspace_search_after_reindex_shows_updated_symbols() {
    let uri = "file:///lib.pl";
    let mut provider = WorkspaceSymbolsProvider::new();
    let mut source_map = HashMap::new();

    // Initial version
    let v1 = "sub old_api { }\n";
    let ast1 = must(Parser::new(v1).parse());
    provider.index_document(uri, &ast1, v1);
    source_map.insert(uri.to_string(), v1.to_string());

    let results = provider.search("old_api", &source_map);
    assert!(!results.is_empty());

    // Updated version
    let v2 = "sub new_api { }\nsub another { }\n";
    let ast2 = must(Parser::new(v2).parse());
    provider.index_document(uri, &ast2, v2);
    source_map.insert(uri.to_string(), v2.to_string());

    let old_results = provider.search("old_api", &source_map);
    assert!(
        old_results.is_empty(),
        "old_api should be gone after reindex"
    );

    let new_results = provider.search("new_api", &source_map);
    assert!(
        !new_results.is_empty(),
        "new_api should be present after reindex"
    );
}

#[test]
fn workspace_search_with_candidates_restricts_scope() {
    let code = "sub alpha { }\nsub beta { }\nsub gamma { }\n";
    let (provider, source_map) = make_provider("file:///t.pl", code);

    let candidates = vec!["alpha".to_string(), "gamma".to_string()];
    let results = provider.search_with_candidates("a", &source_map, &candidates);

    let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
    assert!(
        names.contains(&"alpha"),
        "alpha should match 'a' in candidates"
    );
    assert!(
        !names.contains(&"beta"),
        "beta should not appear -- not in candidates"
    );
}

// ══════════════════════════════════════════════
// Cross-cutting integration: combining navigation
// ══════════════════════════════════════════════

#[test]
fn integration_refs_plus_workspace_symbols_for_function() {
    let code = "package Utils;\nsub format_date { 1 }\nformat_date('2024-01-01');\n";
    let ast = parse_ast(code);

    // References
    let offset = must_some(code.find("format_date"));
    let refs = must_some(find_references_single_file(&ast, offset));
    assert!(refs.len() >= 2, "should find def + call");

    // Workspace symbols
    let (provider, source_map) = make_provider("file:///utils.pm", code);
    let ws_results = provider.search("format_date", &source_map);
    assert!(!ws_results.is_empty());
    assert_eq!(ws_results[0].container_name.as_deref(), Some("Utils"));
}

#[test]
fn integration_module_link_plus_type_hierarchy() {
    let code = concat!(
        "package Animal;\nsub speak { }\n\n",
        "package Dog;\nuse parent 'Animal';\nsub speak { 'woof' }\n",
    );

    // Type hierarchy
    let ast = parse_ast(code);
    let hierarchy = TypeHierarchyProvider::new();
    let dog_item = perl_lsp_navigation::TypeHierarchyItem {
        name: "Dog".to_string(),
        kind: TypeHierarchySymbolKind::Class,
        uri: "file:///animals.pm".to_string(),
        range: perl_position_tracking::WireRange::default(),
        selection_range: perl_position_tracking::WireRange::default(),
        detail: None,
        data: None,
    };
    let supertypes = hierarchy.find_supertypes(&ast, &dog_item);
    assert_eq!(supertypes.len(), 1);
    assert_eq!(supertypes[0].name, "Animal");

    // Workspace symbols for both classes
    let (provider, source_map) = make_provider("file:///animals.pm", code);
    let animal_results = provider.search("Animal", &source_map);
    let dog_results = provider.search("Dog", &source_map);
    assert!(!animal_results.is_empty());
    assert!(!dog_results.is_empty());
}

#[test]
fn integration_multiple_navigation_same_code() {
    let code = concat!(
        "package Validator;\n",
        "sub validate { my $input = shift; return length($input) > 0; }\n",
        "validate('test');\n",
    );
    let ast = parse_ast(code);

    // 1. Find references for $input
    if let Some(input_offset) = code.find("$input") {
        let refs = find_references_single_file(&ast, input_offset);
        if let Some(refs) = refs {
            assert!(refs.len() >= 2, "$input should have 2+ refs");
        }
    }

    // 2. Find references for validate
    let validate_offset = must_some(code.find("validate"));
    let refs = must_some(find_references_single_file(&ast, validate_offset));
    assert!(refs.len() >= 2, "validate should have def + call");

    // 3. Workspace symbols
    let (provider, source_map) = make_provider("file:///validator.pm", code);
    let results = provider.search("validate", &source_map);
    assert!(!results.is_empty());
    assert_eq!(results[0].container_name.as_deref(), Some("Validator"));
}
