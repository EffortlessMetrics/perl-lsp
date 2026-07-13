//! Worktree maintenance helpers used by local agent operations.
//!
//! `cleanup` classifies every worktree under `.claude/worktrees/` as either
//! `Keep` (with a reason) or `Remove` before touching anything. It defaults
//! to a **dry-run report** — nothing is deleted unless the caller opts in
//! with `--force`. Even under `--force`, a worktree is never removed when
//! it is:
//!
//! - **dirty** — `git status --porcelain` reports uncommitted changes;
//! - **locked** — `git worktree list --porcelain` reports a `locked` entry;
//! - on a branch with an **open PR** (or PR status could not be determined —
//!   "unknown" is treated as unsafe, never as "no PR", to avoid destroying a
//!   worktree that turns out to have a live PR when `gh` is unavailable);
//! - has a **merged PR whose head the worktree has moved past** — a merged
//!   PR only proves *that commit's* content landed, not that the worktree
//!   is still sitting at it. The same local branch/worktree can keep
//!   accumulating commits after its PR merges (re-aiming the same builder
//!   at the next round of work before opening the next PR), and those
//!   post-merge commits are invisible to every other guard here. Only when
//!   the worktree's local `HEAD` is *exactly* the commit GitHub recorded as
//!   the merged PR's `headRefOid` is it Remove-eligible on this basis;
//!   otherwise Keep;
//! - has **unpushed commits and no merged PR** — the branch's `HEAD` has
//!   commits not present on its upstream (`@{u}`), or, when no upstream is
//!   configured, not present on `origin/main`/`origin/master`, *and* no
//!   merged PR exists for it either. A clean worktree can still hold
//!   committed-but-unpushed work, and losing it is just as much data loss
//!   as losing uncommitted changes. This repo squash-merges exclusively, so
//!   a branch whose PR was squash-merged is *always* "ahead" of
//!   `origin/main` by commit count (the squash commit is never an ancestor
//!   of the original commits) — a merged-PR-at-current-HEAD hit from `gh`
//!   is checked first and, if found, overrides the ahead-by-ancestry signal
//!   (fail-safe: if neither the upstream/default-branch reference nor the
//!   merged-PR query can be resolved, "unpushed" is assumed);
//! - the **root checkout**.
//!
//! Removal itself never passes `--force` to `git worktree remove` — a
//! concurrent write between classification and removal makes git refuse,
//! and that refusal is logged and skipped rather than treated as a fatal
//! error for the whole run.
//!
//! This mirrors `scripts/swarm-clean`'s dry-run-first, classify-before-delete
//! convention (including its no-`--force` removal and merged-PR-by-head
//! query). It intentionally does *not* implement that script's full
//! KEEP/CACHE-ONLY/REMOVE/SALVAGE/REVIEW bucket taxonomy (active-lock-with-
//! live-pid, etc.) — this is the narrow safety guard against unconditional
//! force-removal; the fuller classification is tracked separately (#3957
//! W2). See issue #4097.

use crate::utils::project_root;
use color_eyre::eyre::{Result, bail};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::str;

/// One worktree entry parsed from `git worktree list --porcelain`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct WorktreeEntry {
    path: PathBuf,
    /// `None` for a detached-HEAD worktree.
    branch: Option<String>,
    locked: bool,
    lock_reason: Option<String>,
}

/// Classification outcome for a single worktree.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Verdict {
    Remove,
    Keep(String),
}

/// Tri-state result of asking whether a branch has an open PR.
///
/// `Unknown` (gh absent, unauthenticated, or the query otherwise failed)
/// must never be treated the same as `None` — doing so would let a
/// worktree with a live PR be removed on a machine where `gh` can't answer.
#[derive(Debug, Clone, PartialEq, Eq)]
enum PrStatus {
    Open(u64),
    None,
    Unknown,
}

/// Tri-state result of asking whether a branch's PR has already been
/// **merged**.
///
/// This repo merges exclusively via GitHub's squash-merge (confirmed:
/// `allow_squash_merge=true`, `delete_branch_on_merge=true`, no merge
/// commits in `origin/main` history — every landed change is a single
/// `(#N)` squash commit). A squash-merge commit is never an ancestor of the
/// original branch's commits, so ancestry-based checks like
/// [`has_unpushed_commits`] cannot tell "already landed via squash-merge"
/// apart from "genuinely never pushed" — both look "ahead" of
/// `origin/main`/`@{u}` by commit count. Querying `gh pr list --state
/// merged --head <branch>` is authoritative instead: GitHub still resolves
/// a PR by its recorded head-branch name even after the branch ref itself
/// has been deleted (the `delete_branch_on_merge` default here).
///
/// `Merged` is matched by **branch name only** — it does not by itself mean
/// "safe to remove". The same local branch name can accumulate commits
/// *after* its PR merged (a builder re-aiming at the same worktree before
/// opening the next PR is a normal swarm pattern), and those post-merge
/// commits are invisible to every other guard: `worktree_dirty` only
/// catches *uncommitted* changes, and a merged PR's ancestry makes
/// [`has_unpushed_commits`] moot. `Merged` therefore carries the PR's
/// recorded `head_ref_oid` — the exact commit GitHub squash-merged — so the
/// caller can compare it against the worktree's actual local `HEAD` and
/// only treat the worktree as safe when they're identical (see `classify`).
///
/// `Unknown` (gh absent, unauthenticated, or the query otherwise failed)
/// must never be treated the same as `NotMerged` — doing so could let an
/// unmerged branch be removed on a machine where `gh` can't answer.
#[derive(Debug, Clone, PartialEq, Eq)]
enum MergeStatus {
    Merged { number: u64, head_ref_oid: String },
    NotMerged,
    Unknown,
}

/// Environment variable that overrides the `gh` binary invoked to check PR
/// status. Used by tests to inject a stub; unset in production, in which
/// case the real `gh` on `PATH` is used.
const GH_BIN_OVERRIDE_ENV: &str = "XTASK_WORKTREE_CLEANUP_GH_BIN";

fn gh_program() -> String {
    std::env::var(GH_BIN_OVERRIDE_ENV).unwrap_or_else(|_| "gh".to_string())
}

/// Report (and, with `force`, remove) stale `.claude/worktrees` entries.
///
/// `root` defaults to the perl-lsp workspace root (via [`project_root`])
/// when `None`; tests pass an explicit fixture root.
///
/// # Errors
///
/// Returns an error when:
/// - `root` is `None` and the workspace root cannot be resolved
///   ([`project_root`]);
/// - `git worktree prune` or `git worktree list --porcelain` fails to run
///   or exits non-zero;
/// - the `git worktree list --porcelain` output is not valid UTF-8;
/// - the final `git worktree prune` (after removal) fails to run or exits
///   non-zero.
///
/// A single worktree's `git worktree remove` refusing (e.g. it became dirty
/// between classification and removal) is *not* an error here — see the
/// module docs — it's logged and skipped so the rest of the batch still
/// gets cleaned up.
pub fn cleanup(root: Option<PathBuf>, force: bool) -> Result<()> {
    let root = match root {
        Some(root) => root,
        None => project_root()?,
    };
    run(&root, force)
}

fn run(root: &Path, force: bool) -> Result<()> {
    let prune_status =
        Command::new("git").current_dir(root).args(["worktree", "prune"]).status()?;
    if !prune_status.success() {
        bail!("failed to prune git worktrees");
    }

    let list_output = Command::new("git")
        .current_dir(root)
        .args(["worktree", "list", "--porcelain"])
        .stdout(Stdio::piped())
        .output()?;
    if !list_output.status.success() {
        bail!("failed to list git worktrees");
    }

    let list = str::from_utf8(&list_output.stdout)?;
    let entries: Vec<WorktreeEntry> =
        parse_worktree_list(list).into_iter().filter(|e| is_agent_worktree(&e.path)).collect();

    println!("Found {} agent worktree(s) under .claude/worktrees/", entries.len());

    if entries.is_empty() {
        println!("No stale worktrees to clean up");
        return Ok(());
    }

    println!(
        "=== {} ===",
        if force {
            "Removing worktrees classified REMOVE"
        } else {
            "Dry-run report (pass --force to remove REMOVE-classified worktrees)"
        }
    );

    let mut to_remove: Vec<&WorktreeEntry> = Vec::new();
    let mut keep_count = 0usize;
    for entry in &entries {
        let verdict = classify(root, entry);
        match &verdict {
            Verdict::Remove => {
                let action = if force { "REMOVE" } else { "REMOVE (dry-run: would remove)" };
                println!("{action:<32} {}", entry.path.display());
                to_remove.push(entry);
            }
            Verdict::Keep(reason) => {
                println!("KEEP                             {} ({reason})", entry.path.display());
                keep_count += 1;
            }
        }
    }

    // Printed before removal actually runs (even in --force mode), so this
    // reports the classification outcome ("to remove") rather than a
    // post-hoc completion count — the per-line REMOVE / REMOVE (dry-run: ...)
    // markers above and the "Cleanup complete" message below distinguish
    // dry-run from an executed removal.
    println!();
    println!("=== summary: {} to remove, {keep_count} kept ===", to_remove.len());

    if !force {
        println!();
        println!("(Dry-run. Re-run with --force to remove REMOVE-classified worktrees.)");
        return Ok(());
    }

    // Deliberately no `--force` here: `to_remove` is a snapshot taken by the
    // classification pass above, before any of these `git worktree remove`
    // calls run. A concurrent agent can write to (or commit into) one of
    // these worktrees in the gap between classification and this loop
    // reaching it — the classification pass can take a while, since it does
    // a `gh` round trip per entry. `--force` bypasses git's own
    // last-resort "worktree is not clean" refusal, which is exactly the
    // hazard #4097 exists to close; running without it keeps that refusal
    // live as a second, independent safety net. Mirrors
    // `scripts/swarm-clean`'s `remove_worktree()`, which uses the same
    // no-`--force` + warn-and-continue pattern.
    //
    // A refusal here must not abort the whole cleanup run — that would let
    // one raced worktree block cleaning every other, genuinely-safe entry
    // in the same batch. Log a skip warning and move on.
    //
    // Not every refusal is the concurrent-write race the doc comment above
    // describes — a submodule/permission error, a filesystem error, or an
    // externally-held lock can also make `git worktree remove` exit
    // non-zero. Captured (not inherited) so its stderr can be surfaced in
    // the warning: silently attributing every refusal to "concurrent
    // write" would mask a genuinely unexpected, worth-investigating
    // failure behind a reassuring but potentially wrong explanation.
    for entry in &to_remove {
        println!("Removing: {}", entry.path.display());
        let remove_output = Command::new("git")
            .current_dir(root)
            .args(["worktree", "remove"])
            .arg(&entry.path)
            .output()?;
        if !remove_output.status.success() {
            let stderr = String::from_utf8_lossy(&remove_output.stderr);
            let stderr = stderr.trim();
            println!(
                "  -> WARNING: skipped {} — `git worktree remove` refused (most likely a \
                 concurrent write landed since classification, but see git's message below) \
                 — keeping{}",
                entry.path.display(),
                if stderr.is_empty() { String::new() } else { format!(": {stderr}") }
            );
        }
    }

    let final_prune_status =
        Command::new("git").current_dir(root).args(["worktree", "prune"]).status()?;
    if !final_prune_status.success() {
        bail!("failed to prune git worktrees after cleanup");
    }

    println!("Cleanup complete");
    Ok(())
}

/// Does `path` live under a `.claude/worktrees/` directory? Git always
/// normalizes worktree list paths to forward slashes, even on Windows.
///
/// Checked as a path-component boundary (`.claude/worktrees` as an
/// ancestor), not a raw substring match, so a path that merely embeds the
/// literal text elsewhere (e.g. as part of a longer directory name) isn't
/// misclassified.
fn is_agent_worktree(path: &Path) -> bool {
    let normalized = path.to_string_lossy().replace('\\', "/");
    Path::new(&normalized).ancestors().any(|a| a.ends_with(Path::new(".claude/worktrees")))
}

/// Classify a single worktree as `Remove` or `Keep(reason)`.
///
/// Order matters, and is deliberate:
///
/// 1. Cheap, purely-local checks (root, locked, dirty) run first, so a
///    dirty or locked worktree never needs a `gh` round trip to be kept
///    safe.
/// 2. **Open-PR status** runs next: an open PR always means Keep.
/// 3. **Merged-PR status** runs only when there is no open PR: a `gh`
///    "merged" hit is the authoritative "this branch's content already
///    landed" signal (see [`MergeStatus`] on why ancestry alone can't tell
///    this apart from "never pushed" under this repo's squash-merge
///    convention) — but it's Remove-eligible *only* when the worktree's
///    local `HEAD` still equals the merged PR's recorded `headRefOid`.
///    If `HEAD` has moved past it (more commits since the merge, on the
///    same branch/worktree), that's Keep — the merge proves the *old*
///    content landed, not the new commits.
/// 4. Only once neither an open nor a merged-and-unmoved PR exists do
///    local **unpushed-commits** ancestry checks decide: ahead of `@{u}`
///    (or the default remote branch) means Keep, otherwise Remove.
///
/// Any `gh` failure/uncertainty (`PrStatus::Unknown` /
/// `MergeStatus::Unknown`) means Keep, never Remove — fail-safe, matching
/// the dirty-check `Err` philosophy.
fn classify(root: &Path, entry: &WorktreeEntry) -> Verdict {
    if paths_match(&entry.path, root) {
        return Verdict::Keep("root checkout — never removed".to_string());
    }

    if entry.locked {
        return match &entry.lock_reason {
            Some(reason) if !reason.is_empty() => Verdict::Keep(format!("locked: {reason}")),
            _ => Verdict::Keep("locked".to_string()),
        };
    }

    match worktree_dirty(&entry.path) {
        Ok(true) => return Verdict::Keep("dirty — uncommitted changes present".to_string()),
        Ok(false) => {}
        Err(error) => {
            return Verdict::Keep(format!(
                "could not determine dirty status ({error}) — not safe to remove"
            ));
        }
    }

    let Some(branch) = &entry.branch else {
        return Verdict::Keep("detached HEAD — no branch to verify PR status".to_string());
    };

    match branch_pr_status(root, branch) {
        PrStatus::Open(number) => {
            return Verdict::Keep(format!("open PR #{number} exists for branch '{branch}'"));
        }
        PrStatus::Unknown => {
            return Verdict::Keep(format!(
                "PR status for branch '{branch}' could not be determined (gh unavailable) \
                 — not safe to remove"
            ));
        }
        PrStatus::None => {}
    }

    match branch_merge_status(root, branch) {
        MergeStatus::Merged { number, head_ref_oid } => {
            // A merged PR only proves *that commit's* content landed — not
            // that the worktree is still sitting at it. The same local
            // branch/worktree can keep accumulating commits after its PR
            // merges (re-aiming the same builder at the next round of
            // work before opening the next PR is a normal swarm pattern),
            // and those post-merge commits are invisible to every other
            // guard here: `worktree_dirty` only catches *uncommitted*
            // changes, and ancestry-based `has_unpushed_commits` is moot
            // once a merge is confirmed. Only Remove when local HEAD is
            // *exactly* the commit GitHub squash-merged.
            return match worktree_head(&entry.path) {
                Ok(local_head) if local_head == head_ref_oid => Verdict::Remove,
                Ok(_) => Verdict::Keep(format!(
                    "branch '{branch}' has a merged PR #{number}, but the worktree's HEAD \
                     is past the merged commit — not safe to remove"
                )),
                Err(error) => Verdict::Keep(format!(
                    "could not determine worktree HEAD to compare against merged PR #{number} \
                     ({error}) — not safe to remove"
                )),
            };
        }
        MergeStatus::Unknown => {
            return Verdict::Keep(format!(
                "merged-PR status for branch '{branch}' could not be determined (gh \
                 unavailable) — not safe to remove"
            ));
        }
        MergeStatus::NotMerged => {}
    }

    match has_unpushed_commits(&entry.path) {
        Ok(true) => Verdict::Keep(
            "unpushed commits — branch is ahead of its upstream/remote, and has no open or \
             merged PR — not safe to remove"
                .to_string(),
        ),
        Ok(false) => Verdict::Remove,
        Err(error) => Verdict::Keep(format!(
            "could not determine unpushed-commit status ({error}) — not safe to remove"
        )),
    }
}

fn paths_match(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

/// `true` if the worktree at `path` has uncommitted changes.
fn worktree_dirty(path: &Path) -> Result<bool> {
    let output = Command::new("git")
        .current_dir(path)
        .args(["status", "--porcelain"])
        .stdout(Stdio::piped())
        .output()?;
    if !output.status.success() {
        bail!("git status --porcelain exited non-zero");
    }
    Ok(!output.stdout.is_empty())
}

/// `true` if the worktree at `path` has commits on `HEAD` that are not
/// provably present on any remote — i.e. removing it would discard
/// unsalvaged committed work.
///
/// Preference order for the "known safe" reference to compare `HEAD`
/// against:
///
/// 1. The branch's configured upstream (`@{u}`), if one is set — the most
///    precise signal, since it names exactly the ref this branch pushes to.
/// 2. `origin/main` or `origin/master`, if a remote-tracking ref for one of
///    those exists — the common case for a freshly created agent worktree
///    that hasn't had its first `git push -u` yet (no upstream configured,
///    but its commits may already all be present on the default branch).
/// 3. Neither resolves: we cannot prove *anything* about this branch is
///    pushed anywhere, so — matching the fail-safe philosophy already used
///    for a dirty-check `Err` and `PrStatus::Unknown` — treat it as
///    unpushed rather than risk deleting real work.
fn has_unpushed_commits(path: &Path) -> Result<bool> {
    let reference = match resolve_upstream(path) {
        Some(upstream) => upstream,
        None => match resolve_default_remote_ref(path) {
            Some(default_ref) => default_ref,
            None => return Ok(true),
        },
    };

    let output = Command::new("git")
        .current_dir(path)
        .args(["rev-list", "--count", &format!("{reference}..HEAD")])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;
    if !output.status.success() {
        bail!(
            "git rev-list --count {reference}..HEAD exited non-zero: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let count: u64 = text.parse().map_err(|error| {
        color_eyre::eyre::eyre!("failed to parse ahead-count '{text}' from git rev-list: {error}")
    })?;
    Ok(count > 0)
}

/// The branch's configured push/pull upstream (`@{u}`), if any is set.
/// `None` covers both "no upstream configured" and any other resolution
/// failure — the caller falls back to comparing against a default remote
/// branch in either case.
fn resolve_upstream(path: &Path) -> Option<String> {
    let output = Command::new("git")
        .current_dir(path)
        .args(["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() { None } else { Some(text) }
}

/// The first of `origin/main` / `origin/master` that resolves to an actual
/// remote-tracking ref in this repository, if any.
fn resolve_default_remote_ref(path: &Path) -> Option<String> {
    for candidate in ["origin/main", "origin/master"] {
        let status = Command::new("git")
            .current_dir(path)
            .args(["rev-parse", "--verify", "--quiet", &format!("refs/remotes/{candidate}")])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        if matches!(status, Ok(status) if status.success()) {
            return Some(candidate.to_string());
        }
    }
    None
}

/// Query whether `branch` has an open PR, via `gh pr list`. Any failure
/// (gh absent, unauthenticated, no network, no remote configured) yields
/// `Unknown` rather than `None` — the caller must treat `Unknown` as unsafe.
///
/// `gh` resolves the target owner/repo from the git remote of its current
/// working directory. It **must** run with `root` (the repo being
/// classified) as its cwd, not whatever cwd the xtask process happens to
/// have inherited — otherwise a `--root` that differs from the ambient
/// process cwd would silently query the wrong repo, turning "PR status
/// unknown" into a false "no PR" and defeating the entire open-PR guard.
fn branch_pr_status(root: &Path, branch: &str) -> PrStatus {
    let output = Command::new(gh_program())
        .current_dir(root)
        .args([
            "pr",
            "list",
            "--head",
            branch,
            "--state",
            "open",
            "--json",
            "number",
            "--jq",
            ".[0].number",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if text.is_empty() {
                PrStatus::None
            } else {
                match text.parse::<u64>() {
                    Ok(number) => PrStatus::Open(number),
                    Err(_) => PrStatus::Unknown,
                }
            }
        }
        _ => PrStatus::Unknown,
    }
}

/// A merged PR's `number` and `headRefOid`, as selected by `gh`'s `--jq
/// '.[0] // empty'` post-filter.
#[derive(Debug, Clone, serde::Deserialize)]
struct MergedPrJson {
    number: u64,
    #[serde(rename = "headRefOid")]
    head_ref_oid: String,
}

/// Query whether `branch` has a **merged** PR, via `gh pr list --state
/// merged`. See [`MergeStatus`] for why this — not an ancestry diff — is
/// the authoritative "content already landed" signal in a repo that
/// squash-merges exclusively, and why `headRefOid` (not just a bare
/// merged/not-merged bit) is required.
///
/// Uses `--jq '.[0] // empty'` (not `.[0].number` like [`branch_pr_status`])
/// specifically so a no-match result is unambiguously **empty stdout** —
/// jq's `empty` generator produces zero output values by construction,
/// unlike relying on however a bare `null` happens to render — while a
/// match still comes back as one JSON object line, parsed directly with
/// `serde_json` rather than a second `--jq` field extraction. An unexpected
/// shape (or any other query failure) fails safe as `Unknown`.
///
/// Same cwd requirement as [`branch_pr_status`]: `gh` infers the target
/// repo from its current working directory, so this must run with `root`
/// as cwd, not the ambient process cwd.
fn branch_merge_status(root: &Path, branch: &str) -> MergeStatus {
    let output = Command::new(gh_program())
        .current_dir(root)
        .args([
            "pr",
            "list",
            "--head",
            branch,
            "--state",
            "merged",
            "--json",
            "number,headRefOid",
            "--jq",
            ".[0] // empty",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    match output {
        Ok(out) if out.status.success() => {
            if out.stdout.iter().all(u8::is_ascii_whitespace) {
                MergeStatus::NotMerged
            } else {
                match serde_json::from_slice::<MergedPrJson>(&out.stdout) {
                    Ok(pr) => {
                        MergeStatus::Merged { number: pr.number, head_ref_oid: pr.head_ref_oid }
                    }
                    Err(_) => MergeStatus::Unknown,
                }
            }
        }
        _ => MergeStatus::Unknown,
    }
}

/// The worktree's current local `HEAD` commit SHA.
fn worktree_head(path: &Path) -> Result<String> {
    let output = Command::new("git")
        .current_dir(path)
        .args(["rev-parse", "HEAD"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;
    if !output.status.success() {
        bail!("git rev-parse HEAD exited non-zero: {}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Parse `git worktree list --porcelain` output into entries.
fn parse_worktree_list(list: &str) -> Vec<WorktreeEntry> {
    let mut entries = Vec::new();
    let mut path: Option<PathBuf> = None;
    let mut branch: Option<String> = None;
    let mut locked = false;
    let mut lock_reason: Option<String> = None;

    let flush = |path: &mut Option<PathBuf>,
                 branch: &mut Option<String>,
                 locked: &mut bool,
                 lock_reason: &mut Option<String>,
                 entries: &mut Vec<WorktreeEntry>| {
        if let Some(p) = path.take() {
            entries.push(WorktreeEntry {
                path: p,
                branch: branch.take(),
                locked: *locked,
                lock_reason: lock_reason.take(),
            });
        }
        *locked = false;
    };

    for line in list.lines() {
        if let Some(rest) = line.strip_prefix("worktree ") {
            flush(&mut path, &mut branch, &mut locked, &mut lock_reason, &mut entries);
            path = Some(PathBuf::from(rest));
        } else if let Some(rest) = line.strip_prefix("branch ") {
            branch = Some(rest.strip_prefix("refs/heads/").unwrap_or(rest).to_string());
        } else if let Some(rest) = line.strip_prefix("locked") {
            locked = true;
            let reason = rest.strip_prefix(' ').unwrap_or("").trim().to_string();
            lock_reason = if reason.is_empty() { None } else { Some(reason) };
        }
        // "HEAD <sha>", "detached", and blank separator lines carry no
        // information this classifier needs.
    }
    flush(&mut path, &mut branch, &mut locked, &mut lock_reason, &mut entries);

    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_worktree_list ──────────────────────────────────────────────

    #[test]
    fn parses_branch_locked_and_detached_entries() {
        let porcelain = "\
worktree /repo
HEAD abc123
detached

worktree /repo/.claude/worktrees/agent-1
HEAD def456
branch refs/heads/impl/123-foo

worktree /repo/.claude/worktrees/agent-2
HEAD ghi789
branch refs/heads/impl/456-bar
locked claude agent agent-2 (pid 4242)
";
        let entries = parse_worktree_list(porcelain);
        assert_eq!(entries.len(), 3);

        assert_eq!(entries[0].path, PathBuf::from("/repo"));
        assert_eq!(entries[0].branch, None);
        assert!(!entries[0].locked);

        assert_eq!(entries[1].path, PathBuf::from("/repo/.claude/worktrees/agent-1"));
        assert_eq!(entries[1].branch.as_deref(), Some("impl/123-foo"));
        assert!(!entries[1].locked);

        assert_eq!(entries[2].path, PathBuf::from("/repo/.claude/worktrees/agent-2"));
        assert_eq!(entries[2].branch.as_deref(), Some("impl/456-bar"));
        assert!(entries[2].locked);
        assert_eq!(entries[2].lock_reason.as_deref(), Some("claude agent agent-2 (pid 4242)"));
    }

    #[test]
    fn parses_lock_with_no_reason() {
        let porcelain = "\
worktree /repo/.claude/worktrees/agent-3
HEAD abc123
branch refs/heads/impl/789-baz
locked
";
        let entries = parse_worktree_list(porcelain);
        assert_eq!(entries.len(), 1);
        assert!(entries[0].locked);
        assert_eq!(entries[0].lock_reason, None);
    }

    // ── is_agent_worktree ────────────────────────────────────────────────

    #[test]
    fn is_agent_worktree_matches_forward_and_back_slash_paths() {
        assert!(is_agent_worktree(Path::new("/repo/.claude/worktrees/agent-1")));
        assert!(is_agent_worktree(Path::new(r"H:\repo\.claude\worktrees\agent-1")));
        assert!(!is_agent_worktree(Path::new("/repo")));
        assert!(!is_agent_worktree(Path::new("/tmp/some-other-worktree")));
    }

    // ── classify: pure branches that don't need a real gh/git call ────────

    #[test]
    fn classify_keeps_root_checkout() -> Result<()> {
        let root = Path::new("/repo");
        let entry = WorktreeEntry {
            path: PathBuf::from("/repo"),
            branch: Some("main".to_string()),
            locked: false,
            lock_reason: None,
        };
        match classify(root, &entry) {
            Verdict::Keep(reason) => {
                assert!(reason.contains("root checkout"));
                Ok(())
            }
            Verdict::Remove => bail!("root checkout must never be classified Remove"),
        }
    }

    #[test]
    fn classify_keeps_locked_worktree_with_reason() -> Result<()> {
        let root = Path::new("/repo");
        let entry = WorktreeEntry {
            path: PathBuf::from("/repo/.claude/worktrees/agent-1"),
            branch: Some("impl/1".to_string()),
            locked: true,
            lock_reason: Some("claude agent agent-1 (pid 99)".to_string()),
        };
        match classify(root, &entry) {
            Verdict::Keep(reason) => {
                assert!(reason.contains("locked"));
                assert!(reason.contains("pid 99"));
                Ok(())
            }
            Verdict::Remove => bail!("locked worktree must never be classified Remove"),
        }
    }

    #[test]
    fn classify_keeps_locked_worktree_without_reason() -> Result<()> {
        let root = Path::new("/repo");
        let entry = WorktreeEntry {
            path: PathBuf::from("/repo/.claude/worktrees/agent-1"),
            branch: Some("impl/1".to_string()),
            locked: true,
            lock_reason: None,
        };
        match classify(root, &entry) {
            Verdict::Keep(reason) => {
                assert_eq!(reason, "locked");
                Ok(())
            }
            Verdict::Remove => bail!("locked worktree must never be classified Remove"),
        }
    }
}
