#![cfg(feature = "lsp-ga-lock")]
use perl_parser::lsp_server::{JsonRpcRequest, LspServer};
use serde_json::json;

#[test]
fn locked_capabilities_are_conservative() {
    let mut srv = LspServer::new();
    let init = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "initialize".into(),
        params: Some(json!({"capabilities":{}})),
    };
    let res = srv.handle_request(init).unwrap();
    let result = res.result.unwrap();
    let caps = &result["capabilities"];

    // Always-on capabilities
    assert_eq!(caps["positionEncoding"], json!("utf-16"));
    assert!(caps["textDocumentSync"].is_object());
    assert_eq!(caps["hoverProvider"], json!(true));
    assert_eq!(caps["definitionProvider"], json!(true));
    assert_eq!(caps["declarationProvider"], json!(true));
    assert_eq!(caps["referencesProvider"], json!(true));
    assert_eq!(caps["documentSymbolProvider"], json!(true));
    assert_eq!(caps["foldingRangeProvider"], json!(true));

    // NOT advertised in lock mode
    for k in [
        "workspaceSymbolProvider",
        "renameProvider",
        "codeActionProvider",
        "semanticTokensProvider",
        "inlayHintProvider",
        "documentLinkProvider",
        "selectionRangeProvider",
        "documentOnTypeFormattingProvider",
    ] {
        assert!(caps.get(k).is_none(), "locked mode should not advertise {}", k);
    }
}
