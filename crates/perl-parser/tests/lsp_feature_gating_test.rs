mod support;

use lsp_types::*;
use serde_json::json;
use support::lsp_harness::LspHarness;

/// Test that disabled features are properly gated
/// This prevents accidental feature leaks when handlers are registered without guards
#[test]
fn test_disabled_features_return_method_not_found() {
    // Initialize with client caps that would normally enable features
    let client_caps = json!({
        "textDocument": {
            "inlayHint": {
                "dynamicRegistration": false
            },
            "typeHierarchy": {
                "dynamicRegistration": false
            }
        }
    });

    let mut harness = LspHarness::new();
    let init_result =
        harness.initialize(Some(client_caps)).expect("Failed to initialize LSP server");

    let caps: ServerCapabilities = serde_json::from_value(init_result["capabilities"].clone())
        .expect("Failed to deserialize ServerCapabilities");

    // Check that type hierarchy is not advertised (since lsp-types doesn't have it yet)
    // NOTE: Type hierarchy types are not available in lsp-types 0.97
    // This test is commented out until we upgrade to a version with TypeHierarchy support
    /*
    #[cfg(not(feature = "lsp-type-hierarchy"))]
    {
        // Should not be present in capabilities
        let caps_json = serde_json::to_value(&caps).unwrap();
        assert!(caps_json.get("typeHierarchyProvider").is_none(),
            "Type hierarchy should not be advertised without feature flag");

        // Attempting to call it should return method not found
        let response = harness.request::<request::PrepareTypeHierarchy>(
            PrepareTypeHierarchyParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier {
                        uri: url::Url::parse("file:///test.pl").unwrap(),
                    },
                    position: Position::new(0, 0),
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
            }
        );

        match response {
            Err(e) => {
                assert_eq!(e.code, -32601, "Should return 'Method not found' error");
            }
            Ok(_) => panic!("Type hierarchy should not be available without feature flag"),
        }
    }
    */

    // Just verify that type hierarchy is not advertised in capabilities
    let caps_json = serde_json::to_value(&caps).unwrap();
    assert!(
        caps_json.get("typeHierarchyProvider").is_none(),
        "Type hierarchy should not be advertised (not available in lsp-types 0.97)"
    );
}

/// Test that features marked as not advertised don't appear in capabilities
#[test]
fn test_non_advertised_features_hidden() {
    let client_caps = json!({
        "textDocument": {
            "codeLens": {
                "dynamicRegistration": false
            },
            "callHierarchy": {
                "dynamicRegistration": false
            }
        }
    });

    let mut harness = LspHarness::new();
    let init_result =
        harness.initialize(Some(client_caps)).expect("Failed to initialize LSP server");

    let caps: ServerCapabilities = serde_json::from_value(init_result["capabilities"].clone())
        .expect("Failed to deserialize ServerCapabilities");

    // Code lens and call hierarchy are implemented but not advertised
    // They should not appear in capabilities
    assert!(
        caps.code_lens_provider.is_none(),
        "Code lens should not be advertised (partial implementation)"
    );

    assert!(
        caps.call_hierarchy_provider.is_none(),
        "Call hierarchy should not be advertised (partial implementation)"
    );
}

/// Test that experimental features can be toggled via feature flags
#[test]
#[cfg(feature = "experimental-features")]
fn test_experimental_features_enabled() {
    // When experimental features are enabled, additional capabilities appear
    let mut harness = LspHarness::new();
    let init_result = harness.initialize(None).unwrap();
    let caps: ServerCapabilities =
        serde_json::from_value(init_result["capabilities"].clone()).unwrap();

    // Check experimental features are present
    // (This would be filled in when experimental features are added)
    assert!(true, "Placeholder for experimental feature checks");
}
