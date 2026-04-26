use color_eyre::eyre::{Context, Result, eyre};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fs;
use std::path::Path;

use super::agent_receipt::QueueSnapshot;

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
    #[serde(default)]
    pub allowed_mutations: Vec<String>,
    #[serde(default)]
    pub forbidden_mutations: Vec<String>,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentLease {
    pub schema_version: u32,
    pub kind: String,
    pub task_id: String,
    pub lease_id: String,
    pub owner: String,
    pub lane: String,
    pub pr: u64,
    pub head_sha: String,
    pub base_sha: String,
    #[serde(default)]
    pub allowed_mutations: Vec<String>,
    pub issued_at: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LeaseVerification {
    pub status: String,
    pub reason: String,
    pub winning_lease_id: Option<String>,
}

pub fn create(task_path: &Path, output_path: &Path, owner: &str) -> Result<()> {
    let task: AgentTask = read_json(task_path)?;
    let now = chrono::Utc::now();
    let lease = AgentLease {
        schema_version: 1,
        kind: "agent_lease".to_string(),
        task_id: task.task_id.clone(),
        lease_id: format!("lease-{}-{}", task.pr, now.timestamp()),
        owner: owner.to_string(),
        lane: task.lane,
        pr: task.pr,
        head_sha: task.head_sha,
        base_sha: task.base_sha,
        allowed_mutations: task.allowed_mutations,
        issued_at: now.to_rfc3339(),
        expires_at: task.expires_at,
    };

    write_json(output_path, &lease)
}

pub fn verify(lease_path: &Path, snapshot_path: &Path) -> Result<LeaseVerification> {
    let lease: AgentLease = read_json(lease_path)?;
    let snapshot: QueueSnapshot = read_json(snapshot_path)?;
    let captured = chrono::DateTime::parse_from_rfc3339(&snapshot.captured_at)
        .context("snapshot.captured_at must be RFC3339")?;
    let expires = chrono::DateTime::parse_from_rfc3339(&lease.expires_at)
        .context("lease.expires_at must be RFC3339")?;
    if expires <= captured {
        return Ok(LeaseVerification {
            status: "stale".to_string(),
            reason: "lease expired at snapshot time".to_string(),
            winning_lease_id: None,
        });
    }

    let Some(pr) = snapshot.prs.iter().find(|p| p.number == lease.pr) else {
        return Ok(LeaseVerification {
            status: "stale".to_string(),
            reason: "PR missing from snapshot".to_string(),
            winning_lease_id: None,
        });
    };

    if pr.head_sha != lease.head_sha {
        return Ok(LeaseVerification {
            status: "stale".to_string(),
            reason: "head sha mismatch".to_string(),
            winning_lease_id: None,
        });
    }

    let winner =
        select_winner(&lease, snapshot.active_leases.as_deref().unwrap_or(&[]), &captured)?;
    if winner.lease_id != lease.lease_id {
        return Ok(LeaseVerification {
            status: "stale".to_string(),
            reason: "lost deterministic lease tie-break".to_string(),
            winning_lease_id: Some(winner.lease_id),
        });
    }

    Ok(LeaseVerification {
        status: "valid".to_string(),
        reason: "lease matches current snapshot".to_string(),
        winning_lease_id: Some(lease.lease_id),
    })
}

fn select_winner(
    current: &AgentLease,
    candidates: &[AgentLease],
    captured: &chrono::DateTime<chrono::FixedOffset>,
) -> Result<AgentLease> {
    let mut all = candidates
        .iter()
        .filter(|lease| lease.task_id == current.task_id && lease.head_sha == current.head_sha)
        .filter_map(|lease| {
            let expires = chrono::DateTime::parse_from_rfc3339(&lease.expires_at).ok()?;
            if expires <= *captured {
                return None;
            }
            Some(lease.clone())
        })
        .collect::<Vec<_>>();
    all.push(current.clone());

    all.sort_by(|left, right| compare_lease_priority(left, right).unwrap_or(Ordering::Equal));
    all.into_iter().next().ok_or_else(|| eyre!("no non-expired leases available"))
}

fn compare_lease_priority(left: &AgentLease, right: &AgentLease) -> Result<Ordering> {
    let left_issued = chrono::DateTime::parse_from_rfc3339(&left.issued_at)
        .context("left lease issued_at must be RFC3339")?;
    let right_issued = chrono::DateTime::parse_from_rfc3339(&right.issued_at)
        .context("right lease issued_at must be RFC3339")?;
    Ok(left_issued.cmp(&right_issued).then_with(|| left.lease_id.cmp(&right.lease_id)))
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("failed to read JSON file {}", path.display()))?;
    serde_json::from_str(&text)
        .with_context(|| format!("failed to parse JSON file {}", path.display()))
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create output dir {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(value).context("failed to serialize json")?;
    fs::write(path, json).with_context(|| format!("failed to write {}", path.display()))
}
