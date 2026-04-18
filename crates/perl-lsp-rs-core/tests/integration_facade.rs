//! Integration tests for Wave F facade pattern.
//!
//! These tests exercise the complete facade pattern for the 8 absorbed crates,
//! verifying that:
//! 1. All modules are accessible from perl-lsp-rs-core directly
//! 2. All modules are re-exported via perl-lsp facade
//! 3. Type identity is preserved across facade boundaries
//! 4. Downstream consumers can use the new module paths
//! 5. The capability map is functional end-to-end

/// Test that the complete facade works for a simulated LSP initialization.
///
/// This simulates how the LSP server would use the absorbed crates post-Wave F.
#[test]
fn test_lsp_initialization_facade_shape() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp::capability_map::caps_from_feature_ids;
    use perl_lsp::features::flags::BuildFlags;
    use perl_lsp::features::ids::LSP_COMPLETION;

    // Simulate server initialization:
    // 1. Parse feature flags
    let _flags = BuildFlags::default();

    // 2. Get feature ID constant
    assert_eq!(LSP_COMPLETION, "lsp.completion");

    // 3. Build capabilities from feature IDs
    let feature_ids = &["lsp.completion"];
    let capabilities = caps_from_feature_ids(feature_ids);

    // Verify the capabilities object is properly formed
    assert!(
        capabilities.completion_provider.is_some(),
        "completion should be enabled for lsp.completion feature"
    );

    Ok(())
}

/// Test that a mock governance use case works with the new module paths.
///
/// Governance (Wave G3, published separately) will consume perl-lsp-rs-core
/// post-Wave F. This tests that pattern.
#[test]
fn test_governance_consumer_complete_usage() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::features::contracts::feature_ids_from_caps;
    use perl_lsp_rs_core::features::flags::BuildFlags;
    use perl_lsp_rs_core::features::policy::from_str_name;

    // Governance queries available profiles
    let profile = from_str_name("production");
    assert!(profile.is_some(), "governance should be able to parse 'production' profile");

    // Governance instantiates build flags
    let _flags = BuildFlags::default();

    // Governance queries feature IDs from server capabilities
    let server_caps = lsp_types::ServerCapabilities::default();
    let feature_ids = feature_ids_from_caps(&server_caps);
    assert!(feature_ids.is_empty(), "default server capabilities should produce no feature IDs");

    Ok(())
}

/// Test that a mock protocol use case works with the new module paths.
///
/// Protocol imports contracts and flags to build the initialize response.
#[test]
fn test_protocol_consumer_complete_usage() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::features::contracts::{all_features, feature_ids_from_caps};
    use perl_lsp_rs_core::features::flags::AdvertisedFeatures;

    // Protocol queries all features for capability reporting
    let all = all_features();
    assert!(!all.is_empty(), "protocol should see all features");

    // Protocol extracts feature IDs from server capabilities
    let server_caps = lsp_types::ServerCapabilities::default();
    let ids = feature_ids_from_caps(&server_caps);
    assert!(ids.is_empty(), "default capabilities should produce empty feature list");

    // Protocol builds advertised features structure
    let _advertised = AdvertisedFeatures::default();

    Ok(())
}

/// Test mixed consumption via facade and core (both paths should work).
///
/// Edge case: what if code imports from both perl_lsp and perl_lsp_rs_core?
/// Types should be identical.
#[test]
fn test_mixed_facade_and_core_consumption() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp::features::contracts::all_features as facade_all_features;
    use perl_lsp::features::flags::BuildFlags as FacadeFlags;
    use perl_lsp_rs_core::features::contracts::all_features as core_all_features;
    use perl_lsp_rs_core::features::flags::BuildFlags as CoreFlags;

    // Types should be identical
    let facade_type = std::any::type_name::<FacadeFlags>();
    let core_type = std::any::type_name::<CoreFlags>();
    assert_eq!(facade_type, core_type, "BuildFlags via facade and core should be the same type");

    // Functions should be identical
    let facade_count = facade_all_features().len();
    let core_count = core_all_features().len();
    assert_eq!(
        facade_count, core_count,
        "all_features() should return the same count via both paths"
    );

    Ok(())
}

/// Test that all 8 feature modules export public API surfaces.
///
/// Regression guard: ensure the absorbed modules didn't lose their public API
/// during the move to perl-lsp-rs-core/src/features/*.
#[test]
fn test_all_feature_modules_export_api() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::features::contracts::{Feature, all_features};
    use perl_lsp_rs_core::features::flags::BuildFlags;
    use perl_lsp_rs_core::features::grid::to_json as grid_to_json;
    use perl_lsp_rs_core::features::ids::LSP_COMPLETION;
    use perl_lsp_rs_core::features::policy::from_str_name;
    use perl_lsp_rs_core::features::profile::from_str_name as profile_from_str_name;
    use perl_lsp_rs_core::features::profile_cli::feature_profile_label;

    // contracts
    assert!(!all_features().is_empty());
    let _ = std::any::type_name::<Feature>();

    // flags
    let _ = std::any::type_name::<BuildFlags>();

    // grid
    let json = grid_to_json();
    assert!(!json.is_empty(), "grid to_json should produce non-empty output");

    // ids
    assert_eq!(LSP_COMPLETION, "lsp.completion");

    // policy
    let prof = from_str_name("production");
    assert!(prof.is_some());

    // profile
    let prof2 = profile_from_str_name("ga-lock");
    assert!(prof2.is_some());

    // profile_cli
    use perl_lsp_rs_core::features::policy::FeatureProfile;
    let label = feature_profile_label(FeatureProfile::GaLock);
    assert!(!label.is_empty(), "feature_profile_label should produce non-empty output");

    Ok(())
}

/// Test that capability_map can be accessed from both core and facade.
///
/// Regression guard: capability_map is a special case (top-level, not in features/).
#[test]
fn test_capability_map_dual_access_complete() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp::capability_map as facade_cm;
    use perl_lsp_rs_core::capability_map as core_cm;

    // Both should work
    let facade_caps = facade_cm::caps_from_feature_ids(&["lsp.completion"]);
    let core_caps = core_cm::caps_from_feature_ids(&["lsp.completion"]);

    // Results should be identical
    assert!(facade_caps.completion_provider.is_some(), "facade should enable completion");
    assert!(core_caps.completion_provider.is_some(), "core should enable completion");

    Ok(())
}

/// Test that the feature catalog is stable and well-formed.
///
/// Regression guard: build.rs generated the catalog; it should be stable and
/// usable for feature queries.
#[test]
fn test_feature_catalog_stability() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::features::contracts::all_features;

    let catalog1 = all_features();
    let catalog2 = all_features();

    // Should be stable (called multiple times)
    assert_eq!(catalog1.len(), catalog2.len(), "catalog size should be stable");

    // Catalog should not be empty
    assert!(!catalog1.is_empty(), "catalog should have features");

    Ok(())
}

/// Test that capability_map correctly filters features by profile.
///
/// Integration test: capability_map takes a feature list and produces server
/// capabilities. This tests the filtering/mapping logic works.
#[test]
fn test_capability_map_profile_filtering() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp_rs_core::capability_map::caps_from_feature_ids;

    // Empty list
    let empty = caps_from_feature_ids(&[]);
    assert!(empty.completion_provider.is_none());

    // Single feature
    let single = caps_from_feature_ids(&["lsp.completion"]);
    assert!(single.completion_provider.is_some());

    // Multiple features (if they affect different capabilities)
    let multi = caps_from_feature_ids(&["lsp.completion", "lsp.hover"]);
    assert!(multi.completion_provider.is_some());

    Ok(())
}

/// Test that all re-exports from perl-lsp are accessible.
///
/// Regression guard: ensures the facade lib.rs has all necessary re-exports.
#[test]
fn test_perl_lsp_facade_has_all_reexports() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lsp::capability_map;
    use perl_lsp::features::contracts;
    use perl_lsp::features::flags;
    use perl_lsp::features::grid;
    use perl_lsp::features::ids;
    use perl_lsp::features::policy;
    use perl_lsp::features::profile;
    use perl_lsp::features::profile_cli;

    // All modules should be accessible
    let _ = capability_map::caps_from_feature_ids(&[]);
    let _ = contracts::all_features();
    let _ = flags::BuildFlags::default();
    let _ = grid::to_json();
    assert_eq!(ids::LSP_COMPLETION, "lsp.completion");
    assert!(policy::from_str_name("production").is_some());
    assert!(profile::from_str_name("ga-lock").is_some());
    let label = profile_cli::feature_profile_label(policy::from_str_name("ga-lock").unwrap());
    assert!(!label.is_empty());

    Ok(())
}
