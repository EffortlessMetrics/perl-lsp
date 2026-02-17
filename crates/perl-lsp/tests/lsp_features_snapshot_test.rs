use insta::assert_yaml_snapshot;
use perl_lsp::features::map::feature_ids_from_caps;
use perl_lsp::features::{advertised_features, compliance_percent};
use serde_json::json;

mod support;
use support::lsp_harness::LspHarness;

#[test]
fn test_advertised_features_match_capabilities() -> Result<(), Box<dyn std::error::Error>> {
    use lsp_types::*;

    // Use shared client capabilities for consistency
    let client_caps = support::client_caps::full();

    // Get real ServerCapabilities from actual LSP initialization
    let mut harness = LspHarness::new();
    let init_result = harness.initialize(Some(client_caps))?;

    // Extract ServerCapabilities from initialization result
    let caps: ServerCapabilities = serde_json::from_value(init_result["capabilities"].clone())?;

    // Get features from capabilities and catalog
    let mut from_caps = feature_ids_from_caps(&caps);
    from_caps.sort();

    let mut from_catalog = advertised_features();
    from_catalog.sort();

    // Create snapshot data
    let snapshot_data = json!({
        "catalog": from_catalog,
        "caps": from_caps,
    });

    // Assert with insta snapshot
    assert_yaml_snapshot!("advertised_vs_caps", &snapshot_data);

    // Also verify compliance percentage is reasonable
    let p = compliance_percent();
    assert!((95.0..=100.0).contains(&p), "unexpected compliance percent: {}", p);

    Ok(())
}

#[test]
fn test_lsp_318_features_present() {
    let advertised = advertised_features();

    // LSP 3.17/3.18 specific features that should be present
    let expected_features = [
        "lsp.pull_diagnostics", // LSP 3.17
                                // Note: type_hierarchy not in lsp-types 0.97 yet
                                // Future LSP 3.18 features to add:
                                // "lsp.inline_completions",
                                // "lsp.notebook_document",
    ];

    for feature in expected_features {
        assert!(advertised.contains(&feature), "LSP feature {} should be advertised", feature);
    }

    // Validate feature count is reasonable (v0.8.8 has comprehensive LSP feature set)
    assert!(!advertised.is_empty(), "Should have advertised features");
    assert!(advertised.len() >= 10, "Should have at least 10 advertised features");
    // Upper bound check removed - feature count grows with LSP version support
}
