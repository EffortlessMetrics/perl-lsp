//! Comprehensive unit tests for `perl-lsp-feature-contracts`.

use perl_lsp_feature_contracts::{
    FEATURE_PROFILE_SPECS, FeatureProfileKind, advertised_trackable_feature_count_for_grid,
    all_features, bdd_feature_rows, catalog, compliance_percent_for_grid, feature_profile_specs,
    trackable_feature_count_for_grid,
};

// ---------------------------------------------------------------------------
// FeatureProfileKind — from_str_name
// ---------------------------------------------------------------------------

#[test]
fn from_str_name_ga_lock_canonical() -> Result<(), Box<dyn std::error::Error>> {
    let kind = perl_tdd_support::must_some(FeatureProfileKind::from_str_name("ga-lock"));
    assert_eq!(kind, FeatureProfileKind::GaLock);
    Ok(())
}

#[test]
fn from_str_name_ga_alias() -> Result<(), Box<dyn std::error::Error>> {
    let kind = perl_tdd_support::must_some(FeatureProfileKind::from_str_name("ga"));
    assert_eq!(kind, FeatureProfileKind::GaLock);
    Ok(())
}

#[test]
fn from_str_name_ga_underscore_alias() -> Result<(), Box<dyn std::error::Error>> {
    let kind = perl_tdd_support::must_some(FeatureProfileKind::from_str_name("ga_lock"));
    assert_eq!(kind, FeatureProfileKind::GaLock);
    Ok(())
}

#[test]
fn from_str_name_production_canonical() -> Result<(), Box<dyn std::error::Error>> {
    let kind = perl_tdd_support::must_some(FeatureProfileKind::from_str_name("production"));
    assert_eq!(kind, FeatureProfileKind::Production);
    Ok(())
}

#[test]
fn from_str_name_prod_alias() -> Result<(), Box<dyn std::error::Error>> {
    let kind = perl_tdd_support::must_some(FeatureProfileKind::from_str_name("prod"));
    assert_eq!(kind, FeatureProfileKind::Production);
    Ok(())
}

#[test]
fn from_str_name_all() -> Result<(), Box<dyn std::error::Error>> {
    let kind = perl_tdd_support::must_some(FeatureProfileKind::from_str_name("all"));
    assert_eq!(kind, FeatureProfileKind::All);
    Ok(())
}

#[test]
fn from_str_name_auto_resolves_to_current() -> Result<(), Box<dyn std::error::Error>> {
    let kind = perl_tdd_support::must_some(FeatureProfileKind::from_str_name("auto"));
    assert_eq!(kind, FeatureProfileKind::current());
    Ok(())
}

#[test]
fn from_str_name_unknown_returns_none() {
    assert!(FeatureProfileKind::from_str_name("unknown").is_none());
    assert!(FeatureProfileKind::from_str_name("").is_none());
    assert!(FeatureProfileKind::from_str_name("GA-LOCK").is_none());
    assert!(FeatureProfileKind::from_str_name("Production").is_none());
    assert!(FeatureProfileKind::from_str_name("ALL").is_none());
}

// ---------------------------------------------------------------------------
// FeatureProfileKind — from_ga_lock_enabled
// ---------------------------------------------------------------------------

#[test]
fn from_ga_lock_enabled_true() {
    assert_eq!(FeatureProfileKind::from_ga_lock_enabled(true), FeatureProfileKind::GaLock);
}

#[test]
fn from_ga_lock_enabled_false() {
    assert_eq!(FeatureProfileKind::from_ga_lock_enabled(false), FeatureProfileKind::Production);
}

// ---------------------------------------------------------------------------
// FeatureProfileKind — current
// ---------------------------------------------------------------------------

#[test]
fn current_matches_cfg_feature() {
    let expected = if cfg!(feature = "lsp-ga-lock") {
        FeatureProfileKind::GaLock
    } else {
        FeatureProfileKind::Production
    };
    assert_eq!(FeatureProfileKind::current(), expected);
}

// ---------------------------------------------------------------------------
// FeatureProfileKind — as_str
// ---------------------------------------------------------------------------

#[test]
fn as_str_roundtrip_all_variants() -> Result<(), Box<dyn std::error::Error>> {
    for &kind in FeatureProfileKind::all() {
        let s = kind.as_str();
        let parsed = perl_tdd_support::must_some(FeatureProfileKind::from_str_name(s));
        assert_eq!(parsed, kind, "roundtrip failed for {s}");
    }
    Ok(())
}

#[test]
fn as_str_values() {
    assert_eq!(FeatureProfileKind::GaLock.as_str(), "ga-lock");
    assert_eq!(FeatureProfileKind::Production.as_str(), "production");
    assert_eq!(FeatureProfileKind::All.as_str(), "all");
}

// ---------------------------------------------------------------------------
// FeatureProfileKind — all
// ---------------------------------------------------------------------------

#[test]
fn all_contains_exactly_three_variants() {
    let all = FeatureProfileKind::all();
    assert_eq!(all.len(), 3);
    assert!(all.contains(&FeatureProfileKind::GaLock));
    assert!(all.contains(&FeatureProfileKind::Production));
    assert!(all.contains(&FeatureProfileKind::All));
}

// ---------------------------------------------------------------------------
// FeatureProfileKind — aliases
// ---------------------------------------------------------------------------

#[test]
fn aliases_ga_lock_contains_expected() {
    let aliases = FeatureProfileKind::GaLock.aliases();
    assert!(aliases.contains(&"ga-lock"));
    assert!(aliases.contains(&"ga"));
    assert!(aliases.contains(&"ga_lock"));
}

#[test]
fn aliases_production_contains_expected() {
    let aliases = FeatureProfileKind::Production.aliases();
    assert!(aliases.contains(&"production"));
    assert!(aliases.contains(&"prod"));
}

#[test]
fn aliases_all_contains_expected() {
    let aliases = FeatureProfileKind::All.aliases();
    assert!(aliases.contains(&"all"));
}

#[test]
fn all_aliases_parse_back_to_their_profile() -> Result<(), Box<dyn std::error::Error>> {
    for &kind in FeatureProfileKind::all() {
        for alias in kind.aliases() {
            let parsed = perl_tdd_support::must_some(FeatureProfileKind::from_str_name(alias));
            assert_eq!(parsed, kind, "alias '{alias}' did not map to {kind:?}");
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// FeatureProfileKind — supported_cli_profiles
// ---------------------------------------------------------------------------

#[test]
fn supported_cli_profiles_includes_auto() {
    let cli = FeatureProfileKind::supported_cli_profiles();
    assert!(cli.contains(&"auto"));
}

#[test]
fn supported_cli_profiles_all_parseable() -> Result<(), Box<dyn std::error::Error>> {
    for &token in FeatureProfileKind::supported_cli_profiles() {
        let _kind = perl_tdd_support::must_some(FeatureProfileKind::from_str_name(token));
    }
    Ok(())
}

#[test]
fn supported_cli_profiles_not_empty() {
    assert!(!FeatureProfileKind::supported_cli_profiles().is_empty());
}

// ---------------------------------------------------------------------------
// FeatureProfileKind — equality and clone
// ---------------------------------------------------------------------------

#[test]
fn profile_kind_eq_and_clone() {
    let a = FeatureProfileKind::GaLock;
    let b = a;
    assert_eq!(a, b);
}

#[test]
fn profile_kind_debug_format() {
    let dbg = format!("{:?}", FeatureProfileKind::Production);
    assert!(dbg.contains("Production"));
}

// ---------------------------------------------------------------------------
// FeatureProfileSpec — static table
// ---------------------------------------------------------------------------

#[test]
fn feature_profile_specs_len() {
    assert_eq!(FEATURE_PROFILE_SPECS.len(), 3);
}

#[test]
fn feature_profile_specs_fn_matches_const() {
    let from_fn = feature_profile_specs();
    assert_eq!(from_fn.len(), FEATURE_PROFILE_SPECS.len());
    for (a, b) in from_fn.iter().zip(FEATURE_PROFILE_SPECS.iter()) {
        assert_eq!(a.canonical, b.canonical);
        assert_eq!(a.description, b.description);
    }
}

#[test]
fn feature_profile_specs_canonical_names() {
    let canonicals: Vec<&str> = FEATURE_PROFILE_SPECS.iter().map(|s| s.canonical).collect();
    assert!(canonicals.contains(&"ga-lock"));
    assert!(canonicals.contains(&"production"));
    assert!(canonicals.contains(&"all"));
}

#[test]
fn feature_profile_specs_descriptions_non_empty() {
    for spec in FEATURE_PROFILE_SPECS {
        assert!(!spec.description.is_empty(), "description empty for {}", spec.canonical);
    }
}

#[test]
fn feature_profile_specs_aliases_non_empty() {
    for spec in FEATURE_PROFILE_SPECS {
        assert!(!spec.aliases.is_empty(), "aliases empty for {}", spec.canonical);
    }
}

#[test]
fn feature_profile_spec_debug_and_clone() {
    let spec = &FEATURE_PROFILE_SPECS[0];
    let cloned = *spec;
    let dbg = format!("{:?}", cloned);
    assert!(!dbg.is_empty());
}

#[test]
fn feature_profile_spec_serialize() -> Result<(), Box<dyn std::error::Error>> {
    // FeatureProfileSpec derives Serialize; verify it round-trips to JSON.
    for spec in FEATURE_PROFILE_SPECS {
        let _json = serde_json::to_string(spec)?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// catalog re-exports — VERSION, LSP_VERSION, compliance_percent
// ---------------------------------------------------------------------------

#[test]
fn catalog_version_non_empty() {
    assert!(!catalog::VERSION.is_empty());
}

#[test]
fn catalog_lsp_version_non_empty() {
    assert!(!catalog::LSP_VERSION.is_empty());
}

#[test]
fn catalog_compliance_percent_in_range() {
    let pct = catalog::compliance_percent();
    assert!((0.0..=100.0).contains(&pct));
}

// ---------------------------------------------------------------------------
// catalog — has_feature / advertised_features
// ---------------------------------------------------------------------------

#[test]
fn has_feature_true_for_known_feature() {
    // lsp.completion should always be advertised
    assert!(catalog::has_feature("lsp.completion"));
}

#[test]
fn has_feature_false_for_unknown() {
    assert!(!catalog::has_feature("lsp.nonexistent_feature_xyz"));
}

#[test]
fn advertised_features_non_empty() {
    assert!(!catalog::advertised_features().is_empty());
}

#[test]
fn advertised_features_sorted() {
    let feats = catalog::advertised_features();
    let mut sorted = feats.to_vec();
    sorted.sort();
    assert_eq!(feats, &sorted[..]);
}

// ---------------------------------------------------------------------------
// all_features
// ---------------------------------------------------------------------------

#[test]
fn all_features_non_empty() {
    assert!(!all_features().is_empty());
}

#[test]
fn all_features_ids_unique() {
    let ids: Vec<&str> = all_features().iter().map(|f| f.id).collect();
    let mut deduped = ids.clone();
    deduped.sort();
    deduped.dedup();
    assert_eq!(ids.len(), deduped.len(), "duplicate feature IDs found");
}

#[test]
fn all_features_have_required_fields() {
    for f in all_features() {
        assert!(!f.id.is_empty(), "feature has empty id");
        assert!(!f.spec.is_empty(), "feature {} has empty spec", f.id);
        assert!(!f.area.is_empty(), "feature {} has empty area", f.id);
        assert!(!f.maturity.is_empty(), "feature {} has empty maturity", f.id);
        assert!(!f.description.is_empty(), "feature {} has empty description", f.id);
    }
}

#[test]
fn all_features_maturity_valid_values() {
    let valid = ["experimental", "preview", "ga", "planned", "production"];
    for f in all_features() {
        assert!(
            valid.contains(&f.maturity),
            "feature {} has unexpected maturity '{}'",
            f.id,
            f.maturity
        );
    }
}

// ---------------------------------------------------------------------------
// bdd_feature_rows
// ---------------------------------------------------------------------------

#[test]
fn bdd_feature_rows_non_empty() {
    assert!(!bdd_feature_rows().is_empty());
}

#[test]
fn bdd_feature_rows_sorted_by_area_then_id() {
    let rows = bdd_feature_rows();
    for pair in rows.windows(2) {
        let ordering = pair[0].area.cmp(pair[1].area).then(pair[0].id.cmp(pair[1].id));
        assert!(
            ordering.is_le(),
            "BDD rows not sorted: {} {} vs {} {}",
            pair[0].area,
            pair[0].id,
            pair[1].area,
            pair[1].id
        );
    }
}

#[test]
fn bdd_feature_rows_count_matches_all_features() {
    assert_eq!(bdd_feature_rows().len(), all_features().len());
}

#[test]
fn bdd_feature_row_fields_populated() {
    for row in bdd_feature_rows() {
        assert!(!row.id.is_empty());
        assert!(!row.spec.is_empty());
        assert!(!row.area.is_empty());
        assert!(!row.maturity.is_empty());
        assert!(!row.description.is_empty());
    }
}

#[test]
fn bdd_feature_row_debug_and_clone() {
    let rows = bdd_feature_rows();
    if let Some(row) = rows.first() {
        let cloned = row.clone();
        let dbg = format!("{:?}", cloned);
        assert!(!dbg.is_empty());
    }
}

#[test]
fn bdd_feature_row_serialize() -> Result<(), Box<dyn std::error::Error>> {
    let rows = bdd_feature_rows();
    if let Some(row) = rows.first() {
        let json = serde_json::to_string(row)?;
        assert!(json.contains(row.id));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// trackable_feature_count_for_grid
// ---------------------------------------------------------------------------

#[test]
fn trackable_feature_count_positive() {
    assert!(trackable_feature_count_for_grid() > 0);
}

#[test]
fn trackable_excludes_planned() {
    let planned_count =
        all_features().iter().filter(|f| f.maturity == "planned" && f.counts_in_coverage).count();
    let total_countable = all_features().iter().filter(|f| f.counts_in_coverage).count();
    assert_eq!(trackable_feature_count_for_grid(), total_countable - planned_count);
}

// ---------------------------------------------------------------------------
// advertised_trackable_feature_count_for_grid
// ---------------------------------------------------------------------------

#[test]
fn advertised_trackable_lte_trackable() {
    assert!(advertised_trackable_feature_count_for_grid() <= trackable_feature_count_for_grid());
}

#[test]
fn advertised_trackable_positive() {
    assert!(advertised_trackable_feature_count_for_grid() > 0);
}

#[test]
fn advertised_trackable_matches_manual_count() {
    let expected = all_features()
        .iter()
        .filter(|f| f.maturity != "planned" && f.counts_in_coverage && f.advertised)
        .count();
    assert_eq!(advertised_trackable_feature_count_for_grid(), expected);
}

// ---------------------------------------------------------------------------
// compliance_percent_for_grid
// ---------------------------------------------------------------------------

#[test]
fn compliance_percent_for_grid_in_range() {
    let pct = compliance_percent_for_grid();
    assert!((0.0..=100.0).contains(&pct));
}

#[test]
fn compliance_percent_for_grid_matches_manual_calculation() {
    let trackable = trackable_feature_count_for_grid();
    let advertised = advertised_trackable_feature_count_for_grid();
    let expected = if trackable == 0 {
        0.0
    } else {
        (advertised as f64 / trackable as f64 * 100.0).round() as f32
    };
    let actual = compliance_percent_for_grid();
    assert!((actual - expected).abs() < f32::EPSILON, "expected {expected}, got {actual}");
}

// ---------------------------------------------------------------------------
// Re-exported capability map functions
// ---------------------------------------------------------------------------

#[test]
fn caps_from_feature_ids_empty_input() {
    use perl_lsp_feature_contracts::caps_from_feature_ids;
    let caps = caps_from_feature_ids(&[]);
    // With no features, no providers should be set
    assert!(caps.completion_provider.is_none());
    assert!(caps.hover_provider.is_none());
}

#[test]
fn feature_ids_from_caps_empty_caps() {
    use perl_lsp_feature_contracts::feature_ids_from_caps;
    let caps = lsp_types::ServerCapabilities::default();
    let ids = feature_ids_from_caps(&caps);
    assert!(ids.is_empty());
}

#[test]
fn caps_roundtrip_completion() {
    use perl_lsp_feature_contracts::{caps_from_feature_ids, feature_ids_from_caps};
    let caps = caps_from_feature_ids(&["lsp.completion"]);
    let ids = feature_ids_from_caps(&caps);
    assert!(ids.contains(&"lsp.completion"));
}

#[test]
fn caps_roundtrip_hover() {
    use perl_lsp_feature_contracts::{caps_from_feature_ids, feature_ids_from_caps};
    let caps = caps_from_feature_ids(&["lsp.hover"]);
    let ids = feature_ids_from_caps(&caps);
    assert!(ids.contains(&"lsp.hover"));
}

#[test]
fn caps_roundtrip_multiple_features() {
    use perl_lsp_feature_contracts::{caps_from_feature_ids, feature_ids_from_caps};
    let input = &["lsp.completion", "lsp.hover", "lsp.definition", "lsp.references"];
    let caps = caps_from_feature_ids(input);
    let ids = feature_ids_from_caps(&caps);
    for &feat in input {
        assert!(ids.contains(&feat), "missing feature {feat} after roundtrip");
    }
}
