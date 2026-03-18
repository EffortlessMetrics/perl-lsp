use insta::assert_yaml_snapshot;
use perl_lsp::features::map::feature_ids_from_caps;
use perl_lsp::features::{advertised_features, compliance_percent};
use perl_lsp_feature_governance::{
    FeatureProfile, catalog_advertised_feature_ids, feature_ids_from_flags, flags_for_profile,
};
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
    from_caps.retain(|feature| !matches!(*feature, "lsp.formatting" | "lsp.range_formatting"));
    from_caps.sort();

    let mut from_catalog: Vec<_> = advertised_features().to_vec();
    from_catalog.sort();

    // Formatting capabilities depend on runtime tool availability (for example
    // whether perltidy is installed in CI), so keep the snapshot focused on
    // deterministic capability coverage.

    let profile_snapshots: Vec<_> =
        [FeatureProfile::GaLock, FeatureProfile::Production, FeatureProfile::All]
            .into_iter()
            .map(|profile| {
                let mut from_flags = feature_ids_from_flags(&flags_for_profile(profile));
                from_flags.sort();

                let mut from_profile_catalog = catalog_advertised_feature_ids(profile);
                from_profile_catalog.sort();

                json!({
                    "profile": profile.as_str(),
                    "catalog": from_profile_catalog,
                    "flags": from_flags,
                })
            })
            .collect();

    // Create snapshot data
    let snapshot_data = json!({
        "runtime": {
            "catalog": from_catalog,
            "caps": from_caps,
        },
        "profiles": profile_snapshots,
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
