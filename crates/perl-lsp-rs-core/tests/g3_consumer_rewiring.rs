//! Regression test: Verify consumer import rewiring after absorption.
//!
//! Wave G3 consumers must be able to access absorbed module content via new
//! perl_lsp_rs_core::* paths. This test verifies that modules are properly
//! re-exported from lib.rs and that key functions are accessible.

#[test]
fn g3_governance_exports_feature_functions() {
    // Verify that governance functions (from absorbed feature-governance crate) are accessible
    use perl_lsp_rs_core::governance;

    // Smoke test: these functions should be reachable
    let _advertised = governance::advertised_features();
    let _all = governance::all_features();
    let _profiles = governance::supported_cli_profiles();

    // Type should be accessible
    let _: Option<governance::FeatureProfileKind> = None;
}
