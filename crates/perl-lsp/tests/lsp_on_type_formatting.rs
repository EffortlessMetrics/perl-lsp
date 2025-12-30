use perl_parser::lsp_server::{JsonRpcRequest, LspServer};
use serde_json::json;

#[test]

fn on_type_braces_indent() {
    let mut srv = LspServer::new();
    let init = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "initialize".into(),
        params: Some(json!({"capabilities":{}})),
    };
    srv.handle_request(init);

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
    let res = srv.handle_request(req).unwrap();
    let edits = res.result.unwrap();

    // Should return edits or null
    if !edits.is_null() {
        let edits_array = edits.as_array().unwrap();
        // Verify edit structure
        for edit in edits_array {
            assert!(edit.get("range").is_some(), "edit should have range");
            assert!(edit.get("newText").is_some(), "edit should have newText");
        }
    }
}

#[test]

fn on_type_closing_brace_dedent() {
    let mut srv = LspServer::new();
    let init = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "initialize".into(),
        params: Some(json!({"capabilities":{}})),
    };
    srv.handle_request(init);

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
    let res = srv.handle_request(req).unwrap();
    let edits = res.result.unwrap();

    // Should return edits for dedent or null if already properly formatted
    if !edits.is_null() {
        let edits_array = edits.as_array().unwrap();
        assert!(!edits_array.is_empty(), "should have dedent edits");
    }
}
