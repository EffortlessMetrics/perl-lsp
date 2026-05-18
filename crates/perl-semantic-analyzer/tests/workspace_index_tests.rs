//! Unit tests for `WorkspaceIndex` in `crates/perl-semantic-analyzer/src/analysis/index.rs`.
//!
//! Covers:
//! - Insert a symbol and look it up by name → returns matching SymbolDef
//! - `symbols_in_file` behaviour via `find_defs` filtered by URI
//! - `update_from_document` replaces previous defs for the same URI (no stale duplicates)
//! - `remove_document` removes all symbols for a URI
//! - `lookup_by_name` distinguishes definitions added by different URIs
//! - Cross-file lookup returns defs from all files
//! - Empty/edge: lookup on missing name returns empty slice

use perl_semantic_analyzer::Parser;
use perl_semantic_analyzer::SourceLocation;
use perl_semantic_analyzer::index::WorkspaceIndex;
use perl_semantic_analyzer::symbol::{Symbol, SymbolExtractor, SymbolKind, SymbolTable};
use perl_tdd_support::must;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Construct a `SymbolTable` with a single subroutine symbol at offsets [start, end).
fn make_symtab_with_sub(name: &str, start: usize, end: usize) -> SymbolTable {
    let mut symtab = SymbolTable::new();
    let symbol = Symbol {
        name: name.to_string(),
        qualified_name: format!("main::{name}"),
        kind: SymbolKind::Subroutine,
        location: SourceLocation { start, end },
        scope_id: 0,
        declaration: Some("sub".to_string()),
        documentation: None,
        attributes: Vec::new(),
    };
    symtab.symbols.entry(name.to_string()).or_default().push(symbol);
    symtab
}

/// Parse real Perl code and extract a symbol table using `SymbolExtractor`.
fn symtab_from_code(code: &str) -> SymbolTable {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    SymbolExtractor::new_with_source(code).extract(&ast)
}

// ---------------------------------------------------------------------------
// 1. Insert a symbol and look it up by name
// ---------------------------------------------------------------------------

#[test]
fn test_find_defs_by_name_returns_matching_def() -> Result<(), Box<dyn std::error::Error>> {
    let mut index = WorkspaceIndex::new();
    let symtab = make_symtab_with_sub("my_func", 0, 20);

    index.update_from_document("file:///a.pl", "", &symtab);

    let defs = index.find_defs("my_func");
    assert_eq!(defs.len(), 1, "expected one definition for my_func");
    assert_eq!(defs[0].name, "my_func");
    assert_eq!(defs[0].uri, "file:///a.pl");
    assert_eq!(defs[0].start, 0);
    assert_eq!(defs[0].end, 20);
    assert_eq!(defs[0].kind, SymbolKind::Subroutine);
    Ok(())
}

// ---------------------------------------------------------------------------
// 2. Symbols of different SymbolKind in the same file
// ---------------------------------------------------------------------------

#[test]
fn test_symbols_of_different_kinds_in_same_file() -> Result<(), Box<dyn std::error::Error>> {
    // Use SymbolExtractor to build a real symbol table from Perl code so we get
    // multiple kinds in a single SymbolTable.
    let code = r#"
sub greet { 1 }
my $count = 0;
package My::Mod;
"#;
    let symtab = symtab_from_code(code);
    let mut index = WorkspaceIndex::new();
    index.update_from_document("file:///multi.pl", code, &symtab);

    // All three names should resolve to this file.
    let sub_defs = index.find_defs("greet");
    assert!(!sub_defs.is_empty(), "expected greet to be indexed");
    assert!(sub_defs.iter().all(|d| d.uri == "file:///multi.pl"));

    let var_defs = index.find_defs("count");
    assert!(!var_defs.is_empty(), "expected count to be indexed");
    assert!(var_defs.iter().all(|d| d.uri == "file:///multi.pl"));

    let pkg_defs = index.find_defs("My::Mod");
    assert!(!pkg_defs.is_empty(), "expected My::Mod package to be indexed");
    assert!(pkg_defs.iter().all(|d| d.uri == "file:///multi.pl"));

    Ok(())
}

// ---------------------------------------------------------------------------
// 3. update_from_document replaces previous defs (no stale duplicates)
// ---------------------------------------------------------------------------

#[test]
fn test_update_from_document_replaces_stale_defs() -> Result<(), Box<dyn std::error::Error>> {
    let mut index = WorkspaceIndex::new();
    let uri = "file:///update_test.pl";

    // First version: one sub
    let symtab_v1 = make_symtab_with_sub("old_func", 0, 30);
    index.update_from_document(uri, "", &symtab_v1);
    assert_eq!(index.find_defs("old_func").len(), 1, "v1: old_func should be present");

    // Second version: different sub — replaces the first indexing
    let symtab_v2 = make_symtab_with_sub("new_func", 0, 30);
    index.update_from_document(uri, "", &symtab_v2);

    assert_eq!(index.find_defs("old_func").len(), 0, "old_func should be removed after re-index");
    assert_eq!(index.find_defs("new_func").len(), 1, "new_func should appear after re-index");
    Ok(())
}

// ---------------------------------------------------------------------------
// 4. remove_document removes all symbols for a URI
// ---------------------------------------------------------------------------

#[test]
fn test_remove_document_clears_all_symbols_for_uri() -> Result<(), Box<dyn std::error::Error>> {
    let mut index = WorkspaceIndex::new();
    let uri = "file:///remove_test.pl";

    let symtab = make_symtab_with_sub("removable_func", 0, 50);
    index.update_from_document(uri, "", &symtab);

    assert_eq!(
        index.find_defs("removable_func").len(),
        1,
        "symbol should be present before remove"
    );
    assert_eq!(index.file_count(), 1, "one file indexed");

    index.remove_document(uri);

    assert_eq!(index.find_defs("removable_func").len(), 0, "symbol should be gone after remove");
    assert_eq!(index.file_count(), 0, "no files after remove");
    Ok(())
}

// ---------------------------------------------------------------------------
// 5. lookup_by_name distinguishes fully-qualified vs bare names
// ---------------------------------------------------------------------------

#[test]
fn test_find_defs_distinguishes_bare_and_qualified_names() -> Result<(), Box<dyn std::error::Error>>
{
    // The index stores symbols by their `name` field (bare name from SymbolTable key).
    // The qualified_name lives inside SymbolDef.kind but is not a separate index key.
    // We verify that bare "foo" and "Bar::foo" (a different symbol name) are independent.
    let mut index = WorkspaceIndex::new();

    let mut symtab = SymbolTable::new();
    let bare = Symbol {
        name: "foo".to_string(),
        qualified_name: "main::foo".to_string(),
        kind: SymbolKind::Subroutine,
        location: SourceLocation { start: 0, end: 10 },
        scope_id: 0,
        declaration: Some("sub".to_string()),
        documentation: None,
        attributes: Vec::new(),
    };
    let qualified = Symbol {
        name: "Bar::foo".to_string(),
        qualified_name: "Bar::foo".to_string(),
        kind: SymbolKind::Subroutine,
        location: SourceLocation { start: 20, end: 40 },
        scope_id: 0,
        declaration: Some("sub".to_string()),
        documentation: None,
        attributes: Vec::new(),
    };
    symtab.symbols.entry("foo".to_string()).or_default().push(bare);
    symtab.symbols.entry("Bar::foo".to_string()).or_default().push(qualified);

    index.update_from_document("file:///qual.pl", "", &symtab);

    let bare_defs = index.find_defs("foo");
    let qualified_defs = index.find_defs("Bar::foo");

    assert_eq!(bare_defs.len(), 1, "bare 'foo' should have one def");
    assert_eq!(qualified_defs.len(), 1, "'Bar::foo' should have one def");
    // They must be different symbols (different offsets)
    assert_ne!(
        bare_defs[0].start, qualified_defs[0].start,
        "bare and qualified foo should have different locations"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 6. Cross-file lookup returns defs from all files
// ---------------------------------------------------------------------------

#[test]
fn test_cross_file_lookup_finds_defs_in_all_files() -> Result<(), Box<dyn std::error::Error>> {
    let mut index = WorkspaceIndex::new();

    // Both files export a sub named "shared_name" — cross-file scenario
    let symtab_a = make_symtab_with_sub("shared_name", 0, 15);
    let symtab_b = make_symtab_with_sub("shared_name", 5, 25);
    index.update_from_document("file:///file_a.pl", "", &symtab_a);
    index.update_from_document("file:///file_b.pl", "", &symtab_b);

    let defs = index.find_defs("shared_name");
    assert_eq!(defs.len(), 2, "shared_name should have two defs (one per file)");

    let uris: Vec<&str> = defs.iter().map(|d| d.uri.as_str()).collect();
    assert!(uris.contains(&"file:///file_a.pl"), "file_a.pl should be in results");
    assert!(uris.contains(&"file:///file_b.pl"), "file_b.pl should be in results");
    Ok(())
}

// ---------------------------------------------------------------------------
// 7. Empty/edge: lookup on missing name returns empty
// ---------------------------------------------------------------------------

#[test]
fn test_find_defs_missing_name_returns_empty() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let defs = index.find_defs("nonexistent_symbol_xyz");
    assert!(defs.is_empty(), "lookup on missing name should return empty slice");
    Ok(())
}

// ---------------------------------------------------------------------------
// 8. search_symbols with substring query
// ---------------------------------------------------------------------------

#[test]
fn test_search_symbols_substring_match() -> Result<(), Box<dyn std::error::Error>> {
    let mut index = WorkspaceIndex::new();
    let symtab = make_symtab_with_sub("process_request", 0, 40);
    index.update_from_document("file:///srv.pl", "", &symtab);

    let results = index.search_symbols("process");
    assert!(!results.is_empty(), "search_symbols('process') should match process_request");
    assert!(
        results.iter().any(|d| d.name == "process_request"),
        "expected process_request in search results"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 9. symbol_count and file_count reflect reality
// ---------------------------------------------------------------------------

#[test]
fn test_symbol_count_and_file_count() -> Result<(), Box<dyn std::error::Error>> {
    let mut index = WorkspaceIndex::new();

    assert_eq!(index.symbol_count(), 0, "empty index has zero symbols");
    assert_eq!(index.file_count(), 0, "empty index has zero files");

    let symtab_a = make_symtab_with_sub("alpha", 0, 10);
    index.update_from_document("file:///alpha.pl", "", &symtab_a);
    assert_eq!(index.symbol_count(), 1, "one symbol after first insert");
    assert_eq!(index.file_count(), 1, "one file after first insert");

    let symtab_b = make_symtab_with_sub("beta", 0, 10);
    index.update_from_document("file:///beta.pl", "", &symtab_b);
    assert_eq!(index.symbol_count(), 2, "two symbols after second insert");
    assert_eq!(index.file_count(), 2, "two files after second insert");

    index.remove_document("file:///alpha.pl");
    assert_eq!(index.symbol_count(), 1, "one symbol after removing first file");
    assert_eq!(index.file_count(), 1, "one file after removing first file");
    Ok(())
}
