#![allow(clippy::collapsible_if)]
use serde_json::json;
use perl_parser::lsp_server::{LspServer, JsonRpcRequest};

#[test]
#[cfg(not(feature = "lsp-ga-lock"))]
fn full_capabilities_match_contract() {
    let mut srv = LspServer::new();
    let init = JsonRpcRequest { 
        _jsonrpc: "2.0".into(), 
        id: Some(json!(1)),
        method: "initialize".into(), 
        params: Some(json!({"capabilities":{}})) 
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

    // Full set now that features are implemented & tested
    assert_eq!(caps["workspaceSymbolProvider"], json!(true));
    assert_eq!(caps["renameProvider"], json!(true));
    assert!(caps["codeActionProvider"].is_object());

    let st = &caps["semanticTokensProvider"];
    assert!(st.is_object());
    assert_eq!(st["full"], json!(true));

    let ih = &caps["inlayHintProvider"];
    assert!(ih.is_object());
    assert_eq!(ih["resolveProvider"], json!(false));

    let dl = &caps["documentLinkProvider"];
    assert!(dl.is_object());
    assert_eq!(dl["resolveProvider"], json!(false));

    assert_eq!(caps["selectionRangeProvider"], json!(true));
    let ot = &caps["documentOnTypeFormattingProvider"];
    assert!(ot.is_object());
}