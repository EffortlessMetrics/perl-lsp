#![warn(missing_docs)]
//! Shared feature contracts for profile parsing, BDD-grid rows, and capability mapping.
//!
//! This crate defines the canonical [`FeatureProfileKind`] enum and associated
//! [`FeatureProfileSpec`] metadata used for feature-coverage reporting. It sits
//! between `perl-lsp-feature-ids` (raw identifiers) and
//! `perl-lsp-feature-policy` (runtime capability selection).

pub use crate::capability_map::{caps_from_feature_ids, feature_ids_from_caps};
use serde::Serialize;

/// Canonical metadata for profile aliases and normalization behavior.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct FeatureProfileSpec {
    /// Canonical profile label used by CLI and runtime APIs.
    pub canonical: &'static str,
    /// Additional accepted CLI aliases for this profile.
    pub aliases: &'static [&'static str],
    /// Short human-friendly description for settings/docs tooling.
    pub description: &'static str,
}

const GA_LOCK_ALIASES: &[&str] = &["ga-lock", "ga", "ga_lock"];
const PRODUCTION_ALIASES: &[&str] = &["production", "prod"];
const ALL_ALIASES: &[&str] = &["all"];

/// Canonical profile definitions and alias map.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum FeatureProfileKind {
    /// Conservative GA-lock feature profile.
    GaLock,
    /// Default production profile.
    Production,
    /// All features enabled.
    All,
}

impl FeatureProfileKind {
    /// Parse a raw profile token into canonical form.
    pub fn from_str_name(s: &str) -> Option<Self> {
        match s {
            "auto" => Some(Self::current()),
            "ga-lock" | "ga" | "ga_lock" => Some(Self::GaLock),
            "production" | "prod" => Some(Self::Production),
            "all" => Some(Self::All),
            _ => None,
        }
    }

    /// Resolve whether the compiled binary default enables GA-lock mode.
    pub const fn current() -> Self {
        Self::from_ga_lock_enabled(cfg!(feature = "lsp-ga-lock"))
    }

    /// Resolve explicit GA-lock toggle into canonical profile.
    pub const fn from_ga_lock_enabled(ga_lock_enabled: bool) -> Self {
        if ga_lock_enabled { Self::GaLock } else { Self::Production }
    }

    /// Canonical runtime label for diagnostics and APIs.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GaLock => "ga-lock",
            Self::Production => "production",
            Self::All => "all",
        }
    }

    /// All canonical profiles.
    pub const fn all() -> &'static [Self] {
        &[Self::GaLock, Self::Production, Self::All]
    }

    /// Supported CLI tokens, including aliases and backward compatible forms.
    pub const fn supported_cli_profiles() -> &'static [&'static str] {
        const PROFILE_CLI_NAMES: &[&str] =
            &["auto", "ga-lock", "ga", "ga_lock", "prod", "production", "all"];

        PROFILE_CLI_NAMES
    }

    /// Static alias metadata for this profile.
    pub const fn aliases(self) -> &'static [&'static str] {
        match self {
            Self::GaLock => GA_LOCK_ALIASES,
            Self::Production => PRODUCTION_ALIASES,
            Self::All => ALL_ALIASES,
        }
    }
}

/// A serializable profile metadata row for tooling and interoperability.
pub const FEATURE_PROFILE_SPECS: &[FeatureProfileSpec] = &[
    FeatureProfileSpec {
        canonical: "ga-lock",
        aliases: GA_LOCK_ALIASES,
        description: "Conservative GA-lock profile for minimal runtime surface.",
    },
    FeatureProfileSpec {
        canonical: "production",
        aliases: PRODUCTION_ALIASES,
        description: "Production profile for normal runtime feature set.",
    },
    FeatureProfileSpec {
        canonical: "all",
        aliases: ALL_ALIASES,
        description: "All in-tree features enabled for snapshot and testing.",
    },
];

/// Return canonical feature profile descriptors for tooling.
pub const fn feature_profile_specs() -> &'static [FeatureProfileSpec] {
    FEATURE_PROFILE_SPECS
}

/// Auto-generated feature catalog from `features.toml`.
#[allow(dead_code, clippy::all, missing_docs)]
pub mod catalog {
    include!(concat!(env!("OUT_DIR"), "/feature_contracts.rs"));
}

/// Human-readable BDD-oriented feature row for automation and reporting.
#[derive(Debug, Clone, Serialize)]
pub struct BddFeatureRow {
    /// Canonical feature identifier (e.g., `lsp.completion`).
    pub id: &'static str,
    /// LSP specification section this feature implements.
    pub spec: &'static str,
    /// Feature area grouping (e.g., `text_document`, `workspace`).
    pub area: &'static str,
    /// Maturity level: `ga`, `beta`, `alpha`, or `planned`.
    pub maturity: &'static str,
    /// Whether this feature is advertised to clients.
    pub advertised: bool,
    /// Whether this feature counts toward compliance percentage.
    pub counts_in_coverage: bool,
    /// Short human-readable description of the feature.
    pub description: &'static str,
    /// Test identifiers that verify this feature.
    pub tests: &'static [&'static str],
}

pub use catalog::{
    Feature, LSP_VERSION, VERSION, advertised_features, compliance_percent, has_feature,
};

/// All discovered LSP features in canonical declaration order.
pub fn all_features() -> &'static [Feature] {
    catalog::ALL_FEATURES
}

/// Export feature rows suitable for BDD matrices and acceptance criteria tooling.
pub fn bdd_feature_rows() -> Vec<BddFeatureRow> {
    let mut rows = all_features()
        .iter()
        .map(|feature| BddFeatureRow {
            id: feature.id,
            spec: feature.spec,
            area: feature.area,
            maturity: feature.maturity,
            advertised: feature.advertised,
            counts_in_coverage: feature.counts_in_coverage,
            description: feature.description,
            tests: feature.tests,
        })
        .collect::<Vec<_>>();

    rows.sort_by(|a, b| a.area.cmp(b.area).then(a.id.cmp(b.id)));
    rows
}

/// Number of BDD rows that participate in coverage accounting.
pub fn trackable_feature_count_for_grid() -> usize {
    all_features()
        .iter()
        .filter(|feature| feature.maturity != "planned" && feature.counts_in_coverage)
        .count()
}

/// Number of advertised BDD rows that participate in coverage accounting.
pub fn advertised_trackable_feature_count_for_grid() -> usize {
    all_features()
        .iter()
        .filter(|feature| {
            feature.maturity != "planned" && feature.counts_in_coverage && feature.advertised
        })
        .count()
}

/// Compliance percentage for the BDD grid (`advertised / trackable`, rounded).
pub fn compliance_percent_for_grid() -> f32 {
    let trackable = trackable_feature_count_for_grid();
    if trackable == 0 {
        return 0.0;
    }
    let advertised = advertised_trackable_feature_count_for_grid();
    (advertised as f64 / trackable as f64 * 100.0).round() as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .map(Path::to_path_buf)
            .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf())
    }

    // ── FeatureProfileKind ──────────────────────────────────────────

    #[test]
    fn from_ga_lock_enabled_true_yields_ga_lock() {
        assert_eq!(FeatureProfileKind::from_ga_lock_enabled(true), FeatureProfileKind::GaLock);
    }

    #[test]
    fn from_ga_lock_enabled_false_yields_production() {
        assert_eq!(FeatureProfileKind::from_ga_lock_enabled(false), FeatureProfileKind::Production,);
    }

    #[test]
    fn all_profiles_returns_three_variants() {
        let all = FeatureProfileKind::all();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0], FeatureProfileKind::GaLock);
        assert_eq!(all[1], FeatureProfileKind::Production);
        assert_eq!(all[2], FeatureProfileKind::All);
    }

    #[test]
    fn from_str_name_rejects_unknown_token() {
        assert!(FeatureProfileKind::from_str_name("bogus").is_none());
        assert!(FeatureProfileKind::from_str_name("").is_none());
        assert!(FeatureProfileKind::from_str_name("GA-LOCK").is_none());
    }

    #[test]
    fn aliases_are_non_empty_for_every_profile() {
        for profile in FeatureProfileKind::all() {
            assert!(
                !profile.aliases().is_empty(),
                "aliases for {} should not be empty",
                profile.as_str()
            );
        }
    }

    #[test]
    fn aliases_contain_canonical_name() {
        for profile in FeatureProfileKind::all() {
            let aliases = profile.aliases();
            assert!(
                aliases.contains(&profile.as_str()),
                "aliases for {} should contain canonical name",
                profile.as_str()
            );
        }
    }

    #[test]
    fn supported_cli_profiles_covers_all_aliases() {
        let cli_tokens = FeatureProfileKind::supported_cli_profiles();
        for profile in FeatureProfileKind::all() {
            for alias in profile.aliases() {
                assert!(
                    cli_tokens.contains(alias),
                    "CLI tokens should include alias '{}' for profile '{}'",
                    alias,
                    profile.as_str()
                );
            }
        }
    }

    #[test]
    fn auto_token_resolves_to_current() {
        let resolved = FeatureProfileKind::from_str_name("auto");
        assert_eq!(resolved, Some(FeatureProfileKind::current()));
    }

    // ── FeatureProfileSpec ──────────────────────────────────────────

    #[test]
    fn feature_profile_specs_has_three_entries() {
        let specs = feature_profile_specs();
        assert_eq!(specs.len(), 3);
    }

    #[test]
    fn feature_profile_specs_canonical_names_match_enum() {
        let specs = feature_profile_specs();
        let expected_names: Vec<&str> =
            FeatureProfileKind::all().iter().map(|p| p.as_str()).collect();
        let spec_names: Vec<&str> = specs.iter().map(|s| s.canonical).collect();
        assert_eq!(spec_names, expected_names);
    }

    #[test]
    fn feature_profile_specs_descriptions_are_non_empty() {
        for spec in feature_profile_specs() {
            assert!(
                !spec.description.is_empty(),
                "description for '{}' should not be empty",
                spec.canonical
            );
        }
    }

    // ── Catalog / BDD grid ──────────────────────────────────────────

    #[test]
    fn all_features_is_non_empty() {
        assert!(!all_features().is_empty());
    }

    #[test]
    fn all_features_have_non_empty_ids() {
        for feature in all_features() {
            assert!(!feature.id.is_empty(), "feature id should not be empty");
        }
    }

    #[test]
    fn all_features_have_valid_areas() {
        let valid_areas = ["text_document", "workspace", "window", "notebook", "debug", "protocol"];
        for feature in all_features() {
            assert!(
                valid_areas.contains(&feature.area),
                "feature '{}' has unexpected area '{}'",
                feature.id,
                feature.area
            );
        }
    }

    #[test]
    fn all_features_have_valid_maturity() {
        let valid_maturities = ["ga", "beta", "alpha", "planned"];
        for feature in all_features() {
            assert!(
                valid_maturities.contains(&feature.maturity),
                "feature '{}' has unexpected maturity '{}'",
                feature.id,
                feature.maturity
            );
        }
    }

    #[test]
    fn feature_ids_are_unique() {
        let ids: Vec<&str> = all_features().iter().map(|f| f.id).collect();
        let mut deduped = ids.clone();
        deduped.sort_unstable();
        deduped.dedup();
        assert_eq!(ids.len(), deduped.len(), "feature IDs must be unique");
    }

    #[test]
    fn bdd_feature_rows_sorted_by_area_then_id() {
        let rows = bdd_feature_rows();
        for window in rows.windows(2) {
            let ordering = window[0].area.cmp(window[1].area).then(window[0].id.cmp(window[1].id));
            assert!(
                ordering.is_le(),
                "BDD rows not sorted: '{}' in '{}' should come before '{}' in '{}'",
                window[0].id,
                window[0].area,
                window[1].id,
                window[1].area,
            );
        }
    }

    #[test]
    fn bdd_feature_rows_count_matches_all_features() {
        assert_eq!(bdd_feature_rows().len(), all_features().len());
    }

    #[test]
    fn lsp_features_have_bdd_test_receipts() {
        for feature in all_features().iter().filter(|feature| feature.id.starts_with("lsp.")) {
            assert!(
                !feature.tests.is_empty(),
                "LSP feature '{}' must include at least one test receipt for BDD grid reporting",
                feature.id
            );
        }
    }

    #[test]
    fn lsp_feature_test_receipts_exist_in_repo() {
        let root = repo_root();
        for feature in all_features().iter().filter(|feature| feature.id.starts_with("lsp.")) {
            for test_path in feature.tests {
                let exists = root.join(test_path).exists();
                assert!(
                    exists,
                    "feature '{}' references missing test receipt path '{}'",
                    feature.id, test_path
                );
            }
        }
    }

    #[test]
    fn bdd_rows_preserve_lsp_test_receipts() {
        for row in bdd_feature_rows().iter().filter(|row| row.id.starts_with("lsp.")) {
            let source = all_features().iter().find(|feature| feature.id == row.id);
            assert!(source.is_some(), "BDD row '{}' should map back to a catalog feature", row.id);
            if let Some(source) = source {
                assert_eq!(
                    row.tests, source.tests,
                    "BDD row '{}' should preserve test receipts from catalog entry",
                    row.id
                );
            }
        }
    }

    #[test]
    fn trackable_features_are_subset_of_all() {
        let all_count = all_features().len();
        let trackable = trackable_feature_count_for_grid();
        assert!(trackable <= all_count);
    }

    #[test]
    fn advertised_trackable_is_subset_of_trackable() {
        let trackable = trackable_feature_count_for_grid();
        let advertised = advertised_trackable_feature_count_for_grid();
        assert!(advertised <= trackable);
    }

    #[test]
    fn compliance_percent_is_in_valid_range() {
        let pct = compliance_percent_for_grid();
        assert!((0.0..=100.0).contains(&pct), "compliance must be 0-100, got {pct}");
    }

    #[test]
    fn has_feature_returns_true_for_known_ids() {
        assert!(has_feature("lsp.completion"));
        assert!(has_feature("lsp.hover"));
        assert!(has_feature("lsp.definition"));
    }

    #[test]
    fn has_feature_returns_false_for_unknown_ids() {
        assert!(!has_feature("lsp.nonexistent"));
        assert!(!has_feature(""));
    }

    #[test]
    fn advertised_features_is_non_empty() {
        assert!(!advertised_features().is_empty());
    }

    #[test]
    fn version_strings_are_non_empty() {
        assert!(!VERSION.is_empty());
        assert!(!LSP_VERSION.is_empty());
    }
}
