//! Integration tests for `cargo xtask freshness-check --binaries`.
//!
//! Each test creates an isolated git repository using `tempfile` and
//! `std::process::Command`, then invokes the xtask binary and asserts on exit
//! codes and JSON output.
//!
//! Binary staleness is simulated by controlling file mtime relative to the
//! HEAD commit timestamp: write the binary before the commit → stale; write
//! after → fresh.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use anyhow::Result;
use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;
use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
    time::{Duration, SystemTime},
};
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Helpers shared with the existing freshness_check tests
// ---------------------------------------------------------------------------

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

fn parse_receipt(stdout: &[u8]) -> Result<Value> {
    let text = std::str::from_utf8(stdout)?;
    Ok(serde_json::from_str(text)?)
}

/// Create a minimal standalone git repo with one commit (no remote needed).
/// Returns the work directory; all freshness checks use `--no-fetch`.
fn init_standalone_repo() -> Result<TempDir> {
    let work_dir = TempDir::new()?;
    git_cmd(&["init", "-b", "master", work_dir.path().to_str().expect("utf8")], None)
        .or_else(|_| git_cmd(&["init", work_dir.path().to_str().expect("utf8")], None))?;
    git_cmd(&["config", "user.email", "test@test.com"], Some(work_dir.path()))?;
    git_cmd(&["config", "user.name", "Test"], Some(work_dir.path()))?;
    // Disable GPG signing — this test environment has a signing server that requires a source.
    git_cmd(&["config", "commit.gpgsign", "false"], Some(work_dir.path()))?;
    let _ = git_cmd(&["checkout", "-b", "master"], Some(work_dir.path()));
    fs::write(work_dir.path().join("README.md"), "init")?;
    git_cmd(&["add", "."], Some(work_dir.path()))?;
    git_cmd(&["commit", "-m", "init"], Some(work_dir.path()))?;
    Ok(work_dir)
}

/// Return the HEAD commit Unix timestamp (seconds) for the given repo.
fn head_commit_time(repo: &Path) -> Result<u64> {
    let output = Command::new("git")
        .args(["log", "-1", "--format=%ct", "HEAD"])
        .current_dir(repo)
        .output()?;
    let ts = std::str::from_utf8(&output.stdout)?.trim().parse::<u64>()?;
    Ok(ts)
}

fn perl_lsp_binary_name() -> String {
    format!("perl-lsp{}", std::env::consts::EXE_SUFFIX)
}

/// Write a fake binary at the host-platform `target/<profile>/perl-lsp*`
/// executable path with a mtime relative to the HEAD commit timestamp.
///
/// `offset_secs > 0` → fresh (mtime is that many seconds after the commit).
/// `offset_secs < 0` → stale (mtime is |offset| seconds before the commit).
fn write_fake_binary(repo: &Path, profile: &str, offset_secs: i64) -> Result<()> {
    let commit_time = head_commit_time(repo)? as i64;
    let target = repo.join("target").join(profile);
    fs::create_dir_all(&target)?;
    let binary_path = target.join(perl_lsp_binary_name());
    fs::write(&binary_path, b"fake binary")?;

    // Set mtime via filetime arithmetic using SystemTime.
    let desired_secs = (commit_time + offset_secs).max(0) as u64;
    let desired_mtime = SystemTime::UNIX_EPOCH + Duration::from_secs(desired_secs);
    filetime::set_file_mtime(&binary_path, filetime::FileTime::from_system_time(desired_mtime))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// With no binary files present, `--binaries` exits 0 and reports
/// `binary_freshness_safe: true` (missing binaries are not stale).
#[test]
fn missing_binaries_are_not_stale() -> Result<()> {
    let work_dir = init_standalone_repo()?;

    let mut cmd = cargo_bin_cmd!("xtask");
    let stdout = cmd
        .current_dir(work_dir.path())
        .args(["freshness-check", "--base", "HEAD", "--no-fetch", "--binaries"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let receipt = parse_receipt(&stdout)?;
    assert_eq!(receipt["binary_freshness_safe"], true);
    let binaries = receipt["binaries_checked"].as_array().expect("array");
    assert_eq!(binaries.len(), 2, "should check debug and release");
    let binary_name = perl_lsp_binary_name();
    for entry in binaries {
        assert!(
            entry["path"].as_str().expect("path").ends_with(&binary_name),
            "path should use the host executable suffix"
        );
        assert_eq!(entry["mtime"], Value::Null, "missing binary has null mtime");
        assert_eq!(entry["stale"], false, "missing binary is not stale");
        assert_eq!(entry["source_sha"], Value::Null);
    }
    Ok(())
}

/// A debug binary whose mtime is newer than HEAD commit time is fresh.
/// `--binaries` exits 0 and reports `binary_freshness_safe: true`.
#[test]
fn fresh_debug_binary_exits_zero() -> Result<()> {
    let work_dir = init_standalone_repo()?;
    // mtime is 60 seconds after the commit → fresh
    write_fake_binary(work_dir.path(), "debug", 60)?;

    let mut cmd = cargo_bin_cmd!("xtask");
    let stdout = cmd
        .current_dir(work_dir.path())
        .args(["freshness-check", "--base", "HEAD", "--no-fetch", "--binaries"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let receipt = parse_receipt(&stdout)?;
    assert_eq!(receipt["binary_freshness_safe"], true);
    let debug_entry = receipt["binaries_checked"]
        .as_array()
        .expect("array")
        .iter()
        .find(|e| e["path"].as_str().map(|p| p.contains("debug")).unwrap_or(false))
        .cloned()
        .expect("debug entry present");
    assert_eq!(debug_entry["stale"], false);
    assert!(debug_entry["mtime"].is_number(), "mtime is set for present binary");
    Ok(())
}

/// A debug binary whose mtime is older than HEAD commit time is stale.
/// `--binaries` exits non-zero.
#[test]
fn stale_debug_binary_exits_nonzero() -> Result<()> {
    let work_dir = init_standalone_repo()?;
    // mtime is 60 seconds before the commit → stale
    write_fake_binary(work_dir.path(), "debug", -60)?;

    let mut cmd = cargo_bin_cmd!("xtask");
    let stdout = cmd
        .current_dir(work_dir.path())
        .args(["freshness-check", "--base", "HEAD", "--no-fetch", "--binaries"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();

    let receipt = parse_receipt(&stdout)?;
    assert_eq!(receipt["binary_freshness_safe"], false);
    let debug_entry = receipt["binaries_checked"]
        .as_array()
        .expect("array")
        .iter()
        .find(|e| e["path"].as_str().map(|p| p.contains("debug")).unwrap_or(false))
        .cloned()
        .expect("debug entry present");
    assert_eq!(debug_entry["stale"], true);
    Ok(())
}

/// A release binary whose mtime is older than HEAD commit time is stale.
/// `--binaries` exits non-zero.
#[test]
fn stale_release_binary_exits_nonzero() -> Result<()> {
    let work_dir = init_standalone_repo()?;
    write_fake_binary(work_dir.path(), "release", -120)?;

    let mut cmd = cargo_bin_cmd!("xtask");
    cmd.current_dir(work_dir.path())
        .args(["freshness-check", "--base", "HEAD", "--no-fetch", "--binaries"])
        .assert()
        .failure();
    Ok(())
}

/// When the debug binary is fresh but the release binary is stale, the whole
/// check fails (`binary_freshness_safe: false`).
#[test]
fn mixed_fresh_and_stale_fails() -> Result<()> {
    let work_dir = init_standalone_repo()?;
    write_fake_binary(work_dir.path(), "debug", 60)?; // fresh
    write_fake_binary(work_dir.path(), "release", -60)?; // stale

    let mut cmd = cargo_bin_cmd!("xtask");
    let stdout = cmd
        .current_dir(work_dir.path())
        .args(["freshness-check", "--base", "HEAD", "--no-fetch", "--binaries"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();

    let receipt = parse_receipt(&stdout)?;
    assert_eq!(receipt["binary_freshness_safe"], false);
    let entries = receipt["binaries_checked"].as_array().expect("array");
    let debug_entry = entries.iter().find(|e| {
        e["path"].as_str().map(|p| p.contains("debug") && !p.contains("release")).unwrap_or(false)
    });
    let release_entry =
        entries.iter().find(|e| e["path"].as_str().map(|p| p.contains("release")).unwrap_or(false));
    assert_eq!(debug_entry.and_then(|e| e["stale"].as_bool()), Some(false));
    assert_eq!(release_entry.and_then(|e| e["stale"].as_bool()), Some(true));
    Ok(())
}

/// Without `--binaries`, the receipt does NOT include `binaries_checked` or
/// `binary_freshness_safe` (they are omitted via serde skip_serializing_if).
#[test]
fn without_flag_receipt_omits_binary_fields() -> Result<()> {
    let work_dir = init_standalone_repo()?;

    let mut cmd = cargo_bin_cmd!("xtask");
    let stdout = cmd
        .current_dir(work_dir.path())
        .args(["freshness-check", "--base", "HEAD", "--no-fetch"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let receipt = parse_receipt(&stdout)?;
    assert!(
        receipt.get("binaries_checked").is_none(),
        "binaries_checked must be absent without --binaries"
    );
    assert!(
        receipt.get("binary_freshness_safe").is_none(),
        "binary_freshness_safe must be absent without --binaries"
    );
    Ok(())
}

/// `$CARGO_TARGET_DIR` override is honoured: when set, binaries are looked up
/// under that directory rather than `target/`.
#[test]
fn cargo_target_dir_override_is_honoured() -> Result<()> {
    let work_dir = init_standalone_repo()?;
    let custom_target = work_dir.path().join("custom_target");
    let debug_dir = custom_target.join("debug");
    fs::create_dir_all(&debug_dir)?;
    // Write a fresh binary inside the custom target dir.
    let binary_path = debug_dir.join(perl_lsp_binary_name());
    fs::write(&binary_path, b"custom target binary")?;

    // Set mtime to 60 seconds after HEAD commit → fresh.
    let commit_time = head_commit_time(work_dir.path())? as i64;
    let desired_secs = (commit_time + 60).max(0) as u64;
    filetime::set_file_mtime(
        &binary_path,
        filetime::FileTime::from_system_time(
            SystemTime::UNIX_EPOCH + Duration::from_secs(desired_secs),
        ),
    )?;

    let mut cmd = cargo_bin_cmd!("xtask");
    let stdout = cmd
        .current_dir(work_dir.path())
        .env("CARGO_TARGET_DIR", custom_target.to_str().expect("utf8"))
        .args(["freshness-check", "--base", "HEAD", "--no-fetch", "--binaries"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let receipt = parse_receipt(&stdout)?;
    assert_eq!(receipt["binary_freshness_safe"], true);
    let entries = receipt["binaries_checked"].as_array().expect("array");
    // All paths should reference the custom_target directory.
    for entry in entries {
        let path = entry["path"].as_str().expect("path string");
        assert!(path.contains("custom_target"), "expected custom_target in path, got {path}");
    }
    Ok(())
}

/// When a stale binary exists but the standard `target/` dir also has a fresh
/// one at the same profile, only the resolved target dir is checked.
/// This test uses the default target dir (no CARGO_TARGET_DIR) with a fresh binary.
#[test]
fn json_output_includes_binary_fields_when_flag_set() -> Result<()> {
    let work_dir = init_standalone_repo()?;
    write_fake_binary(work_dir.path(), "debug", 60)?;

    let receipt_path = work_dir.path().join("receipt.json");
    let mut cmd = cargo_bin_cmd!("xtask");
    cmd.current_dir(work_dir.path())
        .args([
            "freshness-check",
            "--base",
            "HEAD",
            "--no-fetch",
            "--binaries",
            "--json",
            receipt_path.to_str().expect("utf8"),
        ])
        .assert()
        .success();

    assert!(receipt_path.exists());
    let content = fs::read_to_string(&receipt_path)?;
    let receipt: Value = serde_json::from_str(&content)?;
    assert!(receipt.get("binaries_checked").is_some());
    assert!(receipt.get("binary_freshness_safe").is_some());
    assert_eq!(receipt["binary_freshness_safe"], true);
    Ok(())
}
