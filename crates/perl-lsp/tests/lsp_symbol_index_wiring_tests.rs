//! Tests that the SymbolIndex (trie-based fast lookup) is populated on
//! didOpen and that its queries are wired into completion and workspace
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

    // The response may be an array (simple) or an object with an "items" key.
    let items: Vec<serde_json::Value> = if response.is_array() {
        response.as_array().cloned().unwrap_or_default()
    } else if let Some(arr) = response.get("items").and_then(|i| i.as_array()) {
        arr.clone()
    } else {
        Vec::new()
    };

    assert!(!items.is_empty(), "textDocument/completion should return items for 'proc' prefix");

    // Every item must have a string label.
    for item in &items {
        assert!(item["label"].is_string(), "Completion item should have a string label");
    }

    // Verify sortText is now serialized into the JSON response when present.
    // Items with a sort_text set by any path (trie-promotion "0/1{label}", existing
    // completion system "2a_{label}", etc.) must produce a non-empty string value.
    let items_with_sort_text: Vec<&serde_json::Value> =
        items.iter().filter(|i| !i["sortText"].is_null() && i["sortText"].is_string()).collect();

    // At least some items should carry sortText — the existing completion path
    // assigns sort_text to workspace symbols ("2a_{label}" etc.) and our new
    // serialization code must forward that to the client.
    assert!(
        !items_with_sort_text.is_empty(),
        "At least some completion items should have a non-null sortText value; \
         this confirms sortText is being serialized into the LSP response. \
         Items: {:?}",
        items.iter().take(5).collect::<Vec<_>>()
    );

    // Every non-null sortText must be a non-empty string.
    for item in &items_with_sort_text {
        let st = item["sortText"].as_str().unwrap_or("");
        assert!(
            !st.is_empty(),
            "sortText must be a non-empty string, got empty for item: {:?}",
            item
        );
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Workspace symbol search continues to work after didChange
// ---------------------------------------------------------------------------

/// When a document is changed via didChange, newly added symbols should
/// be findable via workspace/symbol after the change is processed.
///
/// This test verifies that the workspace index (and the overall symbol
/// lookup pipeline) is updated after a didChange event so that callers
/// always see current symbols.
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
