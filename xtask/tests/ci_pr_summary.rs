//! Integration tests for `cargo xtask ci pr-summary --dry-run`.
//!
//! Each test creates an isolated git repository using `tempfile` and
//! `std::process::Command`, then invokes the xtask binary and asserts on
//! exit codes and markdown output.
//!
//! Pattern mirrors `tests/freshness_check.rs`.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use anyhow::Result;
use assert_cmd::cargo::cargo_bin_cmd;
use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
};
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Initialize a minimal git repo with one commit and optional Cargo workspace.
///
/// Returns the working directory `TempDir`.
fn init_repo_with_commit(add_cargo_workspace: bool) -> Result<TempDir> {
    let work_dir = TempDir::new()?;

    // Init git
    git_cmd_in(&["init", "-b", "master"], work_dir.path())
        .or_else(|_| git_cmd_in(&["init"], work_dir.path()))?;
    git_cmd_in(&["config", "user.email", "test@test.com"], work_dir.path())?;
    git_cmd_in(&["config", "user.name", "Test"], work_dir.path())?;
    let _ = git_cmd_in(&["checkout", "-b", "master"], work_dir.path());

    // Write README
    fs::write(work_dir.path().join("README.md"), "# test repo\n")?;

    if add_cargo_workspace {
        // Minimal Cargo workspace
        fs::write(
            work_dir.path().join("Cargo.toml"),
            r#"[workspace]
members = ["fake-crate"]
resolver = "2"
"#,
        )?;
        // Minimal member crate
        let crate_dir = work_dir.path().join("fake-crate");
        fs::create_dir_all(crate_dir.join("src"))?;
        fs::write(
            crate_dir.join("Cargo.toml"),
            r#"[package]
name = "fake-crate"
version = "0.1.0"
edition = "2021"
"#,
        )?;
        fs::write(crate_dir.join("src").join("lib.rs"), "// fake\n")?;
    }

    git_cmd_in(&["add", "."], work_dir.path())?;
    git_cmd_in(&["commit", "-m", "init"], work_dir.path())?;

    Ok(work_dir)
}

/// Add a commit that changes a file in the repo.
fn add_change_commit(repo_dir: &Path, rel_path: &str, content: &str) -> Result<()> {
    let full_path = repo_dir.join(rel_path);
    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&full_path, content)?;
    git_cmd_in(&["add", "."], repo_dir)?;
    git_cmd_in(&["commit", "-m", "add change"], repo_dir)?;
    Ok(())
}

/// Run a git command in a given directory.
fn git_cmd_in(args: &[&str], cwd: &Path) -> Result<()> {
    let mut cmd = Command::new("git");
    cmd.args(args).current_dir(cwd).stdout(Stdio::null()).stderr(Stdio::null());
    let status = cmd.status()?;
    if !status.success() {
        anyhow::bail!("git {:?} failed with {:?}", args, status.code());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Basic test: `--dry-run` outputs markdown with all required sections.
#[test]
fn dry_run_outputs_markdown_with_required_sections() -> Result<()> {
    let work_dir = init_repo_with_commit(false)?;

    // Add a change so the diff is non-empty
    add_change_commit(work_dir.path(), "src/lib.rs", "// changed\n")?;

    let mut cmd = cargo_bin_cmd!("xtask");
    let output = cmd
        .current_dir(work_dir.path())
        .args(["ci-pr-summary", "--base", "HEAD~1", "--dry-run"])
        .output()?;

    assert!(
        output.status.success(),
        "ci-pr-summary --dry-run should exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout)?;

    // All required sections must be present
    assert!(stdout.contains("## Changed Crates"), "missing Changed Crates section");
    assert!(stdout.contains("## Widened Crates"), "missing Widened Crates section");
    assert!(stdout.contains("## Gates Run"), "missing Gates Run section");
    assert!(stdout.contains("## Gates Skipped by Policy"), "missing Gates Skipped section");
    assert!(stdout.contains("## Timing Estimate"), "missing Timing Estimate section");
    assert!(stdout.contains("## Receipts"), "missing Receipts section");

    // dry-run marker must be present
    assert!(
        stdout.contains("dry-run") || stdout.contains("dry_run"),
        "should mention dry-run; got: {stdout}"
    );

    Ok(())
}

/// With no changes vs base (empty diff), the output must gracefully show an empty changeset.
#[test]
fn dry_run_with_no_changes_outputs_empty_changeset_section() -> Result<()> {
    let work_dir = init_repo_with_commit(false)?;

    // No new commits — diff against HEAD itself produces empty output
    let mut cmd = cargo_bin_cmd!("xtask");
    let output = cmd
        .current_dir(work_dir.path())
        // HEAD...HEAD is an empty diff
        .args(["ci-pr-summary", "--base", "HEAD", "--dry-run"])
        .output()?;

    assert!(
        output.status.success(),
        "should exit 0 even on empty diff; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout)?;

    // Must have all sections
    assert!(stdout.contains("## Changed Crates"), "missing Changed Crates section");

    // Must gracefully note the empty diff
    assert!(
        stdout.contains("no crates directly changed")
            || stdout.contains("prose")
            || stdout.contains("0 file"),
        "expected empty changeset note; got: {stdout}"
    );

    Ok(())
}

/// When a policy file is present, the output includes a policy note.
#[test]
fn dry_run_reads_policy_files_when_present() -> Result<()> {
    let work_dir = init_repo_with_commit(false)?;
    add_change_commit(work_dir.path(), "README.md", "# changed\n")?;

    // Write a policy file
    let policy_dir = work_dir.path().join("policy");
    fs::create_dir_all(&policy_dir)?;
    fs::write(
        policy_dir.join("ci-budget.toml"),
        "# placeholder policy\n[budget]\nmax_gate_minutes = 10\n",
    )?;
    git_cmd_in(&["add", "."], work_dir.path())?;
    git_cmd_in(&["commit", "-m", "add policy"], work_dir.path())?;

    let mut cmd = cargo_bin_cmd!("xtask");
    let output = cmd
        .current_dir(work_dir.path())
        .args(["ci-pr-summary", "--base", "HEAD~2", "--dry-run"])
        .output()?;

    assert!(
        output.status.success(),
        "should exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout)?;

    // Policy section should appear when policy file is present
    assert!(
        stdout.contains("## Policy") || stdout.contains("policy/ci-budget.toml"),
        "expected policy note when policy file is present; got: {stdout}"
    );

    Ok(())
}

/// When no policy file exists, the command must not crash; no Policy section expected.
#[test]
fn dry_run_handles_missing_policy_gracefully() -> Result<()> {
    let work_dir = init_repo_with_commit(false)?;
    add_change_commit(work_dir.path(), "src/lib.rs", "// no policy here\n")?;

    // Confirm no policy directory exists
    assert!(!work_dir.path().join("policy").exists(), "policy dir should not exist");

    let mut cmd = cargo_bin_cmd!("xtask");
    let output = cmd
        .current_dir(work_dir.path())
        .args(["ci-pr-summary", "--base", "HEAD~1", "--dry-run"])
        .output()?;

    assert!(
        output.status.success(),
        "should exit 0 when no policy file; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout)?;

    // All sections still present
    assert!(stdout.contains("## Changed Crates"), "missing Changed Crates");
    assert!(stdout.contains("## Gates Run"), "missing Gates Run");
    // No crash, no Policy section (the section only appears when a policy file is found)
    assert!(
        !stdout.contains("## Policy"),
        "Policy section should not appear without policy file; got: {stdout}"
    );

    Ok(())
}

#[test]
fn dry_run_degrades_without_cargo_metadata() -> Result<()> {
    let work_dir = init_repo_with_commit(false)?;
    add_change_commit(work_dir.path(), "notes.md", "changed\n")?;

    let mut cmd = cargo_bin_cmd!("xtask");
    let output = cmd
        .current_dir(work_dir.path())
        .args(["ci-pr-summary", "--base", "HEAD~1", "--dry-run"])
        .output()?;

    assert!(
        output.status.success(),
        "should exit 0 without cargo metadata; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout)?;
    let head_line = stdout
        .lines()
        .find(|line| line.starts_with("**HEAD**: `"))
        .ok_or_else(|| anyhow::anyhow!("dry-run output missing HEAD line"))?;

    assert!(!head_line.contains("`unknown`"), "expected discovered HEAD: {stdout}");
    assert!(
        stdout.contains("**Diff class**: `prose_only` (1 file(s) changed)"),
        "expected prose-only diff class: {stdout}"
    );
    assert!(
        stdout.contains("_(no crates directly changed"),
        "metadata failure should not synthesize changed crates: {stdout}"
    );
    assert!(
        stdout.contains("_(no widening"),
        "metadata failure should not synthesize widened crates: {stdout}"
    );
    for gate in ["`fmt`", "`clippy_scoped`", "`test_scoped`"] {
        assert!(stdout.contains(gate), "missing fallback gate {gate}: {stdout}");
    }
    assert!(!stdout.contains("## Policy"), "unexpected policy section: {stdout}");
    assert!(
        stdout.contains("No learned-estimates file found"),
        "expected missing timing estimate note: {stdout}"
    );

    Ok(())
}

#[test]
fn dry_run_maps_minimal_workspace_metadata() -> Result<()> {
    let work_dir = init_repo_with_commit(false)?;
    fs::create_dir_all(work_dir.path().join("crates/demo/src"))?;
    fs::create_dir_all(work_dir.path().join("docs/ci"))?;
    fs::create_dir_all(work_dir.path().join("policy"))?;
    fs::write(
        work_dir.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/demo\"]\nresolver = \"2\"\n",
    )?;
    fs::write(
        work_dir.path().join("crates/demo/Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.0.0\"\nedition = \"2021\"\n",
    )?;
    fs::write(work_dir.path().join("crates/demo/src/lib.rs"), "pub fn value() -> u8 { 1 }\n")?;
    fs::write(work_dir.path().join("docs/ci/learned-estimates.md"), "# estimates\n")?;
    fs::write(work_dir.path().join("policy/ci-budget.toml"), "[budget]\n")?;
    git_cmd_in(&["add", "."], work_dir.path())?;
    git_cmd_in(&["commit", "-m", "workspace base"], work_dir.path())?;

    fs::write(work_dir.path().join("crates/demo/src/lib.rs"), "pub fn value() -> u8 { 2 }\n")?;
    git_cmd_in(&["add", "."], work_dir.path())?;
    git_cmd_in(&["commit", "-m", "change demo"], work_dir.path())?;

    let mut cmd = cargo_bin_cmd!("xtask");
    let output = cmd
        .current_dir(work_dir.path())
        .args(["ci-pr-summary", "--base", "HEAD~1", "--dry-run"])
        .output()?;

    assert!(
        output.status.success(),
        "should exit 0 with minimal workspace metadata; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains("**Diff class**: `code` (1 file(s) changed)"),
        "expected code diff class: {stdout}"
    );
    assert!(stdout.contains("- `demo` (1 file(s))"), "expected changed crate mapping: {stdout}");
    assert!(stdout.contains("`fmt`"), "fmt gate should always be selected: {stdout}");
    assert!(
        stdout.contains("Policy loaded from `policy/ci-budget.toml`"),
        "expected policy note: {stdout}"
    );
    assert!(
        stdout.contains("Learned-estimates file present"),
        "expected timing sentinel note: {stdout}"
    );

    Ok(())
}

/// Verify the `--help` flag works and mentions both `--base` and `--dry-run`.
#[test]
fn help_shows_base_and_dry_run_flags() -> Result<()> {
    let mut cmd = cargo_bin_cmd!("xtask");
    let output = cmd.args(["ci-pr-summary", "--help"]).output()?;

    assert!(output.status.success(), "help should exit 0");
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("--base") || stdout.contains("base"), "help should mention --base");
    assert!(
        stdout.contains("--dry-run") || stdout.contains("dry"),
        "help should mention --dry-run"
    );

    Ok(())
}
