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
    pub canonical_state: serde_json::Value,
    pub allowed_mutations: Vec<String>,
    pub forbidden_mutations: Vec<String>,
    pub required_output_schema: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentLease {
    pub schema_version: String,
    pub issued_at: DateTime<Utc>,
    pub task: AgentTask,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CurrentSnapshot {
    pub snapshot_id: String,
    pub head_sha: String,
}

pub fn acquire(task_path: &Path, out_path: &Path) -> Result<()> {
    let task = load_task(task_path)?;
    validate_task(&task)?;

    let lease = AgentLease { schema_version: "1.0.0".to_string(), issued_at: Utc::now(), task };

    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create output directory {}", parent.display()))?;
    }

    let content = serde_json::to_string_pretty(&lease).context("Failed to serialize lease")?;
    fs::write(out_path, content)
        .with_context(|| format!("Failed to write lease to {}", out_path.display()))?;

    println!("Lease acquired: {}", out_path.display());
    Ok(())
}

pub fn verify(lease_path: &Path, current_path: &Path) -> Result<()> {
    let lease = load_lease(lease_path)?;
    let current = load_snapshot(current_path)?;

    if Utc::now() > lease.task.expires_at {
        bail!("Lease expired at {}; mutation rejected", lease.task.expires_at.to_rfc3339());
    }

    if lease.task.snapshot_id != current.snapshot_id {
        bail!(
            "Snapshot mismatch: lease={} current={}",
            lease.task.snapshot_id,
            current.snapshot_id
        );
    }

    if lease.task.head_sha != current.head_sha {
        bail!("Stale head: lease={} current={}", lease.task.head_sha, current.head_sha);
    }

    println!("Lease verification passed for task {}", lease.task.task_id);
    Ok(())
}

fn load_task(path: &Path) -> Result<AgentTask> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("Failed to read task file {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("Invalid task JSON in {}", path.display()))
}

fn load_lease(path: &Path) -> Result<AgentLease> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("Failed to read lease file {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("Invalid lease JSON in {}", path.display()))
}

fn load_snapshot(path: &Path) -> Result<CurrentSnapshot> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("Failed to read snapshot file {}", path.display()))?;
    serde_json::from_str(&raw)
        .with_context(|| format!("Invalid snapshot JSON in {}", path.display()))
}

fn validate_task(task: &AgentTask) -> Result<()> {
    if task.task_id.trim().is_empty() {
        bail!("task_id must not be empty");
    }
    if task.allowed_mutations.is_empty() {
        bail!("allowed_mutations must include at least one mutation");
    }
    if task.allowed_mutations.iter().any(|value| task.forbidden_mutations.contains(value)) {
        bail!("allowed_mutations and forbidden_mutations overlap");
    }
    Ok(())
}
