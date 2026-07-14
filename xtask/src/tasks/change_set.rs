//! Change-set resolver — a pure library seam for issue #3985.
//!
//! This module is the SINGLE base-resolver + SINGLE `git diff` shared by
//! `ci_scope::run`, `gates::compute_scope_output`/`select_scope_base`, and
//! `targeted_checks::run` (#3985 Slice 2 repointed all three onto
//! [`resolve_change_set`] — see the falsifiable parity corpus in this
//! module's test suite, `tests::test_parity_*` (doc-only, single-crate,
//! multi-crate, mixed, ci-config, a deletion, a rename), which pins that
//! the repoint is zero-behavior-change on real diff fixtures).
//!
//! #3985 Slice 3A exposes this resolver to `hooks/pre-push` through the
//! `cargo xtask change-set` CLI ([`run`] below, registered in
//! `xtask/src/main.rs`'s `Commands::ChangeSet`) so the hook consumes the
//! shared resolver for its new-branch base resolution instead of carrying
//! its own `git merge-base "$local_sha" origin/master || echo "$local_sha"`
//! shell algorithm — which silently produced an empty self-diff for every
//! new-branch push, since `origin/master` never resolves on this remote
//! and the fallback compared `$local_sha` against itself. See
//! `xtask/tests/change_set_cli.rs` for the regression proof (old shell
//! algorithm vs the new resolver, against a real bare-remote fixture).
//!
//! # Why not `origin/master`
//!
//! `origin/master` does not exist on this remote (verified via `git
//! ls-remote --heads origin master`; `refs/remotes/origin/HEAD` points at
//! `origin/main`). [`resolve_base_ref`] therefore never includes
//! `origin/master` in its candidate chain. **Historical note:** prior to
//! #3985 Slice 2, `ci_scope::resolve_base_ref` and
//! `targeted_checks::resolve_base_ref` — private per-consumer copies of
//! this same resolver, both deleted by this PR now that `ci_scope::run`
//! and `targeted_checks::run` call [`resolve_change_set`] instead — still
//! carried `origin/master` as a fallback candidate; this module never did.
//! Per #3985: prefer the canonical `origin/main` and fail loudly when no
//! canonical base resolves, rather than silently resolving a stray
//! `origin/master`-shaped ref if one is ever recreated on the remote.
//!
//! # Composition, not duplication
//!
//! - The base-ref candidate chain and three-dot `git diff` mirror what
//!   `ci_scope::resolve_base_ref` / `ci_scope::get_changed_files` and
//!   `targeted_checks::resolve_base_ref` / `targeted_checks::changed_files`
//!   used to do before #3985 Slice 2 deleted all four (now dead code —
//!   `ci_scope::run`/`targeted_checks::run` call [`resolve_change_set`]
//!   directly); this module's shape was reordered from theirs to drop
//!   `origin/master` from the candidates.
//! - The `StagedTree` arm composes `staged::staged_diff_paths` and
//!   `staged::diff_base` (`xtask/src/tasks/staged.rs`) rather than
//!   reimplementing staged-tree diffing.
//! - `ci_scope::classify_files` / `ScopeOutput` are untouched — this module
//!   is only the input seam (identity → changed paths), not a classifier.

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
    /// is treated as an **explicit** base and must resolve on its own —
    /// see [`resolve_base_ref`]'s doc comment for why an unresolvable
    /// explicit base is a loud [`Err`], never a silent substitution.
    CommitRange { base: String, head: String },
    /// A staged-tree OID as produced by `git write-tree` /
    /// `staged::staged_tree_oid` — the commit-tier identity (#3786).
    ///
    /// `#[allow(dead_code)]`: no production call site constructs this
    /// variant yet — #3985 Slice 2 repoints only the `CommitRange`
    /// (base/HEAD diff) consumers (`ci_scope`, `gates`, `targeted_checks`).
    /// Wiring `StagedTree` into the commit-tier gate path (#3786) is a
    /// later slice; this variant is exercised today only by this module's
    /// own unit tests (`test_resolve_change_set_staged_tree_*`).
    #[allow(dead_code)]
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
/// Consumers: `ci_scope::run`, `gates::compute_scope_output`, and
/// `targeted_checks::run` (#3985 Slice 2), plus the `cargo xtask
/// change-set` CLI ([`run`] below) that `hooks/pre-push` consumes (#3985
/// Slice 3A). Proven correct by the unit tests below, including the
/// falsifiable parity corpus (`tests::test_parity_*`).
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

/// Resolve a base ref.
///
/// `base == "auto"` walks [`BASE_CANDIDATES`] in order and returns the
/// first that exists.
///
/// An **explicit** (non-`"auto"`) `base` must resolve **on its own** —
/// it is never silently substituted with a [`BASE_CANDIDATES`] entry.
/// This is a deliberate safety contract, not an oversight: callers that
/// pass an explicit base (a `--base` CLI flag, `$CI_SCOPE_BASE`, an
/// unvalidated `config.base_ref` threaded through
/// `gates::plan_gates`/`plan_pr_fast_gates`) rely on a loud [`Err`] here
/// to trigger their own safety net — e.g. `plan_pr_fast_gates` catches
/// this error and falls back to the broad `rust_fallback` gate plan
/// rather than silently running PR-fast against a narrower, unintended
/// scope. #3985 Slice 2 review (PR #4153) caught a version of this
/// function that fell through to [`BASE_CANDIDATES`] on an unresolvable
/// explicit base — reproducible as
/// `resolve_base_ref("origin/definitely-not-fetched", repo)` silently
/// returning `Ok("origin/main")` — which defeated that safety net for
/// any caller supplying an unfetched/typo'd explicit base. Never
/// silently falls back to `origin/master` either way (issue #3985: that
/// ref does not exist on this remote).
fn resolve_base_ref(base: &str, root: &Path) -> Result<String> {
    if base != "auto" {
        if git_ref_exists(base, root)? {
            return Ok(base.to_string());
        }
        eprintln!(
            "Warning: explicit base ref '{base}' does not exist; refusing to substitute a \
             different base (a caller-supplied base must resolve on its own — see #3985 \
             Slice 2 review, PR #4153)."
        );
        return Err(eyre!(
            "Explicit base ref '{base}' does not exist. Refusing to silently fall back to \
             {BASE_CANDIDATES:?} for an explicitly-requested base — an explicit base must \
             resolve on its own, so callers with their own safety net (e.g. \
             `plan_pr_fast_gates`'s `rust_fallback` broad-plan fallback) see this error \
             rather than a silently-narrowed scope."
        ));
    }

    for candidate in BASE_CANDIDATES {
        if git_ref_exists(candidate, root)? {
            return Ok(candidate.to_string());
        }
    }

    Err(eyre!(
        "Could not resolve a canonical base ref from auto-resolution: origin/main, main, or \
         HEAD~1. Refusing to fall back to origin/master (issue #3985: that ref does not exist \
         on this remote, and must never be a silent fallback). Ensure the repository has \
         origin/main reachable, a local main branch, or at least two commits of history."
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
/// direct) when the three-dot form fails — the same shape the deleted
/// `ci_scope::get_changed_files` / `targeted_checks::changed_files` used
/// before #3985 Slice 2 repointed both onto this function.
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

// ---------------------------------------------------------------------------
// CLI: `cargo xtask change-set` — the runtime-neutral interface #3985
// Slice 3A exposes to `hooks/pre-push` (and any other shell/non-Rust
// consumer) so they never need their own base-resolution algorithm.
// ---------------------------------------------------------------------------

/// Configuration for the `change-set` subcommand.
pub struct ChangeSetConfig {
    /// Base git ref to diff against. `"auto"` triggers main-first candidate
    /// resolution (see [`resolve_base_ref`]); any other value is an
    /// explicit base that must resolve on its own.
    pub base: String,
    /// Head git ref/SHA to diff to.
    pub head: String,
    /// Output format: `"json"` (the bounded `{base_sha, head_sha,
    /// changed_paths}` contract) or `"paths"` (one changed path per line,
    /// nothing else — no `jq` dependency for shell consumers).
    pub format: String,
    /// Repository root to resolve against. Defaults to the perl-lsp
    /// workspace root (`crate::utils::project_root`) when `None`. Overridable
    /// so integration tests can point this at a fixture repository —
    /// `crate::utils::project_root` resolves from `CARGO_MANIFEST_DIR`
    /// baked in at compile time, not from the process's current directory,
    /// so a fixture repo is otherwise unreachable from the compiled test
    /// binary.
    pub root: Option<std::path::PathBuf>,
}

/// Entry point called from `xtask` main for `cargo xtask change-set`.
///
/// Resolves a [`ChangeSet`] via [`resolve_change_set`] and prints it in the
/// requested format. Returns `Err` (non-zero exit, message on stderr via
/// `color_eyre`) when resolution fails — callers (notably `hooks/pre-push`)
/// must treat a non-zero exit as "could not prove the change set" and stop,
/// never fall back to an empty changed-paths set as if the proof passed
/// (issue #3985 Slice 3A). An unrecognized `--format` value is the same
/// class of failure: a loud `Err`, never a silent fallback to `"json"` —
/// a shell consumer expecting `paths` that typos the flag must see a clear
/// error, not misparse a JSON blob as a newline-separated path list.
pub fn run(config: ChangeSetConfig) -> Result<()> {
    let root = match config.root {
        Some(root) => root,
        None => crate::utils::project_root()?,
    };
    let identity = ArtifactIdentity::CommitRange { base: config.base, head: config.head };
    let resolved = resolve_change_set(identity, &root)?;

    match config.format.as_str() {
        "paths" => {
            for path in &resolved.changed_paths {
                println!("{path}");
            }
        }
        "json" => {
            let json = serde_json::json!({
                "base_sha": resolved.base_sha,
                "head_sha": resolved.head_sha,
                "changed_paths": resolved.changed_paths,
            });
            let pretty = serde_json::to_string_pretty(&json)
                .context("Failed to serialize change set to JSON")?;
            println!("{pretty}");
        }
        other => {
            return Err(eyre!(
                "Unknown --format '{other}'; expected 'json' or 'paths'. Refusing to silently \
                 fall back to JSON for an unrecognized format value."
            ));
        }
    }

    Ok(())
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
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).context("failed to create fixture file parent dir")?;
        }
        fs::write(&path, contents).context("failed to write fixture file")?;
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
    /// module deliberately mirrors the shape the deleted
    /// `ci_scope::get_changed_files` used to fall back on
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

    // -----------------------------------------------------------------
    // #3985 Slice 2 — falsifiable parity: repointed resolver vs the
    // pre-repoint per-consumer logic it replaces
    // -----------------------------------------------------------------
    //
    // Before Slice 2, `ci_scope::run`, `gates::compute_scope_output` /
    // `select_scope_base`, and `targeted_checks::run` each carried a
    // private base-candidate chain + `git diff --name-only` call instead
    // of calling `resolve_change_set`. This corpus proves the shared
    // resolver produces the IDENTICAL resolved base ref, base SHA, and
    // changed-path set as those frozen pre-repoint chains, across real
    // diff shapes (doc-only, single-crate, multi-crate, mixed, ci-config,
    // a deletion, a rename) — falsifying "the repoint is zero-behavior-
    // change" rather than assuming it.
    //
    // Mutation-checked (verified red, then reverted, before landing):
    // - Reordering `BASE_CANDIDATES` to try `"main"` before `"origin/main"`
    //   makes `test_parity_single_crate_change` fail: `init_repo_with_origin_main`
    //   creates both a local `main` branch and an `origin/main`
    //   remote-tracking ref pointing at the same commit, so the pre-repoint
    //   chains (which try `origin/main` first) resolve to the ref string
    //   `"origin/main"`, while the reordered resolver would resolve to
    //   `"main"` — same commit, different ref string, caught by the
    //   `resolved_base == ci_scope_style_base` assertion below.
    // - Adding `--diff-filter=ACMR` to `diff_paths`'s `git diff` invocation
    //   makes `test_parity_deletion_is_visible` fail: that fixture's diff
    //   contains only a deleted file, so `ACMR` (which excludes `D`) reports
    //   zero changed paths against the pre-repoint chain's non-empty result
    //   — this is the exact RIPR `ACMR`-vs-`ACDMRT` regression class
    //   #3985's diff-filter audit comment flags; this slice deliberately
    //   does not touch `ripr_evidence.rs`, and this test is the guard that
    //   `diff_paths` itself never silently grows a filter.

    /// Frozen copy of `ci_scope::resolve_base_ref` /
    /// `targeted_checks::resolve_base_ref`'s pre-repoint candidate chain —
    /// the two were byte-identical (main-first, but still carrying
    /// `origin/master`/`master`). Independent of [`BASE_CANDIDATES`] above
    /// by construction: this does not call `resolve_base_ref` or any other
    /// production resolver, it re-derives the chain from the pre-repoint
    /// source (see #3985's architecture-review comment for the citation:
    /// `ci_scope.rs:812-841` / `targeted_checks.rs:28-63` before this PR).
    fn pre_repoint_ci_scope_style_base(base: &str, root: &Path) -> Result<String> {
        let mut candidates = Vec::new();
        if base != "auto" {
            candidates.push(base.to_string());
        }
        candidates.extend(
            ["origin/main", "origin/master", "main", "master", "HEAD~1"]
                .into_iter()
                .map(str::to_string),
        );
        for candidate in candidates {
            if git_ref_exists(&candidate, root)? {
                return Ok(candidate);
            }
        }
        Err(eyre!("pre-repoint ci_scope-style resolution found no candidate"))
    }

    /// Frozen copy of `gates::select_scope_base`'s pre-repoint static
    /// fallback chain (env-var candidates — `CI_SCOPE_BASE`,
    /// `GITHUB_BASE_REF` — omitted: they are unset in this test process,
    /// and remain gates-local post-repoint too; see
    /// `gates::select_scope_base`'s doc comment). This chain was
    /// **master-first** (`fn select_scope_base` at
    /// `git show 465ceceab~1:xtask/src/tasks/gates.rs:1780`, candidate
    /// array at line 1788 of that same pre-repoint blob — verified by
    /// SHA-anchored `git show`, not a plain line number against the
    /// current tree, since those drift; the architecture-review comment's
    /// original `gates.rs:1499` citation had already drifted onto an
    /// unrelated `Receipt` struct field by the time this PR was built) —
    /// the exact ordering #3985's architecture-review
    /// comment flagged as a latent conflict with `ci_scope`'s main-first
    /// chain, resolved here by proving both chains agree on the live
    /// repository (`origin/master`
    /// absent) before asserting parity against the shared resolver.
    fn pre_repoint_gates_style_base(root: &Path) -> Result<String> {
        let candidates =
            ["origin/master", "origin/main", "origin/HEAD", "master", "main", "HEAD~1"];
        for candidate in candidates {
            if git_ref_exists(candidate, root)? {
                return Ok(candidate.to_string());
            }
        }
        Err(eyre!("pre-repoint gates-style resolution found no candidate"))
    }

    /// Frozen, **independent** copy of the pre-repoint three-dot/two-dot
    /// diff shape shared by `ci_scope::get_changed_files` and
    /// `targeted_checks::changed_files` (byte-identical to each other, and
    /// to `diff_paths` above — the diff-filter audit on #3985 confirmed
    /// neither carried a `--diff-filter`). Deliberately does **not** call
    /// [`diff_paths`]: if it did, mutating `diff_paths`'s `git diff`
    /// invocation (e.g. adding `--diff-filter=ACMR`) would silently
    /// mutate both sides of the parity assertion below and the corpus
    /// would never catch it. This function is what makes
    /// `test_parity_deletion_is_visible` a real regression guard against
    /// that class of change rather than a tautology.
    fn pre_repoint_diff(base: &str, head: &str, root: &Path) -> Result<Vec<String>> {
        let three_dot = format!("{base}...{head}");
        let output = cmd("git", &["diff", "--name-only", &three_dot])
            .dir(root)
            .stdout_capture()
            .stderr_capture()
            .unchecked()
            .run()
            .context("Failed to run git diff (pre-repoint style)")?;

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
            .context("Failed to run git diff (pre-repoint style, two-dot fallback)")?;
        let stdout =
            String::from_utf8(output2.stdout).context("git diff output was not valid UTF-8")?;
        Ok(stdout.lines().map(str::to_string).collect())
    }

    /// Assert that, for the current state of `repo`, the shared
    /// `resolve_change_set("auto", "HEAD")` result is IDENTICAL — same
    /// resolved base ref, same base SHA, same changed-path set — to both
    /// pre-repoint candidate chains' output (base resolution) and the
    /// independent [`pre_repoint_diff`] reimplementation (changed paths).
    /// `ci_scope::classify_files` output is a pure function of
    /// `(changed_files, metadata, workspace_root)`, so identical
    /// `changed_paths` here implies identical `ScopeOutput` without
    /// needing a separate classify_files fixture per scenario.
    fn assert_parity_with_pre_repoint_logic(scenario: &str, repo: &Path) -> Result<()> {
        let ci_scope_style_base = pre_repoint_ci_scope_style_base("auto", repo)?;
        let gates_style_base = pre_repoint_gates_style_base(repo)?;
        ensure!(
            ci_scope_style_base == gates_style_base,
            "[{scenario}] pre-repoint chains disagree on base ref \
             (ci_scope-style: {ci_scope_style_base}, gates-style: {gates_style_base}) — \
             this fixture cannot prove parity against a single shared resolver"
        );

        let mut pre_repoint_paths = pre_repoint_diff(&ci_scope_style_base, "HEAD", repo)?;
        pre_repoint_paths.sort();
        let expected_base_sha = resolve_sha(&ci_scope_style_base, repo)?;

        let identity =
            ArtifactIdentity::CommitRange { base: "auto".to_string(), head: "HEAD".to_string() };
        let resolved = resolve_change_set(identity, repo)?;
        let resolved_base = match &resolved.identity {
            ArtifactIdentity::CommitRange { base, .. } => base.clone(),
            other => {
                return Err(eyre!("[{scenario}] expected CommitRange identity, got {other:?}"));
            }
        };
        let mut new_paths = resolved.changed_paths.clone();
        new_paths.sort();

        ensure!(
            resolved_base == ci_scope_style_base,
            "[{scenario}] resolved base diverged: pre-repoint '{ci_scope_style_base}' vs repointed '{resolved_base}'"
        );
        ensure!(
            resolved.base_sha.as_deref() == Some(expected_base_sha.as_str()),
            "[{scenario}] base_sha diverged: expected {expected_base_sha:?}, got {:?}",
            resolved.base_sha
        );
        ensure!(
            new_paths == pre_repoint_paths,
            "[{scenario}] changed-path sets diverged: pre-repoint {pre_repoint_paths:?} vs repointed {new_paths:?}"
        );
        Ok(())
    }

    #[test]
    fn test_parity_doc_only_change() -> Result<()> {
        let tmp = tempfile::tempdir().context("failed to create tempdir")?;
        let repo = init_repo_with_origin_main(tmp.path())?;
        run_git(&repo, &["checkout", "-q", "-b", "feature"])?;
        commit_file(&repo, "docs/CHANGES.md", "release notes\n", "docs: update changelog")?;
        assert_parity_with_pre_repoint_logic("doc_only", &repo)
    }

    #[test]
    fn test_parity_single_crate_change() -> Result<()> {
        let tmp = tempfile::tempdir().context("failed to create tempdir")?;
        let repo = init_repo_with_origin_main(tmp.path())?;
        run_git(&repo, &["checkout", "-q", "-b", "feature"])?;
        commit_file(&repo, "crates/perl-parser/src/lib.rs", "pub fn parse() {}\n", "feat: parse")?;
        assert_parity_with_pre_repoint_logic("single_crate", &repo)
    }

    #[test]
    fn test_parity_multi_crate_change() -> Result<()> {
        let tmp = tempfile::tempdir().context("failed to create tempdir")?;
        let repo = init_repo_with_origin_main(tmp.path())?;
        run_git(&repo, &["checkout", "-q", "-b", "feature"])?;
        commit_file(&repo, "crates/perl-parser/src/lib.rs", "pub fn parse() {}\n", "feat: parse")?;
        commit_file(&repo, "crates/perl-lsp/src/lib.rs", "pub fn serve() {}\n", "feat: serve")?;
        assert_parity_with_pre_repoint_logic("multi_crate", &repo)
    }

    #[test]
    fn test_parity_mixed_change() -> Result<()> {
        let tmp = tempfile::tempdir().context("failed to create tempdir")?;
        let repo = init_repo_with_origin_main(tmp.path())?;
        run_git(&repo, &["checkout", "-q", "-b", "feature"])?;
        commit_file(&repo, "crates/perl-parser/src/lib.rs", "pub fn parse() {}\n", "feat: parse")?;
        commit_file(&repo, "docs/reference/STABILITY.md", "stability notes\n", "docs: stability")?;
        commit_file(&repo, ".github/workflows/ci.yml", "name: CI\n", "ci: workflow tweak")?;
        assert_parity_with_pre_repoint_logic("mixed", &repo)
    }

    #[test]
    fn test_parity_ci_config_only_change() -> Result<()> {
        let tmp = tempfile::tempdir().context("failed to create tempdir")?;
        let repo = init_repo_with_origin_main(tmp.path())?;
        run_git(&repo, &["checkout", "-q", "-b", "feature"])?;
        commit_file(&repo, ".github/workflows/ci.yml", "name: CI\n", "ci: workflow tweak")?;
        commit_file(&repo, "justfile", "default:\n\techo hi\n", "ci: justfile tweak")?;
        assert_parity_with_pre_repoint_logic("ci_config", &repo)
    }

    #[test]
    fn test_parity_deletion_is_visible() -> Result<()> {
        let tmp = tempfile::tempdir().context("failed to create tempdir")?;
        let repo = init_repo_with_origin_main(tmp.path())?;
        commit_file(&repo, "crates/perl-parser/src/legacy.rs", "pub fn old() {}\n", "add legacy")?;
        run_git(&repo, &["push", "-q", "origin", "main"])?;
        run_git(&repo, &["checkout", "-q", "-b", "feature"])?;
        run_git(&repo, &["rm", "-q", "crates/perl-parser/src/legacy.rs"])?;
        run_git(&repo, &["commit", "-q", "-m", "remove legacy module"])?;
        assert_parity_with_pre_repoint_logic("deletion", &repo)
    }

    #[test]
    fn test_parity_rename_is_visible() -> Result<()> {
        let tmp = tempfile::tempdir().context("failed to create tempdir")?;
        let repo = init_repo_with_origin_main(tmp.path())?;
        commit_file(
            &repo,
            "crates/perl-parser/src/old_name.rs",
            "pub fn f() { /* padding to help rename detection match content */ }\n",
            "add old_name",
        )?;
        run_git(&repo, &["push", "-q", "origin", "main"])?;
        run_git(&repo, &["checkout", "-q", "-b", "feature"])?;
        run_git(
            &repo,
            &["mv", "crates/perl-parser/src/old_name.rs", "crates/perl-parser/src/new_name.rs"],
        )?;
        run_git(&repo, &["commit", "-q", "-m", "rename module"])?;
        assert_parity_with_pre_repoint_logic("rename", &repo)
    }
}
