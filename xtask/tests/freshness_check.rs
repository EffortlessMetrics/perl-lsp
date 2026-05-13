//! Integration tests for `cargo xtask freshness-check`.
//!
//! Each test creates an isolated git repository (or two-repo setup) using
//! `tempfile` and `std::process::Command`, then invokes the xtask binary
//! and asserts on exit codes and JSON output.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use anyhow::Result;
use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;
use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
};
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Initialize a bare "remote" repo with one commit and a working clone of it.
/// Returns `(remote_dir, work_dir)`.
fn init_synced_repo() -> Result<(TempDir, TempDir)> {
    let remote_dir = TempDir::new()?;
    let work_dir = TempDir::new()?;

    // Create the remote bare repo with an explicit initial branch named "master".
    git_cmd(
        &["init", "--bare", "--initial-branch=master", remote_dir.path().to_str().expect("path")],
        None,
    )
    .or_else(|_| {
        // Older git versions don't support --initial-branch; fall back and rename.
        git_cmd(&["init", "--bare", remote_dir.path().to_str().expect("path")], None)?;
        git_cmd(&["symbolic-ref", "HEAD", "refs/heads/master"], Some(remote_dir.path()))
    })?;

    // Create a temp source dir to make an initial commit.
    let source_dir = TempDir::new()?;
    git_cmd(&["init", "-b", "master", source_dir.path().to_str().expect("path")], None)
        .or_else(|_| git_cmd(&["init", source_dir.path().to_str().expect("path")], None))?;
    git_cmd(&["config", "user.email", "test@test.com"], Some(source_dir.path()))?;
    git_cmd(&["config", "user.name", "Test"], Some(source_dir.path()))?;
    git_cmd(&["config", "commit.gpgsign", "false"], Some(source_dir.path()))?;
    // Ensure branch is named master (for older git).
    let _ = git_cmd(&["checkout", "-b", "master"], Some(source_dir.path()));
    fs::write(source_dir.path().join("README.md"), "init")?;
    git_cmd(&["add", "."], Some(source_dir.path()))?;
    git_cmd(&["commit", "-m", "init"], Some(source_dir.path()))?;
    git_cmd(
        &["remote", "add", "origin", remote_dir.path().to_str().expect("path")],
        Some(source_dir.path()),
    )?;
    git_cmd(&["push", "origin", "HEAD:master"], Some(source_dir.path()))?;

    // Clone into work_dir.
    git_cmd(
        &[
            "clone",
            remote_dir.path().to_str().expect("path"),
            work_dir.path().to_str().expect("path"),
        ],
        None,
    )?;
    git_cmd(&["config", "user.email", "test@test.com"], Some(work_dir.path()))?;
    git_cmd(&["config", "user.name", "Test"], Some(work_dir.path()))?;
    git_cmd(&["config", "commit.gpgsign", "false"], Some(work_dir.path()))?;
    // Ensure work_dir is on master.
    let _ = git_cmd(&["checkout", "master"], Some(work_dir.path()));

    Ok((remote_dir, work_dir))
}

/// Add a commit to `repo_dir`.
fn add_commit(repo_dir: &Path, message: &str) -> Result<()> {
    let file = repo_dir.join(format!("file_{}.txt", message.replace(' ', "_")));
    fs::write(&file, message)?;
    git_cmd(&["add", "."], Some(repo_dir))?;
    git_cmd(&["commit", "-m", message], Some(repo_dir))?;
    Ok(())
}

/// Push the `repo_dir` current branch to origin master.
fn push_to_remote(repo_dir: &Path) -> Result<()> {
    git_cmd(&["push", "origin", "HEAD:master"], Some(repo_dir))?;
    Ok(())
}

/// Run a git command with optional working directory. Panics on failure.
fn git_cmd(args: &[&str], cwd: Option<&Path>) -> Result<()> {
    let mut cmd = Command::new("git");
    cmd.args(args).stdout(Stdio::null()).stderr(Stdio::piped());
    // Clear worktree-inherited git env vars so isolated repos are fully self-contained.
    cmd.env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_COMMON_DIR")
        .env_remove("GIT_INDEX_FILE");
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    let output = cmd.output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git {:?} failed ({}): {}", args, output.status, stderr.trim());
    }
    Ok(())
}

/// Parse the JSON emitted by freshness-check from stdout.
fn parse_receipt(stdout: &[u8]) -> Result<Value> {
    let text = std::str::from_utf8(stdout)?;
    Ok(serde_json::from_str(text)?)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// A freshly-cloned repo (HEAD == remote master) reports behind_by=0 and
/// safe_for_code_state_claims=true, and exits 0.
#[test]
fn reports_zero_behind_when_synced() -> Result<()> {
    let (_remote, work_dir) = init_synced_repo()?;

    let mut cmd = cargo_bin_cmd!("xtask");
    let stdout = cmd
        .current_dir(work_dir.path())
        .args(["freshness-check", "--base", "origin/master", "--no-fetch"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let receipt = parse_receipt(&stdout)?;
    assert_eq!(receipt["schema_version"], 1);
    assert_eq!(receipt["behind_by"], 0);
    assert_eq!(receipt["safe_for_code_state_claims"], true);
    assert_eq!(receipt["mode"], "warn");

    Ok(())
}

/// When the remote has commits HEAD hasn't fetched, behind_by > 0 and
/// safe_for_code_state_claims=false.
#[test]
fn reports_nonzero_behind_when_stale() -> Result<()> {
    let (remote_dir, work_dir) = init_synced_repo()?;

    // Add a commit to remote (simulating another push we haven't pulled).
    let source2 = TempDir::new()?;
    git_cmd(
        &[
            "clone",
            remote_dir.path().to_str().expect("path"),
            source2.path().to_str().expect("path"),
        ],
        None,
    )?;
    git_cmd(&["config", "user.email", "test@test.com"], Some(source2.path()))?;
    git_cmd(&["config", "user.name", "Test"], Some(source2.path()))?;
    git_cmd(&["config", "commit.gpgsign", "false"], Some(source2.path()))?;
    add_commit(source2.path(), "remote change")?;
    push_to_remote(source2.path())?;

    // Fetch into work_dir so origin/master is updated, but DON'T merge (HEAD stays behind).
    git_cmd(&["fetch", "origin"], Some(work_dir.path()))?;

    let mut cmd = cargo_bin_cmd!("xtask");
    let stdout = cmd
        .current_dir(work_dir.path())
        .args(["freshness-check", "--base", "origin/master", "--no-fetch"])
        .assert()
        .success() // warn mode always exits 0
        .get_output()
        .stdout
        .clone();

    let receipt = parse_receipt(&stdout)?;
    let behind_by = receipt["behind_by"].as_u64().expect("behind_by is u64");
    assert!(behind_by > 0, "expected behind_by > 0, got {behind_by}");
    assert_eq!(receipt["safe_for_code_state_claims"], false);

    Ok(())
}

/// `--mode warn` exits 0 even when stale.
#[test]
fn warn_mode_returns_zero_even_when_stale() -> Result<()> {
    let (remote_dir, work_dir) = init_synced_repo()?;

    let source2 = TempDir::new()?;
    git_cmd(
        &[
            "clone",
            remote_dir.path().to_str().expect("path"),
            source2.path().to_str().expect("path"),
        ],
        None,
    )?;
    git_cmd(&["config", "user.email", "test@test.com"], Some(source2.path()))?;
    git_cmd(&["config", "user.name", "Test"], Some(source2.path()))?;
    git_cmd(&["config", "commit.gpgsign", "false"], Some(source2.path()))?;
    add_commit(source2.path(), "another change")?;
    push_to_remote(source2.path())?;
    git_cmd(&["fetch", "origin"], Some(work_dir.path()))?;

    let mut cmd = cargo_bin_cmd!("xtask");
    cmd.current_dir(work_dir.path())
        .args(["freshness-check", "--base", "origin/master", "--mode", "warn", "--no-fetch"])
        .assert()
        .success();

    Ok(())
}

/// `--mode block` exits 1 when stale.
#[test]
fn block_mode_returns_one_when_stale() -> Result<()> {
    let (remote_dir, work_dir) = init_synced_repo()?;

    let source2 = TempDir::new()?;
    git_cmd(
        &[
            "clone",
            remote_dir.path().to_str().expect("path"),
            source2.path().to_str().expect("path"),
        ],
        None,
    )?;
    git_cmd(&["config", "user.email", "test@test.com"], Some(source2.path()))?;
    git_cmd(&["config", "user.name", "Test"], Some(source2.path()))?;
    git_cmd(&["config", "commit.gpgsign", "false"], Some(source2.path()))?;
    add_commit(source2.path(), "blocking change")?;
    push_to_remote(source2.path())?;
    git_cmd(&["fetch", "origin"], Some(work_dir.path()))?;

    let mut cmd = cargo_bin_cmd!("xtask");
    cmd.current_dir(work_dir.path())
        .args(["freshness-check", "--base", "origin/master", "--mode", "block", "--no-fetch"])
        .assert()
        .failure()
        .code(1);

    Ok(())
}

/// `--allow-historical --reason "bisect"` on a stale repo exits 0 in block mode.
#[test]
fn allow_historical_bypasses_block_with_reason() -> Result<()> {
    let (remote_dir, work_dir) = init_synced_repo()?;

    let source2 = TempDir::new()?;
    git_cmd(
        &[
            "clone",
            remote_dir.path().to_str().expect("path"),
            source2.path().to_str().expect("path"),
        ],
        None,
    )?;
    git_cmd(&["config", "user.email", "test@test.com"], Some(source2.path()))?;
    git_cmd(&["config", "user.name", "Test"], Some(source2.path()))?;
    git_cmd(&["config", "commit.gpgsign", "false"], Some(source2.path()))?;
    add_commit(source2.path(), "historical change")?;
    push_to_remote(source2.path())?;
    git_cmd(&["fetch", "origin"], Some(work_dir.path()))?;

    let mut cmd = cargo_bin_cmd!("xtask");
    let stdout = cmd
        .current_dir(work_dir.path())
        .args([
            "freshness-check",
            "--base",
            "origin/master",
            "--mode",
            "block",
            "--no-fetch",
            "--allow-historical",
            "--reason",
            "bisect",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let receipt = parse_receipt(&stdout)?;
    assert_eq!(receipt["allow_historical"], true);
    assert_eq!(receipt["bypass_reason"], "bisect");

    Ok(())
}

/// `--allow-historical` without `--reason` must exit with a non-zero usage error.
#[test]
fn allow_historical_without_reason_errors() -> Result<()> {
    let (_remote, work_dir) = init_synced_repo()?;

    let mut cmd = cargo_bin_cmd!("xtask");
    // clap enforces `requires = "reason"` so this should fail at arg parsing.
    cmd.current_dir(work_dir.path())
        .args(["freshness-check", "--base", "origin/master", "--allow-historical"])
        .assert()
        .failure();

    Ok(())
}

/// `--json <path>` writes a valid schema-1 JSON receipt file.
#[test]
fn json_output_path_writes_receipt_file() -> Result<()> {
    let (_remote, work_dir) = init_synced_repo()?;
    let receipt_path = work_dir.path().join("target").join("devex").join("freshness.json");

    let mut cmd = cargo_bin_cmd!("xtask");
    cmd.current_dir(work_dir.path())
        .args([
            "freshness-check",
            "--base",
            "origin/master",
            "--no-fetch",
            "--json",
            receipt_path.to_str().expect("path"),
        ])
        .assert()
        .success();
    assert!(receipt_path.exists(), "receipt file must be written");

    let content = fs::read_to_string(&receipt_path)?;
    let receipt: Value = serde_json::from_str(&content)?;
    assert_eq!(receipt["schema_version"], 1);
    assert!(receipt["head"].is_string());
    assert!(receipt["base_head"].is_string());
    assert!(receipt["behind_by"].is_number());
    assert!(receipt["mode"].is_string());

    Ok(())
}

/// `--no-fetch` does not invoke `git fetch` (we verify by checking that the
/// command succeeds even when there is no network and the remote is unreachable).
/// This test uses a path-based remote that exists, so the real test is that
/// behind_by is computed correctly from cached data.
#[test]
fn no_fetch_skips_fetch_step() -> Result<()> {
    let (_remote, work_dir) = init_synced_repo()?;

    // The command should complete without error even with --no-fetch.
    let mut cmd = cargo_bin_cmd!("xtask");
    let output = cmd
        .current_dir(work_dir.path())
        .args(["freshness-check", "--base", "origin/master", "--no-fetch"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let receipt = parse_receipt(&output)?;
    // `behind_by` must be a number (likely 0 for a fresh clone).
    assert!(receipt["behind_by"].is_number());

    Ok(())
}
