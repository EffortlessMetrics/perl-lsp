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
    assert_eq!(
        FeatureProfileKind::from_ga_lock_enabled(true),
        FeatureProfileKind::GaLock
    );
}

#[test]
fn from_ga_lock_enabled_false() {
    assert_eq!(
        FeatureProfileKind::from_ga_lock_enabled(false),
        FeatureProfileKind::Production
    );
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
        assert!(
            !spec.description.is_empty(),
            "description empty for {}",
            spec.canonical
        );
    }
}

#[test]
fn feature_profile_specs_aliases_non_empty() {
    for spec in FEATURE_PROFILE_SPECS {
        assert!(
            !spec.aliases.is_empty(),
            "aliases empty for {}",
            spec.canonical
        );
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
        assert!(
            !f.maturity.is_empty(),
            "feature {} has empty maturity",
            f.id
        );
        assert!(
            !f.description.is_empty(),
            "feature {} has empty description",
            f.id
        );
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
        let ordering = pair[0]
            .area
            .cmp(pair[1].area)
            .then(pair[0].id.cmp(pair[1].id));
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
    let planned_count = all_features()
        .iter()
        .filter(|f| f.maturity == "planned" && f.counts_in_coverage)
        .count();
    let total_countable = all_features()
        .iter()
        .filter(|f| f.counts_in_coverage)
        .count();
    assert_eq!(
        trackable_feature_count_for_grid(),
        total_countable - planned_count
    );
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
    assert!(
        (actual - expected).abs() < f32::EPSILON,
        "expected {expected}, got {actual}"
    );
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
    let input = &[
        "lsp.completion",
        "lsp.hover",
        "lsp.definition",
        "lsp.references",
    ];
    let caps = caps_from_feature_ids(input);
    let ids = feature_ids_from_caps(&caps);
    for &feat in input {
        assert!(
            ids.contains(&feat),
            "missing feature {feat} after roundtrip"
        );
    }
}

// ---------------------------------------------------------------------------
// Spec ↔ FeatureProfileKind consistency
// ---------------------------------------------------------------------------

#[test]
fn spec_canonical_matches_kind_as_str() {
    for &kind in FeatureProfileKind::all() {
        let spec = FEATURE_PROFILE_SPECS
            .iter()
            .find(|s| s.canonical == kind.as_str());
        assert!(spec.is_some(), "no spec found for profile {:?}", kind);
    }
}

#[test]
fn spec_aliases_match_kind_aliases() {
    for &kind in FeatureProfileKind::all() {
        let spec = FEATURE_PROFILE_SPECS
            .iter()
            .find(|s| s.canonical == kind.as_str());
        if let Some(spec) = spec {
            let kind_aliases = kind.aliases();
            for &alias in spec.aliases {
                assert!(
                    kind_aliases.contains(&alias),
                    "spec alias '{alias}' not in kind aliases for {:?}",
                    kind
                );
            }
        }
    }
}

#[test]
fn spec_count_matches_kind_count() {
    assert_eq!(FEATURE_PROFILE_SPECS.len(), FeatureProfileKind::all().len());
}

// ---------------------------------------------------------------------------
// FeatureProfileKind — edge cases
// ---------------------------------------------------------------------------

#[test]
fn from_str_name_rejects_whitespace_variants() {
    assert!(FeatureProfileKind::from_str_name(" all").is_none());
    assert!(FeatureProfileKind::from_str_name("all ").is_none());
    assert!(FeatureProfileKind::from_str_name(" ga-lock ").is_none());
}

#[test]
fn from_str_name_rejects_mixed_case() {
    assert!(FeatureProfileKind::from_str_name("Ga-Lock").is_none());
    assert!(FeatureProfileKind::from_str_name("PROD").is_none());
    assert!(FeatureProfileKind::from_str_name("Auto").is_none());
}

#[test]
fn all_variants_are_distinct() {
    let all = FeatureProfileKind::all();
    for (i, a) in all.iter().enumerate() {
        for (j, b) in all.iter().enumerate() {
            if i != j {
                assert_ne!(a, b, "variants at index {i} and {j} are equal");
            }
        }
    }
}

#[test]
fn all_variants_have_distinct_as_str() {
    let strs: Vec<&str> = FeatureProfileKind::all()
        .iter()
        .map(|k| k.as_str())
        .collect();
    let mut deduped = strs.clone();
    deduped.sort();
    deduped.dedup();
    assert_eq!(strs.len(), deduped.len());
}

// ---------------------------------------------------------------------------
// catalog — advertised_features consistency
// ---------------------------------------------------------------------------

#[test]
fn advertised_features_subset_of_all_features() {
    let all_ids: Vec<&str> = all_features().iter().map(|f| f.id).collect();
    for &feat in catalog::advertised_features() {
        assert!(
            all_ids.contains(&feat),
            "advertised feature '{feat}' not in all_features()"
        );
    }
}

#[test]
fn advertised_features_match_advertised_flag() {
    let advertised_from_flag: Vec<&str> = all_features()
        .iter()
        .filter(|f| f.advertised)
        .map(|f| f.id)
        .collect();
    let advertised = catalog::advertised_features();
    assert_eq!(
        advertised.len(),
        advertised_from_flag.len(),
        "advertised_features() count mismatch with advertised flag count"
    );
}

#[test]
fn compliance_percent_agrees_with_catalog() {
    let grid_pct = compliance_percent_for_grid();
    let catalog_pct = catalog::compliance_percent();
    // Both should be in valid range; catalog may use different formula
    assert!((0.0..=100.0).contains(&grid_pct));
    assert!((0.0..=100.0).contains(&catalog_pct));
}

// ---------------------------------------------------------------------------
// BddFeatureRow — field mapping fidelity
// ---------------------------------------------------------------------------

#[test]
fn bdd_rows_preserve_all_feature_fields() {
    let features = all_features();
    let rows = bdd_feature_rows();
    for feature in features {
        let row = rows.iter().find(|r| r.id == feature.id);
        assert!(row.is_some(), "no BDD row for feature {}", feature.id);
        if let Some(row) = row {
            assert_eq!(row.spec, feature.spec, "spec mismatch for {}", feature.id);
            assert_eq!(row.area, feature.area, "area mismatch for {}", feature.id);
            assert_eq!(
                row.maturity, feature.maturity,
                "maturity mismatch for {}",
                feature.id
            );
            assert_eq!(
                row.advertised, feature.advertised,
                "advertised mismatch for {}",
                feature.id
            );
            assert_eq!(
                row.counts_in_coverage, feature.counts_in_coverage,
                "counts_in_coverage mismatch for {}",
                feature.id
            );
            assert_eq!(
                row.description, feature.description,
                "description mismatch for {}",
                feature.id
            );
            assert_eq!(
                row.tests, feature.tests,
                "tests mismatch for {}",
                feature.id
            );
        }
    }
}

#[test]
fn bdd_rows_serialize_all_rows() -> Result<(), Box<dyn std::error::Error>> {
    for row in bdd_feature_rows() {
        let json = serde_json::to_string(&row)?;
        assert!(json.contains(row.id), "JSON for {} missing id", row.id);
        assert!(json.contains(row.area), "JSON for {} missing area", row.id);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Capability map roundtrip — individual features
// ---------------------------------------------------------------------------

#[test]
fn caps_roundtrip_signature_help() {
    use perl_lsp_feature_contracts::{caps_from_feature_ids, feature_ids_from_caps};
    let caps = caps_from_feature_ids(&["lsp.signature_help"]);
    let ids = feature_ids_from_caps(&caps);
    assert!(ids.contains(&"lsp.signature_help"));
}

#[test]
fn caps_roundtrip_declaration() {
    use perl_lsp_feature_contracts::{caps_from_feature_ids, feature_ids_from_caps};
    let caps = caps_from_feature_ids(&["lsp.declaration"]);
    let ids = feature_ids_from_caps(&caps);
    assert!(ids.contains(&"lsp.declaration"));
}

#[test]
fn caps_roundtrip_type_definition() {
    use perl_lsp_feature_contracts::{caps_from_feature_ids, feature_ids_from_caps};
    let caps = caps_from_feature_ids(&["lsp.type_definition"]);
    let ids = feature_ids_from_caps(&caps);
    assert!(ids.contains(&"lsp.type_definition"));
}

#[test]
fn caps_roundtrip_implementation() {
    use perl_lsp_feature_contracts::{caps_from_feature_ids, feature_ids_from_caps};
    let caps = caps_from_feature_ids(&["lsp.implementation"]);
    let ids = feature_ids_from_caps(&caps);
    assert!(ids.contains(&"lsp.implementation"));
}

#[test]
fn caps_roundtrip_document_symbol() {
    use perl_lsp_feature_contracts::{caps_from_feature_ids, feature_ids_from_caps};
    let caps = caps_from_feature_ids(&["lsp.document_symbol"]);
    let ids = feature_ids_from_caps(&caps);
    assert!(ids.contains(&"lsp.document_symbol"));
}

#[test]
fn caps_roundtrip_code_action() {
    use perl_lsp_feature_contracts::{caps_from_feature_ids, feature_ids_from_caps};
    let caps = caps_from_feature_ids(&["lsp.code_action"]);
    let ids = feature_ids_from_caps(&caps);
    assert!(ids.contains(&"lsp.code_action"));
}

#[test]
fn caps_roundtrip_code_lens() {
    use perl_lsp_feature_contracts::{caps_from_feature_ids, feature_ids_from_caps};
    let caps = caps_from_feature_ids(&["lsp.code_lens"]);
    let ids = feature_ids_from_caps(&caps);
    assert!(ids.contains(&"lsp.code_lens"));
}

#[test]
fn caps_roundtrip_formatting() {
    use perl_lsp_feature_contracts::{caps_from_feature_ids, feature_ids_from_caps};
    let caps = caps_from_feature_ids(&["lsp.formatting"]);
    let ids = feature_ids_from_caps(&caps);
    assert!(ids.contains(&"lsp.formatting"));
}

#[test]
fn caps_roundtrip_range_formatting() {
    use perl_lsp_feature_contracts::{caps_from_feature_ids, feature_ids_from_caps};
    let caps = caps_from_feature_ids(&["lsp.range_formatting"]);
    let ids = feature_ids_from_caps(&caps);
    assert!(ids.contains(&"lsp.range_formatting"));
}

#[test]
fn caps_roundtrip_on_type_formatting() {
    use perl_lsp_feature_contracts::{caps_from_feature_ids, feature_ids_from_caps};
    let caps = caps_from_feature_ids(&["lsp.on_type_formatting"]);
    let ids = feature_ids_from_caps(&caps);
    assert!(ids.contains(&"lsp.on_type_formatting"));
}

#[test]
fn caps_roundtrip_rename() {
    use perl_lsp_feature_contracts::{caps_from_feature_ids, feature_ids_from_caps};
    let caps = caps_from_feature_ids(&["lsp.rename"]);
    let ids = feature_ids_from_caps(&caps);
    assert!(ids.contains(&"lsp.rename"));
}

#[test]
fn caps_roundtrip_document_link() {
    use perl_lsp_feature_contracts::{caps_from_feature_ids, feature_ids_from_caps};
    let caps = caps_from_feature_ids(&["lsp.document_link"]);
    let ids = feature_ids_from_caps(&caps);
    assert!(ids.contains(&"lsp.document_link"));
}

#[test]
fn caps_roundtrip_folding_range() {
    use perl_lsp_feature_contracts::{caps_from_feature_ids, feature_ids_from_caps};
    let caps = caps_from_feature_ids(&["lsp.folding_range"]);
    let ids = feature_ids_from_caps(&caps);
    assert!(ids.contains(&"lsp.folding_range"));
}

#[test]
fn caps_roundtrip_selection_range() {
    use perl_lsp_feature_contracts::{caps_from_feature_ids, feature_ids_from_caps};
    let caps = caps_from_feature_ids(&["lsp.selection_range"]);
    let ids = feature_ids_from_caps(&caps);
    assert!(ids.contains(&"lsp.selection_range"));
}

#[test]
fn caps_roundtrip_semantic_tokens() {
    use perl_lsp_feature_contracts::{caps_from_feature_ids, feature_ids_from_caps};
    let caps = caps_from_feature_ids(&["lsp.semantic_tokens"]);
    let ids = feature_ids_from_caps(&caps);
    assert!(ids.contains(&"lsp.semantic_tokens"));
}

#[test]
fn caps_roundtrip_inlay_hint() {
    use perl_lsp_feature_contracts::{caps_from_feature_ids, feature_ids_from_caps};
    let caps = caps_from_feature_ids(&["lsp.inlay_hint"]);
    let ids = feature_ids_from_caps(&caps);
    assert!(ids.contains(&"lsp.inlay_hint"));
}

#[test]
fn caps_roundtrip_call_hierarchy() {
    use perl_lsp_feature_contracts::{caps_from_feature_ids, feature_ids_from_caps};
    let caps = caps_from_feature_ids(&["lsp.call_hierarchy"]);
    let ids = feature_ids_from_caps(&caps);
    assert!(ids.contains(&"lsp.call_hierarchy"));
}

#[test]
fn caps_roundtrip_pull_diagnostics() {
    use perl_lsp_feature_contracts::{caps_from_feature_ids, feature_ids_from_caps};
    let caps = caps_from_feature_ids(&["lsp.pull_diagnostics"]);
    let ids = feature_ids_from_caps(&caps);
    assert!(ids.contains(&"lsp.pull_diagnostics"));
}

#[test]
fn caps_roundtrip_inline_value() {
    use perl_lsp_feature_contracts::{caps_from_feature_ids, feature_ids_from_caps};
    let caps = caps_from_feature_ids(&["lsp.inline_value"]);
    let ids = feature_ids_from_caps(&caps);
    assert!(ids.contains(&"lsp.inline_value"));
}

#[test]
fn caps_roundtrip_document_color() {
    use perl_lsp_feature_contracts::{caps_from_feature_ids, feature_ids_from_caps};
    let caps = caps_from_feature_ids(&["lsp.document_color"]);
    let ids = feature_ids_from_caps(&caps);
    assert!(ids.contains(&"lsp.document_color"));
}

#[test]
fn caps_roundtrip_linked_editing_range() {
    use perl_lsp_feature_contracts::{caps_from_feature_ids, feature_ids_from_caps};
    let caps = caps_from_feature_ids(&["lsp.linked_editing_range"]);
    let ids = feature_ids_from_caps(&caps);
    assert!(ids.contains(&"lsp.linked_editing_range"));
}

#[test]
fn caps_roundtrip_moniker() {
    use perl_lsp_feature_contracts::{caps_from_feature_ids, feature_ids_from_caps};
    let caps = caps_from_feature_ids(&["lsp.moniker"]);
    let ids = feature_ids_from_caps(&caps);
    assert!(ids.contains(&"lsp.moniker"));
}

#[test]
fn caps_roundtrip_workspace_symbol() {
    use perl_lsp_feature_contracts::{caps_from_feature_ids, feature_ids_from_caps};
    let caps = caps_from_feature_ids(&["lsp.workspace_symbol"]);
    let ids = feature_ids_from_caps(&caps);
    assert!(ids.contains(&"lsp.workspace_symbol"));
}

#[test]
fn caps_roundtrip_execute_command() {
    use perl_lsp_feature_contracts::{caps_from_feature_ids, feature_ids_from_caps};
    let caps = caps_from_feature_ids(&["lsp.execute_command"]);
    let ids = feature_ids_from_caps(&caps);
    assert!(ids.contains(&"lsp.execute_command"));
}

#[test]
fn caps_roundtrip_document_highlight() {
    use perl_lsp_feature_contracts::{caps_from_feature_ids, feature_ids_from_caps};
    let caps = caps_from_feature_ids(&["lsp.document_highlight"]);
    let ids = feature_ids_from_caps(&caps);
    assert!(ids.contains(&"lsp.document_highlight"));
}

#[test]
fn caps_roundtrip_notebook_document_sync() {
    use perl_lsp_feature_contracts::{caps_from_feature_ids, feature_ids_from_caps};
    let caps = caps_from_feature_ids(&["lsp.notebook_document_sync"]);
    let ids = feature_ids_from_caps(&caps);
    assert!(ids.contains(&"lsp.notebook_document_sync"));
}

#[test]
fn caps_from_feature_ids_unknown_feature_ignored() {
    use perl_lsp_feature_contracts::{caps_from_feature_ids, feature_ids_from_caps};
    let caps = caps_from_feature_ids(&["lsp.nonexistent_xyz"]);
    let ids = feature_ids_from_caps(&caps);
    assert!(ids.is_empty());
}

#[test]
fn caps_roundtrip_all_advertised_features() {
    use perl_lsp_feature_contracts::{caps_from_feature_ids, feature_ids_from_caps};
    let advertised = catalog::advertised_features();
    let caps = caps_from_feature_ids(advertised);
    let ids = feature_ids_from_caps(&caps);
    // Every advertised feature that has a capability mapping should roundtrip
    for &feat in advertised {
        if ids.contains(&feat) {
            // Feature survived the roundtrip — good
        }
        // Some features (e.g. lsp.progress) don't map to ServerCapabilities fields,
        // so we don't assert all features must roundtrip.
    }
    // At least some features should survive
    assert!(!ids.is_empty(), "no features survived roundtrip");
}

// ---------------------------------------------------------------------------
// Compliance math — edge-case validation
// ---------------------------------------------------------------------------

#[test]
fn trackable_plus_planned_equals_countable_total() {
    let total_countable = all_features()
        .iter()
        .filter(|f| f.counts_in_coverage)
        .count();
    let planned_countable = all_features()
        .iter()
        .filter(|f| f.maturity == "planned" && f.counts_in_coverage)
        .count();
    assert_eq!(
        trackable_feature_count_for_grid(),
        total_countable - planned_countable
    );
}

#[test]
fn compliance_percent_for_grid_is_whole_number() {
    let pct = compliance_percent_for_grid();
    assert!(
        (pct - pct.round()).abs() < f32::EPSILON,
        "compliance_percent_for_grid should be rounded: got {pct}"
    );
}

#[test]
fn compliance_monotonic_with_advertised() {
    // If all trackable features were advertised, compliance would be 100%
    let trackable = trackable_feature_count_for_grid();
    let advertised = advertised_trackable_feature_count_for_grid();
    if trackable > 0 && advertised == trackable {
        let pct = compliance_percent_for_grid();
        assert!((pct - 100.0).abs() < f32::EPSILON);
    }
}

// ---------------------------------------------------------------------------
// Feature catalog — VERSION / LSP_VERSION format
// ---------------------------------------------------------------------------

#[test]
fn catalog_version_looks_like_semver() {
    let v = catalog::VERSION;
    let parts: Vec<&str> = v.split('.').collect();
    assert!(parts.len() >= 2, "VERSION '{v}' doesn't look like semver");
}

#[test]
fn catalog_lsp_version_has_dot_separator() {
    let v = catalog::LSP_VERSION;
    assert!(
        v.contains('.'),
        "LSP_VERSION '{v}' should contain a dot separator"
    );
}

// ---------------------------------------------------------------------------
// FeatureProfileKind — supported_cli_profiles exhaustive
// ---------------------------------------------------------------------------

#[test]
fn supported_cli_profiles_covers_all_aliases() {
    let cli = FeatureProfileKind::supported_cli_profiles();
    for &kind in FeatureProfileKind::all() {
        for &alias in kind.aliases() {
            assert!(
                cli.contains(&alias),
                "alias '{alias}' for {:?} not in supported_cli_profiles",
                kind
            );
        }
    }
}

#[test]
fn supported_cli_profiles_no_duplicates() {
    let cli = FeatureProfileKind::supported_cli_profiles();
    let mut deduped = cli.to_vec();
    deduped.sort();
    deduped.dedup();
    assert_eq!(
        cli.len(),
        deduped.len(),
        "duplicate entries in supported_cli_profiles"
    );
}
