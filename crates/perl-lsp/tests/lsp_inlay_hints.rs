use perl_lsp::{JsonRpcRequest, LspServer};
use serde_json::json;

#[test]

fn inlay_hints_for_substr_and_types() {
    let mut srv = LspServer::new();
    let init = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "initialize".into(),
        params: Some(json!({"capabilities":{}})),
    };
    srv.handle_request(init);

    // Send initialized notification to complete handshake
    let initialized = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "initialized".into(),
        params: Some(json!({})),
    };
    srv.handle_request(initialized);

    let uri = "file:///hints.pl";
    let text = r#"my $s = "abcd"; my $x = substr($s, 1, 2);"#;
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
        method: "textDocument/inlayHint".into(),
        params: Some(json!({
            "textDocument": {"uri": uri},
            "range": {
                "start": {"line": 0, "character": 0},
                "end": {"line": 10, "character": 0}
            }
        })),
    };
    let res = srv.handle_request(req).unwrap();
    let result = res.result.unwrap();
    let hints = result.as_array().unwrap();

    // Should have at least one hint
    assert!(!hints.is_empty(), "should have inlay hints");

    // Check that hints have the expected structure
    for hint in hints {
        assert!(hint.get("position").is_some(), "hint should have position");
        assert!(hint.get("label").is_some(), "hint should have label");
    }

    // Collect all labels
    let labels: Vec<_> =
        hints.iter().filter_map(|h| h.get("label").and_then(|l| l.as_str())).collect();

    // Should have parameter hints for substr
    let param_hints = ["str:", "offset:", "len:"];
    let has_substr_hints = param_hints.iter().any(|&h| labels.contains(&h));
    assert!(has_substr_hints, "should have substr parameter hints, found: {:?}", labels);
}
