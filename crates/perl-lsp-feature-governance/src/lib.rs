#![warn(missing_docs)]
//! Feature governance façade for Perl LSP.
//!
//! This crate consolidates profile policy and BDD-grid reporting APIs into a
//! single stability boundary so runtime startup and external tooling share one
//! canonical implementation.

pub use perl_lsp_rs_core::features::contracts::{
    BddFeatureRow, Feature, FeatureProfileSpec, LSP_VERSION, VERSION, advertised_features,
    advertised_trackable_feature_count_for_grid, all_features, bdd_feature_rows,
    caps_from_feature_ids, catalog, compliance_percent, compliance_percent_for_grid,
    feature_ids_from_caps, feature_profile_specs, has_feature, trackable_feature_count_for_grid,
};
pub use perl_lsp_rs_core::features::grid::{
    FEATURE_GRID_COLUMNS, compliance_percent_for_profile, feature_profile_contracts, to_json,
    to_json_for_all_profiles, to_json_for_profile, to_json_for_profiles,
};
pub use perl_lsp_rs_core::features::policy::{
    FeatureProfile, catalog_advertised_feature_ids, feature_ids_from_flags, flags_for_profile,
    flags_for_runtime,
};
pub use perl_lsp_rs_core::features::profile::{
    FeatureProfileKind, from_str_name as parse_profile_name, parse_profile_token,
    supported_cli_profiles,
};
pub use perl_lsp_rs_core::features::profile_cli::{
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

    // ── Feature flag evaluation ─────────────────────────────────────

    #[test]
    fn facade_feature_flag_evaluation_ga_lock_disables_inline_values() {
        let flags = flags_for_profile(FeatureProfile::GaLock);
        let ids = feature_ids_from_flags(&flags);
        assert!(
            !ids.contains(&"lsp.inline_value"),
            "ga-lock profile should not include inline_value"
        );
    }

    #[test]
    fn facade_feature_flag_evaluation_all_enables_formatting() {
        let flags = flags_for_profile(FeatureProfile::All);
        assert!(flags.formatting, "all profile should enable formatting");
        assert!(flags.range_formatting, "all profile should enable range_formatting");
    }

    #[test]
    fn facade_feature_flag_evaluation_production_enables_formatting() {
        let flags = flags_for_profile(FeatureProfile::Production);
        assert!(flags.formatting, "production profile should enable formatting");
        assert!(flags.range_formatting, "production profile should enable range_formatting");
    }

    #[test]
    fn facade_feature_flags_all_superset_of_production() {
        let all_ids = feature_ids_from_flags(&flags_for_profile(FeatureProfile::All));
        let prod_ids = feature_ids_from_flags(&flags_for_profile(FeatureProfile::Production));
        for id in &prod_ids {
            assert!(all_ids.contains(id), "'all' profile should contain production feature '{id}'");
        }
        assert!(
            all_ids.len() >= prod_ids.len(),
            "'all' should have at least as many features as production"
        );
    }

    // ── Build profile feature gating ────────────────────────────────

    #[test]
    fn facade_build_profile_ga_lock_enables_core_features() {
        let flags = flags_for_profile(FeatureProfile::GaLock);
        assert!(flags.completion, "ga-lock must enable completion");
        assert!(flags.hover, "ga-lock must enable hover");
        assert!(flags.definition, "ga-lock must enable definition");
        assert!(flags.references, "ga-lock must enable references");
        assert!(flags.document_symbol, "ga-lock must enable document_symbol");
        assert!(flags.semantic_tokens, "ga-lock must enable semantic_tokens");
    }

    #[test]
    fn facade_build_profile_production_enables_advanced_features() {
        let flags = flags_for_profile(FeatureProfile::Production);
        assert!(flags.inline_values, "production must enable inline_values");
        assert!(flags.call_hierarchy, "production must enable call_hierarchy");
        assert!(flags.type_hierarchy, "production must enable type_hierarchy");
        assert!(flags.code_lens, "production must enable code_lens");
        assert!(flags.rename, "production must enable rename");
        assert!(flags.inlay_hints, "production must enable inlay_hints");
    }

    #[test]
    fn facade_build_profile_gating_runtime_with_perltidy_enables_formatting() {
        let flags = flags_for_runtime(FeatureProfile::GaLock, true);
        assert!(flags.formatting, "runtime with perltidy should enable formatting");
        assert!(flags.range_formatting, "runtime with perltidy should enable range_formatting");
    }

    #[test]
    fn facade_build_profile_gating_runtime_without_perltidy_disables_formatting() {
        let runtime = flags_for_runtime(FeatureProfile::Production, false);
        assert!(!runtime.formatting, "runtime without perltidy should disable formatting");
        assert!(
            !runtime.range_formatting,
            "runtime without perltidy should disable range_formatting"
        );
        // Non-formatting flags should still match production
        let base = flags_for_profile(FeatureProfile::Production);
        assert_eq!(base.completion, runtime.completion);
        assert_eq!(base.hover, runtime.hover);
    }

    // ── Feature ID lookup and validation ────────────────────────────

    #[test]
    fn facade_has_feature_validates_known_feature_ids() {
        let known_ids = ["lsp.completion", "lsp.hover", "lsp.definition", "lsp.references"];
        for id in &known_ids {
            assert!(has_feature(id), "has_feature should return true for '{id}'");
        }
    }

    #[test]
    fn facade_has_feature_rejects_invalid_ids() {
        let invalid_ids = ["", "nonexistent", "lsp.", "completion", "lsp.foobar"];
        for id in &invalid_ids {
            assert!(!has_feature(id), "has_feature should return false for '{id}'");
        }
    }

    #[test]
    fn facade_feature_ids_from_flags_are_sorted() {
        for profile in FeatureProfile::all() {
            let flags = flags_for_profile(*profile);
            let ids = feature_ids_from_flags(&flags);
            let mut sorted = ids.clone();
            sorted.sort_unstable();
            assert_eq!(
                ids,
                sorted,
                "feature_ids_from_flags for {} should be sorted",
                profile.as_str()
            );
        }
    }

    #[test]
    fn facade_feature_ids_from_flags_are_unique() {
        for profile in FeatureProfile::all() {
            let flags = flags_for_profile(*profile);
            let ids = feature_ids_from_flags(&flags);
            let mut deduped = ids.clone();
            deduped.sort_unstable();
            deduped.dedup();
            assert_eq!(
                ids.len(),
                deduped.len(),
                "feature_ids_from_flags for {} should contain no duplicates",
                profile.as_str()
            );
        }
    }

    #[test]
    fn facade_catalog_ids_are_subset_of_all_features() {
        let all_feature_ids: Vec<&str> = all_features().iter().map(|f| f.id).collect();
        for profile in FeatureProfile::all() {
            let ids = catalog_advertised_feature_ids(*profile);
            for id in &ids {
                assert!(
                    all_feature_ids.contains(id),
                    "catalog ID '{id}' for profile {} not found in all_features",
                    profile.as_str()
                );
            }
        }
    }

    // ── Feature enablement/disablement ──────────────────────────────

    #[test]
    fn facade_ga_lock_vs_production_differ_on_inline_values() {
        let ga_flags = flags_for_profile(FeatureProfile::GaLock);
        let prod_flags = flags_for_profile(FeatureProfile::Production);
        assert!(!ga_flags.inline_values, "ga-lock should disable inline_values");
        assert!(prod_flags.inline_values, "production should enable inline_values");
    }

    #[test]
    fn facade_production_and_all_match_on_formatting() {
        let prod_flags = flags_for_profile(FeatureProfile::Production);
        let all_flags = flags_for_profile(FeatureProfile::All);
        assert!(prod_flags.formatting, "production should enable formatting");
        assert!(all_flags.formatting, "all should enable formatting");
    }

    #[test]
    fn facade_runtime_perltidy_only_affects_formatting_flags() {
        let with_perltidy = flags_for_runtime(FeatureProfile::Production, true);
        let without_perltidy = flags_for_runtime(FeatureProfile::Production, false);
        // Formatting flags should differ based on perltidy availability
        assert!(with_perltidy.formatting);
        assert!(!without_perltidy.formatting);
        assert!(with_perltidy.range_formatting);
        assert!(!without_perltidy.range_formatting);
        // Non-formatting flags should be identical
        assert_eq!(with_perltidy.completion, without_perltidy.completion);
        assert_eq!(with_perltidy.hover, without_perltidy.hover);
        assert_eq!(with_perltidy.definition, without_perltidy.definition);
        assert_eq!(with_perltidy.references, without_perltidy.references);
        assert_eq!(with_perltidy.rename, without_perltidy.rename);
        assert_eq!(with_perltidy.code_actions, without_perltidy.code_actions);
        assert_eq!(with_perltidy.semantic_tokens, without_perltidy.semantic_tokens);
    }

    #[test]
    fn facade_advertised_features_reflect_profile_enablement() {
        let ga_adv = FeatureProfile::GaLock.advertised_features();
        let prod_adv = FeatureProfile::Production.advertised_features();
        let all_adv = FeatureProfile::All.advertised_features();
        // All profiles should advertise completion
        assert!(ga_adv.completion);
        assert!(prod_adv.completion);
        assert!(all_adv.completion);
        // Production and all both advertise formatting
        assert!(prod_adv.formatting);
        assert!(all_adv.formatting);
    }

    // ── Default feature profile ─────────────────────────────────────

    #[test]
    fn facade_default_profile_is_production_or_ga_lock() {
        let current = FeatureProfile::current();
        let is_known = current == FeatureProfile::Production || current == FeatureProfile::GaLock;
        assert!(is_known, "current() should be either Production or GaLock");
    }

    #[test]
    fn facade_default_profile_enables_core_capabilities() {
        let flags = flags_for_profile(FeatureProfile::current());
        assert!(flags.completion, "default profile must enable completion");
        assert!(flags.hover, "default profile must enable hover");
        assert!(flags.definition, "default profile must enable definition");
        assert!(flags.references, "default profile must enable references");
    }

    #[test]
    fn facade_from_str_name_resolves_all_canonical_names() {
        assert!(parse_profile_name("all").is_some());
        assert!(parse_profile_name("production").is_some());
        assert!(parse_profile_name("ga-lock").is_some());
    }

    #[test]
    fn facade_from_str_name_rejects_unknown_names() {
        assert!(parse_profile_name("debug").is_none());
        assert!(parse_profile_name("minimal").is_none());
        assert!(parse_profile_name("").is_none());
    }

    // ── Compliance monotonicity ─────────────────────────────────────

    #[test]
    fn facade_compliance_all_gte_ga_lock() {
        let all_pct = compliance_percent_for_profile(FeatureProfile::All);
        let ga_pct = compliance_percent_for_profile(FeatureProfile::GaLock);
        assert!(all_pct >= ga_pct, "all compliance ({all_pct}) should be >= ga-lock ({ga_pct})");
    }

    #[test]
    fn facade_compliance_all_gte_production() {
        let all_pct = compliance_percent_for_profile(FeatureProfile::All);
        let prod_pct = compliance_percent_for_profile(FeatureProfile::Production);
        assert!(
            all_pct >= prod_pct,
            "all compliance ({all_pct}) should be >= production ({prod_pct})"
        );
    }

    // ── JSON round-trips and multi-profile ──────────────────────────

    #[test]
    fn facade_to_json_for_profiles_with_two_profiles() -> Result<(), serde_json::Error> {
        let json_str = to_json_for_profiles(&[FeatureProfile::GaLock, FeatureProfile::Production]);
        let value: serde_json::Value = serde_json::from_str(&json_str)?;
        let profiles = value["profiles"].as_array();
        assert!(profiles.is_some());
        let profiles = profiles.map(|p| p.len()).unwrap_or(0);
        assert_eq!(profiles, 2, "should include exactly 2 profiles");
        Ok(())
    }

    #[test]
    fn facade_to_json_feature_count_matches_all_features() -> Result<(), serde_json::Error> {
        let json_str = to_json();
        let value: serde_json::Value = serde_json::from_str(&json_str)?;
        let json_count = value["feature_count"].as_u64().unwrap_or(0) as usize;
        assert_eq!(
            json_count,
            all_features().len(),
            "JSON feature_count should match all_features().len()"
        );
        Ok(())
    }

    // ── Caps round-trip fidelity ────────────────────────────────────

    #[test]
    fn facade_caps_round_trip_preserves_feature_ids() {
        let original_ids = vec!["lsp.completion", "lsp.hover", "lsp.definition"];
        let caps = caps_from_feature_ids(&original_ids);
        let recovered_ids = feature_ids_from_caps(&caps);
        for id in &original_ids {
            assert!(recovered_ids.contains(id), "round-trip should preserve '{id}'");
        }
    }

    #[test]
    fn facade_caps_from_empty_ids_produces_empty_caps() {
        let caps = caps_from_feature_ids(&[]);
        let ids = feature_ids_from_caps(&caps);
        assert!(ids.is_empty(), "empty input should produce empty output");
    }
}
