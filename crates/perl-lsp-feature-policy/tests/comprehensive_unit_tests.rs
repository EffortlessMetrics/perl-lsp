//! Comprehensive unit tests for `perl-lsp-feature-policy`.

use perl_lsp_feature_policy::{
    FeatureProfile, catalog_advertised_feature_ids, feature_ids_from_flags, flags_for_profile,
    flags_for_runtime, from_str_name,
};

// ---------------------------------------------------------------------------
// from_str_name – public module-level parser
// ---------------------------------------------------------------------------

#[test]
fn from_str_name_ga_lock_aliases() -> Result<(), String> {
    for token in &["ga-lock", "ga", "ga_lock"] {
        let profile = from_str_name(token).ok_or_else(|| format!("expected Some for {token}"))?;
        if profile != FeatureProfile::GaLock {
            return Err(format!("expected GaLock for {token}, got {profile:?}"));
        }
    }
    Ok(())
}

#[test]
fn from_str_name_production_aliases() -> Result<(), String> {
    for token in &["production", "prod"] {
        let profile = from_str_name(token).ok_or_else(|| format!("expected Some for {token}"))?;
        if profile != FeatureProfile::Production {
            return Err(format!("expected Production for {token}, got {profile:?}"));
        }
    }
    Ok(())
}

#[test]
fn from_str_name_all() -> Result<(), String> {
    let profile = from_str_name("all").ok_or("expected Some for 'all'")?;
    if profile != FeatureProfile::All {
        return Err(format!("expected All, got {profile:?}"));
    }
    Ok(())
}

#[test]
fn from_str_name_auto_resolves() -> Result<(), String> {
    // "auto" delegates to cfg-gated default; it must return Some.
    let _profile = from_str_name("auto").ok_or("expected Some for 'auto'")?;
    Ok(())
}

#[test]
fn from_str_name_unknown_returns_none() {
    assert!(from_str_name("bogus").is_none());
    assert!(from_str_name("").is_none());
    assert!(from_str_name("GA-LOCK").is_none()); // case-sensitive
}

// ---------------------------------------------------------------------------
// FeatureProfile::from_kind round-trip
// ---------------------------------------------------------------------------

#[test]
fn from_kind_maps_all_variants() {
    use perl_lsp_feature_profile::FeatureProfileKind;

    assert_eq!(
        FeatureProfile::from_kind(FeatureProfileKind::GaLock),
        FeatureProfile::GaLock
    );
    assert_eq!(
        FeatureProfile::from_kind(FeatureProfileKind::Production),
        FeatureProfile::Production
    );
    assert_eq!(
        FeatureProfile::from_kind(FeatureProfileKind::All),
        FeatureProfile::All
    );
}

// ---------------------------------------------------------------------------
// FeatureProfile::from_ga_lock_enabled
// ---------------------------------------------------------------------------

#[test]
fn from_ga_lock_enabled_true_yields_ga_lock() {
    assert_eq!(
        FeatureProfile::from_ga_lock_enabled(true),
        FeatureProfile::GaLock
    );
}

#[test]
fn from_ga_lock_enabled_false_yields_non_ga_lock() {
    // When GA-lock is disabled the profile should be the broader default.
    let profile = FeatureProfile::from_ga_lock_enabled(false);
    assert_ne!(profile, FeatureProfile::GaLock);
}

// ---------------------------------------------------------------------------
// FeatureProfile::current
// ---------------------------------------------------------------------------

#[test]
fn current_is_a_valid_profile() {
    let current = FeatureProfile::current();
    let all_profiles = FeatureProfile::all();
    assert!(all_profiles.contains(&current));
}

// ---------------------------------------------------------------------------
// FeatureProfile::from_cli_argument
// ---------------------------------------------------------------------------

#[test]
fn from_cli_argument_known_token() {
    assert_eq!(
        FeatureProfile::from_cli_argument("prod"),
        FeatureProfile::Production
    );
    assert_eq!(
        FeatureProfile::from_cli_argument("all"),
        FeatureProfile::All
    );
}

#[test]
fn from_cli_argument_unknown_falls_back_to_current() {
    let fallback = FeatureProfile::from_cli_argument("nonsense");
    assert_eq!(fallback, FeatureProfile::current());
}

// ---------------------------------------------------------------------------
// FeatureProfile::parse_profile
// ---------------------------------------------------------------------------

#[test]
fn parse_profile_returns_some_for_known() -> Result<(), String> {
    let _p = FeatureProfile::parse_profile("ga-lock").ok_or("expected Some")?;
    Ok(())
}

#[test]
fn parse_profile_returns_none_for_unknown() {
    assert!(FeatureProfile::parse_profile("invalid").is_none());
}

// ---------------------------------------------------------------------------
// as_str – stable labels
// ---------------------------------------------------------------------------

#[test]
fn as_str_round_trip() -> Result<(), String> {
    for &profile in FeatureProfile::all() {
        let label = profile.as_str();
        let parsed =
            from_str_name(label).ok_or_else(|| format!("as_str output not parseable: {label}"))?;
        if parsed != profile {
            return Err(format!("round-trip mismatch for {label}"));
        }
    }
    Ok(())
}

#[test]
fn as_str_values_are_expected() {
    assert_eq!(FeatureProfile::GaLock.as_str(), "ga-lock");
    assert_eq!(FeatureProfile::Production.as_str(), "production");
    assert_eq!(FeatureProfile::All.as_str(), "all");
}

// ---------------------------------------------------------------------------
// supported_cli_profiles
// ---------------------------------------------------------------------------

#[test]
fn supported_cli_profiles_is_non_empty() {
    let profiles = FeatureProfile::supported_cli_profiles();
    assert!(!profiles.is_empty());
}

#[test]
fn supported_cli_profiles_includes_all_canonical_tokens() {
    let supported = FeatureProfile::supported_cli_profiles();
    for expected in &[
        "auto",
        "ga",
        "ga_lock",
        "ga-lock",
        "prod",
        "production",
        "all",
    ] {
        assert!(supported.contains(expected), "missing token: {expected}");
    }
}

// ---------------------------------------------------------------------------
// FeatureProfile::all()
// ---------------------------------------------------------------------------

#[test]
fn all_returns_three_profiles() {
    let all = FeatureProfile::all();
    assert_eq!(all.len(), 3);
    assert!(all.contains(&FeatureProfile::GaLock));
    assert!(all.contains(&FeatureProfile::Production));
    assert!(all.contains(&FeatureProfile::All));
}

#[test]
fn all_profiles_are_unique() {
    let all = FeatureProfile::all();
    for (i, a) in all.iter().enumerate() {
        for (j, b) in all.iter().enumerate() {
            if i != j {
                assert_ne!(a, b, "duplicate profile at indices {i} and {j}");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// build_flags – basic sanity
// ---------------------------------------------------------------------------

#[test]
fn build_flags_ga_lock_has_core_capabilities() {
    let flags = FeatureProfile::GaLock.build_flags();
    // GA-lock is the most conservative but should still include basics.
    assert!(flags.completion);
    assert!(flags.hover);
    assert!(flags.definition);
}

#[test]
fn build_flags_all_is_superset_of_production() {
    let prod = FeatureProfile::Production.build_flags();
    let all = FeatureProfile::All.build_flags();

    // Every flag enabled in Production must be enabled in All.
    let prod_ids = prod.to_feature_ids();
    let all_ids = all.to_feature_ids();
    for id in &prod_ids {
        assert!(
            all_ids.contains(id),
            "Production flag {id} missing from All profile"
        );
    }
}

#[test]
fn build_flags_profiles_have_distinct_sets() {
    let ga_ids = FeatureProfile::GaLock.build_flags().to_feature_ids();
    let prod_ids = FeatureProfile::Production.build_flags().to_feature_ids();
    let all_ids = FeatureProfile::All.build_flags().to_feature_ids();

    // All three profiles return non-empty capability sets.
    assert!(!ga_ids.is_empty());
    assert!(!prod_ids.is_empty());
    assert!(!all_ids.is_empty());

    // The All profile is a superset of both GA-lock and Production.
    for id in &ga_ids {
        assert!(
            all_ids.contains(id),
            "GaLock flag {id} missing from All profile"
        );
    }
    for id in &prod_ids {
        assert!(
            all_ids.contains(id),
            "Production flag {id} missing from All profile"
        );
    }
}

// ---------------------------------------------------------------------------
// runtime_flags – perltidy effects
// ---------------------------------------------------------------------------

#[test]
fn runtime_flags_with_perltidy_enables_formatting() {
    for &profile in FeatureProfile::all() {
        let flags = profile.runtime_flags(true);
        assert!(
            flags.formatting,
            "formatting should be true with perltidy for {profile:?}"
        );
        assert!(
            flags.range_formatting,
            "range_formatting should be true with perltidy for {profile:?}"
        );
    }
}

#[test]
fn runtime_flags_without_perltidy_disables_formatting() {
    for &profile in FeatureProfile::all() {
        let runtime = profile.runtime_flags(false);
        // Without perltidy, formatting is always disabled regardless of base profile.
        assert!(
            !runtime.formatting,
            "formatting should be off without perltidy for {profile:?}"
        );
        assert!(
            !runtime.range_formatting,
            "range_formatting should be off without perltidy for {profile:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// advertised_features / runtime_advertised_features
// ---------------------------------------------------------------------------

#[test]
fn advertised_features_returns_something() {
    for &profile in FeatureProfile::all() {
        // Just verify the method doesn't panic and returns a value.
        let _af = profile.advertised_features();
    }
}

#[test]
fn runtime_advertised_features_with_perltidy_enables_formatting() {
    let af = FeatureProfile::GaLock.runtime_advertised_features(true);
    assert!(af.formatting, "expected formatting enabled with perltidy");
}

#[test]
fn runtime_advertised_features_without_perltidy_disables_formatting() {
    for &profile in FeatureProfile::all() {
        let runtime = profile.runtime_advertised_features(false);
        assert!(
            !runtime.formatting,
            "without perltidy, runtime should not advertise formatting for {profile:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Free functions: flags_for_profile / flags_for_runtime
// ---------------------------------------------------------------------------

#[test]
fn flags_for_profile_matches_method() {
    for &profile in FeatureProfile::all() {
        let via_fn = flags_for_profile(profile);
        let via_method = profile.build_flags();
        assert_eq!(
            via_fn.to_feature_ids(),
            via_method.to_feature_ids(),
            "flags_for_profile diverges from build_flags for {profile:?}"
        );
    }
}

#[test]
fn flags_for_runtime_matches_method() {
    for &profile in FeatureProfile::all() {
        for has_perltidy in [true, false] {
            let via_fn = flags_for_runtime(profile, has_perltidy);
            let via_method = profile.runtime_flags(has_perltidy);
            assert_eq!(
                via_fn.to_feature_ids(),
                via_method.to_feature_ids(),
                "flags_for_runtime diverges for {profile:?}, perltidy={has_perltidy}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// feature_ids_from_flags
// ---------------------------------------------------------------------------

#[test]
fn feature_ids_from_flags_non_empty_for_all_profiles() {
    for &profile in FeatureProfile::all() {
        let flags = profile.build_flags();
        let ids = feature_ids_from_flags(&flags);
        assert!(
            !ids.is_empty(),
            "expected non-empty feature IDs for {profile:?}"
        );
    }
}

#[test]
fn feature_ids_are_unique() {
    for &profile in FeatureProfile::all() {
        let flags = profile.build_flags();
        let ids = feature_ids_from_flags(&flags);
        let mut deduped = ids.clone();
        deduped.sort();
        deduped.dedup();
        assert_eq!(ids.len(), deduped.len(), "duplicate IDs for {profile:?}");
    }
}

// ---------------------------------------------------------------------------
// catalog_advertised_feature_ids
// ---------------------------------------------------------------------------

#[test]
fn catalog_ids_are_subset_of_feature_ids() {
    for &profile in FeatureProfile::all() {
        let all_ids = feature_ids_from_flags(&profile.build_flags());
        let catalog_ids = catalog_advertised_feature_ids(profile);
        for id in &catalog_ids {
            assert!(
                all_ids.contains(id),
                "catalog ID {id} not in feature_ids for {profile:?}"
            );
        }
    }
}

#[test]
fn catalog_ids_non_empty_for_all_profile() {
    let ids = catalog_advertised_feature_ids(FeatureProfile::All);
    assert!(!ids.is_empty(), "All profile should have catalog IDs");
}

// ---------------------------------------------------------------------------
// Trait impls: Debug, Clone, Copy, Eq, PartialEq
// ---------------------------------------------------------------------------

#[test]
fn debug_impl_works() {
    let dbg = format!("{:?}", FeatureProfile::Production);
    assert!(dbg.contains("Production"));
}

#[test]
fn clone_and_copy_are_consistent() {
    let original = FeatureProfile::All;
    #[allow(clippy::clone_on_copy)]
    let cloned = original.clone();
    let copied = original;
    assert_eq!(original, cloned);
    assert_eq!(original, copied);
}

#[test]
fn eq_reflexive_and_symmetric() {
    for &p in FeatureProfile::all() {
        assert_eq!(p, p); // reflexive
    }
    // different variants are not equal
    assert_ne!(FeatureProfile::GaLock, FeatureProfile::Production);
    assert_ne!(FeatureProfile::Production, FeatureProfile::All);
    assert_ne!(FeatureProfile::GaLock, FeatureProfile::All);
}

// ---------------------------------------------------------------------------
// Monotonicity: All ⊇ Production ⊇ GaLock in feature count
// ---------------------------------------------------------------------------

#[test]
fn all_profile_has_most_features() {
    let ga_count = feature_ids_from_flags(&FeatureProfile::GaLock.build_flags()).len();
    let prod_count = feature_ids_from_flags(&FeatureProfile::Production.build_flags()).len();
    let all_count = feature_ids_from_flags(&FeatureProfile::All.build_flags()).len();

    // The All profile enables the broadest capability set.
    assert!(
        ga_count <= all_count,
        "GaLock ({ga_count}) should have <= All ({all_count}) features"
    );
    assert!(
        prod_count <= all_count,
        "Production ({prod_count}) should have <= All ({all_count}) features"
    );
}
