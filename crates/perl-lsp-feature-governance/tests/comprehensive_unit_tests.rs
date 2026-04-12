//! Comprehensive integration tests for `perl-lsp-feature-governance`.
//!
//! Covers the façade's own functions, re-exported APIs from the four sub-crates,
//! and cross-layer consistency invariants.

use perl_lsp_feature_governance::{
    // Re-exports: contracts
    FEATURE_GRID_COLUMNS,
    // Re-exports: policy
    FeatureProfile,
    // Re-exports: profile
    FeatureProfileKind,
    LSP_VERSION,
    // Façade-local functions
    UnsupportedFeatureProfileError,
    VERSION,
    advertised_features,
    advertised_trackable_feature_count_for_grid,
    all_features,
    bdd_feature_rows,
    catalog_advertised_feature_ids,
    compliance_percent,
    compliance_percent_for_grid,
    compliance_percent_for_profile,
    feature_ids_from_flags,
    feature_profile_contracts,
    feature_profile_label,
    feature_profile_metadata,
    feature_profile_specs,
    feature_profile_supported_tokens,
    flags_for_profile,
    flags_for_runtime,
    has_feature,
    parse_feature_profile_arg,
    parse_feature_profile_arg_or_current,
    parse_profile_name,
    parse_profile_token,
    supported_cli_profiles,
    to_json,
    to_json_for_all_profiles,
    to_json_for_profile,
    to_json_for_profiles,
    trackable_feature_count_for_grid,
};

// ---------------------------------------------------------------------------
// Façade-local: parse_feature_profile_arg
// ---------------------------------------------------------------------------

#[test]
fn parse_arg_accepts_ga_lock_alias() -> Result<(), Box<dyn std::error::Error>> {
    let profile = parse_feature_profile_arg("ga_lock")?;
    assert_eq!(profile.as_str(), "ga-lock");
    Ok(())
}

#[test]
fn parse_arg_accepts_prod_alias() -> Result<(), Box<dyn std::error::Error>> {
    let profile = parse_feature_profile_arg("Prod")?;
    assert_eq!(profile.as_str(), "production");
    Ok(())
}

#[test]
fn parse_arg_accepts_all_with_whitespace() -> Result<(), Box<dyn std::error::Error>> {
    let profile = parse_feature_profile_arg("  ALL  ")?;
    assert_eq!(profile.as_str(), "all");
    Ok(())
}

#[test]
fn parse_arg_accepts_ga_hyphenated() -> Result<(), Box<dyn std::error::Error>> {
    let profile = parse_feature_profile_arg("ga-lock")?;
    assert_eq!(profile.as_str(), "ga-lock");
    Ok(())
}

#[test]
fn parse_arg_accepts_auto() -> Result<(), Box<dyn std::error::Error>> {
    let profile = parse_feature_profile_arg("auto")?;
    // "auto" resolves to whatever current() is
    assert_eq!(profile, FeatureProfile::current());
    Ok(())
}

#[test]
fn parse_arg_accepts_production_full() -> Result<(), Box<dyn std::error::Error>> {
    let profile = parse_feature_profile_arg("production")?;
    assert_eq!(profile.as_str(), "production");
    Ok(())
}

#[test]
fn parse_arg_rejects_empty_string() {
    let result = parse_feature_profile_arg("");
    assert!(result.is_err());
}

#[test]
fn parse_arg_rejects_unknown_token() {
    let result = parse_feature_profile_arg("nope");
    assert!(result.is_err());
}

#[test]
fn parse_arg_rejects_partial_match() {
    let result = parse_feature_profile_arg("ga-loc");
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Façade-local: parse_feature_profile_arg_or_current
// ---------------------------------------------------------------------------

#[test]
fn parse_arg_or_current_returns_profile_for_valid_input() {
    let profile = parse_feature_profile_arg_or_current("all");
    assert_eq!(profile.as_str(), "all");
}

#[test]
fn parse_arg_or_current_falls_back_for_invalid_input() {
    let profile = parse_feature_profile_arg_or_current("bogus");
    assert_eq!(profile, FeatureProfile::current());
}

#[test]
fn parse_arg_or_current_falls_back_for_empty_string() {
    let profile = parse_feature_profile_arg_or_current("");
    assert_eq!(profile, FeatureProfile::current());
}

// ---------------------------------------------------------------------------
// Façade-local: feature_profile_label
// ---------------------------------------------------------------------------

#[test]
fn label_ga_lock() {
    assert_eq!(feature_profile_label(FeatureProfile::GaLock), "ga-lock");
}

#[test]
fn label_production() {
    assert_eq!(feature_profile_label(FeatureProfile::Production), "production");
}

#[test]
fn label_all() {
    assert_eq!(feature_profile_label(FeatureProfile::All), "all");
}

// ---------------------------------------------------------------------------
// Façade-local: feature_profile_supported_tokens
// ---------------------------------------------------------------------------

#[test]
fn supported_tokens_is_non_empty() {
    let tokens = feature_profile_supported_tokens();
    assert!(!tokens.is_empty());
}

#[test]
fn supported_tokens_contain_canonical_set() {
    let tokens = feature_profile_supported_tokens();
    for expected in &["auto", "ga-lock", "ga", "ga_lock", "prod", "production", "all"] {
        assert!(tokens.contains(expected), "expected supported tokens to contain {expected:?}",);
    }
}

// ---------------------------------------------------------------------------
// Façade-local: feature_profile_metadata
// ---------------------------------------------------------------------------

#[test]
fn metadata_has_three_profiles() {
    let specs = feature_profile_metadata();
    assert_eq!(specs.len(), 3);
}

#[test]
fn metadata_canonical_labels_are_known() {
    let canonicals: Vec<&str> = feature_profile_metadata().iter().map(|s| s.canonical).collect();
    assert!(canonicals.contains(&"ga-lock"));
    assert!(canonicals.contains(&"production"));
    assert!(canonicals.contains(&"all"));
}

#[test]
fn metadata_specs_have_non_empty_descriptions() {
    for spec in feature_profile_metadata() {
        assert!(
            !spec.description.is_empty(),
            "profile spec {canonical} has empty description",
            canonical = spec.canonical,
        );
    }
}

#[test]
fn metadata_specs_have_non_empty_aliases() {
    for spec in feature_profile_metadata() {
        assert!(
            !spec.aliases.is_empty(),
            "profile spec {canonical} has no aliases",
            canonical = spec.canonical,
        );
    }
}

// ---------------------------------------------------------------------------
// UnsupportedFeatureProfileError
// ---------------------------------------------------------------------------

#[test]
fn error_message_includes_raw_profile() {
    let err = UnsupportedFeatureProfileError { raw_profile: "bad-token".to_string() };
    let msg = err.message();
    assert!(msg.contains("bad-token"), "message should contain the raw token");
}

#[test]
fn error_message_lists_supported_tokens() {
    let err = UnsupportedFeatureProfileError { raw_profile: "x".to_string() };
    let msg = err.message();
    assert!(msg.contains("auto"), "message should list supported tokens");
}

#[test]
fn error_display_matches_message() {
    let err = UnsupportedFeatureProfileError { raw_profile: "xyz".to_string() };
    let display = format!("{err}");
    assert_eq!(display, err.message());
}

#[test]
fn error_debug_includes_raw_profile() {
    let err = UnsupportedFeatureProfileError { raw_profile: "test".to_string() };
    let debug = format!("{err:?}");
    assert!(debug.contains("test"));
}

#[test]
fn error_implements_std_error() {
    let err = UnsupportedFeatureProfileError { raw_profile: "t".to_string() };
    let _: &dyn std::error::Error = &err;
}

// ---------------------------------------------------------------------------
// Re-exports: FeatureProfile (policy)
// ---------------------------------------------------------------------------

#[test]
fn feature_profile_all_contains_three_variants() {
    assert_eq!(FeatureProfile::all().len(), 3);
}

#[test]
fn feature_profile_round_trips_through_kind() {
    for &kind in FeatureProfileKind::all() {
        let profile = FeatureProfile::from_kind(kind);
        assert_eq!(profile.as_str(), kind.as_str());
    }
}

#[test]
fn feature_profile_from_ga_lock_enabled() {
    let locked = FeatureProfile::from_ga_lock_enabled(true);
    assert_eq!(locked, FeatureProfile::GaLock);
    let unlocked = FeatureProfile::from_ga_lock_enabled(false);
    assert_eq!(unlocked, FeatureProfile::Production);
}

#[test]
fn feature_profile_from_cli_argument_valid() {
    let p = FeatureProfile::from_cli_argument("all");
    assert_eq!(p, FeatureProfile::All);
}

#[test]
fn feature_profile_from_cli_argument_invalid_falls_back() {
    let p = FeatureProfile::from_cli_argument("nope");
    assert_eq!(p, FeatureProfile::current());
}

#[test]
fn feature_profile_parse_profile_known() {
    assert_eq!(FeatureProfile::parse_profile("ga"), Some(FeatureProfile::GaLock));
}

#[test]
fn feature_profile_parse_profile_unknown_is_none() {
    assert_eq!(FeatureProfile::parse_profile("???"), None);
}

// ---------------------------------------------------------------------------
// Re-exports: FeatureProfileKind (profile)
// ---------------------------------------------------------------------------

#[test]
fn profile_kind_all_contains_three_variants() {
    assert_eq!(FeatureProfileKind::all().len(), 3);
}

#[test]
fn profile_kind_as_str_is_stable() {
    assert_eq!(FeatureProfileKind::GaLock.as_str(), "ga-lock");
    assert_eq!(FeatureProfileKind::Production.as_str(), "production");
    assert_eq!(FeatureProfileKind::All.as_str(), "all");
}

#[test]
fn profile_kind_aliases_are_non_empty() {
    for &kind in FeatureProfileKind::all() {
        assert!(!kind.aliases().is_empty(), "expected aliases for {kind:?}");
    }
}

#[test]
fn profile_kind_from_ga_lock_enabled() {
    assert_eq!(FeatureProfileKind::from_ga_lock_enabled(true), FeatureProfileKind::GaLock);
    assert_eq!(FeatureProfileKind::from_ga_lock_enabled(false), FeatureProfileKind::Production);
}

// ---------------------------------------------------------------------------
// Re-exports: parse_profile_name / parse_profile_token (profile)
// ---------------------------------------------------------------------------

#[test]
fn parse_profile_name_known_values() {
    assert_eq!(parse_profile_name("ga-lock"), Some(FeatureProfileKind::GaLock));
    assert_eq!(parse_profile_name("prod"), Some(FeatureProfileKind::Production));
    assert_eq!(parse_profile_name("all"), Some(FeatureProfileKind::All));
}

#[test]
fn parse_profile_name_unknown_returns_none() {
    assert_eq!(parse_profile_name("invalid"), None);
}

#[test]
fn parse_profile_token_normalizes_case() {
    assert_eq!(parse_profile_token("GA-LOCK"), Some(FeatureProfileKind::GaLock));
    assert_eq!(parse_profile_token("ALL"), Some(FeatureProfileKind::All));
    assert_eq!(parse_profile_token("Prod"), Some(FeatureProfileKind::Production));
}

#[test]
fn parse_profile_token_trims_whitespace() {
    assert_eq!(parse_profile_token("  all  "), Some(FeatureProfileKind::All));
}

#[test]
fn parse_profile_token_converts_underscores() {
    assert_eq!(parse_profile_token("ga_lock"), Some(FeatureProfileKind::GaLock));
}

#[test]
fn parse_profile_token_combined_normalization() {
    assert_eq!(parse_profile_token("  GA_LOCK  "), Some(FeatureProfileKind::GaLock));
}

#[test]
fn parse_profile_token_returns_none_for_garbage() {
    assert_eq!(parse_profile_token("not-a-real-profile"), None);
}

#[test]
fn supported_cli_profiles_equals_facade_tokens() {
    let facade_tokens = feature_profile_supported_tokens();
    let profile_tokens = supported_cli_profiles();
    let policy_tokens = FeatureProfile::supported_cli_profiles();

    assert_eq!(facade_tokens, profile_tokens);
    assert_eq!(facade_tokens, policy_tokens);
}

// ---------------------------------------------------------------------------
// Re-exports: flags_for_profile / flags_for_runtime (policy)
// ---------------------------------------------------------------------------

#[test]
fn flags_for_all_profiles_are_non_default() {
    for &profile in FeatureProfile::all() {
        let flags = flags_for_profile(profile);
        assert!(flags.completion, "profile {profile:?} should enable completion");
    }
}

#[test]
fn flags_for_runtime_enables_formatting_with_perltidy() {
    let flags = flags_for_runtime(FeatureProfile::Production, true);
    assert!(flags.formatting, "formatting should be on when perltidy is available");
    assert!(flags.range_formatting, "range_formatting should be on when perltidy is available");
}

#[test]
fn flags_for_runtime_without_perltidy_disables_formatting() {
    let runtime = flags_for_runtime(FeatureProfile::Production, false);
    assert!(!runtime.formatting, "without perltidy, runtime should disable formatting");
    assert!(!runtime.range_formatting, "without perltidy, runtime should disable range_formatting");
    // Other flags should still match production
    let base = flags_for_profile(FeatureProfile::Production);
    assert_eq!(base.completion, runtime.completion);
    assert_eq!(base.hover, runtime.hover);
}

#[test]
fn feature_ids_from_flags_returns_sorted_list() {
    let flags = flags_for_profile(FeatureProfile::All);
    let ids = feature_ids_from_flags(&flags);
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    assert_eq!(ids, sorted, "feature IDs should be sorted");
}

#[test]
fn feature_ids_from_flags_has_no_duplicates() {
    let flags = flags_for_profile(FeatureProfile::All);
    let ids = feature_ids_from_flags(&flags);
    let mut deduped = ids.clone();
    deduped.dedup();
    assert_eq!(ids, deduped, "feature IDs should be unique");
}

#[test]
fn catalog_advertised_ids_subset_of_all_flags() {
    let all_ids = feature_ids_from_flags(&flags_for_profile(FeatureProfile::All));
    for &profile in FeatureProfile::all() {
        let catalog_ids = catalog_advertised_feature_ids(profile);
        for id in &catalog_ids {
            assert!(
                all_ids.contains(id),
                "catalog ID {id:?} for {profile:?} not in All profile flags",
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Re-exports: contracts (catalog, features, BDD rows)
// ---------------------------------------------------------------------------

#[test]
fn version_constants_are_non_empty() {
    assert!(!VERSION.is_empty(), "VERSION should be non-empty");
    assert!(!LSP_VERSION.is_empty(), "LSP_VERSION should be non-empty");
}

#[test]
fn all_features_is_non_empty() {
    assert!(!all_features().is_empty());
}

#[test]
fn all_features_have_non_empty_ids() {
    for feature in all_features() {
        assert!(!feature.id.is_empty(), "feature should have non-empty id");
    }
}

#[test]
fn advertised_features_is_non_empty() {
    assert!(!advertised_features().is_empty());
}

#[test]
fn advertised_features_are_subset_of_all() {
    let all_ids: Vec<&str> = all_features().iter().map(|f| f.id).collect();
    for &id in advertised_features() {
        assert!(all_ids.contains(&id), "advertised feature {id:?} not in all_features()");
    }
}

#[test]
fn has_feature_returns_true_for_advertised_ids() {
    for &id in advertised_features() {
        assert!(has_feature(id), "has_feature({id}) should be true for advertised feature");
    }
}

#[test]
fn has_feature_returns_false_for_unknown_id() {
    assert!(!has_feature("this-does-not-exist-99"));
}

#[test]
fn bdd_feature_rows_matches_all_features_count() {
    let rows = bdd_feature_rows();
    let features = all_features();
    assert_eq!(rows.len(), features.len());
}

#[test]
fn bdd_feature_rows_are_sorted_by_area_then_id() {
    let rows = bdd_feature_rows();
    for window in rows.windows(2) {
        let ordering = window[0].area.cmp(window[1].area).then(window[0].id.cmp(window[1].id));
        assert!(
            ordering.is_le(),
            "BDD rows not sorted: ({area_a}, {id_a}) > ({area_b}, {id_b})",
            area_a = window[0].area,
            id_a = window[0].id,
            area_b = window[1].area,
            id_b = window[1].id,
        );
    }
}

#[test]
fn bdd_rows_have_non_empty_fields() {
    for row in bdd_feature_rows() {
        assert!(!row.id.is_empty());
        assert!(!row.area.is_empty());
        assert!(!row.maturity.is_empty());
        assert!(!row.description.is_empty());
    }
}

#[test]
fn trackable_count_lte_total_features() {
    assert!(trackable_feature_count_for_grid() <= all_features().len());
}

#[test]
fn advertised_trackable_count_lte_trackable_count() {
    assert!(advertised_trackable_feature_count_for_grid() <= trackable_feature_count_for_grid());
}

#[test]
fn compliance_percent_in_range() {
    let pct = compliance_percent();
    assert!((0.0..=100.0).contains(&pct), "compliance_percent {pct} out of range");
}

#[test]
fn compliance_percent_for_grid_in_range() {
    let pct = compliance_percent_for_grid();
    assert!((0.0..=100.0).contains(&pct), "grid compliance {pct} out of range");
}

#[test]
fn feature_profile_specs_equals_contracts() {
    let specs = feature_profile_specs();
    let contracts = feature_profile_contracts();
    assert_eq!(specs.len(), contracts.len());
    for (s, c) in specs.iter().zip(contracts.iter()) {
        assert_eq!(s.canonical, c.canonical);
    }
}

// ---------------------------------------------------------------------------
// Re-exports: grid (JSON payloads)
// ---------------------------------------------------------------------------

#[test]
fn feature_grid_columns_are_non_empty() {
    assert!(!FEATURE_GRID_COLUMNS.is_empty());
}

#[test]
fn feature_grid_columns_contain_expected_keys() {
    for key in &["area", "id", "spec", "maturity", "advertised", "description"] {
        assert!(FEATURE_GRID_COLUMNS.contains(key), "FEATURE_GRID_COLUMNS missing {key:?}");
    }
}

#[test]
fn to_json_produces_valid_json() -> Result<(), Box<dyn std::error::Error>> {
    let raw = to_json();
    let _: serde_json::Value = serde_json::from_str(&raw)?;
    Ok(())
}

#[test]
fn to_json_contains_version_and_grid() -> Result<(), Box<dyn std::error::Error>> {
    let val: serde_json::Value = serde_json::from_str(&to_json())?;
    assert!(val.get("version").is_some(), "missing version");
    assert!(val.get("lsp_version").is_some(), "missing lsp_version");
    assert!(val.get("feature_grid").is_some(), "missing feature_grid");
    assert!(val.get("profiles").is_some(), "missing profiles");
    assert!(val.get("compliance_percent").is_some(), "missing compliance_percent");
    Ok(())
}

#[test]
fn to_json_for_profile_scopes_to_single_profile() -> Result<(), Box<dyn std::error::Error>> {
    for &profile in FeatureProfile::all() {
        let val: serde_json::Value = serde_json::from_str(&to_json_for_profile(profile))?;
        let label = val.get("profile").and_then(|v| v.as_str()).ok_or("missing profile key")?;
        assert_eq!(label, profile.as_str());
    }
    Ok(())
}

#[test]
fn to_json_for_all_profiles_includes_all_three() -> Result<(), Box<dyn std::error::Error>> {
    let val: serde_json::Value = serde_json::from_str(&to_json_for_all_profiles())?;
    let profiles =
        val.get("profiles").and_then(|v| v.as_array()).ok_or("missing profiles array")?;
    let labels: Vec<&str> =
        profiles.iter().filter_map(|p| p.get("profile").and_then(|v| v.as_str())).collect();
    assert!(labels.contains(&"ga-lock"));
    assert!(labels.contains(&"production"));
    assert!(labels.contains(&"all"));
    Ok(())
}

#[test]
fn to_json_for_profiles_with_subset() -> Result<(), Box<dyn std::error::Error>> {
    let val: serde_json::Value = serde_json::from_str(&to_json_for_profiles(&[
        FeatureProfile::GaLock,
        FeatureProfile::All,
    ]))?;
    let profiles =
        val.get("profiles").and_then(|v| v.as_array()).ok_or("missing profiles array")?;
    assert_eq!(profiles.len(), 2);
    Ok(())
}

#[test]
fn compliance_percent_for_profile_in_range() {
    for &profile in FeatureProfile::all() {
        let pct = compliance_percent_for_profile(profile);
        assert!((0.0..=100.0).contains(&pct), "compliance for {profile:?} out of range: {pct}",);
    }
}

#[test]
fn compliance_all_gte_ga_lock() {
    let all_pct = compliance_percent_for_profile(FeatureProfile::All);
    let ga_pct = compliance_percent_for_profile(FeatureProfile::GaLock);
    assert!(all_pct >= ga_pct, "All ({all_pct}) should be >= GaLock ({ga_pct})");
}

// ---------------------------------------------------------------------------
// Cross-layer consistency
// ---------------------------------------------------------------------------

#[test]
fn every_profile_label_round_trips_through_parse() -> Result<(), Box<dyn std::error::Error>> {
    for &profile in FeatureProfile::all() {
        let label = profile.as_str();
        let parsed = parse_feature_profile_arg(label)?;
        assert_eq!(parsed, profile, "round-trip failed for {label:?}");
    }
    Ok(())
}

#[test]
fn metadata_canonical_round_trips_through_parse() -> Result<(), Box<dyn std::error::Error>> {
    for spec in feature_profile_metadata() {
        let parsed = parse_feature_profile_arg(spec.canonical)?;
        assert_eq!(parsed.as_str(), spec.canonical);
    }
    Ok(())
}

#[test]
fn metadata_aliases_round_trip_through_parse() -> Result<(), Box<dyn std::error::Error>> {
    for spec in feature_profile_metadata() {
        for alias in spec.aliases {
            let parsed = parse_feature_profile_arg(alias)?;
            assert_eq!(
                parsed.as_str(),
                spec.canonical,
                "alias {alias:?} should resolve to {canonical}",
                canonical = spec.canonical,
            );
        }
    }
    Ok(())
}

#[test]
fn json_compliance_matches_computed_compliance() -> Result<(), Box<dyn std::error::Error>> {
    for &profile in FeatureProfile::all() {
        let computed = compliance_percent_for_profile(profile);
        let val: serde_json::Value = serde_json::from_str(&to_json_for_profile(profile))?;
        let json_pct = val
            .get("compliance_percent")
            .and_then(|v| v.as_f64())
            .ok_or("missing compliance_percent")?;
        assert!(
            (json_pct - computed as f64).abs() < f64::from(f32::EPSILON),
            "compliance mismatch for {profile:?}: JSON={json_pct} computed={computed}",
        );
    }
    Ok(())
}

#[test]
fn grid_rows_json_matches_bdd_rows() -> Result<(), Box<dyn std::error::Error>> {
    let val: serde_json::Value = serde_json::from_str(&to_json())?;
    let rows = val
        .get("feature_grid")
        .and_then(|g| g.get("rows"))
        .and_then(|r| r.as_array())
        .ok_or("missing grid rows")?;
    let bdd = bdd_feature_rows();
    assert_eq!(rows.len(), bdd.len(), "grid row count should match BDD row count");
    Ok(())
}

#[test]
fn profile_build_flags_are_deterministic() {
    for &profile in FeatureProfile::all() {
        let a = flags_for_profile(profile);
        let b = flags_for_profile(profile);
        assert_eq!(a, b, "flags should be deterministic for {profile:?}");
    }
}

#[test]
fn all_profile_is_superset_of_others() {
    let all_ids = feature_ids_from_flags(&flags_for_profile(FeatureProfile::All));
    for &profile in &[FeatureProfile::GaLock, FeatureProfile::Production] {
        let ids = feature_ids_from_flags(&flags_for_profile(profile));
        for id in &ids {
            assert!(all_ids.contains(id), "All profile missing {id:?} from {profile:?}");
        }
    }
}
