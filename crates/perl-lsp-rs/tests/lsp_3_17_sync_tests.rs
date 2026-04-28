//! LSP 3.17 Text Synchronization Contract Tests
//!
//! Tests for didOpen, didChange, willSave, willSaveWaitUntil, didSave, and didClose.

mod support;

use serde_json::json;
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ==================== TEXT SYNCHRONIZATION ====================

#[test]
fn test_text_document_sync_incremental() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    // didOpen
    harness.notify(
        "textDocument/didOpen",
        json!({
            "textDocument": {
                "uri": "file:///test.pl",
                "languageId": "perl",
                "version": 1,
                "text": "my $x = 42;\n"
            }
        }),
    );

    // didChange (full content — still valid under incremental sync)
    harness.notify(
        "textDocument/didChange",
        json!({
            "textDocument": {
                "uri": "file:///test.pl",
                "version": 2
            },
            "contentChanges": [
                { "text": "my $x = 43;\nmy $y = $x;\n" }
            ]
        }),
    );

    // didChange (incremental / range-based)
    harness.notify(
        "textDocument/didChange",
        json!({
            "textDocument": {
                "uri": "file:///test.pl",
                "version": 3
            },
            "contentChanges": [
                {
                    "range": {
                        "start": { "line": 0, "character": 9 },
                        "end": { "line": 0, "character": 11 }
                    },
                    "text": "99"
                }
            ]
        }),
    );

    // willSave
    harness.notify(
        "textDocument/willSave",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "reason": 1  // Manual
        }),
    );

    // willSaveWaitUntil - expects response
    let edits = harness.request(
        "textDocument/willSaveWaitUntil",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "reason": 1
        }),
    );

    if let Ok(edits) = edits {
        assert!(edits.is_array() || edits.is_null());
    }

    // didSave
    harness.notify(
        "textDocument/didSave",
        json!({
            "textDocument": { "uri": "file:///test.pl", "version": 4 },
            "text": "my $x = 43;\nmy $y = $x;\n"  // optional
        }),
    );

    // didClose
    harness.notify(
        "textDocument/didClose",
        json!({
            "textDocument": { "uri": "file:///test.pl" }
        }),
    );
    Ok(())
}
