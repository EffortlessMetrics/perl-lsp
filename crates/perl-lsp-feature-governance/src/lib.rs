#![warn(missing_docs)]
//! Feature governance façade for Perl LSP.
//!
//! This crate consolidates profile policy and BDD-grid reporting APIs into a
//! single stability boundary so runtime startup and external tooling share one
//! canonical implementation.

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
    flags_for_runtime, flags_for_runtime_with_overrides,
};
pub use perl_lsp_feature_profile::{
    FeatureProfileKind, from_str_name as parse_profile_name, parse_profile_token,
    supported_cli_profiles,
};
pub use perl_lsp_feature_profile_cli::{
    UnsupportedFeatureProfileError, feature_profile_label, feature_profile_supported_tokens,
    parse_feature_profile_arg, parse_feature_profile_arg_or_current,
};
/// Return the canonical profile metadata contract rows used by BDD reporting.
pub const fn feature_profile_metadata() -> &'static [FeatureProfileSpec] {
    feature_profile_specs()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Facade re-exports are functional ────────────────────────────

    #[test]
    fn facade_exposes_all_features() {
        let features = all_features();
        assert!(!features.is_empty(), "all_features() should return non-empty list");
    }

    #[test]
    fn facade_exposes_has_feature() {
        assert!(has_feature("lsp.completion"));
        assert!(!has_feature("lsp.nonexistent"));
    }

    #[test]
    fn facade_exposes_advertised_features() {
        let adv = advertised_features();
        assert!(!adv.is_empty());
    }

    #[test]
    fn facade_exposes_bdd_feature_rows() {
        let rows = bdd_feature_rows();
        assert!(!rows.is_empty());
    }

    #[test]
    fn facade_exposes_compliance_percent() {
        let pct = compliance_percent();
        assert!((0.0..=100.0).contains(&pct));
    }

    #[test]
    fn facade_exposes_compliance_percent_for_grid() {
        let pct = compliance_percent_for_grid();
        assert!((0.0..=100.0).contains(&pct));
    }

    #[test]
    fn facade_exposes_trackable_counts() {
        let trackable = trackable_feature_count_for_grid();
        let advertised = advertised_trackable_feature_count_for_grid();
        assert!(trackable > 0);
        assert!(advertised <= trackable);
    }

    #[test]
    fn facade_exposes_version_constants() {
        assert!(!VERSION.is_empty());
        assert!(!LSP_VERSION.is_empty());
    }

    // ── Grid re-exports ─────────────────────────────────────────────

    #[test]
    fn facade_exposes_feature_grid_columns() {
        assert!(FEATURE_GRID_COLUMNS.contains(&"id"));
        assert!(FEATURE_GRID_COLUMNS.contains(&"area"));
        assert!(FEATURE_GRID_COLUMNS.contains(&"maturity"));
        assert!(FEATURE_GRID_COLUMNS.contains(&"advertised"));
    }

    #[test]
    fn facade_to_json_produces_valid_json() -> Result<(), serde_json::Error> {
        let json_str = to_json();
        let value: serde_json::Value = serde_json::from_str(&json_str)?;
        assert!(value.get("version").is_some());
        Ok(())
    }

    #[test]
    fn facade_to_json_for_profile_includes_profile_key() -> Result<(), serde_json::Error> {
        let json_str = to_json_for_profile(FeatureProfile::All);
        let value: serde_json::Value = serde_json::from_str(&json_str)?;
        assert_eq!(value["profile"].as_str(), Some("all"));
        Ok(())
    }

    #[test]
    fn facade_to_json_for_all_profiles_includes_all_three() -> Result<(), serde_json::Error> {
        let json_str = to_json_for_all_profiles();
        let value: serde_json::Value = serde_json::from_str(&json_str)?;
        let profiles = value["profiles"].as_array();
        assert!(profiles.is_some());
        let profiles = profiles.map(|p| p.len()).unwrap_or(0);
        assert!(profiles >= 3, "should include at least ga-lock, production, all");
        Ok(())
    }

    #[test]
    fn facade_compliance_percent_for_profile_returns_valid_range() {
        for profile in FeatureProfile::all() {
            let pct = compliance_percent_for_profile(*profile);
            assert!(
                (0.0..=100.0).contains(&pct),
                "compliance for {} should be 0-100, got {}",
                profile.as_str(),
                pct
            );
        }
    }

    // ── Policy re-exports ───────────────────────────────────────────

    #[test]
    fn facade_flags_for_profile_returns_expected_types() {
        let flags = flags_for_profile(FeatureProfile::Production);
        assert!(flags.completion);
    }

    #[test]
    fn facade_flags_for_runtime_enables_formatting_with_perltidy() {
        let flags = flags_for_runtime(FeatureProfile::Production, true);
        assert!(flags.formatting);
    }

    #[test]
    fn facade_feature_ids_from_flags_returns_sorted() {
        let flags = flags_for_profile(FeatureProfile::All);
        let ids = feature_ids_from_flags(&flags);
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        assert_eq!(ids, sorted);
    }

    #[test]
    fn facade_catalog_advertised_feature_ids_non_empty() {
        let ids = catalog_advertised_feature_ids(FeatureProfile::Production);
        assert!(!ids.is_empty());
    }

    // ── Profile re-exports ──────────────────────────────────────────

    #[test]
    fn facade_parse_profile_name_resolves_known() {
        assert!(parse_profile_name("all").is_some());
        assert!(parse_profile_name("bogus").is_none());
    }

    #[test]
    fn facade_parse_profile_token_normalizes() {
        assert!(parse_profile_token("  ALL  ").is_some());
    }

    #[test]
    fn facade_supported_cli_profiles_non_empty() {
        assert!(!supported_cli_profiles().is_empty());
    }

    #[test]
    fn facade_feature_profile_kind_variants() {
        assert_eq!(FeatureProfileKind::GaLock.as_str(), "ga-lock");
        assert_eq!(FeatureProfileKind::Production.as_str(), "production");
        assert_eq!(FeatureProfileKind::All.as_str(), "all");
    }

    // ── Profile CLI re-exports ──────────────────────────────────────

    #[test]
    fn facade_parse_feature_profile_arg_works() {
        let result = parse_feature_profile_arg("prod");
        assert!(result.is_ok());
    }

    #[test]
    fn facade_parse_feature_profile_arg_or_current_fallback() {
        let profile = parse_feature_profile_arg_or_current("nonsense");
        assert_eq!(profile, FeatureProfile::current());
    }

    #[test]
    fn facade_feature_profile_label_is_canonical() {
        assert_eq!(feature_profile_label(FeatureProfile::GaLock), "ga-lock");
    }

    #[test]
    fn facade_feature_profile_supported_tokens_non_empty() {
        assert!(!feature_profile_supported_tokens().is_empty());
    }

    // ── feature_profile_metadata ────────────────────────────────────

    #[test]
    fn feature_profile_metadata_matches_specs() {
        let metadata = feature_profile_metadata();
        let specs = feature_profile_specs();
        assert_eq!(metadata.len(), specs.len());
        for (m, s) in metadata.iter().zip(specs.iter()) {
            assert_eq!(m.canonical, s.canonical);
        }
    }

    // ── Capability map re-exports ───────────────────────────────────

    #[test]
    fn facade_caps_from_feature_ids_works() {
        let caps = caps_from_feature_ids(&["lsp.completion", "lsp.hover"]);
        let ids = feature_ids_from_caps(&caps);
        assert!(ids.contains(&"lsp.completion"));
        assert!(ids.contains(&"lsp.hover"));
    }

    // ── Feature struct fields ───────────────────────────────────────

    #[test]
    fn feature_struct_has_expected_fields() {
        let feature = &all_features()[0];
        assert!(!feature.id.is_empty());
        assert!(!feature.spec.is_empty());
        assert!(!feature.area.is_empty());
        assert!(!feature.maturity.is_empty());
        assert!(!feature.description.is_empty());
    }

    #[test]
    fn bdd_feature_row_serialization_has_all_column_fields() {
        let rows = bdd_feature_rows();
        let first = &rows[0];
        // Verify all FEATURE_GRID_COLUMNS fields are present in BddFeatureRow
        assert!(!first.id.is_empty());
        assert!(!first.spec.is_empty());
        assert!(!first.area.is_empty());
        assert!(!first.maturity.is_empty());
        assert!(!first.description.is_empty());
        // advertised and counts_in_coverage are bools, always present
    }
}
