use chrono::{DateTime, Utc};
use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fs;
use std::path::Path;

const SCHEMA_VERSION: u32 = 1;
const KIND_AGENT_TASK: &str = "agent_task";
const KIND_AGENT_LEASE: &str = "agent_lease";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    pub schema_version: u32,
    pub kind: String,
    pub task_id: String,
    pub snapshot_id: String,
    pub lane: String,
    pub pr: u64,
    pub head_sha: String,
    pub base_sha: String,
    pub canonical_state: String,
    #[serde(default)]
    pub allowed_mutations: Vec<String>,
    #[serde(default)]
    pub forbidden_mutations: Vec<String>,
    pub required_output_schema: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentLease {
    pub schema_version: u32,
    pub kind: String,
    pub task_id: String,
    pub lease_id: String,
    pub owner: String,
    pub pr: u64,
    pub head_sha: String,
    pub base_sha: String,
    #[serde(default)]
    pub allowed_mutations: Vec<String>,
    pub expires_at: DateTime<Utc>,
    #[serde(default)]
    pub claimed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseSnapshot {
    pub snapshot_id: String,
    #[serde(default)]
    pub leases: Vec<AgentLease>,
    #[serde(default)]
    pub prs: Vec<SnapshotPrState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotPrState {
    pub number: u64,
    pub head_sha: String,
    pub base_sha: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseVerification {
    pub valid: bool,
    pub status: String,
    pub reason: String,
    pub winning_lease_id: Option<String>,
}

pub fn create(task_path: &Path, out_path: &Path, owner: &str, now: DateTime<Utc>) -> Result<()> {
    let task: AgentTask = read_json(task_path)?;
    validate_task(&task)?;

    let lease = AgentLease {
        schema_version: SCHEMA_VERSION,
        kind: KIND_AGENT_LEASE.to_string(),
        task_id: task.task_id.clone(),
        lease_id: format!("lease-{}-{}", task.head_sha, now.timestamp()),
        owner: owner.to_string(),
        pr: task.pr,
        head_sha: task.head_sha,
        base_sha: task.base_sha,
        allowed_mutations: task.allowed_mutations,
        expires_at: task.expires_at,
        claimed_at: Some(now),
    };

    write_json(out_path, &lease)
}

pub fn verify(
    lease_path: &Path,
    snapshot_path: &Path,
    now: DateTime<Utc>,
) -> Result<LeaseVerification> {
    let lease: AgentLease = read_json(lease_path)?;
    let snapshot: LeaseSnapshot = read_json(snapshot_path)?;
    validate_lease(&lease)?;

    if lease.expires_at <= now {
        return Ok(LeaseVerification {
            valid: false,
            status: "stale".to_string(),
            reason: "lease expired".to_string(),
            winning_lease_id: find_winner_id(&snapshot, &lease.task_id, &lease.head_sha, now),
        });
    }

    if let Some(pr) = snapshot.prs.iter().find(|pr| pr.number == lease.pr)
        && pr.head_sha != lease.head_sha
    {
        return Ok(LeaseVerification {
            valid: false,
            status: "stale".to_string(),
            reason: "head sha mismatch".to_string(),
            winning_lease_id: find_winner_id(&snapshot, &lease.task_id, &pr.head_sha, now),
        });
    }

    let winner = select_winner(&snapshot, &lease.task_id, &lease.head_sha, now);
    match winner {
        Some(winning) if winning.lease_id == lease.lease_id => Ok(LeaseVerification {
            valid: true,
            status: "current".to_string(),
            reason: "lease is deterministic winner".to_string(),
            winning_lease_id: Some(winning.lease_id.clone()),
        }),
        Some(winning) => Ok(LeaseVerification {
            valid: false,
            status: "superseded".to_string(),
            reason: "another lease won deterministic ordering".to_string(),
            winning_lease_id: Some(winning.lease_id.clone()),
        }),
        None => Ok(LeaseVerification {
            valid: true,
            status: "current".to_string(),
            reason: "no competing leases found".to_string(),
            winning_lease_id: Some(lease.lease_id),
        }),
    }
}

pub fn select_winner<'a>(
    snapshot: &'a LeaseSnapshot,
    task_id: &str,
    head_sha: &str,
    now: DateTime<Utc>,
) -> Option<&'a AgentLease> {
    snapshot
        .leases
        .iter()
        .filter(|lease| {
            lease.task_id == task_id && lease.head_sha == head_sha && lease.expires_at > now
        })
        .min_by(|a, b| compare_leases(a, b))
}

fn compare_leases(a: &AgentLease, b: &AgentLease) -> Ordering {
    let a_time = a.claimed_at.unwrap_or(a.expires_at);
    let b_time = b.claimed_at.unwrap_or(b.expires_at);
    a_time.cmp(&b_time).then_with(|| a.lease_id.cmp(&b.lease_id))
}

fn find_winner_id(
    snapshot: &LeaseSnapshot,
    task_id: &str,
    head_sha: &str,
    now: DateTime<Utc>,
) -> Option<String> {
    select_winner(snapshot, task_id, head_sha, now).map(|lease| lease.lease_id.clone())
}

fn validate_task(task: &AgentTask) -> Result<()> {
    if task.schema_version != SCHEMA_VERSION {
        bail!("unsupported task schema version: {}", task.schema_version);
    }
    if task.kind != KIND_AGENT_TASK {
        bail!("task kind must be '{}'", KIND_AGENT_TASK);
    }
    Ok(())
}

fn validate_lease(lease: &AgentLease) -> Result<()> {
    if lease.schema_version != SCHEMA_VERSION {
        bail!("unsupported lease schema version: {}", lease.schema_version);
    }
    if lease.kind != KIND_AGENT_LEASE {
        bail!("lease kind must be '{}'", KIND_AGENT_LEASE);
    }
    Ok(())
}

pub fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("failed to read JSON file: {}", path.display()))?;
    serde_json::from_str(&text)
        .with_context(|| format!("failed to parse JSON file: {}", path.display()))
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create parent dir for {}", path.display()))?;
    }
    let payload = serde_json::to_string_pretty(value)?;
    fs::write(path, payload)
        .with_context(|| format!("failed to write JSON file: {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use color_eyre::eyre::{ContextCompat, Result};

    #[test]
    fn duplicate_lease_fixture_selects_deterministic_winner() -> Result<()> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/agent-leases/duplicate-leases.json");
        let snapshot: LeaseSnapshot = read_json(&path)?;
        let now = DateTime::parse_from_rfc3339("2026-04-26T18:45:00Z")?.with_timezone(&Utc);

        let winner = select_winner(&snapshot, "task-review-6854-abc123", "abc123", now)
            .context("winner should exist")?;

        assert_eq!(winner.lease_id, "lease-a");
        Ok(())
    }
}
