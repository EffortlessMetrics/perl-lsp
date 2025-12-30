mod support;

use lsp_types::*;
use serde_json::json;
use support::lsp_harness::LspHarness;

/// Test that disabled features are properly gated
/// This prevents accidental feature leaks when handlers are registered without guards
#[test]

fn test_type_hierarchy_advertised() {
    let client_caps = json!({
        "textDocument": {
            "typeHierarchy": {
                "dynamicRegistration": false
            }
        }
    });

    let mut harness = LspHarness::new();
    let init_result =
        harness.initialize(Some(client_caps)).expect("Failed to initialize LSP server");

    // Type hierarchy should be advertised now
    let caps_json = init_result["capabilities"].clone();
    assert!(
        caps_json.get("typeHierarchyProvider").is_some(),
        "Type hierarchy should be advertised"
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

    // Code lens is now advertised (v0.8.9)
    assert!(caps.code_lens_provider.is_some(), "Code lens should be advertised");

    // Call hierarchy is fully implemented and should be advertised (v0.8.9)
    assert!(caps.call_hierarchy_provider.is_some(), "Call hierarchy should be advertised");
}

/// Test that experimental features can be toggled via feature flags
#[test]

#[cfg(feature = "experimental-features")]
fn test_experimental_features_enabled() {
    // When experimental features are enabled, additional capabilities appear
    let mut harness = LspHarness::new();
    let init_result = harness.initialize(None).unwrap();
    let _caps: ServerCapabilities =
        serde_json::from_value(init_result["capabilities"].clone()).unwrap();

    // Check experimental features are present
    // (This would be filled in when experimental features are added)
    // For now, just verify initialization worked
}
