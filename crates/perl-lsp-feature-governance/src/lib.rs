//! Feature governance façade for Perl LSP.
//!
//! This crate consolidates profile parsing, profile policy, and BDD-grid reporting
//! APIs into a single stability boundary so CLI, runtime startup, and external
//! tooling share one canonical implementation.

pub use perl_lsp_feature_cli::{
    FeatureProfile, UnsupportedFeatureProfileError, feature_profile_label,
    feature_profile_supported_tokens, parse_feature_profile_arg,
    parse_feature_profile_arg_or_current,
};
pub use perl_lsp_feature_contracts::{
    BddFeatureRow, Feature, FeatureProfileSpec, LSP_VERSION, VERSION, advertised_features,
    advertised_trackable_feature_count_for_grid, all_features, bdd_feature_rows,
    caps_from_feature_ids, catalog, compliance_percent, compliance_percent_for_grid,
    feature_ids_from_caps, feature_profile_specs, has_feature, trackable_feature_count_for_grid,
};
pub use perl_lsp_feature_grid::{
    FEATURE_GRID_COLUMNS, compliance_percent_for_profile, feature_profile_contracts, to_json,
    to_json_for_all_profiles, to_json_for_profile, to_json_for_profiles,
};
pub use perl_lsp_feature_policy::{
    catalog_advertised_feature_ids, feature_ids_from_flags, flags_for_profile, flags_for_runtime,
};
pub use perl_lsp_feature_profile::{
    FeatureProfileKind, from_str_name as parse_profile_name, parse_profile_token,
    supported_cli_profiles,
};

/// Return the canonical profile metadata contract rows used by BDD reporting.
pub const fn feature_profile_metadata() -> &'static [FeatureProfileSpec] {
    feature_profile_specs()
}
