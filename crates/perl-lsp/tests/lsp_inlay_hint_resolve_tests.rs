use perl_lsp::{JsonRpcRequest, LspServer};
use serde_json::json;

/// Test that inlayHint/resolve adds tooltip when requested
#[test]
fn lsp_inlay_hint_resolve_adds_tooltip() -> Result<(), Box<dyn std::error::Error>> {
    let mut srv = LspServer::new();
    let init = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "initialize".into(),
        params: Some(json!({
            "capabilities": {
                "textDocument": {
                    "inlayHint": {
                        "resolveSupport": {
                            "properties": ["tooltip"]
                        }
                    }
                }
            }
        })),
    };
    srv.handle_request(init);

    let initialized = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "initialized".into(),
        params: Some(json!({})),
    };
    srv.handle_request(initialized);

    // Resolve a parameter hint
    let hint = json!({
        "position": {"line": 0, "character": 10},
        "label": "str:",
        "kind": 2,
        "paddingLeft": false,
        "paddingRight": true,
        "data": {
            "uri": "file:///test.pl",
            "function": "substr",
            "paramIndex": 0
        }
    });

    let req = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "inlayHint/resolve".into(),
        params: Some(hint.clone()),
    };

    let res = srv.handle_request(req).ok_or("Failed to handle inlayHint/resolve request")?;
    let result = res.result.ok_or("No result in inlayHint/resolve response")?;

    // Should add tooltip
    assert!(result.get("tooltip").is_some(), "should add tooltip");

    // Should preserve original fields
    assert_eq!(result["label"], "str:");
    assert_eq!(result["kind"], 2);
    assert_eq!(result["data"]["uri"], "file:///test.pl");

    Ok(())
}

/// Test that resolve preserves data field
#[test]
fn lsp_inlay_hint_resolve_preserves_data() -> Result<(), Box<dyn std::error::Error>> {
    let mut srv = LspServer::new();
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
        params: Some(json!({})),
    };
    srv.handle_request(initialized);

    let hint = json!({
        "position": {"line": 5, "character": 20},
        "label": ": Str",
        "kind": 1,
        "paddingLeft": true,
        "paddingRight": false,
        "data": {
            "uri": "file:///test.pl",
            "type": "String",
            "custom": "preserved"
        }
    });

    let req = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "inlayHint/resolve".into(),
        params: Some(hint.clone()),
    };

    let res = srv.handle_request(req).ok_or("Failed to handle inlayHint/resolve request")?;
    let result = res.result.ok_or("No result in inlayHint/resolve response")?;

    // Data field should be preserved
    assert_eq!(result["data"], hint["data"]);
    assert_eq!(result["data"]["custom"], "preserved");

    Ok(())
}

/// Test that resolve returns same hint if already has tooltip
#[test]
fn lsp_inlay_hint_resolve_no_op_when_complete() -> Result<(), Box<dyn std::error::Error>> {
    let mut srv = LspServer::new();
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
        params: Some(json!({})),
    };
    srv.handle_request(initialized);

    // Hint already has tooltip
    let hint = json!({
        "position": {"line": 0, "character": 10},
        "label": "param:",
        "kind": 2,
        "paddingLeft": false,
        "paddingRight": true,
        "tooltip": "Already has tooltip",
        "data": {"uri": "file:///test.pl"}
    });

    let req = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "inlayHint/resolve".into(),
        params: Some(hint.clone()),
    };

    let res = srv.handle_request(req).ok_or("Failed to handle inlayHint/resolve request")?;
    let result = res.result.ok_or("No result in inlayHint/resolve response")?;

    // Should return same hint
    assert_eq!(result["tooltip"], "Already has tooltip");
    assert_eq!(result["label"], "param:");

    Ok(())
}

/// Test that resolve handles missing params gracefully
#[test]
fn lsp_inlay_hint_resolve_handles_invalid_params() -> Result<(), Box<dyn std::error::Error>> {
    let mut srv = LspServer::new();
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
        params: Some(json!({})),
    };
    srv.handle_request(initialized);

    let req = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "inlayHint/resolve".into(),
        params: None,
    };

    let res = srv.handle_request(req).ok_or("Failed to handle inlayHint/resolve request")?;

    // Should return error for invalid params
    assert!(res.error.is_some());
    let error = res.error.ok_or("Expected error for invalid params")?;
    assert_eq!(error.code, -32602); // InvalidParams

    Ok(())
}
