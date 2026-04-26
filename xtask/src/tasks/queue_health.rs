use color_eyre::eyre::{Result, bail};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum QueueHealthMode {
    Green,
    Pending,
    Red,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueHealthReceipt {
    pub master_sha: String,
    pub mode: QueueHealthMode,
    pub allowed_lanes: Vec<String>,
    pub blocked_lanes: Vec<String>,
    pub reasons: Vec<String>,
    pub verdict: String,
}

#[derive(Debug, Clone, Deserialize)]
struct QueueHealthFixture {
    master_sha: String,
    master_ci: MasterCiState,
    #[serde(default)]
    failure_classifier: Option<FailureClassifier>,
    #[serde(default)]
    gate_policy: Option<GatePolicy>,
}

#[derive(Debug, Clone, Deserialize)]
struct MasterCiState {
    status: String,
    #[serde(default)]
    pending_checks: usize,
    #[serde(default)]
    running_checks: usize,
}

#[derive(Debug, Clone, Deserialize)]
struct FailureClassifier {
    #[serde(default)]
    summary: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct GatePolicy {
    #[serde(default)]
    ruleset_name: Option<String>,
}

pub fn run_health(receipt: Option<PathBuf>, fixture: Option<PathBuf>) -> Result<()> {
    let fixture_path = fixture.ok_or_else(|| {
        color_eyre::eyre::eyre!(
            "queue health currently requires --fixture <json> to provide CI inputs"
        )
    })?;

    let fixture_data = fs::read_to_string(&fixture_path)?;
    let fixture: QueueHealthFixture = serde_json::from_str(&fixture_data)?;
    let receipt_payload = build_receipt(&fixture)?;

    if let Some(receipt_path) = receipt {
        if let Some(parent) = receipt_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let pretty = serde_json::to_string_pretty(&receipt_payload)?;
        fs::write(receipt_path, pretty)?;
    }

    println!("{}", serde_json::to_string_pretty(&receipt_payload)?);
    Ok(())
}

fn build_receipt(input: &QueueHealthFixture) -> Result<QueueHealthReceipt> {
    let status = input.master_ci.status.to_ascii_lowercase();
    let mode = match status.as_str() {
        "green" | "success" | "passed" => QueueHealthMode::Green,
        "pending" | "running" => QueueHealthMode::Pending,
        "red" | "failed" | "failure" => QueueHealthMode::Red,
        _ => bail!(
            "unknown master_ci.status '{}'; expected green|pending|red",
            input.master_ci.status
        ),
    };

    let mut reasons = vec![format!("master CI status is {}", input.master_ci.status)];
    if input.master_ci.pending_checks > 0 {
        reasons.push(format!("{} check(s) pending", input.master_ci.pending_checks));
    }
    if input.master_ci.running_checks > 0 {
        reasons.push(format!("{} check(s) running", input.master_ci.running_checks));
    }
    if let Some(classifier) = &input.failure_classifier
        && let Some(summary) = &classifier.summary
    {
        reasons.push(format!("failure classifier: {summary}"));
    }
    if let Some(gate_policy) = &input.gate_policy
        && let Some(name) = &gate_policy.ruleset_name
    {
        reasons.push(format!("gate policy: {name}"));
    }

    let (allowed_lanes, blocked_lanes, verdict) = match mode {
        QueueHealthMode::Green => (
            vec![
                "merge-drain".to_string(),
                "cascade-update".to_string(),
                "green-ci-promotion".to_string(),
            ],
            Vec::new(),
            "safe-to-merge".to_string(),
        ),
        QueueHealthMode::Pending => (
            vec!["review-design-read-only".to_string()],
            vec![
                "merge-ready-promotion-non-current-candidate".to_string(),
                "broad-cascade-final-labels".to_string(),
            ],
            "review-only".to_string(),
        ),
        QueueHealthMode::Red => (
            vec!["master-fix".to_string(), "review-design-read-only".to_string()],
            vec![
                "merge-drain".to_string(),
                "cascade-update".to_string(),
                "green-ci-promotion".to_string(),
            ],
            "freeze-merge-drain".to_string(),
        ),
    };

    Ok(QueueHealthReceipt {
        master_sha: input.master_sha.clone(),
        mode,
        allowed_lanes,
        blocked_lanes,
        reasons,
        verdict,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use color_eyre::eyre::Result;

    #[test]
    fn maps_green_mode() -> Result<()> {
        let fixture = QueueHealthFixture {
            master_sha: "abc123".to_string(),
            master_ci: MasterCiState {
                status: "green".to_string(),
                pending_checks: 0,
                running_checks: 0,
            },
            failure_classifier: None,
            gate_policy: None,
        };

        let receipt = build_receipt(&fixture)?;
        assert_eq!(receipt.mode, QueueHealthMode::Green);
        assert!(receipt.blocked_lanes.is_empty());
        Ok(())
    }
}
