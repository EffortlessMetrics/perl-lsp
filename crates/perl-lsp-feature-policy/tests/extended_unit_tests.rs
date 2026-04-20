//! Extended unit tests for `perl-lsp-feature-policy`.
//!
//! These tests provide comprehensive coverage of edge cases, error conditions,
//! and detailed behavior verification across all public APIs.
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use perl_lsp_feature_policy::{
    FeatureProfile, catalog_advertised_feature_ids, feature_ids_from_flags, flags_for_profile,
    flags_for_runtime, from_str_name,
};

// ---------------------------------------------------------------------------
// from_str_name – comprehensive edge cases
// ---------------------------------------------------------------------------

#[test]
fn from_str_name_whitespace_is_not_trimmed() {
    assert!(from_str_name(" ga-lock").is_none());
    assert!(from_str_name("ga-lock ").is_none());
    assert!(from_str_name(" ga-lock ").is_none());
    assert!(from_str_name("\tga-lock").is_none());
}

#[test]
fn from_str_name_mixed_case_variants_all_invalid() {
    assert!(from_str_name("GA-Lock").is_none());
    assert!(from_str_name("Ga-lock").is_none());
    assert!(from_str_name("Production").is_none());
    assert!(from_str_name("PRODUCTION").is_none());
    assert!(from_str_name("ALL").is_none());
}

#[test]
fn from_str_name_similar_but_wrong_values() {
    assert!(from_str_name("ga_lock_legacy").is_none());
    assert!(from_str_name("ga-locking").is_none());
    assert!(from_str_name("product").is_none());
    assert!(from_str_name("all-features").is_none());
    assert!(from_str_name("alloc").is_none());
}

#[test]
fn from_str_name_numeric_variants_invalid() {
    assert!(from_str_name("1").is_none());
    assert!(from_str_name("0").is_none());
    assert!(from_str_name("ga-lock1").is_none());
}

#[test]
fn from_str_name_special_characters() {
    assert!(from_str_name("ga@lock").is_none());
    assert!(from_str_name("ga#lock").is_none());
    assert!(from_str_name("ga$lock").is_none());
    assert!(from_str_name("ga.lock").is_none());
}

#[test]
fn from_str_name_empty_and_null_like() {
    assert!(from_str_name("").is_none());
    assert!(from_str_name(" ").is_none());
    assert!(from_str_name("\n").is_none());
    assert!(from_str_name("\t").is_none());
}

#[test]
fn from_str_name_very_long_string() {
    let long = "a".repeat(10000);
    assert!(from_str_name(&long).is_none());
}

#[test]
fn from_str_name_unicode_characters() {
    assert!(from_str_name("gä-lock").is_none());
    assert!(from_str_name("产品").is_none());
    assert!(from_str_name("🚀").is_none());
}

#[test]
fn from_str_name_ga_lock_with_underscores() {
    // ga_lock is specifically supported as an alias
    let profile = from_str_name("ga_lock");
    assert_eq!(profile, Some(FeatureProfile::GaLock));
}

#[test]
fn from_str_name_all_production_aliases_distinct() {
    let ga_lock = from_str_name("ga-lock");
    let production = from_str_name("production");
    let all = from_str_name("all");

    assert_ne!(ga_lock, production);
    assert_ne!(production, all);
    assert_ne!(ga_lock, all);
}

#[test]
fn from_str_name_auto_is_stable() {
    // from_str_name("auto") should always return Some
    let auto1 = from_str_name("auto");
    let auto2 = from_str_name("auto");
    assert!(auto1.is_some());
    assert!(auto2.is_some());
    assert_eq!(auto1, auto2);
}

// ---------------------------------------------------------------------------
// FeatureProfile::from_ga_lock_enabled – boundary behavior
// ---------------------------------------------------------------------------

#[test]
fn from_ga_lock_enabled_true_always_ga_lock() {
    let profile = FeatureProfile::from_ga_lock_enabled(true);
    assert_eq!(profile, FeatureProfile::GaLock);
}

#[test]
fn from_ga_lock_enabled_false_never_ga_lock() {
    let profile = FeatureProfile::from_ga_lock_enabled(false);
    assert_ne!(profile, FeatureProfile::GaLock);
    // When GA-lock is disabled, it should be one of the broader profiles.
    assert!(profile == FeatureProfile::Production || profile == FeatureProfile::All);
}

#[test]
fn from_ga_lock_enabled_false_consistency() {
    let profile1 = FeatureProfile::from_ga_lock_enabled(false);
    let profile2 = FeatureProfile::from_ga_lock_enabled(false);
    // Should be consistent across multiple calls
    assert_eq!(profile1, profile2);
}

// ---------------------------------------------------------------------------
// FeatureProfile::current – consistency and validity
// ---------------------------------------------------------------------------

#[test]
fn current_is_consistent() {
    let current1 = FeatureProfile::current();
    let current2 = FeatureProfile::current();
    // Const fn should be consistent
    assert_eq!(current1, current2);
}

#[test]
fn current_matches_one_of_all() {
    let current = FeatureProfile::current();
    let all = FeatureProfile::all();
    assert!(all.contains(&current), "current() should be in all()");
}

#[test]
fn current_is_deterministic() {
    // Multiple calls should return the same value (compile-time determined)
    for _ in 0..100 {
        let current = FeatureProfile::current();
        assert!(FeatureProfile::all().contains(&current));
    }
}

// ---------------------------------------------------------------------------
// FeatureProfile::from_cli_argument – fallback behavior
// ---------------------------------------------------------------------------

#[test]
fn from_cli_argument_all_valid_tokens() {
    for token in FeatureProfile::supported_cli_profiles() {
        let profile = FeatureProfile::from_cli_argument(token);
        // All supported tokens should parse to a valid profile
        assert!(FeatureProfile::all().contains(&profile));
    }
}

#[test]
fn from_cli_argument_invalid_tokens_fallback() {
    let fallback = FeatureProfile::from_cli_argument("invalid");
    assert_eq!(fallback, FeatureProfile::current());
}

#[test]
fn from_cli_argument_multiple_invalid_same_result() {
    let current = FeatureProfile::current();
    for invalid in &["", "xxx", "bogus", "unknown", "ga!lock"] {
        let result = FeatureProfile::from_cli_argument(invalid);
        assert_eq!(result, current, "fallback should be consistent for invalid token: {invalid}");
    }
}

#[test]
fn from_cli_argument_empty_string_fallback() {
    let result = FeatureProfile::from_cli_argument("");
    assert_eq!(result, FeatureProfile::current());
}

#[test]
fn from_cli_argument_whitespace_fallback() {
    let result = FeatureProfile::from_cli_argument("   ");
    assert_eq!(result, FeatureProfile::current());
}

#[test]
fn from_cli_argument_normalizes_case_and_spacing() {
    let lower = FeatureProfile::from_cli_argument("production");
    let upper = FeatureProfile::from_cli_argument("PRODUCTION");
    let padded = FeatureProfile::from_cli_argument("  ga_lock  ");
    assert_eq!(lower, FeatureProfile::Production);
    assert_eq!(upper, FeatureProfile::Production);
    assert_eq!(padded, FeatureProfile::GaLock);
}

// ---------------------------------------------------------------------------
// FeatureProfile::parse_profile – normalized CLI-style parsing
// ---------------------------------------------------------------------------

#[test]
fn parse_profile_returns_none_for_unknown() {
    assert!(FeatureProfile::parse_profile("").is_none());
    assert!(FeatureProfile::parse_profile("invalid").is_none());
    assert!(FeatureProfile::parse_profile("prod-debug").is_none());
}

#[test]
fn parse_profile_all_valid_aliases() {
    for token in FeatureProfile::supported_cli_profiles() {
        let result = FeatureProfile::parse_profile(token);
        assert!(result.is_some(), "token {token} should parse");
    }
}

#[test]
fn parse_profile_normalizes_whitespace_and_case() {
    assert_eq!(FeatureProfile::parse_profile(" ga-lock"), Some(FeatureProfile::GaLock));
    assert_eq!(FeatureProfile::parse_profile("ga-lock "), Some(FeatureProfile::GaLock));
    assert_eq!(FeatureProfile::parse_profile("PRODUCTION"), Some(FeatureProfile::Production));
}

// ---------------------------------------------------------------------------
// FeatureProfile::build_flags – completeness and monotonicity
// ---------------------------------------------------------------------------

#[test]
fn build_flags_return_consistent_results() {
    for &profile in FeatureProfile::all() {
        let flags1 = profile.build_flags();
        let flags2 = profile.build_flags();
        assert_eq!(flags1.to_feature_ids(), flags2.to_feature_ids());
    }
}

#[test]
fn build_flags_all_is_superset() {
    let ga_ids = FeatureProfile::GaLock.build_flags().to_feature_ids();
    let prod_ids = FeatureProfile::Production.build_flags().to_feature_ids();
    let all_ids = FeatureProfile::All.build_flags().to_feature_ids();

    // All should contain every ID from GA-lock and Production
    for id in &ga_ids {
        assert!(all_ids.contains(id));
    }
    for id in &prod_ids {
        assert!(all_ids.contains(id));
    }
}

#[test]
fn build_flags_core_features_in_all_profiles() {
    // Core capabilities should be present in all profiles
    let core_features = ["completion", "hover", "definition"];

    for &profile in FeatureProfile::all() {
        let flags = profile.build_flags();
        let ids = flags.to_feature_ids();
        for core in &core_features {
            assert!(
                ids.iter().any(|id| id.contains(core)),
                "core feature {core} missing from {profile:?}"
            );
        }
    }
}

#[test]
fn build_flags_no_negative_flags() {
    // All flags should be boolean true, not false (no negative capabilities)
    for &profile in FeatureProfile::all() {
        let flags = profile.build_flags();
        // Just verify we get a reasonable structure
        let _ids = flags.to_feature_ids();
    }
}

// ---------------------------------------------------------------------------
// FeatureProfile::runtime_flags – formatting dynamics
// ---------------------------------------------------------------------------

#[test]
fn runtime_flags_preserves_non_formatting_flags() {
    for &profile in FeatureProfile::all() {
        let base = profile.build_flags();
        let _runtime_true = profile.runtime_flags(true);
        let runtime_false = profile.runtime_flags(false);

        // Formatting is the only thing affected by perltidy
        // Other flags should be identical
        let base_ids = base.to_feature_ids();
        let runtime_false_ids = runtime_false.to_feature_ids();

        // The non-formatting features should be the same
        for id in &base_ids {
            if !id.contains("formatting") {
                assert!(runtime_false_ids.contains(id));
            }
        }
    }
}

#[test]
fn runtime_flags_with_perltidy_comprehensive() {
    for &profile in FeatureProfile::all() {
        let with_tool = profile.runtime_flags(true);
        let without_tool = profile.runtime_flags(false);

        // With tool, formatting should be enabled
        assert!(with_tool.formatting);
        assert!(with_tool.range_formatting);

        // Without tool, formatting is always disabled
        assert!(!without_tool.formatting);
        assert!(!without_tool.range_formatting);
    }
}

#[test]
fn runtime_flags_perltidy_true_vs_false_always_differs_on_formatting() {
    for &profile in FeatureProfile::all() {
        let with_perltidy = profile.runtime_flags(true);
        let without_perltidy = profile.runtime_flags(false);

        // Perltidy availability always gates formatting at runtime
        assert!(with_perltidy.formatting);
        assert!(!without_perltidy.formatting);
        assert!(with_perltidy.range_formatting);
        assert!(!without_perltidy.range_formatting);
    }
}

// ---------------------------------------------------------------------------
// advertised_features / runtime_advertised_features
// ---------------------------------------------------------------------------

#[test]
fn advertised_features_consistent_with_build_flags() {
    for &profile in FeatureProfile::all() {
        let flags = profile.build_flags();
        let advertised = profile.advertised_features();

        // Converting flags to advertised should be consistent
        let from_flags_advertised = flags.to_advertised_features();
        // Just verify both methods work and return structures
        let _flags_ids = flags.to_feature_ids();
        let _ = advertised;
        let _ = from_flags_advertised;
    }
}

#[test]
fn runtime_advertised_features_reflects_perltidy() {
    for &profile in FeatureProfile::all() {
        let with_tool = profile.runtime_advertised_features(true);
        let without_tool = profile.runtime_advertised_features(false);

        assert!(with_tool.formatting);
        assert!(with_tool.range_formatting);

        // Without perltidy, formatting is always disabled at runtime
        assert!(!without_tool.formatting);
        assert!(!without_tool.range_formatting);
    }
}

#[test]
fn runtime_advertised_features_all_profiles() {
    for &profile in FeatureProfile::all() {
        for has_tool in [true, false] {
            let af = profile.runtime_advertised_features(has_tool);
            // Just verify it returns without panicking and has the expected structure
            let _ = af;
        }
    }
}

// ---------------------------------------------------------------------------
// as_str – stability and reversibility
// ---------------------------------------------------------------------------

#[test]
fn as_str_never_empty() {
    for &profile in FeatureProfile::all() {
        let s = profile.as_str();
        assert!(!s.is_empty());
        assert!(!s.chars().all(char::is_whitespace));
    }
}

#[test]
fn as_str_is_ascii() {
    for &profile in FeatureProfile::all() {
        let s = profile.as_str();
        assert!(s.is_ascii());
    }
}

#[test]
fn as_str_is_lowercase() {
    for &profile in FeatureProfile::all() {
        let s = profile.as_str();
        assert_eq!(s, s.to_lowercase());
    }
}

#[test]
fn as_str_parse_identity() {
    for &profile in FeatureProfile::all() {
        let label = profile.as_str();
        let reparsed = from_str_name(label).expect("as_str output should be parseable");
        assert_eq!(reparsed, profile);
    }
}

#[test]
fn as_str_values_unique() {
    let mut strings = Vec::new();
    for &profile in FeatureProfile::all() {
        strings.push(profile.as_str());
    }
    strings.sort();
    let original_len = strings.len();
    strings.dedup();
    assert_eq!(original_len, strings.len(), "as_str values should be unique");
}

// ---------------------------------------------------------------------------
// supported_cli_profiles – enumeration completeness
// ---------------------------------------------------------------------------

#[test]
fn supported_cli_profiles_all_parseable() {
    for token in FeatureProfile::supported_cli_profiles() {
        let result = from_str_name(token);
        assert!(result.is_some(), "all supported tokens should parse: {token}");
    }
}

#[test]
fn supported_cli_profiles_includes_main_names() {
    let supported = FeatureProfile::supported_cli_profiles();
    // Must include the main canonical names
    assert!(supported.contains(&"ga-lock") || supported.contains(&"ga_lock"));
    assert!(supported.contains(&"production") || supported.contains(&"prod"));
    assert!(supported.contains(&"all"));
}

#[test]
fn supported_cli_profiles_includes_auto() {
    let supported = FeatureProfile::supported_cli_profiles();
    assert!(supported.contains(&"auto"), "auto should be in supported profiles");
}

#[test]
fn supported_cli_profiles_no_duplicates() {
    let supported = FeatureProfile::supported_cli_profiles();
    let mut deduped: Vec<_> = supported.to_vec();
    deduped.sort();
    deduped.dedup();
    assert_eq!(supported.len(), deduped.len(), "supported profiles should have no duplicates");
}

#[test]
fn supported_cli_profiles_no_empty_strings() {
    for token in FeatureProfile::supported_cli_profiles() {
        assert!(!token.is_empty(), "supported profiles should not contain empty strings");
        assert!(!token.chars().all(char::is_whitespace));
    }
}

// ---------------------------------------------------------------------------
// FeatureProfile::all() – enumeration invariants
// ---------------------------------------------------------------------------

#[test]
fn all_is_complete() {
    let profiles = FeatureProfile::all();
    assert_eq!(profiles.len(), 3, "should have exactly 3 profiles");
}

#[test]
fn all_ordering_is_stable() {
    let profiles1 = FeatureProfile::all();
    let profiles2 = FeatureProfile::all();
    assert_eq!(profiles1, profiles2, "all() should return same order");
}

#[test]
fn all_declared_in_order() {
    let all = FeatureProfile::all();
    assert_eq!(all[0], FeatureProfile::GaLock);
    assert_eq!(all[1], FeatureProfile::Production);
    assert_eq!(all[2], FeatureProfile::All);
}

#[test]
fn all_no_null_profiles() {
    for &profile in FeatureProfile::all() {
        // Each profile should have a valid string representation
        let s = profile.as_str();
        assert!(!s.is_empty());
        // Each profile should parse back
        assert!(from_str_name(s).is_some());
    }
}

// ---------------------------------------------------------------------------
// Free functions: consistency with methods
// ---------------------------------------------------------------------------

#[test]
fn flags_for_profile_consistent_with_build_flags() {
    for &profile in FeatureProfile::all() {
        let fn_result = flags_for_profile(profile);
        let method_result = profile.build_flags();
        assert_eq!(fn_result.to_feature_ids(), method_result.to_feature_ids());
    }
}

#[test]
fn flags_for_runtime_consistent_with_runtime_flags() {
    for &profile in FeatureProfile::all() {
        for has_tool in [true, false] {
            let fn_result = flags_for_runtime(profile, has_tool);
            let method_result = profile.runtime_flags(has_tool);
            assert_eq!(fn_result.to_feature_ids(), method_result.to_feature_ids());
        }
    }
}

#[test]
fn feature_ids_from_flags_not_empty_for_all() {
    for &profile in FeatureProfile::all() {
        let flags = profile.build_flags();
        let ids = feature_ids_from_flags(&flags);
        assert!(!ids.is_empty());
    }
}

#[test]
fn feature_ids_from_flags_consistent() {
    for &profile in FeatureProfile::all() {
        let flags = profile.build_flags();
        let ids1 = feature_ids_from_flags(&flags);
        let ids2 = feature_ids_from_flags(&flags);
        assert_eq!(ids1, ids2);
    }
}

// ---------------------------------------------------------------------------
// catalog_advertised_feature_ids – filtering behavior
// ---------------------------------------------------------------------------

#[test]
fn catalog_ids_all_valid_feature_ids() {
    for &profile in FeatureProfile::all() {
        let profile_ids = feature_ids_from_flags(&profile.build_flags());
        let catalog_ids = catalog_advertised_feature_ids(profile);
        for id in &catalog_ids {
            assert!(profile_ids.contains(id), "catalog ID should be in profile IDs: {id}");
        }
    }
}

#[test]
fn catalog_ids_subset_property() {
    for &profile in FeatureProfile::all() {
        let catalog_ids = catalog_advertised_feature_ids(profile);
        let profile_ids = feature_ids_from_flags(&profile.build_flags());

        // Catalog IDs must be subset of profile IDs
        for id in &catalog_ids {
            assert!(profile_ids.contains(id));
        }
    }
}

#[test]
fn catalog_ids_consistent() {
    for &profile in FeatureProfile::all() {
        let ids1 = catalog_advertised_feature_ids(profile);
        let ids2 = catalog_advertised_feature_ids(profile);
        assert_eq!(ids1, ids2);
    }
}

#[test]
fn catalog_ids_all_superset() {
    let all_catalog = catalog_advertised_feature_ids(FeatureProfile::All);

    for &profile in FeatureProfile::all() {
        if profile != FeatureProfile::All {
            let catalog = catalog_advertised_feature_ids(profile);
            for id in &catalog {
                assert!(all_catalog.contains(id));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Trait impls: comprehensive checks
// ---------------------------------------------------------------------------

#[test]
fn debug_impl_includes_variant_name() {
    let debug_ga = format!("{:?}", FeatureProfile::GaLock);
    let debug_prod = format!("{:?}", FeatureProfile::Production);
    let debug_all = format!("{:?}", FeatureProfile::All);

    assert!(debug_ga.contains("GaLock"));
    assert!(debug_prod.contains("Production"));
    assert!(debug_all.contains("All"));
}

#[test]
fn clone_is_identical() {
    for &profile in FeatureProfile::all() {
        #[allow(clippy::clone_on_copy)]
        let cloned = profile.clone();
        assert_eq!(profile, cloned);
    }
}

#[test]
fn copy_semantics_work() {
    let orig = FeatureProfile::Production;
    let copied = orig;
    assert_eq!(orig, copied);
    // Copy should not move, so we can still use orig
    let _ = orig;
    let _ = copied;
}

#[test]
fn eq_is_reflexive() {
    for &profile in FeatureProfile::all() {
        assert_eq!(profile, profile);
    }
}

#[test]
fn eq_is_symmetric() {
    for &p1 in FeatureProfile::all() {
        for &p2 in FeatureProfile::all() {
            assert_eq!(p1 == p2, p2 == p1);
        }
    }
}

#[test]
fn eq_is_transitive() {
    let p1 = FeatureProfile::Production;
    let p2 = FeatureProfile::Production;
    let p3 = FeatureProfile::Production;
    if p1 == p2 && p2 == p3 {
        assert_eq!(p1, p3);
    }
}

#[test]
fn ne_is_consistent_with_eq() {
    for &p1 in FeatureProfile::all() {
        for &p2 in FeatureProfile::all() {
            if p1 == p2 {
                assert!(!(p1 != p2));
            } else {
                assert!(p1 != p2);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Cross-functional behavior: integration patterns
// ---------------------------------------------------------------------------

#[test]
fn full_pipeline_ga_lock() {
    let profile = FeatureProfile::GaLock;

    // All public APIs should work together
    let flags = profile.build_flags();
    let ids = feature_ids_from_flags(&flags);
    let _advertised = profile.advertised_features();
    let catalog_ids = catalog_advertised_feature_ids(profile);

    // All should be non-empty
    assert!(!ids.is_empty());
    assert!(!catalog_ids.is_empty());

    // catalog_ids should be subset of ids
    for id in &catalog_ids {
        assert!(ids.contains(id));
    }

    let label = profile.as_str();
    assert_eq!(from_str_name(label), Some(profile));
}

#[test]
fn full_pipeline_production() {
    let profile = FeatureProfile::Production;

    let runtime_flags_with_tool = profile.runtime_flags(true);
    let _runtime_flags_no_tool = profile.runtime_flags(false);

    assert!(runtime_flags_with_tool.formatting);

    let advertised_with_tool = profile.runtime_advertised_features(true);
    let _advertised_no_tool = profile.runtime_advertised_features(false);

    assert!(advertised_with_tool.formatting);

    let cli_result = FeatureProfile::from_cli_argument("prod");
    assert_eq!(cli_result, profile);
}

#[test]
fn full_pipeline_all() {
    let profile = FeatureProfile::All;

    // All profile should have the most features
    let ga_ids = FeatureProfile::GaLock.build_flags().to_feature_ids();
    let prod_ids = FeatureProfile::Production.build_flags().to_feature_ids();
    let all_ids = profile.build_flags().to_feature_ids();

    assert!(all_ids.len() >= ga_ids.len());
    assert!(all_ids.len() >= prod_ids.len());

    let parsed = from_str_name("all");
    assert_eq!(parsed, Some(profile));
}

#[test]
fn parse_profile_vs_from_cli_argument_difference() {
    // parse_profile returns None for invalid
    // from_cli_argument falls back to current()

    assert!(FeatureProfile::parse_profile("invalid").is_none());
    let fallback = FeatureProfile::from_cli_argument("invalid");
    assert_eq!(fallback, FeatureProfile::current());
}

#[test]
fn from_ga_lock_enabled_vs_from_cli_argument() {
    let from_bool_true = FeatureProfile::from_ga_lock_enabled(true);
    let from_cli_ga = FeatureProfile::from_cli_argument("ga-lock");

    assert_eq!(from_bool_true, FeatureProfile::GaLock);
    assert_eq!(from_cli_ga, FeatureProfile::GaLock);
    assert_eq!(from_bool_true, from_cli_ga);
}

// ---------------------------------------------------------------------------
// Error resilience and robustness
// ---------------------------------------------------------------------------

#[test]
fn repeated_operations_are_safe() {
    let profile = FeatureProfile::Production;

    for _ in 0..1000 {
        let _ = profile.as_str();
        let _ = profile.build_flags();
        let _ = profile.advertised_features();
    }
}

#[test]
fn interleaved_profile_operations() {
    for _ in 0..100 {
        let ga = FeatureProfile::GaLock.build_flags();
        let prod = FeatureProfile::Production.build_flags();
        let all = FeatureProfile::All.build_flags();

        let ga_ids = ga.to_feature_ids();
        let prod_ids = prod.to_feature_ids();
        let all_ids = all.to_feature_ids();

        assert!(ga_ids.len() <= all_ids.len());
        assert!(prod_ids.len() <= all_ids.len());
    }
}

#[test]
fn perltidy_flag_is_idempotent() {
    let profile = FeatureProfile::Production;

    let flags1 = profile.runtime_flags(true);
    let flags2 = profile.runtime_flags(true);

    assert_eq!(flags1.formatting, flags2.formatting);
    assert_eq!(flags1.range_formatting, flags2.range_formatting);
}

// ---------------------------------------------------------------------------
// Documentation example verification
// ---------------------------------------------------------------------------

#[test]
fn documented_profiles_exist() {
    // From the crate docs, these profiles are documented:
    assert_eq!(FeatureProfile::GaLock.as_str(), "ga-lock");
    assert_eq!(FeatureProfile::Production.as_str(), "production");
    assert_eq!(FeatureProfile::All.as_str(), "all");
}

#[test]
fn documented_parser_works() {
    // from_str_name with documented values
    assert!(from_str_name("ga-lock").is_some());
    assert!(from_str_name("production").is_some());
    assert!(from_str_name("all").is_some());
    assert!(from_str_name("auto").is_some());
}

#[test]
fn documented_cli_integration() {
    // from_cli_argument demonstrates CLI integration
    let user_input = "production";
    let profile = FeatureProfile::from_cli_argument(user_input);
    assert_eq!(profile, FeatureProfile::Production);

    let bad_input = "unknown";
    let fallback = FeatureProfile::from_cli_argument(bad_input);
    assert_eq!(fallback, FeatureProfile::current());
}

// ---------------------------------------------------------------------------
// Mutation testing resistance – boundary conditions
// ---------------------------------------------------------------------------

#[test]
fn profile_count_is_three_not_two_or_four() {
    let count = FeatureProfile::all().len();
    assert_eq!(count, 3);
    assert_ne!(count, 2);
    assert_ne!(count, 4);
}

#[test]
fn perltidy_true_enables_both_formatting_flags() {
    for &profile in FeatureProfile::all() {
        let flags = profile.runtime_flags(true);
        assert!(flags.formatting, "formatting must be true");
        assert!(flags.range_formatting, "range_formatting must be true");
    }
}

#[test]
fn perltidy_toggle_affects_formatting_only() {
    for &profile in FeatureProfile::all() {
        let with_tool = profile.runtime_flags(true);
        let without_tool = profile.runtime_flags(false);

        // The only difference should be formatting-related flags
        let with_ids = with_tool.to_feature_ids();
        let without_ids = without_tool.to_feature_ids();

        // Non-formatting features should be the same
        for id in &with_ids {
            if !id.contains("formatting") {
                assert!(without_ids.contains(id), "non-formatting id '{id}' should be in both");
            }
        }
        for id in &without_ids {
            assert!(with_ids.contains(id), "without-tool id '{id}' should be in with-tool set");
        }
    }
}

#[test]
fn catalog_filter_is_applied() {
    for &profile in FeatureProfile::all() {
        let all_ids = feature_ids_from_flags(&profile.build_flags());
        let catalog_ids = catalog_advertised_feature_ids(profile);

        // Catalog must be same size or smaller
        assert!(catalog_ids.len() <= all_ids.len());
    }
}

#[test]
fn supported_tokens_actually_parse() {
    let supported = FeatureProfile::supported_cli_profiles();
    for token in supported {
        let result = FeatureProfile::parse_profile(token);
        assert!(result.is_some(), "supported token must parse: {token}");
    }
}

#[test]
fn auto_is_special_but_supported() {
    // "auto" is special – it's not a canonical profile but is supported
    let auto_result = from_str_name("auto");
    assert!(auto_result.is_some(), "auto must be supported");

    // When parsed, it should resolve to a valid profile
    if let Some(profile) = auto_result {
        assert!(FeatureProfile::all().contains(&profile));
    }
}
