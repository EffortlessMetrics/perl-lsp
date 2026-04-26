//! Local worktree lease allocator for agent orchestration.

use crate::utils::project_root;
use chrono::{DateTime, Duration, Utc};
use color_eyre::eyre::{Result, bail, eyre};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const LEASE_STATE_PATH: &str = ".claude/worktrees/leases.json";
const LEASE_RECEIPTS_DIR: &str = "target/receipts";
const DEFAULT_TTL_HOURS: i64 = 8;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeLease {
    pub worktree_id: String,
    pub path: PathBuf,
    pub task_id: String,
    pub pr: u64,
    pub branch: String,
    pub base_sha: String,
    pub owner: String,
    pub lease_expiry: DateTime<Utc>,
    pub last_heartbeat: DateTime<Utc>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct LeaseState {
    leases: Vec<WorktreeLease>,
}

#[derive(Debug, Clone)]
pub(crate) struct GitWorktree {
    pub(crate) path: PathBuf,
    pub(crate) branch: Option<String>,
}

pub fn acquire(pr: u64, base: &str, task_id: Option<String>, owner: Option<String>) -> Result<()> {
    let root = project_root()?;
    let now = Utc::now();
    let base_sha = git_stdout(&root, &["rev-parse", base])?;
    let branch = format!("agent/pr-{pr}");

    let existing = git_worktrees(&root)?;
    if duplicate_branch_exists(&existing, &branch) {
        bail!("branch `{branch}` is already checked out in another writable worktree");
    }

    let agent_root = root.join(".claude/worktrees");
    validate_no_nested_agent_worktrees(&existing, &agent_root)?;

    fs::create_dir_all(&agent_root)?;
    fs::create_dir_all(root.join(LEASE_RECEIPTS_DIR))?;

    let worktree_id = format!("pr{pr}-{}", now.timestamp_millis());
    let path = agent_root.join(&worktree_id);

    let add_status = Command::new("git")
        .current_dir(&root)
        .args(["worktree", "add", "-b", &branch])
        .arg(&path)
        .arg(&base_sha)
        .status()?;
    if !add_status.success() {
        bail!("failed to create git worktree at {}", path.display());
    }

    let task_id = task_id.unwrap_or_else(|| format!("pr-{pr}"));
    let owner = owner
        .or_else(|| std::env::var("USER").ok())
        .or_else(|| std::env::var("USERNAME").ok())
        .unwrap_or_else(|| "unknown".to_string());
    let lease = WorktreeLease {
        worktree_id: worktree_id.clone(),
        path: path.clone(),
        task_id,
        pr,
        branch,
        base_sha,
        owner,
        lease_expiry: now + Duration::hours(DEFAULT_TTL_HOURS),
        last_heartbeat: now,
    };

    let mut state = load_state(&root)?;
    state.leases.push(lease.clone());
    persist_state(&root, &state)?;

    let receipt_path =
        root.join(LEASE_RECEIPTS_DIR).join(format!("worktree-lease-{worktree_id}.json"));
    fs::write(&receipt_path, serde_json::to_string_pretty(&lease)?)?;

    println!("acquired {}", lease.worktree_id);
    println!("path: {}", lease.path.display());
    println!("receipt: {}", receipt_path.display());
    Ok(())
}

pub fn release(id: &str, force: bool) -> Result<()> {
    let root = project_root()?;
    let mut state = load_state(&root)?;
    let lease = state
        .leases
        .iter()
        .find(|entry| entry.worktree_id == id)
        .cloned()
        .ok_or_else(|| eyre!("no lease found for id `{id}`"))?;

    if !force && worktree_has_changes(&lease.path)? {
        bail!(
            "worktree {} has uncommitted changes; re-run with --force to remove",
            lease.path.display()
        );
    }

    println!("releasing {}", lease.path.display());
    let mut args = vec!["worktree", "remove"];
    if force {
        args.push("--force");
    }
    args.push(
        lease
            .path
            .to_str()
            .ok_or_else(|| eyre!("path is not valid UTF-8: {}", lease.path.display()))?,
    );

    let status = Command::new("git").current_dir(&root).args(args).status()?;
    if !status.success() {
        bail!("failed to remove worktree {}", lease.path.display());
    }

    state.leases.retain(|entry| entry.worktree_id != id);
    persist_state(&root, &state)?;
    Ok(())
}

pub fn list() -> Result<()> {
    let root = project_root()?;
    let state = load_state(&root)?;
    println!("{}", serde_json::to_string_pretty(&state.leases)?);
    Ok(())
}

pub fn gc(stale_only: bool, apply: bool, force: bool) -> Result<()> {
    let root = project_root()?;
    let mut state = load_state(&root)?;
    let now = Utc::now();

    let candidates = stale_candidates(&state.leases, stale_only, now);

    if candidates.is_empty() {
        println!("no matching worktree leases");
        return Ok(());
    }

    for lease in &candidates {
        println!("candidate: {} ({})", lease.path.display(), lease.worktree_id);
    }

    if !apply {
        println!("dry-run: no worktrees deleted (use --apply to execute)");
        return Ok(());
    }

    let mut removed = Vec::new();
    for lease in candidates {
        if !force && worktree_has_changes(&lease.path)? {
            println!("skipping dirty worktree: {}", lease.path.display());
            continue;
        }

        println!("removing: {}", lease.path.display());
        let mut args = vec!["worktree", "remove"];
        if force {
            args.push("--force");
        }
        args.push(
            lease
                .path
                .to_str()
                .ok_or_else(|| eyre!("path is not valid UTF-8: {}", lease.path.display()))?,
        );

        let status = Command::new("git").current_dir(&root).args(args).status()?;
        if status.success() {
            removed.push(lease.worktree_id.clone());
        } else {
            println!("failed to remove: {}", lease.path.display());
        }
    }

    state.leases.retain(|lease| !removed.iter().any(|id| id == &lease.worktree_id));
    persist_state(&root, &state)?;
    Ok(())
}

fn worktree_has_changes(path: &Path) -> Result<bool> {
    if !path.exists() {
        return Ok(false);
    }

    let output = Command::new("git").current_dir(path).args(["status", "--porcelain"]).output()?;
    if !output.status.success() {
        bail!("failed to inspect git status at {}", path.display());
    }

    Ok(!String::from_utf8(output.stdout)?.trim().is_empty())
}

fn load_state(root: &Path) -> Result<LeaseState> {
    let path = root.join(LEASE_STATE_PATH);
    if !path.exists() {
        return Ok(LeaseState::default());
    }

    let content = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&content)?)
}

fn persist_state(root: &Path, state: &LeaseState) -> Result<()> {
    let path = root.join(LEASE_STATE_PATH);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(state)?)?;
    Ok(())
}

fn git_stdout(root: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git").current_dir(root).args(args).output()?;
    if !output.status.success() {
        bail!("git command failed: git {}", args.join(" "));
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

fn git_worktrees(root: &Path) -> Result<Vec<GitWorktree>> {
    let output =
        Command::new("git").current_dir(root).args(["worktree", "list", "--porcelain"]).output()?;
    if !output.status.success() {
        bail!("failed to list git worktrees");
    }

    parse_git_worktree_porcelain(&String::from_utf8(output.stdout)?)
}

fn validate_no_nested_agent_worktrees(existing: &[GitWorktree], agent_root: &Path) -> Result<()> {
    for worktree in existing {
        if worktree.path.starts_with(agent_root) {
            let Ok(relative) = worktree.path.strip_prefix(agent_root) else {
                continue;
            };
            if relative.components().count() > 1 {
                bail!("nested agent worktree detected at {}", worktree.path.display());
            }
        }
    }
    Ok(())
}

fn parse_git_worktree_porcelain(raw: &str) -> Result<Vec<GitWorktree>> {
    let mut worktrees = Vec::new();
    let mut current_path: Option<PathBuf> = None;
    let mut current_branch: Option<String> = None;

    for line in raw.lines() {
        if line.is_empty() {
            if let Some(path) = current_path.take() {
                worktrees.push(GitWorktree { path, branch: current_branch.take() });
            }
            continue;
        }

        if let Some(path) = line.strip_prefix("worktree ") {
            current_path = Some(PathBuf::from(path));
            continue;
        }

        if let Some(branch) = line.strip_prefix("branch refs/heads/") {
            current_branch = Some(branch.to_string());
        }
    }

    if let Some(path) = current_path {
        worktrees.push(GitWorktree { path, branch: current_branch });
    }

    Ok(worktrees)
}

pub(crate) fn duplicate_branch_exists(worktrees: &[GitWorktree], branch: &str) -> bool {
    worktrees.iter().any(|worktree| worktree.branch.as_deref() == Some(branch))
}

pub(crate) fn stale_candidates(
    leases: &[WorktreeLease],
    stale_only: bool,
    now: DateTime<Utc>,
) -> Vec<WorktreeLease> {
    leases
        .iter()
        .filter(|lease| !stale_only || lease.lease_expiry < now)
        .cloned()
        .collect::<Vec<_>>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[test]
    fn parses_worktree_porcelain_output() -> Result<()> {
        let raw = "worktree /repo\nHEAD abc\nbranch refs/heads/master\n\nworktree /repo/.claude/worktrees/pr1\nHEAD def\nbranch refs/heads/agent/pr-1\n";
        let parsed = parse_git_worktree_porcelain(raw)?;
        assert_eq!(parsed.len(), 2);
        assert!(duplicate_branch_exists(&parsed, "agent/pr-1"));
        Ok(())
    }

    #[derive(Debug, Deserialize)]
    struct FixtureWorktree {
        path: String,
        branch: String,
    }

    #[derive(Debug, Deserialize)]
    struct DuplicateFixture {
        requested_branch: String,
        worktrees: Vec<FixtureWorktree>,
        expected_duplicate: bool,
    }

    #[derive(Debug, Deserialize)]
    struct FixtureLease {
        worktree_id: String,
        lease_expiry: DateTime<Utc>,
    }

    #[derive(Debug, Deserialize)]
    struct StaleFixture {
        now: DateTime<Utc>,
        leases: Vec<FixtureLease>,
        expected_stale_ids: Vec<String>,
    }

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/worktree-allocator")
            .join(name)
    }

    #[test]
    fn duplicate_branch_detection_fixture() -> Result<()> {
        let raw = fs::read_to_string(fixture_path("duplicate-branch.json"))?;
        let fixture: DuplicateFixture = serde_json::from_str(&raw)?;
        let worktrees = fixture
            .worktrees
            .into_iter()
            .map(|entry| GitWorktree { path: entry.path.into(), branch: Some(entry.branch) })
            .collect::<Vec<_>>();
        assert_eq!(
            duplicate_branch_exists(&worktrees, &fixture.requested_branch),
            fixture.expected_duplicate
        );
        Ok(())
    }

    #[test]
    fn unique_branch_fixture_is_not_duplicate() -> Result<()> {
        let raw = fs::read_to_string(fixture_path("unique-branch.json"))?;
        let fixture: DuplicateFixture = serde_json::from_str(&raw)?;
        let worktrees = fixture
            .worktrees
            .into_iter()
            .map(|entry| GitWorktree { path: entry.path.into(), branch: Some(entry.branch) })
            .collect::<Vec<_>>();
        assert_eq!(
            duplicate_branch_exists(&worktrees, &fixture.requested_branch),
            fixture.expected_duplicate
        );
        Ok(())
    }

    #[test]
    fn stale_gc_fixture_selects_only_expired() -> Result<()> {
        let raw = fs::read_to_string(fixture_path("stale-gc-dry-run.json"))?;
        let fixture: StaleFixture = serde_json::from_str(&raw)?;
        let leases = fixture
            .leases
            .into_iter()
            .map(|lease| WorktreeLease {
                worktree_id: lease.worktree_id,
                path: "/tmp/worktree".into(),
                task_id: "task".to_string(),
                pr: 1,
                branch: "agent/pr-1".to_string(),
                base_sha: "0123456789abcdef".to_string(),
                owner: "owner".to_string(),
                lease_expiry: lease.lease_expiry,
                last_heartbeat: fixture.now,
            })
            .collect::<Vec<_>>();
        let stale_ids = stale_candidates(&leases, true, fixture.now)
            .into_iter()
            .map(|lease| lease.worktree_id)
            .collect::<Vec<_>>();
        assert_eq!(stale_ids, fixture.expected_stale_ids);
        Ok(())
    }
}
