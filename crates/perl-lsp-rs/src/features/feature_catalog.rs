//! LSP feature catalog and compliance tracking compatibility shim.
//!
//! This module now delegates to `perl-lsp-rs-core::governance`, which owns the
//! canonical BDD-grid and profile governance API while keeping the existing
//! `perl_lsp::features` compatibility surface stable.
//!
//! Wave G3 (#4535): `perl-lsp-feature-governance` absorbed into `perl-lsp-rs-core::governance`.

pub use perl_lsp_rs_core::governance::{
    BddFeatureRow, Feature, FeatureProfile, LSP_VERSION, VERSION, advertised_features,
    advertised_trackable_feature_count_for_grid, all_features, bdd_feature_rows, catalog,
    compliance_percent, compliance_percent_for_grid, compliance_percent_for_profile,
    feature_profile_contracts, has_feature, to_json, to_json_for_all_profiles, to_json_for_profile,
    trackable_feature_count_for_grid,
};
