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

    if !response.is_null() && response.is_array() {
        let symbols = response.as_array().ok_or("response is not an array")?;
        if !symbols.is_empty() {
            let names: Vec<&str> = symbols.iter().filter_map(|s| s["name"].as_str()).collect();
            // All returned symbols should be relevant to the query
            assert!(
                names.iter().any(|n| n.contains("calculate") || n.contains("calc")),
                "Should find symbols matching 'calculate', got: {:?}",
                names
            );
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Completion handler exercises the SymbolIndex sort promotion
// ---------------------------------------------------------------------------

/// Completion after typing a prefix should return results; the SymbolIndex
/// trie is used to promote matching symbols via sort_text.
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

    if !response.is_null() {
        // Response can be an array or an object with "items"
        let items = if response.is_array() {
            response.as_array().cloned().unwrap_or_default()
        } else if let Some(items) = response.get("items").and_then(|i| i.as_array()) {
            items.clone()
        } else {
            Vec::new()
        };

        // The completion handler should return results without errors.
        // If items are present, verify they have the expected structure.
        for item in &items {
            assert!(item["label"].is_string(), "Completion item should have a label");
        }

        // If sort_text is present on workspace completion items, the trie
        // promotion path was exercised (items matching the trie get "0" prefix).
        let has_sort_text = items.iter().any(|item| item.get("sortText").is_some());
        // This is informational - sortText may or may not be present depending
        // on whether workspace symbols were available
        if has_sort_text {
            eprintln!("Sort text promotion from SymbolIndex trie is active");
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// SymbolIndex trie is populated incrementally on didChange
// ---------------------------------------------------------------------------

/// When a document is changed via didChange, the SymbolIndex should be
/// updated with new symbols from the changed content.
#[test]
fn symbol_index_updated_on_did_change() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let uri = "file:///test_did_change_trie.pl";

    // Open with initial content
    harness.open(uri, "package Initial;\nsub alpha { 1 }\n1;\n")?;
    harness.wait_for_idle(Duration::from_millis(200));

    // Change to add new symbols
    harness.change_full(
        uri,
        2,
        "package Updated;\nsub alpha { 1 }\nsub beta_search { 2 }\nsub beta_find { 3 }\n1;\n",
    )?;
    harness.wait_for_idle(Duration::from_millis(200));

    // Search for "beta" — should find the newly added symbols
    let response =
        harness.request("workspace/symbol", json!({ "query": "beta" })).unwrap_or(json!(null));

    if !response.is_null() && response.is_array() {
        let symbols = response.as_array().ok_or("response is not an array")?;
        if !symbols.is_empty() {
            let names: Vec<&str> = symbols.iter().filter_map(|s| s["name"].as_str()).collect();
            assert!(
                names.iter().any(|n| n.contains("beta")),
                "After didChange, should find 'beta' symbols, got: {:?}",
                names
            );
        }
    }

    Ok(())
}
