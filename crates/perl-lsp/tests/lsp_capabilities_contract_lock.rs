#![cfg(feature = "lsp-ga-lock")]
use perl_lsp::{JsonRpcRequest, LspServer};
use serde_json::json;

#[test]
fn locked_capabilities_are_conservative() -> Result<(), Box<dyn std::error::Error>> {
    let mut srv = LspServer::new();
    let init = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "initialize".into(),
        params: Some(json!({"capabilities":{}})),
    };
    let res = srv.handle_request(init).ok_or("Request handling failed")?;
    let result = res.result.ok_or("missing result field")?;
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

    // ADVERTISED in lock mode (now considered stable)
    for k in [
        "workspaceSymbolProvider",
        "renameProvider",
        "codeActionProvider",
        "semanticTokensProvider",
        "inlayHintProvider",
        "documentLinkProvider",
        "selectionRangeProvider",
        "documentOnTypeFormattingProvider",
        "executeCommandProvider",
        "typeHierarchyProvider",
        "callHierarchyProvider",
    ] {
        let cap_value = caps.get(k).ok_or_else(|| format!("Missing capability: {}", k))?;
        assert!(!cap_value.is_null(), "locked mode SHOULD advertise {} (now stable)", k);
    }

    // NOT advertised in lock mode (still experimental/risky)
    for k in [
        "typeDefinitionProvider",
        "implementationProvider",
        "linkedEditingRangeProvider",
        "inlineValueProvider",
        "monikerProvider",
        "colorProvider",
    ] {
        assert!(caps.get(k).is_none(), "locked mode should not advertise {}", k);
    }

    // codeLens is explicitly null or missing
    if let Some(cl) = caps.get("codeLensProvider") {
        assert!(cl.is_null(), "codeLensProvider must be null in lock mode");
    }

    Ok(())
}
