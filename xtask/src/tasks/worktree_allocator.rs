//! Local agent worktree lease allocator.

use crate::utils::project_root;
use chrono::{DateTime, Duration, Utc};
use clap::Subcommand;
use color_eyre::eyre::{Result, bail, eyre};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const DEFAULT_LEASE_MINUTES: i64 = 120;
const DEFAULT_STALE_TTL_HOURS: i64 = 24;
const STATE_FILE: &str = ".claude/worktrees/lease-state.json";
const RECEIPT_FILE: &str = "target/receipts/worktree-lease.json";

#[derive(Subcommand)]
pub enum AgentWorktreeCommand {
    /// Acquire a lease for an agent worktree.
    Acquire {
        /// Pull request number associated with this lease.
        #[arg(long)]
        pr: u64,
        /// Base reference used to derive base SHA.
        #[arg(long)]
        base: String,
        /// Agent task identifier.
        #[arg(long)]
        task_id: Option<String>,
        /// Owner identity for this lease.
        #[arg(long)]
        owner: Option<String>,
        /// Desired branch name.
        #[arg(long)]
        branch: Option<String>,
        /// Proposed worktree path.
        #[arg(long)]
        path: Option<PathBuf>,
        /// Lease TTL in minutes.
        #[arg(long, default_value_t = DEFAULT_LEASE_MINUTES)]
        ttl_minutes: i64,
    },
    /// Release a lease by worktree id.
    Release {
        #[arg(long)]
        id: String,
    },
    /// List active leases.
    List,
    /// Garbage-collect stale leases and optionally remove worktrees.
    Gc {
        /// GC stale leases/worktrees only.
        #[arg(long)]
        stale: bool,
        /// Actually remove stale worktrees (default is dry-run).
        #[arg(long)]
        apply: bool,
        /// Remove even with uncommitted changes.
        #[arg(long)]
        force: bool,
        /// Treat leases older than this many hours as stale.
        #[arg(long, default_value_t = DEFAULT_STALE_TTL_HOURS)]
        ttl_hours: i64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseState {
    pub leases: Vec<WorktreeLease>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeLease {
    pub worktree_id: String,
    pub path: String,
    pub task_id: String,
    pub pr: u64,
    pub branch: String,
    pub base_sha: String,
    pub owner: String,
    pub lease_expiry: DateTime<Utc>,
    pub last_heartbeat: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorktreeLeaseReceipt {
    worktree_id: String,
    path: String,
    pr: u64,
    branch: String,
    base_sha: String,
    owner: String,
    task_id: String,
    lease_expiry: DateTime<Utc>,
}

pub fn run(command: AgentWorktreeCommand) -> Result<()> {
    match command {
        AgentWorktreeCommand::Acquire { pr, base, task_id, owner, branch, path, ttl_minutes } => {
            acquire(pr, &base, task_id, owner, branch, path, ttl_minutes)
        }
        AgentWorktreeCommand::Release { id } => release(&id),
        AgentWorktreeCommand::List => list(),
        AgentWorktreeCommand::Gc { stale, apply, force, ttl_hours } => {
            gc(stale, apply, force, ttl_hours)
        }
    }
}

fn acquire(
    pr: u64,
    base: &str,
    task_id: Option<String>,
    owner: Option<String>,
    branch: Option<String>,
    path: Option<PathBuf>,
    ttl_minutes: i64,
) -> Result<()> {
    let root = project_root()?;
    let mut state = load_state(&root)?;
    let branch = branch.unwrap_or_else(|| format!("agent/pr-{pr}"));
    let task_id = task_id.unwrap_or_else(|| format!("pr-{pr}"));
    let owner = owner.unwrap_or_else(default_owner);
    let worktree_id = format!("wt-pr-{pr}-{}", short_timestamp());
    let path =
        path.unwrap_or_else(|| root.join(".claude/worktrees").join(format!("agent-{worktree_id}")));
    reject_nested_agent_path(&root, &path)?;
    reject_duplicate_branch(&state, &branch)?;
    reject_existing_git_branch_checkout(&root, &branch)?;

    let now = Utc::now();
    let lease = WorktreeLease {
        worktree_id: worktree_id.clone(),
        path: path.to_string_lossy().to_string(),
        task_id,
        pr,
        branch: branch.clone(),
        base_sha: resolve_base_sha(&root, base)?,
        owner: owner.clone(),
        lease_expiry: now + Duration::minutes(ttl_minutes),
        last_heartbeat: now,
    };
    state.leases.push(lease.clone());
    save_state(&root, &state)?;
    save_receipt(&root, &lease)?;

    println!("acquired {}", lease.worktree_id);
    println!("path: {}", lease.path);
    println!("branch: {}", lease.branch);
    Ok(())
}

fn release(id: &str) -> Result<()> {
    let root = project_root()?;
    let mut state = load_state(&root)?;
    let before = state.leases.len();
    state.leases.retain(|lease| lease.worktree_id != id);
    if before == state.leases.len() {
        bail!("no lease found for id '{id}'");
    }
    save_state(&root, &state)?;
    println!("released {id}");
    Ok(())
}

fn list() -> Result<()> {
    let root = project_root()?;
    let state = load_state(&root)?;
    if state.leases.is_empty() {
        println!("no active leases");
        return Ok(());
    }
    for lease in state.leases {
        println!(
            "{} | pr={} | branch={} | {} | expires={}",
            lease.worktree_id, lease.pr, lease.branch, lease.path, lease.lease_expiry
        );
    }
    Ok(())
}

fn gc(stale_only: bool, apply: bool, force: bool, ttl_hours: i64) -> Result<()> {
    let root = project_root()?;
    let mut state = load_state(&root)?;
    let cutoff = Utc::now() - Duration::hours(ttl_hours);
    let stale_ids = gc_candidates(&state, stale_only, cutoff);

    if stale_ids.is_empty() {
        println!("no leases eligible for gc");
        return Ok(());
    }

    for lease in &state.leases {
        if !stale_ids.contains(&lease.worktree_id) {
            continue;
        }
        println!("candidate: {} -> {}", lease.worktree_id, lease.path);
        if !apply {
            continue;
        }
        if has_uncommitted_changes(&root, &lease.path)? && !force {
            println!("skipping {} (uncommitted changes, use --force)", lease.path);
            continue;
        }
        println!("removing {}", lease.path);
        let status = Command::new("git")
            .current_dir(&root)
            .args(["worktree", "remove"])
            .args(if force { vec!["--force"] } else { Vec::new() })
            .arg(&lease.path)
            .status()?;
        if !status.success() {
            bail!("failed to remove worktree {}", lease.path);
        }
    }

    state.leases.retain(|lease| !stale_ids.contains(&lease.worktree_id) || !apply);
    save_state(&root, &state)?;
    Ok(())
}

fn gc_candidates(state: &LeaseState, stale_only: bool, cutoff: DateTime<Utc>) -> BTreeSet<String> {
    state
        .leases
        .iter()
        .filter(|lease| {
            if stale_only {
                lease.last_heartbeat < cutoff || lease.lease_expiry < Utc::now()
            } else {
                true
            }
        })
        .map(|lease| lease.worktree_id.clone())
        .collect()
}

fn reject_nested_agent_path(root: &Path, candidate: &Path) -> Result<()> {
    let agent_root = root.join(".claude/worktrees");
    if !candidate.starts_with(&agent_root) {
        return Ok(());
    }

    let candidate_norm = candidate.components().collect::<Vec<_>>();
    let agent_norm = agent_root.components().collect::<Vec<_>>();
    if candidate_norm.len() > agent_norm.len() + 1 {
        bail!("nested agent worktrees are not allowed: {}", candidate.display());
    }
    Ok(())
}

fn reject_duplicate_branch(state: &LeaseState, branch: &str) -> Result<()> {
    if state.leases.iter().any(|lease| lease.branch == branch) {
        bail!("branch '{branch}' is already leased");
    }
    Ok(())
}

fn reject_existing_git_branch_checkout(root: &Path, branch: &str) -> Result<()> {
    let output =
        Command::new("git").current_dir(root).args(["worktree", "list", "--porcelain"]).output()?;
    if !output.status.success() {
        return Err(eyre!("failed to query git worktree list"));
    }
    let stdout = String::from_utf8(output.stdout)?;
    for line in stdout.lines() {
        if line.trim_start().starts_with("branch refs/heads/") && line.ends_with(branch) {
            bail!("branch '{branch}' is already checked out in another worktree");
        }
    }
    Ok(())
}

fn has_uncommitted_changes(root: &Path, worktree_path: &str) -> Result<bool> {
    let output = Command::new("git")
        .current_dir(root)
        .args(["-C", worktree_path, "status", "--porcelain"])
        .output()?;
    if !output.status.success() {
        return Ok(false);
    }
    Ok(!String::from_utf8(output.stdout)?.trim().is_empty())
}

fn resolve_base_sha(root: &Path, base: &str) -> Result<String> {
    let output = Command::new("git").current_dir(root).args(["rev-parse", base]).output()?;
    if !output.status.success() {
        bail!("unable to resolve base ref '{base}'");
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

fn load_state(root: &Path) -> Result<LeaseState> {
    let path = root.join(STATE_FILE);
    if !path.exists() {
        return Ok(LeaseState { leases: Vec::new() });
    }
    let data = fs::read_to_string(path)?;
    let parsed = serde_json::from_str::<LeaseState>(&data)?;
    Ok(parsed)
}

fn save_state(root: &Path, state: &LeaseState) -> Result<()> {
    let path = root.join(STATE_FILE);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(state)?)?;
    Ok(())
}

fn save_receipt(root: &Path, lease: &WorktreeLease) -> Result<()> {
    let receipt = WorktreeLeaseReceipt {
        worktree_id: lease.worktree_id.clone(),
        path: lease.path.clone(),
        pr: lease.pr,
        branch: lease.branch.clone(),
        base_sha: lease.base_sha.clone(),
        owner: lease.owner.clone(),
        task_id: lease.task_id.clone(),
        lease_expiry: lease.lease_expiry,
    };
    let path = root.join(RECEIPT_FILE);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(&receipt)?)?;
    Ok(())
}

fn default_owner() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown".to_string())
}

fn short_timestamp() -> i64 {
    Utc::now().timestamp()
}

#[cfg(test)]
mod tests {
    use super::*;
    use color_eyre::eyre::Result;

    #[test]
    fn fixture_duplicate_branch_detected() -> Result<()> {
        let fixture = include_str!("../../tests/fixtures/worktree-allocator/duplicate-branch.json");
        let state: LeaseState = serde_json::from_str(fixture)?;
        let err = reject_duplicate_branch(&state, "agent/pr-101")
            .expect_err("duplicate branch should fail");
        assert!(err.to_string().contains("already leased"));
        Ok(())
    }

    #[test]
    fn fixture_unique_branch_allowed() -> Result<()> {
        let fixture = include_str!("../../tests/fixtures/worktree-allocator/unique-branch.json");
        let state: LeaseState = serde_json::from_str(fixture)?;
        reject_duplicate_branch(&state, "agent/pr-999")?;
        Ok(())
    }

    #[test]
    fn stale_gc_candidates_are_identified_without_deleting() -> Result<()> {
        let fixture = include_str!("../../tests/fixtures/worktree-allocator/gc-stale.json");
        let state: LeaseState = serde_json::from_str(fixture)?;
        let cutoff = DateTime::parse_from_rfc3339("2026-04-30T00:00:00Z")?.with_timezone(&Utc);
        let candidates = gc_candidates(&state, true, cutoff);
        assert!(candidates.contains("wt-stale-1"));
        assert!(!candidates.contains("wt-fresh-1"));
        Ok(())
    }
}
