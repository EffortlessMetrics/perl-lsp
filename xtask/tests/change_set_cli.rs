// CLI integration tests for `cargo xtask change-set` (#3985 Slice 3A).
//
// #3985 Slice 3A exposes the shared `change_set::resolve_change_set`
// resolver to `hooks/pre-push` through this CLI, so the hook never carries
// its own base-resolution shell algorithm. Before this slice, the hook
// resolved a brand-new branch's diff base as
//
//   git merge-base "$local_sha" origin/master 2>/dev/null || echo "$local_sha"
//
// On this remote (and any fork of it — `origin/master` does not exist,
// only `origin/main` does), `git merge-base ... origin/master` always
// fails, so the fallback fires and compares `$local_sha` against itself —
// an empty self-diff, every time, for every new-branch push.
//
// `test_regression_old_shell_algorithm_self_diffs_new_branch_new_resolver_does_not`
// is the direct regression proof: it reimplements the OLD shell algorithm
// verbatim (as `git` subprocess calls, so it needs no shell interpreter and
// runs identically on Windows/macOS/Linux CI) against a real bare-remote
// git fixture with `origin/main` present and `origin/master` genuinely
// absent, and asserts it produces an empty changed-path set — documenting
// the bug rather than merely asserting it exists. It then runs the real
// `cargo xtask change-set` entry point against the same fixture and asserts
// a non-empty, correct changed-path set.
//
// Root override: `crate::utils::project_root()` resolves from
// `CARGO_MANIFEST_DIR` baked in at compile time (this checkout), not from
// the process's current directory — a fixture repo elsewhere on disk is
// otherwise unreachable from the already-compiled `xtask` test binary. The
// `change-set` subcommand accepts `--root` for exactly this reason (the
// same convention `worktree-cleanup --root` established for #4097).

use anyhow::{Context, Result, bail};
use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn run_git(dir: &Path, args: &[&str]) -> Result<()> {
    let status =
        Command::new("git").current_dir(dir).args(args).status().context("failed to spawn git")?;
    if !status.success() {
        bail!("git {args:?} failed in {}", dir.display());
    }
    Ok(())
}

fn commit_file(dir: &Path, name: &str, contents: &str, message: &str) -> Result<()> {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("failed to create fixture file parent dir")?;
    }
    fs::write(&path, contents).context("failed to write fixture file")?;
    run_git(dir, &["add", name])?;
    run_git(dir, &["commit", "-q", "-m", message])?;
    Ok(())
}

/// A repo with a real `origin` bare remote and `main` pushed. Mirrors the
/// real perl-lsp-swarm topology: `origin/main` exists, `origin/master`
/// never did (verified in `test_fixture_has_no_origin_master`).
fn init_fixture_repo(tmp: &Path) -> Result<PathBuf> {
    let repo = tmp.join("repo");
    fs::create_dir_all(&repo)?;
    if run_git(&repo, &["init", "-q", "-b", "main"]).is_err() {
        // Older git without `-b` on `init`.
        run_git(&repo, &["init", "-q"])?;
        run_git(&repo, &["symbolic-ref", "HEAD", "refs/heads/main"])?;
    }
    run_git(&repo, &["config", "user.email", "test@test.local"])?;
    run_git(&repo, &["config", "user.name", "Test"])?;
    run_git(&repo, &["config", "commit.gpgsign", "false"])?;
    commit_file(&repo, "README.md", "init\n", "init")?;

    let remote = tmp.join("origin.git");
    run_git(tmp, &["init", "-q", "--bare", &remote.to_string_lossy()])?;
    run_git(&repo, &["remote", "add", "origin", &remote.to_string_lossy()])?;
    run_git(&repo, &["push", "-q", "origin", "main"])?;
    Ok(repo)
}

fn head_sha(repo: &Path) -> Result<String> {
    let output = Command::new("git")
        .current_dir(repo)
        .args(["rev-parse", "HEAD"])
        .output()
        .context("failed to run git rev-parse HEAD")?;
    if !output.status.success() {
        bail!("git rev-parse HEAD failed in {}", repo.display());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn git_ref_exists(repo: &Path, reference: &str) -> Result<bool> {
    let status = Command::new("git")
        .current_dir(repo)
        .args(["rev-parse", "--verify", "--quiet", reference])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .context("failed to run git rev-parse --verify")?;
    Ok(status.success())
}

/// Verbatim reimplementation of the pre-#3985-Slice-3A `hooks/pre-push`
/// new-branch base resolution (lines ~92-96 before this PR):
///
/// ```bash
/// if [ "$remote_sha" = "0000...0000" ]; then
///     remote_sha="$(git merge-base "$local_sha" origin/master 2>/dev/null || echo "$local_sha")"
/// fi
/// CHANGED_FILES="$(git diff --name-only "$remote_sha" "$local_sha" 2>/dev/null || true)"
/// ```
///
/// Reimplemented as `git` subprocess calls (not a shelled-out bash script)
/// so this proof runs identically on every CI platform, including the
/// Windows dev machines this repo is built on, without depending on a
/// bash interpreter being on PATH.
fn old_shell_style_new_branch_diff(repo: &Path, local_sha: &str) -> Result<Vec<String>> {
    let merge_base_output = Command::new("git")
        .current_dir(repo)
        .args(["merge-base", local_sha, "origin/master"])
        .output()
        .context("failed to run git merge-base")?;
    let resolved_remote_sha = if merge_base_output.status.success() {
        String::from_utf8_lossy(&merge_base_output.stdout).trim().to_string()
    } else {
        // The `|| echo "$local_sha"` fallback.
        local_sha.to_string()
    };

    let diff_output = Command::new("git")
        .current_dir(repo)
        .args(["diff", "--name-only", &resolved_remote_sha, local_sha])
        .output()
        .context("failed to run git diff")?;
    if !diff_output.status.success() {
        // The `|| true` fallback: the old hook treated a diff failure as
        // an empty changed-file set, not an error.
        return Ok(Vec::new());
    }
    let stdout = String::from_utf8_lossy(&diff_output.stdout);
    Ok(stdout.lines().map(str::to_string).filter(|line| !line.is_empty()).collect())
}

fn run_change_set_paths(
    root: &Path,
    base: &str,
    head: &str,
) -> Result<(bool, Vec<String>, String)> {
    let mut cmd = cargo_bin_cmd!("xtask");
    cmd.args(["change-set", "--base", base, "--head", head, "--format", "paths", "--root"])
        .arg(root);
    let output = cmd.output().context("failed to run cargo xtask change-set")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let paths: Vec<String> =
        stdout.lines().map(str::to_string).filter(|line| !line.is_empty()).collect();
    Ok((output.status.success(), paths, stderr))
}

// ---------------------------------------------------------------------------
// A. Subcommand exists and responds to --help
// ---------------------------------------------------------------------------

#[test]
fn test_change_set_help_shows_expected_flags() -> Result<()> {
    let mut cmd = cargo_bin_cmd!("xtask");
    let output = cmd.args(["change-set", "--help"]).output().context("failed to run --help")?;
    assert!(output.status.success(), "--help should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    for flag in ["--base", "--head", "--format", "--root"] {
        assert!(stdout.contains(flag), "help output should mention {flag}; got: {stdout}");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// B. Fixture sanity — origin/master genuinely does not exist
// ---------------------------------------------------------------------------

#[test]
fn test_fixture_has_no_origin_master() -> Result<()> {
    let tmp = tempfile::tempdir().context("failed to create tempdir")?;
    let repo = init_fixture_repo(tmp.path())?;
    assert!(
        !git_ref_exists(&repo, "origin/master")?,
        "fixture must not have origin/master — the whole point of the fixture is that it doesn't"
    );
    assert!(git_ref_exists(&repo, "origin/main")?, "fixture must have origin/main");
    Ok(())
}

// ---------------------------------------------------------------------------
// C. JSON output shape: {base_sha, head_sha, changed_paths}
// ---------------------------------------------------------------------------

#[test]
fn test_change_set_json_output_has_bounded_shape() -> Result<()> {
    let tmp = tempfile::tempdir().context("failed to create tempdir")?;
    let repo = init_fixture_repo(tmp.path())?;
    run_git(&repo, &["checkout", "-q", "-b", "feature"])?;
    commit_file(&repo, "feature.rs", "fn feature() {}\n", "add feature")?;
    let local_sha = head_sha(&repo)?;

    let mut cmd = cargo_bin_cmd!("xtask");
    cmd.args(["change-set", "--base", "auto", "--head", &local_sha, "--format", "json", "--root"])
        .arg(&repo);
    let output = cmd.output().context("failed to run cargo xtask change-set")?;
    assert!(
        output.status.success(),
        "change-set should exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .with_context(|| format!("JSON parse failed; output was: {stdout}"))?;
    assert!(parsed["base_sha"].is_string(), "base_sha must be a string; got: {parsed}");
    assert!(parsed["head_sha"].is_string(), "head_sha must be a string; got: {parsed}");
    assert!(parsed["changed_paths"].is_array(), "changed_paths must be an array; got: {parsed}");
    let changed_paths = parsed["changed_paths"]
        .as_array()
        .context("changed_paths not an array")?
        .iter()
        .filter_map(|v| v.as_str())
        .collect::<Vec<_>>();
    assert_eq!(changed_paths, vec!["feature.rs"], "unexpected changed_paths: {changed_paths:?}");
    Ok(())
}

// ---------------------------------------------------------------------------
// D. DIRECT REGRESSION PROOF — old shell algorithm self-diffs a new branch
//    to empty; the new resolver does not.
// ---------------------------------------------------------------------------
//
// Mutation-checked: with `old_shell_style_new_branch_diff` swapped in for
// `run_change_set_paths` on the "new path" assertion below (i.e. asserting
// the OLD algorithm's output is non-empty), this test fails — confirming
// the assertion is not a tautology. Reverted before landing; see the
// `assert_ne!` inline comment below for the exact swap performed.
#[test]
fn test_regression_old_shell_algorithm_self_diffs_new_branch_new_resolver_does_not() -> Result<()> {
    let tmp = tempfile::tempdir().context("failed to create tempdir")?;
    let repo = init_fixture_repo(tmp.path())?;

    // A brand-new branch with one committed change, never pushed — exactly
    // the state `hooks/pre-push` sees for a first-time branch push (git
    // sends the all-zero SHA as `$remote_sha` for that ref; `$local_sha`
    // is this branch's HEAD).
    run_git(&repo, &["checkout", "-q", "-b", "new-feature-branch"])?;
    commit_file(&repo, "src/new_module.rs", "pub fn new_thing() {}\n", "add new module")?;
    let local_sha = head_sha(&repo)?;

    // --- OLD PATH: reimplemented pre-#3985-Slice-3A shell algorithm ---
    let old_paths = old_shell_style_new_branch_diff(&repo, &local_sha)?;
    assert!(
        old_paths.is_empty(),
        "BUG DOCUMENTATION: the old shell algorithm (git merge-base \"$local_sha\" \
         origin/master || echo \"$local_sha\", then git diff --name-only against itself) \
         was expected to self-diff to an empty changed-path set for a new branch when \
         origin/master does not exist. Got non-empty: {old_paths:?} — if this fails, the \
         fixture or the reimplementation has drifted from the real bug and this test's \
         premise needs re-verifying, not silently loosening the assertion."
    );

    // --- NEW PATH: the shared #3985 change_set resolver via the xtask CLI ---
    let (success, new_paths, stderr) = run_change_set_paths(&repo, "auto", &local_sha)?;
    assert!(success, "cargo xtask change-set should resolve a new-branch base; stderr: {stderr}");
    assert_eq!(
        new_paths,
        vec!["src/new_module.rs".to_string()],
        "expected the new resolver to report the real changed file for a new-branch push, \
         got: {new_paths:?} (old, broken path reported: {old_paths:?})"
    );

    // The core regression assertion, stated directly: same fixture, same
    // "new branch about to be pushed" scenario — old path empty, new path
    // non-empty.
    assert_ne!(
        old_paths.is_empty(),
        new_paths.is_empty(),
        "old and new paths should disagree on emptiness for this fixture \
         (old: {old_paths:?}, new: {new_paths:?})"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// E. git-state-resolve failure -> NOT PROVEN (loud non-zero exit), never
//    an empty-changed-paths "success".
// ---------------------------------------------------------------------------

#[test]
fn test_change_set_unresolvable_explicit_base_fails_loudly_not_empty_success() -> Result<()> {
    let tmp = tempfile::tempdir().context("failed to create tempdir")?;
    let repo = init_fixture_repo(tmp.path())?;
    run_git(&repo, &["checkout", "-q", "-b", "feature"])?;
    commit_file(&repo, "feature.rs", "fn feature() {}\n", "add feature")?;
    let local_sha = head_sha(&repo)?;

    let (success, paths, stderr) =
        run_change_set_paths(&repo, "origin/definitely-not-fetched", &local_sha)?;
    assert!(
        !success,
        "an unresolvable explicit base must fail loudly (NOT PROVEN), not succeed with an \
         empty changed-paths set; got paths: {paths:?}, stderr: {stderr}"
    );
    assert!(
        paths.is_empty(),
        "on failure stdout must not carry a changed-paths list at all: {paths:?}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// F. Auto-resolution on a genuinely new branch resolves to origin/main,
//    never origin/master (which does not exist on this remote).
// ---------------------------------------------------------------------------

#[test]
fn test_change_set_auto_base_resolves_to_origin_main_sha() -> Result<()> {
    let tmp = tempfile::tempdir().context("failed to create tempdir")?;
    let repo = init_fixture_repo(tmp.path())?;
    let expected_base_sha = head_sha(&repo)?;

    run_git(&repo, &["checkout", "-q", "-b", "feature"])?;
    commit_file(&repo, "feature.rs", "fn feature() {}\n", "add feature")?;
    let local_sha = head_sha(&repo)?;

    let mut cmd = cargo_bin_cmd!("xtask");
    cmd.args(["change-set", "--base", "auto", "--head", &local_sha, "--format", "json", "--root"])
        .arg(&repo);
    let output = cmd.output().context("failed to run cargo xtask change-set")?;
    assert!(
        output.status.success(),
        "change-set should exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .with_context(|| format!("JSON parse failed; output was: {stdout}"))?;
    assert_eq!(
        parsed["base_sha"].as_str(),
        Some(expected_base_sha.as_str()),
        "auto base resolution should land on origin/main's SHA; got: {parsed}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// G. Unrecognized --format value -> loud error, never a silent fallback to
//    JSON (kilocode-bot review finding on PR #4171).
// ---------------------------------------------------------------------------

#[test]
fn test_change_set_unknown_format_fails_loudly_not_silent_json() -> Result<()> {
    let tmp = tempfile::tempdir().context("failed to create tempdir")?;
    let repo = init_fixture_repo(tmp.path())?;
    run_git(&repo, &["checkout", "-q", "-b", "feature"])?;
    commit_file(&repo, "feature.rs", "fn feature() {}\n", "add feature")?;
    let local_sha = head_sha(&repo)?;

    let mut cmd = cargo_bin_cmd!("xtask");
    cmd.args([
        "change-set",
        "--base",
        "auto",
        "--head",
        &local_sha,
        // A plausible typo of "paths" — a shell consumer expecting
        // newline-separated paths must not silently receive JSON instead.
        "--format",
        "pathss",
        "--root",
    ])
    .arg(&repo);
    let output = cmd.output().context("failed to run cargo xtask change-set")?;

    assert!(
        !output.status.success(),
        "an unrecognized --format value must fail loudly, not silently succeed; \
         stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.trim().is_empty(),
        "on an unrecognized --format, stdout must not carry a JSON payload that a caller \
         could misparse as a paths list; got: {stdout}"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("pathss") && stderr.to_lowercase().contains("format"),
        "error should name the offending --format value; got: {stderr}"
    );
    Ok(())
}
