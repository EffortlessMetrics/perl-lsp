use crate::tasks::agent_lease::{AgentLease, read_lease};
use chrono::{DateTime, Utc};
use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AgentReceipt {
    pub schema_version: u32,
    pub task_id: String,
    pub snapshot_id: String,
    pub head_sha: String,
    pub lease_path: String,
    pub required_output_schema: String,
    pub received_at: DateTime<Utc>,
    pub idempotency_key: String,
    pub mutation: String,
    pub status: String,
}

pub fn validate(receipt_path: &Path) -> Result<()> {
    let receipt = read_receipt(receipt_path)?;
    validate_core_fields(&receipt)?;

    let lease = read_lease(Path::new(&receipt.lease_path))?;
    validate_against_lease(&receipt, &lease)?;

    println!("Receipt validation succeeded for task {}", receipt.task_id);
    Ok(())
}

fn read_receipt(path: &Path) -> Result<AgentReceipt> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("reading receipt JSON from {}", path.display()))?;
    serde_json::from_str(&content)
        .with_context(|| format!("parsing receipt JSON from {}", path.display()))
}

fn validate_core_fields(receipt: &AgentReceipt) -> Result<()> {
    if receipt.task_id.trim().is_empty() {
        bail!("task_id must not be empty");
    }
    if receipt.idempotency_key.trim().is_empty() {
        bail!("idempotency_key must not be empty");
    }
    if receipt.mutation.trim().is_empty() {
        bail!("mutation must not be empty");
    }
    Ok(())
}

fn validate_against_lease(receipt: &AgentReceipt, lease: &AgentLease) -> Result<()> {
    if receipt.task_id != lease.task.task_id {
        bail!("task_id mismatch: receipt={}, lease={}", receipt.task_id, lease.task.task_id);
    }
    if receipt.snapshot_id != lease.task.snapshot_id {
        bail!(
            "snapshot_id mismatch: receipt={}, lease={}",
            receipt.snapshot_id,
            lease.task.snapshot_id
        );
    }
    if receipt.head_sha != lease.task.head_sha {
        bail!("stale head: receipt={}, lease={}", receipt.head_sha, lease.task.head_sha);
    }
    if receipt.required_output_schema != lease.task.required_output_schema {
        bail!(
            "required_output_schema mismatch: receipt={}, lease={}",
            receipt.required_output_schema,
            lease.task.required_output_schema
        );
    }

    let allowed = lease.task.allowed_mutations.iter().collect::<HashSet<_>>();
    if !allowed.contains(&receipt.mutation) {
        bail!(
            "mutation '{}' is not in allowed_mutations [{}]",
            receipt.mutation,
            lease.task.allowed_mutations.join(", ")
        );
    }

    let forbidden = lease.task.forbidden_mutations.iter().collect::<HashSet<_>>();
    if forbidden.contains(&receipt.mutation) {
        bail!("mutation '{}' is forbidden", receipt.mutation);
    }

    let now = Utc::now();
    if now > lease.task.expires_at {
        bail!(
            "lease expired at {}; mutation '{}' rejected",
            lease.task.expires_at.to_rfc3339(),
            receipt.mutation
        );
    }

    Ok(())
}
