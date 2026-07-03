//! Quality-gate receipt reader.
//!
//! Sources `quality.no_new_severe_gaps` from the `quality_gate` receipt written
//! by `cargo xtask quality-gate` (default
//! `target/receipts/quality/quality-gate.json`). The receipt's top-level
//! `decision` field is `"pass"` iff no blocking coverage/ripr next-action is
//! present.

use std::path::Path;

use serde::Deserialize;

use crate::evidence::Outcome;
use crate::indicator::{EvidenceRef, IndicatorStatus};

/// Minimal projection of the quality-gate receipt.
#[derive(Debug, Deserialize)]
struct QualityGateReceipt {
    #[serde(default)]
    decision: String,
    #[serde(default)]
    head: String,
}

const FIX: &str = "Run `cargo xtask quality-gate` and resolve any blocking coverage or ripr \
                   next-actions before merge.";

/// `quality.no_new_severe_gaps`.
pub(crate) fn no_new_severe_gaps(path: Option<&Path>, expected_commit: &str) -> Outcome {
    let Some(path) = path else {
        return Outcome::unverified(
            vec![EvidenceRef::command("cargo xtask quality-gate")],
            format!("Provide the quality-gate receipt. {FIX}"),
        );
    };
    let display = path.display().to_string();
    let receipt_ev = EvidenceRef::receipt(display.clone());

    let receipt: QualityGateReceipt =
        match std::fs::read_to_string(path).ok().and_then(|t| serde_json::from_str(&t).ok()) {
            Some(r) => r,
            None => {
                return Outcome::unverified(
                    vec![receipt_ev],
                    format!("Quality-gate receipt at {display} is missing or unparseable. {FIX}"),
                );
            }
        };

    let decision = receipt.decision.trim().to_ascii_lowercase();
    let mut status = match decision.as_str() {
        "pass" => IndicatorStatus::Pass,
        "fail" => IndicatorStatus::Fail,
        _ => IndicatorStatus::Unverified,
    };

    let mut evidence = vec![receipt_ev, EvidenceRef::new("decision", decision)];

    let stale = !expected_commit.is_empty()
        && expected_commit != "unknown"
        && !receipt.head.is_empty()
        && receipt.head != expected_commit;
    if stale {
        evidence.push(EvidenceRef::new(
            "note",
            format!("stale receipt: head {} != HEAD {}", receipt.head, expected_commit),
        ));
        if status == IndicatorStatus::Pass {
            status = IndicatorStatus::Warn;
        }
    }

    match status {
        IndicatorStatus::Pass => Outcome::pass(evidence),
        IndicatorStatus::Warn => Outcome::warn(evidence, FIX.to_string()),
        IndicatorStatus::Fail => Outcome::fail(evidence, FIX.to_string()),
        _ => Outcome::unverified(evidence, FIX.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write(decision: &str, head: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().expect("tmp");
        let path = dir.path().join("quality-gate.json");
        fs::write(
            &path,
            format!(
                "{{\"kind\":\"quality_gate\",\"decision\":\"{decision}\",\"head\":\"{head}\"}}"
            ),
        )
        .expect("write");
        (dir, path)
    }

    #[test]
    fn missing_is_unverified() {
        assert_eq!(no_new_severe_gaps(None, "").status, IndicatorStatus::Unverified);
    }

    #[test]
    fn pass_decision_passes() {
        let (_d, p) = write("pass", "abc");
        assert_eq!(no_new_severe_gaps(Some(&p), "abc").status, IndicatorStatus::Pass);
    }

    #[test]
    fn fail_decision_fails() {
        let (_d, p) = write("fail", "abc");
        assert_eq!(no_new_severe_gaps(Some(&p), "abc").status, IndicatorStatus::Fail);
    }

    #[test]
    fn stale_pass_downgrades_to_warn() {
        let (_d, p) = write("pass", "oldsha");
        assert_eq!(no_new_severe_gaps(Some(&p), "newsha").status, IndicatorStatus::Warn);
    }

    #[test]
    fn unknown_expected_commit_does_not_downgrade() {
        let (_d, p) = write("pass", "realsha");
        assert_eq!(no_new_severe_gaps(Some(&p), "unknown").status, IndicatorStatus::Pass);
    }
}
