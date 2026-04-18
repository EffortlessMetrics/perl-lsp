#![warn(missing_docs)]
//! BDD grid and feature-profile interoperability primitives.
//!
//! This crate intentionally contains only compatibility and reporting logic used by
//! both the LSP binary and external tooling. It sits above the contract and
//! policy microcrates to avoid feature-flag logic leaking back into the server
//! module tree.

pub use crate::features::contracts::FeatureProfileSpec;
pub use crate::features::contracts::catalog;
pub use crate::features::contracts::feature_profile_specs;
pub use crate::features::contracts::{
    BddFeatureRow, Feature, LSP_VERSION, VERSION, advertised_features,
    advertised_trackable_feature_count_for_grid, all_features, bdd_feature_rows,
    compliance_percent, compliance_percent_for_grid, has_feature, trackable_feature_count_for_grid,
};
pub use crate::features::policy::{FeatureProfile, catalog_advertised_feature_ids};

use serde_json::{Value, json};

/// Return profile metadata for interoperability with CLI and editor tooling.
pub const fn feature_profile_contracts() -> &'static [FeatureProfileSpec] {
    feature_profile_specs()
}

/// Stable BDD grid column order used by reporting tools.
pub const FEATURE_GRID_COLUMNS: &[&str] =
    &["area", "id", "spec", "maturity", "advertised", "counts_in_coverage", "description", "tests"];

/// Get the global feature catalog as JSON.
///
/// This mirrors the historical server output and includes catalog-wide
/// advertised features (not profile-filtered), plus all profile summaries for
/// visibility and interoperability.
pub fn to_json() -> String {
    to_json_for_profiles(FeatureProfile::all())
}

/// Profile-aware feature catalog JSON.
///
/// The advertised feature list and compliance math are derived from the provided
/// runtime profile. This is useful for feature flag snapshots in CI and tooling.
pub fn to_json_for_profile(profile: FeatureProfile) -> String {
    feature_grid_payload(&[profile], Some(profile)).to_string()
}

/// BDD-compatible feature catalog JSON for an explicit profile set.
pub fn to_json_for_profiles(profiles: &[FeatureProfile]) -> String {
    feature_grid_payload(profiles, None).to_string()
}

/// BDD-compatible feature catalog JSON with all canonical profiles.
pub fn to_json_for_all_profiles() -> String {
    to_json_for_profiles(FeatureProfile::all())
}

/// Compliance percent for a specific runtime profile, using the same grid semantics.
pub fn compliance_percent_for_profile(profile: FeatureProfile) -> f32 {
    let trackable_feature_count = trackable_feature_count_for_grid();
    if trackable_feature_count == 0 {
        return 0.0;
    }

    let advertised = catalog_advertised_feature_ids(profile);
    let advertised_trackable_feature_count = advertised_trackable_feature_count(&advertised);
    (advertised_trackable_feature_count as f64 / trackable_feature_count as f64 * 100.0).round()
        as f32
}

fn advertised_trackable_feature_count(advertised: &[&'static str]) -> usize {
    advertised
        .iter()
        .filter(|&&id| {
            has_feature(id)
                && all_features()
                    .iter()
                    .find(|feature| feature.id == id)
                    .is_some_and(|feature| feature.counts_in_coverage)
        })
        .count()
}

fn feature_grid_payload(
    profiles: &[FeatureProfile],
    selected_profile: Option<FeatureProfile>,
) -> Value {
    let profile_summaries: Vec<Value> = profiles.iter().copied().map(profile_summary).collect();

    let (advertised, advertised_trackable_feature_count) = match selected_profile {
        Some(profile) => {
            let advertised = catalog_advertised_feature_ids(profile);
            let advertised_trackable_feature_count =
                advertised_trackable_feature_count(&advertised);
            (advertised, advertised_trackable_feature_count)
        }
        None => (advertised_features().to_vec(), advertised_trackable_feature_count_for_grid()),
    };
    let trackable_feature_count = trackable_feature_count_for_grid();
    let compliance_percent = if trackable_feature_count == 0 {
        0.0
    } else {
        (advertised_trackable_feature_count as f64 / trackable_feature_count as f64 * 100.0).round()
            as f32
    };
    let mut payload = json!({
        "version": VERSION,
        "lsp_version": LSP_VERSION,
        "compliance_percent": compliance_percent,
        "trackable_feature_count": trackable_feature_count,
        "advertised_trackable_feature_count": advertised_trackable_feature_count,
        "advertised": advertised,
        "feature_profiles": feature_profile_contracts(),
        "feature_grid": {
            "columns": FEATURE_GRID_COLUMNS,
            "rows": bdd_feature_rows(),
        },
        "profiles": profile_summaries,
        "feature_count": all_features().len(),
    });

    if let Some(profile) = selected_profile {
        payload["profile"] = json!(profile.as_str());
    }

    payload
}

fn profile_summary(profile: FeatureProfile) -> Value {
    let advertised = catalog_advertised_feature_ids(profile);
    let advertised_trackable_feature_count = advertised_trackable_feature_count(&advertised);
    let trackable_feature_count = trackable_feature_count_for_grid();
    let compliance_percent = if trackable_feature_count == 0 {
        0.0
    } else {
        (advertised_trackable_feature_count as f64 / trackable_feature_count as f64 * 100.0).round()
            as f32
    };

    json!({
        "profile": profile.as_str(),
        "advertised": advertised,
        "compliance_percent": compliance_percent,
        "trackable_feature_count": trackable_feature_count,
        "advertised_trackable_feature_count": advertised_trackable_feature_count,
        "advertised_feature_count": advertised.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::{
        FeatureProfile, compliance_percent_for_profile, to_json, to_json_for_all_profiles,
        to_json_for_profile,
    };
    use perl_tdd_support::{must, must_some};

    #[test]
    fn payload_is_stable_for_default_catalog_json() {
        let payload = to_json();
        let value: serde_json::Value = must(serde_json::from_str(&payload));

        assert!(value.get("version").is_some());
        assert!(value.get("lsp_version").is_some());
        assert!(value.get("compliance_percent").is_some());
        assert!(value.get("feature_grid").is_some());
        assert!(value.get("feature_profiles").is_some());
        assert!(value.get("profiles").is_some());
        assert!(value["feature_grid"].get("columns").is_some());
        assert!(value["feature_grid"].get("rows").is_some());
        let profiles = must_some(value.get("profiles").and_then(|profiles| profiles.as_array()));
        assert!(!profiles.is_empty());
        let rows = must_some(
            value
                .get("feature_grid")
                .and_then(|grid| grid.get("rows"))
                .and_then(|rows| rows.as_array()),
        );
        assert!(!rows.is_empty());
    }

    #[test]
    fn payload_is_profile_scoped() {
        let all = to_json_for_profile(FeatureProfile::All);
        let ga_lock = to_json_for_profile(FeatureProfile::GaLock);
        let all_compliance = compliance_percent_for_profile(FeatureProfile::All);
        let ga_compliance = compliance_percent_for_profile(FeatureProfile::GaLock);

        let all_value: serde_json::Value = must(serde_json::from_str(&all));
        let ga_lock_value: serde_json::Value = must(serde_json::from_str(&ga_lock));

        assert_eq!(all_value["profile"].as_str(), Some("all"));
        assert_eq!(ga_lock_value["profile"].as_str(), Some("ga-lock"));

        let json_all_compliance = must_some(all_value["compliance_percent"].as_f64());
        let json_ga_compliance = must_some(ga_lock_value["compliance_percent"].as_f64());
        assert!((json_all_compliance - all_compliance as f64).abs() < f32::EPSILON as f64);
        assert!((json_ga_compliance - ga_compliance as f64).abs() < f32::EPSILON as f64);

        let all_count = all_value["advertised_trackable_feature_count"].as_u64().unwrap_or(0);
        let ga_count = ga_lock_value["advertised_trackable_feature_count"].as_u64().unwrap_or(0);
        assert!(all_count >= ga_count);
    }

    #[test]
    fn payload_includes_multi_profile_projection() {
        let payload = to_json_for_all_profiles();
        let value: serde_json::Value = must(serde_json::from_str(&payload));
        let profiles = must_some(value.get("profiles").and_then(|value| value.as_array()));
        assert!(profiles.len() >= 3);

        let keys: Vec<_> = profiles
            .iter()
            .filter_map(|profile| profile.get("profile").and_then(|p| p.as_str()))
            .collect();
        assert!(keys.contains(&"ga-lock"));
        assert!(keys.contains(&"production"));
        assert!(keys.contains(&"all"));
    }

    // ── compliance_percent_for_profile ───────────────────────────────

    #[test]
    fn compliance_percent_is_in_valid_range_for_all_profiles() {
        for profile in FeatureProfile::all() {
            let pct = compliance_percent_for_profile(*profile);
            assert!(
                (0.0..=100.0).contains(&pct),
                "compliance for {} should be in [0, 100], got {}",
                profile.as_str(),
                pct
            );
        }
    }

    #[test]
    fn all_profile_compliance_gte_ga_lock_compliance() {
        let all_pct = compliance_percent_for_profile(FeatureProfile::All);
        let ga_pct = compliance_percent_for_profile(FeatureProfile::GaLock);
        assert!(all_pct >= ga_pct, "'all' compliance ({all_pct}) should be >= ga-lock ({ga_pct})");
    }

    // ── feature_profile_contracts ───────────────────────────────────

    #[test]
    fn feature_profile_contracts_returns_specs() {
        let contracts = super::feature_profile_contracts();
        assert_eq!(contracts.len(), 3);
        assert_eq!(contracts[0].canonical, "ga-lock");
        assert_eq!(contracts[1].canonical, "production");
        assert_eq!(contracts[2].canonical, "all");
    }

    // ── FEATURE_GRID_COLUMNS ────────────────────────────────────────

    #[test]
    fn feature_grid_columns_has_expected_entries() {
        assert!(super::FEATURE_GRID_COLUMNS.contains(&"id"));
        assert!(super::FEATURE_GRID_COLUMNS.contains(&"area"));
        assert!(super::FEATURE_GRID_COLUMNS.contains(&"spec"));
        assert!(super::FEATURE_GRID_COLUMNS.contains(&"maturity"));
        assert!(super::FEATURE_GRID_COLUMNS.contains(&"advertised"));
        assert!(super::FEATURE_GRID_COLUMNS.contains(&"counts_in_coverage"));
        assert!(super::FEATURE_GRID_COLUMNS.contains(&"description"));
        assert!(super::FEATURE_GRID_COLUMNS.contains(&"tests"));
    }

    // ── to_json_for_profiles ────────────────────────────────────────

    #[test]
    fn to_json_for_profiles_subset() {
        let payload = super::to_json_for_profiles(&[FeatureProfile::GaLock]);
        let value: serde_json::Value = must(serde_json::from_str(&payload));
        let profiles = must_some(value.get("profiles").and_then(|v| v.as_array()));
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0]["profile"].as_str(), Some("ga-lock"));
    }

    // ── Profile summary fields ──────────────────────────────────────

    #[test]
    fn profile_summary_contains_required_keys() {
        let payload = to_json_for_all_profiles();
        let value: serde_json::Value = must(serde_json::from_str(&payload));
        let profiles = must_some(value.get("profiles").and_then(|v| v.as_array()));
        for profile_value in profiles {
            assert!(profile_value.get("profile").is_some(), "missing 'profile' key");
            assert!(profile_value.get("advertised").is_some(), "missing 'advertised' key");
            assert!(
                profile_value.get("compliance_percent").is_some(),
                "missing 'compliance_percent'"
            );
            assert!(
                profile_value.get("trackable_feature_count").is_some(),
                "missing 'trackable_feature_count'"
            );
            assert!(
                profile_value.get("advertised_trackable_feature_count").is_some(),
                "missing 'advertised_trackable_feature_count'"
            );
            assert!(
                profile_value.get("advertised_feature_count").is_some(),
                "missing 'advertised_feature_count'"
            );
        }
    }

    // ── Production profile JSON ─────────────────────────────────────

    #[test]
    fn to_json_for_production_profile() {
        let payload = to_json_for_profile(FeatureProfile::Production);
        let value: serde_json::Value = must(serde_json::from_str(&payload));
        assert_eq!(value["profile"].as_str(), Some("production"));
        assert!(value.get("feature_count").is_some());
    }

    // ── Default to_json has no profile key ───────────────────────────

    #[test]
    fn default_to_json_omits_profile_key() {
        let payload = to_json();
        let value: serde_json::Value = must(serde_json::from_str(&payload));
        assert!(
            value.get("profile").is_none(),
            "default to_json() should not have a 'profile' key"
        );
    }
}
