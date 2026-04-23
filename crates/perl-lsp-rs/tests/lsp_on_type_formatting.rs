use perl_lsp::{JsonRpcRequest, LspServer};
use serde_json::json;

#[test]

fn on_type_braces_indent() -> Result<(), Box<dyn std::error::Error>> {
    let srv = LspServer::new();
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
    let srv = LspServer::new();
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

#[test]
fn on_type_newline_after_open_brace_indents() -> Result<(), Box<dyn std::error::Error>> {
    let srv = LspServer::new();
    let init = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "initialize".into(),
        params: Some(json!({"capabilities":{}})),
    };
    srv.handle_request(init);

    let initialized = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "initialized".into(),
        params: None,
    };
    srv.handle_request(initialized);

    let uri = "file:///newline_indent.pl";
    // Document text: line 0 = "sub foo {", line 1 = "" (cursor here after pressing Enter)
    let text = "sub foo {\n\n}\n";
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

    // Simulate pressing Enter after "sub foo {" — cursor is now at line 1, col 0
    let req = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "textDocument/onTypeFormatting".into(),
        params: Some(json!({
            "textDocument": {"uri": uri},
            "position": {"line": 1, "character": 0},
            "ch": "\n",
            "options": {"tabSize": 4, "insertSpaces": true}
        })),
    };
    let res = srv.handle_request(req).ok_or("onTypeFormatting newline request failed")?;
    let edits = res.result.ok_or("onTypeFormatting newline response missing result")?;

    // The \n handler should return non-null edits for indentation after open brace
    assert!(
        !edits.is_null(),
        "onTypeFormatting with ch='\\n' after open brace should return edits, got null"
    );
    let edits_array = edits.as_array().ok_or("edits result is not an array")?;
    assert!(!edits_array.is_empty(), "should return indent edits after open brace newline");
    Ok(())
}

#[test]
fn on_type_tab_size_4_produces_four_space_indent() -> Result<(), Box<dyn std::error::Error>> {
    // Verify that tabSize:4 is actually threaded through to the formatter:
    // pressing Enter after `{` with tabSize=4 should produce "    " (4 spaces),
    // not "  " (the old hardcoded 2-space step).
    let srv = LspServer::new();
    let init = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "initialize".into(),
        params: Some(json!({"capabilities":{}})),
    };
    srv.handle_request(init);
    let initialized = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "initialized".into(),
        params: None,
    };
    srv.handle_request(initialized);

    let uri = "file:///tab_size_4.pl";
    // line 0 = "if ($x) {", line 1 = "" (the new blank line after Enter)
    let text = "if ($x) {\n\n}\n";
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

    let req = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "textDocument/onTypeFormatting".into(),
        params: Some(json!({
            "textDocument": {"uri": uri},
            "position": {"line": 1, "character": 0},
            "ch": "\n",
            "options": {"tabSize": 4, "insertSpaces": true}
        })),
    };
    let res = srv.handle_request(req).ok_or("onTypeFormatting tabSize:4 request failed")?;
    let edits = res.result.ok_or("onTypeFormatting tabSize:4 response missing result")?;

    let edits_array = edits.as_array().ok_or("edits result is not an array")?;
    assert!(!edits_array.is_empty(), "should return indent edit");

    let new_text =
        edits_array[0].get("newText").and_then(|v| v.as_str()).ok_or("edit missing newText")?;
    assert_eq!(new_text, "    ", "tabSize:4 should produce 4 spaces, got {:?}", new_text);
    Ok(())
}
