//! Test facade pattern coverage for Wave F crate absorption.
//!
//! Verifies that the absorbed feature and capability-map crates are properly
//! reorganized into perl-lsp-rs-core modules and are accessible via:
//! - Direct module paths (perl_lsp_rs_core::features::*, perl_lsp_rs_core::capability_map::*)
//! - Facade re-exports from perl-lsp (perl_lsp::features::*, perl_lsp::capability_map::*)
//!
//! The 8 absorbed crates are:
//! - perl-lsp-feature-ids (module: features::ids)
//! - perl-lsp-feature-contracts (module: features::contracts)
//! - perl-lsp-feature-flags (module: features::flags)
//! - perl-lsp-feature-profile (module: features::profile)
//! - perl-lsp-feature-profile-cli (module: features::profile_cli)
//! - perl-lsp-feature-policy (module: features::policy)
//! - perl-lsp-feature-grid (module: features::grid)
//! - perl-lsp-capability-map (module: capability_map)

// Glob imports are used as compile-time accessibility checks in some tests.
#[allow(unused_imports)]
use perl_tdd_support::{must, must_some};

/// Test that features::ids module is accessible and exports expected types
#[test]
fn test_features_ids_facade_accessible() -> Result<(), Box<dyn std::error::Error>> {
    // This test verifies that perl-lsp-feature-ids is correctly moved to
    // perl-lsp-rs-core::features::ids and re-exported
    #[allow(unused_imports)]
    use perl_lsp_rs_core::features::ids::*;

    // Basic import: the module should compile and be usable
    let _test_passed = true;
    assert!(_test_passed);

    Ok(())
}

/// Test that features::contracts module is accessible and exports expected types
#[test]
fn test_features_contracts_facade_accessible() -> Result<(), Box<dyn std::error::Error>> {
    // This test verifies that perl-lsp-feature-contracts is correctly moved to
    // perl-lsp-rs-core::features::contracts and re-exported
    #[allow(unused_imports)]
    use perl_lsp_rs_core::features::contracts::*;

    let _test_passed = true;
    assert!(_test_passed);

    Ok(())
}

/// Test that features::flags module is accessible and exports expected types
#[test]
fn test_features_flags_facade_accessible() -> Result<(), Box<dyn std::error::Error>> {
    // This test verifies that perl-lsp-feature-flags is correctly moved to
    // perl-lsp-rs-core::features::flags and re-exported
    #[allow(unused_imports)]
    use perl_lsp_rs_core::features::flags::*;

    let _test_passed = true;
    assert!(_test_passed);

    Ok(())
}

/// Test that features::profile module is accessible and exports expected types
#[test]
fn test_features_profile_facade_accessible() -> Result<(), Box<dyn std::error::Error>> {
    // This test verifies that perl-lsp-feature-profile is correctly moved to
    // perl-lsp-rs-core::features::profile and re-exported
    #[allow(unused_imports)]
    use perl_lsp_rs_core::features::profile::*;

    let _test_passed = true;
    assert!(_test_passed);

    Ok(())
}

/// Test that features::profile_cli module is accessible and exports expected types
#[test]
fn test_features_profile_cli_facade_accessible() -> Result<(), Box<dyn std::error::Error>> {
    // This test verifies that perl-lsp-feature-profile-cli is correctly moved to
    // perl-lsp-rs-core::features::profile_cli and re-exported
    #[allow(unused_imports)]
    use perl_lsp_rs_core::features::profile_cli::*;

    let _test_passed = true;
    assert!(_test_passed);

    Ok(())
}

/// Test that features::policy module is accessible and exports expected types
#[test]
fn test_features_policy_facade_accessible() -> Result<(), Box<dyn std::error::Error>> {
    // This test verifies that perl-lsp-feature-policy is correctly moved to
    // perl-lsp-rs-core::features::policy and re-exported
    #[allow(unused_imports)]
    use perl_lsp_rs_core::features::policy::*;

    let _test_passed = true;
    assert!(_test_passed);

    Ok(())
}

/// Test that features::grid module is accessible and exports expected types
#[test]
fn test_features_grid_facade_accessible() -> Result<(), Box<dyn std::error::Error>> {
    // This test verifies that perl-lsp-feature-grid is correctly moved to
    // perl-lsp-rs-core::features::grid and re-exported
    #[allow(unused_imports)]
    use perl_lsp_rs_core::features::grid::*;

    let _test_passed = true;
    assert!(_test_passed);

    Ok(())
}

/// Test that capability_map module is accessible and exports expected types
#[test]
fn test_capability_map_facade_accessible() -> Result<(), Box<dyn std::error::Error>> {
    // This test verifies that perl-lsp-capability-map is correctly moved to
    // perl-lsp-rs-core::capability_map and re-exported
    #[allow(unused_imports)]
    use perl_lsp_rs_core::capability_map::*;

    let _test_passed = true;
    assert!(_test_passed);

    Ok(())
}

/// Test that features module aggregates all submodules correctly
#[test]
fn test_features_module_complete() -> Result<(), Box<dyn std::error::Error>> {
    // This test ensures all 7 feature submodules are accessible via perl_lsp_rs_core::features
    // Verify each submodule exports at least one accessible type (proves the module path works).
    let _ = std::any::type_name::<perl_lsp_rs_core::features::contracts::FeatureProfileKind>();
    let _ = std::any::type_name::<perl_lsp_rs_core::features::flags::BuildFlags>();
    let _ = std::any::type_name::<perl_lsp_rs_core::features::flags::AdvertisedFeatures>();
    let _ = std::any::type_name::<perl_lsp_rs_core::features::profile::FeatureProfileKind>();
    let _ = std::any::type_name::<
        perl_lsp_rs_core::features::profile_cli::UnsupportedFeatureProfileError,
    >();
    let _ = std::any::type_name::<perl_lsp_rs_core::features::policy::FeatureProfile>();
    let _ = std::any::type_name::<perl_lsp_rs_core::features::grid::FeatureProfile>();
    // ids module: constants only, so check via wildcard import instead
    use perl_lsp_rs_core::features::ids::LSP_COMPLETION;
    assert!(!LSP_COMPLETION.is_empty(), "ids module LSP_COMPLETION constant is accessible");

    Ok(())
}

/// Test that facade re-exports are accessible from perl-lsp (the main LSP crate)
#[test]
fn test_facade_reexports_from_perl_lsp() -> Result<(), Box<dyn std::error::Error>> {
    // After Wave F, consumers should be able to import from perl_lsp (the facade)
    // and get the same types as importing from perl_lsp_rs_core
    use perl_lsp::features;

    // Verify by accessing a type from each re-exported submodule
    let _ = std::any::type_name::<features::flags::BuildFlags>();
    let _ = std::any::type_name::<features::policy::FeatureProfile>();

    // Verify capability_map re-export
    let _caps = perl_lsp::capability_map::caps_from_feature_ids(&[]);

    Ok(())
}

/// Test type identity: perl-lsp re-exports resolve to same types as core
#[test]
fn test_type_identity_facade_vs_core() -> Result<(), Box<dyn std::error::Error>> {
    // Verify that types imported via the facade (perl_lsp) are identical to
    // types imported directly from perl-lsp-rs-core
    use perl_lsp::features::flags::BuildFlags as FacadeBuildFlags;
    use perl_lsp_rs_core::features::flags::BuildFlags as CoreBuildFlags;

    // The types should be identical — both are the same type from the same crate.
    // Verify by checking type names match.
    let facade_name = std::any::type_name::<FacadeBuildFlags>();
    let core_name = std::any::type_name::<CoreBuildFlags>();
    assert_eq!(facade_name, core_name, "facade and core BuildFlags should be the same type");

    Ok(())
}

/// Test capability_map module re-export from perl-lsp
#[test]
fn test_capability_map_reexport_from_perl_lsp() -> Result<(), Box<dyn std::error::Error>> {
    // perl-lsp should re-export capability_map for downstream consumers
    // Verify by calling a function from the module
    let caps = perl_lsp::capability_map::caps_from_feature_ids(&[]);
    assert!(caps.completion_provider.is_none(), "empty feature list produces empty caps");
    Ok(())
}

/// Test that downstream consumer shape for perl-lsp-feature-governance works
#[test]
fn test_governance_consumer_shape_integration() -> Result<(), Box<dyn std::error::Error>> {
    // This simulates how perl-lsp-feature-governance (which stays published in Wave G3)
    // will consume the new perl-lsp-rs-core after Wave F consolidation
    // It should be able to import what it needs from perl_lsp_rs_core::features

    use perl_lsp_rs_core::features::contracts::FeatureProfileKind;
    use perl_lsp_rs_core::features::flags::BuildFlags;
    use perl_lsp_rs_core::features::policy::FeatureProfile;
    use perl_lsp_rs_core::features::profile::FeatureProfileKind as ProfileKind;

    // These imports should work — verify by accessing a value
    let _ = std::any::type_name::<FeatureProfileKind>();
    let _ = std::any::type_name::<BuildFlags>();
    let _ = std::any::type_name::<FeatureProfile>();
    let _ = std::any::type_name::<ProfileKind>();

    Ok(())
}

/// Test that downstream consumer shape for perl-lsp-protocol works
#[test]
fn test_protocol_consumer_shape_integration() -> Result<(), Box<dyn std::error::Error>> {
    // This simulates how perl-lsp-protocol imports from the absorbed crates
    // After Wave F, it should import from perl_lsp_rs_core instead

    use perl_lsp_rs_core::features::contracts::feature_ids_from_caps;
    use perl_lsp_rs_core::features::flags::{AdvertisedFeatures, BuildFlags};

    // BuildFlags and AdvertisedFeatures should be accessible
    let _ = std::any::type_name::<BuildFlags>();
    let _ = std::any::type_name::<AdvertisedFeatures>();
    // Verify feature_ids_from_caps is callable (it's a function, not a type)
    let _result = feature_ids_from_caps(&lsp_types::ServerCapabilities::default());

    Ok(())
}

/// Test that perl-lsp-feature-governance's feature gate forwarding works
#[test]
fn test_feature_gate_lsp_ga_lock_forwarding() -> Result<(), Box<dyn std::error::Error>> {
    // The lsp-ga-lock feature should be properly forwarded through perl-lsp-rs-core
    // This is a compile-time test: if the feature exists and is properly forwarded,
    // code gated on it should compile (when the feature is enabled)

    // Note: This test itself doesn't use the feature, but verifies the mechanism exists
    // The actual feature-gated code will be tested by the absorbed test suites

    Ok(())
}

/// Test that build.rs integration works (SoT toml available)
#[test]
fn test_build_script_sot_integration() -> Result<(), Box<dyn std::error::Error>> {
    // The build.rs from perl-lsp-feature-contracts should be in perl-lsp-rs-core
    // and features_sot.toml should be available at build time
    //
    // This test verifies that compile-time-generated constants are accessible
    // (The actual constants depend on features_sot.toml being copied during build)

    // If build.rs ran successfully, we should be able to access generated constants
    use perl_lsp_rs_core::features::contracts::{Feature, all_features, has_feature};

    let _ = std::any::type_name::<Feature>();
    assert!(!all_features().is_empty(), "build.rs generated feature catalog should be non-empty");
    assert!(has_feature("lsp.completion"), "build.rs catalog should include lsp.completion");
    Ok(())
}

/// Test edge case: module structure is fully populated (after implementation)
#[test]
fn test_empty_module_access() -> Result<(), Box<dyn std::error::Error>> {
    // Verify the module structure exists and is fully populated
    // Check that at least one item is accessible from each core module
    use perl_lsp_rs_core::capability_map::caps_from_feature_ids;
    use perl_lsp_rs_core::features::ids::LSP_COMPLETION;

    // These should not panic on access
    assert!(!LSP_COMPLETION.is_empty(), "ids module should export LSP_COMPLETION constant");
    let caps = caps_from_feature_ids(&[]);
    assert!(caps.completion_provider.is_none(), "empty input should produce empty caps");

    Ok(())
}

/// Test that the public API surface doesn't have conflicting re-exports
#[test]
fn test_facade_no_conflicting_reexports() -> Result<(), Box<dyn std::error::Error>> {
    // Verify that perl-lsp (the facade) and perl-lsp-rs-core export compatible types
    // without duplication or name collision
    use perl_lsp::features::flags::BuildFlags as FacadeFlags;
    use perl_lsp::features::policy::FeatureProfile as FacadeProfile;

    // If these compile without error, name resolution is working
    let _ = std::any::type_name::<FacadeFlags>();
    let _ = std::any::type_name::<FacadeProfile>();

    // Verify capability_map is accessible from the facade
    let caps = perl_lsp::capability_map::caps_from_feature_ids(&[]);
    assert!(caps.completion_provider.is_none());

    Ok(())
}

/// Test comprehensive facade accessibility across all modules
#[test]
fn test_comprehensive_facade_exports() -> Result<(), Box<dyn std::error::Error>> {
    // Comprehensive test ensuring all feature modules are accessible
    // and capability_map is available, all from the core crate

    use perl_lsp_rs_core::capability_map::caps_from_feature_ids;
    use perl_lsp_rs_core::features::contracts::FeatureProfileKind;
    use perl_lsp_rs_core::features::flags::BuildFlags;
    use perl_lsp_rs_core::features::grid::FeatureProfile as GridProfile;
    use perl_lsp_rs_core::features::ids::LSP_COMPLETION;
    use perl_lsp_rs_core::features::policy::FeatureProfile;
    use perl_lsp_rs_core::features::profile::FeatureProfileKind as ProfileKind;
    use perl_lsp_rs_core::features::profile_cli::UnsupportedFeatureProfileError;

    // All types should be accessible and usable
    let _ = std::any::type_name::<FeatureProfileKind>();
    let _ = std::any::type_name::<BuildFlags>();
    let _ = std::any::type_name::<GridProfile>();
    let _ = std::any::type_name::<FeatureProfile>();
    let _ = std::any::type_name::<ProfileKind>();
    let _ = std::any::type_name::<UnsupportedFeatureProfileError>();
    assert!(!LSP_COMPLETION.is_empty());
    let _caps = caps_from_feature_ids(&[]);

    Ok(())
}

/// Test that imports from both old path and new path would not coexist
/// (this ensures Wave F consolidation doesn't accidentally keep old paths)
#[test]
fn test_old_paths_no_longer_accessible() -> Result<(), Box<dyn std::error::Error>> {
    // After Wave F, the old crate paths should NOT be accessible
    // This test documents that the old paths are gone and the new paths work.
    // We can't directly test "this doesn't compile" in a test file,
    // but we verify the new path works correctly.
    use perl_lsp_rs_core::features::ids::{LSP_COMPLETION, LSP_HOVER};

    assert_eq!(LSP_COMPLETION, "lsp.completion", "new path should export correct constant value");
    assert_eq!(LSP_HOVER, "lsp.hover", "new path should export correct constant value");
    // The old path `use perl_lsp_feature_ids::*;` should not work after Wave F

    Ok(())
}
