use chrono::{DateTime, Utc};
use color_eyre::eyre::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
pub struct AgentLease {
    pub lease_version: String,
    pub lease_id: String,
    pub acquired_at: DateTime<Utc>,
    pub task: AgentTask,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotState {
    pub snapshot_id: String,
    pub head_sha: String,
}

pub fn acquire(task_path: &Path, out_path: &Path) -> Result<()> {
    let task = read_json::<AgentTask>(task_path)?;
    validate_task_shape(&task)?;

    let lease = AgentLease {
        lease_version: "1".to_string(),
        lease_id: format!("{}:{}", task.task_id, task.snapshot_id),
        acquired_at: Utc::now(),
        task,
    };

    write_json(out_path, &lease)?;
    println!("Lease acquired: {}", lease.lease_id);
    Ok(())
}

pub fn verify(lease_path: &Path, current_path: &Path) -> Result<()> {
    let lease = read_json::<AgentLease>(lease_path)?;
    let current = read_json::<SnapshotState>(current_path)?;

    validate_task_shape(&lease.task)?;

    if Utc::now() > lease.task.expires_at {
        bail!("Lease expired at {} (current UTC time {})", lease.task.expires_at, Utc::now());
    }

    ensure!(
        lease.task.snapshot_id == current.snapshot_id,
        "Snapshot mismatch: lease={} current={}",
        lease.task.snapshot_id,
        current.snapshot_id
    );

    ensure!(
        lease.task.head_sha == current.head_sha,
        "Stale head: lease={} current={}",
        lease.task.head_sha,
        current.head_sha
    );

    println!("Lease is valid for task {}", lease.task.task_id);
    Ok(())
}

fn validate_task_shape(task: &AgentTask) -> Result<()> {
    ensure!(!task.task_id.trim().is_empty(), "task_id must not be empty");
    ensure!(!task.snapshot_id.trim().is_empty(), "snapshot_id must not be empty");
    ensure!(!task.head_sha.trim().is_empty(), "head_sha must not be empty");
    ensure!(!task.base_sha.trim().is_empty(), "base_sha must not be empty");
    ensure!(
        !task.allowed_mutations.is_empty(),
        "allowed_mutations must include at least one mutation"
    );
    Ok(())
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("Failed to read JSON file at {}", path.display()))?;
    serde_json::from_str::<T>(&raw)
        .with_context(|| format!("Failed to parse JSON file at {}", path.display()))
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create output directory {}", parent.display()))?;
    }

    let json = serde_json::to_string_pretty(value).context("Failed to serialize JSON")?;
    fs::write(path, json)
        .with_context(|| format!("Failed to write JSON file at {}", path.display()))
}
