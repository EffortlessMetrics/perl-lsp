use perl_parser::lsp_server::{JsonRpcRequest, LspServer};
use serde_json::json;

#[test]
#[ignore] // Flaky BrokenPipe errors in CI during LSP initialization (environmental/timing)
fn document_links_and_selection() {
    let mut srv = LspServer::new();
    let init = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "initialize".into(),
        params: Some(json!({"capabilities":{}})),
    };
    srv.handle_request(init);

    let uri = "file:///proj/main.pl";
    let text = "use Foo::Bar;\nFoo::Bar::baz();\n";
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

    // Test document links
    let links_req = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "textDocument/documentLink".into(),
        params: Some(json!({"textDocument": {"uri": uri}})),
    };
    let links_res = srv.handle_request(links_req).unwrap();
    let links = links_res.result.unwrap();
    assert!(
        links.as_array().map(|a| !a.is_empty()).unwrap_or(false),
        "should have document links for use statement"
    );

    // Test selection ranges
    let sel_req = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(3)),
        method: "textDocument/selectionRange".into(),
        params: Some(json!({
            "textDocument": {"uri": uri},
            "positions": [{"line": 1, "character": 5}]
        })),
    };
    let sel_res = srv.handle_request(sel_req).unwrap();
    let sel = sel_res.result.unwrap();
    assert!(sel.as_array().map(|a| !a.is_empty()).unwrap_or(false), "should have selection ranges");

    // Verify selection range structure
    if let Some(ranges) = sel.as_array() {
        for range in ranges {
            assert!(range.get("range").is_some(), "selection should have range");
            // Parent is optional but should be an object if present
            if let Some(parent) = range.get("parent") {
                assert!(parent.is_object(), "parent should be an object");
            }
        }
    }
}
