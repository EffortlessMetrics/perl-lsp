//! Tests for LSP inline completion support

use lsp_types::*;
use perl_parser::LspServer;
use serde_json::json;
use std::sync::Arc;

#[test]
fn test_inline_completion_after_arrow() {
    let server = Arc::new(LspServer::new());

    // Open a document
    let uri = "file:///test.pl";
    server
        .did_open(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": "my $obj = Package->"
            }
        }))
        .unwrap();

    // Request inline completions after ->
    let result = server
        .handle_request(
            "textDocument/inlineCompletion",
            Some(json!({
                "textDocument": { "uri": uri },
                "position": { "line": 0, "character": 19 }
            })),
        )
        .unwrap();

    assert!(result.is_some());
    let items = result.unwrap();
    assert!(items.get("items").is_some());
    let items = items["items"].as_array().unwrap();
    assert!(!items.is_empty());

    // Should suggest new()
    let first = &items[0];
    assert_eq!(first["insertText"].as_str().unwrap(), "new()");
}

#[test]
fn test_inline_completion_after_use() {
    let server = Arc::new(LspServer::new());

    let uri = "file:///test.pl";
    server
        .did_open(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": "use "
            }
        }))
        .unwrap();

    let result = server
        .handle_request(
            "textDocument/inlineCompletion",
            Some(json!({
                "textDocument": { "uri": uri },
                "position": { "line": 0, "character": 4 }
            })),
        )
        .unwrap();

    assert!(result.is_some());
    let items = result.unwrap();
    let items = items["items"].as_array().unwrap();
    assert!(!items.is_empty());

    // Should include strict and warnings
    let suggestions: Vec<String> =
        items.iter().map(|i| i["insertText"].as_str().unwrap().to_string()).collect();
    assert!(suggestions.contains(&"strict;".to_string()));
    assert!(suggestions.contains(&"warnings;".to_string()));
}

#[test]
fn test_inline_completion_shebang() {
    let server = Arc::new(LspServer::new());

    let uri = "file:///test.pl";
    server
        .did_open(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": "#!"
            }
        }))
        .unwrap();

    let result = server
        .handle_request(
            "textDocument/inlineCompletion",
            Some(json!({
                "textDocument": { "uri": uri },
                "position": { "line": 0, "character": 2 }
            })),
        )
        .unwrap();

    assert!(result.is_some());
    let items = result.unwrap();
    let items = items["items"].as_array().unwrap();
    assert!(!items.is_empty());

    // Should suggest shebang
    let first = &items[0];
    assert_eq!(first["insertText"].as_str().unwrap(), "/usr/bin/env perl");
}

#[test]
fn test_inline_completion_sub_body() {
    let server = Arc::new(LspServer::new());

    let uri = "file:///test.pl";
    server
        .did_open(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": "sub test "
            }
        }))
        .unwrap();

    let result = server
        .handle_request(
            "textDocument/inlineCompletion",
            Some(json!({
                "textDocument": { "uri": uri },
                "position": { "line": 0, "character": 9 }
            })),
        )
        .unwrap();

    assert!(result.is_some());
    let items = result.unwrap();
    let items = items["items"].as_array().unwrap();
    assert!(!items.is_empty());

    // Should suggest opening brace
    let first = &items[0];
    assert!(first["insertText"].as_str().unwrap().contains("{"));
}

#[test]
fn test_inline_completion_no_suggestions() {
    let server = Arc::new(LspServer::new());

    let uri = "file:///test.pl";
    server
        .did_open(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": "my $x = 42;"
            }
        }))
        .unwrap();

    let result = server
        .handle_request(
            "textDocument/inlineCompletion",
            Some(json!({
                "textDocument": { "uri": uri },
                "position": { "line": 0, "character": 10 }
            })),
        )
        .unwrap();

    assert!(result.is_some());
    let items = result.unwrap();
    let items = items["items"].as_array().unwrap();
    // Should have no suggestions in middle of statement
    assert!(items.is_empty());
}
