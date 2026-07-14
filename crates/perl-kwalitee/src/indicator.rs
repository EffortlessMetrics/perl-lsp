//! Kwalitee indicator model and the static indicator catalog.
//!
//! An *indicator* is one measurable distribution-quality claim (e.g. "release
//! archives bundle no external Perl tooling"). Each indicator carries a stable
//! [`id`](KwaliteeIndicator::id), an evaluated [`status`](IndicatorStatus), the
//! [`evidence`](KwaliteeIndicator::evidence) that backs the status, and — when
//! it did not pass — a [`remediation`](KwaliteeIndicator::remediation) hint.
//!
//! The catalog ([`catalog`]) is the single source of truth for *what* is
//! evaluated. The evaluator ([`crate::evaluate`]) turns each [`IndicatorSpec`]
//! into a concrete [`KwaliteeIndicator`] for a given repository and profile.

use serde::{Deserialize, Serialize};

/// Outcome of a single indicator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IndicatorStatus {
    /// The indicator's requirement is met.
    Pass,
    /// The indicator's requirement is not met.
    Fail,
    /// A soft/advisory concern; does not by itself fail a mandatory gate.
    Warn,
    /// Out of scope for the current profile (e.g. release archives under `pr`).
    NotApplicable,
    /// Evidence to decide the indicator was not available.
    Unverified,
}

impl IndicatorStatus {
    /// Lowercase wire/display name.
    pub fn as_str(self) -> &'static str {
        match self {
            IndicatorStatus::Pass => "pass",
            IndicatorStatus::Fail => "fail",
            IndicatorStatus::Warn => "warn",
            IndicatorStatus::NotApplicable => "not_applicable",
            IndicatorStatus::Unverified => "unverified",
        }
    }

    /// Whether the indicator participates in scoring (everything except
    /// [`NotApplicable`](IndicatorStatus::NotApplicable)).
    pub fn is_applicable(self) -> bool {
        !matches!(self, IndicatorStatus::NotApplicable)
    }
}

/// A single piece of evidence backing an indicator's status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceRef {
    /// Evidence category. An open set; the values the evaluator emits are
    /// `"command"`, `"receipt"`, `"file"`, `"test"`, `"criterion"`,
    /// `"decision"`, and `"note"` (free-form context such as an error message).
    pub kind: String,
    /// The concrete pointer (a command line, receipt path, file:line, etc.).
    pub value: String,
}

impl EvidenceRef {
    /// Convenience constructor.
    pub fn new(kind: impl Into<String>, value: impl Into<String>) -> Self {
        EvidenceRef { kind: kind.into(), value: value.into() }
    }

    /// A `command` evidence reference.
    pub fn command(value: impl Into<String>) -> Self {
        EvidenceRef::new("command", value)
    }

    /// A `receipt` evidence reference.
    pub fn receipt(value: impl Into<String>) -> Self {
        EvidenceRef::new("receipt", value)
    }

    /// A `file` evidence reference.
    pub fn file(value: impl Into<String>) -> Self {
        EvidenceRef::new("file", value)
    }
}

/// One evaluated indicator in a [`KwaliteeReceipt`](crate::KwaliteeReceipt).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KwaliteeIndicator {
    /// Stable dotted identifier, e.g. `release.no_external_tooling`.
    pub id: String,
    /// Coarse grouping, e.g. `release`, `product_surface`, `critic`.
    pub area: String,
    /// Human-readable one-line title.
    pub title: String,
    /// Whether a non-pass on this indicator blocks the mandatory gate.
    pub mandatory: bool,
    /// Evaluated outcome.
    pub status: IndicatorStatus,
    /// Relative weight in the numeric score (0..=100 band, per area).
    pub score_weight: u8,
    /// Evidence backing the status.
    pub evidence: Vec<EvidenceRef>,
    /// How to fix a non-pass, when known.
    pub remediation: Option<String>,
}

/// How an indicator's status is obtained.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EvalSource {
    /// The crate computes the status from the repository filesystem alone.
    Native,
    /// The crate reads the native-tooling readiness receipt.
    ReadinessReceipt,
    /// The crate reads the quality-gate receipt.
    QualityGateReceipt,
    /// The crate reads one of the nightly receipt-backed advisory indicators.
    NightlyReceipt,
    /// The caller (e.g. xtask) supplies the result by running a heavier gate.
    External,
}

/// Which profiles an indicator applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IndicatorScope {
    /// Evaluated on every profile.
    All,
    /// Only under the release profile (release archives are not present for
    /// `pr`/`nightly`).
    ReleaseOnly,
    /// Only under the nightly profile (broad, receipt-heavy advisory rows).
    NightlyOnly,
}

/// A static catalog entry describing one indicator independent of any repo.
#[derive(Debug, Clone, Copy)]
pub(crate) struct IndicatorSpec {
    pub id: &'static str,
    pub area: &'static str,
    pub title: &'static str,
    pub mandatory: bool,
    pub weight: u8,
    pub source: EvalSource,
    /// Which profiles this indicator applies to.
    pub scope: IndicatorScope,
    pub remediation: &'static str,
    /// Long-form explanation surfaced by `explain <id>`.
    pub rationale: &'static str,
}

impl IndicatorSpec {
    /// Whether this indicator is release-scoped (kept for the area invariant test).
    pub fn is_release_only(&self) -> bool {
        matches!(self.scope, IndicatorScope::ReleaseOnly)
    }
}

/// The full indicator catalog, in report order.
///
/// This is the single source of truth for the indicator set. Weights are
/// grouped by area so the numeric score reflects distribution-quality
/// priorities (release + product surface + native tooling dominate).
pub(crate) const CATALOG: &[IndicatorSpec] = &[
    IndicatorSpec {
        id: "manifest.workspace_member_declared",
        area: "manifest",
        title: "perl-kwalitee is a declared workspace member",
        mandatory: true,
        weight: 3,
        source: EvalSource::Native,
        scope: IndicatorScope::All,
        remediation: "Add \"crates/perl-kwalitee\" to [workspace].members in the root Cargo.toml.",
        rationale: "The evaluator crate must itself be part of the workspace so it \
                    builds, tests, and lints under the normal gates rather than \
                    drifting as an out-of-tree script.",
    },
    IndicatorSpec {
        id: "manifest.publish_policy_clean",
        area: "manifest",
        title: "publish policy is intentional",
        mandatory: true,
        weight: 4,
        source: EvalSource::Native,
        scope: IndicatorScope::All,
        remediation: "Set `publish = false` in crates/perl-kwalitee/Cargo.toml while the \
                      schema stabilizes, or add the crate to \
                      [workspace.metadata.publish].allow once it is publishable.",
        rationale: "Publishability must be a deliberate choice: either explicitly \
                    private (publish = false) or explicitly allowlisted. An \
                    ambiguous state risks an accidental publish or a manifest-check \
                    failure.",
    },
    IndicatorSpec {
        id: "license.declared",
        area: "license",
        title: "crate declares license metadata",
        mandatory: true,
        weight: 3,
        source: EvalSource::Native,
        scope: IndicatorScope::All,
        remediation: "Add `license.workspace = true` (or an explicit SPDX license) to \
                      crates/perl-kwalitee/Cargo.toml.",
        rationale: "Every shippable crate needs license metadata; publish-manifest-check \
                    enforces this for allowlisted crates, and downstream consumers rely \
                    on it.",
    },
    IndicatorSpec {
        id: "product_surface.native_only",
        area: "product_surface",
        title: "first-mile surfaces stay native-only",
        mandatory: true,
        weight: 15,
        source: EvalSource::Native,
        scope: IndicatorScope::All,
        remediation: "Move any external-tool/legacy-bridge product framing off the \
                      first-mile surfaces into docs/reference/ (e.g. \
                      DAP_LEGACY_BRIDGE_COMPAT.md).",
        rationale: "The product ships the native stack. The surfaces users first read \
                    must not claim the product *requires* perltidy/perlcritic or a \
                    Perl::LanguageServer bridge. Mirrors `cargo xtask \
                    check-native-product-surface`.",
    },
    IndicatorSpec {
        id: "dap.cli_native_only",
        area: "dap",
        title: "shipped perl-dap CLI stays native-only",
        mandatory: true,
        weight: 7,
        source: EvalSource::Native,
        scope: IndicatorScope::All,
        remediation: "Remove the `--bridge` flag from the shipped perl-dap CLI; bridge mode is a \
                      library-only path, not a product surface.",
        rationale: "The legacy `--bridge` proxy to Perl::LanguageServer was removed from the \
                    shipped perl-dap CLI (#3277). Reintroducing it as a product flag would put \
                    an external-tool path back on the product surface.",
    },
    IndicatorSpec {
        id: "release.native_binaries_present",
        area: "release",
        title: "release archives contain the native binaries",
        mandatory: true,
        weight: 7,
        source: EvalSource::External,
        scope: IndicatorScope::ReleaseOnly,
        remediation: "Run `cargo xtask release artifact-check --dist <dir>` and add the \
                      missing native binaries to the release archives.",
        rationale: "The shipped product is the native stack; a release that omits the \
                    native binaries is not shippable.",
    },
    IndicatorSpec {
        id: "release.no_external_tooling",
        area: "release",
        title: "release archives bundle no external Perl tooling",
        mandatory: true,
        weight: 8,
        source: EvalSource::External,
        scope: IndicatorScope::ReleaseOnly,
        remediation: "Remove perltidy/perlcritic/Perl::LanguageServer/TSPerlDAP payloads \
                      from the release archives; external tools are conformance-only.",
        rationale: "If it is not native, we do not ship it. Bundling external tooling \
                    contradicts the native-only product promise. Mirrors the negative \
                    contract in `release artifact-check`.",
    },
    IndicatorSpec {
        id: "release.checksums_valid",
        area: "release",
        title: "consolidated checksums are present and valid",
        mandatory: true,
        weight: 5,
        source: EvalSource::External,
        scope: IndicatorScope::ReleaseOnly,
        remediation: "Ensure every archive is listed in the consolidated SHA256SUMS and \
                      each digest matches the file on disk.",
        rationale: "Checksums are the integrity contract for released artifacts; a \
                    missing or mismatched digest breaks verifiable distribution.",
    },
    IndicatorSpec {
        id: "formatter.native_default",
        area: "formatter",
        title: "formatter defaults to the native engine",
        mandatory: true,
        weight: 10,
        source: EvalSource::ReadinessReceipt,
        scope: IndicatorScope::All,
        remediation: "Run `cargo xtask native-tooling readiness` and confirm the \
                      formatter native-default criterion is ready.",
        rationale: "Formatting must work out of the box without external perltidy; the \
                    native formatter is the default engine.",
    },
    IndicatorSpec {
        id: "critic.native_default",
        area: "critic",
        title: "critic defaults to the native engine",
        mandatory: true,
        weight: 8,
        source: EvalSource::ReadinessReceipt,
        scope: IndicatorScope::All,
        remediation: "Run `cargo xtask native-tooling readiness` and confirm the critic \
                      native-default criterion is ready.",
        rationale: "Linting must work out of the box without external perlcritic; the \
                    native critic registry is the default engine.",
    },
    IndicatorSpec {
        id: "critic.run_critic_registry_parity",
        area: "critic",
        title: "perl.runCritic matches editor native diagnostics",
        mandatory: true,
        weight: 7,
        source: EvalSource::External,
        scope: IndicatorScope::All,
        remediation: "Run `cargo test -p perl-lsp-rs --lib \
                      execute_command::tests::run_critic_native_matches_pull_diagnostics_registry` \
                      and resolve the parity failure.",
        rationale: "The `perl.runCritic` command and on-type native pull diagnostics must \
                    agree; a divergence is a user-visible inconsistency. #3303 landed the \
                    NativeCriticRegistry routing plus the parity test \
                    (run_critic_native_matches_pull_diagnostics_registry), so this is now a \
                    mandatory gate rather than an advisory one.",
    },
    IndicatorSpec {
        id: "quality.no_new_severe_gaps",
        area: "quality",
        title: "no new severe coverage/ripr regressions",
        mandatory: true,
        weight: 15,
        source: EvalSource::QualityGateReceipt,
        scope: IndicatorScope::All,
        remediation: "Run `cargo xtask quality-gate` and resolve any blocking coverage or \
                      ripr next-actions before merge.",
        rationale: "The quality gate aggregates patch coverage and ripr proof receipts; a \
                    blocking decision means a severe regression is riding along.",
    },
    IndicatorSpec {
        id: "docs.status_current",
        area: "docs",
        title: "generated status docs are current",
        mandatory: true,
        weight: 5,
        source: EvalSource::External,
        scope: IndicatorScope::All,
        remediation: "Run `cargo xtask update-status --check`; regenerate with \
                      `--write` if drift is reported.",
        rationale: "The status docs are computed truth sources; stale generated docs \
                    misreport the project state to users and downstream tooling.",
    },
    // ----- Nightly-only advisory indicators (broad, receipt-heavy) -----
    IndicatorSpec {
        id: "formatter.corpus_idempotent",
        area: "formatter",
        title: "native formatter is idempotent + parse-preserving over the corpus",
        mandatory: false,
        weight: 3,
        source: EvalSource::NightlyReceipt,
        scope: IndicatorScope::NightlyOnly,
        remediation: "Run `cargo xtask native-format corpus` and fix the files where the \
                      native formatter is not idempotent or changes the parse.",
        rationale: "A formatter that is not idempotent or alters the AST is not safe to \
                    ship as the default; the nightly corpus sweep proves it over a broad \
                    body of real Perl.",
    },
    IndicatorSpec {
        id: "critic.no_false_positives",
        area: "critic",
        title: "native critic raises no findings on the clean fixtures",
        mandatory: false,
        weight: 3,
        source: EvalSource::NightlyReceipt,
        scope: IndicatorScope::NightlyOnly,
        remediation: "Run the native-critic false-positive fixtures and eliminate any \
                      findings/parse errors on known-clean code.",
        rationale: "A critic that flags known-clean code erodes trust in the native \
                    default; the false-positive fixtures must stay clean.",
    },
    IndicatorSpec {
        id: "formatter.perltidy_compat_no_external_only",
        area: "formatter",
        title: "perltidy compatibility has no external-only gaps",
        mandatory: false,
        weight: 2,
        source: EvalSource::NightlyReceipt,
        scope: IndicatorScope::NightlyOnly,
        remediation: "Run `cargo xtask native-format perltidy-compat` and close or \
                      re-classify the external-only options.",
        rationale: "External-only perltidy options are ones the native formatter cannot \
                    honor; tracking them at zero keeps the native path a full replacement.",
    },
    IndicatorSpec {
        id: "critic.perlcritic_compat_no_external_only",
        area: "critic",
        title: "perlcritic compatibility has no external-only gaps",
        mandatory: false,
        weight: 2,
        source: EvalSource::NightlyReceipt,
        scope: IndicatorScope::NightlyOnly,
        remediation: "Run `cargo xtask native-tooling perlcritic-compat` and close or \
                      re-classify the external-only rules.",
        rationale: "External-only perlcritic rules are ones the native critic cannot \
                    cover; tracking them at zero keeps the native path a full replacement.",
    },
];

/// Look up a catalog spec by its stable id.
pub(crate) fn spec_for(id: &str) -> Option<&'static IndicatorSpec> {
    CATALOG.iter().find(|s| s.id == id)
}

/// All indicator ids, in catalog order (used by `explain` and tests).
pub fn indicator_ids() -> Vec<&'static str> {
    CATALOG.iter().map(|s| s.id).collect()
}

/// Long-form explanation for a single indicator id, if it exists.
///
/// Returns a `(title, area, mandatory, remediation, rationale)` tuple for the
/// `explain <id>` command surface.
pub fn explain(id: &str) -> Option<IndicatorExplanation> {
    spec_for(id).map(|s| IndicatorExplanation {
        id: s.id,
        area: s.area,
        title: s.title,
        mandatory: s.mandatory,
        remediation: s.remediation,
        rationale: s.rationale,
    })
}

/// Static explanation of an indicator, returned by [`explain`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndicatorExplanation {
    /// Stable dotted id.
    pub id: &'static str,
    /// Coarse grouping.
    pub area: &'static str,
    /// One-line title.
    pub title: &'static str,
    /// Whether a non-pass blocks the mandatory gate.
    pub mandatory: bool,
    /// How to fix a non-pass.
    pub remediation: &'static str,
    /// Why the indicator exists.
    pub rationale: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn catalog_ids_are_unique() {
        let mut seen = BTreeSet::new();
        for spec in CATALOG {
            assert!(seen.insert(spec.id), "duplicate indicator id: {}", spec.id);
        }
    }

    #[test]
    fn catalog_ids_are_dotted() {
        for spec in CATALOG {
            assert!(
                spec.id.contains('.'),
                "indicator id `{}` should be `area.name` dotted form",
                spec.id
            );
        }
    }

    #[test]
    fn release_only_indicators_are_in_release_area() {
        for spec in CATALOG {
            if spec.is_release_only() {
                assert_eq!(
                    spec.area, "release",
                    "{} is release_only but not release area",
                    spec.id
                );
            }
        }
    }

    #[test]
    fn every_id_is_explainable() {
        for id in indicator_ids() {
            assert!(explain(id).is_some(), "no explanation for {id}");
        }
        assert!(explain("nope.missing").is_none());
    }

    #[test]
    fn critic_registry_parity_is_mandatory() {
        // #3303 landed the NativeCriticRegistry routing plus the parity test
        // (run_critic_native_matches_pull_diagnostics_registry), so this
        // indicator is promoted from advisory to mandatory (#3309).
        let spec =
            spec_for("critic.run_critic_registry_parity").expect("indicator must be in catalog");
        assert!(spec.mandatory, "critic.run_critic_registry_parity must be mandatory");
        assert_eq!(spec.scope, IndicatorScope::All, "must still apply to every profile");
    }

    #[test]
    fn status_applicability() {
        assert!(IndicatorStatus::Pass.is_applicable());
        assert!(!IndicatorStatus::NotApplicable.is_applicable());
    }
}
