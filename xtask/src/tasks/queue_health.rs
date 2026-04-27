//! Queue health classifier for orchestrator merge safety.
//!
//! Computes one of three queue health modes:
//! - GREEN: merge/cascade/promotion lanes are open.
//! - PENDING: read-only review/design lanes only while master checks settle.
//! - RED: merge drain frozen; only master-fix and read-only review lanes remain.

use std::fs;
use std::path::{Path, PathBuf};

use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};

const DEFAULT_RECEIPT_PATH: &str = "target/receipts/queue-health.json";
const DEFAULT_INPUT_PATH: &str = "target/receipts/master-ci-state.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum QueueMode {
    Green,
    Pending,
    Red,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueHealthReceipt {
    pub master_sha: String,
    pub mode: QueueMode,
    pub allowed_lanes: Vec<String>,
    pub blocked_lanes: Vec<String>,
    pub reasons: Vec<String>,
    pub verdict: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QueueHealthInput {
    pub master_sha: String,
    #[serde(default)]
    pub ci_state: Option<String>,
    #[serde(default)]
    pub pending_checks: Vec<String>,
    #[serde(default)]
    pub running_checks: Vec<String>,
    #[serde(default)]
    pub failed_checks: Vec<String>,
    #[serde(default)]
    pub failure_classifier: Option<FailureClassifier>,
    #[serde(default)]
    pub gate_policy: Option<GatePolicy>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FailureClassifier {
    #[serde(default)]
    pub shared_blocker: bool,
    #[serde(default)]
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GatePolicy {
    #[serde(default)]
    pub pending_allows_merge_ready_if_candidate_current: bool,
}

#[derive(Debug, Clone)]
pub struct QueueHealthArgs {
    pub receipt: Option<PathBuf>,
    pub fixture: Option<PathBuf>,
}

pub fn run(args: QueueHealthArgs) -> Result<()> {
    let input = load_input(args.fixture.as_deref())?;
    let receipt = classify(&input);

    let out = serde_json::to_string_pretty(&receipt)?;
    let receipt_path = args.receipt.unwrap_or_else(|| PathBuf::from(DEFAULT_RECEIPT_PATH));

    if let Some(parent) = receipt_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create parent dir: {}", parent.display()))?;
    }
    fs::write(&receipt_path, format!("{out}\n"))
        .with_context(|| format!("failed writing receipt to {}", receipt_path.display()))?;

    println!("{}", receipt.mode_as_str());
    println!("wrote {}", receipt_path.display());

    Ok(())
}

fn load_input(fixture: Option<&Path>) -> Result<QueueHealthInput> {
    let input_path =
        fixture.map(PathBuf::from).unwrap_or_else(|| PathBuf::from(DEFAULT_INPUT_PATH));
    if fixture.is_none() && !input_path.exists() {
        bail!(
            "missing default input {} (pass --fixture <json> to classify from a fixture)",
            input_path.display()
        );
    }

    let raw = fs::read_to_string(&input_path)
        .with_context(|| format!("failed to read input {}", input_path.display()))?;
    let input: QueueHealthInput = serde_json::from_str(&raw)
        .with_context(|| format!("invalid queue-health input {}", input_path.display()))?;
    Ok(input)
}

pub fn classify(input: &QueueHealthInput) -> QueueHealthReceipt {
    let mut reasons = Vec::new();

    let state = input
        .ci_state
        .as_deref()
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_else(|| "unknown".to_string());

    if !input.pending_checks.is_empty() {
        reasons.push(format!("{} pending check(s)", input.pending_checks.len()));
    }
    if !input.running_checks.is_empty() {
        reasons.push(format!("{} running check(s)", input.running_checks.len()));
    }
    if !input.failed_checks.is_empty() {
        reasons.push(format!("{} failed check(s)", input.failed_checks.len()));
    }

    if let Some(classifier) = &input.failure_classifier {
        if classifier.shared_blocker {
            reasons.push("failure classifier marked shared blocker".to_string());
        }
        if let Some(summary) = &classifier.summary {
            reasons.push(format!("failure summary: {summary}"));
        }
    }

    let has_pending = !input.pending_checks.is_empty() || !input.running_checks.is_empty();
    let has_failures = !input.failed_checks.is_empty();
    let shared_blocker = input.failure_classifier.as_ref().is_some_and(|c| c.shared_blocker);

    let mode = if state == "red" || has_failures || shared_blocker {
        QueueMode::Red
    } else if state == "pending" || has_pending {
        QueueMode::Pending
    } else {
        QueueMode::Green
    };

    if reasons.is_empty() {
        reasons.push("master CI reported fully settled".to_string());
    }

    let (allowed_lanes, blocked_lanes, verdict) = build_policy(mode, input.gate_policy.as_ref());

    QueueHealthReceipt {
        master_sha: input.master_sha.clone(),
        mode,
        allowed_lanes,
        blocked_lanes,
        reasons,
        verdict,
    }
}

fn build_policy(
    mode: QueueMode,
    gate_policy: Option<&GatePolicy>,
) -> (Vec<String>, Vec<String>, String) {
    match mode {
        QueueMode::Green => (
            vec![
                "merge-drain".to_string(),
                "cascade-update".to_string(),
                "green-ci-promotion".to_string(),
            ],
            Vec::new(),
            "master healthy: merge drain/cascade/promotions allowed".to_string(),
        ),
        QueueMode::Pending => {
            let mut allowed = vec!["read-only-review".to_string(), "read-only-design".to_string()];
            if gate_policy
                .is_some_and(|policy| policy.pending_allows_merge_ready_if_candidate_current)
            {
                allowed.push("merge-ready-promotion-if-candidate-current".to_string());
            }

            (
                allowed,
                vec![
                    "merge-drain".to_string(),
                    "green-ci-promotion".to_string(),
                    "broad-cascade-final-labels".to_string(),
                ],
                "master unsettled: review/design only; defer broad promotion lanes".to_string(),
            )
        }
        QueueMode::Red => (
            vec!["master-fix".to_string(), "read-only-review".to_string()],
            vec![
                "merge-drain".to_string(),
                "cascade-update".to_string(),
                "green-ci-promotion".to_string(),
                "merge-ready-promotion".to_string(),
                "broad-cascade-final-labels".to_string(),
            ],
            "master failing: freeze merge drain and classify shared blocker before reopening lanes"
                .to_string(),
        ),
    }
}

impl QueueHealthReceipt {
    fn mode_as_str(&self) -> &'static str {
        match self.mode {
            QueueMode::Green => "GREEN",
            QueueMode::Pending => "PENDING",
            QueueMode::Red => "RED",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn green_mode_when_master_is_settled() {
        let input = QueueHealthInput {
            master_sha: "abc123".to_string(),
            ci_state: Some("green".to_string()),
            pending_checks: Vec::new(),
            running_checks: Vec::new(),
            failed_checks: Vec::new(),
            failure_classifier: None,
            gate_policy: None,
        };

        let receipt = classify(&input);
        assert_eq!(receipt.mode, QueueMode::Green);
    }

    #[test]
    fn pending_mode_when_checks_are_still_running() {
        let input = QueueHealthInput {
            master_sha: "def456".to_string(),
            ci_state: Some("green".to_string()),
            pending_checks: vec!["merge-gate".to_string()],
            running_checks: Vec::new(),
            failed_checks: Vec::new(),
            failure_classifier: None,
            gate_policy: None,
        };

        let receipt = classify(&input);
        assert_eq!(receipt.mode, QueueMode::Pending);
    }

    #[test]
    fn red_mode_when_failures_exist() {
        let input = QueueHealthInput {
            master_sha: "ghi789".to_string(),
            ci_state: Some("green".to_string()),
            pending_checks: Vec::new(),
            running_checks: Vec::new(),
            failed_checks: vec!["clippy".to_string()],
            failure_classifier: Some(FailureClassifier {
                shared_blocker: true,
                summary: Some("clippy broken".to_string()),
            }),
            gate_policy: None,
        };

        let receipt = classify(&input);
        assert_eq!(receipt.mode, QueueMode::Red);
    }
}
