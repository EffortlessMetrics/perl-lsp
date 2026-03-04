//! Shared feature contracts for profile parsing, BDD-grid rows, and capability mapping.
//!
//! This crate defines the canonical [`FeatureProfileKind`] enum and associated
//! [`FeatureProfileSpec`] metadata used for feature-coverage reporting. It sits
//! between `perl-lsp-feature-ids` (raw identifiers) and
//! `perl-lsp-feature-policy` (runtime capability selection).

pub use perl_lsp_capability_map::{caps_from_feature_ids, feature_ids_from_caps};
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

#[allow(dead_code, clippy::all)]
pub mod catalog {
    include!(concat!(env!("OUT_DIR"), "/feature_contracts.rs"));
}

/// Human-readable BDD-oriented feature row for automation and reporting.
#[derive(Debug, Clone, Serialize)]
pub struct BddFeatureRow {
    pub id: &'static str,
    pub spec: &'static str,
    pub area: &'static str,
    pub maturity: &'static str,
    pub advertised: bool,
    pub counts_in_coverage: bool,
    pub description: &'static str,
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
