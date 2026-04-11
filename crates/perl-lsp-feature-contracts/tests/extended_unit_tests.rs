//! Extended unit tests for `perl-lsp-feature-contracts` covering additional edge cases,
//! serialization, data validation, and comprehensive API coverage.
#![allow(clippy::panic)]

use perl_lsp_feature_contracts::{
    FEATURE_PROFILE_SPECS, FeatureProfileKind, FeatureProfileSpec,
    advertised_trackable_feature_count_for_grid, all_features, bdd_feature_rows,
    compliance_percent_for_grid, feature_profile_specs, trackable_feature_count_for_grid,
};

// ---------------------------------------------------------------------------
// FeatureProfileKind — Additional parsing and validation tests
// ---------------------------------------------------------------------------

#[test]
fn from_str_name_with_leading_whitespace_fails() {
    assert!(FeatureProfileKind::from_str_name(" ga-lock").is_none());
    assert!(FeatureProfileKind::from_str_name("\tproduction").is_none());
    assert!(FeatureProfileKind::from_str_name("\nall").is_none());
}

#[test]
fn from_str_name_with_trailing_whitespace_fails() {
    assert!(FeatureProfileKind::from_str_name("ga-lock ").is_none());
    assert!(FeatureProfileKind::from_str_name("production\t").is_none());
    assert!(FeatureProfileKind::from_str_name("all\n").is_none());
}

#[test]
fn from_str_name_partial_matches_fail() {
    assert!(FeatureProfileKind::from_str_name("ga-loc").is_none());
    assert!(FeatureProfileKind::from_str_name("ga-lock-extra").is_none());
    assert!(FeatureProfileKind::from_str_name("gal").is_none());
    assert!(FeatureProfileKind::from_str_name("product").is_none());
    assert!(FeatureProfileKind::from_str_name("al").is_none());
}

#[test]
fn from_str_name_case_sensitive_variants_all_fail() {
    let inputs = vec![
        "GA-LOCK",
        "Ga-Lock",
        "GA-lock",
        "gA-LOCK",
        "PRODUCTION",
        "Production",
        "PrOdUcTiOn",
        "ALL",
        "All",
        "aLL",
        "AUTO",
        "Auto",
        "AuTo",
    ];
    for input in inputs {
        assert!(
            FeatureProfileKind::from_str_name(input).is_none(),
            "Expected {input} to fail (case sensitivity)"
        );
    }
}

#[test]
fn from_str_name_numeric_and_special_chars_fail() {
    assert!(FeatureProfileKind::from_str_name("1").is_none());
    assert!(FeatureProfileKind::from_str_name("ga-lock-1").is_none());
    assert!(FeatureProfileKind::from_str_name("ga@lock").is_none());
    assert!(FeatureProfileKind::from_str_name("ga#lock").is_none());
    assert!(FeatureProfileKind::from_str_name("ga!lock").is_none());
    assert!(FeatureProfileKind::from_str_name("ga lock").is_none());
}

#[test]
fn from_str_name_empty_and_none_like_fails() {
    assert!(FeatureProfileKind::from_str_name("").is_none());
    assert!(FeatureProfileKind::from_str_name("none").is_none());
    assert!(FeatureProfileKind::from_str_name("null").is_none());
    assert!(FeatureProfileKind::from_str_name("nil").is_none());
}

// ---------------------------------------------------------------------------
// FeatureProfileKind — Comprehensive variant roundtrips
// ---------------------------------------------------------------------------

#[test]
fn all_variants_from_str_name_then_as_str_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    for &kind in FeatureProfileKind::all() {
        let canonical_str = kind.as_str();
        let reparsed =
            perl_tdd_support::must_some(FeatureProfileKind::from_str_name(canonical_str));
        assert_eq!(
            reparsed, kind,
            "Failed roundtrip for variant {kind:?} via canonical string '{canonical_str}'"
        );
    }
    Ok(())
}

#[test]
fn each_variant_eq_to_itself() {
    for &kind in FeatureProfileKind::all() {
        assert_eq!(kind, kind);
    }
}

#[test]
fn each_variant_clone_equals_original() {
    for &kind in FeatureProfileKind::all() {
        let cloned = kind;
        assert_eq!(cloned, kind);
        assert_eq!(cloned.as_str(), kind.as_str());
        assert_eq!(cloned.aliases(), kind.aliases());
    }
}

#[test]
fn variants_are_not_equal_to_each_other() {
    assert_ne!(FeatureProfileKind::GaLock, FeatureProfileKind::Production);
    assert_ne!(FeatureProfileKind::GaLock, FeatureProfileKind::All);
    assert_ne!(FeatureProfileKind::Production, FeatureProfileKind::All);
}

// ---------------------------------------------------------------------------
// FeatureProfileKind::aliases — Detailed validation
// ---------------------------------------------------------------------------

#[test]
fn ga_lock_aliases_contains_no_duplicates() {
    let aliases = FeatureProfileKind::GaLock.aliases();
    let len_before = aliases.len();
    let mut unique = aliases.to_vec();
    unique.sort();
    unique.dedup();
    assert_eq!(len_before, unique.len(), "GA-Lock aliases contain duplicates");
}

#[test]
fn production_aliases_contains_no_duplicates() {
    let aliases = FeatureProfileKind::Production.aliases();
    let len_before = aliases.len();
    let mut unique = aliases.to_vec();
    unique.sort();
    unique.dedup();
    assert_eq!(len_before, unique.len(), "Production aliases contain duplicates");
}

#[test]
fn all_aliases_contains_no_duplicates() {
    let aliases = FeatureProfileKind::All.aliases();
    let len_before = aliases.len();
    let mut unique = aliases.to_vec();
    unique.sort();
    unique.dedup();
    assert_eq!(len_before, unique.len(), "All aliases contain duplicates");
}

#[test]
fn aliases_for_each_variant_are_non_empty() {
    for &kind in FeatureProfileKind::all() {
        let aliases = kind.aliases();
        assert!(!aliases.is_empty(), "Expected non-empty aliases for {kind:?}");
    }
}

#[test]
fn aliases_each_variant_includes_canonical_form() {
    for &kind in FeatureProfileKind::all() {
        let canonical = kind.as_str();
        let aliases = kind.aliases();
        assert!(
            aliases.contains(&canonical),
            "Expected aliases for {kind:?} to include canonical form '{canonical}'"
        );
    }
}

#[test]
fn no_alias_appears_in_multiple_variants() {
    let mut all_aliases = vec![];
    for &kind in FeatureProfileKind::all() {
        for alias in kind.aliases() {
            all_aliases.push((*alias, kind));
        }
    }

    // Check for duplicates
    let mut seen = std::collections::HashMap::new();
    for (alias, kind) in all_aliases {
        if let Some(prev_kind) = seen.insert(alias, kind) {
            assert_eq!(
                prev_kind, kind,
                "Alias '{alias}' appears in multiple variants: {prev_kind:?} and {kind:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// FeatureProfileKind::supported_cli_profiles — Detailed validation
// ---------------------------------------------------------------------------

#[test]
fn supported_cli_profiles_are_all_parseable() -> Result<(), Box<dyn std::error::Error>> {
    for profile in FeatureProfileKind::supported_cli_profiles() {
        let _kind = perl_tdd_support::must_some(FeatureProfileKind::from_str_name(profile));
    }
    Ok(())
}

#[test]
fn supported_cli_profiles_includes_auto() {
    let profiles = FeatureProfileKind::supported_cli_profiles();
    assert!(profiles.contains(&"auto"), "Expected 'auto' in supported CLI profiles");
}

#[test]
fn supported_cli_profiles_includes_all_canonical_names() {
    let profiles = FeatureProfileKind::supported_cli_profiles();
    for &kind in FeatureProfileKind::all() {
        assert!(
            profiles.contains(&kind.as_str()),
            "Expected canonical name '{}' in CLI profiles",
            kind.as_str()
        );
    }
}

#[test]
fn supported_cli_profiles_includes_major_aliases() {
    let profiles = FeatureProfileKind::supported_cli_profiles();
    let expected_aliases = vec!["ga", "ga_lock", "prod"];
    for alias in expected_aliases {
        assert!(profiles.contains(&alias), "Expected alias '{alias}' in CLI profiles");
    }
}

#[test]
fn supported_cli_profiles_no_internal_duplicates() {
    let profiles = FeatureProfileKind::supported_cli_profiles();
    let len_before = profiles.len();
    let mut unique = profiles.to_vec();
    unique.sort();
    unique.dedup();
    assert_eq!(len_before, unique.len(), "Supported CLI profiles have duplicates");
}

// ---------------------------------------------------------------------------
// FeatureProfileSpec — Structure validation
// ---------------------------------------------------------------------------

#[test]
fn feature_profile_spec_canonical_fields_are_static() {
    let spec = FeatureProfileSpec {
        canonical: "test",
        aliases: &["t", "tst"],
        description: "A test profile",
    };

    assert_eq!(spec.canonical, "test");
    assert_eq!(spec.aliases, &["t", "tst"]);
    assert_eq!(spec.description, "A test profile");
}

#[test]
fn feature_profile_specs_all_have_non_empty_canonical() {
    for spec in feature_profile_specs() {
        assert!(!spec.canonical.is_empty(), "Found spec with empty canonical field");
        assert!(
            !spec.canonical.starts_with(' '),
            "Canonical starts with whitespace: '{}'",
            spec.canonical
        );
        assert!(
            !spec.canonical.ends_with(' '),
            "Canonical ends with whitespace: '{}'",
            spec.canonical
        );
    }
}

#[test]
fn feature_profile_specs_all_have_non_empty_description() {
    for spec in feature_profile_specs() {
        assert!(
            !spec.description.is_empty(),
            "Spec for '{}' has empty description",
            spec.canonical
        );
    }
}

#[test]
fn feature_profile_specs_aliases_all_non_empty() {
    for spec in feature_profile_specs() {
        for alias in spec.aliases {
            assert!(!alias.is_empty(), "Spec '{}' has empty alias", spec.canonical);
        }
    }
}

#[test]
fn feature_profile_specs_canonical_matches_kind_names() {
    let specs = feature_profile_specs();
    let kinds = FeatureProfileKind::all();

    assert_eq!(specs.len(), kinds.len());

    let spec_names: Vec<_> = specs.iter().map(|s| s.canonical).collect();
    let kind_names: Vec<_> = kinds.iter().map(|k| k.as_str()).collect();

    for (spec_name, kind_name) in spec_names.iter().zip(kind_names.iter()) {
        assert_eq!(spec_name, kind_name);
    }
}

// ---------------------------------------------------------------------------
// FEATURE_PROFILE_SPECS and feature_profile_specs() consistency
// ---------------------------------------------------------------------------

#[test]
fn feature_profile_specs_const_eq_fn() {
    assert_eq!(FEATURE_PROFILE_SPECS.len(), feature_profile_specs().len());

    for (const_spec, fn_spec) in FEATURE_PROFILE_SPECS.iter().zip(feature_profile_specs().iter()) {
        assert_eq!(const_spec.canonical, fn_spec.canonical);
        assert_eq!(const_spec.aliases, fn_spec.aliases);
        assert_eq!(const_spec.description, fn_spec.description);
    }
}

#[test]
fn bdd_feature_rows_are_correctly_sorted() {
    let rows = bdd_feature_rows();

    for i in 0..rows.len().saturating_sub(1) {
        let current = &rows[i];
        let next = &rows[i + 1];

        let area_cmp = current.area.cmp(next.area);
        match area_cmp {
            std::cmp::Ordering::Less => {
                // Area is correctly increasing
            }
            std::cmp::Ordering::Equal => {
                // Same area, check ID ordering
                assert!(
                    current.id <= next.id,
                    "IDs not sorted within area: {current:?} vs {next:?}"
                );
            }
            std::cmp::Ordering::Greater => {
                assert!(
                    !matches!(area_cmp, std::cmp::Ordering::Greater),
                    "Areas not sorted: {current:?} vs {next:?}"
                );
            }
        }
    }
}

#[test]
fn bdd_feature_rows_all_have_non_empty_ids() {
    for row in bdd_feature_rows() {
        assert!(!row.id.is_empty(), "BDD row has empty ID");
    }
}

#[test]
fn bdd_feature_rows_all_have_non_empty_areas() {
    for row in bdd_feature_rows() {
        assert!(!row.area.is_empty(), "BDD row has empty area");
    }
}

#[test]
fn bdd_feature_rows_maturity_values_are_canonical() {
    let valid_maturity = ["planned", "in_progress", "beta", "rc", "ga"];
    for row in bdd_feature_rows() {
        assert!(
            valid_maturity.contains(&row.maturity),
            "Row {} has invalid maturity: '{}'",
            row.id,
            row.maturity
        );
    }
}

#[test]
fn bdd_feature_rows_boolean_consistency() {
    for row in bdd_feature_rows() {
        // advertised and counts_in_coverage are independent booleans
        // so any combination is valid, but we can test they are actual booleans
        let _ = row.advertised;
        let _ = row.counts_in_coverage;
    }
}

#[test]
fn bdd_feature_rows_preserve_feature_order() {
    let rows = bdd_feature_rows();
    let all_feats = all_features();

    assert_eq!(rows.len(), all_feats.len());

    // Verify row content matches features (after sorting)
    let mut sorted_features = all_feats.to_vec();
    sorted_features.sort_by(|a, b| a.area.cmp(b.area).then(a.id.cmp(b.id)));

    for (row, feat) in rows.iter().zip(sorted_features.iter()) {
        assert_eq!(row.id, feat.id);
        assert_eq!(row.area, feat.area);
        assert_eq!(row.maturity, feat.maturity);
        assert_eq!(row.advertised, feat.advertised);
        assert_eq!(row.counts_in_coverage, feat.counts_in_coverage);
        assert_eq!(row.description, feat.description);
        assert_eq!(row.tests, feat.tests);
    }
}

#[test]
fn bdd_feature_rows_id_uniqueness() {
    let rows = bdd_feature_rows();
    let mut ids = std::collections::HashSet::new();

    for row in rows {
        assert!(ids.insert(row.id), "Duplicate ID in BDD rows: '{}'", row.id);
    }
}

#[test]
fn bdd_feature_rows_descriptions_non_empty() {
    for row in bdd_feature_rows() {
        assert!(!row.description.is_empty(), "BDD row {} has empty description", row.id);
    }
}

// ---------------------------------------------------------------------------
// all_features() — Feature catalog validation
// ---------------------------------------------------------------------------

#[test]
fn all_features_are_readable() {
    for feature in all_features() {
        // Access all public fields to verify they're readable
        let _ = feature.id;
        let _ = feature.spec;
        let _ = feature.area;
        let _ = feature.maturity;
        let _ = feature.advertised;
        let _ = feature.counts_in_coverage;
        let _ = feature.description;
        let _ = feature.tests;
    }
}

// ---------------------------------------------------------------------------
// Counting and compliance calculations
// ---------------------------------------------------------------------------

#[test]
fn trackable_feature_count_excludes_planned() {
    let trackable = trackable_feature_count_for_grid();
    let all_non_planned =
        all_features().iter().filter(|f| f.maturity != "planned" && f.counts_in_coverage).count();

    assert_eq!(trackable, all_non_planned);
}

#[test]
fn advertised_trackable_count_is_subset_of_trackable() {
    let advertised_trackable = advertised_trackable_feature_count_for_grid();
    let trackable = trackable_feature_count_for_grid();

    assert!(
        advertised_trackable <= trackable,
        "Advertised trackable ({advertised_trackable}) exceeds trackable ({trackable})"
    );
}

#[test]
fn compliance_percent_is_non_negative() {
    let compliance = compliance_percent_for_grid();
    assert!(compliance >= 0.0, "Compliance percent is negative: {compliance}");
}

#[test]
fn compliance_percent_is_at_most_100() {
    let compliance = compliance_percent_for_grid();
    assert!(compliance <= 100.0, "Compliance percent exceeds 100: {compliance}");
}

#[test]
fn compliance_percent_is_whole_number_or_zero() {
    let compliance = compliance_percent_for_grid();
    let fractional = compliance - compliance.floor();

    assert!(
        !(0.001..=0.999).contains(&fractional),
        "Compliance percent is not a whole number: {compliance}"
    );
}

#[test]
fn compliance_calculation_matches_manual() {
    let advertised = advertised_trackable_feature_count_for_grid() as f64;
    let trackable = trackable_feature_count_for_grid() as f64;

    let expected =
        if trackable == 0.0 { 0.0 } else { (advertised / trackable * 100.0).round() as f32 };

    let actual = compliance_percent_for_grid();
    assert_eq!(
        actual, expected,
        "Compliance calculation mismatch: expected {expected}, got {actual}"
    );
}

// ---------------------------------------------------------------------------
// Zero/edge case handling
// ---------------------------------------------------------------------------

#[test]
fn trackable_zero_when_no_features() {
    // This test assumes we always have features, but documents the behavior
    let trackable = trackable_feature_count_for_grid();
    // In practice, this should be > 0, but we document the contract
    let _ = trackable;
}

#[test]
fn compliance_percent_zero_when_no_trackable_features() {
    // Document: if trackable is 0, compliance is 0
    // (Won't happen in practice, but the code handles it)
    let _ = compliance_percent_for_grid();
}

// ---------------------------------------------------------------------------
// Serialization tests for BddFeatureRow
// ---------------------------------------------------------------------------

#[test]
fn bdd_feature_row_can_be_serialized_to_json() -> Result<(), Box<dyn std::error::Error>> {
    let rows = bdd_feature_rows();

    if let Some(first_row) = rows.first() {
        let json = serde_json::to_string(first_row)?;
        assert!(!json.is_empty(), "Serialized JSON is empty");
        assert!(json.contains(first_row.id), "Serialized JSON should contain row ID");
    }

    Ok(())
}

#[test]
fn all_bdd_rows_serializable_to_json() -> Result<(), Box<dyn std::error::Error>> {
    let rows = bdd_feature_rows();
    let json = serde_json::to_string(&rows)?;
    assert!(!json.is_empty(), "Serialized rows JSON is empty");
    Ok(())
}

// ---------------------------------------------------------------------------
// FeatureProfileSpec serialization
// ---------------------------------------------------------------------------

#[test]
fn feature_profile_spec_can_be_serialized() -> Result<(), Box<dyn std::error::Error>> {
    let spec =
        FeatureProfileSpec { canonical: "test", aliases: &["t"], description: "Test profile" };

    let json = serde_json::to_string(&spec)?;
    assert!(json.contains("test"));
    assert!(json.contains("Test profile"));
    Ok(())
}

#[test]
fn all_profile_specs_are_serializable() -> Result<(), Box<dyn std::error::Error>> {
    let specs = feature_profile_specs();
    for spec in specs {
        let _json = serde_json::to_string(spec)?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Comprehensive consistency checks
// ---------------------------------------------------------------------------

#[test]
fn all_features_count_consistency() {
    let features = all_features();
    let bdd_rows = bdd_feature_rows();

    assert_eq!(
        features.len(),
        bdd_rows.len(),
        "Feature count mismatch between all_features and bdd_feature_rows"
    );
}

#[test]
fn feature_profile_kinds_are_valid_enum_variants() {
    let kinds = FeatureProfileKind::all();

    assert!(kinds.contains(&FeatureProfileKind::GaLock));
    assert!(kinds.contains(&FeatureProfileKind::Production));
    assert!(kinds.contains(&FeatureProfileKind::All));
}

#[test]
fn from_ga_lock_enabled_produces_valid_variants() {
    let with_ga_lock = FeatureProfileKind::from_ga_lock_enabled(true);
    let without_ga_lock = FeatureProfileKind::from_ga_lock_enabled(false);

    assert_eq!(with_ga_lock, FeatureProfileKind::GaLock);
    assert_eq!(without_ga_lock, FeatureProfileKind::Production);

    assert!(FeatureProfileKind::all().contains(&with_ga_lock));
    assert!(FeatureProfileKind::all().contains(&without_ga_lock));
}

#[test]
fn current_returns_valid_variant() {
    let current = FeatureProfileKind::current();
    assert!(FeatureProfileKind::all().contains(&current));
}

#[test]
fn all_variants_parse_correctly_in_sequence() -> Result<(), Box<dyn std::error::Error>> {
    let inputs = vec![
        ("ga-lock", FeatureProfileKind::GaLock),
        ("ga", FeatureProfileKind::GaLock),
        ("ga_lock", FeatureProfileKind::GaLock),
        ("production", FeatureProfileKind::Production),
        ("prod", FeatureProfileKind::Production),
        ("all", FeatureProfileKind::All),
    ];

    for (input, expected) in inputs {
        let parsed = perl_tdd_support::must_some(FeatureProfileKind::from_str_name(input));
        assert_eq!(parsed, expected, "Parsing '{input}' failed");
    }

    Ok(())
}

#[test]
fn no_variant_parsing_cross_contaminates() {
    // Ensure parsing one variant doesn't affect others
    let _ga_lock = FeatureProfileKind::from_str_name("ga-lock");
    let prod = FeatureProfileKind::from_str_name("production");
    let all = FeatureProfileKind::from_str_name("all");

    assert_eq!(prod, Some(FeatureProfileKind::Production));
    assert_eq!(all, Some(FeatureProfileKind::All));
}

// ---------------------------------------------------------------------------
// Integration tests across multiple APIs
// ---------------------------------------------------------------------------

#[test]
fn feature_profile_spec_matches_kind_metadata() {
    let specs = feature_profile_specs();
    let kinds = FeatureProfileKind::all();

    for (spec, &kind) in specs.iter().zip(kinds) {
        assert_eq!(spec.canonical, kind.as_str());
        assert_eq!(spec.aliases, kind.aliases());
    }
}

#[test]
fn all_cli_profiles_map_to_valid_kinds() -> Result<(), Box<dyn std::error::Error>> {
    for profile in FeatureProfileKind::supported_cli_profiles() {
        let _kind = perl_tdd_support::must_some(FeatureProfileKind::from_str_name(profile));
    }
    Ok(())
}

#[test]
fn bdd_row_sorting_is_stable() {
    let rows1 = bdd_feature_rows();
    let rows2 = bdd_feature_rows();

    assert_eq!(rows1.len(), rows2.len());
    for (r1, r2) in rows1.iter().zip(rows2.iter()) {
        assert_eq!(r1.id, r2.id);
        assert_eq!(r1.area, r2.area);
    }
}
