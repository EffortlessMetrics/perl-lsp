use color_eyre::eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use super::agent_lease::{AgentLease, AgentTask};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueSnapshot {
    pub snapshot_id: String,
    pub captured_at: String,
    pub prs: Vec<SnapshotPr>,
    #[serde(default)]
    pub active_leases: Option<Vec<AgentLease>>,
    #[serde(default)]
    pub receipts: Option<Vec<AgentReceipt>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotPr {
    pub number: u64,
    pub head_sha: String,
    pub base_sha: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentReceipt {
    pub schema_version: u32,
    pub kind: String,
    pub task_id: String,
    pub lease_id: String,
    pub lane: String,
    pub pr: u64,
    pub head_sha: String,
    pub base_sha: String,
    pub verdict: String,
    #[serde(default)]
    pub requested_mutations: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReceiptStatus {
    pub status: String,
    pub reason: String,
}

pub fn validate(
    receipt_path: &Path,
    task_path: &Path,
    snapshot_path: &Path,
) -> Result<ReceiptStatus> {
    let receipt: AgentReceipt = read_json(receipt_path)?;
    let task: AgentTask = read_json(task_path)?;
    let snapshot: QueueSnapshot = read_json(snapshot_path)?;

    if receipt.task_id != task.task_id || receipt.pr != task.pr {
        return Ok(ReceiptStatus {
            status: "rejected".to_string(),
            reason: "task mismatch".to_string(),
        });
    }

    if task.allowed_mutations.iter().any(|mutation| task.forbidden_mutations.contains(mutation)) {
        return Ok(ReceiptStatus {
            status: "rejected".to_string(),
            reason: "task mutation policy is self-contradictory".to_string(),
        });
    }

    if receipt
        .requested_mutations
        .iter()
        .any(|mutation| task.forbidden_mutations.contains(mutation))
    {
        return Ok(ReceiptStatus {
            status: "rejected".to_string(),
            reason: "receipt requested forbidden mutation".to_string(),
        });
    }

    let Some(pr) = snapshot.prs.iter().find(|pr| pr.number == receipt.pr) else {
        return Ok(ReceiptStatus {
            status: "stale".to_string(),
            reason: "pr not present in snapshot".to_string(),
        });
    };

    if pr.head_sha != receipt.head_sha {
        return Ok(ReceiptStatus {
            status: "stale".to_string(),
            reason: "head sha mismatch".to_string(),
        });
    }

    if pr.base_sha != receipt.base_sha {
        return Ok(ReceiptStatus {
            status: "advisory".to_string(),
            reason: "base sha mismatch".to_string(),
        });
    }

    if has_newer_receipt(&snapshot, &receipt)? {
        return Ok(ReceiptStatus {
            status: "superseded".to_string(),
            reason: "newer receipt exists for same task/lane/head".to_string(),
        });
    }

    Ok(ReceiptStatus {
        status: "valid".to_string(),
        reason: "receipt is current for snapshot head/base".to_string(),
    })
}

pub fn status(receipt_path: &Path, snapshot_path: &Path) -> Result<ReceiptStatus> {
    let receipt: AgentReceipt = read_json(receipt_path)?;
    let snapshot: QueueSnapshot = read_json(snapshot_path)?;
    let Some(pr) = snapshot.prs.iter().find(|pr| pr.number == receipt.pr) else {
        return Ok(ReceiptStatus {
            status: "stale".to_string(),
            reason: "pr not present".to_string(),
        });
    };
    if pr.head_sha != receipt.head_sha {
        return Ok(ReceiptStatus {
            status: "stale".to_string(),
            reason: "head sha mismatch".to_string(),
        });
    }
    if has_newer_receipt(&snapshot, &receipt)? {
        return Ok(ReceiptStatus {
            status: "superseded".to_string(),
            reason: "newer receipt exists".to_string(),
        });
    }
    Ok(ReceiptStatus {
        status: "current".to_string(),
        reason: "receipt tracks latest head".to_string(),
    })
}

fn has_newer_receipt(snapshot: &QueueSnapshot, receipt: &AgentReceipt) -> Result<bool> {
    let Some(receipts) = snapshot.receipts.as_ref() else {
        return Ok(false);
    };
    let created = chrono::DateTime::parse_from_rfc3339(&receipt.created_at)
        .context("receipt.created_at must be RFC3339")?;
    for candidate in receipts {
        if candidate.task_id != receipt.task_id
            || candidate.lane != receipt.lane
            || candidate.head_sha != receipt.head_sha
        {
            continue;
        }
        let candidate_created = chrono::DateTime::parse_from_rfc3339(&candidate.created_at)
            .context("snapshot receipt created_at must be RFC3339")?;
        if candidate_created > created {
            return Ok(true);
        }
    }
    Ok(false)
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("failed to read JSON file {}", path.display()))?;
    serde_json::from_str(&text)
        .with_context(|| format!("failed to parse JSON file {}", path.display()))
}
