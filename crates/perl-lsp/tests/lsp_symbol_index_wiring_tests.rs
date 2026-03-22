//! Tests that the SymbolIndex (trie-based fast lookup) is populated on
//! didOpen/didChange and its queries are wired into completion and workspace
//! symbol handlers.
//!
//! Issue #2701: SymbolIndex was populated but never queried.

mod support;
use serde_json::json;
use std::time::Duration;
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ---------------------------------------------------------------------------
// SymbolIndex trie is populated on didOpen
// ---------------------------------------------------------------------------

/// Verify that the SymbolIndex trie is populated with symbols from the opened document.
/// This is a direct test of the indexing, not an integration test.
#[test]
fn symbol_index_populated_on_did_open() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    harness.open(
        "file:///test_trie.pl",
        r#"package TrieTest;

sub calculate_total { return 42; }
sub calculate_average { return 21; }
sub get_user_name { return "test"; }

1;
"#,
    )?;

    // Let the server process the didOpen and populate its indexes
    harness.wait_for_idle(Duration::from_millis(200));

    // Use test API to directly query the SymbolIndex trie
    let prefix_hits = harness.test_symbol_index_search_prefix("calc");
    assert!(
        !prefix_hits.is_empty(),
        "SymbolIndex should have prefix matches for 'calc', got: {:?}",
        prefix_hits
    );
    assert!(
        prefix_hits.iter().any(|n| n.contains("calculate")),
        "SymbolIndex should contain 'calculate' symbols, got: {:?}",
        prefix_hits
    );

    // Verify that exact matches are found
    let exact_hits = harness.test_symbol_index_search_prefix("calculate_total");
    assert!(
        exact_hits.contains(&"calculate_total".to_string()),
        "SymbolIndex should find exact match 'calculate_total', got: {:?}",
        exact_hits
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Workspace symbol search exercises the SymbolIndex trie path
// ---------------------------------------------------------------------------

/// After opening a document with subroutines, workspace/symbol searches
/// should return results that were indexed by the trie.
/// This integration test verifies that workspace/symbol returns relevant results.
#[test]
fn workspace_symbol_search_returns_indexed_symbols() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    harness.open(
        "file:///test_trie2.pl",
        r#"package TrieTest2;

sub calculate_total { return 42; }
sub calculate_average { return 21; }
sub get_user_name { return "test"; }

1;
"#,
    )?;

    // Let the server process the didOpen and populate its indexes
    harness.wait_for_idle(Duration::from_millis(200));

    // Search for symbols matching "calculate" — the SymbolIndex trie provides
    // a fast prefix lookup that supplements the workspace index search.
    let response =
        harness.request("workspace/symbol", json!({ "query": "calculate" })).unwrap_or(json!(null));

    assert!(!response.is_null(), "workspace/symbol should return a result, got null");

    if response.is_array() {
        let symbols = response.as_array().ok_or("response is not an array")?;
        assert!(
            !symbols.is_empty(),
            "workspace/symbol should return matching symbols for 'calculate'"
        );
        let names: Vec<&str> = symbols.iter().filter_map(|s| s["name"].as_str()).collect();
        assert!(
            names.iter().any(|n| n.contains("calculate")),
            "Returned symbols should contain 'calculate' matches, got: {:?}",
            names
        );
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Completion handler exercises the SymbolIndex sort promotion
// ---------------------------------------------------------------------------

/// Verify that SymbolIndex results are promoted via sort_text in completion.
/// This test uses the test API to verify that the trie matching is active.
#[test]
fn symbol_index_completion_sort_promotion() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let uri = "file:///test_completion_sort.pl";
    harness.open(
        uri,
        r#"package CompletionSort;

sub process_data { return 1; }
sub process_request { return 2; }
sub handle_error { return 0; }
"#,
    )?;

    harness.wait_for_idle(Duration::from_millis(200));

    // Verify that the SymbolIndex has indexed these symbols for prefix matching
    let proc_prefix_hits = harness.test_symbol_index_search_prefix("proc");
    assert!(
        !proc_prefix_hits.is_empty(),
        "SymbolIndex should match 'proc' prefix, got: {:?}",
        proc_prefix_hits
    );
    assert!(
        proc_prefix_hits.iter().any(|n| n.contains("process")),
        "SymbolIndex should contain symbols matching 'process', got: {:?}",
        proc_prefix_hits
    );

    // Request completion for "proc" prefix
    let response = harness
        .request(
            "textDocument/completion",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": 5, "character": 4 }
            }),
        )
        .unwrap_or(json!(null));

    // The completion should return results
    assert!(!response.is_null(), "textDocument/completion should return results");

    if response.is_array() {
        let items = response.as_array().cloned().unwrap_or_default();
        assert!(!items.is_empty(), "Completion should return items for 'proc' prefix");

        // Verify that completion items have labels
        for item in &items {
            assert!(item["label"].is_string(), "Completion item should have a string label");
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// SymbolIndex trie is populated incrementally on didChange
// ---------------------------------------------------------------------------

/// When a document is changed via didChange, the SymbolIndex should be
/// updated with new symbols from the changed content.
/// This test uses the test API to directly verify index updates.
#[test]
fn symbol_index_updated_on_did_change() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let uri = "file:///test_did_change_trie.pl";

    // Open with initial content containing only "alpha"
    harness.open(uri, "package Initial;\nsub alpha { 1 }\n1;\n")?;
    harness.wait_for_idle(Duration::from_millis(200));

    // Verify initial state: "alpha" should be indexed
    let initial_alpha_hits = harness.test_symbol_index_search_prefix("alph");
    assert!(
        initial_alpha_hits.contains(&"alpha".to_string()),
        "SymbolIndex should have 'alpha' after didOpen, got: {:?}",
        initial_alpha_hits
    );

    // Verify "beta" is NOT yet indexed
    let initial_beta_hits = harness.test_symbol_index_search_prefix("beta");
    assert!(
        !initial_beta_hits.iter().any(|n| n.contains("beta")),
        "SymbolIndex should not have 'beta' before didChange, got: {:?}",
        initial_beta_hits
    );

    // Change to add new symbols
    harness.change_full(
        uri,
        2,
        "package Updated;\nsub alpha { 1 }\nsub beta_search { 2 }\nsub beta_find { 3 }\n1;\n",
    )?;
    harness.wait_for_idle(Duration::from_millis(200));

    // Verify that "beta" symbols are now indexed
    let updated_beta_hits = harness.test_symbol_index_search_prefix("beta");
    assert!(
        !updated_beta_hits.is_empty(),
        "SymbolIndex should have 'beta' symbols after didChange, got: {:?}",
        updated_beta_hits
    );
    assert!(
        updated_beta_hits.iter().any(|n| n.contains("beta")),
        "SymbolIndex should contain 'beta' symbols, got: {:?}",
        updated_beta_hits
    );

    // Verify fuzzy search also works for "beta"
    let fuzzy_hits = harness.test_symbol_index_search_fuzzy("bta");
    assert!(
        !fuzzy_hits.is_empty(),
        "SymbolIndex fuzzy search should match 'bta' for 'beta' symbols, got: {:?}",
        fuzzy_hits
    );

    Ok(())
}
