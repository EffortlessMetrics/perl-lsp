//! Change-set resolver — a pure library seam for issue #3985.
//!
//! This module is the SINGLE base-resolver + SINGLE `git diff` that later
//! slices of #3985 will repoint `ci_scope`/`targeted_checks`/the pre-push
//! hook at. **This slice has zero consumers** — `resolve_change_set` is not
//! called from any production path (no CLI subcommand, no wiring into
//! `gates.rs` or `ci_scope.rs`). It exists here, fully tested in isolation,
//! so a later slice can repoint existing call sites with a falsifiable
//! parity test instead of introducing a new classifier and a new resolver
//! in the same change.
//!
//! # Why not `origin/master`
//!
//! `origin/master` does not exist on this remote (verified via `git
//! ls-remote --heads origin master`; `refs/remotes/origin/HEAD` points at
//! `origin/main`). [`resolve_base_ref`] therefore never includes
//! `origin/master` in its candidate chain — unlike `ci_scope::resolve_base_ref`
//! (`xtask/src/tasks/ci_scope.rs`), which still carries it as a historical
//! fallback. Per #3985: prefer the canonical `origin/main` and fail loudly
//! when no canonical base resolves, rather than silently resolving a stray
//! `origin/master`-shaped ref if one is ever recreated on the remote.
//!
//! # Composition, not duplication
//!
//! - The base-ref candidate chain and three-dot `git diff` mirror
//!   `ci_scope::resolve_base_ref` / `ci_scope::get_changed_files`
//!   (`xtask/src/tasks/ci_scope.rs:812-890`), reordered to drop
//!   `origin/master` from the candidates.
//! - The `StagedTree` arm composes `staged::staged_diff_paths` and
//!   `staged::diff_base` (`xtask/src/tasks/staged.rs`) rather than
//!   reimplementing staged-tree diffing.
//! - `ci_scope::classify_files` / `ScopeOutput` are untouched — this module
//!   is only the input seam (identity → changed paths), not a classifier.
//!
//! # `dead_code` for this slice only
//!
//! Every public item here is unreachable from any production call path by
//! design (see the module-level intent above). The crate-wide `dead_code`
//! lint would otherwise flag the whole module on a non-test build. Slice 2
//! of #3985 repoints `ci_scope`/`targeted_checks`/`gates` at this module,
//! at which point this allow should be removed — leaving it in place would
//! silently mask a genuinely-unused item again.
#![allow(dead_code)]

use std::path::Path;

use color_eyre::eyre::{Context, Result, eyre};
use duct::cmd;

use crate::tasks::staged;

/// What is being diffed: an explicit commit range, or the staged tree
/// (frozen by `git write-tree`) against its diff base.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactIdentity {
    /// A base/head commit-ish pair. `base == "auto"` triggers the
    /// main-first candidate chain in [`resolve_base_ref`]; any other value
    /// is tried first, then the chain is used as a fallback (mirroring
    /// `ci_scope::resolve_base_ref`'s behavior).
    CommitRange { base: String, head: String },
    /// A staged-tree OID as produced by `git write-tree` /
    /// `staged::staged_tree_oid` — the commit-tier identity (#3786).
    StagedTree { oid: String },
}

/// The result of resolving an [`ArtifactIdentity`] against a repository:
/// the concrete identity actually used, the resolved base/head SHAs where
/// applicable, and the changed repo-relative paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeSet {
    /// The identity that was resolved, with `CommitRange::base` replaced by
    /// the concrete ref that was actually selected (never `"auto"`).
    pub identity: ArtifactIdentity,
    /// The resolved base commit SHA. `Some` for `CommitRange` (and for
    /// `StagedTree` when the diff base is `HEAD`); `None` when the staged
    /// tree's diff base is a derived empty-tree OID with no commit behind
    /// it (unborn `HEAD`) and SHA resolution is therefore not meaningful.
    pub base_sha: Option<String>,
    /// The resolved head commit SHA. `Some` for `CommitRange`; always
    /// `None` for `StagedTree` — a staged tree has no commit SHA of its
    /// own until `git commit` runs.
    pub head_sha: Option<String>,
    /// Repo-relative, forward-slash-separated changed paths.
    pub changed_paths: Vec<String>,
}

/// Resolve an [`ArtifactIdentity`] into a [`ChangeSet`]: the single base
/// resolver (for `CommitRange`) plus the single `git diff` (both arms).
///
/// This function has no consumers as of this slice (#3985 Slice 1) — it is
/// dead-but-compiled library code, proven correct by the unit tests below.
pub fn resolve_change_set(identity: ArtifactIdentity, root: &Path) -> Result<ChangeSet> {
    match identity {
        ArtifactIdentity::CommitRange { base, head } => {
            let resolved_base = resolve_base_ref(&base, root)?;
            let base_sha = resolve_sha(&resolved_base, root)?;
            let head_sha = resolve_sha(&head, root)?;
            let changed_paths = diff_paths(&resolved_base, &head, root)?;
            Ok(ChangeSet {
                identity: ArtifactIdentity::CommitRange { base: resolved_base, head },
                base_sha: Some(base_sha),
                head_sha: Some(head_sha),
                changed_paths,
            })
        }
        ArtifactIdentity::StagedTree { oid } => {
            let changed_paths = staged::staged_diff_paths(root, Some(&oid))?;
            let base = staged::diff_base(root)?;
            // Best-effort: an unborn HEAD's diff base is a derived
            // empty-tree OID, which resolves to itself under
            // `git rev-parse` — not a genuine failure, just not a commit.
            // Any other resolution failure is folded into `None` rather
            // than surfaced, since `base_sha` is informational for the
            // staged-tree arm (the load-bearing identity is `oid`).
            let base_sha = resolve_sha(&base, root).ok();
            Ok(ChangeSet {
                identity: ArtifactIdentity::StagedTree { oid },
                base_sha,
                head_sha: None,
                changed_paths,
            })
        }
    }
}

/// Main-first base-ref candidate chain, deliberately **excluding**
/// `origin/master` (issue #3985: that ref does not exist on this remote,
/// and must never be a silent fallback if one is ever recreated).
///
/// Order: `origin/main` (canonical), `main` (local mirror, e.g. a clone
/// without a configured remote), `HEAD~1` (shallow-clone / single-remote
/// -less-history fallback).
const BASE_CANDIDATES: [&str; 3] = ["origin/main", "main", "HEAD~1"];

/// Resolve a base ref: an explicit non-`"auto"` `base` is tried first (and
/// used if it exists), then [`BASE_CANDIDATES`] in order. Returns a loud
/// [`Err`] — never a silent `origin/master` — when nothing resolves.
fn resolve_base_ref(base: &str, root: &Path) -> Result<String> {
    let mut candidates = Vec::new();
    if base != "auto" {
        candidates.push(base.to_string());
    }
    candidates.extend(BASE_CANDIDATES.iter().map(|s| (*s).to_string()));

    for candidate in candidates {
        if git_ref_exists(&candidate, root)? {
            return Ok(candidate);
        }
    }

    Err(eyre!(
        "Could not resolve a canonical base ref from '{base}', origin/main, main, or HEAD~1. \
         Refusing to fall back to origin/master (issue #3985: that ref does not exist on this \
         remote, and must never be a silent fallback). Ensure the repository has origin/main \
         reachable, a local main branch, or at least two commits of history."
    ))
}

fn git_ref_exists(candidate: &str, root: &Path) -> Result<bool> {
    let verify = cmd("git", &["rev-parse", "--verify", "--quiet", candidate])
        .dir(root)
        .stdout_null()
        .stderr_null()
        .unchecked()
        .run()
        .context("Failed to run git rev-parse")?;
    Ok(verify.status.success())
}

/// Resolve any ref/OID (branch, remote-tracking branch, tag, `HEAD~N`, or
/// an already-resolved OID) to its full SHA.
fn resolve_sha(reference: &str, root: &Path) -> Result<String> {
    let output = cmd("git", &["rev-parse", reference])
        .dir(root)
        .stdout_capture()
        .stderr_capture()
        .unchecked()
        .run()
        .context("Failed to run git rev-parse")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(eyre!("`git rev-parse {reference}` failed: {stderr}"));
    }
    Ok(String::from_utf8(output.stdout)
        .context("git rev-parse output was not valid UTF-8")?
        .trim()
        .to_string())
}

/// Changed paths between `base` and `head`: three-dot (`base...head`,
/// merge-base relative) first, falling back to two-dot (`base..head`,
/// direct) when the three-dot form fails — mirrors
/// `ci_scope::get_changed_files`.
fn diff_paths(base: &str, head: &str, root: &Path) -> Result<Vec<String>> {
    let three_dot = format!("{base}...{head}");
    let output = cmd("git", &["diff", "--name-only", &three_dot])
        .dir(root)
        .stdout_capture()
        .stderr_capture()
        .unchecked()
        .run()
        .context("Failed to run git diff")?;

    if output.status.success() {
        let stdout =
            String::from_utf8(output.stdout).context("git diff output was not valid UTF-8")?;
        return Ok(stdout.lines().map(str::to_string).collect());
    }

    let two_dot = format!("{base}..{head}");
    let output2 = cmd("git", &["diff", "--name-only", &two_dot])
        .dir(root)
        .stdout_capture()
        .stderr_capture()
        .run()
        .context("Failed to run git diff (two-dot fallback)")?;
    let stdout =
        String::from_utf8(output2.stdout).context("git diff output was not valid UTF-8")?;
    Ok(stdout.lines().map(str::to_string).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use color_eyre::eyre::ensure;
    use std::fs;
    use std::process::Command;

    fn run_git(dir: &Path, args: &[&str]) -> Result<()> {
        let status = Command::new("git")
            .current_dir(dir)
            .args(args)
            .status()
            .context("failed to spawn git")?;
        if !status.success() {
            return Err(eyre!("git {args:?} failed in {}", dir.display()));
        }
        Ok(())
    }

    fn init_repo(dir: &Path, branch: &str) -> Result<()> {
        fs::create_dir_all(dir).context("failed to create repo dir")?;
        if run_git(dir, &["init", "-q", "-b", branch]).is_err() {
            // Older git without `-b` on `init`.
            run_git(dir, &["init", "-q"])?;
            run_git(dir, &["symbolic-ref", "HEAD", &format!("refs/heads/{branch}")])?;
        }
        run_git(dir, &["config", "user.email", "test@test.local"])?;
        run_git(dir, &["config", "user.name", "Test"])?;
        run_git(dir, &["config", "commit.gpgsign", "false"])?;
        Ok(())
    }

    fn commit_file(dir: &Path, name: &str, contents: &str, message: &str) -> Result<()> {
        fs::write(dir.join(name), contents).context("failed to write fixture file")?;
        run_git(dir, &["add", name])?;
        run_git(dir, &["commit", "-q", "-m", message])?;
        Ok(())
    }

    /// A repo with a real `origin` bare remote and `main` pushed —
    /// the "current-repo case": `origin/main` exists, `origin/master`
    /// never did.
    fn init_repo_with_origin_main(tmp: &Path) -> Result<std::path::PathBuf> {
        let repo = tmp.join("repo");
        init_repo(&repo, "main")?;
        commit_file(&repo, "README.md", "init\n", "init")?;

        let remote = tmp.join("origin.git");
        run_git(tmp, &["init", "-q", "--bare", &remote.to_string_lossy()])?;
        run_git(&repo, &["remote", "add", "origin", &remote.to_string_lossy()])?;
        run_git(&repo, &["push", "-q", "origin", "main"])?;
        Ok(repo)
    }

    #[test]
    fn test_resolve_base_ref_auto_resolves_origin_main_when_origin_master_absent() -> Result<()> {
        let tmp = tempfile::tempdir().context("failed to create tempdir")?;
        let repo = init_repo_with_origin_main(tmp.path())?;

        // origin/master genuinely does not exist in this fixture.
        ensure!(
            !git_ref_exists("origin/master", &repo)?,
            "origin/master must not exist in this fixture"
        );

        let resolved = resolve_base_ref("auto", &repo)?;
        ensure!(resolved == "origin/main", "expected origin/main, got {resolved}");
        Ok(())
    }

    #[test]
    fn test_resolve_base_ref_explicit_base_honored() -> Result<()> {
        let tmp = tempfile::tempdir().context("failed to create tempdir")?;
        let repo = init_repo_with_origin_main(tmp.path())?;
        commit_file(&repo, "second.txt", "two\n", "second commit")?;
        run_git(&repo, &["branch", "feature-base"])?;

        let resolved = resolve_base_ref("feature-base", &repo)?;
        ensure!(resolved == "feature-base", "expected feature-base, got {resolved}");
        Ok(())
    }

    #[test]
    fn test_resolve_base_ref_shallow_clone_falls_back_to_head_tilde_1() -> Result<()> {
        let tmp = tempfile::tempdir().context("failed to create tempdir")?;
        let repo = tmp.path().join("repo");
        // No remote at all, and the branch is deliberately not named
        // "main" or "master" so neither `origin/main` nor `main` resolve —
        // only the HEAD~1 fallback can succeed here.
        init_repo(&repo, "work")?;
        commit_file(&repo, "one.txt", "one\n", "first")?;
        commit_file(&repo, "two.txt", "two\n", "second")?;

        ensure!(!git_ref_exists("origin/main", &repo)?, "origin/main must not exist here");
        ensure!(!git_ref_exists("main", &repo)?, "main must not exist here");

        let resolved = resolve_base_ref("auto", &repo)?;
        ensure!(resolved == "HEAD~1", "expected HEAD~1, got {resolved}");
        Ok(())
    }

    #[test]
    fn test_resolve_base_ref_loud_error_when_nothing_resolves() -> Result<()> {
        let tmp = tempfile::tempdir().context("failed to create tempdir")?;
        let repo = tmp.path().join("repo");
        // No remote, branch not named main/master, and only ONE commit —
        // so HEAD~1 doesn't exist either. Nothing in the candidate chain
        // can resolve.
        init_repo(&repo, "work")?;
        commit_file(&repo, "one.txt", "one\n", "only commit")?;

        let err = resolve_base_ref("auto", &repo)
            .err()
            .ok_or_else(|| eyre!("expected resolve_base_ref to fail loudly"))?;
        let message = format!("{err}");
        ensure!(
            !message.to_lowercase().contains("origin/master")
                || message.contains("Refusing to fall back to origin/master"),
            "error must not silently name origin/master as a resolution, got: {message}"
        );
        ensure!(
            message.contains("Could not resolve a canonical base ref"),
            "expected a loud, descriptive error, got: {message}"
        );
        Ok(())
    }

    #[test]
    fn test_resolve_change_set_commit_range_extracts_changed_paths() -> Result<()> {
        let tmp = tempfile::tempdir().context("failed to create tempdir")?;
        let repo = init_repo_with_origin_main(tmp.path())?;

        run_git(&repo, &["checkout", "-q", "-b", "feature"])?;
        commit_file(&repo, "feature.rs", "fn feature() {}\n", "add feature")?;
        commit_file(&repo, "another.rs", "fn another() {}\n", "add another")?;

        let identity =
            ArtifactIdentity::CommitRange { base: "auto".to_string(), head: "HEAD".to_string() };
        let change_set = resolve_change_set(identity, &repo)?;

        match &change_set.identity {
            ArtifactIdentity::CommitRange { base, .. } => {
                ensure!(base == "origin/main", "expected origin/main, got {base}");
            }
            other => return Err(eyre!("expected CommitRange identity, got {other:?}")),
        }
        ensure!(change_set.base_sha.is_some(), "base_sha should be resolved for CommitRange");
        ensure!(change_set.head_sha.is_some(), "head_sha should be resolved for CommitRange");
        let mut paths = change_set.changed_paths.clone();
        paths.sort();
        let expected = vec!["another.rs".to_string(), "feature.rs".to_string()];
        ensure!(paths == expected, "expected {expected:?}, got {paths:?}");
        Ok(())
    }

    #[test]
    fn test_resolve_change_set_commit_range_explicit_base() -> Result<()> {
        let tmp = tempfile::tempdir().context("failed to create tempdir")?;
        let repo = init_repo_with_origin_main(tmp.path())?;
        run_git(&repo, &["branch", "known-base"])?;
        commit_file(&repo, "only_after_base.txt", "x\n", "after base")?;

        let identity = ArtifactIdentity::CommitRange {
            base: "known-base".to_string(),
            head: "HEAD".to_string(),
        };
        let change_set = resolve_change_set(identity, &repo)?;
        let expected = vec!["only_after_base.txt".to_string()];
        ensure!(
            change_set.changed_paths == expected,
            "expected {expected:?}, got {:?}",
            change_set.changed_paths
        );
        Ok(())
    }

    #[test]
    fn test_resolve_change_set_staged_tree_composes_staged_diff_paths() -> Result<()> {
        let tmp = tempfile::tempdir().context("failed to create tempdir")?;
        let repo = init_repo_with_origin_main(tmp.path())?;

        fs::write(repo.join("staged_only.txt"), "staged content\n")
            .context("failed to write staged fixture file")?;
        run_git(&repo, &["add", "staged_only.txt"])?;
        let tree_oid = staged::staged_tree_oid(&repo)?;

        let identity = ArtifactIdentity::StagedTree { oid: tree_oid.clone() };
        let change_set = resolve_change_set(identity, &repo)?;

        let expected_identity = ArtifactIdentity::StagedTree { oid: tree_oid };
        ensure!(
            change_set.identity == expected_identity,
            "expected identity {expected_identity:?}, got {:?}",
            change_set.identity
        );
        ensure!(
            change_set.head_sha.is_none(),
            "head_sha should be None for StagedTree, got {:?}",
            change_set.head_sha
        );
        let expected_paths = vec!["staged_only.txt".to_string()];
        ensure!(
            change_set.changed_paths == expected_paths,
            "expected {expected_paths:?}, got {:?}",
            change_set.changed_paths
        );
        Ok(())
    }

    #[test]
    fn test_resolve_change_set_staged_tree_empty_when_nothing_staged() -> Result<()> {
        let tmp = tempfile::tempdir().context("failed to create tempdir")?;
        let repo = init_repo_with_origin_main(tmp.path())?;

        let tree_oid = staged::staged_tree_oid(&repo)?;
        let identity = ArtifactIdentity::StagedTree { oid: tree_oid };
        let change_set = resolve_change_set(identity, &repo)?;

        ensure!(
            change_set.changed_paths.is_empty(),
            "expected no changed paths, got {:?}",
            change_set.changed_paths
        );
        Ok(())
    }

    /// Three-dot diff (`base...head`) requires a merge base; on genuinely
    /// unrelated histories (an orphan branch sharing no ancestor with
    /// `main`) git fails it outright (`fatal: ... no merge base`), which is
    /// exactly the failure `diff_paths` is documented to fall back from.
    /// Without this test the two-dot fallback branch — the one place this
    /// module deliberately mirrors `ci_scope::get_changed_files`'s fallback
    /// — had zero coverage; a change that silently dropped the fallback
    /// (or swapped which paths it reports) would still pass every other
    /// test in this file.
    #[test]
    fn test_diff_paths_falls_back_to_two_dot_on_unrelated_histories() -> Result<()> {
        let tmp = tempfile::tempdir().context("failed to create tempdir")?;
        let repo = init_repo_with_origin_main(tmp.path())?;

        // An orphan branch shares no common ancestor with `main` at all, so
        // `git diff main...unrelated` fails with "no merge base" — unlike
        // ordinary divergent branches, which always share the initial
        // commit and would make three-dot succeed trivially.
        run_git(&repo, &["checkout", "-q", "--orphan", "unrelated"])?;
        // `--orphan` carries the current index/working tree forward with no
        // parent commit; committing here keeps README.md (identical
        // content to main's) plus a new file, so the only genuine diff
        // between the two trees is the new file.
        commit_file(&repo, "orphan.txt", "orphan content\n", "orphan init")?;

        let paths = diff_paths("main", "unrelated", &repo)?;
        let expected = vec!["orphan.txt".to_string()];
        ensure!(paths == expected, "expected {expected:?}, got {paths:?}");
        Ok(())
    }
}
