use chrono::{DateTime, Utc};
use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AgentLease {
    pub schema_version: u32,
    pub issued_at: DateTime<Utc>,
    #[serde(flatten)]
    pub task: AgentTask,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CurrentSnapshot {
    pub snapshot_id: String,
    pub head_sha: String,
}

pub fn acquire(task_path: &Path, out_path: &Path) -> Result<()> {
    let task = read_task(task_path)?;
    validate_task(&task)?;

    let lease = AgentLease { schema_version: 1, issued_at: Utc::now(), task };

    write_json(out_path, &lease)?;
    println!("Lease written to {}", out_path.display());
    Ok(())
}

pub fn verify(lease_path: &Path, current_path: &Path) -> Result<()> {
    let lease = read_lease(lease_path)?;
    validate_task(&lease.task)?;

    let now = Utc::now();
    if now > lease.task.expires_at {
        bail!("Lease expired at {} (now {})", lease.task.expires_at.to_rfc3339(), now.to_rfc3339());
    }

    let current = read_snapshot(current_path)?;
    if current.snapshot_id != lease.task.snapshot_id {
        bail!(
            "snapshot mismatch: lease={}, current={}",
            lease.task.snapshot_id,
            current.snapshot_id
        );
    }

    if current.head_sha != lease.task.head_sha {
        bail!("stale head: lease={}, current={}", lease.task.head_sha, current.head_sha);
    }

    println!("Lease verification succeeded for task {}", lease.task.task_id);
    Ok(())
}

pub fn read_lease(path: &Path) -> Result<AgentLease> {
    read_json(path, "lease")
}

fn read_task(path: &Path) -> Result<AgentTask> {
    read_json(path, "task")
}

fn read_snapshot(path: &Path) -> Result<CurrentSnapshot> {
    read_json(path, "snapshot")
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path, label: &str) -> Result<T> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("reading {label} JSON from {}", path.display()))?;
    serde_json::from_str(&content)
        .with_context(|| format!("parsing {label} JSON from {}", path.display()))
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating parent directory {}", parent.display()))?;
    }

    let raw = serde_json::to_string_pretty(value).context("serializing lease JSON")?;
    fs::write(path, format!("{raw}\n"))
        .with_context(|| format!("writing JSON output to {}", path.display()))
}

fn validate_task(task: &AgentTask) -> Result<()> {
    if task.task_id.trim().is_empty() {
        bail!("task_id must not be empty");
    }
    if task.snapshot_id.trim().is_empty() {
        bail!("snapshot_id must not be empty");
    }
    if task.head_sha.trim().is_empty() {
        bail!("head_sha must not be empty");
    }
    if task.required_output_schema.trim().is_empty() {
        bail!("required_output_schema must not be empty");
    }
    if task.allowed_mutations.is_empty() {
        bail!("allowed_mutations must not be empty");
    }

    let duplicates = task
        .allowed_mutations
        .iter()
        .filter(|mutation| task.forbidden_mutations.contains(*mutation))
        .cloned()
        .collect::<Vec<_>>();
    if !duplicates.is_empty() {
        bail!("allowed_mutations and forbidden_mutations overlap: {}", duplicates.join(", "));
    }

    Ok(())
}
