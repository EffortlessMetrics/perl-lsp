//! Extended unit tests for the `perl-lsp-feature-grid` crate.
//!
//! These tests supplement the comprehensive test suite by covering additional
//! edge cases, validation scenarios, and combinations of public API functions.
//! Focus areas include:
//! - Advanced JSON payload validation
//! - Feature profile interactions and edge cases
//! - Compliance percent calculations with various conditions
//! - Grid structure consistency checks
//! - Multi-profile payload scenarios
#![allow(clippy::expect_used)]

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
use std::collections::HashSet;

// ---------------------------------------------------------------------------
// Helper Functions
// ---------------------------------------------------------------------------

fn _parse_json(s: &str) -> Result<Value, Box<dyn std::error::Error>> {
    Ok(serde_json::from_str(s)?)
}

fn get_profile_from_json(value: &Value, profile_name: &str) -> Option<Value> {
    value
        .get("profiles")?
        .as_array()?
        .iter()
        .find(|p| p.get("profile").and_then(|pn| pn.as_str()) == Some(profile_name))
        .cloned()
}

fn extract_grid_rows(value: &Value) -> Option<Vec<Value>> {
    value.get("feature_grid")?.get("rows")?.as_array().cloned()
}

// ===================================================================
// 1. Version and Constants Edge Cases
// ===================================================================

#[test]
fn version_constant_no_leading_trailing_whitespace() {
    assert_eq!(
        VERSION,
        VERSION.trim(),
        "VERSION should not have leading/trailing whitespace"
    );
}

#[test]
fn lsp_version_constant_no_leading_trailing_whitespace() {
    assert_eq!(
        LSP_VERSION,
        LSP_VERSION.trim(),
        "LSP_VERSION should not have leading/trailing whitespace"
    );
}

#[test]
fn feature_grid_columns_exactly_eight_columns() {
    assert_eq!(
        FEATURE_GRID_COLUMNS.len(),
        8,
        "Expected exactly 8 grid columns"
    );
}

#[test]
fn feature_grid_columns_order_is_stable() {
    let expected_order = vec![
        "area",
        "id",
        "spec",
        "maturity",
        "advertised",
        "counts_in_coverage",
        "description",
        "tests",
    ];
    assert_eq!(
        FEATURE_GRID_COLUMNS.to_vec(),
        expected_order,
        "Grid columns order must be stable for BDD tools"
    );
}

// ===================================================================
// 2. Feature Catalog Advanced Validation
// ===================================================================

#[test]
fn all_features_maintains_stable_order() {
    let features1 = all_features();
    let features2 = all_features();
    let ids1: Vec<&str> = features1.iter().map(|f| f.id).collect();
    let ids2: Vec<&str> = features2.iter().map(|f| f.id).collect();
    assert_eq!(
        ids1, ids2,
        "all_features should return consistent ordering across calls"
    );
}

#[test]
fn all_features_areas_are_non_empty_strings() {
    for feature in all_features() {
        assert!(
            !feature.area.is_empty(),
            "feature {} area must not be empty",
            feature.id
        );
    }
}

#[test]
fn all_features_descriptions_are_non_empty_strings() {
    for feature in all_features() {
        assert!(
            !feature.description.is_empty(),
            "feature {} description must not be empty",
            feature.id
        );
    }
}

#[test]
fn all_features_have_valid_maturity_values() {
    // Maturity values should be one of the known types, but we don't assert consistency
    let valid_maturities = ["draft", "beta", "stable", "deprecated", "ga"];
    for feature in all_features() {
        assert!(
            valid_maturities.contains(&feature.maturity),
            "feature {} has invalid maturity: {}",
            feature.id,
            feature.maturity
        );
    }
}

#[test]
fn advertised_features_count_is_reasonable() {
    let advertised = advertised_features();
    let all = all_features();
    assert!(
        !advertised.is_empty() && advertised.len() <= all.len(),
        "advertised features count must be between 1 and total features"
    );
}

#[test]
fn trackable_features_are_subset_of_all() {
    let all_ids: HashSet<&str> = all_features().iter().map(|f| f.id).collect();
    for feature in all_features() {
        if feature.counts_in_coverage {
            assert!(
                all_ids.contains(feature.id),
                "trackable feature {} not in all_features",
                feature.id
            );
        }
    }
}

#[test]
fn advertised_trackable_count_le_all_trackable() {
    let all_trackable = trackable_feature_count_for_grid();
    let adv_trackable = advertised_trackable_feature_count_for_grid();
    assert!(
        adv_trackable <= all_trackable,
        "advertised trackable must not exceed total trackable"
    );
}

// ===================================================================
// 3. Feature Lookup and Validation
// ===================================================================

#[test]
fn has_feature_returns_false_for_unknown_ids() {
    assert!(
        !has_feature("unknown_feature_xyz_123"),
        "has_feature should return false for unknown id"
    );
}

#[test]
fn has_feature_case_sensitive() {
    if has_feature("completion") {
        assert!(
            !has_feature("Completion"),
            "has_feature should be case-sensitive"
        );
        assert!(
            !has_feature("COMPLETION"),
            "has_feature should be case-sensitive"
        );
    }
}

#[test]
fn has_feature_whitespace_sensitive() {
    if has_feature("completion") {
        assert!(
            !has_feature(" completion"),
            "has_feature should reject leading whitespace"
        );
        assert!(
            !has_feature("completion "),
            "has_feature should reject trailing whitespace"
        );
    }
}

// ===================================================================
// 4. Compliance Percent Edge Cases
// ===================================================================

#[test]
fn compliance_percent_never_exceeds_100() {
    let pct = compliance_percent();
    assert!(pct <= 100.0, "compliance_percent must not exceed 100%");
}

#[test]
fn compliance_percent_is_non_negative() {
    let pct = compliance_percent();
    assert!(pct >= 0.0, "compliance_percent must be non-negative");
}

#[test]
fn compliance_percent_for_grid_never_exceeds_100() {
    let pct = compliance_percent_for_grid();
    assert!(
        pct <= 100.0,
        "compliance_percent_for_grid must not exceed 100%"
    );
}

#[test]
fn compliance_percent_for_grid_is_non_negative() {
    let pct = compliance_percent_for_grid();
    assert!(
        pct >= 0.0,
        "compliance_percent_for_grid must be non-negative"
    );
}

#[test]
fn compliance_percent_for_all_profile_is_reasonable() {
    let pct = compliance_percent_for_profile(FeatureProfile::All);
    assert!(
        pct > 50.0,
        "All profile compliance should be substantial (>50%)"
    );
}

#[test]
fn compliance_for_ga_lock_le_compliance_for_all() {
    let ga_pct = compliance_percent_for_profile(FeatureProfile::GaLock);
    let all_pct = compliance_percent_for_profile(FeatureProfile::All);
    assert!(
        ga_pct <= all_pct + f32::EPSILON,
        "GaLock compliance should be <= All compliance"
    );
}

#[test]
fn compliance_for_production_le_compliance_for_all() {
    let prod_pct = compliance_percent_for_profile(FeatureProfile::Production);
    let all_pct = compliance_percent_for_profile(FeatureProfile::All);
    assert!(
        prod_pct <= all_pct + f32::EPSILON,
        "Production compliance should be <= All compliance"
    );
}

// ===================================================================
// 5. BDD Feature Rows Validation
// ===================================================================

#[test]
fn bdd_rows_count_equals_all_features_count() {
    let rows = bdd_feature_rows();
    let features = all_features();
    assert_eq!(
        rows.len(),
        features.len(),
        "bdd rows must match feature count"
    );
}

#[test]
fn bdd_rows_maintains_stable_order() {
    let rows1 = bdd_feature_rows();
    let rows2 = bdd_feature_rows();
    let ids1: Vec<&str> = rows1.iter().map(|r| r.id).collect();
    let ids2: Vec<&str> = rows2.iter().map(|r| r.id).collect();
    assert_eq!(
        ids1, ids2,
        "bdd_feature_rows should return consistent ordering across calls"
    );
}

#[test]
fn bdd_rows_all_areas_non_empty() {
    for row in bdd_feature_rows() {
        assert!(!row.area.is_empty(), "bdd row {} has empty area", row.id);
    }
}

#[test]
fn bdd_rows_all_specs_non_empty() {
    for row in bdd_feature_rows() {
        assert!(!row.spec.is_empty(), "bdd row {} has empty spec", row.id);
    }
}

#[test]
fn bdd_rows_all_descriptions_non_empty() {
    for row in bdd_feature_rows() {
        assert!(
            !row.description.is_empty(),
            "bdd row {} has empty description",
            row.id
        );
    }
}

#[test]
fn bdd_rows_advertised_field_consistency() {
    for row in bdd_feature_rows() {
        let is_advertised = advertised_features().contains(&row.id);
        assert_eq!(
            row.advertised, is_advertised,
            "bdd row {} advertised field mismatch",
            row.id
        );
    }
}

// ===================================================================
// 6. Feature Profile Functions
// ===================================================================

#[test]
fn all_profile_returns_all_available_profiles() {
    let profiles = FeatureProfile::all();
    assert!(
        profiles.len() >= 3,
        "should have at least ga-lock, production, and all"
    );
    let profile_strs: Vec<_> = profiles.iter().map(|p| p.as_str()).collect();
    assert!(profile_strs.contains(&"ga-lock"));
    assert!(profile_strs.contains(&"production"));
    assert!(profile_strs.contains(&"all"));
}

#[test]
fn catalog_advertised_ids_for_all_contains_most_features() {
    let all_ids = catalog_advertised_feature_ids(FeatureProfile::All);
    let ga_ids = catalog_advertised_feature_ids(FeatureProfile::GaLock);
    assert!(
        all_ids.len() >= ga_ids.len(),
        "All profile should advertise at least as many features as GaLock"
    );
}

#[test]
fn catalog_advertised_ids_are_real_features() {
    for profile in FeatureProfile::all() {
        let ids = catalog_advertised_feature_ids(*profile);
        for id in ids {
            assert!(has_feature(id), "advertised id {} not found in catalog", id);
        }
    }
}

#[test]
fn profile_contracts_describe_all_profiles() {
    let contracts = feature_profile_contracts();
    let profile_strs: Vec<_> = contracts.iter().map(|c| c.canonical).collect();
    for profile in FeatureProfile::all() {
        assert!(
            profile_strs.contains(&profile.as_str()),
            "profile {} not described in contracts",
            profile.as_str()
        );
    }
}

// ===================================================================
// 7. JSON Payload Generation - to_json Variants
// ===================================================================

#[test]
fn to_json_is_valid_utf8() {
    let json = to_json();
    // Should not panic if it's valid UTF-8
    assert!(!json.is_empty());
    // Try to parse to ensure it's really JSON
    let _: Value = serde_json::from_str(&json).expect("should be valid JSON");
}

#[test]
fn to_json_reproducible() {
    let json1 = to_json();
    let json2 = to_json();
    assert_eq!(json1, json2, "to_json should be reproducible across calls");
}

#[test]
fn to_json_for_profile_returns_single_profile_key() -> Result<(), Box<dyn std::error::Error>> {
    for profile in FeatureProfile::all() {
        let payload = to_json_for_profile(*profile);
        let value = serde_json::from_str::<Value>(&payload)?;
        assert!(
            value.get("profile").is_some(),
            "to_json_for_profile should include profile key"
        );
        let profile_val = must_some(value.get("profile").and_then(|p| p.as_str()));
        assert_eq!(
            profile_val,
            profile.as_str(),
            "profile key should match requested profile"
        );
    }
    Ok(())
}

#[test]
fn to_json_for_profiles_with_single_profile_structure_matches()
-> Result<(), Box<dyn std::error::Error>> {
    for profile in FeatureProfile::all() {
        let single = to_json_for_profile(*profile);
        let multi = to_json_for_profiles(&[*profile]);

        // Parse both
        let single_val: Value = serde_json::from_str(&single)?;
        let multi_val: Value = serde_json::from_str(&multi)?;

        // Both should have feature_grid with same structure
        assert!(
            single_val.get("feature_grid").is_some(),
            "single payload should have feature_grid"
        );
        assert!(
            multi_val.get("feature_grid").is_some(),
            "multi payload should have feature_grid"
        );

        // Both should have trackable feature counts
        assert!(
            single_val.get("trackable_feature_count").is_some(),
            "single should have trackable_feature_count"
        );
        assert!(
            multi_val.get("trackable_feature_count").is_some(),
            "multi should have trackable_feature_count"
        );
    }
    Ok(())
}

#[test]
fn to_json_for_all_profiles_includes_profile_array() -> Result<(), Box<dyn std::error::Error>> {
    let payload = to_json_for_all_profiles();
    let value = serde_json::from_str::<Value>(&payload)?;
    assert!(
        value.get("profile").is_none(),
        "to_json_for_all_profiles should not have a single profile key"
    );
    let profiles = must_some(value.get("profiles").and_then(|p| p.as_array()));
    assert!(profiles.len() >= 3, "should have at least 3 profiles");
    Ok(())
}

#[test]
fn to_json_for_profiles_deterministic() {
    let profiles = FeatureProfile::all();
    let payload1 = to_json_for_profiles(profiles);
    let payload2 = to_json_for_profiles(profiles);
    assert_eq!(
        payload1, payload2,
        "to_json_for_profiles should be deterministic"
    );
}

// ===================================================================
// 8. JSON Payload Structure Validation
// ===================================================================

#[test]
fn json_payload_feature_count_matches_actual_features() -> Result<(), Box<dyn std::error::Error>> {
    let payload = to_json();
    let value = serde_json::from_str::<Value>(&payload)?;
    let json_count = must_some(value["feature_count"].as_u64());
    let actual_count = all_features().len() as u64;
    assert_eq!(
        json_count, actual_count,
        "feature_count in JSON should match actual features"
    );
    Ok(())
}

#[test]
fn json_payload_grid_rows_count_matches_feature_count() -> Result<(), Box<dyn std::error::Error>> {
    let payload = to_json();
    let value = serde_json::from_str::<Value>(&payload)?;
    let rows = extract_grid_rows(&value).ok_or("grid rows not found")?;
    let feature_count = must_some(value["feature_count"].as_u64());
    assert_eq!(
        rows.len() as u64,
        feature_count,
        "grid rows count should match feature count"
    );
    Ok(())
}

#[test]
fn json_grid_rows_all_have_id_field() -> Result<(), Box<dyn std::error::Error>> {
    let payload = to_json();
    let value = serde_json::from_str::<Value>(&payload)?;
    let rows = extract_grid_rows(&value).ok_or("grid rows not found")?;
    for row in rows {
        assert!(row.get("id").is_some(), "grid row missing id field");
    }
    Ok(())
}

#[test]
fn json_profile_summaries_consistency() -> Result<(), Box<dyn std::error::Error>> {
    let payload = to_json_for_all_profiles();
    let value = serde_json::from_str::<Value>(&payload)?;
    let profiles = must_some(value.get("profiles").and_then(|p| p.as_array()));

    for profile_summary in profiles {
        // Check structure
        assert!(profile_summary.get("profile").is_some());
        assert!(profile_summary.get("advertised").is_some());
        assert!(profile_summary.get("compliance_percent").is_some());
        assert!(profile_summary.get("trackable_feature_count").is_some());
        assert!(
            profile_summary
                .get("advertised_trackable_feature_count")
                .is_some()
        );

        // Check consistency: advertised array length should match advertised_feature_count
        let advertised_arr =
            must_some(profile_summary.get("advertised").and_then(|a| a.as_array()));
        let advertised_count = must_some(
            profile_summary
                .get("advertised_feature_count")
                .and_then(|c| c.as_u64()),
        );
        assert_eq!(
            advertised_arr.len() as u64,
            advertised_count,
            "advertised array length must match advertised_feature_count"
        );
    }
    Ok(())
}

// ===================================================================
// 9. Cross-Payload Consistency
// ===================================================================

#[test]
fn all_profiles_json_includes_ga_lock_profile() -> Result<(), Box<dyn std::error::Error>> {
    let payload = to_json_for_all_profiles();
    let value = serde_json::from_str::<Value>(&payload)?;
    assert!(
        get_profile_from_json(&value, "ga-lock").is_some(),
        "all profiles should include ga-lock"
    );
    Ok(())
}

#[test]
fn all_profiles_json_includes_production_profile() -> Result<(), Box<dyn std::error::Error>> {
    let payload = to_json_for_all_profiles();
    let value = serde_json::from_str::<Value>(&payload)?;
    assert!(
        get_profile_from_json(&value, "production").is_some(),
        "all profiles should include production"
    );
    Ok(())
}

#[test]
fn all_profiles_json_includes_all_profile() -> Result<(), Box<dyn std::error::Error>> {
    let payload = to_json_for_all_profiles();
    let value = serde_json::from_str::<Value>(&payload)?;
    assert!(
        get_profile_from_json(&value, "all").is_some(),
        "all profiles should include all"
    );
    Ok(())
}

#[test]
fn feature_grid_is_same_across_all_payloads() -> Result<(), Box<dyn std::error::Error>> {
    let payload1 = to_json();
    let payload2 = to_json_for_profile(FeatureProfile::All);
    let payload3 = to_json_for_all_profiles();

    let value1 = serde_json::from_str::<Value>(&payload1)?;
    let value2 = serde_json::from_str::<Value>(&payload2)?;
    let value3 = serde_json::from_str::<Value>(&payload3)?;

    let grid1 = value1["feature_grid"]["rows"].to_string();
    let grid2 = value2["feature_grid"]["rows"].to_string();
    let grid3 = value3["feature_grid"]["rows"].to_string();

    assert_eq!(
        grid1, grid2,
        "feature grid should be same in to_json and to_json_for_profile"
    );
    assert_eq!(grid1, grid3, "feature grid should be same in all payloads");
    Ok(())
}

// ===================================================================
// 10. Profile-Specific Compliance Validation
// ===================================================================

#[test]
fn profile_compliance_matches_advertised_count() -> Result<(), Box<dyn std::error::Error>> {
    let total_trackable = trackable_feature_count_for_grid();
    if total_trackable == 0 {
        return Ok(());
    }

    for profile in FeatureProfile::all() {
        let compliance = compliance_percent_for_profile(*profile);
        let advertised = catalog_advertised_feature_ids(*profile);
        let trackable_advertised = advertised
            .iter()
            .filter(|&&id| {
                has_feature(id)
                    && all_features()
                        .iter()
                        .any(|f| f.id == id && f.counts_in_coverage)
            })
            .count();

        let expected =
            (trackable_advertised as f64 / total_trackable as f64 * 100.0).round() as f32;
        assert!(
            (compliance - expected).abs() < f32::EPSILON,
            "profile {} compliance {} doesn't match advertised count {}",
            profile.as_str(),
            compliance,
            expected
        );
    }
    Ok(())
}

// ===================================================================
// 11. Advertised Features Validation
// ===================================================================

#[test]
fn advertised_features_array_all_valid() {
    let advertised = advertised_features();
    for id in advertised {
        assert!(!id.is_empty(), "advertised feature id must not be empty");
        assert!(has_feature(id), "advertised feature {} not in catalog", id);
    }
}

#[test]
fn advertised_features_no_duplicates() {
    let advertised = advertised_features();
    let mut seen = HashSet::new();
    for id in advertised {
        assert!(
            seen.insert(*id),
            "advertised features contains duplicate: {}",
            id
        );
    }
}

// ===================================================================
// 12. JSON Array and Object Validation
// ===================================================================

#[test]
fn json_advertised_array_is_array_of_strings() -> Result<(), Box<dyn std::error::Error>> {
    let payload = to_json();
    let value = serde_json::from_str::<Value>(&payload)?;
    let advertised = must_some(value["advertised"].as_array());
    for item in advertised {
        assert!(
            item.is_string(),
            "advertised array should contain only strings"
        );
    }
    Ok(())
}

#[test]
fn json_feature_grid_columns_is_array_of_strings() -> Result<(), Box<dyn std::error::Error>> {
    let payload = to_json();
    let value = serde_json::from_str::<Value>(&payload)?;
    let columns = must_some(
        value["feature_grid"]
            .get("columns")
            .and_then(|c| c.as_array()),
    );
    for col in columns {
        assert!(col.is_string(), "columns should be strings");
    }
    Ok(())
}

#[test]
fn json_lsp_version_format_is_semver_like() {
    // Should be something like "1.19.0-rc.1" or "1.19.0"
    assert!(!LSP_VERSION.is_empty());
    let parts: Vec<&str> = LSP_VERSION.split('.').collect();
    assert!(
        parts.len() >= 2,
        "LSP_VERSION should have at least major.minor format"
    );
}

// ===================================================================
// 13. Boundary and Special Cases
// ===================================================================

#[test]
fn empty_profile_list_in_to_json_for_profiles_is_safe() {
    let payload = to_json_for_profiles(&[]);
    // Should still produce valid JSON
    let _: Value =
        serde_json::from_str(&payload).expect("should be valid JSON even with empty profiles");
}

#[test]
fn duplicate_profiles_in_to_json_for_profiles_handled() -> Result<(), Box<dyn std::error::Error>> {
    let payload = to_json_for_profiles(&[FeatureProfile::All, FeatureProfile::All]);
    let value = serde_json::from_str::<Value>(&payload)?;
    let profiles = must_some(value.get("profiles").and_then(|p| p.as_array()));
    // May have duplicates or not - both are acceptable, just ensure structure is valid
    assert!(!profiles.is_empty());
    Ok(())
}

#[test]
fn feature_with_missing_description_cannot_exist() {
    for feature in all_features() {
        assert!(
            !feature.description.is_empty(),
            "all features must have descriptions"
        );
    }
}

#[test]
fn trackable_feature_count_never_zero_if_any_feature_is_trackable() {
    let trackable_count = trackable_feature_count_for_grid();
    let has_trackable = all_features().iter().any(|f| f.counts_in_coverage);
    if has_trackable {
        assert!(
            trackable_count > 0,
            "trackable count must be > 0 if features are trackable"
        );
    }
}
