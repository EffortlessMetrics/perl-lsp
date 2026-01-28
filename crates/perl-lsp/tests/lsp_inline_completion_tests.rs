//! Tests for LSP inline completion support

use perl_lsp::{JsonRpcRequest, LspServer};
use serde_json::json;

fn setup_server() -> Result<LspServer, Box<dyn std::error::Error>> {
    let mut server = LspServer::new();

    // Initialize the server
    let init_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "initialize".to_string(),
        params: Some(json!({
            "processId": 1,
            "capabilities": {}
        })),
        id: Some(json!(1)),
    };

    server.handle_request(init_request);

    // Send initialized notification per LSP 3.17 protocol requirements
    let initialized_notification = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "initialized".to_string(),
        params: Some(json!({})),
    };
    server.handle_request(initialized_notification);

    Ok(server)
}

fn open_doc(server: &mut LspServer, uri: &str, text: &str) {
    let request = JsonRpcRequest {
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
    server.handle_request(request);
}

fn inline_completion(
    server: &mut LspServer,
    uri: &str,
    line: u32,
    character: u32,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "textDocument/inlineCompletion".into(),
        params: Some(json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        })),
    };
    let response = server.handle_request(request).ok_or("inline completion response")?;
    response.result.ok_or("result field present".into())
}

#[test]
fn test_inline_completion_after_arrow() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = setup_server()?;
    let uri = "file:///test.pl";
    open_doc(&mut server, uri, "my $obj = Package->");
    let result = inline_completion(&mut server, uri, 0, 19)?;
    let items = result["items"].as_array().ok_or("items array")?;
    assert!(!items.is_empty());
    assert_eq!(items[0]["insertText"].as_str().ok_or("insertText not a string")?, "new()");
    Ok(())
}

#[test]
fn test_inline_completion_after_use() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = setup_server()?;
    let uri = "file:///test.pl";
    open_doc(&mut server, uri, "use ");
    let result = inline_completion(&mut server, uri, 0, 4)?;
    let items = result["items"].as_array().ok_or("items array")?;
    assert!(!items.is_empty());
    let mut suggestions = Vec::new();
    for item in items.iter() {
        let text = item["insertText"].as_str().ok_or("insertText not a string")?;
        suggestions.push(text.to_string());
    }
    assert!(suggestions.contains(&"strict;".to_string()));
    assert!(suggestions.contains(&"warnings;".to_string()));
    Ok(())
}

#[test]
fn test_inline_completion_shebang() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = setup_server()?;
    let uri = "file:///test.pl";
    open_doc(&mut server, uri, "#!");
    let result = inline_completion(&mut server, uri, 0, 2)?;
    let items = result["items"].as_array().ok_or("items array")?;
    assert!(!items.is_empty());
    assert_eq!(
        items[0]["insertText"].as_str().ok_or("insertText not a string")?,
        "/usr/bin/env perl"
    );
    Ok(())
}

#[test]
fn test_inline_completion_sub_body() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = setup_server()?;
    let uri = "file:///test.pl";
    open_doc(&mut server, uri, "sub test ");
    let result = inline_completion(&mut server, uri, 0, 9)?;
    let items = result["items"].as_array().ok_or("items array")?;
    assert!(!items.is_empty());
    assert!(items[0]["insertText"].as_str().ok_or("insertText not a string")?.contains("{"));
    Ok(())
}

#[test]
fn test_inline_completion_no_suggestions() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = setup_server()?;
    let uri = "file:///test.pl";
    open_doc(&mut server, uri, "my $x = 42;");
    let result = inline_completion(&mut server, uri, 0, 10)?;
    let items = result["items"].as_array().ok_or("items array")?;
    assert!(items.is_empty());
    Ok(())
}
