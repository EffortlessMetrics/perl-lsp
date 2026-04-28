//! Integration test: `perl-lsp-feature-governance` public API reachable via `perl_lsp_rs_core::governance`.

use perl_lsp_rs_core::governance::*;

#[test]
fn governance_module_exposes_advertised_features() {
    // Verify that advertised_features() is accessible post-absorption
    let features = advertised_features();
    assert!(!features.is_empty(), "advertised_features() should return non-empty");
}

#[test]
fn governance_module_exposes_all_features() {
    // Verify that all_features() is accessible post-absorption
    let features = all_features();
    assert!(!features.is_empty(), "all_features() should return non-empty");
}

#[test]
fn governance_module_exposes_has_feature() {
    // Verify that has_feature() is accessible post-absorption
    let result = has_feature("lsp.completion");
    assert!(result, "has_feature('lsp.completion') should be true");
}

#[test]
fn governance_module_exposes_feature_profile_specs() {
    // Verify that feature_profile_specs() is accessible post-absorption
    let specs = feature_profile_specs();
    assert!(!specs.is_empty(), "feature_profile_specs() should return non-empty");
}

#[test]
fn governance_module_exposes_feature_profile_kind() {
    // Verify that FeatureProfileKind enum is accessible post-absorption
    let _: Option<FeatureProfileKind> = None;
}

#[test]
fn governance_module_exposes_supported_cli_profiles() {
    // Verify that supported_cli_profiles() is accessible post-absorption
    let profiles = supported_cli_profiles();
    assert!(!profiles.is_empty(), "supported_cli_profiles() should return non-empty");
}
