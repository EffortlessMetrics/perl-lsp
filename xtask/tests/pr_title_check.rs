//! Integration tests for `cargo xtask pr title-check`.
//!
//! Each test invokes the xtask binary and asserts on exit codes and output.
//! Tests that require a git repo use `tempfile` + a minimal init sequence.

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

/// Initialize a minimal git repo with one commit in a temp directory.
fn init_git_repo() -> Result<TempDir> {
    let dir = TempDir::new()?;
    git_cmd(&["init", "-b", "master"], dir.path()).or_else(|_| git_cmd(&["init"], dir.path()))?;
    git_cmd(&["config", "user.email", "test@test.com"], dir.path())?;
    git_cmd(&["config", "user.name", "Test"], dir.path())?;
    git_cmd(&["config", "commit.gpgsign", "false"], dir.path())?;
    fs::write(dir.path().join("README.md"), "init")?;
    git_cmd(&["add", "."], dir.path())?;
    git_cmd(&["commit", "-m", "fix(scope): initial commit (#1234)"], dir.path())?;
    Ok(dir)
}

fn git_cmd(args: &[&str], cwd: &Path) -> Result<()> {
    let status = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    if !status.success() {
        anyhow::bail!("git {:?} failed with {:?}", args, status.code());
    }
    Ok(())
}

/// Parse JSON from stdout bytes.
fn parse_json(stdout: &[u8]) -> Result<Value> {
    let text = std::str::from_utf8(stdout)?;
    Ok(serde_json::from_str(text)?)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// A well-formed conventional-commit title with an issue ref passes.
#[test]
fn valid_title_passes() -> Result<()> {
    let mut cmd = cargo_bin_cmd!("xtask");
    cmd.args(["pr", "title-check", "--no-gh", "fix(scope): subject (#1234)"]).assert().success();
    Ok(())
}

/// A title with no issue reference fails (hard failure).
#[test]
fn missing_issue_ref_fails() -> Result<()> {
    let mut cmd = cargo_bin_cmd!("xtask");
    cmd.args(["pr", "title-check", "--no-gh", "fix(scope): subject"]).assert().failure().code(1);
    Ok(())
}

/// A zero-valued issue reference (#0 / #0000) is the sanctioned placeholder
/// for an unknown issue (issue #724). It WARNS but does not fail in default
/// mode (exit 0), so agents never have to guess a real issue number.
#[test]
fn zero_issue_ref_warns_exits_zero() -> Result<()> {
    let mut cmd = cargo_bin_cmd!("xtask");
    cmd.args(["pr", "title-check", "--no-gh", "fix(scope): subject (#0)"]).assert().success();
    Ok(())
}

/// The four-digit placeholder `#0000` is also accepted (warn, exit 0).
#[test]
fn placeholder_0000_warns_exits_zero() -> Result<()> {
    let mut cmd = cargo_bin_cmd!("xtask");
    cmd.args(["pr", "title-check", "--no-gh", "fix(scope): subject (#0000)"]).assert().success();
    Ok(())
}

/// In `--strict` mode, the placeholder reference fails (warns become errors).
#[test]
fn placeholder_fails_in_strict_mode() -> Result<()> {
    let mut cmd = cargo_bin_cmd!("xtask");
    cmd.args(["pr", "title-check", "--no-gh", "--strict", "fix(scope): subject (#0000)"])
        .assert()
        .failure()
        .code(1);
    Ok(())
}

/// JSON receipt for a placeholder reference reports overall=warn and a warn
/// status on the issue-ref-present check, with no real issue_ref captured.
#[test]
fn json_placeholder_reports_warn() -> Result<()> {
    let mut cmd = cargo_bin_cmd!("xtask");
    let stdout = cmd
        .args(["pr", "title-check", "--no-gh", "--json", "fix(scope): subject (#0000)"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let v = parse_json(&stdout)?;
    assert_eq!(v["overall"], "warn", "placeholder ref must yield overall=warn");
    assert!(v["issue_ref"].is_null(), "placeholder must not capture a real issue_ref");
    let checks =
        v["checks"].as_array().ok_or_else(|| anyhow::anyhow!("checks should be an array"))?;
    let present = checks
        .iter()
        .find(|c| c["name"] == "issue-ref-present")
        .ok_or_else(|| anyhow::anyhow!("missing issue-ref-present check"))?;
    assert_eq!(present["status"], "warn", "issue-ref-present must warn for placeholder");
    Ok(())
}

/// A real reference still wins even if a placeholder also appears in the title.
#[test]
fn real_ref_alongside_placeholder_passes_ok() -> Result<()> {
    let mut cmd = cargo_bin_cmd!("xtask");
    let stdout = cmd
        .args(["pr", "title-check", "--no-gh", "--json", "fix(scope): subject (#0000) (#1234)"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let v = parse_json(&stdout)?;
    assert_eq!(v["issue_ref"], 1234, "real reference must take precedence over placeholder");
    Ok(())
}

/// A title that doesn't follow conventional-commit format fails.
#[test]
fn malformed_type_fails() -> Result<()> {
    let mut cmd = cargo_bin_cmd!("xtask");
    cmd.args(["pr", "title-check", "--no-gh", "Bad title format (#1234)"])
        .assert()
        .failure()
        .code(1);
    Ok(())
}

/// A subject longer than 72 chars (before the issue ref) warns in default mode but exits 0.
#[test]
fn subject_too_long_warns_exits_zero() -> Result<()> {
    // Construct a subject that is exactly 73 chars long.
    let long_subject = "a".repeat(73);
    let title = format!("fix(scope): {long_subject} (#1234)");
    let mut cmd = cargo_bin_cmd!("xtask");
    cmd.args(["pr", "title-check", "--no-gh", &title]).assert().success(); // warn mode: exits 0
    Ok(())
}

/// In `--strict` mode, a too-long subject causes exit 1.
#[test]
fn strict_mode_fails_on_length_warn() -> Result<()> {
    let long_subject = "a".repeat(73);
    let title = format!("fix(scope): {long_subject} (#1234)");
    let mut cmd = cargo_bin_cmd!("xtask");
    cmd.args(["pr", "title-check", "--no-gh", "--strict", &title]).assert().failure().code(1);
    Ok(())
}

/// `--json` emits a JSON receipt with schema_version=1 and required fields.
#[test]
fn json_output_schema_v1() -> Result<()> {
    let mut cmd = cargo_bin_cmd!("xtask");
    let stdout = cmd
        .args(["pr", "title-check", "--no-gh", "--json", "fix(scope): subject (#1234)"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let v = parse_json(&stdout)?;
    assert_eq!(v["schema_version"], 1, "schema_version must be 1");
    assert!(v["title"].is_string(), "title must be a string");
    assert!(v["overall"].is_string(), "overall must be a string");
    assert!(v["checks"].is_array(), "checks must be an array");
    assert_eq!(v["issue_ref"], 1234, "issue_ref must be 1234");
    assert_eq!(v["type"], "fix", "type must be fix");
    assert_eq!(v["scope"], "scope", "scope must be scope");
    Ok(())
}

/// JSON receipt includes all check names.
#[test]
fn json_checks_include_required_names() -> Result<()> {
    let mut cmd = cargo_bin_cmd!("xtask");
    let stdout = cmd
        .args(["pr", "title-check", "--no-gh", "--json", "fix(scope): subject (#1234)"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let v = parse_json(&stdout)?;
    let checks =
        v["checks"].as_array().ok_or_else(|| anyhow::anyhow!("checks should be an array"))?;
    let names: Vec<&str> = checks.iter().filter_map(|c| c["name"].as_str()).collect();

    assert!(names.contains(&"issue-ref-present"), "missing issue-ref-present check");
    assert!(names.contains(&"conventional-format"), "missing conventional-format check");
    assert!(names.contains(&"subject-length"), "missing subject-length check");
    Ok(())
}

/// `--no-gh` skips the issue-exists check (status should be `skipped`).
#[test]
fn no_gh_skips_issue_check() -> Result<()> {
    let mut cmd = cargo_bin_cmd!("xtask");
    let stdout = cmd
        .args(["pr", "title-check", "--no-gh", "--json", "fix(scope): subject (#1234)"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let v = parse_json(&stdout)?;
    let checks =
        v["checks"].as_array().ok_or_else(|| anyhow::anyhow!("checks should be an array"))?;
    let issue_exists = checks.iter().find(|c| c["name"] == "issue-exists");

    if let Some(check) = issue_exists {
        assert_eq!(
            check["status"], "skipped",
            "issue-exists check must be skipped when --no-gh is passed"
        );
    }
    // If issue-exists is not emitted at all when skipped, that's also acceptable.
    Ok(())
}

/// When no title arg is provided, the command reads from `git log -1 --pretty=%s`.
/// The HEAD commit has a valid title, so the command should exit 0.
#[test]
fn reads_head_commit_when_no_title_arg() -> Result<()> {
    let repo = init_git_repo()?;

    let mut cmd = cargo_bin_cmd!("xtask");
    cmd.current_dir(repo.path()).args(["pr", "title-check", "--no-gh"]).assert().success();
    Ok(())
}

/// When no title arg is provided and HEAD commit lacks an issue ref, it exits 1.
#[test]
fn reads_head_commit_fails_when_missing_issue_ref() -> Result<()> {
    let dir = TempDir::new()?;
    git_cmd(&["init", "-b", "master"], dir.path()).or_else(|_| git_cmd(&["init"], dir.path()))?;
    git_cmd(&["config", "user.email", "test@test.com"], dir.path())?;
    git_cmd(&["config", "user.name", "Test"], dir.path())?;
    git_cmd(&["config", "commit.gpgsign", "false"], dir.path())?;
    fs::write(dir.path().join("README.md"), "init")?;
    git_cmd(&["add", "."], dir.path())?;
    // Commit with no issue ref.
    git_cmd(&["commit", "-m", "fix(scope): no issue number here"], dir.path())?;

    let mut cmd = cargo_bin_cmd!("xtask");
    cmd.current_dir(dir.path()).args(["pr", "title-check", "--no-gh"]).assert().failure().code(1);
    Ok(())
}

/// JSON overall field is `"ok"` for a valid title.
#[test]
fn json_overall_ok_for_valid_title() -> Result<()> {
    let mut cmd = cargo_bin_cmd!("xtask");
    let stdout = cmd
        .args(["pr", "title-check", "--no-gh", "--json", "feat(parser): add cool thing (#9999)"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let v = parse_json(&stdout)?;
    assert_eq!(v["overall"], "ok");
    Ok(())
}

/// JSON overall field is `"fail"` for a title missing the issue ref.
#[test]
fn json_overall_fail_for_missing_ref() -> Result<()> {
    let mut cmd = cargo_bin_cmd!("xtask");
    let stdout = cmd
        .args(["pr", "title-check", "--no-gh", "--json", "feat(parser): add cool thing"])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();

    let v = parse_json(&stdout)?;
    assert_eq!(v["overall"], "fail");
    Ok(())
}
