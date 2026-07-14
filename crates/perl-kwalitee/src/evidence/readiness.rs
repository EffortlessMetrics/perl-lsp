//! Native-tooling readiness receipt reader.
//!
//! Sources the `formatter.native_default` and `critic.native_default`
//! indicators from the `native_tooling_readiness` receipt written by
//! `cargo xtask native-tooling readiness` (default
//! `target/receipts/native-tooling/readiness.json`). The receipt carries a list
//! of readiness criteria, each with an `area`, `name`, and `status`
//! (`ready`/`warning`/`blocked`/`unverified`).

use std::path::Path;

use serde::Deserialize;

use crate::evidence::Outcome;
use crate::indicator::{EvidenceRef, IndicatorStatus};

/// Minimal projection of the native-tooling readiness receipt.
#[derive(Debug, Deserialize)]
struct ReadinessReceipt {
    #[serde(default)]
    commit: String,
    #[serde(default)]
    criteria: Vec<Criterion>,
}

#[derive(Debug, Deserialize)]
struct Criterion {
    #[serde(default)]
    area: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    status: String,
}

/// Map a readiness criterion status string to an indicator status.
fn map_status(s: &str) -> IndicatorStatus {
    match s.trim().to_ascii_lowercase().as_str() {
        "ready" | "pass" => IndicatorStatus::Pass,
        "warning" | "warn" | "provisional" => IndicatorStatus::Warn,
        "blocked" | "blocker" | "fail" => IndicatorStatus::Fail,
        _ => IndicatorStatus::Unverified,
    }
}

/// Read the receipt at `path`, returning `None` on any error.
fn read(path: &Path) -> Option<ReadinessReceipt> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

/// Shared implementation: find the native-default criterion for `area`.
fn native_default(path: Option<&Path>, expected_commit: &str, area: &str, fix: &str) -> Outcome {
    let Some(path) = path else {
        return Outcome::unverified(
            vec![EvidenceRef::command("cargo xtask native-tooling readiness")],
            format!("Provide the native-tooling readiness receipt; then {fix}"),
        );
    };
    let display = path.display().to_string();
    let receipt_ev = EvidenceRef::receipt(display.clone());

    let Some(receipt) = read(path) else {
        return Outcome::unverified(
            vec![receipt_ev],
            format!("Readiness receipt at {display} is missing or unparseable. {fix}"),
        );
    };

    // Find the native-default criterion for this area.
    let criterion = receipt.criteria.iter().find(|c| {
        c.area.eq_ignore_ascii_case(area) && c.name.to_ascii_lowercase().contains("default")
    });

    let Some(criterion) = criterion else {
        return Outcome::unverified(
            vec![receipt_ev],
            format!("No {area} native-default criterion found in the readiness receipt. {fix}"),
        );
    };

    let mut status = map_status(&criterion.status);
    let mut evidence = vec![receipt_ev, EvidenceRef::new("criterion", criterion.name.clone())];

    // Freshness: a receipt from a different commit is not trustworthy as a pass.
    let stale = !expected_commit.is_empty()
        && expected_commit != "unknown"
        && !receipt.commit.is_empty()
        && receipt.commit != expected_commit;
    if stale {
        evidence.push(EvidenceRef::new(
            "note",
            format!("stale receipt: commit {} != HEAD {}", receipt.commit, expected_commit),
        ));
        if status == IndicatorStatus::Pass {
            status = IndicatorStatus::Warn;
        }
    }

    match status {
        IndicatorStatus::Pass => Outcome::pass(evidence),
        IndicatorStatus::Warn => Outcome::warn(evidence, fix.to_string()),
        IndicatorStatus::Fail => Outcome::fail(evidence, fix.to_string()),
        _ => Outcome::unverified(evidence, fix.to_string()),
    }
}

/// `formatter.native_default`.
pub(crate) fn formatter_native_default(path: Option<&Path>, expected_commit: &str) -> Outcome {
    native_default(
        path,
        expected_commit,
        "formatter",
        "Run `cargo xtask native-tooling readiness` and confirm the formatter native-default criterion is ready.",
    )
}

/// `critic.native_default`.
pub(crate) fn critic_native_default(path: Option<&Path>, expected_commit: &str) -> Outcome {
    native_default(
        path,
        expected_commit,
        "critic",
        "Run `cargo xtask native-tooling readiness` and confirm the critic native-default criterion is ready.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write_receipt(commit: &str, criteria: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().expect("tmp");
        let path = dir.path().join("readiness.json");
        let body = format!(
            "{{\"kind\":\"native_tooling_readiness\",\"commit\":\"{commit}\",\"criteria\":{criteria}}}"
        );
        fs::write(&path, body).expect("write");
        (dir, path)
    }

    #[test]
    fn missing_path_is_unverified() {
        assert_eq!(formatter_native_default(None, "").status, IndicatorStatus::Unverified);
    }

    #[test]
    fn ready_formatter_criterion_passes() {
        let (_d, path) = write_receipt(
            "abc",
            "[{\"area\":\"formatter\",\"name\":\"native-default engine\",\"status\":\"ready\"}]",
        );
        assert_eq!(formatter_native_default(Some(&path), "abc").status, IndicatorStatus::Pass);
    }

    #[test]
    fn blocked_critic_criterion_fails() {
        let (_d, path) = write_receipt(
            "abc",
            "[{\"area\":\"critic\",\"name\":\"native default\",\"status\":\"blocked\"}]",
        );
        assert_eq!(critic_native_default(Some(&path), "abc").status, IndicatorStatus::Fail);
    }

    #[test]
    fn stale_receipt_downgrades_pass_to_warn() {
        let (_d, path) = write_receipt(
            "oldsha",
            "[{\"area\":\"formatter\",\"name\":\"native-default engine\",\"status\":\"ready\"}]",
        );
        assert_eq!(formatter_native_default(Some(&path), "newsha").status, IndicatorStatus::Warn);
    }

    #[test]
    fn unknown_expected_commit_does_not_downgrade() {
        // When git is unavailable the wrapper passes "unknown"; that must not be
        // treated as a real HEAD and downgrade a ready criterion to warn.
        let (_d, path) = write_receipt(
            "realsha",
            "[{\"area\":\"formatter\",\"name\":\"native-default engine\",\"status\":\"ready\"}]",
        );
        assert_eq!(formatter_native_default(Some(&path), "unknown").status, IndicatorStatus::Pass);
    }

    #[test]
    fn missing_criterion_is_unverified() {
        let (_d, path) = write_receipt("abc", "[]");
        assert_eq!(
            formatter_native_default(Some(&path), "abc").status,
            IndicatorStatus::Unverified
        );
    }
}
