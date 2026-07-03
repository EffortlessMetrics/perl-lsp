//! Evidence adapters: the pure functions that turn repository state (manifests,
//! first-mile surfaces, receipt files) into an indicator [`Outcome`].
//!
//! Every adapter is filesystem-only and deterministic — no process spawning, no
//! network. Heavier checks that genuinely need to run a subprocess (release
//! archive validation, the runCritic parity test, `update-status --check`) are
//! supplied to the evaluator from outside as
//! [`ExternalResult`](crate::ExternalResult)s; the crate never shells out.

pub(crate) mod cargo_manifest;
pub(crate) mod dap;
pub(crate) mod nightly;
pub(crate) mod product_surface;
pub(crate) mod quality_gate;
pub(crate) mod readiness;

use crate::indicator::{EvidenceRef, IndicatorStatus};

/// The raw result of evaluating one indicator, before it is combined with its
/// catalog metadata (id/area/title/weight/mandatory).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Outcome {
    pub status: IndicatorStatus,
    pub evidence: Vec<EvidenceRef>,
    pub remediation: Option<String>,
}

impl Outcome {
    pub fn pass(evidence: Vec<EvidenceRef>) -> Self {
        Outcome { status: IndicatorStatus::Pass, evidence, remediation: None }
    }

    pub fn fail(evidence: Vec<EvidenceRef>, remediation: impl Into<String>) -> Self {
        Outcome { status: IndicatorStatus::Fail, evidence, remediation: Some(remediation.into()) }
    }

    pub fn warn(evidence: Vec<EvidenceRef>, remediation: impl Into<String>) -> Self {
        Outcome { status: IndicatorStatus::Warn, evidence, remediation: Some(remediation.into()) }
    }

    pub fn unverified(evidence: Vec<EvidenceRef>, remediation: impl Into<String>) -> Self {
        Outcome {
            status: IndicatorStatus::Unverified,
            evidence,
            remediation: Some(remediation.into()),
        }
    }
}
