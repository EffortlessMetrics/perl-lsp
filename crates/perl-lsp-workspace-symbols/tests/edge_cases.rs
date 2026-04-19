use perl_lsp_workspace_symbols::{WorkspaceSymbol, WorkspaceSymbolsProvider};
use perl_parser_core::Parser;
use perl_tdd_support::must;
use std::collections::HashMap;

fn parse_and_index(
    provider: &mut WorkspaceSymbolsProvider,
    uri: &str,
    source: &str,
) -> HashMap<String, String> {
    let mut source_map = HashMap::new();
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    provider.index_document(uri, &ast, source);
    source_map.insert(uri.to_string(), source.to_string());
    source_map
}

// ---- Empty workspace -------------------------------------------------------

#[test]
fn empty_workspace_search_returns_empty() {
    let provider = WorkspaceSymbolsProvider::new();
    let source_map = HashMap::new();
    let results = provider.search("anything", &source_map);
    assert!(
        results.is_empty(),
        "empty workspace should return no results"
    );
}

#[test]
fn empty_workspace_get_all_symbols_returns_empty() {
    let provider = WorkspaceSymbolsProvider::default();
    assert!(provider.get_all_symbols().is_empty());
}

#[test]
fn empty_workspace_search_with_candidates_returns_empty() {
    let provider = WorkspaceSymbolsProvider::new();
    let source_map = HashMap::new();
    let candidates = vec!["anything".to_string()];
    let results = provider.search_with_candidates("anything", &source_map, &candidates);
    assert!(results.is_empty());
}

// ---- Empty query -----------------------------------------------------------

#[test]
fn empty_query_returns_all_symbols() {
    let mut provider = WorkspaceSymbolsProvider::new();
    let source = "sub alpha { 1 }\nsub beta { 2 }\nsub gamma { 3 }\n";
    let source_map = parse_and_index(&mut provider, "file:///all.pl", source);

    // Empty query should match everything
    let results = provider.search("", &source_map);
    assert!(
        results.len() >= 3,
        "empty query should return all symbols, got {}",
        results.len()
    );
}

// ---- No results ------------------------------------------------------------

#[test]
fn query_with_no_match_returns_empty() {
    let mut provider = WorkspaceSymbolsProvider::new();
    let source = "sub real_function { 1 }\n";
    let source_map = parse_and_index(&mut provider, "file:///real.pl", source);

    let results = provider.search("zzz_nonexistent_zzz", &source_map);
    assert!(
        results.is_empty(),
        "query with no match should return empty"
    );
}

// ---- Case sensitivity ------------------------------------------------------

#[test]
fn search_is_case_insensitive_for_prefix() {
    let mut provider = WorkspaceSymbolsProvider::new();
    let source = "sub MyFunction { 1 }\n";
    let source_map = parse_and_index(&mut provider, "file:///case.pl", source);

    // lowercase query should match mixed-case symbol
    let results_lower = provider.search("myfunction", &source_map);
    let results_upper = provider.search("MYFUNCTION", &source_map);
    let results_exact = provider.search("MyFunction", &source_map);

    assert!(!results_exact.is_empty(), "exact case should match");
    assert!(
        !results_lower.is_empty(),
        "lowercase query should match mixed-case symbol"
    );
    assert!(
        !results_upper.is_empty(),
        "uppercase query should match mixed-case symbol"
    );
}

// ---- Special characters in symbol names ------------------------------------

#[test]
fn symbol_with_double_colon_separator_is_found() {
    let mut provider = WorkspaceSymbolsProvider::new();
    let source = "package My::Deep::Namespace;\nsub handler { 1 }\n";
    let source_map = parse_and_index(&mut provider, "file:///ns.pm", source);

    let results = provider.search("handler", &source_map);
    assert!(
        !results.is_empty(),
        "should find handler in namespaced package"
    );
}

#[test]
fn underscore_in_symbol_name_matches() {
    let mut provider = WorkspaceSymbolsProvider::new();
    let source = "sub _private_helper { 1 }\nsub public_api { 2 }\n";
    let source_map = parse_and_index(&mut provider, "file:///private.pl", source);

    let results = provider.search("_private", &source_map);
    let names: Vec<&str> = results.iter().map(|s| s.name.as_str()).collect();
    assert!(
        names.contains(&"_private_helper"),
        "underscore-prefixed symbol should match"
    );
}

// ---- WorkspaceSymbol response shape ----------------------------------------

#[test]
fn workspace_symbol_has_required_fields() {
    let mut provider = WorkspaceSymbolsProvider::new();
    let source = "sub my_sub { 1 }\n";
    let source_map = parse_and_index(&mut provider, "file:///shape.pl", source);

    let results = provider.search("my_sub", &source_map);
    assert!(!results.is_empty());

    let sym = &results[0];
    assert!(!sym.name.is_empty(), "name must be non-empty");
    assert!(sym.kind > 0, "kind must be positive LSP symbol kind");
    assert!(
        !sym.location.uri.is_empty(),
        "location.uri must be non-empty"
    );
    assert_eq!(sym.location.uri, "file:///shape.pl");
}

#[test]
fn workspace_symbol_location_uri_matches_indexed_file() {
    let mut provider = WorkspaceSymbolsProvider::new();
    let source = "sub located_fn { 1 }\n";
    let source_map = parse_and_index(&mut provider, "file:///located.pl", source);

    let results = provider.search("located_fn", &source_map);
    assert!(!results.is_empty());
    assert_eq!(results[0].location.uri, "file:///located.pl");
}

#[test]
fn workspace_symbol_serializes_to_valid_json() {
    let sym = WorkspaceSymbol {
        name: "test_fn".to_string(),
        kind: 12,
        location: perl_position_tracking::WireLocation::new(
            "file:///x.pl".to_string(),
            perl_position_tracking::WireRange::default(),
        ),
        container_name: Some("MyPackage".to_string()),
    };

    let json = must(serde_json::to_string(&sym));
    assert!(json.contains("\"name\":\"test_fn\""));
    assert!(json.contains("\"kind\":12"));
    assert!(json.contains("\"containerName\":\"MyPackage\""));
}

#[test]
fn workspace_symbol_without_container_omits_container_name_field() {
    let sym = WorkspaceSymbol {
        name: "no_container".to_string(),
        kind: 12,
        location: perl_position_tracking::WireLocation::new(
            "file:///x.pl".to_string(),
            perl_position_tracking::WireRange::default(),
        ),
        container_name: None,
    };

    let json = must(serde_json::to_string(&sym));
    // skip_serializing_if = "Option::is_none" means absent when None
    assert!(
        !json.contains("containerName"),
        "containerName should be absent when None"
    );
}

// ---- Re-indexing replaces old symbols --------------------------------------

#[test]
fn re_indexing_document_replaces_old_symbols() {
    let mut provider = WorkspaceSymbolsProvider::new();
    let uri = "file:///evolving.pl";

    let source_v1 = "sub old_name { 1 }\n";
    let source_v2 = "sub new_name { 2 }\n";

    let mut source_map = HashMap::new();
    let mut parser = Parser::new(source_v1);
    let ast = must(parser.parse());
    provider.index_document(uri, &ast, source_v1);
    source_map.insert(uri.to_string(), source_v1.to_string());

    assert!(!provider.search("old_name", &source_map).is_empty());

    // Re-index with new content
    let mut parser = Parser::new(source_v2);
    let ast = must(parser.parse());
    provider.index_document(uri, &ast, source_v2);
    source_map.insert(uri.to_string(), source_v2.to_string());

    assert!(
        provider.search("old_name", &source_map).is_empty(),
        "old symbol should be gone"
    );
    assert!(
        !provider.search("new_name", &source_map).is_empty(),
        "new symbol should appear"
    );
}

// ---- get_all_symbols -------------------------------------------------------

#[test]
fn get_all_symbols_returns_symbols_from_all_documents() {
    let mut provider = WorkspaceSymbolsProvider::new();

    let src_a = "sub fn_a { 1 }\n";
    let src_b = "sub fn_b { 2 }\n";

    for (uri, src) in [("file:///a.pl", src_a), ("file:///b.pl", src_b)] {
        let mut parser = Parser::new(src);
        let ast = must(parser.parse());
        provider.index_document(uri, &ast, src);
    }

    let all = provider.get_all_symbols();
    let names: Vec<&str> = all.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"fn_a"), "should include fn_a");
    assert!(names.contains(&"fn_b"), "should include fn_b");
}

// ---- Missing source_map entry ----------------------------------------------

#[test]
fn search_skips_documents_not_in_source_map() {
    let mut provider = WorkspaceSymbolsProvider::new();
    let source = "sub missing_src { 1 }\n";
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    provider.index_document("file:///missing.pl", &ast, source);

    // Don't include the file in source_map
    let empty_map = HashMap::new();
    let results = provider.search("missing_src", &empty_map);
    assert!(
        results.is_empty(),
        "should skip documents without source_map entry"
    );
}
