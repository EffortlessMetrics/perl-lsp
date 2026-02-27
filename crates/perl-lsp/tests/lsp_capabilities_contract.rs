use perl_lsp::{JsonRpcRequest, LspServer};
use perl_tdd_support::must;
use serde_json::json;

/// Test that ensures LSP capabilities match GA contract
/// This prevents accidental drift where we advertise features that don't work
/// PHASE 1 RE-ENABLED: Pure API contract test, no I/O, safe for CI
#[test]
fn test_ga_capabilities_contract() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = LspServer::new();

    // Send initialize request through public API
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "initialize".to_string(),
        params: Some(json!({
            "processId": null,
            "rootUri": "file:///tmp/test",
            "capabilities": {}
        })),
    };

    let response =
        server.handle_request(request).ok_or("Initialize request failed to return response")?;
    assert!(response.error.is_none(), "Initialize should succeed");
    let caps = response.result.ok_or("Initialize response missing result")?["capabilities"].clone();

    // Assert what SHOULD be advertised (working features)
    assert!(caps["textDocumentSync"].is_object(), "textDocumentSync must be advertised");
    assert!(caps["completionProvider"].is_object(), "completionProvider must be advertised");
    assert_eq!(caps["hoverProvider"], json!(true), "hoverProvider must be true");
    assert_eq!(caps["definitionProvider"], json!(true), "definitionProvider must be true");
    assert_eq!(caps["declarationProvider"], json!(true), "declarationProvider must be true");
    assert_eq!(caps["referencesProvider"], json!(true), "referencesProvider must be true");
    assert_eq!(
        caps["documentHighlightProvider"],
        json!(true),
        "documentHighlightProvider must be true"
    );
    assert!(caps["signatureHelpProvider"].is_object(), "signatureHelpProvider must be advertised");
    assert_eq!(caps["documentSymbolProvider"], json!(true), "documentSymbolProvider must be true");
    assert_eq!(caps["foldingRangeProvider"], json!(true), "foldingRangeProvider must be true");
    // PR 3: Workspace symbols now work via index
    // workspaceSymbolProvider can be either bool or object with resolveProvider
    match &caps["workspaceSymbolProvider"] {
        serde_json::Value::Bool(true) => {}
        serde_json::Value::Object(obj) => {
            assert_eq!(
                obj["resolveProvider"],
                json!(true),
                "workspaceSymbolProvider.resolveProvider must be true"
            );
        }
        other => {
            must(Err::<(), _>(format!("unexpected workspaceSymbolProvider: {:?}", other)));
            unreachable!()
        }
    }

    // Assert what SHOULD be advertised (v0.8.4 features)
    assert!(!caps["renameProvider"].is_null(), "renameProvider must be advertised (v0.8.4)");
    assert!(
        !caps["codeActionProvider"].is_null(),
        "codeActionProvider must be advertised (v0.8.4)"
    );
    assert!(
        !caps["semanticTokensProvider"].is_null(),
        "semanticTokensProvider must be advertised (v0.8.4)"
    );
    assert!(!caps["inlayHintProvider"].is_null(), "inlayHintProvider must be advertised (v0.8.4)");

    // Assert new features that SHOULD be advertised
    #[cfg(not(feature = "lsp-ga-lock"))]
    {
        assert!(caps["codeLensProvider"].is_object(), "codeLensProvider must be advertised");
        assert_eq!(
            caps["codeLensProvider"]["resolveProvider"],
            json!(true),
            "codeLensProvider.resolveProvider must be true"
        );
    }
    #[cfg(feature = "lsp-ga-lock")]
    {
        // CodeLens is disabled in GA lock
        assert!(
            caps["codeLensProvider"].is_null(),
            "codeLensProvider must NOT be advertised in GA lock"
        );
    }
    // v0.8.4 NEW features that ARE implemented
    assert!(
        !caps["documentLinkProvider"].is_null(),
        "documentLinkProvider must be advertised (v0.8.4)"
    );
    assert!(
        !caps["selectionRangeProvider"].is_null(),
        "selectionRangeProvider must be advertised (v0.8.4)"
    );
    assert!(
        !caps["documentOnTypeFormattingProvider"].is_null(),
        "documentOnTypeFormattingProvider must be advertised (v0.8.4)"
    );

    // Features that should NOT be advertised
    assert!(!caps["typeHierarchyProvider"].is_null(), "typeHierarchyProvider must be advertised");
    assert!(!caps["callHierarchyProvider"].is_null(), "callHierarchyProvider must be advertised");
    // ExecuteCommand is now implemented in v0.8.6
    assert!(
        !caps["executeCommandProvider"].is_null(),
        "executeCommandProvider must be advertised (implemented in v0.8.6)"
    );

    // documentFormattingProvider is conditional on perltidy availability - that's OK
    // It can be either true or null depending on environment

    Ok(())
}

/// Test that unsupported methods return proper errors
#[test]

fn test_unsupported_methods_return_error() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = LspServer::new();

    // Initialize first through public API
    let init_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(0)),
        method: "initialize".to_string(),
        params: Some(json!({
            "processId": null,
            "rootUri": "file:///tmp/test",
            "capabilities": {}
        })),
    };
    server.handle_request(init_request);

    let initialized_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "initialized".to_string(),
        params: None,
    };
    server.handle_request(initialized_request);

    // Test that truly unsupported methods return method_not_found error
    // Updated for v0.8.8 - color methods are now implemented, use fictional methods
    let unsupported_methods = [
        "textDocument/notARealMethod", // Fictional - will never be implemented
        "workspace/notImplementedFeature", // Fictional - will never be implemented
    ];

    for method in &unsupported_methods {
        let request = perl_lsp::JsonRpcRequest {
            _jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: method.to_string(),
            params: Some(json!({})),
        };

        let response = server.handle_request(request);
        assert!(response.is_some(), "Method {} should return a response", method);

        let resp = response.ok_or(format!("Method {} failed to return response", method))?;
        let error = resp.error.ok_or(format!("Method {} should return an error", method))?;
        assert_eq!(error.code, -32601, "Method {} should return method_not_found error", method);
    }

    Ok(())
}
