use std::fs;
use std::path::Path;

use assert_cmd::Command;
use color_eyre::eyre::{Context, Result, bail};
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn parser_ratchet_run_writes_scaffold_receipt() -> Result<()> {
    let receipt = tempfile::NamedTempFile::new()?;

    Command::cargo_bin("xtask")?
        .args([
            "parser-ratchet",
            "run",
            "--profile",
            "pr",
            "--base",
            "HEAD~1",
            "--head",
            "HEAD",
            "--receipt",
            receipt.path().to_string_lossy().as_ref(),
        ])
        .assert()
        .success();

    let parsed: Value = serde_json::from_str(&fs::read_to_string(receipt.path())?)?;
    assert_eq!(parsed["profile"], "pr");
    assert_eq!(parsed["selected"], false);
    assert_eq!(parsed["verdict"], "pass");
    assert!(parsed["base_sha"].as_str().is_some());
    assert!(parsed["head_sha"].as_str().is_some());

    Ok(())
}

#[test]
fn parser_ratchet_detached_head_with_explicit_revisions_succeeds() -> Result<()> {
    let repo = init_git_fixture_repo()?;
    let receipt = repo.path().join("target/receipts/parser-ratchet.json");

    let base = git_stdout(repo.path(), &["rev-parse", "HEAD~1"])?;
    let head = git_stdout(repo.path(), &["rev-parse", "HEAD"])?;

    git(repo.path(), &["checkout", "--detach", "HEAD"])?;

    Command::cargo_bin("xtask")?
        .current_dir(repo.path())
        .args([
            "parser-ratchet",
            "run",
            "--profile",
            "pr",
            "--base",
            &base,
            "--head",
            &head,
            "--receipt",
            receipt.to_string_lossy().as_ref(),
        ])
        .assert()
        .success();

    let parsed: Value = serde_json::from_str(&fs::read_to_string(&receipt)?)?;
    assert_eq!(parsed["selected"], false);
    assert_eq!(parsed["verdict"], "pass");
    assert_eq!(parsed["base_sha"], base);
    assert_eq!(parsed["head_sha"], head);

    Ok(())
}

fn init_git_fixture_repo() -> Result<TempDir> {
    let dir = TempDir::new()?;
    git(dir.path(), &["init"])?;
    git(dir.path(), &["config", "user.name", "CI"])?;
    git(dir.path(), &["config", "user.email", "ci@example.com"])?;

    let file_path = dir.path().join("fixture.txt");
    fs::write(&file_path, "one\n")?;
    git(dir.path(), &["add", "fixture.txt"])?;
    git(dir.path(), &["commit", "-m", "first"])?;

    fs::write(&file_path, "two\n")?;
    git(dir.path(), &["add", "fixture.txt"])?;
    git(dir.path(), &["commit", "-m", "second"])?;

    Ok(dir)
}

fn git(cwd: &Path, args: &[&str]) -> Result<()> {
    let output = std::process::Command::new("git")
        .current_dir(cwd)
        .args(args)
        .output()
        .with_context(|| format!("failed to spawn git {:?}", args))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8(output.stderr).context("git stderr invalid utf-8")?;
    bail!("git {:?} failed: {}", args, stderr.trim());
}

fn git_stdout(cwd: &Path, args: &[&str]) -> Result<String> {
    let output = std::process::Command::new("git")
        .current_dir(cwd)
        .args(args)
        .output()
        .with_context(|| format!("failed to spawn git {:?}", args))?;

    if !output.status.success() {
        let stderr = String::from_utf8(output.stderr).context("git stderr invalid utf-8")?;
        bail!("git {:?} failed: {}", args, stderr.trim());
    }

    String::from_utf8(output.stdout)
        .context("git stdout invalid utf-8")
        .map(|value| value.trim().to_string())
}
