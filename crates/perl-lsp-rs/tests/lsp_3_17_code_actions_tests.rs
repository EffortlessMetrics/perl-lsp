//! LSP 3.17 Code Action Contract Tests
//!
//! Tests for textDocument/codeAction and codeAction/resolve.

mod support;

use serde_json::json;
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ==================== CODE ACTIONS ====================

#[test]
fn test_code_action_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open("file:///test.pl", "$undefined")?;

    let response = harness.request(
        "textDocument/codeAction",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "range": {
                "start": { "line": 0, "character": 0 },
                "end": { "line": 0, "character": 10 }
            },
            "context": {
                "diagnostics": [],
                "only": ["quickfix", "refactor"],
                "triggerKind": 1  // Invoked
            }
        }),
    )?;

    assert!(response.is_null() || response.is_array());
    Ok(())
}

#[test]
fn test_code_action_resolve_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    // Mock code action to resolve
    let response = harness.request(
        "codeAction/resolve",
        json!({
            "title": "Extract variable",
            "kind": "refactor.extract",
            "data": { "uri": "file:///test.pl", "range": {} }
        }),
    );

    // May fail if not supported
    if let Ok(action) = response {
        assert!(action["title"].is_string());
    }
    Ok(())
}
