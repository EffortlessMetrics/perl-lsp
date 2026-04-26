//! Lease-based worktree allocator for local agent orchestration.

use crate::utils::project_root;
use chrono::{DateTime, Duration, Utc};
use color_eyre::eyre::{Result, bail, eyre};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const LEASES_PATH: &str = ".claude/worktree-allocator/leases.json";
const RECEIPT_PATH: &str = "target/receipts/worktree-lease.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeLease {
    pub worktree_id: String,
    pub path: String,
    pub agent_task_id: String,
    pub pr: u64,
    pub branch: String,
    pub base_sha: String,
    pub owner: String,
    pub lease_expiry: DateTime<Utc>,
    pub last_heartbeat: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LeaseState {
    leases: Vec<WorktreeLease>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorktreeLeaseReceipt {
    action: String,
    worktree_id: String,
    pr: u64,
    branch: String,
    path: String,
    base_sha: String,
    lease_expiry: DateTime<Utc>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct WorktreeEntry {
    path: String,
    branch: Option<String>,
    bare: bool,
    detached: bool,
}

pub fn acquire(pr: u64, base: String, agent_task_id: Option<String>) -> Result<()> {
    let root = project_root()?;
    let mut state = load_state(&root)?;

    let base_sha = resolve_ref(&root, &base)?;
    let worktree_id = generate_worktree_id();
    let branch = format!("pr-{pr}");
    let worktree_path = root.join(".claude/worktrees").join(&worktree_id);
    let worktree_path_str = worktree_path.to_string_lossy().into_owned();

    let entries = list_git_worktrees(&root)?;
    ensure_branch_available(&entries, &branch)?;
    ensure_not_nested(&entries, &worktree_path_str)?;

    fs::create_dir_all(worktree_path.parent().ok_or_else(|| eyre!("missing parent path"))?)?;

    let branch_exists = branch_exists(&root, &branch)?;
    let status = if branch_exists {
        Command::new("git")
            .current_dir(&root)
            .args(["worktree", "add", &worktree_path_str, &branch])
            .status()?
    } else {
        Command::new("git")
            .current_dir(&root)
            .args(["worktree", "add", "-b", &branch, &worktree_path_str, &base_sha])
            .status()?
    };

    if !status.success() {
        bail!("failed to create worktree at {worktree_path_str}");
    }

    let now = Utc::now();
    let lease = WorktreeLease {
        worktree_id: worktree_id.clone(),
        path: worktree_path_str.clone(),
        agent_task_id: agent_task_id.unwrap_or_else(default_agent_task_id),
        pr,
        branch: branch.clone(),
        base_sha: base_sha.clone(),
        owner: default_owner(),
        lease_expiry: now + Duration::hours(8),
        last_heartbeat: now,
    };
    state.leases.push(lease.clone());
    save_state(&root, &state)?;

    let receipt = WorktreeLeaseReceipt {
        action: "acquire".to_string(),
        worktree_id,
        pr,
        branch,
        path: worktree_path_str,
        base_sha,
        lease_expiry: lease.lease_expiry,
    };
    write_receipt(&root, &receipt)?;

    println!("Acquired worktree {} at {}", receipt.worktree_id, receipt.path);
    Ok(())
}

pub fn release(id: String, force: bool) -> Result<()> {
    let root = project_root()?;
    let mut state = load_state(&root)?;

    let Some(idx) = state.leases.iter().position(|lease| lease.worktree_id == id) else {
        bail!("lease id not found");
    };

    let lease = state.leases[idx].clone();
    let path = PathBuf::from(&lease.path);
    println!("Releasing worktree path: {}", lease.path);

    if path.exists() {
        ensure_clean_or_force(&path, force)?;

        let mut args = vec!["worktree", "remove"];
        if force {
            args.push("--force");
        }
        args.push(&lease.path);

        let status = Command::new("git").current_dir(&root).args(args).status()?;
        if !status.success() {
            bail!("failed to remove worktree {}", lease.path);
        }
    }

    state.leases.remove(idx);
    save_state(&root, &state)?;
    println!("Released lease {}", lease.worktree_id);
    Ok(())
}

pub fn list() -> Result<()> {
    let root = project_root()?;
    let state = load_state(&root)?;

    if state.leases.is_empty() {
        println!("No active worktree leases");
        return Ok(());
    }

    for lease in &state.leases {
        println!(
            "{} pr={} branch={} expires={} path={}",
            lease.worktree_id, lease.pr, lease.branch, lease.lease_expiry, lease.path
        );
    }
    Ok(())
}

pub fn gc_stale(apply: bool, force: bool) -> Result<()> {
    let root = project_root()?;
    let mut state = load_state(&root)?;
    let now = Utc::now();

    let stale_ids = state
        .leases
        .iter()
        .filter(|lease| lease.lease_expiry <= now)
        .map(|lease| lease.worktree_id.clone())
        .collect::<Vec<_>>();

    if stale_ids.is_empty() {
        println!("No stale worktrees found");
        return Ok(());
    }

    println!("Stale worktrees ({}):", stale_ids.len());
    for id in &stale_ids {
        if let Some(lease) = state.leases.iter().find(|lease| &lease.worktree_id == id) {
            println!("{} -> {}", lease.worktree_id, lease.path);
        }
    }

    if !apply {
        println!("Dry-run only. Re-run with --apply to remove stale worktrees.");
        return Ok(());
    }

    for id in &stale_ids {
        if let Some(lease) = state.leases.iter().find(|lease| &lease.worktree_id == id) {
            let path = PathBuf::from(&lease.path);
            if path.exists() {
                ensure_clean_or_force(&path, force)?;
                let mut args = vec!["worktree", "remove"];
                if force {
                    args.push("--force");
                }
                args.push(&lease.path);
                let status = Command::new("git").current_dir(&root).args(args).status()?;
                if !status.success() {
                    bail!("failed to remove stale worktree {}", lease.path);
                }
            }
        }
    }

    state.leases.retain(|lease| !stale_ids.contains(&lease.worktree_id));
    save_state(&root, &state)?;
    println!("Removed {} stale worktrees", stale_ids.len());
    Ok(())
}

fn load_state(root: &Path) -> Result<LeaseState> {
    let path = root.join(LEASES_PATH);
    if !path.exists() {
        return Ok(LeaseState { leases: Vec::new() });
    }

    let content = fs::read_to_string(path)?;
    let state: LeaseState = serde_json::from_str(&content)?;
    Ok(state)
}

fn save_state(root: &Path, state: &LeaseState) -> Result<()> {
    let path = root.join(LEASES_PATH);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let body = serde_json::to_string_pretty(state)?;
    fs::write(path, body)?;
    Ok(())
}

fn write_receipt(root: &Path, receipt: &WorktreeLeaseReceipt) -> Result<()> {
    let path = root.join(RECEIPT_PATH);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let body = serde_json::to_string_pretty(receipt)?;
    fs::write(path, body)?;
    Ok(())
}

fn resolve_ref(root: &Path, git_ref: &str) -> Result<String> {
    let output = Command::new("git").current_dir(root).args(["rev-parse", git_ref]).output()?;
    if !output.status.success() {
        bail!("failed to resolve git ref {git_ref}");
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

fn branch_exists(root: &Path, branch: &str) -> Result<bool> {
    let status = Command::new("git")
        .current_dir(root)
        .args(["show-ref", "--verify", "--quiet", &format!("refs/heads/{branch}")])
        .status()?;
    Ok(status.success())
}

fn list_git_worktrees(root: &Path) -> Result<Vec<WorktreeEntry>> {
    let output =
        Command::new("git").current_dir(root).args(["worktree", "list", "--porcelain"]).output()?;
    if !output.status.success() {
        bail!("failed to list worktrees");
    }

    let text = String::from_utf8(output.stdout)?;
    parse_worktree_porcelain(&text)
}

fn parse_worktree_porcelain(input: &str) -> Result<Vec<WorktreeEntry>> {
    let mut entries = Vec::new();
    let mut current_path: Option<String> = None;
    let mut current_branch: Option<String> = None;
    let mut current_bare = false;
    let mut current_detached = false;

    for line in input.lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            if let Some(path_value) = current_path.take() {
                entries.push(WorktreeEntry {
                    path: path_value,
                    branch: current_branch.take(),
                    bare: current_bare,
                    detached: current_detached,
                });
                current_bare = false;
                current_detached = false;
            }
            current_path = Some(path.to_string());
            continue;
        }

        if line == "bare" {
            current_bare = true;
            continue;
        }

        if line == "detached" {
            current_detached = true;
            continue;
        }

        if let Some(head) = line.strip_prefix("branch refs/heads/") {
            current_branch = Some(head.to_string());
        }
    }

    if let Some(path_value) = current_path.take() {
        entries.push(WorktreeEntry {
            path: path_value,
            branch: current_branch,
            bare: current_bare,
            detached: current_detached,
        });
    }

    Ok(entries)
}

fn ensure_branch_available(entries: &[WorktreeEntry], branch: &str) -> Result<()> {
    if entries
        .iter()
        .any(|entry| !entry.bare && !entry.detached && entry.branch.as_deref() == Some(branch))
    {
        bail!("branch {branch} is already checked out in another writable worktree");
    }
    Ok(())
}

fn ensure_not_nested(entries: &[WorktreeEntry], candidate: &str) -> Result<()> {
    let candidate_path = Path::new(candidate);
    for entry in entries {
        let existing = Path::new(&entry.path);
        if candidate_path.starts_with(existing) || existing.starts_with(candidate_path) {
            bail!("nested worktree path detected: candidate={candidate} existing={}", entry.path);
        }
    }
    Ok(())
}

fn ensure_clean_or_force(worktree_path: &Path, force: bool) -> Result<()> {
    let output =
        Command::new("git").current_dir(worktree_path).args(["status", "--porcelain"]).output()?;

    if !output.status.success() {
        bail!("unable to inspect worktree status at {}", worktree_path.display());
    }

    let dirty = !String::from_utf8(output.stdout)?.trim().is_empty();
    if dirty && !force {
        bail!(
            "worktree {} has uncommitted changes; rerun with --force to delete",
            worktree_path.display()
        );
    }
    Ok(())
}

fn generate_worktree_id() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    format!("wt-{nanos:x}")
}

fn default_owner() -> String {
    env::var("USER")
        .or_else(|_| env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown-owner".to_string())
}

fn default_agent_task_id() -> String {
    env::var("CODEX_SESSION_ID")
        .or_else(|_| env::var("CLAUDE_SESSION_ID"))
        .unwrap_or_else(|_| "manual-session".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Deserialize)]
    struct BranchFixture {
        worktrees_porcelain: String,
        target_branch: String,
        expected_conflict: bool,
    }

    fn load_fixture(name: &str) -> Result<BranchFixture> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/worktree-allocator")
            .join(name);
        let body = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&body)?)
    }

    #[test]
    fn detects_duplicate_writable_branch_from_fixture() -> Result<()> {
        let fixture = load_fixture("duplicate-branch.json")?;
        let entries = parse_worktree_porcelain(&fixture.worktrees_porcelain)?;
        let conflict = ensure_branch_available(&entries, &fixture.target_branch).is_err();
        assert_eq!(fixture.expected_conflict, conflict);
        Ok(())
    }

    #[test]
    fn allows_detached_duplicate_branch_from_fixture() -> Result<()> {
        let fixture = load_fixture("detached-no-conflict.json")?;
        let entries = parse_worktree_porcelain(&fixture.worktrees_porcelain)?;
        let conflict = ensure_branch_available(&entries, &fixture.target_branch).is_err();
        assert_eq!(fixture.expected_conflict, conflict);
        Ok(())
    }

    #[test]
    fn stale_gc_dry_run_lists_without_deleting() -> Result<()> {
        let now = Utc::now();
        let stale = WorktreeLease {
            worktree_id: "wt-old".to_string(),
            path: "/tmp/wt-old".to_string(),
            agent_task_id: "agent-1".to_string(),
            pr: 6853,
            branch: "pr-6853".to_string(),
            base_sha: "abc123".to_string(),
            owner: "tester".to_string(),
            lease_expiry: now - Duration::minutes(1),
            last_heartbeat: now - Duration::minutes(1),
        };
        let fresh = WorktreeLease {
            worktree_id: "wt-new".to_string(),
            path: "/tmp/wt-new".to_string(),
            agent_task_id: "agent-2".to_string(),
            pr: 6854,
            branch: "pr-6854".to_string(),
            base_sha: "def456".to_string(),
            owner: "tester".to_string(),
            lease_expiry: now + Duration::minutes(30),
            last_heartbeat: now,
        };

        let stale_ids = vec![stale, fresh]
            .into_iter()
            .filter(|lease| lease.lease_expiry <= now)
            .map(|lease| lease.worktree_id)
            .collect::<Vec<_>>();

        assert_eq!(stale_ids, vec!["wt-old".to_string()]);
        Ok(())
    }
}
