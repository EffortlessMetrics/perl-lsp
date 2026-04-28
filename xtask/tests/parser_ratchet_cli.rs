use anyhow::{Context, Result, bail};
use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn parser_ratchet_run_succeeds_from_detached_head_with_explicit_refs() -> Result<()> {
    let repo = TempDir::new()?;
    init_git_repo(repo.path())?;

    fs::write(repo.path().join("file.txt"), "v1\n")?;
    run_git(repo.path(), &["add", "file.txt"])?;
    run_git(repo.path(), &["commit", "-m", "first"])?;

    fs::write(repo.path().join("file.txt"), "v2\n")?;
    run_git(repo.path(), &["add", "file.txt"])?;
    run_git(repo.path(), &["commit", "-m", "second"])?;

    let base_sha = git_stdout(repo.path(), &["rev-parse", "HEAD~1"])?;
    let head_sha = git_stdout(repo.path(), &["rev-parse", "HEAD"])?;
    run_git(repo.path(), &["checkout", "--detach", &head_sha])?;

    let receipt_rel = "target/receipts/parser-ratchet.json";
    let mut cmd = cargo_bin_cmd!("xtask");
    let output = cmd
        .current_dir(repo.path())
        .args([
            "parser-ratchet",
            "run",
            "--profile",
            "pr",
            "--base",
            &base_sha,
            "--head",
            "HEAD",
            "--receipt",
            receipt_rel,
        ])
        .output()?;

    if !output.status.success() {
        bail!("xtask parser-ratchet run failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    let receipt_path = repo.path().join(receipt_rel);
    let receipt: Value = serde_json::from_str(&fs::read_to_string(&receipt_path)?)?;

    assert_eq!(receipt["check"], Value::String("parser-ratchet".to_string()));
    assert_eq!(receipt["profile"], Value::String("pr".to_string()));
    assert_eq!(receipt["base_sha"], Value::String(base_sha));
    assert_eq!(receipt["head_sha"], Value::String(head_sha));
    assert_eq!(receipt["selected"], Value::Bool(false));
    assert_eq!(receipt["verdict"], Value::String("pass".to_string()));

    Ok(())
}

#[test]
fn parser_ratchet_run_force_selected_sets_selected_true() -> Result<()> {
    let repo = TempDir::new()?;
    init_git_repo(repo.path())?;

    fs::write(repo.path().join("file.txt"), "one\n")?;
    run_git(repo.path(), &["add", "file.txt"])?;
    run_git(repo.path(), &["commit", "-m", "first"])?;

    fs::write(repo.path().join("file.txt"), "two\n")?;
    run_git(repo.path(), &["add", "file.txt"])?;
    run_git(repo.path(), &["commit", "-m", "second"])?;

    let base = git_stdout(repo.path(), &["rev-parse", "HEAD~1"])?;

    let receipt_rel = "target/receipts/parser-ratchet-force.json";
    let mut cmd = cargo_bin_cmd!("xtask");
    let output = cmd
        .current_dir(repo.path())
        .args([
            "parser-ratchet",
            "run",
            "--profile",
            "nightly",
            "--base",
            &base,
            "--head",
            "HEAD",
            "--receipt",
            receipt_rel,
            "--force-selected",
        ])
        .output()?;

    if !output.status.success() {
        bail!(
            "xtask parser-ratchet run --force-selected failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let receipt: Value = serde_json::from_str(&fs::read_to_string(repo.path().join(receipt_rel))?)?;
    assert_eq!(receipt["selected"], Value::Bool(true));
    assert_eq!(receipt["profile"], Value::String("nightly".to_string()));

    Ok(())
}

fn init_git_repo(repo: &Path) -> Result<()> {
    run_git(repo, &["init"])?;
    run_git(repo, &["config", "user.name", "xtask-test"])?;
    run_git(repo, &["config", "user.email", "xtask-test@example.com"])?;
    Ok(())
}

fn run_git(repo: &Path, args: &[&str]) -> Result<()> {
    let status = Command::new("git")
        .current_dir(repo)
        .args(args)
        .status()
        .with_context(|| format!("failed to run git {}", args.join(" ")))?;
    if !status.success() {
        bail!("git {} failed", args.join(" "));
    }
    Ok(())
}

fn git_stdout(repo: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .current_dir(repo)
        .args(args)
        .output()
        .with_context(|| format!("failed to run git {}", args.join(" ")))?;
    if !output.status.success() {
        bail!("git {} failed: {}", args.join(" "), String::from_utf8_lossy(&output.stderr));
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}
