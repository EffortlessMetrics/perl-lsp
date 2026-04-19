//! Comprehensive unit tests for the `perl-lsp-feature-grid` crate.
//!
//! Covers the public API surface: JSON payload generation, profile-aware compliance,
//! BDD grid structure, feature-profile contracts, and re-exported catalog functions.

use perl_lsp_feature_grid::{
    FEATURE_GRID_COLUMNS, FeatureProfile, LSP_VERSION, VERSION, advertised_features,
    advertised_trackable_feature_count_for_grid, all_features, bdd_feature_rows,
    catalog_advertised_feature_ids, compliance_percent, compliance_percent_for_grid,
    compliance_percent_for_profile, feature_profile_contracts, has_feature, to_json,
    to_json_for_all_profiles, to_json_for_profile, to_json_for_profiles,
    trackable_feature_count_for_grid,
};
use perl_tdd_support::must_some;
use serde_json::Value;

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn parse_json(s: &str) -> Result<Value, Box<dyn std::error::Error>> {
    Ok(serde_json::from_str(s)?)
}

// ===================================================================
// 1. Constants and version metadata
// ===================================================================

#[test]
fn version_constant_is_non_empty() {
    assert!(!VERSION.is_empty(), "VERSION must not be empty");
}

#[test]
fn lsp_version_constant_is_non_empty() {
    assert!(!LSP_VERSION.is_empty(), "LSP_VERSION must not be empty");
}

#[test]
fn feature_grid_columns_has_expected_entries() {
    assert!(
        FEATURE_GRID_COLUMNS.len() >= 7,
        "Expected at least 7 grid columns"
    );
    assert!(FEATURE_GRID_COLUMNS.contains(&"id"));
    assert!(FEATURE_GRID_COLUMNS.contains(&"area"));
    assert!(FEATURE_GRID_COLUMNS.contains(&"spec"));
    assert!(FEATURE_GRID_COLUMNS.contains(&"maturity"));
    assert!(FEATURE_GRID_COLUMNS.contains(&"advertised"));
    assert!(FEATURE_GRID_COLUMNS.contains(&"description"));
    assert!(FEATURE_GRID_COLUMNS.contains(&"counts_in_coverage"));
    assert!(FEATURE_GRID_COLUMNS.contains(&"tests"));
}

#[test]
fn feature_grid_columns_are_unique() {
    let mut seen = std::collections::HashSet::new();
    for col in FEATURE_GRID_COLUMNS {
        assert!(seen.insert(*col), "duplicate column: {col}");
    }
}

// ===================================================================
// 2. Catalog functions (re-exported from contracts)
// ===================================================================

#[test]
fn all_features_returns_non_empty() {
    let features = all_features();
    assert!(
        !features.is_empty(),
        "all_features must return at least one feature"
    );
}

#[test]
fn all_features_have_non_empty_ids() {
    for feature in all_features() {
        assert!(!feature.id.is_empty(), "feature id must not be empty");
    }
}

#[test]
fn all_feature_ids_are_unique() {
    let ids: Vec<&str> = all_features().iter().map(|f| f.id).collect();
    let mut unique = ids.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(ids.len(), unique.len(), "feature ids must be unique");
}

#[test]
fn advertised_features_is_subset_of_all() {
    let all_ids: Vec<&str> = all_features().iter().map(|f| f.id).collect();
    for id in advertised_features() {
        assert!(
            all_ids.contains(id),
            "advertised feature '{id}' not found in all_features"
        );
    }
}

#[test]
fn has_feature_returns_true_for_advertised() {
    for id in advertised_features() {
        assert!(
            has_feature(id),
            "has_feature should be true for advertised '{id}'"
        );
    }
}

#[test]
fn has_feature_returns_false_for_unknown() {
    assert!(
        !has_feature("__nonexistent_feature_xyz_1234__"),
        "has_feature should return false for unknown feature"
    );
}

#[test]
fn compliance_percent_is_bounded() {
    let pct = compliance_percent();
    assert!(
        (0.0..=100.0).contains(&pct),
        "compliance_percent out of range: {pct}"
    );
}

#[test]
fn compliance_percent_for_grid_is_bounded() {
    let pct = compliance_percent_for_grid();
    assert!(
        (0.0..=100.0).contains(&pct),
        "compliance_percent_for_grid out of range: {pct}"
    );
}

#[test]
fn trackable_feature_count_is_positive() {
    assert!(
        trackable_feature_count_for_grid() > 0,
        "expected at least one trackable feature"
    );
}

#[test]
fn advertised_trackable_count_le_trackable() {
    assert!(
        advertised_trackable_feature_count_for_grid() <= trackable_feature_count_for_grid(),
        "advertised trackable must not exceed total trackable"
    );
}

// ===================================================================
// 3. BDD feature rows
// ===================================================================

#[test]
fn bdd_feature_rows_non_empty() {
    let rows = bdd_feature_rows();
    assert!(!rows.is_empty(), "bdd_feature_rows must not be empty");
}

#[test]
fn bdd_rows_have_required_fields() {
    for row in bdd_feature_rows() {
        assert!(!row.id.is_empty(), "BddFeatureRow.id must not be empty");
        assert!(
            !row.area.is_empty(),
            "BddFeatureRow.area must not be empty for {}",
            row.id
        );
        assert!(
            !row.description.is_empty(),
            "BddFeatureRow.description must not be empty for {}",
            row.id
        );
    }
}

#[test]
fn bdd_rows_ids_match_all_features() {
    let all_ids: Vec<&str> = all_features().iter().map(|f| f.id).collect();
    for row in bdd_feature_rows() {
        assert!(
            all_ids.contains(&row.id),
            "BddFeatureRow id '{}' not in all_features",
            row.id
        );
    }
}

#[test]
fn bdd_rows_trackable_count_matches() {
    let rows = bdd_feature_rows();
    let trackable = rows.iter().filter(|r| r.counts_in_coverage).count();
    assert_eq!(trackable, trackable_feature_count_for_grid());
}

// ===================================================================
// 4. Feature profile contracts
// ===================================================================

#[test]
fn feature_profile_contracts_non_empty() {
    let contracts = feature_profile_contracts();
    assert!(
        !contracts.is_empty(),
        "feature_profile_contracts must return at least one spec"
    );
}

#[test]
fn feature_profile_specs_have_canonical_names() {
    for spec in feature_profile_contracts() {
        assert!(
            !spec.canonical.is_empty(),
            "FeatureProfileSpec.canonical must not be empty"
        );
        assert!(
            !spec.description.is_empty(),
            "FeatureProfileSpec.description must not be empty for '{}'",
            spec.canonical
        );
    }
}

#[test]
fn profile_contracts_include_known_profiles() {
    let names: Vec<&str> = feature_profile_contracts()
        .iter()
        .map(|s| s.canonical)
        .collect();
    assert!(names.contains(&"ga-lock"), "missing ga-lock profile");
    assert!(names.contains(&"production"), "missing production profile");
    assert!(names.contains(&"all"), "missing all profile");
}

// ===================================================================
// 5. FeatureProfile enum (re-exported from policy)
// ===================================================================

#[test]
fn feature_profile_all_returns_three_or_more() {
    let profiles = FeatureProfile::all();
    assert!(profiles.len() >= 3, "expected at least 3 profiles");
}

#[test]
fn feature_profile_as_str_round_trips() {
    for &profile in FeatureProfile::all() {
        let label = profile.as_str();
        assert!(!label.is_empty());
    }
}

#[test]
fn feature_profile_ga_lock_str() {
    assert_eq!(FeatureProfile::GaLock.as_str(), "ga-lock");
}

#[test]
fn feature_profile_production_str() {
    assert_eq!(FeatureProfile::Production.as_str(), "production");
}

#[test]
fn feature_profile_all_str() {
    assert_eq!(FeatureProfile::All.as_str(), "all");
}

#[test]
fn catalog_advertised_feature_ids_non_empty_for_all() {
    let ids = catalog_advertised_feature_ids(FeatureProfile::All);
    assert!(
        !ids.is_empty(),
        "All profile should advertise at least one feature"
    );
}

#[test]
fn catalog_advertised_ga_lock_subset_of_all() {
    let all = catalog_advertised_feature_ids(FeatureProfile::All);
    let ga = catalog_advertised_feature_ids(FeatureProfile::GaLock);
    for id in &ga {
        assert!(
            all.contains(id),
            "ga-lock feature '{id}' should also be in 'all' profile"
        );
    }
}

#[test]
fn catalog_advertised_production_subset_of_all() {
    let all = catalog_advertised_feature_ids(FeatureProfile::All);
    let prod = catalog_advertised_feature_ids(FeatureProfile::Production);
    for id in &prod {
        assert!(
            all.contains(id),
            "production feature '{id}' should also be in 'all' profile"
        );
    }
}

#[test]
fn all_profile_has_most_advertised_features() {
    let all_count = catalog_advertised_feature_ids(FeatureProfile::All).len();
    let ga_count = catalog_advertised_feature_ids(FeatureProfile::GaLock).len();
    let prod_count = catalog_advertised_feature_ids(FeatureProfile::Production).len();
    assert!(all_count >= ga_count);
    assert!(all_count >= prod_count);
}

// ===================================================================
// 6. compliance_percent_for_profile
// ===================================================================

#[test]
fn compliance_percent_for_each_profile_is_bounded() {
    for &profile in FeatureProfile::all() {
        let pct = compliance_percent_for_profile(profile);
        assert!(
            (0.0..=100.0).contains(&pct),
            "compliance for {} out of range: {pct}",
            profile.as_str()
        );
    }
}

#[test]
fn compliance_all_ge_ga_lock() {
    let all = compliance_percent_for_profile(FeatureProfile::All);
    let ga = compliance_percent_for_profile(FeatureProfile::GaLock);
    assert!(
        all >= ga,
        "All compliance ({all}) should be >= ga-lock ({ga})"
    );
}

#[test]
fn compliance_all_ge_production() {
    let all = compliance_percent_for_profile(FeatureProfile::All);
    let prod = compliance_percent_for_profile(FeatureProfile::Production);
    assert!(
        all >= prod,
        "All compliance ({all}) should be >= production ({prod})"
    );
}

// ===================================================================
// 7. to_json() — default catalog JSON
// ===================================================================

#[test]
fn to_json_is_valid_json() -> Result<(), Box<dyn std::error::Error>> {
    let _: Value = parse_json(&to_json())?;
    Ok(())
}

#[test]
fn to_json_contains_required_top_level_keys() -> Result<(), Box<dyn std::error::Error>> {
    let value = parse_json(&to_json())?;
    let required = [
        "version",
        "lsp_version",
        "compliance_percent",
        "trackable_feature_count",
        "advertised_trackable_feature_count",
        "advertised",
        "feature_profiles",
        "feature_grid",
        "profiles",
        "feature_count",
    ];
    for key in required {
        assert!(value.get(key).is_some(), "missing key: {key}");
    }
    Ok(())
}

#[test]
fn to_json_does_not_include_profile_key() -> Result<(), Box<dyn std::error::Error>> {
    let value = parse_json(&to_json())?;
    assert!(
        value.get("profile").is_none(),
        "to_json() should not set a single 'profile' key"
    );
    Ok(())
}

#[test]
fn to_json_version_matches_constant() -> Result<(), Box<dyn std::error::Error>> {
    let value = parse_json(&to_json())?;
    let version = must_some(value["version"].as_str());
    assert_eq!(version, VERSION);
    Ok(())
}

#[test]
fn to_json_lsp_version_matches_constant() -> Result<(), Box<dyn std::error::Error>> {
    let value = parse_json(&to_json())?;
    let lsp_version = must_some(value["lsp_version"].as_str());
    assert_eq!(lsp_version, LSP_VERSION);
    Ok(())
}

#[test]
fn to_json_feature_count_matches_catalog() -> Result<(), Box<dyn std::error::Error>> {
    let value = parse_json(&to_json())?;
    let count = must_some(value["feature_count"].as_u64());
    assert_eq!(count as usize, all_features().len());
    Ok(())
}

#[test]
fn to_json_grid_columns_match_constant() -> Result<(), Box<dyn std::error::Error>> {
    let value = parse_json(&to_json())?;
    let columns = must_some(value["feature_grid"]["columns"].as_array());
    let col_strs: Vec<&str> = columns.iter().filter_map(|v| v.as_str()).collect();
    assert_eq!(col_strs.as_slice(), FEATURE_GRID_COLUMNS);
    Ok(())
}

#[test]
fn to_json_grid_rows_count_matches_all_features() -> Result<(), Box<dyn std::error::Error>> {
    let value = parse_json(&to_json())?;
    let rows = must_some(value["feature_grid"]["rows"].as_array());
    assert_eq!(rows.len(), all_features().len());
    Ok(())
}

#[test]
fn to_json_advertised_is_array() -> Result<(), Box<dyn std::error::Error>> {
    let value = parse_json(&to_json())?;
    assert!(
        value["advertised"].is_array(),
        "'advertised' should be an array"
    );
    Ok(())
}

#[test]
fn to_json_compliance_percent_is_number() -> Result<(), Box<dyn std::error::Error>> {
    let value = parse_json(&to_json())?;
    assert!(
        value["compliance_percent"].is_number(),
        "'compliance_percent' should be a number"
    );
    Ok(())
}

#[test]
fn to_json_trackable_counts_are_consistent() -> Result<(), Box<dyn std::error::Error>> {
    let value = parse_json(&to_json())?;
    let trackable = must_some(value["trackable_feature_count"].as_u64());
    let advertised_trackable = must_some(value["advertised_trackable_feature_count"].as_u64());
    assert!(
        advertised_trackable <= trackable,
        "advertised_trackable ({advertised_trackable}) > trackable ({trackable})"
    );
    Ok(())
}

#[test]
fn to_json_feature_profiles_is_non_empty_array() -> Result<(), Box<dyn std::error::Error>> {
    let value = parse_json(&to_json())?;
    let profiles = must_some(value["feature_profiles"].as_array());
    assert!(!profiles.is_empty());
    Ok(())
}

// ===================================================================
// 8. to_json_for_profile — single-profile JSON
// ===================================================================

#[test]
fn to_json_for_profile_includes_profile_key() -> Result<(), Box<dyn std::error::Error>> {
    for &profile in FeatureProfile::all() {
        let value = parse_json(&to_json_for_profile(profile))?;
        let profile_str = must_some(value["profile"].as_str());
        assert_eq!(profile_str, profile.as_str());
    }
    Ok(())
}

#[test]
fn to_json_for_profile_has_single_profile_in_profiles_array()
-> Result<(), Box<dyn std::error::Error>> {
    let value = parse_json(&to_json_for_profile(FeatureProfile::Production))?;
    let profiles = must_some(value["profiles"].as_array());
    assert_eq!(
        profiles.len(),
        1,
        "single-profile JSON should have exactly 1 profile summary"
    );
    Ok(())
}

#[test]
fn to_json_for_profile_compliance_matches_function() -> Result<(), Box<dyn std::error::Error>> {
    for &profile in FeatureProfile::all() {
        let value = parse_json(&to_json_for_profile(profile))?;
        let json_pct = must_some(value["compliance_percent"].as_f64());
        let fn_pct = compliance_percent_for_profile(profile) as f64;
        assert!(
            (json_pct - fn_pct).abs() < f64::from(f32::EPSILON),
            "compliance mismatch for {}: json={json_pct} fn={fn_pct}",
            profile.as_str()
        );
    }
    Ok(())
}

#[test]
fn to_json_for_profile_advertised_matches_catalog() -> Result<(), Box<dyn std::error::Error>> {
    for &profile in FeatureProfile::all() {
        let value = parse_json(&to_json_for_profile(profile))?;
        let json_advertised: Vec<&str> = must_some(value["advertised"].as_array())
            .iter()
            .filter_map(|v| v.as_str())
            .collect();
        let catalog_ids = catalog_advertised_feature_ids(profile);
        assert_eq!(
            json_advertised.len(),
            catalog_ids.len(),
            "advertised count mismatch for {}",
            profile.as_str()
        );
    }
    Ok(())
}

// ===================================================================
// 9. to_json_for_profiles — multi-profile JSON
// ===================================================================

#[test]
fn to_json_for_profiles_with_empty_slice() -> Result<(), Box<dyn std::error::Error>> {
    let value = parse_json(&to_json_for_profiles(&[]))?;
    let profiles = must_some(value["profiles"].as_array());
    assert!(
        profiles.is_empty(),
        "empty input should yield empty profiles array"
    );
    Ok(())
}

#[test]
fn to_json_for_profiles_with_single_profile() -> Result<(), Box<dyn std::error::Error>> {
    let value = parse_json(&to_json_for_profiles(&[FeatureProfile::GaLock]))?;
    let profiles = must_some(value["profiles"].as_array());
    assert_eq!(profiles.len(), 1);
    let name = must_some(profiles[0]["profile"].as_str());
    assert_eq!(name, "ga-lock");
    Ok(())
}

#[test]
fn to_json_for_profiles_does_not_set_profile_key() -> Result<(), Box<dyn std::error::Error>> {
    let value = parse_json(&to_json_for_profiles(FeatureProfile::all()))?;
    assert!(
        value.get("profile").is_none(),
        "multi-profile variant should not set top-level 'profile'"
    );
    Ok(())
}

#[test]
fn to_json_for_profiles_includes_all_requested() -> Result<(), Box<dyn std::error::Error>> {
    let requested = &[FeatureProfile::GaLock, FeatureProfile::All];
    let value = parse_json(&to_json_for_profiles(requested))?;
    let profiles = must_some(value["profiles"].as_array());
    assert_eq!(profiles.len(), requested.len());
    let names: Vec<&str> = profiles
        .iter()
        .filter_map(|p| p["profile"].as_str())
        .collect();
    assert!(names.contains(&"ga-lock"));
    assert!(names.contains(&"all"));
    Ok(())
}

// ===================================================================
// 10. to_json_for_all_profiles
// ===================================================================

#[test]
fn to_json_for_all_profiles_includes_every_canonical_profile()
-> Result<(), Box<dyn std::error::Error>> {
    let value = parse_json(&to_json_for_all_profiles())?;
    let profiles = must_some(value["profiles"].as_array());
    let names: Vec<&str> = profiles
        .iter()
        .filter_map(|p| p["profile"].as_str())
        .collect();
    for &profile in FeatureProfile::all() {
        assert!(
            names.contains(&profile.as_str()),
            "missing profile '{}' in to_json_for_all_profiles()",
            profile.as_str()
        );
    }
    Ok(())
}

#[test]
fn to_json_for_all_profiles_is_deterministic() -> Result<(), Box<dyn std::error::Error>> {
    let first = to_json_for_all_profiles();
    let second = to_json_for_all_profiles();
    assert_eq!(
        first, second,
        "to_json_for_all_profiles should be deterministic"
    );
    Ok(())
}

// ===================================================================
// 11. Profile summary structure
// ===================================================================

#[test]
fn profile_summaries_contain_required_keys() -> Result<(), Box<dyn std::error::Error>> {
    let value = parse_json(&to_json_for_all_profiles())?;
    let profiles = must_some(value["profiles"].as_array());
    let required = [
        "profile",
        "advertised",
        "compliance_percent",
        "trackable_feature_count",
        "advertised_trackable_feature_count",
        "advertised_feature_count",
    ];
    for profile in profiles {
        for key in required {
            assert!(
                profile.get(key).is_some(),
                "profile summary missing key '{key}'"
            );
        }
    }
    Ok(())
}

#[test]
fn profile_summary_trackable_count_matches_global() -> Result<(), Box<dyn std::error::Error>> {
    let value = parse_json(&to_json_for_all_profiles())?;
    let global_trackable = must_some(value["trackable_feature_count"].as_u64());
    let profiles = must_some(value["profiles"].as_array());
    for profile in profiles {
        let trackable = must_some(profile["trackable_feature_count"].as_u64());
        assert_eq!(
            trackable, global_trackable,
            "each profile summary should share the same trackable_feature_count"
        );
    }
    Ok(())
}

#[test]
fn profile_summary_advertised_count_matches_array_len() -> Result<(), Box<dyn std::error::Error>> {
    let value = parse_json(&to_json_for_all_profiles())?;
    let profiles = must_some(value["profiles"].as_array());
    for p in profiles {
        let arr_len = must_some(p["advertised"].as_array()).len() as u64;
        let count = must_some(p["advertised_feature_count"].as_u64());
        assert_eq!(
            arr_len, count,
            "advertised array length should match advertised_feature_count"
        );
    }
    Ok(())
}

// ===================================================================
// 12. BDD grid rows in JSON
// ===================================================================

#[test]
fn json_grid_rows_have_expected_structure() -> Result<(), Box<dyn std::error::Error>> {
    let value = parse_json(&to_json())?;
    let rows = must_some(value["feature_grid"]["rows"].as_array());
    for row in rows {
        assert!(row.get("id").is_some(), "grid row missing 'id'");
        assert!(row.get("area").is_some(), "grid row missing 'area'");
        assert!(
            row.get("advertised").is_some(),
            "grid row missing 'advertised'"
        );
        assert!(
            row.get("counts_in_coverage").is_some(),
            "grid row missing 'counts_in_coverage'"
        );
    }
    Ok(())
}

// ===================================================================
// 13. Cross-consistency checks
// ===================================================================

#[test]
fn default_json_compliance_matches_grid_function() -> Result<(), Box<dyn std::error::Error>> {
    let value = parse_json(&to_json())?;
    let json_pct = must_some(value["compliance_percent"].as_f64());
    let grid_pct = compliance_percent_for_grid() as f64;
    assert!(
        (json_pct - grid_pct).abs() < f64::from(f32::EPSILON),
        "to_json compliance {json_pct} != compliance_percent_for_grid {grid_pct}"
    );
    Ok(())
}

#[test]
fn default_json_trackable_matches_grid_function() -> Result<(), Box<dyn std::error::Error>> {
    let value = parse_json(&to_json())?;
    let json_trackable = must_some(value["trackable_feature_count"].as_u64());
    assert_eq!(json_trackable as usize, trackable_feature_count_for_grid());
    Ok(())
}

#[test]
fn default_json_advertised_trackable_matches_grid_function()
-> Result<(), Box<dyn std::error::Error>> {
    let value = parse_json(&to_json())?;
    let json_adv = must_some(value["advertised_trackable_feature_count"].as_u64());
    assert_eq!(
        json_adv as usize,
        advertised_trackable_feature_count_for_grid()
    );
    Ok(())
}

#[test]
fn all_profile_compliance_le_grid_compliance() {
    let profile_pct = compliance_percent_for_profile(FeatureProfile::All);
    let grid_pct = compliance_percent_for_grid();
    // The grid compliance uses built-in advertised features directly, while
    // the profile compliance intersects catalog IDs. Grid compliance is an
    // upper bound.
    assert!(
        profile_pct <= grid_pct,
        "All profile ({profile_pct}) should be <= grid compliance ({grid_pct})"
    );
}

#[test]
fn to_json_deterministic() {
    let a = to_json();
    let b = to_json();
    assert_eq!(a, b, "to_json() should be deterministic");
}
