use crate::tasks::agent_lease::{AgentTask, LeaseSnapshot, read_json};
use chrono::{DateTime, Utc};
use color_eyre::eyre::{Result, bail};
use serde::{Deserialize, Serialize};
use std::path::Path;

const SCHEMA_VERSION: u32 = 1;
const KIND_AGENT_RECEIPT: &str = "agent_receipt";

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
    pub classification: String,
    #[serde(default)]
    pub forbidden_mutations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptValidation {
    pub accepted: bool,
    pub status: String,
    pub reason: String,
}

pub fn validate(
    receipt_path: &Path,
    task_path: &Path,
    snapshot_path: &Path,
    now: DateTime<Utc>,
) -> Result<ReceiptValidation> {
    let receipt: AgentReceipt = read_json(receipt_path)?;
    let task: AgentTask = read_json(task_path)?;
    let snapshot: LeaseSnapshot = read_json(snapshot_path)?;

    if receipt.schema_version != SCHEMA_VERSION || receipt.kind != KIND_AGENT_RECEIPT {
        bail!("invalid receipt schema or kind");
    }
    if receipt.task_id != task.task_id {
        bail!("receipt task_id does not match task");
    }
    if receipt
        .forbidden_mutations
        .iter()
        .any(|mutation| task.forbidden_mutations.contains(mutation))
    {
        return Ok(ReceiptValidation {
            accepted: false,
            status: "rejected".to_string(),
            reason: "receipt included forbidden mutation".to_string(),
        });
    }

    if task.expires_at <= now {
        return Ok(ReceiptValidation {
            accepted: false,
            status: "stale".to_string(),
            reason: "task/lease expired".to_string(),
        });
    }

    if let Some(pr) = snapshot.prs.iter().find(|pr| pr.number == receipt.pr) {
        if pr.head_sha != receipt.head_sha {
            return Ok(ReceiptValidation {
                accepted: false,
                status: "stale".to_string(),
                reason: "head sha mismatch".to_string(),
            });
        }

        if pr.base_sha != receipt.base_sha {
            return Ok(ReceiptValidation {
                accepted: false,
                status: "advisory".to_string(),
                reason: "base sha mismatch".to_string(),
            });
        }
    }

    Ok(ReceiptValidation {
        accepted: true,
        status: "current".to_string(),
        reason: "receipt is fresh".to_string(),
    })
}

pub fn status(
    receipt_path: &Path,
    snapshot_path: &Path,
    now: DateTime<Utc>,
) -> Result<ReceiptValidation> {
    let receipt: AgentReceipt = read_json(receipt_path)?;
    let snapshot: LeaseSnapshot = read_json(snapshot_path)?;

    if let Some(pr) = snapshot.prs.iter().find(|pr| pr.number == receipt.pr)
        && pr.head_sha != receipt.head_sha
    {
        return Ok(ReceiptValidation {
            accepted: false,
            status: "stale".to_string(),
            reason: "head sha mismatch".to_string(),
        });
    }

    let winner_exists = snapshot
        .leases
        .iter()
        .any(|lease| lease.lease_id == receipt.lease_id && lease.expires_at > now);

    if !winner_exists {
        return Ok(ReceiptValidation {
            accepted: false,
            status: "advisory".to_string(),
            reason: "lease missing or expired".to_string(),
        });
    }

    Ok(ReceiptValidation {
        accepted: true,
        status: "current".to_string(),
        reason: "receipt lease still valid".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::agent_lease::{AgentLease, SnapshotPrState};
    use color_eyre::eyre::Result;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn stale_head_receipt_is_marked_stale() -> Result<()> {
        let dir = tempdir()?;
        let receipt_path = dir.path().join("receipt.json");
        let task_path = dir.path().join("task.json");
        let snapshot_path = dir.path().join("snapshot.json");

        let receipt = AgentReceipt {
            schema_version: 1,
            kind: "agent_receipt".to_string(),
            task_id: "task-1".to_string(),
            lease_id: "lease-1".to_string(),
            lane: "green_ci".to_string(),
            pr: 6854,
            head_sha: "old".to_string(),
            base_sha: "base".to_string(),
            verdict: "ci_green".to_string(),
            classification: "CURRENT_GREEN".to_string(),
            forbidden_mutations: vec![],
        };

        let task = AgentTask {
            schema_version: 1,
            kind: "agent_task".to_string(),
            task_id: "task-1".to_string(),
            snapshot_id: "snap".to_string(),
            lane: "green_ci".to_string(),
            pr: 6854,
            head_sha: "old".to_string(),
            base_sha: "base".to_string(),
            canonical_state: "NEEDS_CI".to_string(),
            allowed_mutations: vec!["emit_receipt".to_string()],
            forbidden_mutations: vec!["merge".to_string()],
            required_output_schema: "schema.json".to_string(),
            expires_at: DateTime::parse_from_rfc3339("2026-04-26T19:10:00Z")?.with_timezone(&Utc),
        };

        let snapshot = LeaseSnapshot {
            snapshot_id: "snap".to_string(),
            prs: vec![SnapshotPrState {
                number: 6854,
                head_sha: "new".to_string(),
                base_sha: "base".to_string(),
            }],
            leases: vec![AgentLease {
                schema_version: 1,
                kind: "agent_lease".to_string(),
                task_id: "task-1".to_string(),
                lease_id: "lease-1".to_string(),
                owner: "box".to_string(),
                pr: 6854,
                head_sha: "old".to_string(),
                base_sha: "base".to_string(),
                allowed_mutations: vec!["emit_receipt".to_string()],
                expires_at: DateTime::parse_from_rfc3339("2026-04-26T19:10:00Z")?
                    .with_timezone(&Utc),
                claimed_at: Some(
                    DateTime::parse_from_rfc3339("2026-04-26T18:40:00Z")?.with_timezone(&Utc),
                ),
            }],
        };

        fs::write(&receipt_path, serde_json::to_string(&receipt)?)?;
        fs::write(&task_path, serde_json::to_string(&task)?)?;
        fs::write(&snapshot_path, serde_json::to_string(&snapshot)?)?;

        let now = DateTime::parse_from_rfc3339("2026-04-26T18:45:00Z")?.with_timezone(&Utc);
        let result = validate(&receipt_path, &task_path, &snapshot_path, now)?;
        assert_eq!(result.status, "stale");
        Ok(())
    }
}
