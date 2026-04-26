//! Agent worktree allocator with lease tracking.

use crate::utils::project_root;
use color_eyre::eyre::{Context, Result, bail, eyre};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_TTL_SECS: u64 = 4 * 60 * 60;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeLease {
    pub worktree_id: String,
    pub path: PathBuf,
    pub agent_task_id: String,
    pub pr: u64,
    pub branch: String,
    pub base_sha: String,
    pub owner: String,
    pub lease_expiry_epoch_secs: u64,
    pub last_heartbeat_epoch_secs: u64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct AllocatorState {
    leases: Vec<WorktreeLease>,
}

pub fn acquire(pr: u64, base: String, owner: String, agent_task_id: String) -> Result<()> {
    let root = project_root()?;
    let mut state = load_state(&root)?;
    let worktrees_dir = root.join(".claude").join("worktrees");
    fs::create_dir_all(&worktrees_dir)
        .with_context(|| format!("creating {}", worktrees_dir.display()))?;

    let now = now_secs()?;
    let worktree_id = format!("wt-{}-{}", pr, now);
    let branch = format!("agent/pr-{pr}-{now}");
    ensure_branch_is_not_leased(&state, &branch)?;

    let path = worktrees_dir.join(format!("agent-pr-{pr}-{now}"));
    ensure_not_nested_with_existing(&state, &path)?;

    let base_sha = git_output(&root, ["rev-parse", &base])?;

    let status = Command::new("git")
        .current_dir(&root)
        .args(["worktree", "add", "-b", &branch, &path.display().to_string(), &base])
        .status()
        .context("running git worktree add")?;
    if !status.success() {
        bail!("failed to create worktree at {}", path.display());
    }

    let lease = WorktreeLease {
        worktree_id: worktree_id.clone(),
        path: path.clone(),
        agent_task_id,
        pr,
        branch,
        base_sha,
        owner,
        lease_expiry_epoch_secs: now + DEFAULT_TTL_SECS,
        last_heartbeat_epoch_secs: now,
    };

    state.leases.push(lease.clone());
    save_state(&root, &state)?;
    write_receipt(&root, &lease)?;

    println!("acquired worktree lease {}", lease.worktree_id);
    println!("path: {}", path.display());
    println!("branch: {}", lease.branch);
    println!("base_sha: {}", lease.base_sha);
    Ok(())
}

pub fn release(worktree_id: String, force: bool) -> Result<()> {
    let root = project_root()?;
    let mut state = load_state(&root)?;
    let Some(index) = state.leases.iter().position(|lease| lease.worktree_id == worktree_id) else {
        bail!("unknown worktree lease id");
    };
    let lease = state.leases[index].clone();

    if !force && has_uncommitted_changes(&lease.path)? {
        bail!(
            "refusing to remove {} with uncommitted changes (pass --force to override)",
            lease.path.display()
        );
    }

    println!("removing worktree path: {}", lease.path.display());
    let mut args = vec!["worktree".to_string(), "remove".to_string()];
    if force {
        args.push("--force".to_string());
    }
    args.push(lease.path.to_string_lossy().into_owned());

    let status = Command::new("git").current_dir(&root).args(&args).status()?;
    if !status.success() {
        bail!("git worktree remove failed for {}", lease.path.display());
    }

    state.leases.remove(index);
    save_state(&root, &state)?;
    println!("released worktree lease {}", lease.worktree_id);
    Ok(())
}

pub fn list() -> Result<()> {
    let root = project_root()?;
    let state = load_state(&root)?;
    if state.leases.is_empty() {
        println!("no active worktree leases");
        return Ok(());
    }

    for lease in &state.leases {
        println!(
            "{}\t{}\t{}\t{}\t{}",
            lease.worktree_id,
            lease.pr,
            lease.branch,
            lease.owner,
            lease.path.display()
        );
    }
    Ok(())
}

pub fn gc(stale_only: bool, apply: bool, force: bool) -> Result<()> {
    let root = project_root()?;
    let mut state = load_state(&root)?;
    let now = now_secs()?;

    let stale = state
        .leases
        .iter()
        .filter(|lease| !stale_only || lease.lease_expiry_epoch_secs <= now)
        .cloned()
        .collect::<Vec<_>>();

    if stale.is_empty() {
        println!("no stale leased worktrees found");
        return Ok(());
    }

    println!("stale lease candidates:");
    for lease in &stale {
        println!("- {} ({})", lease.worktree_id, lease.path.display());
    }

    if !apply {
        println!("dry-run mode (no deletion performed). pass --apply to remove.");
        return Ok(());
    }

    let ids: HashSet<&str> = stale.iter().map(|lease| lease.worktree_id.as_str()).collect();
    for lease in &stale {
        if !force && has_uncommitted_changes(&lease.path)? {
            println!("skipping {} due to uncommitted changes (use --force)", lease.path.display());
            continue;
        }
        println!("removing stale worktree path: {}", lease.path.display());
        let mut args = vec!["worktree".to_string(), "remove".to_string()];
        if force {
            args.push("--force".to_string());
        }
        args.push(lease.path.to_string_lossy().into_owned());
        let status = Command::new("git").current_dir(&root).args(&args).status()?;
        if !status.success() {
            bail!("git worktree remove failed for {}", lease.path.display());
        }
    }

    state.leases.retain(|lease| !ids.contains(lease.worktree_id.as_str()));
    save_state(&root, &state)?;
    Ok(())
}

fn write_receipt(root: &Path, lease: &WorktreeLease) -> Result<()> {
    let dir = root.join("target").join("receipts").join("worktree-leases");
    fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    let receipt = serde_json::to_string_pretty(lease)?;
    fs::write(dir.join(format!("{}.json", lease.worktree_id)), receipt)?;
    Ok(())
}

fn ensure_branch_is_not_leased(state: &AllocatorState, branch: &str) -> Result<()> {
    if state.leases.iter().any(|lease| lease.branch == branch) {
        bail!("branch {branch} is already leased as writable");
    }
    Ok(())
}

fn ensure_not_nested_with_existing(state: &AllocatorState, candidate: &Path) -> Result<()> {
    let candidate = normalize(candidate);
    for lease in &state.leases {
        let existing = normalize(&lease.path);
        if candidate.starts_with(&existing) || existing.starts_with(&candidate) {
            bail!(
                "nested worktree paths are not allowed: candidate={} existing={}",
                candidate.display(),
                existing.display()
            );
        }
    }
    Ok(())
}

fn has_uncommitted_changes(path: &Path) -> Result<bool> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["status", "--porcelain"])
        .output()
        .with_context(|| format!("checking git status in {}", path.display()))?;
    if !output.status.success() {
        bail!("git status failed in {}", path.display());
    }
    Ok(!String::from_utf8_lossy(&output.stdout).trim().is_empty())
}

fn git_output<const N: usize>(root: &Path, args: [&str; N]) -> Result<String> {
    let output = Command::new("git").current_dir(root).args(args).output()?;
    if !output.status.success() {
        return Err(eyre!("git command failed"));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn state_path(root: &Path) -> PathBuf {
    root.join(".claude").join("worktrees").join("allocator-state.json")
}

fn load_state(root: &Path) -> Result<AllocatorState> {
    let path = state_path(root);
    if !path.exists() {
        return Ok(AllocatorState::default());
    }
    let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    Ok(serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?)
}

fn save_state(root: &Path, state: &AllocatorState) -> Result<()> {
    let path = state_path(root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(state)?;
    fs::write(&path, text).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

fn normalize(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        normalized.push(component);
    }
    normalized
}

fn now_secs() -> Result<u64> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_branch_is_rejected() -> Result<()> {
        let fixture = include_str!("../../tests/fixtures/worktree-allocator/duplicate-branch.json");
        let state: AllocatorState = serde_json::from_str(fixture)?;
        let err = ensure_branch_is_not_leased(&state, "agent/pr-6853-123").expect_err("must fail");
        assert!(err.to_string().contains("already leased"));
        Ok(())
    }

    #[test]
    fn nested_path_is_rejected() -> Result<()> {
        let fixture = include_str!("../../tests/fixtures/worktree-allocator/nested-worktree.json");
        let state: AllocatorState = serde_json::from_str(fixture)?;
        let candidate = PathBuf::from("/repo/.claude/worktrees/agent-pr-6853-200/subdir");
        let err = ensure_not_nested_with_existing(&state, &candidate).expect_err("must fail");
        assert!(err.to_string().contains("nested worktree"));
        Ok(())
    }

    #[test]
    fn stale_filter_selects_expired_entries() -> Result<()> {
        let fixture = include_str!("../../tests/fixtures/worktree-allocator/stale-leases.json");
        let state: AllocatorState = serde_json::from_str(fixture)?;
        let now = 1_700_000_000_u64;
        let stale = state
            .leases
            .iter()
            .filter(|lease| lease.lease_expiry_epoch_secs <= now)
            .collect::<Vec<_>>();
        assert_eq!(stale.len(), 1);
        assert_eq!(stale[0].worktree_id, "wt-stale");
        Ok(())
    }
}
