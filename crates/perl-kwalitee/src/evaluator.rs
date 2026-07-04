//! The evaluation engine: turn a repository + profile + supplied evidence into
//! a [`KwaliteeReceipt`].
//!
//! [`evaluate`] walks the static [catalog](crate::indicator) and, for each
//! indicator, obtains an [`Outcome`] from the appropriate source:
//!
//! - **Native** filesystem checks (manifests, first-mile surfaces) computed
//!   directly by the crate from [`KwaliteeOptions::repo_root`];
//! - **Receipt** readers that parse the native-tooling readiness and
//!   quality-gate JSON receipts (paths in [`EvidencePaths`]);
//! - **External** results supplied by the caller in
//!   [`KwaliteeOptions::external_results`] — used for the heavier gates the
//!   crate deliberately does not run itself (release artifact-check, the
//!   runCritic parity test, `update-status --check`).

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::evidence::{
    Outcome, cargo_manifest, dap, nightly, product_surface, quality_gate, readiness,
};
use crate::indicator::{
    CATALOG, EvalSource, EvidenceRef, IndicatorScope, IndicatorSpec, IndicatorStatus,
    KwaliteeIndicator, spec_for,
};
use crate::profile::KwaliteeProfile;
use crate::receipt::{KwaliteeReceipt, RECEIPT_KIND, SCHEMA_VERSION};
use crate::score;

/// Paths to the JSON receipts the crate reads for receipt-backed indicators.
///
/// Only the receipts the crate itself parses are listed here; heavier gates
/// (release archive check, runCritic parity, `update-status --check`) are fed in
/// through [`KwaliteeOptions::external_results`] instead, since the crate never
/// shells out.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EvidencePaths {
    /// `native_tooling_readiness` receipt
    /// (default `target/receipts/native-tooling/readiness.json`).
    pub native_tooling_readiness: Option<PathBuf>,
    /// `quality_gate` receipt
    /// (default `target/receipts/quality/quality-gate.json`).
    pub quality_gate_receipt: Option<PathBuf>,
    /// `native_format_corpus` receipt (nightly)
    /// (default `target/receipts/format/native-format-corpus.json`).
    pub native_format_corpus: Option<PathBuf>,
    /// `native_critic_check` false-positive receipt (nightly)
    /// (default `target/receipts/native-tooling/native-critic-false-positive.json`).
    pub native_critic_false_positive: Option<PathBuf>,
    /// `native_format_perltidy_compat` receipt (nightly)
    /// (default `target/receipts/format/native-format-perltidy-compat.json`).
    pub native_format_perltidy_compat: Option<PathBuf>,
    /// `native_tooling_perlcritic_compat` receipt (nightly)
    /// (default `target/receipts/native-tooling/perlcritic-compat.json`).
    pub native_tooling_perlcritic_compat: Option<PathBuf>,
}

/// A result the caller obtained by running a heavier gate, keyed into
/// [`KwaliteeOptions::external_results`] by indicator id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalResult {
    /// The evaluated status.
    pub status: IndicatorStatus,
    /// Evidence backing the status.
    pub evidence: Vec<EvidenceRef>,
    /// How to fix a non-pass, when known.
    pub remediation: Option<String>,
}

impl ExternalResult {
    /// A passing external result.
    pub fn pass(evidence: Vec<EvidenceRef>) -> Self {
        ExternalResult { status: IndicatorStatus::Pass, evidence, remediation: None }
    }

    /// A failing external result.
    pub fn fail(evidence: Vec<EvidenceRef>, remediation: impl Into<String>) -> Self {
        ExternalResult {
            status: IndicatorStatus::Fail,
            evidence,
            remediation: Some(remediation.into()),
        }
    }

    /// Convert the `Ok`/`Err` of a gate run into a pass/fail result.
    ///
    /// `evidence` typically names the command that produced the result; on the
    /// error path the error text is appended as a `note`.
    pub fn from_gate<E: std::fmt::Display>(
        result: Result<(), E>,
        mut evidence: Vec<EvidenceRef>,
        remediation: impl Into<String>,
    ) -> Self {
        match result {
            Ok(()) => ExternalResult::pass(evidence),
            Err(e) => {
                evidence.push(EvidenceRef::new("note", e.to_string()));
                ExternalResult::fail(evidence, remediation)
            }
        }
    }
}

impl From<ExternalResult> for Outcome {
    fn from(r: ExternalResult) -> Self {
        Outcome { status: r.status, evidence: r.evidence, remediation: r.remediation }
    }
}

/// Inputs to [`evaluate`].
#[derive(Debug, Clone)]
pub struct KwaliteeOptions {
    /// Workspace root.
    pub repo_root: PathBuf,
    /// Profile to evaluate.
    pub profile: KwaliteeProfile,
    /// Release `dist` directory (required to satisfy release indicators).
    pub dist_dir: Option<PathBuf>,
    /// Treat unverified mandatory indicators as failures.
    pub strict: bool,
    /// Git commit the evaluation reflects (used for receipt freshness + output).
    pub commit: String,
    /// Timestamp string recorded in the receipt (RFC 3339 recommended).
    pub generated_at: String,
    /// Receipt paths for receipt-backed indicators.
    pub evidence: EvidencePaths,
    /// Externally-supplied gate results, keyed by indicator id.
    pub external_results: BTreeMap<String, ExternalResult>,
}

impl KwaliteeOptions {
    /// Construct options for `repo_root` and `profile` with everything else
    /// defaulted (no dist, non-strict, empty evidence/results).
    pub fn new(repo_root: impl Into<PathBuf>, profile: KwaliteeProfile) -> Self {
        KwaliteeOptions {
            repo_root: repo_root.into(),
            profile,
            dist_dir: None,
            strict: false,
            commit: String::new(),
            generated_at: String::new(),
            evidence: EvidencePaths::default(),
            external_results: BTreeMap::new(),
        }
    }
}

/// Evaluate the Kwalitee indicators for `options` and return a receipt.
pub fn evaluate(options: &KwaliteeOptions) -> KwaliteeReceipt {
    let indicators: Vec<KwaliteeIndicator> =
        CATALOG.iter().map(|spec| evaluate_spec(spec, options)).collect();

    let scored = score::score(&indicators, options.strict);

    KwaliteeReceipt {
        kind: RECEIPT_KIND.to_string(),
        schema_version: SCHEMA_VERSION,
        generated_at: options.generated_at.clone(),
        commit: options.commit.clone(),
        profile: options.profile,
        score: scored.score,
        verdict: scored.verdict,
        mandatory_passed: scored.mandatory_passed,
        mandatory_failed_count: scored.mandatory_failed_count,
        mandatory_unverified_count: scored.mandatory_unverified_count,
        warning_count: scored.warning_count,
        unverified_count: scored.unverified_count,
        indicators,
    }
}

/// Whether `scope` applies under `profile`.
fn scope_applies(scope: IndicatorScope, profile: KwaliteeProfile) -> bool {
    match scope {
        IndicatorScope::All => true,
        IndicatorScope::ReleaseOnly => profile.requires_release_artifacts(),
        IndicatorScope::NightlyOnly => matches!(profile, KwaliteeProfile::Nightly),
    }
}

/// Evaluate one catalog spec into a full indicator.
fn evaluate_spec(spec: &IndicatorSpec, options: &KwaliteeOptions) -> KwaliteeIndicator {
    let outcome = if scope_applies(spec.scope, options.profile) {
        obtain_outcome(spec, options)
    } else {
        let note = match spec.scope {
            IndicatorScope::ReleaseOnly => {
                "release archives are only evaluated under the release profile"
            }
            IndicatorScope::NightlyOnly => {
                "receipt-heavy advisory indicators are only evaluated under the nightly profile"
            }
            IndicatorScope::All => "not applicable",
        };
        Outcome {
            status: IndicatorStatus::NotApplicable,
            evidence: vec![EvidenceRef::new("note", note)],
            remediation: None,
        }
    };

    KwaliteeIndicator {
        id: spec.id.to_string(),
        area: spec.area.to_string(),
        title: spec.title.to_string(),
        mandatory: spec.mandatory,
        status: outcome.status,
        score_weight: spec.weight,
        evidence: outcome.evidence,
        // Only attach remediation when the indicator did not pass; fall back to
        // the catalog default. Scoped so the default string is not allocated for
        // passing/not-applicable indicators.
        remediation: if matches!(
            outcome.status,
            IndicatorStatus::Pass | IndicatorStatus::NotApplicable
        ) {
            None
        } else {
            outcome.remediation.or_else(|| Some(spec.remediation.to_string()))
        },
    }
}

/// Obtain the raw outcome for an applicable indicator based on its source.
fn obtain_outcome(spec: &IndicatorSpec, options: &KwaliteeOptions) -> Outcome {
    let root = &options.repo_root;
    match spec.source {
        EvalSource::Native => match spec.id {
            "manifest.workspace_member_declared" => cargo_manifest::workspace_member_declared(root),
            "manifest.publish_policy_clean" => cargo_manifest::publish_policy_clean(root),
            "license.declared" => cargo_manifest::license_declared(root),
            "product_surface.native_only" => product_surface::native_only(root, options.profile),
            "dap.cli_native_only" => dap::cli_native_only(root),
            other => Outcome::unverified(
                vec![EvidenceRef::new("note", format!("no native evaluator wired for {other}"))],
                "This indicator has no native evaluator; wire one in evaluator.rs.",
            ),
        },
        EvalSource::ReadinessReceipt => {
            let path = options.evidence.native_tooling_readiness.as_deref();
            match spec.id {
                "formatter.native_default" => {
                    readiness::formatter_native_default(path, &options.commit)
                }
                "critic.native_default" => readiness::critic_native_default(path, &options.commit),
                other => Outcome::unverified(
                    vec![EvidenceRef::new("note", format!("no readiness mapping for {other}"))],
                    "This readiness indicator has no mapping; wire one in evaluator.rs.",
                ),
            }
        }
        EvalSource::QualityGateReceipt => quality_gate::no_new_severe_gaps(
            options.evidence.quality_gate_receipt.as_deref(),
            &options.commit,
        ),
        EvalSource::NightlyReceipt => {
            let ev = &options.evidence;
            match spec.id {
                "formatter.corpus_idempotent" => {
                    nightly::formatter_corpus_idempotent(ev.native_format_corpus.as_deref())
                }
                "critic.no_false_positives" => {
                    nightly::critic_no_false_positives(ev.native_critic_false_positive.as_deref())
                }
                "formatter.perltidy_compat_no_external_only" => {
                    nightly::formatter_perltidy_compat(ev.native_format_perltidy_compat.as_deref())
                }
                "critic.perlcritic_compat_no_external_only" => nightly::critic_perlcritic_compat(
                    ev.native_tooling_perlcritic_compat.as_deref(),
                ),
                other => Outcome::unverified(
                    vec![EvidenceRef::new("note", format!("no nightly mapping for {other}"))],
                    "This nightly indicator has no mapping; wire one in evaluator.rs.",
                ),
            }
        }
        EvalSource::External => external_outcome(spec, options),
    }
}

/// Resolve an external indicator from the supplied results, or fall back to a
/// profile-aware unverified/failed default.
fn external_outcome(spec: &IndicatorSpec, options: &KwaliteeOptions) -> Outcome {
    if let Some(result) = options.external_results.get(spec.id) {
        return result.clone().into();
    }

    // Release indicators under the release profile require a dist directory; if
    // one was not provided the gate cannot even run, which is a hard fail.
    if spec.is_release_only()
        && options.profile.requires_release_artifacts()
        && options.dist_dir.is_none()
    {
        return Outcome::fail(
            vec![EvidenceRef::command("cargo xtask release artifact-check --dist <dir>")],
            "The release profile requires --dist; supply a populated dist directory.",
        );
    }

    Outcome::unverified(
        vec![EvidenceRef::new("note", format!("no external result supplied for {}", spec.id))],
        spec.remediation.to_string(),
    )
}

/// Whether an indicator id exists in the catalog (used by callers to validate
/// external-result keys before evaluation).
pub fn is_known_indicator(id: &str) -> bool {
    spec_for(id).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tmp");
        let root = dir.path();
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/perl-kwalitee\"]\n\
             [workspace.metadata.publish]\nallow = []\n",
        )
        .expect("write root");
        std::fs::create_dir_all(root.join("crates/perl-kwalitee")).expect("mkdir");
        std::fs::write(
            root.join("crates/perl-kwalitee/Cargo.toml"),
            "[package]\nname = \"perl-kwalitee\"\nlicense.workspace = true\npublish = false\n",
        )
        .expect("write crate");
        // A clean first-mile surface so product_surface.native_only can report
        // Pass (an empty tree is now Unverified).
        std::fs::create_dir_all(root.join("vscode-extension")).expect("mkdir ext");
        std::fs::write(
            root.join("vscode-extension/package.json"),
            "\"description\": \"native Perl language server + debugger\"\n",
        )
        .expect("write surface");
        dir
    }

    #[test]
    fn pr_profile_marks_release_indicators_not_applicable() {
        let dir = fixture_repo();
        let opts = KwaliteeOptions::new(dir.path(), KwaliteeProfile::Pr);
        let receipt = evaluate(&opts);
        let release: Vec<_> = receipt.indicators.iter().filter(|i| i.area == "release").collect();
        assert!(!release.is_empty());
        assert!(release.iter().all(|i| i.status == IndicatorStatus::NotApplicable));
    }

    #[test]
    fn nightly_indicators_scoped_to_nightly_profile() {
        let dir = fixture_repo();
        // Under pr, nightly-only indicators are NotApplicable.
        let pr = evaluate(&KwaliteeOptions::new(dir.path(), KwaliteeProfile::Pr));
        let nightly_ids = ["formatter.corpus_idempotent", "critic.no_false_positives"];
        for id in nightly_ids {
            let ind = pr.indicators.iter().find(|i| i.id == id).expect(id);
            assert_eq!(ind.status, IndicatorStatus::NotApplicable, "pr: {id}");
        }
        // Under nightly, they are evaluated (Unverified here — no receipts).
        let nightly = evaluate(&KwaliteeOptions::new(dir.path(), KwaliteeProfile::Nightly));
        for id in nightly_ids {
            let ind = nightly.indicators.iter().find(|i| i.id == id).expect(id);
            assert_eq!(ind.status, IndicatorStatus::Unverified, "nightly: {id}");
        }
        // Nightly indicators are advisory, so their Unverified must not force a
        // Fail verdict (non-strict).
        assert_ne!(nightly.verdict, crate::KwaliteeVerdict::Pass); // docs.status drift etc.
    }

    #[test]
    fn native_manifest_indicators_pass_on_fixture() {
        let dir = fixture_repo();
        let opts = KwaliteeOptions::new(dir.path(), KwaliteeProfile::Pr);
        let receipt = evaluate(&opts);
        for id in [
            "manifest.workspace_member_declared",
            "manifest.publish_policy_clean",
            "license.declared",
            "product_surface.native_only",
        ] {
            let ind = receipt.indicators.iter().find(|i| i.id == id).expect(id);
            assert_eq!(ind.status, IndicatorStatus::Pass, "{id}");
        }
    }

    #[test]
    fn release_profile_without_dist_fails_release_indicators() {
        let dir = fixture_repo();
        let mut opts = KwaliteeOptions::new(dir.path(), KwaliteeProfile::Release);
        opts.dist_dir = None;
        let receipt = evaluate(&opts);
        let release: Vec<_> = receipt.indicators.iter().filter(|i| i.area == "release").collect();
        assert!(release.iter().all(|i| i.status == IndicatorStatus::Fail));
        assert_eq!(receipt.verdict, crate::KwaliteeVerdict::Fail);
    }

    #[test]
    fn external_result_is_honored() {
        let dir = fixture_repo();
        let mut opts = KwaliteeOptions::new(dir.path(), KwaliteeProfile::Release);
        opts.dist_dir = Some(dir.path().join("dist"));
        for id in [
            "release.native_binaries_present",
            "release.no_external_tooling",
            "release.checksums_valid",
        ] {
            opts.external_results.insert(
                id.to_string(),
                ExternalResult::pass(vec![EvidenceRef::command("release artifact-check")]),
            );
        }
        let receipt = evaluate(&opts);
        let release: Vec<_> = receipt.indicators.iter().filter(|i| i.area == "release").collect();
        assert!(release.iter().all(|i| i.status == IndicatorStatus::Pass));
    }

    #[test]
    fn critic_parity_is_mandatory_and_unverified_without_external_result() {
        // The crate never shells out, so with no external result supplied the
        // indicator resolves Unverified (not Fail) — but it is mandatory
        // (#3309), so under --strict that Unverified must drive a Fail
        // verdict, matching every other mandatory external indicator.
        let dir = fixture_repo();
        let opts = KwaliteeOptions::new(dir.path(), KwaliteeProfile::Pr);
        let receipt = evaluate(&opts);
        let ind = receipt
            .indicators
            .iter()
            .find(|i| i.id == "critic.run_critic_registry_parity")
            .expect("critic.run_critic_registry_parity");
        assert!(ind.mandatory, "must be mandatory now that #3303 landed");
        assert_eq!(ind.status, IndicatorStatus::Unverified);

        let mut strict_opts = KwaliteeOptions::new(dir.path(), KwaliteeProfile::Pr);
        strict_opts.strict = true;
        let strict_receipt = evaluate(&strict_opts);
        assert_eq!(strict_receipt.verdict, crate::KwaliteeVerdict::Fail);
    }

    #[test]
    fn critic_parity_external_pass_is_honored() {
        let dir = fixture_repo();
        let mut opts = KwaliteeOptions::new(dir.path(), KwaliteeProfile::Pr);
        opts.external_results.insert(
            "critic.run_critic_registry_parity".to_string(),
            ExternalResult::pass(vec![EvidenceRef::command(
                "cargo test -p perl-lsp-rs --lib \
                 execute_command::tests::run_critic_native_matches_pull_diagnostics_registry",
            )]),
        );
        let receipt = evaluate(&opts);
        let ind = receipt
            .indicators
            .iter()
            .find(|i| i.id == "critic.run_critic_registry_parity")
            .expect("critic.run_critic_registry_parity");
        assert_eq!(ind.status, IndicatorStatus::Pass);
    }

    #[test]
    fn unverified_mandatory_fails_under_strict() {
        let dir = fixture_repo();
        let mut opts = KwaliteeOptions::new(dir.path(), KwaliteeProfile::Pr);
        opts.strict = true;
        // No readiness/quality receipts supplied → those mandatory indicators
        // are Unverified → strict fail.
        let receipt = evaluate(&opts);
        assert_eq!(receipt.verdict, crate::KwaliteeVerdict::Fail);
    }

    #[test]
    fn passing_indicator_has_no_remediation() {
        let dir = fixture_repo();
        let opts = KwaliteeOptions::new(dir.path(), KwaliteeProfile::Pr);
        let receipt = evaluate(&opts);
        let ind = receipt.indicators.iter().find(|i| i.id == "license.declared").expect("license");
        assert!(ind.remediation.is_none());
    }
}
