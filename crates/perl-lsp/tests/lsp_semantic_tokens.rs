use perl_parser::lsp_server::{JsonRpcRequest, LspServer};
use serde_json::json;

#[test]

fn semantic_tokens_emit_data() {
    let mut srv = LspServer::new();
    let init = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "initialize".into(),
        params: Some(json!({"capabilities":{}})),
    };
    srv.handle_request(init);

    let uri = "file:///tokens.pl";
    let text = r#"package Foo; my $x = 1; sub bar { return $x } $x = 2; bar();"#;
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
        method: "textDocument/semanticTokens/full".into(),
        params: Some(json!({"textDocument": {"uri": uri}})),
    };
    let res = srv.handle_request(req).unwrap();
    let result = res.result.unwrap();
    let arr = result["data"].as_array().unwrap();
    assert!(!arr.is_empty(), "semantic tokens should return data");

    // Verify encoding is valid (5-tuples)
    assert_eq!(arr.len() % 5, 0, "semantic tokens must be 5-tuples");
}
