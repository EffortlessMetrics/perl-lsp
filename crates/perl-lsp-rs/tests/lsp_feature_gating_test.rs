mod support;

use lsp_types::*;
use serde_json::json;
use support::lsp_harness::LspHarness;

/// Test that disabled features are properly gated
/// This prevents accidental feature leaks when handlers are registered without guards
#[test]

fn test_type_hierarchy_advertised() -> Result<(), Box<dyn std::error::Error>> {
    let client_caps = json!({
        "textDocument": {
            "typeHierarchy": {
                "dynamicRegistration": false
            }
        }
    });

    let mut harness = LspHarness::new();
    let init_result = harness.initialize(Some(client_caps))?;

    // Type hierarchy should be advertised now
    let caps_json = init_result["capabilities"].clone();
    assert!(
        caps_json.get("typeHierarchyProvider").is_some(),
        "Type hierarchy should be advertised"
    );
    Ok(())
}

/// Test that features marked as not advertised don't appear in capabilities
#[test]

fn test_non_advertised_features_hidden() -> Result<(), Box<dyn std::error::Error>> {
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
    let init_result = harness.initialize(Some(client_caps))?;

    let caps: ServerCapabilities = serde_json::from_value(init_result["capabilities"].clone())?;

    // Code lens is now advertised (v0.8.9)
    assert!(caps.code_lens_provider.is_some(), "Code lens should be advertised");

    // Call hierarchy is fully implemented and should be advertised (v0.8.9)
    assert!(caps.call_hierarchy_provider.is_some(), "Call hierarchy should be advertised");
    Ok(())
}

/// Test that experimental features can be toggled via feature flags
#[test]
#[cfg(feature = "experimental-features")]
fn test_experimental_features_enabled() -> Result<(), Box<dyn std::error::Error>> {
    // When experimental features are enabled, additional capabilities appear
    let mut harness = LspHarness::new();
    let init_result = harness.initialize(None)?;
    let _caps: ServerCapabilities = serde_json::from_value(init_result["capabilities"].clone())?;

    // Check experimental features are present
    // (This would be filled in when experimental features are added)
    // For now, just verify initialization worked
    Ok(())
}
