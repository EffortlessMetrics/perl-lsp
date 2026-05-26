//! Property tests for perl-pragma invariants.
//!
//! Covers:
//! 1. Version-parsing determinism and round-trip (for canonical forms).
//! 2. Feature-bundle versioning rules: features added/removed per spec.
//! 3. Version comparison ordering consistency.
//! 4. `version_implies_strict` / `version_implies_warnings` monotonicity.
//! 5. `PragmaState` determinism on repeated calls.
//!
//! ## Non-monotonic feature note
//!
//! Perl version feature bundles are NOT strictly monotonic (i.e. newer is not
//! a superset of older). Specific features are *removed* at certain milestones:
//!
//! - `switch`: present up to v5.34, removed from the v5.36 bundle.
//! - `smartmatch`: present up to v5.40, removed from the v5.42 bundle.
//! - `indirect` / `multidimensional`: removed from v5.38 bundle.
//! - `bareword_filehandles`: removed from v5.38 bundle.
//! - `apostrophe_as_package_separator`: removed from v5.42 bundle.
//!
//! Tests that check feature inclusion account for these removal points rather
//! than asserting a strict superset relationship.

use perl_pragma::{
    PerlVersion, features_enabled_by_version, parse_perl_version, version_implies_strict,
    version_implies_warnings,
};
use proptest::prelude::*;
use proptest::test_runner::Config as ProptestConfig;

// ---------------------------------------------------------------------------
// Strategies
// ---------------------------------------------------------------------------

/// Generate a realistic minor version (0..=99).
fn minor_strategy() -> impl Strategy<Value = u32> {
    0_u32..=99_u32
}

/// Generate a PerlVersion with major=5 and a realistic minor.
fn perl5_version_strategy() -> impl Strategy<Value = PerlVersion> {
    minor_strategy().prop_map(|minor| PerlVersion::new(5, minor))
}

/// Generate a v5.X version string (e.g. "v5.36").
fn v_prefix_string_strategy() -> impl Strategy<Value = String> {
    minor_strategy().prop_map(|minor| format!("v5.{minor}"))
}

/// Generate a 5.0XX version string where minor is zero-padded to 3 digits
/// (e.g. "5.036").
fn decimal_string_strategy() -> impl Strategy<Value = String> {
    minor_strategy().prop_map(|minor| format!("5.{minor:03}"))
}

/// Generate a plain 5.X version string without zero-padding (e.g. "5.36").
fn plain_decimal_strategy() -> impl Strategy<Value = String> {
    minor_strategy().prop_map(|minor| format!("5.{minor}"))
}

/// Mix of all version string syntaxes for a major-5 version.
fn mixed_version_string_strategy() -> impl Strategy<Value = String> {
    prop_oneof![v_prefix_string_strategy(), decimal_string_strategy(), plain_decimal_strategy(),]
}

// ---------------------------------------------------------------------------
// Proptest configuration
// ---------------------------------------------------------------------------

fn config() -> ProptestConfig {
    ProptestConfig { cases: 64, failure_persistence: None, ..ProptestConfig::default() }
}

// ---------------------------------------------------------------------------
// 1. Version parsing: determinism
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(config())]

    /// `parse_perl_version` is deterministic: the same input always returns the
    /// same result.
    #[test]
    fn prop_parse_version_deterministic(s in mixed_version_string_strategy()) {
        let first = parse_perl_version(&s);
        let second = parse_perl_version(&s);
        prop_assert_eq!(first, second, "parse_perl_version({:?}) gave different results on two calls", s);
    }

    /// `v5.X` and `5.X` both parse to the same PerlVersion when X is given
    /// without zero-padding.  The v-prefix form is always canonical when
    /// comparing same-minor variants across the two syntaxes.
    #[test]
    fn prop_v_prefix_and_plain_decimal_agree(minor in minor_strategy()) {
        let v_form   = format!("v5.{minor}");
        let dec_form = format!("5.{minor}");
        let parsed_v   = parse_perl_version(&v_form);
        let parsed_dec = parse_perl_version(&dec_form);
        prop_assert_eq!(
            parsed_v, parsed_dec,
            "v5.{} and 5.{} should parse to the same PerlVersion",
            minor, minor
        );
    }

    /// Zero-padded form `5.0XY` (3-digit minor like `5.036`) parses to the
    /// same value as the plain decimal `5.36` for the same minor number.
    #[test]
    fn prop_zero_padded_and_plain_agree(minor in minor_strategy()) {
        let padded = format!("5.{minor:03}");
        let plain  = format!("5.{minor}");
        let parsed_padded = parse_perl_version(&padded);
        let parsed_plain  = parse_perl_version(&plain);
        prop_assert_eq!(
            parsed_padded, parsed_plain,
            "5.{:03} and 5.{} should parse to the same PerlVersion",
            minor, minor
        );
    }

    /// Round-trip: for v-prefix forms "v5.X", parsing and re-formatting
    /// recovers the original minor.
    #[test]
    fn prop_v_prefix_round_trip(minor in minor_strategy()) {
        let input = format!("v5.{minor}");
        let version = parse_perl_version(&input);
        prop_assert!(version.is_some(), "Expected Some for well-formed input {:?}", input);
        if let Some(v) = version {
            prop_assert_eq!(v.major, 5);
            prop_assert_eq!(v.minor, minor, "minor mismatch for input {:?}", input);
        }
    }

    /// Round-trip: for zero-padded decimal forms "5.0XY", parsing recovers the
    /// minor value.
    #[test]
    fn prop_decimal_padded_round_trip(minor in minor_strategy()) {
        let input = format!("5.{minor:03}");
        let version = parse_perl_version(&input);
        prop_assert!(version.is_some(), "Expected Some for well-formed input {:?}", input);
        if let Some(v) = version {
            prop_assert_eq!(v.major, 5);
            prop_assert_eq!(v.minor, minor, "minor mismatch for input {:?}", input);
        }
    }

    /// All well-formed major-5 strings should parse to Some.
    #[test]
    fn prop_well_formed_strings_always_parse(s in mixed_version_string_strategy()) {
        let result = parse_perl_version(&s);
        prop_assert!(result.is_some(), "Expected Some for well-formed input {:?}", s);
    }
}

// ---------------------------------------------------------------------------
// 2. `version_implies_strict` / `version_implies_warnings` monotonicity
//
// These predicates are step functions:
//   - strict:   false for < 5.12, true for >= 5.12
//   - warnings: false for < 5.35, true for >= 5.35
//
// Monotonicity: once true at version V, it stays true for all V' >= V.
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(config())]

    /// If strict is implied at version V, it must also be implied at all V' > V
    /// (within the same major-5 generation, up to minor 99).
    #[test]
    fn prop_strict_implication_is_monotone(minor in 0_u32..=98_u32) {
        let v1 = PerlVersion::new(5, minor);
        let v2 = PerlVersion::new(5, minor + 1);
        if version_implies_strict(v1) {
            prop_assert!(
                version_implies_strict(v2),
                "strict implication should be monotone: true at 5.{} but false at 5.{}",
                minor, minor + 1
            );
        }
    }

    /// If warnings are implied at version V, they must also be implied at all
    /// V' > V (within the same major-5 generation, up to minor 99).
    #[test]
    fn prop_warnings_implication_is_monotone(minor in 0_u32..=98_u32) {
        let v1 = PerlVersion::new(5, minor);
        let v2 = PerlVersion::new(5, minor + 1);
        if version_implies_warnings(v1) {
            prop_assert!(
                version_implies_warnings(v2),
                "warnings implication should be monotone: true at 5.{} but false at 5.{}",
                minor, minor + 1
            );
        }
    }

    /// `version_implies_strict` is deterministic.
    #[test]
    fn prop_strict_implication_deterministic(v in perl5_version_strategy()) {
        let first  = version_implies_strict(v);
        let second = version_implies_strict(v);
        prop_assert_eq!(first, second);
    }

    /// `version_implies_warnings` is deterministic.
    #[test]
    fn prop_warnings_implication_deterministic(v in perl5_version_strategy()) {
        let first  = version_implies_warnings(v);
        let second = version_implies_warnings(v);
        prop_assert_eq!(first, second);
    }
}

// ---------------------------------------------------------------------------
// 3. Feature-bundle determinism
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(config())]

    /// `features_enabled_by_version` is deterministic: same input always
    /// produces the same set of features.
    #[test]
    fn prop_features_by_version_deterministic(v in perl5_version_strategy()) {
        let first  = features_enabled_by_version(v);
        let second = features_enabled_by_version(v);
        // Sort both to ignore ordering differences
        let mut f1 = first.clone();
        let mut f2 = second.clone();
        f1.sort_unstable();
        f2.sort_unstable();
        prop_assert_eq!(f1, f2, "features_enabled_by_version(5.{}) was non-deterministic", v.minor);
    }

    /// No duplicate features are returned in a bundle.
    #[test]
    fn prop_features_by_version_no_duplicates(v in perl5_version_strategy()) {
        let features = features_enabled_by_version(v);
        let mut sorted = features.clone();
        sorted.sort_unstable();
        sorted.dedup();
        prop_assert_eq!(
            features.len(), sorted.len(),
            "Duplicate features in bundle for 5.{}: {:?}",
            v.minor,
            features
        );
    }

    /// The feature list is never empty for any Perl 5 version (even pre-5.10
    /// uses a default bundle with legacy features).
    #[test]
    fn prop_features_always_nonempty(v in perl5_version_strategy()) {
        let features = features_enabled_by_version(v);
        prop_assert!(
            !features.is_empty(),
            "Expected non-empty feature list for 5.{}", v.minor
        );
    }
}

// ---------------------------------------------------------------------------
// 4. Specific-feature monotonicity within stable windows
//
// The overall feature set is not monotonic (features are removed at Perl
// version milestones), but within each stable window a feature should either
// always be present or always be absent.  Here we test a selection of
// features that have well-defined inclusion windows.
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(config())]

    /// `say` is present in every bundle from 5.10 onwards up to 5.99 within
    /// the current definition range.
    #[test]
    fn prop_say_present_from_5_10(minor in 10_u32..=99_u32) {
        let features = features_enabled_by_version(PerlVersion::new(5, minor));
        prop_assert!(
            features.contains(&"say"),
            "Expected 'say' in feature bundle for 5.{minor}, got: {features:?}"
        );
    }

    /// `state` is present in every bundle from 5.10 onwards.
    #[test]
    fn prop_state_present_from_5_10(minor in 10_u32..=99_u32) {
        let features = features_enabled_by_version(PerlVersion::new(5, minor));
        prop_assert!(
            features.contains(&"state"),
            "Expected 'state' in feature bundle for 5.{minor}, got: {features:?}"
        );
    }

    /// `signatures` is present from 5.36 onwards.
    #[test]
    fn prop_signatures_present_from_5_36(minor in 36_u32..=99_u32) {
        let features = features_enabled_by_version(PerlVersion::new(5, minor));
        prop_assert!(
            features.contains(&"signatures"),
            "Expected 'signatures' in feature bundle for 5.{minor}, got: {features:?}"
        );
    }

    /// `bitwise` is present from 5.28 onwards.
    #[test]
    fn prop_bitwise_present_from_5_28(minor in 28_u32..=99_u32) {
        let features = features_enabled_by_version(PerlVersion::new(5, minor));
        prop_assert!(
            features.contains(&"bitwise"),
            "Expected 'bitwise' in feature bundle for 5.{minor}, got: {features:?}"
        );
    }

    /// `switch` is absent from 5.36 and above (it was removed from the bundle).
    /// NOTE: This is the non-monotonic removal case the property tests document.
    #[test]
    fn prop_switch_absent_from_5_36(minor in 36_u32..=99_u32) {
        let features = features_enabled_by_version(PerlVersion::new(5, minor));
        prop_assert!(
            !features.contains(&"switch"),
            "'switch' should NOT be in feature bundle for 5.{minor} (removed at 5.36), got: {features:?}"
        );
    }

    /// `indirect` is absent from 5.38 and above (removed from bundle at 5.38).
    #[test]
    fn prop_indirect_absent_from_5_38(minor in 38_u32..=99_u32) {
        let features = features_enabled_by_version(PerlVersion::new(5, minor));
        prop_assert!(
            !features.contains(&"indirect"),
            "'indirect' should NOT be in feature bundle for 5.{minor} (removed at 5.38), got: {features:?}"
        );
    }

    /// `smartmatch` is absent from 5.42 and above (removed from bundle at 5.42).
    #[test]
    fn prop_smartmatch_absent_from_5_42(minor in 42_u32..=99_u32) {
        let features = features_enabled_by_version(PerlVersion::new(5, minor));
        prop_assert!(
            !features.contains(&"smartmatch"),
            "'smartmatch' should NOT be in feature bundle for 5.{minor} (removed at 5.42), got: {features:?}"
        );
    }

    /// `apostrophe_as_package_separator` is absent from 5.42 and above.
    #[test]
    fn prop_apostrophe_separator_absent_from_5_42(minor in 42_u32..=99_u32) {
        let features = features_enabled_by_version(PerlVersion::new(5, minor));
        prop_assert!(
            !features.contains(&"apostrophe_as_package_separator"),
            "'apostrophe_as_package_separator' should NOT be in feature bundle for 5.{minor}, got: {features:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// 5. PerlVersion ordering consistency
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(config())]

    /// PerlVersion ordering is reflexive: v <= v.
    #[test]
    fn prop_version_order_reflexive(v in perl5_version_strategy()) {
        prop_assert!(v <= v, "Version ordering should be reflexive: 5.{} <= 5.{}", v.minor, v.minor);
    }

    /// PerlVersion strict ordering is asymmetric: v1 < v2 implies !(v2 < v1).
    #[test]
    fn prop_version_strict_order_asymmetric(minor1 in minor_strategy(), minor2 in minor_strategy()) {
        let v1 = PerlVersion::new(5, minor1);
        let v2 = PerlVersion::new(5, minor2);
        if v1 < v2 {
            prop_assert_eq!(
                v2.cmp(&v1),
                std::cmp::Ordering::Greater,
                "5.{} < 5.{} should mean 5.{} > 5.{}",
                minor1,
                minor2,
                minor2,
                minor1
            );
        }
    }

    /// PerlVersion ordering is transitive: v1 <= v2 && v2 <= v3 implies v1 <= v3.
    #[test]
    fn prop_version_order_transitive(
        minor1 in minor_strategy(),
        minor2 in minor_strategy(),
        minor3 in minor_strategy(),
    ) {
        let v1 = PerlVersion::new(5, minor1);
        let v2 = PerlVersion::new(5, minor2);
        let v3 = PerlVersion::new(5, minor3);
        if v1 <= v2 && v2 <= v3 {
            prop_assert!(v1 <= v3, "Version order should be transitive: 5.{} <= 5.{} <= 5.{} implies 5.{} <= 5.{}", minor1, minor2, minor3, minor1, minor3);
        }
    }
}

// ---------------------------------------------------------------------------
// 6. Regression: boundary versions
// ---------------------------------------------------------------------------

#[test]
fn regression_v5_10_has_say_and_state() {
    let features = features_enabled_by_version(PerlVersion::new(5, 10));
    assert!(features.contains(&"say"), "v5.10 should have 'say'");
    assert!(features.contains(&"state"), "v5.10 should have 'state'");
}

#[test]
fn regression_v5_12_implies_strict() {
    assert!(version_implies_strict(PerlVersion::new(5, 12)), "v5.12 should imply strict");
    assert!(!version_implies_strict(PerlVersion::new(5, 11)), "v5.11 should NOT imply strict");
    assert!(!version_implies_strict(PerlVersion::new(5, 10)), "v5.10 should NOT imply strict");
}

#[test]
fn regression_v5_35_implies_warnings() {
    assert!(version_implies_warnings(PerlVersion::new(5, 35)), "v5.35 should imply warnings");
    assert!(!version_implies_warnings(PerlVersion::new(5, 34)), "v5.34 should NOT imply warnings");
}

#[test]
fn regression_v5_36_has_signatures_and_no_switch() {
    let features = features_enabled_by_version(PerlVersion::new(5, 36));
    assert!(features.contains(&"signatures"), "v5.36 should have 'signatures'");
    assert!(!features.contains(&"switch"), "v5.36 should NOT have 'switch'");
}

#[test]
fn regression_v5_40_has_try() {
    let features = features_enabled_by_version(PerlVersion::new(5, 40));
    assert!(features.contains(&"try"), "v5.40 should have 'try'");
    assert!(features.contains(&"signatures"), "v5.40 should have 'signatures'");
}

#[test]
fn regression_v5_42_drops_smartmatch_and_apostrophe_separator() {
    let features = features_enabled_by_version(PerlVersion::new(5, 42));
    assert!(!features.contains(&"smartmatch"), "v5.42 should NOT have 'smartmatch'");
    assert!(
        !features.contains(&"apostrophe_as_package_separator"),
        "v5.42 should NOT have 'apostrophe_as_package_separator'"
    );
}

#[test]
fn regression_pre_5_10_uses_default_bundle() {
    let pre10 = features_enabled_by_version(PerlVersion::new(5, 8));
    let default_has_smartmatch = pre10.contains(&"smartmatch");
    // Default bundle includes legacy features including smartmatch
    assert!(
        default_has_smartmatch,
        "pre-5.10 default bundle should contain 'smartmatch', got: {pre10:?}"
    );
}

#[test]
fn regression_parse_v5_36_forms() {
    // All three forms should parse to 5.36
    let expected = Some(PerlVersion::new(5, 36));
    assert_eq!(parse_perl_version("v5.36"), expected);
    assert_eq!(parse_perl_version("5.036"), expected);
    assert_eq!(parse_perl_version("5.36"), expected);
}

#[test]
fn regression_parse_v5_36_0_three_part() {
    // Three-part form: v5.36.0 should also parse to 5.36
    let result = parse_perl_version("v5.36.0");
    assert_eq!(result, Some(PerlVersion::new(5, 36)));
}

#[test]
fn regression_parse_developer_release() {
    // Developer releases like 5.012_001 should parse to 5.12
    let result = parse_perl_version("5.012_001");
    assert_eq!(result, Some(PerlVersion::new(5, 12)));
}
