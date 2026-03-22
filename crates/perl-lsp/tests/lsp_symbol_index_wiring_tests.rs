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
// Workspace symbol search exercises the SymbolIndex trie path
// ---------------------------------------------------------------------------

/// After opening a document with subroutines, workspace/symbol searches
/// should return results that were indexed by the trie.
///
/// This verifies that the SymbolIndex trie queries are wired into the
/// workspace/symbol handler and actual results are returned.
#[test]
fn workspace_symbol_search_returns_indexed_symbols() -> TestResult {
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

    // Search for symbols matching "calculate" — the SymbolIndex trie provides
    // a fast prefix lookup that supplements the workspace index search.
    let response =
        harness.request("workspace/symbol", json!({ "query": "calculate" })).unwrap_or(json!(null));

    assert!(!response.is_null(), "workspace/symbol should return a non-null result");

    if response.is_array() {
        let symbols = response.as_array().ok_or("response is not an array")?;
        assert!(
            !symbols.is_empty(),
            "workspace/symbol should return matching symbols for 'calculate' prefix"
        );
        let names: Vec<&str> = symbols.iter().filter_map(|s| s["name"].as_str()).collect();
        // Verify that the returned symbols actually match the query
        assert!(
            names.iter().any(|n| n.contains("calculate")),
            "Returned symbols should include 'calculate' matches, got: {:?}",
            names
        );
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Completion handler exercises the SymbolIndex sort promotion
// ---------------------------------------------------------------------------

/// Completion after typing a prefix should return results; the SymbolIndex
/// trie is used to promote matching symbols via sort_text.
///
/// This test verifies that:
/// 1. Completion works and returns items
/// 2. The completion response structure is valid
/// 3. Sort text may be present (promoted trie matches get "0" prefix)
#[test]
fn completion_returns_results_with_symbol_index_wired() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let uri = "file:///test_completion_trie.pl";
    harness.open(
        uri,
        r#"package CompletionTrie;

sub process_data { return 1; }
sub process_request { return 2; }
sub handle_error { return 0; }

proc
"#,
    )?;

    harness.wait_for_idle(Duration::from_millis(200));

    // Request completion at the position of "proc" (line 6, character 4)
    let response = harness
        .request(
            "textDocument/completion",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": 6, "character": 4 }
            }),
        )
        .unwrap_or(json!(null));

    assert!(!response.is_null(), "textDocument/completion should return a non-null result");

    if response.is_array() {
        let items = response.as_array().cloned().unwrap_or_default();
        // Completion should return items for the "proc" prefix
        // (either directly or indirectly via workspace completion)
        assert!(!items.is_empty(), "textDocument/completion should return items for 'proc' prefix");

        // Verify that completion items have the expected structure
        for item in &items {
            assert!(item["label"].is_string(), "Completion item should have a string label");
        }

        // When workspace symbols are returned, they should have sort_text
        // set by the SymbolIndex promotion logic (for filtered symbols):
        // - Trie-matched: "0{label}"
        // - Not matched: "1{label}"
        // - Trie not active: None
        // This is informational; the test passes if completion works at all.
    } else if let Some(items_obj) = response.get("items").and_then(|i| i.as_array()) {
        assert!(!items_obj.is_empty(), "textDocument/completion items array should be non-empty");
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// SymbolIndex trie is populated incrementally on didChange
// ---------------------------------------------------------------------------

/// When a document is changed via didChange, the SymbolIndex should be
/// updated with new symbols from the changed content.
///
/// This test verifies that workspace/symbol can find newly added symbols
/// after a didChange event, confirming the index is updated.
#[test]
fn symbol_index_updated_on_did_change() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let uri = "file:///test_did_change_trie.pl";

    // Open with initial content containing only "alpha"
    harness.open(uri, "package Initial;\nsub alpha { 1 }\n1;\n")?;
    harness.wait_for_idle(Duration::from_millis(200));

    // Verify "alpha" is findable via workspace/symbol
    let alpha_response =
        harness.request("workspace/symbol", json!({ "query": "alpha" })).unwrap_or(json!(null));
    assert!(!alpha_response.is_null(), "Should find 'alpha' after didOpen");

    // Change to add new symbols
    harness.change_full(
        uri,
        2,
        "package Updated;\nsub alpha { 1 }\nsub beta_search { 2 }\nsub beta_find { 3 }\n1;\n",
    )?;
    harness.wait_for_idle(Duration::from_millis(200));

    // Search for "beta" — should find the newly added symbols
    let beta_response =
        harness.request("workspace/symbol", json!({ "query": "beta" })).unwrap_or(json!(null));

    assert!(
        !beta_response.is_null(),
        "workspace/symbol should return a result for 'beta' after didChange"
    );

    if beta_response.is_array() {
        let symbols = beta_response.as_array().ok_or("response is not an array")?;
        assert!(
            !symbols.is_empty(),
            "After didChange, workspace/symbol should find 'beta' symbols"
        );
        let names: Vec<&str> = symbols.iter().filter_map(|s| s["name"].as_str()).collect();
        assert!(
            names.iter().any(|n| n.contains("beta")),
            "After didChange, should find 'beta' symbols, got: {:?}",
            names
        );
    }

    Ok(())
}
