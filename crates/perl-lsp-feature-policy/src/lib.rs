#![warn(missing_docs)]
//! LSP feature policy and capability profile helpers.
//!
//! This microcrate centralizes capability set selection, turning high-level profile
//! decisions (e.g. `ga-lock`, `production`, `all`) into runtime [`BuildFlags`] and
//! catalog-oriented feature IDs. It bridges [`FeatureProfileKind`] to the
//! [`AdvertisedFeatures`] projection consumed by server startup and the
//! `initialize` response.

use perl_lsp_feature_contracts::advertised_features;
use perl_lsp_feature_flags::{AdvertisedFeatures, BuildFlags};
use perl_lsp_feature_profile::{FeatureProfileKind, parse_profile_token};

/// Parse a user-facing feature profile name into a `FeatureProfile`.
///
/// Supported values:
/// - `ga-lock` or `ga`
/// - `production` or `prod`
/// - `all`
/// - `auto` (falls back to `cfg`-gated default)
///
/// Unknown values return `None`.
pub fn from_str_name(s: &str) -> Option<FeatureProfile> {
    parse_profile_token(s).map(FeatureProfile::from_kind)
}

/// Known feature profiles for runtime capability selection.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum FeatureProfile {
    /// Conservative GA-lock set (legacy compatibility mode).
    GaLock,
    /// Standard production profile used for normal runtime operation.
    Production,
    /// All in-tree capabilities, useful for test matrices and snapshots.
    All,
}

impl FeatureProfile {
    /// Convert canonical profile IDs to `FeatureProfile` values.
    pub const fn from_kind(profile: FeatureProfileKind) -> Self {
        match profile {
            FeatureProfileKind::GaLock => Self::GaLock,
            FeatureProfileKind::Production => Self::Production,
            FeatureProfileKind::All => Self::All,
        }
    }

    /// Build the profile from an explicit GA-lock toggle.
    pub const fn from_ga_lock_enabled(ga_lock_enabled: bool) -> Self {
        Self::from_kind(FeatureProfileKind::from_ga_lock_enabled(ga_lock_enabled))
    }

    /// Resolve the active policy from compiled crate features.
    ///
    /// This keeps all consumers using a single profile source and reduces
    /// duplication where capability selection previously hardcoded
    /// `cfg!(feature = "lsp-ga-lock")` at each call-site.
    pub const fn current() -> Self {
        Self::from_kind(FeatureProfileKind::current())
    }

    /// Resolve a user-provided profile, falling back to `current()` on invalid input.
    ///
    /// This API is intended for CLI and editor integration where users may provide
    /// explicit profile controls at runtime.
    pub fn from_cli_argument(raw_profile: &str) -> Self {
        from_str_name(raw_profile).unwrap_or_else(Self::current)
    }

    /// Parse a CLI argument and return `None` for unknown values.
    pub fn parse_profile(raw_profile: &str) -> Option<Self> {
        from_str_name(raw_profile)
    }

    /// Convert this policy into base `BuildFlags`.
    pub fn build_flags(self) -> BuildFlags {
        match self {
            Self::GaLock => BuildFlags::ga_lock(),
            Self::Production => BuildFlags::production(),
            Self::All => BuildFlags::all(),
        }
    }

    /// Convert this policy into runtime `BuildFlags` that include
    /// per-tool availability effects.
    pub fn runtime_flags(self, has_perltidy: bool) -> BuildFlags {
        let mut flags = self.build_flags();

        if has_perltidy {
            flags.formatting = true;
            flags.range_formatting = true;
        }

        flags
    }

    /// Convert this policy into server advertised features.
    pub fn advertised_features(self) -> AdvertisedFeatures {
        self.build_flags().to_advertised_features()
    }

    /// Convert this policy into advertised features with runtime tooling checks.
    pub fn runtime_advertised_features(self, has_perltidy: bool) -> AdvertisedFeatures {
        self.runtime_flags(has_perltidy).to_advertised_features()
    }

    /// Return the user-facing CLI/profile display label for this profile.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GaLock => FeatureProfileKind::GaLock.as_str(),
            Self::Production => FeatureProfileKind::Production.as_str(),
            Self::All => FeatureProfileKind::All.as_str(),
        }
    }

    /// Return every supported CLI token accepted by `FeatureProfile::parse_profile`.
    pub const fn supported_cli_profiles() -> &'static [&'static str] {
        perl_lsp_feature_profile::supported_cli_profiles()
    }

    /// Return all canonical profiles in declaration order.
    pub const fn all() -> &'static [Self] {
        &[Self::GaLock, Self::Production, Self::All]
    }
}

/// Resolve `BuildFlags` for the profile.
pub fn flags_for_profile(profile: FeatureProfile) -> BuildFlags {
    profile.build_flags()
}

/// Resolve `BuildFlags` for runtime startup where formatting is conditional
/// on external tooling availability.
pub fn flags_for_runtime(profile: FeatureProfile, has_perltidy: bool) -> BuildFlags {
    profile.runtime_flags(has_perltidy)
}

/// Convert `BuildFlags` into canonical LSP feature identifiers.
pub fn feature_ids_from_flags(flags: &BuildFlags) -> Vec<&'static str> {
    flags.to_feature_ids()
}

/// Return advertised feature IDs from the current profile, intersecting with
/// the catalog so this API remains aligned to the BDD grid.
pub fn catalog_advertised_feature_ids(profile: FeatureProfile) -> Vec<&'static str> {
    let catalog_ids = advertised_features();
    let mut ids = feature_ids_from_flags(&flags_for_profile(profile));

    ids.retain(|id| catalog_ids.contains(id));
    ids
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_labels_are_stable() {
        assert_eq!(FeatureProfile::GaLock.as_str(), "ga-lock");
        assert_eq!(FeatureProfile::Production.as_str(), "production");
        assert_eq!(FeatureProfile::All.as_str(), "all");
    }

    #[test]
    fn supported_cli_profiles_contains_expected_values() {
        let supported = FeatureProfile::supported_cli_profiles();
        assert!(supported.contains(&"auto"));
        assert!(supported.contains(&"ga"));
        assert!(supported.contains(&"ga_lock"));
        assert!(supported.contains(&"ga-lock"));
        assert!(supported.contains(&"prod"));
        assert!(supported.contains(&"production"));
        assert!(supported.contains(&"all"));
    }

    // ── from_kind round-trip ────────────────────────────────────────

    #[test]
    fn from_kind_preserves_all_variants() {
        assert_eq!(FeatureProfile::from_kind(FeatureProfileKind::GaLock), FeatureProfile::GaLock,);
        assert_eq!(
            FeatureProfile::from_kind(FeatureProfileKind::Production),
            FeatureProfile::Production,
        );
        assert_eq!(FeatureProfile::from_kind(FeatureProfileKind::All), FeatureProfile::All,);
    }

    // ── from_ga_lock_enabled ────────────────────────────────────────

    #[test]
    fn from_ga_lock_enabled_true_is_ga_lock() {
        assert_eq!(FeatureProfile::from_ga_lock_enabled(true), FeatureProfile::GaLock);
    }

    #[test]
    fn from_ga_lock_enabled_false_is_production() {
        assert_eq!(FeatureProfile::from_ga_lock_enabled(false), FeatureProfile::Production);
    }

    // ── from_cli_argument ───────────────────────────────────────────

    #[test]
    fn from_cli_argument_resolves_known_tokens() {
        assert_eq!(FeatureProfile::from_cli_argument("ga-lock"), FeatureProfile::GaLock);
        assert_eq!(FeatureProfile::from_cli_argument("prod"), FeatureProfile::Production);
        assert_eq!(FeatureProfile::from_cli_argument("all"), FeatureProfile::All);
    }

    #[test]
    fn from_cli_argument_falls_back_for_unknown() {
        let result = FeatureProfile::from_cli_argument("bogus");
        assert_eq!(result, FeatureProfile::current());
    }

    // ── parse_profile ───────────────────────────────────────────────

    #[test]
    fn parse_profile_returns_none_for_unknown() {
        assert!(FeatureProfile::parse_profile("nope").is_none());
    }

    #[test]
    fn parse_profile_returns_some_for_valid() {
        assert_eq!(FeatureProfile::parse_profile("all"), Some(FeatureProfile::All));
    }

    // ── build_flags and profile shapes ──────────────────────────────

    #[test]
    fn build_flags_returns_ga_lock_for_ga_lock_profile() {
        let flags = FeatureProfile::GaLock.build_flags();
        let expected = BuildFlags::ga_lock();
        assert_eq!(flags, expected);
    }

    #[test]
    fn build_flags_returns_production_for_production_profile() {
        let flags = FeatureProfile::Production.build_flags();
        let expected = BuildFlags::production();
        assert_eq!(flags, expected);
    }

    #[test]
    fn build_flags_returns_all_for_all_profile() {
        let flags = FeatureProfile::All.build_flags();
        let expected = BuildFlags::all();
        assert_eq!(flags, expected);
    }

    // ── runtime_flags with perltidy ─────────────────────────────────

    #[test]
    fn runtime_flags_enables_formatting_when_perltidy_available() {
        let flags = FeatureProfile::Production.runtime_flags(true);
        assert!(flags.formatting, "formatting should be enabled with perltidy");
        assert!(flags.range_formatting, "range_formatting should be enabled with perltidy");
    }

    #[test]
    fn runtime_flags_preserves_disabled_formatting_without_perltidy() {
        let flags = FeatureProfile::Production.runtime_flags(false);
        assert!(!flags.formatting, "formatting should remain off without perltidy");
        assert!(!flags.range_formatting, "range_formatting should remain off without perltidy");
    }

    // ── flags_for_profile / flags_for_runtime ───────────────────────

    #[test]
    fn flags_for_profile_matches_build_flags() {
        for profile in FeatureProfile::all() {
            assert_eq!(
                flags_for_profile(*profile),
                profile.build_flags(),
                "flags_for_profile({}) should match build_flags()",
                profile.as_str(),
            );
        }
    }

    #[test]
    fn flags_for_runtime_matches_runtime_flags() {
        for &has_perltidy in &[true, false] {
            for profile in FeatureProfile::all() {
                assert_eq!(
                    flags_for_runtime(*profile, has_perltidy),
                    profile.runtime_flags(has_perltidy),
                );
            }
        }
    }

    // ── advertised_features ─────────────────────────────────────────

    #[test]
    fn advertised_features_reflects_build_flags() {
        let adv = FeatureProfile::Production.advertised_features();
        assert!(adv.completion);
        assert!(adv.hover);
        assert!(!adv.formatting, "production does not advertise formatting without perltidy");
    }

    #[test]
    fn runtime_advertised_features_with_perltidy() {
        let adv = FeatureProfile::Production.runtime_advertised_features(true);
        assert!(adv.formatting, "production should advertise formatting with perltidy");
    }

    // ── catalog_advertised_feature_ids ──────────────────────────────

    #[test]
    fn catalog_advertised_ids_are_non_empty_for_all_profiles() {
        for profile in FeatureProfile::all() {
            let ids = catalog_advertised_feature_ids(*profile);
            assert!(
                !ids.is_empty(),
                "catalog_advertised_feature_ids({}) should not be empty",
                profile.as_str(),
            );
        }
    }

    #[test]
    fn catalog_advertised_ids_all_superset_of_ga_lock() {
        let all_ids = catalog_advertised_feature_ids(FeatureProfile::All);
        let ga_ids = catalog_advertised_feature_ids(FeatureProfile::GaLock);
        for id in &ga_ids {
            assert!(all_ids.contains(id), "'all' advertised IDs should contain ga-lock ID '{id}'");
        }
    }

    #[test]
    fn catalog_advertised_ids_only_contain_catalog_known_ids() {
        let catalog_ids = advertised_features();
        for profile in FeatureProfile::all() {
            let ids = catalog_advertised_feature_ids(*profile);
            for id in &ids {
                assert!(
                    catalog_ids.contains(id),
                    "profile '{}' emitted non-catalog ID '{id}'",
                    profile.as_str(),
                );
            }
        }
    }

    // ── all() profiles ──────────────────────────────────────────────

    #[test]
    fn all_profiles_returns_three() {
        assert_eq!(FeatureProfile::all().len(), 3);
    }

    // ── feature_ids_from_flags ──────────────────────────────────────

    #[test]
    fn feature_ids_from_flags_for_default_is_empty() {
        let ids = feature_ids_from_flags(&BuildFlags::default());
        assert!(ids.is_empty());
    }
}
