//! Feature governance façade for Perl LSP.
//!
//! This crate consolidates profile parsing, profile policy, and BDD-grid reporting
//! APIs into a single stability boundary so CLI, runtime startup, and external
//! tooling share one canonical implementation.

use std::fmt;

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
    FeatureProfile, catalog_advertised_feature_ids, feature_ids_from_flags, flags_for_profile,
    flags_for_runtime,
};
pub use perl_lsp_feature_profile::{
    FeatureProfileKind, from_str_name as parse_profile_name, parse_profile_token,
    supported_cli_profiles,
};

/// Parse a `--feature-profile` argument into a runtime policy.
///
/// Returns a structured error when the raw token is unknown. The error includes
/// the current supported token set for CLI/user-facing diagnostics.
pub fn parse_feature_profile_arg(
    raw_profile: &str,
) -> Result<FeatureProfile, UnsupportedFeatureProfileError> {
    match parse_profile_token(raw_profile) {
        Some(kind) => Ok(FeatureProfile::from_kind(kind)),
        None => Err(UnsupportedFeatureProfileError { raw_profile: raw_profile.to_string() }),
    }
}

/// Parse a profile argument and fall back to `FeatureProfile::current()`.
pub fn parse_feature_profile_arg_or_current(raw_profile: &str) -> FeatureProfile {
    parse_feature_profile_arg(raw_profile).unwrap_or_else(|_| FeatureProfile::current())
}

/// Canonical profile label for logs and diagnostics.
pub const fn feature_profile_label(profile: FeatureProfile) -> &'static str {
    profile.as_str()
}

/// Supported CLI tokens accepted by the profile parser.
pub const fn feature_profile_supported_tokens() -> &'static [&'static str] {
    FeatureProfile::supported_cli_profiles()
}

/// Return the canonical profile metadata contract rows used by BDD reporting.
pub const fn feature_profile_metadata() -> &'static [FeatureProfileSpec] {
    feature_profile_specs()
}

/// Structured parse error for invalid profile tokens.
#[derive(Debug)]
pub struct UnsupportedFeatureProfileError {
    /// Raw token that could not be resolved.
    pub raw_profile: String,
}

impl UnsupportedFeatureProfileError {
    /// Human-friendly error message with support list.
    pub fn message(&self) -> String {
        let supported = feature_profile_supported_tokens().join(", ");
        format!("Invalid feature profile: {}. Supported: {}", self.raw_profile, supported)
    }
}

impl fmt::Display for UnsupportedFeatureProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message())
    }
}

impl std::error::Error for UnsupportedFeatureProfileError {}

#[cfg(test)]
mod tests {
    use super::{
        FeatureProfile, feature_profile_supported_tokens, parse_feature_profile_arg,
        parse_feature_profile_arg_or_current,
    };
    use perl_tdd_support::must;

    #[test]
    fn parse_feature_profile_accepts_known_aliases() {
        let profile = must(parse_feature_profile_arg("ga_lock"));
        assert_eq!(profile.as_str(), "ga-lock");

        let profile = must(parse_feature_profile_arg("Prod"));
        assert_eq!(profile.as_str(), "production");

        let profile = must(parse_feature_profile_arg("  ALL  "));
        assert_eq!(profile.as_str(), "all");
    }

    #[test]
    fn parse_unknown_profile_falls_back_to_current() {
        let profile = parse_feature_profile_arg_or_current("unknown-profile");
        assert_eq!(profile, FeatureProfile::current());
    }

    #[test]
    fn supported_tokens_contain_expected() {
        let supported = feature_profile_supported_tokens();
        assert!(supported.contains(&"auto"));
        assert!(supported.contains(&"ga"));
        assert!(supported.contains(&"prod"));
        assert!(supported.contains(&"all"));
    }
}
