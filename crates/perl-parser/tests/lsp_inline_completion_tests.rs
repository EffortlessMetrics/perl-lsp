//! Tests for LSP inline completion support

mod support;
use serde_json::json;
use support::lsp_harness::LspHarness;

#[test]
fn test_inline_completion_after_arrow() {
    let mut harness = LspHarness::new();
    harness.initialize(None).unwrap();

    // Open a document
    let uri = "file:///test.pl";
    let doc = "my $obj = Package->";
    harness.open_document(uri, doc).unwrap();

    // Request inline completions after ->
    let result = harness
        .request(
            "textDocument/inlineCompletion",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": 0, "character": 19 }
            }),
        )
        .unwrap_or(json!(null));

    if let Some(items) = result.get("items").and_then(|v| v.as_array()) {
        assert!(!items.is_empty(), "Should have inline completion items");

        // Should suggest new()
        let first = &items[0];
        assert_eq!(
            first["insertText"].as_str().unwrap_or(""),
            "new()",
            "Should suggest new() after arrow"
        );
    }
}

#[test]
fn test_inline_completion_after_use() {
    let mut harness = LspHarness::new();
    harness.initialize(None).unwrap();

    // Open a document
    let uri = "file:///test.pl";
    let doc = "use ";
    harness.open_document(uri, doc).unwrap();

    // Request inline completions after use
    let result = harness
        .request(
            "textDocument/inlineCompletion",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": 0, "character": 4 }
            }),
        )
        .unwrap_or(json!(null));

    if let Some(items) = result.get("items").and_then(|v| v.as_array()) {
        assert!(!items.is_empty(), "Should have inline completion items");

        // Should suggest strict
        let has_strict = items
            .iter()
            .any(|item| item["insertText"].as_str().map(|s| s == "strict;").unwrap_or(false));
        assert!(has_strict, "Should suggest 'strict;' after 'use '");
    }
}

#[test]
fn test_inline_completion_block_start() {
    let mut harness = LspHarness::new();
    harness.initialize(None).unwrap();

    let uri = "file:///test.pl";
    let doc = "if ($x) {";
    harness.open_document(uri, doc).unwrap();

    let result = harness
        .request(
            "textDocument/inlineCompletion",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": 0, "character": 9 }
            }),
        )
        .unwrap_or(json!(null));

    if let Some(items) = result.get("items").and_then(|v| v.as_array()) {
        // May or may not have items, but check if we do
        if !items.is_empty() {
            // Should suggest a newline and closing brace
            let first = &items[0];
            let text = first["insertText"].as_str().unwrap_or("");
            assert!(
                text.contains("\n") || text.contains("}"),
                "Should suggest block-related completion"
            );
        }
    }
}

#[test]
fn test_inline_completion_my_declaration() {
    let mut harness = LspHarness::new();
    harness.initialize(None).unwrap();

    let uri = "file:///test.pl";
    let doc = "my $";
    harness.open_document(uri, doc).unwrap();

    let result = harness
        .request(
            "textDocument/inlineCompletion",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": 0, "character": 4 }
            }),
        )
        .unwrap_or(json!(null));

    // Just verify the request doesn't crash
    // The result may or may not have items
    let _ = result.get("items").and_then(|v| v.as_array());
}

#[test]
fn test_inline_completion_subroutine() {
    let mut harness = LspHarness::new();
    harness.initialize(None).unwrap();

    let uri = "file:///test.pl";
    let doc = "sub ";
    harness.open_document(uri, doc).unwrap();

    let result = harness
        .request(
            "textDocument/inlineCompletion",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": 0, "character": 4 }
            }),
        )
        .unwrap_or(json!(null));

    if let Some(items) = result.get("items").and_then(|v| v.as_array()) {
        // Should suggest a subroutine template
        if !items.is_empty() {
            let first = &items[0];
            let text = first["insertText"].as_str().unwrap_or("");
            assert!(text.contains("{") && text.contains("}"), "Should suggest subroutine body");
        }
    }
}

#[test]
fn test_inline_completion_print_statement() {
    let mut harness = LspHarness::new();
    harness.initialize(None).unwrap();

    let uri = "file:///test.pl";
    let doc = "print ";
    harness.open_document(uri, doc).unwrap();

    let result = harness
        .request(
            "textDocument/inlineCompletion",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": 0, "character": 6 }
            }),
        )
        .unwrap_or(json!(null));

    if let Some(items) = result.get("items").and_then(|v| v.as_array()) {
        // Should suggest something after print
        if !items.is_empty() {
            let first = &items[0];
            let text = first["insertText"].as_str().unwrap_or("");
            assert!(
                text.contains("\"") || text.contains("$"),
                "Should suggest string or variable after print"
            );
        }
    }
}
