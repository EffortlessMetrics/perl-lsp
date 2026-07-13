// Worktree-cleanup CLI integration tests (#4097).
//
// Runs the real `cargo xtask worktree-cleanup --root ...` entry point
// against fixture git repositories with real linked worktrees (dirty,
// locked, clean-with-open-PR, clean-with-no-PR), asserting that the guard
// never force-removes a dirty / locked / open-PR / root worktree, and that
// a clean worktree with an affirmative "no open PR" answer from `gh` is
// removed only under `--force` (never under the dry-run default).
//
// `gh` is stubbed via the `XTASK_WORKTREE_CLEANUP_GH_BIN` env var override
// (see xtask/src/tasks/worktrees.rs) rather than PATH manipulation, so the
// exact stub binary (with the correct platform extension) is resolved
// deterministically regardless of PATH/PATHEXT search order.
use anyhow::{Result, bail};
use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const GH_BIN_ENV: &str = "XTASK_WORKTREE_CLEANUP_GH_BIN";

fn run_git(dir: &Path, args: &[&str]) -> Result<()> {
    let status = Command::new("git").current_dir(dir).args(args).status()?;
    if !status.success() {
        bail!("git {args:?} failed in {}", dir.display());
    }
    Ok(())
}

/// Initializes a fresh git repo at `<tmp>/repo` with one commit on `main`,
/// plus a bare `origin` remote with that commit already pushed. Real
/// perl-lsp-swarm worktrees are always clones of a GitHub `origin`, so the
/// fixture mirrors that topology — it's what lets the unpushed-commits
/// guard compare a branch against `origin/main` even when the branch
/// itself never had `git push -u` run against it (the common state for a
/// freshly created agent worktree).
fn init_fixture_repo(tmp: &Path) -> Result<PathBuf> {
    let repo = tmp.join("repo");
    fs::create_dir_all(&repo)?;
    if run_git(&repo, &["init", "-q", "-b", "main"]).is_err() {
        // Older git (< 2.28) without `-b` support on `init`. Plain `git
        // init -q` honors `init.defaultBranch`, which defaults to
        // `master` when unset — not `main`, which every downstream
        // `run_git` call in this fixture (including the final `git push
        // -q origin main`) assumes. Pin the branch name explicitly so the
        // fixture is internally consistent regardless of which init path
        // ran or how `init.defaultBranch` is configured on the host,
        // mirroring the same-crate convention in
        // `xtask/tests/freshness_check.rs`.
        run_git(&repo, &["init", "-q"])?;
        run_git(&repo, &["symbolic-ref", "HEAD", "refs/heads/main"])?;
    }
    run_git(&repo, &["config", "user.email", "test@test.local"])?;
    run_git(&repo, &["config", "user.name", "Test"])?;
    run_git(&repo, &["config", "commit.gpgsign", "false"])?;
    fs::write(repo.join("README.md"), "init\n")?;
    run_git(&repo, &["add", "README.md"])?;
    run_git(&repo, &["commit", "-q", "-m", "init"])?;

    let remote = tmp.join("origin.git");
    run_git(tmp, &["init", "-q", "--bare", &remote.to_string_lossy()])?;
    run_git(&repo, &["remote", "add", "origin", &remote.to_string_lossy()])?;
    run_git(&repo, &["push", "-q", "origin", "main"])?;

    Ok(repo)
}

/// Adds a linked worktree under `<repo>/.claude/worktrees/<name>` on a new
/// branch `<name>`. Returns the worktree's absolute path.
///
/// The branch is created purely locally (no `--track`/upstream), matching
/// how agent worktrees actually come into being: `git worktree add -b` runs
/// before the branch has ever been pushed.
fn add_agent_worktree(repo: &Path, name: &str) -> Result<PathBuf> {
    let wt_path = repo.join(".claude").join("worktrees").join(name);
    run_git(repo, &["worktree", "add", "-q", "-b", name, &wt_path.to_string_lossy()])?;
    Ok(wt_path)
}

/// Commits a new file in the worktree at `path`, without pushing —
/// simulates in-progress agent work that has been committed but never
/// reached any remote.
fn commit_unpushed_change(path: &Path, filename: &str, contents: &str) -> Result<()> {
    fs::write(path.join(filename), contents)?;
    run_git(path, &["add", filename])?;
    run_git(path, &["commit", "-q", "-m", &format!("add {filename}")])?;
    Ok(())
}

/// The worktree's current local `HEAD` commit SHA — used to build a `gh`
/// merged-PR stub response whose `headRefOid` either matches (worktree is
/// exactly at the merged state) or doesn't (worktree has post-merge
/// commits) the worktree's actual HEAD.
fn worktree_head_sha(path: &Path) -> Result<String> {
    let output = Command::new("git").current_dir(path).args(["rev-parse", "HEAD"]).output()?;
    if !output.status.success() {
        bail!("git rev-parse HEAD failed in {}", path.display());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(windows)]
fn write_gh_stub(dir: &Path, exit_code: i32, stdout: &str) -> Result<PathBuf> {
    let path = dir.join("gh.cmd");
    let mut body = String::from("@echo off\r\n");
    if !stdout.is_empty() {
        body.push_str(&format!("echo {stdout}\r\n"));
    }
    body.push_str(&format!("exit /b {exit_code}\r\n"));
    fs::write(&path, body)?;
    Ok(path)
}

#[cfg(unix)]
fn write_gh_stub(dir: &Path, exit_code: i32, stdout: &str) -> Result<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("gh");
    let mut body = String::from("#!/bin/sh\n");
    if !stdout.is_empty() {
        body.push_str(&format!("echo {stdout}\n"));
    }
    body.push_str(&format!("exit {exit_code}\n"));
    fs::write(&path, body)?;
    let mut perms = fs::metadata(&path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms)?;
    Ok(path)
}

/// A `gh` stub that records its own invocation cwd to `marker` and reports
/// no open PR. Used to pin exactly which directory `gh` is invoked from —
/// a cwd-blind stub (like [`write_gh_stub`]) cannot catch a regression
/// where `gh` silently queries the wrong repo.
#[cfg(windows)]
fn write_cwd_probe_gh_stub(dir: &Path, marker: &Path) -> Result<PathBuf> {
    let path = dir.join("gh.cmd");
    let body = format!("@echo off\r\necho %CD%>\"{}\"\r\nexit /b 0\r\n", marker.display());
    fs::write(&path, body)?;
    Ok(path)
}

#[cfg(unix)]
fn write_cwd_probe_gh_stub(dir: &Path, marker: &Path) -> Result<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("gh");
    let body = format!("#!/bin/sh\npwd > \"{}\"\nexit 0\n", marker.display());
    fs::write(&path, body)?;
    let mut perms = fs::metadata(&path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms)?;
    Ok(path)
}

/// A `gh` stub that answers differently depending on the `--state` flag in
/// its invocation: `open_stdout`/`open_exit` for `--state open` (bare text,
/// matching `branch_pr_status`'s `--jq '.[0].number'` post-filter — empty
/// string or a bare PR number), `merged_stdout`/`merged_exit` for `--state
/// merged` (matching `branch_merge_status`'s `--jq '.[0] // empty'` output
/// — empty string for no merged PR, or a single JSON object literal like
/// `{"number":99,"headRefOid":"<sha>"}` for one; never a JSON array).
///
/// Matches on the literal, space-joined token pair `--state merged` (not a
/// bare `merged` substring) — a bare substring match would false-positive
/// on any test branch name that happens to contain the text "merged"
/// (e.g. a branch literally named after a squash-merged PR).
#[cfg(windows)]
fn write_gh_stub_by_state(dir: &Path, open: (i32, &str), merged: (i32, &str)) -> Result<PathBuf> {
    let path = dir.join("gh.cmd");
    let (open_exit, open_stdout) = open;
    let (merged_exit, merged_stdout) = merged;
    let mut body = String::from("@echo off\r\necho %* | findstr /C:\"--state merged\" >nul\r\n");
    body.push_str("if %ERRORLEVEL%==0 (\r\n");
    if !merged_stdout.is_empty() {
        body.push_str(&format!("  echo {merged_stdout}\r\n"));
    }
    body.push_str(&format!("  exit /b {merged_exit}\r\n"));
    body.push_str(") else (\r\n");
    if !open_stdout.is_empty() {
        body.push_str(&format!("  echo {open_stdout}\r\n"));
    }
    body.push_str(&format!("  exit /b {open_exit}\r\n"));
    body.push_str(")\r\n");
    fs::write(&path, body)?;
    Ok(path)
}

#[cfg(unix)]
fn write_gh_stub_by_state(dir: &Path, open: (i32, &str), merged: (i32, &str)) -> Result<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("gh");
    let (open_exit, open_stdout) = open;
    let (merged_exit, merged_stdout) = merged;
    let mut body = String::from("#!/bin/sh\ncase \"$*\" in\n  *\"--state merged\"*)\n");
    if !merged_stdout.is_empty() {
        // Double-quoted: the JSON stub text contains `[`/`{` which a POSIX
        // shell would otherwise be free to glob-expand or word-split.
        body.push_str(&format!("    echo \"{merged_stdout}\"\n"));
    }
    body.push_str(&format!("    exit {merged_exit}\n    ;;\n  *)\n"));
    if !open_stdout.is_empty() {
        body.push_str(&format!("    echo {open_stdout}\n"));
    }
    body.push_str(&format!("    exit {open_exit}\n    ;;\nesac\n"));
    fs::write(&path, body)?;
    let mut perms = fs::metadata(&path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms)?;
    Ok(path)
}

/// A `gh` stub that sleeps for `sleep_secs` seconds when invoked with
/// `--head <branch>` where `branch` contains the literal substring
/// "decoy", and answers instantly with "no PR" for anything else.
///
/// Used to open a deterministic classification-to-removal window: while
/// xtask is still blocked on the decoy worktree's slow `gh` call (during
/// the classify-all pass, which runs entirely before any removal begins),
/// the test injects a filesystem write into an *already-classified*
/// worktree — simulating a concurrent agent write landing in the gap
/// between that worktree's classification and its actual removal.
#[cfg(windows)]
fn write_slow_decoy_gh_stub(dir: &Path, sleep_secs: u32) -> Result<PathBuf> {
    let path = dir.join("gh.cmd");
    let body = format!(
        "@echo off\r\necho %* | findstr /C:\"decoy\" >nul\r\nif %ERRORLEVEL%==0 (ping -n {} 127.0.0.1 >nul)\r\nexit /b 0\r\n",
        sleep_secs + 1
    );
    fs::write(&path, body)?;
    Ok(path)
}

#[cfg(unix)]
fn write_slow_decoy_gh_stub(dir: &Path, sleep_secs: u32) -> Result<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("gh");
    let body =
        format!("#!/bin/sh\ncase \"$*\" in\n  *decoy*) sleep {sleep_secs} ;;\nesac\nexit 0\n");
    fs::write(&path, body)?;
    let mut perms = fs::metadata(&path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms)?;
    Ok(path)
}

/// Spawns (non-blocking) `cargo xtask worktree-cleanup --root <root>
/// --force`, so the caller can inject state changes into the fixture while
/// the child process is still running — needed to prove the
/// classification-to-removal concurrent-write race is handled safely.
fn spawn_xtask_cleanup(root: &Path, gh_bin: &Path) -> Result<std::process::Child> {
    let bin = assert_cmd::cargo_bin!("xtask");
    let mut cmd = Command::new(bin);
    cmd.arg("worktree-cleanup")
        .arg("--root")
        .arg(root)
        .arg("--force")
        .env(GH_BIN_ENV, gh_bin)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    Ok(cmd.spawn()?)
}

fn run_xtask_cleanup(root: &Path, force: bool, gh_bin: Option<&Path>) -> Result<(bool, String)> {
    let mut cmd = cargo_bin_cmd!("xtask");
    cmd.arg("worktree-cleanup").arg("--root").arg(root);
    if force {
        cmd.arg("--force");
    }
    if let Some(gh) = gh_bin {
        cmd.env(GH_BIN_ENV, gh);
    } else {
        // Ensure no ambient override leaks in from the test-runner's own env.
        cmd.env_remove(GH_BIN_ENV);
    }
    let output = cmd.output()?;
    let combined = format!(
        "{}\n---stderr---\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok((output.status.success(), combined))
}

// ── Core safety proof: dirty survives, clean-no-PR is removed ──────────────

#[test]
fn dry_run_keeps_dirty_and_flags_clean_for_removal_without_deleting_either() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let repo = init_fixture_repo(tmp.path())?;
    let dirty_wt = add_agent_worktree(&repo, "wt-dirty")?;
    fs::write(dirty_wt.join("uncommitted.txt"), "uncommitted work\n")?;
    let clean_wt = add_agent_worktree(&repo, "wt-clean")?;

    let gh_stub_dir = tmp.path().join("gh-nopr");
    fs::create_dir_all(&gh_stub_dir)?;
    let gh_stub = write_gh_stub(&gh_stub_dir, 0, "")?;

    let (ok, output) = run_xtask_cleanup(&repo, false, Some(&gh_stub))?;
    assert!(ok, "dry-run must exit 0: {output}");
    assert!(
        output.contains("KEEP") && output.contains("dirty"),
        "expected dirty worktree to be reported KEEP with a dirty reason: {output}"
    );
    assert!(
        output.contains("REMOVE"),
        "expected clean worktree to be reported REMOVE-eligible in dry-run: {output}"
    );
    assert!(dirty_wt.exists(), "dry-run must never delete the dirty worktree");
    assert!(
        clean_wt.exists(),
        "dry-run must never delete anything, including REMOVE-eligible ones"
    );
    Ok(())
}

#[test]
fn force_removes_only_the_clean_worktree_and_never_the_dirty_one() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let repo = init_fixture_repo(tmp.path())?;
    let dirty_wt = add_agent_worktree(&repo, "wt-dirty-force")?;
    fs::write(dirty_wt.join("uncommitted.txt"), "uncommitted work\n")?;
    let clean_wt = add_agent_worktree(&repo, "wt-clean-force")?;

    let gh_stub_dir = tmp.path().join("gh-nopr");
    fs::create_dir_all(&gh_stub_dir)?;
    let gh_stub = write_gh_stub(&gh_stub_dir, 0, "")?;

    let (ok, output) = run_xtask_cleanup(&repo, true, Some(&gh_stub))?;
    assert!(ok, "--force run must exit 0: {output}");

    assert!(
        dirty_wt.exists(),
        "SAFETY VIOLATION: dirty worktree was removed under --force: {output}"
    );
    assert!(
        !clean_wt.exists(),
        "clean, no-open-PR worktree should have been removed under --force: {output}"
    );
    Ok(())
}

// ── Unpushed-commits guard ──────────────────────────────────────────────
//
// A worktree can be clean (no uncommitted changes) yet still hold real,
// unsalvaged work: commits made locally that were never pushed anywhere.
// With no open PR, `PrStatus::None` alone would previously classify this
// as `Remove` — deleting committed-but-unpushed work is exactly as
// destructive as deleting uncommitted work, so it must be kept.

#[test]
fn clean_worktree_with_unpushed_commits_and_no_open_pr_is_kept_not_removed() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let repo = init_fixture_repo(tmp.path())?;
    let wt = add_agent_worktree(&repo, "wt-unpushed")?;
    // Committed, not just uncommitted: worktree_dirty() would report this
    // worktree as clean, so this is exercising the unpushed-commits guard
    // specifically, not the pre-existing dirty guard.
    commit_unpushed_change(&wt, "wip.txt", "unpushed agent work\n")?;

    let gh_stub_dir = tmp.path().join("gh-nopr-unpushed");
    fs::create_dir_all(&gh_stub_dir)?;
    let gh_stub = write_gh_stub(&gh_stub_dir, 0, "")?;

    let (dry_ok, dry_output) = run_xtask_cleanup(&repo, false, Some(&gh_stub))?;
    assert!(dry_ok, "dry-run must exit 0: {dry_output}");
    assert!(
        dry_output.contains("KEEP") && dry_output.contains("unpushed"),
        "expected the committed-but-unpushed worktree to be reported KEEP with an \
         unpushed-commits reason: {dry_output}"
    );

    let (force_ok, force_output) = run_xtask_cleanup(&repo, true, Some(&gh_stub))?;
    assert!(force_ok, "--force run must exit 0: {force_output}");
    assert!(
        wt.exists(),
        "SAFETY VIOLATION: worktree with unpushed commits and no open PR was removed \
         under --force: {force_output}"
    );
    Ok(())
}

// This repo squash-merges exclusively (allow_squash_merge=true,
// delete_branch_on_merge=true, no merge commits in origin/main history).
// A squash-merge commit is never an ancestor of the branch's original
// commits, so `rev-list --count origin/main..HEAD` stays > 0 forever even
// after the PR merges — the unpushed-commits guard alone would misread an
// already-landed, already-safe-to-delete branch as "still has unpushed
// work" and Keep it forever. A merged-PR hit from `gh` must override that
// — but ONLY when the worktree's local HEAD is still exactly the commit
// GitHub recorded as the merged PR's `headRefOid`. The same branch/worktree
// can keep accumulating commits after its PR merges (a builder re-aiming at
// the next round of work before opening the next PR), and those post-merge
// commits are invisible to every other guard: `worktree_dirty` only catches
// *uncommitted* changes, and a merged PR's ancestry makes the
// unpushed-commits check moot. A bare "branch name has A merged PR" check
// (matched by branch name only, ignoring which commit merged) would
// silently destroy that post-merge work — this is the defect this pair of
// tests pins down.

#[test]
fn squash_merged_branch_at_merged_head_is_removed_despite_ahead_by_ancestry() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let repo = init_fixture_repo(tmp.path())?;
    // Deliberately does NOT contain the substring "merged" — the branch
    // name must not collide with the gh-stub's `--state merged` matching.
    let wt = add_agent_worktree(&repo, "wt-landed-pr")?;
    // Committed locally, never pushed to `origin/main` — this makes
    // `rev-list --count origin/main..HEAD` > 0, exactly as it would stay
    // forever for a real squash-merged branch (the squash commit that
    // landed the content is never an ancestor of this commit).
    commit_unpushed_change(&wt, "landed.txt", "already squash-merged upstream\n")?;
    let head_sha = worktree_head_sha(&wt)?;

    let gh_stub_dir = tmp.path().join("gh-landed-pr");
    fs::create_dir_all(&gh_stub_dir)?;
    // No open PR, but PR #99 shows up as merged for this branch's head name
    // (GitHub still resolves it by head-branch name after the ref is
    // deleted, per `delete_branch_on_merge`) — with `headRefOid` exactly
    // matching the worktree's current HEAD (no commits since the merge).
    let merged_json = format!(r#"{{"number":99,"headRefOid":"{head_sha}"}}"#);
    let gh_stub = write_gh_stub_by_state(&gh_stub_dir, (0, ""), (0, &merged_json))?;

    // Match on the worktree's basename, not its full path: `entry.path`
    // (and thus every printed line) always uses forward slashes (git
    // normalizes `worktree list --porcelain` paths that way, even on
    // Windows — see `is_agent_worktree`'s doc comment), but a `PathBuf`
    // built in this test via `.join()` uses the platform separator
    // (backslashes on Windows), so a full-path substring check would
    // never match there.
    let wt_name = wt
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("worktree path has no file name"))?
        .to_string_lossy()
        .to_string();

    let (dry_ok, dry_output) = run_xtask_cleanup(&repo, false, Some(&gh_stub))?;
    assert!(dry_ok, "dry-run must exit 0: {dry_output}");
    assert!(
        dry_output.contains("REMOVE (dry-run: would remove)") && dry_output.contains(&wt_name),
        "expected the squash-merged worktree (HEAD == merged PR's headRefOid) to be reported \
         REMOVE-eligible, not KEEP: {dry_output}"
    );
    assert!(
        !dry_output.contains("unpushed"),
        "must not be classified via the unpushed-commits guard once a merged PR at the \
         current HEAD is confirmed for this branch: {dry_output}"
    );

    let (force_ok, force_output) = run_xtask_cleanup(&repo, true, Some(&gh_stub))?;
    assert!(force_ok, "--force run must exit 0: {force_output}");
    assert!(
        !wt.exists(),
        "squash-merged worktree at the merged PR's exact head (no open PR, ahead-by-ancestry) \
         should have been removed under --force: {force_output}"
    );
    Ok(())
}

#[test]
fn branch_with_merged_pr_but_post_merge_commits_is_kept_not_removed() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let repo = init_fixture_repo(tmp.path())?;
    let wt = add_agent_worktree(&repo, "wt-post-merge-work")?;
    // This is the commit GitHub recorded as the merged PR's headRefOid —
    // captured BEFORE the next commit, so the stub's recorded merge point
    // is stale relative to the worktree's actual (later) HEAD.
    commit_unpushed_change(&wt, "landed.txt", "already squash-merged upstream\n")?;
    let merged_head_sha = worktree_head_sha(&wt)?;
    // A builder re-aimed at the same worktree for the next round of work
    // before opening a new PR — a normal swarm pattern (CLAUDE.md: "re-aim
    // the same builder across churn rounds"). This commit exists nowhere
    // else: no open PR references it, and it's newer than what merged.
    commit_unpushed_change(&wt, "next-round.txt", "new work after the merge\n")?;

    let gh_stub_dir = tmp.path().join("gh-post-merge");
    fs::create_dir_all(&gh_stub_dir)?;
    // No open PR; PR #99 merged, but at the OLDER commit — stale relative
    // to this worktree's current HEAD.
    let merged_json = format!(r#"{{"number":99,"headRefOid":"{merged_head_sha}"}}"#);
    let gh_stub = write_gh_stub_by_state(&gh_stub_dir, (0, ""), (0, &merged_json))?;

    let (dry_ok, dry_output) = run_xtask_cleanup(&repo, false, Some(&gh_stub))?;
    assert!(dry_ok, "dry-run must exit 0: {dry_output}");
    assert!(
        dry_output.contains("KEEP"),
        "expected the worktree with post-merge commits to be reported KEEP, not \
         REMOVE-eligible on the strength of a merged PR whose head it has moved past: \
         {dry_output}"
    );

    let (force_ok, force_output) = run_xtask_cleanup(&repo, true, Some(&gh_stub))?;
    assert!(force_ok, "--force run must exit 0: {force_output}");
    assert!(
        wt.exists(),
        "SAFETY VIOLATION: worktree with post-merge commits (HEAD past the merged PR's \
         headRefOid) was removed under --force: {force_output}"
    );
    Ok(())
}

// Direct coverage for the `MergeStatus::Unknown` path specifically: the
// *open*-PR query succeeds (and reports no open PR), but the *merged*-PR
// query itself fails. This must be indistinguishable from any other
// gh-unavailable case: Keep, never Remove. The existing
// `gh_failure_yields_unknown_pr_status_and_keeps_the_worktree` test doesn't
// exercise this path — it fails BOTH queries, so `PrStatus::Unknown`
// short-circuits `classify()` before `branch_merge_status` is ever called.
#[test]
fn merged_pr_query_failure_yields_unknown_status_and_keeps_the_worktree() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let repo = init_fixture_repo(tmp.path())?;
    let wt = add_agent_worktree(&repo, "wt-merged-query-fails")?;

    let gh_stub_dir = tmp.path().join("gh-merged-query-fails");
    fs::create_dir_all(&gh_stub_dir)?;
    // --state open: succeeds, no open PR. --state merged: exits non-zero
    // (simulates gh auth/network failure specifically on that query).
    let gh_stub = write_gh_stub_by_state(&gh_stub_dir, (0, ""), (1, ""))?;

    let (dry_ok, dry_output) = run_xtask_cleanup(&repo, false, Some(&gh_stub))?;
    assert!(dry_ok, "dry-run must exit 0: {dry_output}");
    assert!(
        dry_output.contains("KEEP"),
        "expected KEEP when the merged-PR query itself fails, even though the open-PR \
         query succeeded with no open PR: {dry_output}"
    );
    assert!(
        dry_output.contains("could not be determined") || dry_output.contains("unavailable"),
        "expected the merged-PR-status-unknown reason to be reported: {dry_output}"
    );

    let (force_ok, force_output) = run_xtask_cleanup(&repo, true, Some(&gh_stub))?;
    assert!(force_ok, "--force run must exit 0: {force_output}");
    assert!(
        wt.exists(),
        "SAFETY VIOLATION: worktree was removed under --force when merged-PR status was \
         unknown: {force_output}"
    );
    Ok(())
}

// ── Concurrent-write-during-removal guard ───────────────────────────────
//
// The classify-all pass runs entirely before the removal loop starts, and
// can take a while (one `gh` round trip per entry). A worktree that was
// clean and Remove-eligible at classification time can be written to by a
// concurrent agent before the removal loop actually reaches it. Removal
// must never pass `--force` to `git worktree remove` — that flag bypasses
// git's own last-resort "this worktree is not clean" refusal, which is the
// only thing standing between that concurrent write and permanent data
// loss. A refusal must be logged and skipped, not treated as fatal for the
// whole cleanup run.

#[test]
fn worktree_dirtied_after_classification_survives_force_cleanup_and_is_skipped() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let repo = init_fixture_repo(tmp.path())?;
    // `git worktree list --porcelain` returns entries in lexicographic path
    // order, so the `a`/`b` prefixes pin which one classifies first.
    //
    // Classified first: clean, no PR, no unpushed commits — genuinely
    // Remove-eligible at classification time.
    let target_wt = add_agent_worktree(&repo, "wt-a-race-target")?;
    // Classified second, with a `gh` stub that sleeps on this worktree's
    // branch name — holds the classify-all pass open long enough for the
    // test to inject a write into `target_wt` after it was already
    // classified Remove-eligible but before the removal loop reaches it.
    let _decoy_wt = add_agent_worktree(&repo, "wt-b-race-decoy")?;

    let gh_stub_dir = tmp.path().join("gh-race");
    fs::create_dir_all(&gh_stub_dir)?;
    let gh_stub = write_slow_decoy_gh_stub(&gh_stub_dir, 5)?;

    let mut child = spawn_xtask_cleanup(&repo, &gh_stub)?;
    let child_stdout =
        child.stdout.take().ok_or_else(|| anyhow::anyhow!("child stdout was not piped"))?;
    let mut child_stderr =
        child.stderr.take().ok_or_else(|| anyhow::anyhow!("child stderr was not piped"))?;

    // Drain stderr on a background thread concurrently with reading stdout
    // below, so a full stderr pipe buffer can never deadlock this test.
    let stderr_handle: std::thread::JoinHandle<Result<String>> = std::thread::spawn(move || {
        use std::io::Read;
        let mut buf = String::new();
        child_stderr.read_to_string(&mut buf)?;
        Ok(buf)
    });

    // `run()`'s classify-all loop prints a `REMOVE`/`KEEP` line for each
    // worktree the instant it finishes classifying — before the removal
    // loop starts (which only happens once every entry, including the
    // slow decoy, has been classified). Rust's stdout is line-buffered, so
    // reading line-by-line here lets us inject the concurrent write
    // exactly once `target_wt`'s classification line has been observed —
    // deterministic regardless of system load, unlike a fixed sleep.
    //
    // Match on the basename, not the full path: every printed line uses
    // forward slashes (git normalizes `worktree list --porcelain` output
    // that way, even on Windows), but `target_wt` was built via `.join()`
    // in this test and so uses the platform separator (backslashes on
    // Windows) — a full-path substring check would never match there.
    use std::io::{BufRead, BufReader};
    let target_marker = target_wt
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("target_wt path has no file name"))?
        .to_string_lossy()
        .to_string();
    let mut stdout_lines = Vec::new();
    let mut injected = false;
    for line in BufReader::new(child_stdout).lines() {
        let line = line?;
        let is_target_line = line.contains(&target_marker);
        stdout_lines.push(line);
        if is_target_line && !injected {
            // Leave the write uncommitted/untracked — that's exactly what
            // makes `git worktree remove` (without `--force`) refuse: it
            // treats an unclean working tree (per `git status
            // --porcelain`) as unsafe to remove, the same signal
            // `worktree_dirty()` uses during classification.
            fs::write(target_wt.join("late-write.txt"), "concurrent agent write\n")?;
            injected = true;
        }
    }

    let status = child.wait()?;
    let stderr_output =
        stderr_handle.join().map_err(|_| anyhow::anyhow!("stderr-draining thread panicked"))??;
    let combined = format!("{}\n---stderr---\n{stderr_output}", stdout_lines.join("\n"));

    assert!(
        injected,
        "never observed target_wt's classification line in stdout — test setup is broken: \
         {combined}"
    );
    assert!(status.success(), "--force run must exit 0 even with a skipped entry: {combined}");
    assert!(
        target_wt.exists(),
        "SAFETY VIOLATION: worktree written to after classification but before removal \
         was destroyed by --force: {combined}"
    );
    assert!(
        combined.contains("skipped") || combined.contains("WARNING"),
        "expected a skip warning for the raced worktree, not silent success: {combined}"
    );
    Ok(())
}

// ── Open-PR guard ────────────────────────────────────────────────────────

#[test]
fn open_pr_worktree_is_never_removed_even_under_force() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let repo = init_fixture_repo(tmp.path())?;
    let pr_wt = add_agent_worktree(&repo, "wt-open-pr")?;

    let gh_stub_dir = tmp.path().join("gh-openpr");
    fs::create_dir_all(&gh_stub_dir)?;
    // Reports PR #123 open for every `gh pr list --head <branch> ...` call.
    let gh_stub = write_gh_stub(&gh_stub_dir, 0, "123")?;

    let (dry_ok, dry_output) = run_xtask_cleanup(&repo, false, Some(&gh_stub))?;
    assert!(dry_ok, "dry-run must exit 0: {dry_output}");
    assert!(
        dry_output.contains("KEEP") && dry_output.contains("open PR"),
        "expected open-PR worktree to be reported KEEP with an open-PR reason: {dry_output}"
    );

    let (force_ok, force_output) = run_xtask_cleanup(&repo, true, Some(&gh_stub))?;
    assert!(force_ok, "--force run must exit 0: {force_output}");
    assert!(
        pr_wt.exists(),
        "SAFETY VIOLATION: worktree with an open PR was removed under --force: {force_output}"
    );
    Ok(())
}

// ── gh must be invoked with the classified repo as cwd, not the ambient
//    process cwd (a `--root` that differs from where the xtask process
//    happens to be launched from must not silently query the wrong repo).

#[test]
fn gh_is_invoked_with_the_classified_root_as_cwd_not_the_ambient_process_cwd() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let repo = init_fixture_repo(tmp.path())?;
    let wt = add_agent_worktree(&repo, "wt-cwd-probe")?;

    // A directory that is NOT the fixture repo — simulates the xtask
    // process being launched from somewhere other than the repo passed
    // via --root (e.g. a different worktree, or the shell's own cwd).
    let ambient_cwd = tmp.path().join("ambient-cwd-decoy");
    fs::create_dir_all(&ambient_cwd)?;

    let marker = tmp.path().join("gh-invocation-cwd.txt");
    let gh_stub_dir = tmp.path().join("gh-cwd-probe");
    fs::create_dir_all(&gh_stub_dir)?;
    let gh_stub = write_cwd_probe_gh_stub(&gh_stub_dir, &marker)?;

    let mut cmd = cargo_bin_cmd!("xtask");
    cmd.current_dir(&ambient_cwd)
        .arg("worktree-cleanup")
        .arg("--root")
        .arg(&repo)
        .arg("--force")
        .env(GH_BIN_ENV, &gh_stub);
    let output = cmd.output()?;
    let combined = format!(
        "{}\n---stderr---\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.status.success(), "run must exit 0: {combined}");

    let recorded = fs::read_to_string(&marker)
        .map_err(|e| anyhow::anyhow!("gh stub did not write its cwd marker: {e}\n{combined}"))?;
    let recorded_cwd = PathBuf::from(recorded.trim());
    let recorded_canon = recorded_cwd.canonicalize().map_err(|e| {
        anyhow::anyhow!("recorded cwd '{}' does not canonicalize: {e}", recorded_cwd.display())
    })?;
    let expected_canon = repo.canonicalize()?;

    assert_eq!(
        recorded_canon,
        expected_canon,
        "gh must be invoked with cwd = the classified repo root ({}), not the ambient \
         process cwd ({}); gh actually ran in: {}\n{combined}",
        expected_canon.display(),
        ambient_cwd.display(),
        recorded_cwd.display()
    );

    // Sanity: the stub reports "no PR", so with a correctly-scoped gh call
    // the worktree is still eligible for removal (this test is about cwd
    // correctness, not about the open-PR guard itself).
    assert!(!wt.exists(), "expected the no-PR worktree to be removed under --force: {combined}");
    Ok(())
}

// ── Locked guard ─────────────────────────────────────────────────────────

#[test]
fn locked_worktree_is_never_removed_even_under_force() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let repo = init_fixture_repo(tmp.path())?;
    let locked_wt = add_agent_worktree(&repo, "wt-locked")?;
    run_git(
        &repo,
        &[
            "worktree",
            "lock",
            "--reason",
            "claude agent wt-locked (pid 1)",
            &locked_wt.to_string_lossy(),
        ],
    )?;

    let gh_stub_dir = tmp.path().join("gh-nopr-locked");
    fs::create_dir_all(&gh_stub_dir)?;
    let gh_stub = write_gh_stub(&gh_stub_dir, 0, "")?;

    let (ok, output) = run_xtask_cleanup(&repo, true, Some(&gh_stub))?;
    assert!(ok, "--force run must exit 0: {output}");
    assert!(
        locked_wt.exists(),
        "SAFETY VIOLATION: locked worktree was removed under --force: {output}"
    );
    Ok(())
}

// ── gh-unavailable safety default ───────────────────────────────────────

#[test]
fn gh_failure_yields_unknown_pr_status_and_keeps_the_worktree() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let repo = init_fixture_repo(tmp.path())?;
    let wt = add_agent_worktree(&repo, "wt-gh-unknown")?;

    let gh_stub_dir = tmp.path().join("gh-fail");
    fs::create_dir_all(&gh_stub_dir)?;
    let gh_stub = write_gh_stub(&gh_stub_dir, 1, "")?;

    let (ok, output) = run_xtask_cleanup(&repo, true, Some(&gh_stub))?;
    assert!(ok, "--force run must exit 0: {output}");
    assert!(
        wt.exists(),
        "SAFETY VIOLATION: worktree was removed under --force when gh PR status was unknown: {output}"
    );
    assert!(
        output.contains("could not be determined") || output.contains("unknown"),
        "expected the unknown-PR-status reason to be reported: {output}"
    );
    Ok(())
}

// ── Root checkout guard ──────────────────────────────────────────────────

#[test]
fn no_agent_worktrees_reports_nothing_to_clean() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let repo = init_fixture_repo(tmp.path())?;

    let (ok, output) = run_xtask_cleanup(&repo, false, None)?;
    assert!(ok, "dry-run on a repo with no agent worktrees must exit 0: {output}");
    assert!(
        output.contains("No stale worktrees to clean up"),
        "expected an explicit nothing-to-clean message: {output}"
    );
    Ok(())
}
