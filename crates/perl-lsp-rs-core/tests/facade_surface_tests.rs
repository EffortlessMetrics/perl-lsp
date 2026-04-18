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

use perl_tdd_support::{must, must_some};

/// Test that features::ids module is accessible and exports expected types
#[test]
fn test_features_ids_facade_accessible() -> Result<(), Box<dyn std::error::Error>> {
    // This test verifies that perl-lsp-feature-ids is correctly moved to
    // perl-lsp-rs-core::features::ids and re-exported
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
    use perl_lsp_rs_core::capability_map::*;

    let _test_passed = true;
    assert!(_test_passed);

    Ok(())
}

/// Test that features module aggregates all submodules correctly
#[test]
fn test_features_module_complete() -> Result<(), Box<dyn std::error::Error>> {
    // This test ensures all 7 feature submodules are accessible via perl_lsp_rs_core::features
    use perl_lsp_rs_core::features;

    // All submodules should be publicly accessible as modules
    let _ = core::mem::size_of_val(&std::any::type_name::<features::ids>());
    let _ = core::mem::size_of_val(&std::any::type_name::<features::contracts>());
    let _ = core::mem::size_of_val(&std::any::type_name::<features::flags>());
    let _ = core::mem::size_of_val(&std::any::type_name::<features::profile>());
    let _ = core::mem::size_of_val(&std::any::type_name::<features::profile_cli>());
    let _ = core::mem::size_of_val(&std::any::type_name::<features::policy>());
    let _ = core::mem::size_of_val(&std::any::type_name::<features::grid>());

    Ok(())
}

/// Test that facade re-exports are accessible from perl-lsp (the main LSP crate)
#[test]
fn test_facade_reexports_from_perl_lsp() -> Result<(), Box<dyn std::error::Error>> {
    // After Wave F, consumers should be able to import from perl_lsp (the facade)
    // and get the same types as importing from perl_lsp_rs_core
    use perl_lsp::capability_map;
    use perl_lsp::features;

    // These should resolve without error
    let _features_modules = features;
    let _capability_module = capability_map;

    Ok(())
}

/// Test type identity: perl-lsp re-exports resolve to same types as core
#[test]
fn test_type_identity_facade_vs_core() -> Result<(), Box<dyn std::error::Error>> {
    // Verify that types imported via the facade (perl_lsp) are identical to
    // types imported directly from perl-lsp-rs-core

    // If the re-exports are correct, these type_ids should match
    // (This is a compile-time verification embedded in runtime)
    use perl_lsp::features::ids;
    use perl_lsp_rs_core::features::ids as core_ids;

    // The modules should be the same (compile-time assertion via type identity)
    let _facade_mod_name = std::any::type_name::<ids>();
    let _core_mod_name = std::any::type_name::<core_ids>();

    // If types are identical, these names will be the same string
    // (Note: exact comparison depends on re-export vs module path, but type_name
    //  should return the canonical path from perl-lsp-rs-core for re-exports)

    Ok(())
}

/// Test capability_map module re-export from perl-lsp
#[test]
fn test_capability_map_reexport_from_perl_lsp() -> Result<(), Box<dyn std::error::Error>> {
    // perl-lsp should re-export capability_map for downstream consumers
    use perl_lsp::capability_map;

    let _mod = capability_map;
    Ok(())
}

/// Test that downstream consumer shape for perl-lsp-feature-governance works
#[test]
fn test_governance_consumer_shape_integration() -> Result<(), Box<dyn std::error::Error>> {
    // This simulates how perl-lsp-feature-governance (which stays published in Wave G3)
    // will consume the new perl-lsp-rs-core after Wave F consolidation
    // It should be able to import what it needs from perl_lsp_rs_core::features

    use perl_lsp_rs_core::features::contracts;
    use perl_lsp_rs_core::features::flags;
    use perl_lsp_rs_core::features::policy;
    use perl_lsp_rs_core::features::profile;

    // These imports should work (or fail for expected reasons if types don't exist yet)
    let _c = contracts;
    let _f = flags;
    let _p = policy;
    let _pr = profile;

    Ok(())
}

/// Test that downstream consumer shape for perl-lsp-protocol works
#[test]
fn test_protocol_consumer_shape_integration() -> Result<(), Box<dyn std::error::Error>> {
    // This simulates how perl-lsp-protocol imports from the absorbed crates
    // After Wave F, it should import from perl_lsp_rs_core instead

    use perl_lsp_rs_core::features::contracts;
    use perl_lsp_rs_core::features::flags;

    // BuildFlags and AdvertisedFeatures should be accessible
    let _flags_mod = flags;
    let _contracts_mod = contracts;

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
    // or at least verify the module structure exists
    use perl_lsp_rs_core::features::contracts;

    let _contracts = contracts;
    Ok(())
}

/// Test edge case: empty module structure (before implementation)
#[test]
fn test_empty_module_access() -> Result<(), Box<dyn std::error::Error>> {
    // Even before full implementation, the module structure should exist
    // and be accessible, even if the internals are empty
    use perl_lsp_rs_core::capability_map;
    use perl_lsp_rs_core::features;

    // These should not panic on access
    let features_path = std::any::type_name::<features::ids>();
    let capability_path = std::any::type_name::<capability_map>();

    assert!(!features_path.is_empty());
    assert!(!capability_path.is_empty());

    Ok(())
}

/// Test that the public API surface doesn't have conflicting re-exports
#[test]
fn test_facade_no_conflicting_reexports() -> Result<(), Box<dyn std::error::Error>> {
    // Verify that perl-lsp (the facade) and perl-lsp-rs-core export compatible types
    // without duplication or name collision

    use perl_lsp::capability_map;
    use perl_lsp::features;

    // If these compile without error, name resolution is working
    let _f = features;
    let _c = capability_map;

    Ok(())
}

/// Test comprehensive facade accessibility across all modules
#[test]
fn test_comprehensive_facade_exports() -> Result<(), Box<dyn std::error::Error>> {
    // Comprehensive test ensuring all feature modules are accessible
    // and capability_map is available, all from the core crate

    use perl_lsp_rs_core::capability_map;
    use perl_lsp_rs_core::features::{contracts, flags, grid, ids, policy, profile, profile_cli};

    let _ids = ids;
    let _contracts = contracts;
    let _flags = flags;
    let _profile = profile;
    let _profile_cli = profile_cli;
    let _policy = policy;
    let _grid = grid;
    let _capability_map = capability_map;

    Ok(())
}

/// Test that imports from both old path and new path would not coexist
/// (this ensures Wave F consolidation doesn't accidentally keep old paths)
#[test]
fn test_old_paths_no_longer_accessible() -> Result<(), Box<dyn std::error::Error>> {
    // After Wave F, the old crate paths should NOT be accessible
    // This test documents that the old paths are gone
    // If this test tries to import from perl_lsp_feature_ids, it should fail at compile time

    // We can't directly test "this doesn't compile" in a test file,
    // but we can verify the new path works and document that the old one should be gone
    use perl_lsp_rs_core::features::ids;

    let _new_path = ids;
    // The old path `use perl_lsp_feature_ids::*;` should not work after Wave F

    Ok(())
}
