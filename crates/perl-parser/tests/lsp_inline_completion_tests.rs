//! Tests for LSP inline completion support

use perl_parser::{JsonRpcRequest, LspServer};
use serde_json::json;

fn open_doc(server: &mut LspServer, uri: &str, text: &str) {
    let req = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "textDocument/didOpen".into(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": text,
            }
        })),
    };
    server.handle_request(req);
}

fn inline_complete(server: &mut LspServer, uri: &str, line: u32, character: u32) -> serde_json::Value {
    let resp = server
        .handle_request(JsonRpcRequest {
            _jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "textDocument/inlineCompletion".into(),
            params: Some(json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character }
            })),
        })
        .unwrap();
    resp.result.unwrap()
}

#[test]
fn test_inline_completion_after_arrow() {
    let mut server = LspServer::new();
    let uri = "file:///test.pl";
    open_doc(&mut server, uri, "my $obj = Package->");

    let items = inline_complete(&mut server, uri, 0, 19)["items"].as_array().unwrap().clone();
    assert!(!items.is_empty());
    let first = &items[0];
    assert_eq!(first["insertText"].as_str().unwrap(), "new()");
}

#[test]
fn test_inline_completion_after_use() {
    let mut server = LspServer::new();
    let uri = "file:///test.pl";
    open_doc(&mut server, uri, "use ");

    let items = inline_complete(&mut server, uri, 0, 4)["items"].as_array().unwrap().clone();
    assert!(!items.is_empty());

    let suggestions: Vec<String> = items
        .iter()
        .map(|i| i["insertText"].as_str().unwrap().to_string())
        .collect();
    assert!(suggestions.contains(&"strict;".to_string()));
    assert!(suggestions.contains(&"warnings;".to_string()));
}

#[test]
fn test_inline_completion_shebang() {
    let mut server = LspServer::new();
    let uri = "file:///test.pl";
    open_doc(&mut server, uri, "#!");

    let items = inline_complete(&mut server, uri, 0, 2)["items"].as_array().unwrap().clone();
    assert!(!items.is_empty());
    let first = &items[0];
    assert_eq!(first["insertText"].as_str().unwrap(), "/usr/bin/env perl");
}

#[test]
fn test_inline_completion_sub_body() {
    let mut server = LspServer::new();
    let uri = "file:///test.pl";
    open_doc(&mut server, uri, "sub test ");

    let items = inline_complete(&mut server, uri, 0, 9)["items"].as_array().unwrap().clone();
    assert!(!items.is_empty());
    let first = &items[0];
    assert!(first["insertText"].as_str().unwrap().contains("{"));
}

#[test]
fn test_inline_completion_no_suggestions() {
    let mut server = LspServer::new();
    let uri = "file:///test.pl";
    open_doc(&mut server, uri, "my $x = 42;");

    let items = inline_complete(&mut server, uri, 0, 10)["items"].as_array().unwrap().clone();
    assert!(items.is_empty());
}

