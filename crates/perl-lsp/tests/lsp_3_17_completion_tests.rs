//! LSP 3.17 Completion Contract Tests
//!
//! Tests for textDocument/completion.

mod support;

use serde_json::json;
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ==================== COMPLETION ====================

#[test]
fn test_completion_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open("file:///test.pl", "print $")?;

    let response = harness.request(
        "textDocument/completion",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "position": { "line": 0, "character": 7 },
            "context": {
                "triggerKind": 1,  // Invoked
                "triggerCharacter": "$"
            }
        }),
    )?;

    // Response can be array or CompletionList
    assert!(response.is_array() || (response.is_object() && response.get("items").is_some()));
    Ok(())
}
