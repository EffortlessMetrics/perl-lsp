use chrono::{DateTime, Utc};
use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    pub task_id: String,
    pub snapshot_id: String,
    pub lane: String,
    pub pr: u64,
    pub head_sha: String,
    pub base_sha: String,
    pub canonical_state: String,
    pub allowed_mutations: Vec<String>,
    pub forbidden_mutations: Vec<String>,
    pub required_output_schema: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentLease {
    pub schema_version: u32,
    pub leased_at: DateTime<Utc>,
    #[serde(flatten)]
    pub task: AgentTask,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentSnapshot {
    pub snapshot_id: String,
    pub head_sha: String,
}

pub fn acquire(task_path: &Path, out_path: &Path) -> Result<()> {
    let task_json = fs::read_to_string(task_path)
        .with_context(|| format!("failed to read task file: {}", task_path.display()))?;
    let task: AgentTask = serde_json::from_str(&task_json)
        .with_context(|| format!("failed to parse task file: {}", task_path.display()))?;
    task.validate()?;

    let lease = AgentLease { schema_version: 1, leased_at: Utc::now(), task };
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory: {}", parent.display()))?;
    }
    fs::write(out_path, format!("{}\n", serde_json::to_string_pretty(&lease)?))
        .with_context(|| format!("failed to write lease file: {}", out_path.display()))?;
    println!("lease written: {}", out_path.display());
    Ok(())
}

pub fn verify(lease_path: &Path, current_path: &Path) -> Result<()> {
    let lease_raw = fs::read_to_string(lease_path)
        .with_context(|| format!("failed to read lease file: {}", lease_path.display()))?;
    let lease: AgentLease = serde_json::from_str(&lease_raw)
        .with_context(|| format!("failed to parse lease file: {}", lease_path.display()))?;
    lease.task.validate()?;

    let current_raw = fs::read_to_string(current_path)
        .with_context(|| format!("failed to read snapshot file: {}", current_path.display()))?;
    let current: CurrentSnapshot = serde_json::from_str(&current_raw)
        .with_context(|| format!("failed to parse snapshot file: {}", current_path.display()))?;

    if lease.task.expires_at <= Utc::now() {
        bail!("lease expired at {}", lease.task.expires_at.to_rfc3339());
    }

    if current.snapshot_id != lease.task.snapshot_id {
        bail!(
            "snapshot mismatch: lease={} current={}",
            lease.task.snapshot_id,
            current.snapshot_id
        );
    }

    if current.head_sha != lease.task.head_sha {
        bail!("stale head: lease={} current={}", lease.task.head_sha, current.head_sha);
    }

    println!("lease verified: {}", lease.task.task_id);
    Ok(())
}

impl AgentTask {
    fn validate(&self) -> Result<()> {
        if self.task_id.trim().is_empty() {
            bail!("task_id cannot be empty");
        }
        if self.snapshot_id.trim().is_empty() {
            bail!("snapshot_id cannot be empty");
        }
        if self.head_sha.trim().is_empty() || self.base_sha.trim().is_empty() {
            bail!("head_sha and base_sha cannot be empty");
        }
        if self.allowed_mutations.is_empty() {
            bail!("allowed_mutations cannot be empty");
        }
        Ok(())
    }
}
