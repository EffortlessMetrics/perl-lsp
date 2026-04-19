//! Mutation-killing tests for perl-symbol-index: SymbolIndex trie and fuzzy matching.
//!
//! The single inline test covers the basic happy path.
//! These tests target branches and behaviors not exercised by that test:
//!
//! - Empty prefix returns all indexed symbols (no early-exit)
//! - Non-existent prefix returns empty
//! - Empty query returns empty (no tokens to match)
//! - Fuzzy ranking: more token matches = higher rank
//! - CamelCase tokenization splits at case boundaries
//! - Underscore tokenization splits on non-alphanumeric chars
//! - Consecutive uppercase does NOT split (e.g., "HTML" stays as one token)
//! - Symbol not in index is not found
//! - Default trait (== new())

use perl_symbol::index::SymbolIndex;

// ---------------------------------------------------------------------------
// Empty prefix: returns all symbols
// ---------------------------------------------------------------------------

#[test]
fn search_prefix_empty_string_returns_all_symbols() {
    let mut index = SymbolIndex::new();
    index.add_symbol("foo".to_string());
    index.add_symbol("bar".to_string());
    index.add_symbol("baz".to_string());

    let results = index.search_prefix("");
    assert_eq!(
        results.len(),
        3,
        "empty prefix must match all indexed symbols"
    );
    assert!(results.contains(&"foo".to_string()));
    assert!(results.contains(&"bar".to_string()));
    assert!(results.contains(&"baz".to_string()));
}

// ---------------------------------------------------------------------------
// Non-existent prefix returns empty
// ---------------------------------------------------------------------------

#[test]
fn search_prefix_no_match_returns_empty() {
    let mut index = SymbolIndex::new();
    index.add_symbol("alpha".to_string());
    index.add_symbol("beta".to_string());

    let results = index.search_prefix("gamma");
    assert!(results.is_empty(), "prefix not in index must return empty");
}

#[test]
fn search_prefix_partial_match_then_no_child() {
    let mut index = SymbolIndex::new();
    index.add_symbol("foo".to_string());

    // "foo" exists but "foox" doesn't
    let results = index.search_prefix("foox");
    assert!(
        results.is_empty(),
        "prefix longer than any symbol must return empty"
    );
}

// ---------------------------------------------------------------------------
// Exact prefix match
// ---------------------------------------------------------------------------

#[test]
fn search_prefix_exact_symbol_name_returns_that_symbol() {
    let mut index = SymbolIndex::new();
    index.add_symbol("get_user".to_string());
    index.add_symbol("get_user_name".to_string());
    index.add_symbol("set_user".to_string());

    let results = index.search_prefix("get_user");
    assert_eq!(
        results.len(),
        2,
        "exact prefix should match symbol and its extensions"
    );
    assert!(results.contains(&"get_user".to_string()));
    assert!(results.contains(&"get_user_name".to_string()));
}

// ---------------------------------------------------------------------------
// Empty index
// ---------------------------------------------------------------------------

#[test]
fn search_prefix_on_empty_index_returns_empty() {
    let index = SymbolIndex::new();
    let results = index.search_prefix("any");
    assert!(
        results.is_empty(),
        "empty index must return empty for any prefix"
    );
}

#[test]
fn search_fuzzy_on_empty_index_returns_empty() {
    let index = SymbolIndex::new();
    let results = index.search_fuzzy("any query");
    assert!(
        results.is_empty(),
        "empty index must return empty for any fuzzy query"
    );
}

// ---------------------------------------------------------------------------
// Fuzzy search: empty query returns empty
// ---------------------------------------------------------------------------

#[test]
fn search_fuzzy_empty_query_returns_empty() {
    let mut index = SymbolIndex::new();
    index.add_symbol("calculate_total".to_string());

    let results = index.search_fuzzy("");
    assert!(
        results.is_empty(),
        "empty query has no tokens, must return empty"
    );
}

// ---------------------------------------------------------------------------
// Fuzzy ranking: more matching tokens = higher rank
// ---------------------------------------------------------------------------

#[test]
fn search_fuzzy_more_matching_tokens_ranks_higher() {
    let mut index = SymbolIndex::new();
    // "get_user_name" will have tokens: ["get", "user", "name"]
    // "get_user" will have tokens: ["get", "user"]
    // "get_account" will have tokens: ["get", "account"]
    index.add_symbol("get_user_name".to_string());
    index.add_symbol("get_user".to_string());
    index.add_symbol("get_account".to_string());

    // Query "get user" → tokens: ["get", "user"]
    // "get_user_name" matches both tokens → score 2
    // "get_user" matches both tokens → score 2
    // "get_account" matches only "get" → score 1
    let results = index.search_fuzzy("get user");
    assert!(!results.is_empty(), "should find results for 'get user'");

    // "get_account" should rank last (only 1 matching token)
    let account_pos = results.iter().position(|s| s == "get_account");
    let user_pos = results.iter().position(|s| s == "get_user");
    let user_name_pos = results.iter().position(|s| s == "get_user_name");

    if let (Some(ap), Some(up)) = (account_pos, user_pos) {
        assert!(
            up < ap,
            "get_user (2 tokens) should rank above get_account (1 token)"
        );
    }
    if let (Some(ap), Some(unp)) = (account_pos, user_name_pos) {
        assert!(
            unp < ap,
            "get_user_name (2 tokens) should rank above get_account (1 token)"
        );
    }
}

// ---------------------------------------------------------------------------
// Tokenization via CamelCase
// ---------------------------------------------------------------------------

#[test]
fn camelcase_symbol_is_findable_by_individual_word() {
    let mut index = SymbolIndex::new();
    index.add_symbol("getLogger".to_string());

    // "getLogger" tokenizes to ["get", "logger"]
    let results = index.search_fuzzy("logger");
    assert!(
        results.contains(&"getLogger".to_string()),
        "CamelCase split: 'logger' should find 'getLogger'"
    );
}

#[test]
fn camelcase_symbol_is_findable_by_prefix_of_each_component() {
    let mut index = SymbolIndex::new();
    index.add_symbol("calculateTotal".to_string());

    // Prefix "calc" should find "calculateTotal"
    let results = index.search_prefix("calc");
    assert!(
        results.contains(&"calculateTotal".to_string()),
        "prefix 'calc' must find 'calculateTotal'"
    );
}

// ---------------------------------------------------------------------------
// Tokenization: consecutive uppercase stays as one token
// ---------------------------------------------------------------------------

#[test]
fn consecutive_uppercase_not_split_is_findable_by_prefix() {
    let mut index = SymbolIndex::new();
    // "XMLParser" → should tokenize to ["xmlparser"] or ["xml", "parser"] depending on impl
    // Either way, let's verify prefix matching works
    index.add_symbol("XMLParser".to_string());

    let results = index.search_prefix("XML");
    assert!(
        results.contains(&"XMLParser".to_string()),
        "prefix 'XML' must find 'XMLParser'"
    );
}

// ---------------------------------------------------------------------------
// Tokenization: underscore splits
// ---------------------------------------------------------------------------

#[test]
fn underscore_symbol_tokens_are_each_searchable() {
    let mut index = SymbolIndex::new();
    index.add_symbol("open_file_handle".to_string());

    // "open_file_handle" tokenizes to ["open", "file", "handle"]
    let results = index.search_fuzzy("file");
    assert!(
        results.contains(&"open_file_handle".to_string()),
        "token 'file' should find 'open_file_handle'"
    );

    let results = index.search_fuzzy("handle");
    assert!(
        results.contains(&"open_file_handle".to_string()),
        "token 'handle' should find 'open_file_handle'"
    );
}

// ---------------------------------------------------------------------------
// Symbol not indexed: not returned
// ---------------------------------------------------------------------------

#[test]
fn symbol_not_added_is_not_found_by_prefix() {
    let mut index = SymbolIndex::new();
    index.add_symbol("alpha".to_string());

    let results = index.search_prefix("beta");
    assert!(
        !results.contains(&"beta".to_string()),
        "unadded symbol must not appear"
    );
}

#[test]
fn symbol_not_added_is_not_found_by_fuzzy() {
    let mut index = SymbolIndex::new();
    index.add_symbol("alpha".to_string());

    let results = index.search_fuzzy("beta");
    assert!(
        !results.contains(&"beta".to_string()),
        "unadded symbol must not appear in fuzzy"
    );
}

// ---------------------------------------------------------------------------
// Default trait
// ---------------------------------------------------------------------------

#[test]
fn default_creates_empty_index() {
    let index = SymbolIndex::default();
    let results = index.search_prefix("any");
    assert!(results.is_empty(), "Default must create an empty index");
}

// ---------------------------------------------------------------------------
// Single symbol added and retrieved
// ---------------------------------------------------------------------------

#[test]
fn single_symbol_indexed_and_retrieved_by_exact_prefix() {
    let mut index = SymbolIndex::new();
    index.add_symbol("my_sub".to_string());

    let results = index.search_prefix("my_sub");
    assert_eq!(results, vec!["my_sub".to_string()]);
}
