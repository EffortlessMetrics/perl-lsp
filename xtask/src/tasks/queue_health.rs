use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::utils::project_root;

#[derive(Debug, Clone)]
pub struct QueueHealthConfig {
    pub receipt: Option<PathBuf>,
    pub fixture: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum MasterCiState {
    Green,
    Pending,
    Red,
}

#[derive(Debug, Deserialize, Default)]
struct FailureClassifier {
    #[serde(default)]
    shared_blocker: bool,
    #[serde(default)]
    category: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct GatePolicy {
    #[serde(default)]
    candidate_current: bool,
    #[serde(default)]
    policy_name: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct QueueHealthInput {
    master_sha: Option<String>,
    master_ci_state: Option<MasterCiState>,
    #[serde(default)]
    pending_checks: usize,
    #[serde(default)]
    running_checks: usize,
    #[serde(default)]
    failure_classifier: Option<FailureClassifier>,
    #[serde(default)]
    gate_policy: Option<GatePolicy>,
    #[serde(default)]
    expected_mode: Option<QueueMode>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum QueueMode {
    Green,
    Pending,
    Red,
}

#[derive(Debug, Serialize)]
struct QueueHealthReceipt {
    master_sha: String,
    mode: QueueMode,
    allowed_lanes: Vec<String>,
    blocked_lanes: Vec<String>,
    reasons: Vec<String>,
    verdict: String,
}

pub fn run(config: QueueHealthConfig) -> Result<()> {
    let input = load_input(config.fixture.as_deref())?;
    let receipt = evaluate(&input)?;

    if let Some(expected_mode) = input.expected_mode
        && expected_mode != receipt.mode
    {
        bail!("fixture expected mode {:?}, got {:?}", expected_mode, receipt.mode);
    }

    let json = serde_json::to_string_pretty(&receipt).context("serialize queue health receipt")?;

    if let Some(receipt_path) = config.receipt {
        if let Some(parent) = receipt_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create receipt dir {}", parent.display()))?;
        }
        fs::write(&receipt_path, format!("{json}\n"))
            .with_context(|| format!("write receipt {}", receipt_path.display()))?;
        println!("queue-health receipt: {}", receipt_path.display());
    }

    println!("{json}");
    Ok(())
}

fn load_input(fixture: Option<&Path>) -> Result<QueueHealthInput> {
    if let Some(path) = fixture {
        let body = fs::read_to_string(path)
            .with_context(|| format!("read queue health fixture {}", path.display()))?;
        let input: QueueHealthInput = serde_json::from_str(&body)
            .with_context(|| format!("parse fixture {}", path.display()))?;
        return Ok(input);
    }

    let root = project_root()?;
    let master_sha = resolve_head_sha(&root)?;

    Ok(QueueHealthInput {
        master_sha: Some(master_sha),
        master_ci_state: Some(MasterCiState::Pending),
        pending_checks: 0,
        running_checks: 0,
        failure_classifier: None,
        gate_policy: None,
        expected_mode: None,
    })
}

fn resolve_head_sha(root: &Path) -> Result<String> {
    let output = std::process::Command::new("git")
        .arg("rev-parse")
        .arg("HEAD")
        .current_dir(root)
        .output()
        .context("run git rev-parse HEAD")?;

    if !output.status.success() {
        bail!("git rev-parse HEAD failed with status {}", output.status);
    }

    let sha = String::from_utf8(output.stdout).context("decode git rev-parse output")?;
    Ok(sha.trim().to_string())
}

fn evaluate(input: &QueueHealthInput) -> Result<QueueHealthReceipt> {
    let mode = determine_mode(input);
    let master_sha = input
        .master_sha
        .clone()
        .ok_or_else(|| color_eyre::eyre::eyre!("master_sha is required"))?;

    let mut reasons = Vec::new();
    reasons.push(format!("master_sha={master_sha}"));
    reasons.push(format!("pending_checks={}", input.pending_checks));
    reasons.push(format!("running_checks={}", input.running_checks));

    if let Some(policy) = &input.gate_policy {
        if let Some(policy_name) = &policy.policy_name {
            reasons.push(format!("gate_policy={policy_name}"));
        }
        reasons.push(format!("candidate_current={}", policy.candidate_current));
    }

    if let Some(classifier) = &input.failure_classifier {
        reasons.push(format!("shared_blocker={}", classifier.shared_blocker));
        if let Some(category) = &classifier.category {
            reasons.push(format!("failure_category={category}"));
        }
    }

    let (allowed_lanes, blocked_lanes, verdict) = match mode {
        QueueMode::Green => (
            vec![
                "merge-drain".to_string(),
                "cascade-update".to_string(),
                "green-ci-promotion".to_string(),
                "review-only".to_string(),
            ],
            Vec::new(),
            "safe-to-merge".to_string(),
        ),
        QueueMode::Pending => (
            vec!["review-only".to_string(), "design".to_string()],
            vec![
                "merge-drain".to_string(),
                "broad-cascade-final-labels".to_string(),
                "merge-ready-promotion-when-candidate-stale".to_string(),
            ],
            "review-only-until-master-current".to_string(),
        ),
        QueueMode::Red => (
            vec!["master-fix".to_string(), "review-only".to_string()],
            vec![
                "merge-drain".to_string(),
                "cascade-update".to_string(),
                "green-ci-promotion".to_string(),
            ],
            "freeze-merge-drain-classify-shared-blocker".to_string(),
        ),
    };

    Ok(QueueHealthReceipt { master_sha, mode, allowed_lanes, blocked_lanes, reasons, verdict })
}

fn determine_mode(input: &QueueHealthInput) -> QueueMode {
    if input.pending_checks > 0 || input.running_checks > 0 {
        return QueueMode::Pending;
    }

    match input.master_ci_state {
        Some(MasterCiState::Green) => QueueMode::Green,
        Some(MasterCiState::Pending) | None => QueueMode::Pending,
        Some(MasterCiState::Red) => QueueMode::Red,
    }
}
