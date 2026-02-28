//! LSP feature policy and capability profile helpers.
//!
//! This microcrate centralizes capability set selection, turning high-level profile
//! decisions (e.g. `ga-lock`, `production`, `all`) into runtime [`BuildFlags`] and
//! catalog-oriented feature IDs. It bridges [`FeatureProfileKind`] to the
//! [`AdvertisedFeatures`] projection consumed by server startup and the
//! `initialize` response.

use perl_lsp_feature_contracts::advertised_features;
use perl_lsp_feature_flags::{AdvertisedFeatures, BuildFlags};
use perl_lsp_feature_profile::FeatureProfileKind;

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
    FeatureProfileKind::from_str_name(s).map(FeatureProfile::from_kind)
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
    use super::FeatureProfile;

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
}
