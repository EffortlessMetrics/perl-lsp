use crate::utils;
use color_eyre::eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct QueueHealthConfig {
    pub fixture: Option<PathBuf>,
    pub receipt: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize)]
struct QueueHealthInput {
    master_sha: Option<String>,
    master_ci_state: Option<String>,
    pending_checks: Option<u64>,
    running_checks: Option<u64>,
    failed_checks: Option<u64>,
    failure_classifier: Option<FailureClassifier>,
    ruleset: Option<RulesetState>,
}

#[derive(Debug, Clone, Deserialize)]
struct FailureClassifier {
    shared_blocker: Option<bool>,
    summary: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct RulesetState {
    candidate_current: Option<bool>,
    gate_policy: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct QueueHealthReceipt {
    master_sha: String,
    mode: QueueHealthMode,
    allowed_lanes: Vec<String>,
    blocked_lanes: Vec<String>,
    reasons: Vec<String>,
    verdict: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum QueueHealthMode {
    Green,
    Pending,
    Red,
}

pub fn run(config: QueueHealthConfig) -> Result<()> {
    let input = load_input(config.fixture.as_ref())?;
    let receipt = evaluate_queue_health(input);
    let payload =
        serde_json::to_string_pretty(&receipt).context("serializing queue health receipt")?;

    if let Some(receipt_path) = config.receipt {
        let root = utils::project_root()?;
        let target =
            if receipt_path.is_absolute() { receipt_path } else { root.join(receipt_path) };
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating receipt directory {}", parent.display()))?;
        }
        fs::write(&target, format!("{payload}\n"))
            .with_context(|| format!("writing queue health receipt {}", target.display()))?;
        println!("wrote queue health receipt: {}", target.display());
    } else {
        println!("{payload}");
    }

    Ok(())
}

fn load_input(fixture: Option<&PathBuf>) -> Result<QueueHealthInput> {
    if let Some(path) = fixture {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("reading fixture {}", path.display()))?;
        return serde_json::from_str(&raw)
            .with_context(|| format!("parsing fixture {}", path.display()));
    }

    Ok(QueueHealthInput {
        master_sha: Some(current_master_sha_fallback()),
        master_ci_state: Some("pending".to_string()),
        pending_checks: Some(0),
        running_checks: Some(0),
        failed_checks: Some(0),
        failure_classifier: None,
        ruleset: None,
    })
}

fn current_master_sha_fallback() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|raw| raw.trim().to_string())
        .filter(|sha| !sha.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

fn evaluate_queue_health(input: QueueHealthInput) -> QueueHealthReceipt {
    let master_sha = input.master_sha.unwrap_or_else(current_master_sha_fallback);
    let pending_checks = input.pending_checks.unwrap_or(0);
    let running_checks = input.running_checks.unwrap_or(0);
    let failed_checks = input.failed_checks.unwrap_or(0);
    let ci_state = input.master_ci_state.unwrap_or_else(|| "pending".to_string());
    let normalized_state = ci_state.trim().to_ascii_lowercase();

    if is_red(&normalized_state, failed_checks) {
        return red_receipt(master_sha, input.failure_classifier, failed_checks);
    }

    if is_pending(&normalized_state, pending_checks, running_checks) {
        return pending_receipt(master_sha, pending_checks, running_checks, input.ruleset);
    }

    green_receipt(master_sha)
}

fn is_red(normalized_state: &str, failed_checks: u64) -> bool {
    matches!(normalized_state, "red" | "failure" | "failed" | "error") || failed_checks > 0
}

fn is_pending(normalized_state: &str, pending_checks: u64, running_checks: u64) -> bool {
    matches!(normalized_state, "pending" | "running" | "queued")
        || pending_checks > 0
        || running_checks > 0
}

fn green_receipt(master_sha: String) -> QueueHealthReceipt {
    QueueHealthReceipt {
        master_sha,
        mode: QueueHealthMode::Green,
        allowed_lanes: vec![
            "merge-drain".to_string(),
            "cascade-update".to_string(),
            "green-ci-promotion".to_string(),
        ],
        blocked_lanes: Vec::new(),
        reasons: vec!["master CI is green and no pending/running blockers detected".to_string()],
        verdict: "safe_to_merge".to_string(),
    }
}

fn pending_receipt(
    master_sha: String,
    pending_checks: u64,
    running_checks: u64,
    ruleset: Option<RulesetState>,
) -> QueueHealthReceipt {
    let candidate_current =
        ruleset.as_ref().and_then(|current| current.candidate_current).unwrap_or(false);
    let gate_policy =
        ruleset.and_then(|current| current.gate_policy).unwrap_or_else(|| "unknown".to_string());

    let mut reasons = vec![format!(
        "master CI is pending (pending_checks={pending_checks}, running_checks={running_checks})"
    )];
    reasons.push(format!("gate policy: {gate_policy}"));
    if !candidate_current {
        reasons.push("merge-ready promotion blocked until candidate is current".to_string());
    }

    QueueHealthReceipt {
        master_sha,
        mode: QueueHealthMode::Pending,
        allowed_lanes: vec!["review-read-only".to_string(), "design-read-only".to_string()],
        blocked_lanes: vec![
            "merge-drain".to_string(),
            "green-ci-promotion".to_string(),
            "broad-cascade-final-labels".to_string(),
        ],
        reasons,
        verdict: if candidate_current {
            "review_only_candidate_current".to_string()
        } else {
            "review_only_candidate_stale".to_string()
        },
    }
}

fn red_receipt(
    master_sha: String,
    classifier: Option<FailureClassifier>,
    failed_checks: u64,
) -> QueueHealthReceipt {
    let mut reasons = vec![format!("master CI is red (failed_checks={failed_checks})")];
    let shared_blocker =
        classifier.as_ref().and_then(|value| value.shared_blocker).unwrap_or(false);
    if shared_blocker {
        reasons.push("failure classifier marked this as a shared blocker".to_string());
    }
    if let Some(summary) = classifier.and_then(|value| value.summary) {
        reasons.push(format!("failure classifier summary: {summary}"));
    }

    QueueHealthReceipt {
        master_sha,
        mode: QueueHealthMode::Red,
        allowed_lanes: vec!["master-fix".to_string(), "review-read-only".to_string()],
        blocked_lanes: vec![
            "merge-drain".to_string(),
            "cascade-update".to_string(),
            "green-ci-promotion".to_string(),
        ],
        reasons,
        verdict: if shared_blocker {
            "freeze_merge_drain_shared_blocker".to_string()
        } else {
            "freeze_merge_drain".to_string()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_path(name: &str) -> Result<PathBuf> {
        Ok(utils::project_root()?.join("xtask/tests/fixtures/queue-health").join(name))
    }

    #[test]
    fn green_fixture_maps_to_green_mode() -> Result<()> {
        let fixture = fixture_path("master-green.json")?;
        let receipt = evaluate_queue_health(load_input(Some(&fixture))?);
        assert!(matches!(receipt.mode, QueueHealthMode::Green));
        Ok(())
    }

    #[test]
    fn pending_fixture_maps_to_pending_mode() -> Result<()> {
        let fixture = fixture_path("master-pending.json")?;
        let receipt = evaluate_queue_health(load_input(Some(&fixture))?);
        assert!(matches!(receipt.mode, QueueHealthMode::Pending));
        Ok(())
    }

    #[test]
    fn red_fixture_maps_to_red_mode() -> Result<()> {
        let fixture = fixture_path("master-red.json")?;
        let receipt = evaluate_queue_health(load_input(Some(&fixture))?);
        assert!(matches!(receipt.mode, QueueHealthMode::Red));
        Ok(())
    }
}
