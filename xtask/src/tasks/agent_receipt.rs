use chrono::{DateTime, Utc};
use color_eyre::eyre::{Context, Result, bail};
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct AgentReceipt {
    pub schema_version: String,
    pub receipt_id: String,
    pub task_id: String,
    pub snapshot_id: String,
    pub head_sha: String,
    pub lane: String,
    pub pr: u64,
    pub mutation: String,
    pub allowed_mutations: Vec<String>,
    pub forbidden_mutations: Vec<String>,
    pub required_output_schema: String,
    pub emitted_at: DateTime<Utc>,
    pub lease_expires_at: DateTime<Utc>,
    pub sequence: u64,
}

pub fn validate(receipt_path: &Path) -> Result<()> {
    let receipt = load_receipt(receipt_path)?;
    validate_receipt(&receipt)?;
    println!("Receipt validation passed for task {}", receipt.task_id);
    Ok(())
}

fn load_receipt(path: &Path) -> Result<AgentReceipt> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("Failed to read receipt file {}", path.display()))?;
    serde_json::from_str(&raw)
        .with_context(|| format!("Invalid receipt JSON in {}", path.display()))
}

fn validate_receipt(receipt: &AgentReceipt) -> Result<()> {
    if receipt.schema_version != "1.0.0" {
        bail!("Unsupported schema_version: {}", receipt.schema_version);
    }

    if receipt.receipt_id.trim().is_empty() {
        bail!("receipt_id must not be empty");
    }
    if receipt.task_id.trim().is_empty() {
        bail!("task_id must not be empty");
    }
    if receipt.snapshot_id.trim().is_empty() {
        bail!("snapshot_id must not be empty");
    }
    if receipt.head_sha.trim().is_empty() {
        bail!("head_sha must not be empty");
    }
    if receipt.lane.trim().is_empty() {
        bail!("lane must not be empty");
    }
    if receipt.required_output_schema.trim().is_empty() {
        bail!("required_output_schema must not be empty");
    }
    if receipt.pr == 0 {
        bail!("pr must be >= 1");
    }
    if receipt.sequence == 0 {
        bail!("sequence must be >= 1 for supersession ordering");
    }

    if Utc::now() > receipt.lease_expires_at {
        bail!("Lease expired at {}; mutation rejected", receipt.lease_expires_at);
    }
    if receipt.emitted_at > receipt.lease_expires_at {
        bail!("emitted_at must be on or before lease_expires_at");
    }

    if !receipt.allowed_mutations.contains(&receipt.mutation) {
        bail!("Mutation '{}' is not in allowed_mutations", receipt.mutation);
    }

    if receipt.forbidden_mutations.contains(&receipt.mutation) {
        bail!("Mutation '{}' is forbidden", receipt.mutation);
    }

    Ok(())
}
