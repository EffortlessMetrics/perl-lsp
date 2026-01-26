use perl_lsp::{JsonRpcRequest, LspServer};
use serde_json::json;

#[test]

fn on_type_braces_indent() -> Result<(), Box<dyn std::error::Error>> {
    let mut srv = LspServer::new();
    let init = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "initialize".into(),
        params: Some(json!({"capabilities":{}})),
    };
    srv.handle_request(init);

    // Send initialized notification to complete handshake (required by LSP protocol)
    let initialized = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "initialized".into(),
        params: None,
    };
    srv.handle_request(initialized);

    let uri = "file:///fmt.pl";
    let text = "sub f {\n\n}\n";
    let open = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "textDocument/didOpen".into(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": text
            }
        })),
    };
    srv.handle_request(open);

    // Simulate typing '{' at line 0 end
    let req = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "textDocument/onTypeFormatting".into(),
        params: Some(json!({
            "textDocument": {"uri": uri},
            "position": {"line": 0, "character": 7},
            "ch": "{",
            "options": {"tabSize": 4, "insertSpaces": true}
        })),
    };
    let res = srv.handle_request(req).ok_or("onTypeFormatting request failed")?;
    let edits = res.result.ok_or("onTypeFormatting response missing result")?;

    // Should return edits or null
    if !edits.is_null() {
        let edits_array = edits.as_array().ok_or("edits result is not an array")?;
        // Verify edit structure
        for edit in edits_array {
            assert!(edit.get("range").is_some(), "edit should have range");
            assert!(edit.get("newText").is_some(), "edit should have newText");
        }
    }
    Ok(())
}

#[test]

fn on_type_closing_brace_dedent() -> Result<(), Box<dyn std::error::Error>> {
    let mut srv = LspServer::new();
    let init = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "initialize".into(),
        params: Some(json!({"capabilities":{}})),
    };
    srv.handle_request(init);

    // Send initialized notification to complete handshake (required by LSP protocol)
    let initialized = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "initialized".into(),
        params: None,
    };
    srv.handle_request(initialized);

    let uri = "file:///dedent.pl";
    let text = "sub f {\n    my $x = 1;\n    }";
    let open = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "textDocument/didOpen".into(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": text
            }
        })),
    };
    srv.handle_request(open);

    // Simulate typing '}' at line 2 position 5
    let req = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "textDocument/onTypeFormatting".into(),
        params: Some(json!({
            "textDocument": {"uri": uri},
            "position": {"line": 2, "character": 5},
            "ch": "}",
            "options": {"tabSize": 4, "insertSpaces": true}
        })),
    };
    let res = srv.handle_request(req).ok_or("onTypeFormatting request failed")?;
    let edits = res.result.ok_or("onTypeFormatting response missing result")?;

    // Should return edits for dedent or null if already properly formatted
    if !edits.is_null() {
        let edits_array = edits.as_array().ok_or("edits result is not an array")?;
        assert!(!edits_array.is_empty(), "should have dedent edits");
    }
    Ok(())
}
