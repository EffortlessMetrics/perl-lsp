use anyhow::{Context, Result};
use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn parser_ratchet_run_writes_valid_scaffold_receipt() -> Result<()> {
    let temp = TempDir::new()?;
    let repo = temp.path();
    init_repo_with_two_commits(repo)?;

    let receipt_path = repo.join("target/receipts/parser-ratchet.json");

    let output = cargo_bin_cmd!("xtask")
        .current_dir(repo)
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
            receipt_path.to_str().context("receipt path contains non-utf8 characters")?,
        ])
        .output()
        .context("parser-ratchet run should execute")?;

    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let receipt_raw = fs::read_to_string(&receipt_path)?;
    let receipt: Value = serde_json::from_str(&receipt_raw)?;

    assert_eq!(receipt.get("check").and_then(Value::as_str), Some("parser-ratchet"));
    assert_eq!(receipt.get("profile").and_then(Value::as_str), Some("pr"));
    assert_eq!(receipt.get("selected").and_then(Value::as_bool), Some(false));
    assert_eq!(receipt.get("verdict").and_then(Value::as_str), Some("pass"));

    Ok(())
}

#[test]
fn parser_ratchet_run_works_in_detached_head_with_explicit_shas() -> Result<()> {
    let temp = TempDir::new()?;
    let repo = temp.path();
    init_repo_with_two_commits(repo)?;

    run_git(repo, ["checkout", "--detach", "HEAD"])?;

    let receipt_path = repo.join("target/receipts/parser-ratchet-detached.json");

    let output = cargo_bin_cmd!("xtask")
        .current_dir(repo)
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
            receipt_path.to_str().context("receipt path contains non-utf8 characters")?,
            "--force-selected",
        ])
        .output()
        .context("parser-ratchet run should execute in detached HEAD")?;

    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let receipt_raw = fs::read_to_string(&receipt_path)?;
    let receipt: Value = serde_json::from_str(&receipt_raw)?;
    assert_eq!(receipt.get("selected").and_then(Value::as_bool), Some(true));
    assert_eq!(receipt.get("verdict").and_then(Value::as_str), Some("pass"));

    Ok(())
}

fn init_repo_with_two_commits(repo: &Path) -> Result<()> {
    run_git(repo, ["init"])?;
    run_git(repo, ["config", "user.name", "xtask-test"])?;
    run_git(repo, ["config", "user.email", "xtask-test@example.com"])?;

    fs::write(repo.join("sample.txt"), "base\n")?;
    run_git(repo, ["add", "sample.txt"])?;
    run_git(repo, ["commit", "-m", "base"])?;

    fs::write(repo.join("sample.txt"), "head\n")?;
    run_git(repo, ["add", "sample.txt"])?;
    run_git(repo, ["commit", "-m", "head"])?;

    Ok(())
}

fn run_git<const N: usize>(repo: &Path, args: [&str; N]) -> Result<()> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .with_context(|| format!("failed to execute git {:?}", args))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    anyhow::bail!("git {:?} failed: {}", args, stderr);
}
