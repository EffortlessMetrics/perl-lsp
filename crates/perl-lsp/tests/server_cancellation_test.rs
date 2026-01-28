use perl_lsp::{JsonRpcRequest, LspServer};
use serde_json::json;

#[test]
fn server_side_cancellation_emits_err_server_cancelled() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = LspServer::new();

    // Initialize server
    let _ = server.handle_request(serde_json::from_value::<JsonRpcRequest>(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "rootUri": null,
            "capabilities": {}
        }
    }))?);
    let _ = server.handle_request(serde_json::from_value::<JsonRpcRequest>(json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "initialized",
        "params": {}
    }))?);

    // Request slow operation with server-side timeout
    let response = server.handle_request(serde_json::from_value::<JsonRpcRequest>(json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "$/test/slowOperation",
        "params": {"serverTimeoutMs": 200}
    }))?);

    let resp = response.ok_or("expected JSON-RPC response")?;
    let err = resp.error.ok_or("expected error response")?;
    assert_eq!(err.code, -32802, "expected ERR_SERVER_CANCELLED");

    Ok(())
}
