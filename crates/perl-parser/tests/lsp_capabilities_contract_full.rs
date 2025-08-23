#![allow(clippy::collapsible_if)]
use perl_parser::lsp_server::{JsonRpcRequest, LspServer};
use serde_json::json;

#[test]
#[cfg(not(feature = "lsp-ga-lock"))]
fn full_capabilities_match_contract() {
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

    // Full set now that features are implemented & tested
    assert!(caps["workspaceSymbolProvider"].is_object(), "workspaceSymbolProvider should be object");
    assert_eq!(caps["workspaceSymbolProvider"]["resolveProvider"], json!(true));
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
    
    // Type hierarchy is now advertised (v0.8.5)
    assert_eq!(caps["typeHierarchyProvider"], json!(true), "typeHierarchyProvider must be advertised");
    
    // Pull diagnostics is now advertised (v0.8.5)
    assert!(caps["diagnosticProvider"].is_object(), "diagnosticProvider must be advertised");
    let diag = &caps["diagnosticProvider"];
    assert_eq!(diag["interFileDependencies"], json!(false));
    assert_eq!(diag["workspaceDiagnostics"], json!(true));
    
    // Must NOT be advertised until fully supported
    assert!(caps["codeLensProvider"].is_null(), "codeLensProvider must NOT be advertised");
    assert!(caps["executeCommandProvider"].is_null(), "executeCommandProvider must NOT be advertised");
}
