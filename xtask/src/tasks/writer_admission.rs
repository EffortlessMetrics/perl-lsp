//! Writer admission — read-only pre-admission diagnostic (#3957 W1).
//!
//! Produces one typed `AdmissionVerdict` (`PASS` / `BLOCK` / `NOT_PROVEN`)
//! for "is it safe to open a writer worktree/branch here?", with a
//! per-check breakdown. **Read-only**: this module never mutates git state,
//! the filesystem, or GitHub — it only gathers signals and reports.
//!
//! It composes the semantics of the existing report-only tooling rather
//! than reimplementing them:
//! - `scripts/swarm-doctor` — worktree inventory / dirty / disk / divergence
//!   shape (`--json` mode).
//! - `scripts/swarm-clean::branch_pr_status` — the tri-state (`open` /
//!   `none` / `unknown`) PR-ownership pattern, where `unknown` (gh absent
//!   or the query failed) is never silently promoted to a safe verdict.
//! - `scripts/clean-worktrees.sh` — the `FLOOR_GB=200` / `FLOOR_PCT=5` disk
//!   floor convention (reused verbatim, not reinvented).
//!
//! Checks (each contributes one `CheckResult` to the verdict):
//! 1. `canonical-base` — the live `refs/remotes/origin/<base>` SHA vs an
//!    optional caller-supplied expected SHA.
//! 2. `shadow-ref` — a reserved `refs/heads/origin/*` ref shadowing a
//!    remote-tracking ref (the cited real incident).
//! 3. `symbolic-head` — a dangling symbolic HEAD (points at a ref that does
//!    not resolve).
//! 4. `branch-worktree-mapping` — either the root checkout has drifted onto
//!    a feature branch (production-mutation risk), or the target branch is
//!    checked out in more than one worktree.
//! 5. `dirty-unpushed` — an abnormally large staged/dirty change set
//!    (possible synthetic mass-staged additions).
//! 6. `disk-capacity` — free disk below the `clean-worktrees.sh` floor.
//! 7. `writer-collision` — an open PR already exists for the target branch.
//!
//! An instrument failure (a check's underlying git/gh call errors) reports
//! that check as `NOT_PROVEN`, never a false `PASS` — see
//! `docs/reference/ISSUE_PLAN_DOCTRINE.md`-style report-only doctrine.
//!
//! Advisory-first: `run` always returns `Ok(())`. The verdict is
//! informational; nothing is blocked or mutated by W1 itself. Consuming the
//! verdict to gate a real writer-worktree creation is #3982's job, not
//! this module's.

use color_eyre::eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::tasks::git_context::git_stdout_with_worktree_fallback;

// ---- CLI config -------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AdmissionConfig {
    pub branch: Option<String>,
    pub base: String,
    pub worktree: Option<PathBuf>,
    pub expected_base_sha: Option<String>,
    pub repo: Option<String>,
    pub fixture: Option<PathBuf>,
    pub json: bool,
    pub floor_gb: f64,
    pub floor_pct: f64,
    pub large_staged_threshold: u32,
}

// ---- Snapshot (fixture schema / live-gathered signal bundle) ---------------

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct HeadInfo {
    #[serde(default)]
    pub symbolic_ref: Option<String>,
    #[serde(default)]
    pub resolved_sha: Option<String>,
    #[serde(default)]
    pub dangling: bool,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ShadowRefInfo {
    #[serde(default)]
    pub refs: Vec<String>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct CanonicalBaseInfo {
    #[serde(default)]
    pub remote_sha: Option<String>,
    #[serde(default)]
    pub selected_sha: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct WorktreeEntry {
    pub path: String,
    #[serde(default)]
    pub branch: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct WorktreeMappingInfo {
    #[serde(default)]
    pub entries: Vec<WorktreeEntry>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct DirtyInfo {
    #[serde(default)]
    pub status_count: u32,
    #[serde(default)]
    pub unpushed_commits: u32,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct DiskInfo {
    #[serde(default)]
    pub avail_gb: Option<f64>,
    #[serde(default)]
    pub total_gb: Option<f64>,
    #[serde(default)]
    pub worktree_count: Option<u32>,
    #[serde(default)]
    pub error: Option<String>,
}

/// Mirrors `scripts/swarm-clean::branch_pr_status`'s tri-state exactly:
/// `unknown` (gh absent or the query failed) must never be treated as
/// `none` — see that script's own comment for the rationale.
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PrStatus {
    Open,
    None,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PrOwnershipInfo {
    #[serde(default)]
    pub status: PrStatus,
    #[serde(default)]
    pub pr_number: Option<u64>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WriterAdmissionSnapshot {
    pub target_branch: String,
    #[serde(default = "default_base")]
    pub requested_base: String,
    #[serde(default)]
    pub is_root_checkout: bool,
    #[serde(default)]
    pub head: HeadInfo,
    #[serde(default)]
    pub shadow_refs: ShadowRefInfo,
    #[serde(default)]
    pub canonical_base: CanonicalBaseInfo,
    #[serde(default)]
    pub worktree_mapping: WorktreeMappingInfo,
    #[serde(default)]
    pub dirty: DirtyInfo,
    #[serde(default)]
    pub disk: DiskInfo,
    #[serde(default)]
    pub pr_ownership: PrOwnershipInfo,
}

fn default_base() -> String {
    "origin/main".to_string()
}

// ---- Verdict model -----------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CheckStatus {
    Pass,
    Block,
    NotProven,
}

impl std::fmt::Display for CheckStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            CheckStatus::Pass => "PASS",
            CheckStatus::Block => "BLOCK",
            CheckStatus::NotProven => "NOT_PROVEN",
        };
        write!(f, "{text}")
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CheckResult {
    pub name: String,
    pub status: CheckStatus,
    pub reason: String,
}

/// Distinct from `Posture` (`xtask/src/tasks/commit_checks.rs`) on purpose:
/// `Posture` is defined as never-blocking. `AdmissionVerdict::Block` is
/// explicitly a blocking signal for whatever consumes it (e.g. #3982's
/// `/start-work`) — this module must not extend `Posture` and silently
/// violate its never-blocks invariant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AdmissionVerdict {
    Pass,
    Block,
    NotProven,
}

impl std::fmt::Display for AdmissionVerdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            AdmissionVerdict::Pass => "PASS",
            AdmissionVerdict::Block => "BLOCK",
            AdmissionVerdict::NotProven => "NOT_PROVEN",
        };
        write!(f, "{text}")
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AdmissionReport {
    pub schema_version: String,
    pub mode: String,
    pub target_branch: String,
    pub verdict: AdmissionVerdict,
    pub checks: Vec<CheckResult>,
}

/// Worst-status-wins: any `Block` makes the whole verdict `Block`; absent
/// that, any `NotProven` makes it `NotProven`; only an all-`Pass` check set
/// is `Pass`.
pub fn aggregate_verdict(checks: &[CheckResult]) -> AdmissionVerdict {
    if checks.iter().any(|c| c.status == CheckStatus::Block) {
        AdmissionVerdict::Block
    } else if checks.iter().any(|c| c.status == CheckStatus::NotProven) {
        AdmissionVerdict::NotProven
    } else {
        AdmissionVerdict::Pass
    }
}

// ---- Entry point --------------------------------------------------------------

pub fn run(config: AdmissionConfig) -> Result<()> {
    let snapshot = load_snapshot(&config)?;
    let checks = run_checks(&snapshot, &config);
    let verdict = aggregate_verdict(&checks);
    let report = AdmissionReport {
        schema_version: "1".to_string(),
        mode: "advisory".to_string(),
        target_branch: snapshot.target_branch.clone(),
        verdict,
        checks,
    };
    print_report(&config, &report)?;
    // Advisory-first: W1 produces a verdict, it never blocks the command
    // itself. A consumer (e.g. #3982's /start-work) decides what to do
    // with AdmissionVerdict::Block.
    Ok(())
}

fn load_snapshot(config: &AdmissionConfig) -> Result<WriterAdmissionSnapshot> {
    if let Some(path) = &config.fixture {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("failed to read fixture {}", path.display()))?;
        let mut snapshot: WriterAdmissionSnapshot = serde_json::from_str(&contents)
            .with_context(|| format!("failed to parse fixture {}", path.display()))?;
        if let Some(branch) = &config.branch {
            snapshot.target_branch = branch.clone();
        }
        return Ok(snapshot);
    }
    Ok(gather_live_snapshot(config))
}

// ---- Checks --------------------------------------------------------------------

pub fn run_checks(
    snapshot: &WriterAdmissionSnapshot,
    config: &AdmissionConfig,
) -> Vec<CheckResult> {
    vec![
        check_canonical_base(snapshot),
        check_shadow_ref(snapshot),
        check_symbolic_head(snapshot),
        check_branch_worktree_mapping(snapshot),
        check_dirty_unpushed(snapshot, config),
        check_disk_capacity(snapshot, config),
        check_writer_collision(snapshot),
    ]
}

fn check_canonical_base(snapshot: &WriterAdmissionSnapshot) -> CheckResult {
    let name = "canonical-base".to_string();
    let info = &snapshot.canonical_base;
    if let Some(err) = &info.error {
        return CheckResult {
            name,
            status: CheckStatus::NotProven,
            reason: format!("could not resolve `{}`: {err}", snapshot.requested_base),
        };
    }
    let Some(remote_sha) = &info.remote_sha else {
        return CheckResult {
            name,
            status: CheckStatus::NotProven,
            reason: format!("no SHA resolved for `{}`", snapshot.requested_base),
        };
    };
    match &info.selected_sha {
        // No expectation was supplied — nothing to compare against.
        None => CheckResult {
            name,
            status: CheckStatus::Pass,
            reason: format!("`{}` resolves to {remote_sha}", snapshot.requested_base),
        },
        Some(selected_sha) if sha_matches(remote_sha, selected_sha) => CheckResult {
            name,
            status: CheckStatus::Pass,
            reason: format!("`{}` matches expected {remote_sha}", snapshot.requested_base),
        },
        Some(selected_sha) => CheckResult {
            name,
            status: CheckStatus::Block,
            reason: format!(
                "base ref mismatch: `{}` is {remote_sha} live, expected {selected_sha}",
                snapshot.requested_base
            ),
        },
    }
}

/// A caller-supplied `--expected-base-sha` is commonly a short/abbreviated
/// SHA (this is how commits are conventionally referenced in issues, PR
/// descriptions, and plan-review comments — e.g. "currently 6fade008c"),
/// not necessarily the full 40-hex-character SHA. Exact string equality
/// would false-BLOCK every short-SHA caller even when the base is exactly
/// correct. Accept a full match OR a case-insensitive hex-prefix match, with
/// a minimum prefix length (git's own historical default abbreviation is 7
/// hex characters) to avoid a degenerate short prefix matching too loosely.
fn sha_matches(remote_sha: &str, selected_sha: &str) -> bool {
    const MIN_PREFIX_LEN: usize = 4;
    if selected_sha.eq_ignore_ascii_case(remote_sha) {
        return true;
    }
    selected_sha.len() >= MIN_PREFIX_LEN
        && selected_sha.len() <= remote_sha.len()
        && selected_sha.chars().all(|c| c.is_ascii_hexdigit())
        && remote_sha[..selected_sha.len()].eq_ignore_ascii_case(selected_sha)
}

fn check_shadow_ref(snapshot: &WriterAdmissionSnapshot) -> CheckResult {
    let name = "shadow-ref".to_string();
    let info = &snapshot.shadow_refs;
    if let Some(err) = &info.error {
        return CheckResult {
            name,
            status: CheckStatus::NotProven,
            reason: format!("could not scan `refs/heads/origin/*`: {err}"),
        };
    }
    if info.refs.is_empty() {
        return CheckResult {
            name,
            status: CheckStatus::Pass,
            reason: "no reserved `refs/heads/origin/*` shadow refs found".to_string(),
        };
    }
    CheckResult {
        name,
        status: CheckStatus::Block,
        reason: format!(
            "reserved `refs/heads/origin/*` shadow ref(s) found, shadowing remote-tracking refs: {}",
            info.refs.join(", ")
        ),
    }
}

fn check_symbolic_head(snapshot: &WriterAdmissionSnapshot) -> CheckResult {
    let name = "symbolic-head".to_string();
    let info = &snapshot.head;
    if let Some(err) = &info.error {
        return CheckResult {
            name,
            status: CheckStatus::NotProven,
            reason: format!("could not resolve HEAD: {err}"),
        };
    }
    if info.dangling {
        return CheckResult {
            name,
            status: CheckStatus::Block,
            reason: format!(
                "HEAD is dangling (symbolic_ref={:?}, resolved_sha={:?})",
                info.symbolic_ref, info.resolved_sha
            ),
        };
    }
    CheckResult {
        name,
        status: CheckStatus::Pass,
        reason: match &info.symbolic_ref {
            Some(sym) => format!("HEAD -> {sym}, resolved"),
            None => "HEAD is detached but resolves cleanly".to_string(),
        },
    }
}

fn canonical_main_name(base: &str) -> &str {
    base.rsplit('/').next().unwrap_or(base)
}

fn check_branch_worktree_mapping(snapshot: &WriterAdmissionSnapshot) -> CheckResult {
    let name = "branch-worktree-mapping".to_string();
    let info = &snapshot.worktree_mapping;
    if let Some(err) = &info.error {
        return CheckResult {
            name,
            status: CheckStatus::NotProven,
            reason: format!("worktree inventory unavailable: {err}"),
        };
    }

    // Root-checkout health: production writes must never mutate the root
    // checkout in place — it must stay on the canonical base (or detached
    // at it), never drift onto a feature branch (#3957's "root checkout on
    // a feature branch" negative case).
    if snapshot.is_root_checkout {
        if let Some(sym) = &snapshot.head.symbolic_ref {
            let current_branch = sym.strip_prefix("refs/heads/").unwrap_or(sym);
            let canonical = canonical_main_name(&snapshot.requested_base);
            if current_branch != canonical && current_branch != "master" {
                return CheckResult {
                    name,
                    status: CheckStatus::Block,
                    reason: format!(
                        "root checkout is on feature branch `{current_branch}`, expected the \
                         canonical base `{canonical}` (production writes belong in a worktree)"
                    ),
                };
            }
        }
        return CheckResult {
            name,
            status: CheckStatus::Pass,
            reason: "root checkout is on the canonical base (or detached at it)".to_string(),
        };
    }

    // Linked worktree: the target branch must not already be checked out
    // elsewhere (git itself refuses this, but a stale/corrupted admin dir
    // entry could still surface it — detect rather than assume).
    let matches: Vec<&str> = info
        .entries
        .iter()
        .filter(|e| e.branch.as_deref() == Some(snapshot.target_branch.as_str()))
        .map(|e| e.path.as_str())
        .collect();
    if matches.len() > 1 {
        return CheckResult {
            name,
            status: CheckStatus::Block,
            reason: format!(
                "branch `{}` is checked out in {} worktrees: {}",
                snapshot.target_branch,
                matches.len(),
                matches.join(", ")
            ),
        };
    }

    CheckResult {
        name,
        status: CheckStatus::Pass,
        reason: "branch/worktree mapping is unambiguous".to_string(),
    }
}

fn check_dirty_unpushed(
    snapshot: &WriterAdmissionSnapshot,
    config: &AdmissionConfig,
) -> CheckResult {
    let name = "dirty-unpushed".to_string();
    let info = &snapshot.dirty;
    if let Some(err) = &info.error {
        return CheckResult {
            name,
            status: CheckStatus::NotProven,
            reason: format!("could not read working-tree status: {err}"),
        };
    }
    if info.status_count > config.large_staged_threshold {
        return CheckResult {
            name,
            status: CheckStatus::Block,
            reason: format!(
                "abnormally large change set: {} file(s) changed (threshold {}) — possible \
                 synthetic mass-staged additions",
                info.status_count, config.large_staged_threshold
            ),
        };
    }
    CheckResult {
        name,
        status: CheckStatus::Pass,
        reason: format!(
            "{} file(s) changed, {} unpushed commit(s) (informational)",
            info.status_count, info.unpushed_commits
        ),
    }
}

fn check_disk_capacity(
    snapshot: &WriterAdmissionSnapshot,
    config: &AdmissionConfig,
) -> CheckResult {
    let name = "disk-capacity".to_string();
    let info = &snapshot.disk;
    if let Some(err) = &info.error {
        return CheckResult {
            name,
            status: CheckStatus::NotProven,
            reason: format!("could not read disk usage: {err}"),
        };
    }
    let Some(avail_gb) = info.avail_gb else {
        return CheckResult {
            name,
            status: CheckStatus::NotProven,
            reason: "no disk-availability data reported".to_string(),
        };
    };
    // Matches scripts/clean-worktrees.sh's floor convention verbatim:
    // max(FLOOR_GB, FLOOR_PCT% of total volume size).
    let pct_floor = info.total_gb.map(|total| total * config.floor_pct / 100.0).unwrap_or(0.0);
    let floor = config.floor_gb.max(pct_floor);
    let worktree_note =
        info.worktree_count.map(|n| format!(", {n} worktree(s) present")).unwrap_or_default();
    if avail_gb < floor {
        return CheckResult {
            name,
            status: CheckStatus::Block,
            reason: format!(
                "disk headroom {avail_gb:.1}G is below the floor {floor:.1}G \
                 (max(FLOOR_GB={}, FLOOR_PCT={}%)){worktree_note}",
                config.floor_gb, config.floor_pct
            ),
        };
    }
    CheckResult {
        name,
        status: CheckStatus::Pass,
        reason: format!(
            "disk headroom {avail_gb:.1}G is above the floor {floor:.1}G{worktree_note}"
        ),
    }
}

fn check_writer_collision(snapshot: &WriterAdmissionSnapshot) -> CheckResult {
    let name = "writer-collision".to_string();
    let info = &snapshot.pr_ownership;
    if let Some(err) = &info.error {
        return CheckResult {
            name,
            status: CheckStatus::NotProven,
            reason: format!("could not query PR ownership: {err}"),
        };
    }
    match info.status {
        PrStatus::Open => CheckResult {
            name,
            status: CheckStatus::Block,
            reason: match info.pr_number {
                Some(n) => format!(
                    "open PR #{n} already exists for branch `{}` — writer collision",
                    snapshot.target_branch
                ),
                None => format!(
                    "an open PR already exists for branch `{}` — writer collision",
                    snapshot.target_branch
                ),
            },
        },
        PrStatus::None => CheckResult {
            name,
            status: CheckStatus::Pass,
            reason: format!("no open PR for branch `{}`", snapshot.target_branch),
        },
        PrStatus::Unknown => CheckResult {
            name,
            status: CheckStatus::NotProven,
            // gh absent or the query failed — never silently treated as
            // "none" (see scripts/swarm-clean::branch_pr_status).
            reason: "gh unavailable or the PR-ownership query failed — not provable".to_string(),
        },
    }
}

// ---- Live gathering --------------------------------------------------------------

/// Runs a read-only `git` query and returns its non-empty stdout lines.
///
/// A non-zero exit is always treated as a genuine instrument failure
/// (`Err`), never silently folded into "no results". This matters for
/// callers like `for-each-ref`: it exits **0** with empty stdout when
/// nothing matches a pattern — a real, legitimate absence — so a non-zero
/// exit from it means something actually went wrong (corrupt ref store,
/// invalid pattern, not a git repository at all), not "zero matches".
/// Conflating the two would let a genuine failure silently report as a
/// clean/empty result — the exact instrument-failure-must-never-be-PASS
/// invariant this tool exists to uphold. Callers that DO have a
/// documented, command-specific "non-zero exit legitimately means
/// absence" case (e.g. `git symbolic-ref -q HEAD` on a detached HEAD)
/// must not route through this helper — they inspect `Command::output()`
/// directly, as `gather_head_info` does.
fn git_lines(root: &Path, args: &[&str]) -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|e| format!("failed to spawn `git {}`: {e}", args.join(" ")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("git {} failed with status {}", args.join(" "), output.status)
        } else {
            format!("git {} failed: {stderr}", args.join(" "))
        });
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text.lines().filter(|l| !l.is_empty()).map(str::to_string).collect())
}

/// Builds the early-return `HeadInfo` for a `git` spawn failure encountered
/// while gathering HEAD info. Carries forward whatever fields an earlier
/// `git` call already resolved (e.g. `symbolic_ref`) so a later spawn error
/// can never silently discard already-known state via `..Default::default()`.
fn head_info_spawn_error(symbolic_ref: Option<String>, message: String) -> HeadInfo {
    HeadInfo { symbolic_ref, error: Some(message), ..Default::default() }
}

fn gather_head_info(root: &Path) -> HeadInfo {
    let symbolic =
        Command::new("git").args(["symbolic-ref", "-q", "HEAD"]).current_dir(root).output();
    let symbolic_ref = match symbolic {
        Ok(out) if out.status.success() => {
            let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if text.is_empty() { None } else { Some(text) }
        }
        // Non-zero exit from `symbolic-ref -q` legitimately means detached
        // HEAD, not an error.
        Ok(_) => None,
        Err(e) => {
            return head_info_spawn_error(None, format!("failed to spawn git symbolic-ref: {e}"));
        }
    };

    let resolved = Command::new("git")
        .args(["rev-parse", "-q", "--verify", "HEAD"])
        .current_dir(root)
        .output();
    let resolved_sha = match resolved {
        Ok(out) if out.status.success() => {
            let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if text.is_empty() { None } else { Some(text) }
        }
        Ok(_) => None,
        Err(e) => {
            return head_info_spawn_error(
                symbolic_ref,
                format!("failed to spawn git rev-parse: {e}"),
            );
        }
    };

    // Dangling: HEAD does not resolve to a commit at all (broken symbolic
    // target, or a genuinely empty/corrupt ref).
    let dangling = resolved_sha.is_none();

    HeadInfo { symbolic_ref, resolved_sha, dangling, error: None }
}

fn gather_shadow_refs(root: &Path) -> ShadowRefInfo {
    match git_lines(root, &["for-each-ref", "--format=%(refname)", "refs/heads/origin/"]) {
        Ok(refs) => ShadowRefInfo { refs, error: None },
        Err(err) => ShadowRefInfo { refs: Vec::new(), error: Some(err) },
    }
}

fn gather_canonical_base(root: &Path, config: &AdmissionConfig) -> CanonicalBaseInfo {
    let output = Command::new("git")
        .args(["rev-parse", "-q", "--verify", &config.base])
        .current_dir(root)
        .output();
    match output {
        Ok(out) if out.status.success() => {
            let sha = String::from_utf8_lossy(&out.stdout).trim().to_string();
            CanonicalBaseInfo {
                remote_sha: Some(sha),
                selected_sha: config.expected_base_sha.clone(),
                error: None,
            }
        }
        Ok(out) => CanonicalBaseInfo {
            remote_sha: None,
            selected_sha: config.expected_base_sha.clone(),
            error: Some(format!(
                "git rev-parse --verify {} failed: {}",
                config.base,
                String::from_utf8_lossy(&out.stderr).trim()
            )),
        },
        Err(e) => CanonicalBaseInfo {
            remote_sha: None,
            selected_sha: config.expected_base_sha.clone(),
            error: Some(format!("failed to spawn git rev-parse: {e}")),
        },
    }
}

fn gather_worktree_mapping(root: &Path) -> WorktreeMappingInfo {
    let output =
        Command::new("git").args(["worktree", "list", "--porcelain"]).current_dir(root).output();
    let text = match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).into_owned(),
        Ok(out) => {
            return WorktreeMappingInfo {
                entries: Vec::new(),
                error: Some(format!(
                    "git worktree list failed: {}",
                    String::from_utf8_lossy(&out.stderr).trim()
                )),
            };
        }
        Err(e) => {
            return WorktreeMappingInfo {
                entries: Vec::new(),
                error: Some(format!("failed to spawn git worktree list: {e}")),
            };
        }
    };

    let mut entries = Vec::new();
    let mut path: Option<String> = None;
    let mut branch: Option<String> = None;
    for line in text.lines() {
        if let Some(p) = line.strip_prefix("worktree ") {
            if let Some(prev_path) = path.take() {
                entries.push(WorktreeEntry { path: prev_path, branch: branch.take() });
            }
            path = Some(p.to_string());
        } else if let Some(b) = line.strip_prefix("branch ") {
            branch = Some(b.trim_start_matches("refs/heads/").to_string());
        } else if line == "detached" {
            branch = None;
        }
    }
    if let Some(prev_path) = path.take() {
        entries.push(WorktreeEntry { path: prev_path, branch: branch.take() });
    }

    WorktreeMappingInfo { entries, error: None }
}

fn gather_dirty_info(worktree_root: &Path) -> DirtyInfo {
    let status =
        Command::new("git").args(["status", "--porcelain"]).current_dir(worktree_root).output();
    let status_count = match status {
        Ok(out) if out.status.success() => {
            String::from_utf8_lossy(&out.stdout).lines().filter(|l| !l.is_empty()).count() as u32
        }
        Ok(out) => {
            return DirtyInfo {
                status_count: 0,
                unpushed_commits: 0,
                error: Some(format!(
                    "git status failed: {}",
                    String::from_utf8_lossy(&out.stderr).trim()
                )),
            };
        }
        Err(e) => {
            return DirtyInfo {
                status_count: 0,
                unpushed_commits: 0,
                error: Some(format!("failed to spawn git status: {e}")),
            };
        }
    };

    // No upstream configured is a normal state for a brand-new branch, not
    // an instrument failure — treat as zero unpushed commits.
    let unpushed_commits = Command::new("git")
        .args(["rev-list", "@{u}..HEAD", "--count"])
        .current_dir(worktree_root)
        .output()
        .ok()
        .filter(|out| out.status.success())
        .and_then(|out| String::from_utf8_lossy(&out.stdout).trim().parse::<u32>().ok())
        .unwrap_or(0);

    DirtyInfo { status_count, unpushed_commits, error: None }
}

/// Disposition note (reviewed, not changed): dividing 1024-byte blocks by
/// `1024*1024` yields **binary** gigabytes (GiB), not decimal GB, even
/// though the fields/flags are named `_gb`/`FLOOR_GB`. This is intentional
/// bug-for-bug compatibility with `scripts/clean-worktrees.sh`, which the
/// #3957 W1 spec requires reusing verbatim (`avail_gb=$((avail_kb / 1024 /
/// 1024))`, displayed as `${avail_gb}G`) rather than inventing a new
/// convention. The numerator and denominator always use the same units
/// here, so the floor comparison itself is correct; only the "GB" naming
/// is imprecise. Not relabeled to avoid diverging the CLI flag names
/// (`--floor-gb`, matching `FLOOR_GB`) from the reused source.
fn gather_disk_info(root: &Path, worktree_count: u32) -> DiskInfo {
    let output = Command::new("df").args(["-Pk", "."]).current_dir(root).output();
    match output {
        Ok(out) if out.status.success() => {
            let text = String::from_utf8_lossy(&out.stdout);
            let Some(fields_line) = text.lines().nth(1) else {
                return DiskInfo {
                    avail_gb: None,
                    total_gb: None,
                    worktree_count: Some(worktree_count),
                    error: Some("df produced no data line".to_string()),
                };
            };
            let fields: Vec<&str> = fields_line.split_whitespace().collect();
            // POSIX df -Pk: Filesystem 1024-blocks Used Available Capacity Mounted-on
            let total_kb = fields.get(1).and_then(|s| s.parse::<f64>().ok());
            let avail_kb = fields.get(3).and_then(|s| s.parse::<f64>().ok());
            DiskInfo {
                avail_gb: avail_kb.map(|kb| kb / (1024.0 * 1024.0)),
                total_gb: total_kb.map(|kb| kb / (1024.0 * 1024.0)),
                worktree_count: Some(worktree_count),
                error: None,
            }
        }
        Ok(out) => DiskInfo {
            avail_gb: None,
            total_gb: None,
            worktree_count: Some(worktree_count),
            error: Some(format!("df failed: {}", String::from_utf8_lossy(&out.stderr).trim())),
        },
        Err(e) => DiskInfo {
            avail_gb: None,
            total_gb: None,
            worktree_count: Some(worktree_count),
            error: Some(format!("failed to spawn df: {e}")),
        },
    }
}

/// Tri-state PR-ownership lookup — mirrors
/// `scripts/swarm-clean::branch_pr_status` exactly: gh absent or the query
/// failing must map to `Unknown`, never `None`.
fn gather_pr_ownership(branch: &str, repo: Option<&str>) -> PrOwnershipInfo {
    if branch.is_empty() {
        // An empty `--head` filter is not "no filter" from gh's point of
        // view in every code path — it must never be sent, or the query
        // can match an unrelated PR and misattribute a writer collision.
        return PrOwnershipInfo { status: PrStatus::Unknown, pr_number: None, error: None };
    }
    if which_gh().is_none() {
        return PrOwnershipInfo { status: PrStatus::Unknown, pr_number: None, error: None };
    }
    let mut cmd = Command::new("gh");
    cmd.args(["pr", "list", "--head", branch, "--state", "open", "--json", "number"]);
    if let Some(repo) = repo {
        cmd.args(["--repo", repo]);
    }
    let output = cmd.output();
    match output {
        Ok(out) if out.status.success() => {
            let text = String::from_utf8_lossy(&out.stdout);
            match serde_json::from_str::<Vec<serde_json::Value>>(&text) {
                Ok(items) if items.is_empty() => {
                    PrOwnershipInfo { status: PrStatus::None, pr_number: None, error: None }
                }
                Ok(items) => {
                    let pr_number =
                        items.first().and_then(|v| v.get("number")).and_then(|n| n.as_u64());
                    PrOwnershipInfo { status: PrStatus::Open, pr_number, error: None }
                }
                Err(e) => PrOwnershipInfo {
                    status: PrStatus::Unknown,
                    pr_number: None,
                    error: Some(format!("could not parse gh pr list output: {e}")),
                },
            }
        }
        Ok(_) | Err(_) => {
            PrOwnershipInfo { status: PrStatus::Unknown, pr_number: None, error: None }
        }
    }
}

fn which_gh() -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    let exe_name = if cfg!(windows) { "gh.exe" } else { "gh" };
    std::env::split_paths(&path_var)
        .map(|dir| dir.join(exe_name))
        .find(|candidate| candidate.is_file())
}

fn gather_live_snapshot(config: &AdmissionConfig) -> WriterAdmissionSnapshot {
    let root = config.worktree.clone().unwrap_or_else(|| PathBuf::from("."));

    let head = gather_head_info(&root);

    let is_root_checkout = {
        let git_dir = git_stdout_with_worktree_fallback(&root, &["rev-parse", "--git-dir"]).ok();
        let common_dir =
            git_stdout_with_worktree_fallback(&root, &["rev-parse", "--git-common-dir"]).ok();
        match (git_dir, common_dir) {
            (Some(a), Some(b)) => normalize_dir(&root, &a) == normalize_dir(&root, &b),
            _ => false,
        }
    };

    let branch = config.branch.clone().or_else(|| {
        head.symbolic_ref.as_ref().map(|s| s.strip_prefix("refs/heads/").unwrap_or(s).to_string())
    });
    let target_branch = branch.unwrap_or_else(|| "(detached)".to_string());

    let worktree_mapping = gather_worktree_mapping(&root);
    let worktree_count = worktree_mapping.entries.len() as u32;

    // Use the *resolved* target branch (never the raw, possibly-absent
    // `--branch` flag) for the PR-ownership query. Querying `gh pr list
    // --head ""` would silently drop the filter and return an arbitrary
    // open PR — a false writer-collision BLOCK against an unrelated
    // branch, which is worse than a false PASS: it would misattribute a
    // real incident. A detached (branch-less) checkout has no PR to
    // collide with, so that case is Unknown/not-applicable, not queried.
    let pr_ownership = if target_branch == "(detached)" {
        PrOwnershipInfo { status: PrStatus::None, pr_number: None, error: None }
    } else {
        gather_pr_ownership(&target_branch, config.repo.as_deref())
    };

    WriterAdmissionSnapshot {
        target_branch,
        requested_base: config.base.clone(),
        is_root_checkout,
        head,
        shadow_refs: gather_shadow_refs(&root),
        canonical_base: gather_canonical_base(&root, config),
        worktree_mapping,
        dirty: gather_dirty_info(&root),
        disk: gather_disk_info(&root, worktree_count),
        pr_ownership,
    }
}

fn normalize_dir(root: &Path, raw: &str) -> PathBuf {
    let candidate = Path::new(raw);
    let joined =
        if candidate.is_absolute() { candidate.to_path_buf() } else { root.join(candidate) };
    fs::canonicalize(&joined).unwrap_or(joined)
}

// ---- Output --------------------------------------------------------------------

fn print_report(config: &AdmissionConfig, report: &AdmissionReport) -> Result<()> {
    if config.json {
        let rendered = serde_json::to_string_pretty(report)?;
        println!("{rendered}");
        return Ok(());
    }
    println!("Writer Admission [{}]: {} — {}", report.mode, report.target_branch, report.verdict);
    for check in &report.checks {
        println!("  [{}] {}: {}", check.status, check.name, check.reason);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_snapshot() -> WriterAdmissionSnapshot {
        WriterAdmissionSnapshot {
            target_branch: "impl/1234-feature".to_string(),
            requested_base: "origin/main".to_string(),
            is_root_checkout: false,
            head: HeadInfo {
                symbolic_ref: Some("refs/heads/impl/1234-feature".to_string()),
                resolved_sha: Some("abc123".to_string()),
                dangling: false,
                error: None,
            },
            shadow_refs: ShadowRefInfo { refs: Vec::new(), error: None },
            canonical_base: CanonicalBaseInfo {
                remote_sha: Some("deadbeef".to_string()),
                selected_sha: None,
                error: None,
            },
            worktree_mapping: WorktreeMappingInfo {
                entries: vec![WorktreeEntry {
                    path: "/repo/.claude/worktrees/agent-1".to_string(),
                    branch: Some("impl/1234-feature".to_string()),
                }],
                error: None,
            },
            dirty: DirtyInfo { status_count: 0, unpushed_commits: 1, error: None },
            disk: DiskInfo {
                avail_gb: Some(500.0),
                total_gb: Some(2000.0),
                worktree_count: Some(3),
                error: None,
            },
            pr_ownership: PrOwnershipInfo { status: PrStatus::None, pr_number: None, error: None },
        }
    }

    fn default_config() -> AdmissionConfig {
        AdmissionConfig {
            branch: None,
            base: "origin/main".to_string(),
            worktree: None,
            expected_base_sha: None,
            repo: None,
            fixture: None,
            json: false,
            floor_gb: 200.0,
            floor_pct: 5.0,
            large_staged_threshold: 1000,
        }
    }

    #[test]
    fn healthy_snapshot_is_pass() {
        let snapshot = base_snapshot();
        let checks = run_checks(&snapshot, &default_config());
        assert!(checks.iter().all(|c| c.status == CheckStatus::Pass), "{checks:?}");
        assert_eq!(aggregate_verdict(&checks), AdmissionVerdict::Pass);
    }

    #[test]
    fn shadow_ref_blocks() {
        let mut snapshot = base_snapshot();
        snapshot.shadow_refs.refs = vec!["refs/heads/origin/main".to_string()];
        let checks = run_checks(&snapshot, &default_config());
        assert_eq!(aggregate_verdict(&checks), AdmissionVerdict::Block);
        assert!(
            checks.iter().any(|c| c.name == "shadow-ref" && c.status == CheckStatus::Block),
            "expected shadow-ref check present and blocking: {checks:?}"
        );
    }

    #[test]
    fn dangling_head_blocks_without_mutating_anything() {
        let mut snapshot = base_snapshot();
        snapshot.head.dangling = true;
        snapshot.head.resolved_sha = None;
        let checks = run_checks(&snapshot, &default_config());
        assert_eq!(aggregate_verdict(&checks), AdmissionVerdict::Block);
        assert!(
            checks.iter().any(|c| c.name == "symbolic-head" && c.status == CheckStatus::Block),
            "expected symbolic-head check present and blocking: {checks:?}"
        );
    }

    #[test]
    fn detached_root_checkout_at_canonical_base_is_pass() {
        let mut snapshot = base_snapshot();
        snapshot.is_root_checkout = true;
        snapshot.target_branch = "(detached)".to_string();
        snapshot.head.symbolic_ref = None;
        snapshot.worktree_mapping.entries =
            vec![WorktreeEntry { path: "/repo".to_string(), branch: None }];
        let checks = run_checks(&snapshot, &default_config());
        assert_eq!(aggregate_verdict(&checks), AdmissionVerdict::Pass, "{checks:?}");
    }

    #[test]
    fn root_checkout_on_feature_branch_blocks() {
        let mut snapshot = base_snapshot();
        snapshot.is_root_checkout = true;
        let checks = run_checks(&snapshot, &default_config());
        assert_eq!(aggregate_verdict(&checks), AdmissionVerdict::Block);
        assert!(
            checks
                .iter()
                .any(|c| c.name == "branch-worktree-mapping" && c.status == CheckStatus::Block),
            "expected branch-worktree-mapping check present and blocking: {checks:?}"
        );
    }

    #[test]
    fn duplicate_worktree_mapping_blocks() {
        let mut snapshot = base_snapshot();
        snapshot.worktree_mapping.entries.push(WorktreeEntry {
            path: "/repo/.claude/worktrees/agent-2".to_string(),
            branch: Some("impl/1234-feature".to_string()),
        });
        let checks = run_checks(&snapshot, &default_config());
        assert_eq!(aggregate_verdict(&checks), AdmissionVerdict::Block);
    }

    #[test]
    fn base_mismatch_blocks() {
        let mut snapshot = base_snapshot();
        snapshot.canonical_base.selected_sha = Some("stale-sha".to_string());
        let checks = run_checks(&snapshot, &default_config());
        assert_eq!(aggregate_verdict(&checks), AdmissionVerdict::Block);
    }

    #[test]
    fn short_sha_prefix_of_the_correct_base_is_not_a_false_block() {
        // --expected-base-sha is commonly a short SHA (e.g. "currently
        // 6fade008c" is how commits get cited in issues/plan-reviews).
        // Exact-string equality would false-BLOCK this every time even
        // though the base is exactly correct.
        let mut snapshot = base_snapshot();
        // base_snapshot()'s remote_sha is "deadbeef".
        snapshot.canonical_base.selected_sha = Some("dead".to_string());
        let checks = run_checks(&snapshot, &default_config());
        assert_eq!(aggregate_verdict(&checks), AdmissionVerdict::Pass, "{checks:?}");
    }

    #[test]
    fn short_sha_that_is_genuinely_a_different_commit_still_blocks() {
        let mut snapshot = base_snapshot();
        // Not a prefix of "deadbeef" at all — a real mismatch, short or not.
        snapshot.canonical_base.selected_sha = Some("beef".to_string());
        let checks = run_checks(&snapshot, &default_config());
        assert_eq!(aggregate_verdict(&checks), AdmissionVerdict::Block, "{checks:?}");
    }

    #[test]
    fn sha_matches_accepts_exact_and_prefix_rejects_short_and_wrong() {
        assert!(sha_matches("deadbeef", "deadbeef"));
        assert!(sha_matches("deadbeef", "DEAD"), "case-insensitive prefix should match");
        assert!(!sha_matches("deadbeef", "dea"), "below the minimum prefix length must not match");
        assert!(!sha_matches("deadbeef", "beef"), "a non-prefix substring must not match");
        assert!(
            !sha_matches("deadbeef", "deadbeefff"),
            "longer than the remote SHA must not match"
        );
        assert!(!sha_matches("deadbeef", "dead-bad"), "non-hex characters must not match");
    }

    #[test]
    fn low_disk_blocks() {
        let mut snapshot = base_snapshot();
        snapshot.disk.avail_gb = Some(10.0);
        let checks = run_checks(&snapshot, &default_config());
        assert_eq!(aggregate_verdict(&checks), AdmissionVerdict::Block);
    }

    #[test]
    fn large_staged_change_set_blocks() {
        let mut snapshot = base_snapshot();
        snapshot.dirty.status_count = 5000;
        let checks = run_checks(&snapshot, &default_config());
        assert_eq!(aggregate_verdict(&checks), AdmissionVerdict::Block);
    }

    #[test]
    fn open_pr_is_writer_collision_block() {
        let mut snapshot = base_snapshot();
        snapshot.pr_ownership =
            PrOwnershipInfo { status: PrStatus::Open, pr_number: Some(42), error: None };
        let checks = run_checks(&snapshot, &default_config());
        assert_eq!(aggregate_verdict(&checks), AdmissionVerdict::Block);
    }

    #[test]
    fn gh_unavailable_is_not_proven_never_pass() {
        let mut snapshot = base_snapshot();
        snapshot.pr_ownership =
            PrOwnershipInfo { status: PrStatus::Unknown, pr_number: None, error: None };
        let checks = run_checks(&snapshot, &default_config());
        assert_eq!(aggregate_verdict(&checks), AdmissionVerdict::NotProven);
        assert!(
            checks
                .iter()
                .any(|c| c.name == "writer-collision" && c.status == CheckStatus::NotProven),
            "expected writer-collision check present and NOT_PROVEN: {checks:?}"
        );
    }

    #[test]
    fn tool_error_on_any_check_is_not_proven_never_pass() {
        let mut snapshot = base_snapshot();
        snapshot.disk.error = Some("df: command not found".to_string());
        snapshot.disk.avail_gb = None;
        let checks = run_checks(&snapshot, &default_config());
        assert_eq!(aggregate_verdict(&checks), AdmissionVerdict::NotProven);
    }

    #[test]
    fn aggregate_prefers_block_over_not_proven() {
        let checks = vec![
            CheckResult {
                name: "a".to_string(),
                status: CheckStatus::NotProven,
                reason: String::new(),
            },
            CheckResult {
                name: "b".to_string(),
                status: CheckStatus::Block,
                reason: String::new(),
            },
        ];
        assert_eq!(aggregate_verdict(&checks), AdmissionVerdict::Block);
    }

    #[test]
    fn fixture_deserializes_with_minimal_fields() -> Result<()> {
        let json = r#"{"target_branch": "impl/1-x"}"#;
        let snapshot: WriterAdmissionSnapshot = serde_json::from_str(json)?;
        assert_eq!(snapshot.target_branch, "impl/1-x");
        assert_eq!(snapshot.requested_base, "origin/main");
        assert!(!snapshot.is_root_checkout);
        assert_eq!(snapshot.pr_ownership.status, PrStatus::Unknown);
        Ok(())
    }

    #[test]
    fn empty_branch_never_reaches_the_pr_ownership_query() {
        // Regression test for a real bug caught by a live smoke test: when
        // no --branch is supplied and HEAD can't be resolved to a name,
        // the PR-ownership lookup must never be sent an empty `--head`
        // filter (gh silently drops it and can match an unrelated open
        // PR, misattributing a writer collision). Empty branch must map
        // straight to Unknown without spawning gh at all.
        let info = gather_pr_ownership("", None);
        assert_eq!(info.status, PrStatus::Unknown);
        assert!(info.pr_number.is_none());
    }

    #[test]
    fn shadow_ref_check_with_error_is_not_proven_never_pass() {
        // Regression guard on the check-level contract: an empty `refs`
        // list is legitimate ("no matches", e.g. `for-each-ref` exiting 0
        // with nothing to report) and must PASS, but the SAME empty list
        // paired with a genuine tool error must never be folded into that
        // same PASS — it must be NOT_PROVEN. This is the invariant the
        // live-gathering bug below violated: a real `git for-each-ref`
        // failure was indistinguishable from a legitimate empty match.
        let mut snapshot = base_snapshot();
        snapshot.shadow_refs = ShadowRefInfo {
            refs: Vec::new(),
            error: Some("git for-each-ref failed: fatal: not a git repository".to_string()),
        };
        let checks = run_checks(&snapshot, &default_config());
        assert_eq!(aggregate_verdict(&checks), AdmissionVerdict::NotProven, "{checks:?}");
        assert!(
            checks.iter().any(|c| c.name == "shadow-ref" && c.status == CheckStatus::NotProven),
            "expected shadow-ref to be NOT_PROVEN, never PASS, on a tool error: {checks:?}"
        );
    }

    #[test]
    fn gather_shadow_refs_on_a_genuine_git_failure_reports_error_not_empty() -> Result<()> {
        // The actual live-gathering bug: `git for-each-ref` exits 0 with
        // empty stdout when nothing matches (a legitimate absence), so a
        // NON-zero exit means something is genuinely wrong (here: not a
        // git repository at all). Before the fix, `git_lines` swallowed
        // ANY non-zero exit into `Ok(vec![])`, so this genuine failure
        // was indistinguishable from "no shadow refs found" — a silent
        // instrument-failure-to-false-PASS. It must instead surface as
        // `ShadowRefInfo.error`, which `check_shadow_ref` already routes
        // to NOT_PROVEN (see the check-level test above).
        let dir = tempfile::tempdir()?;
        // dir.path() is deliberately NOT a git repository.
        let info = gather_shadow_refs(dir.path());
        assert!(
            info.error.is_some(),
            "expected a genuine git failure (not a git repository) to be \
             surfaced as an error, not silently folded into an empty \
             match list: {info:?}"
        );
        assert!(info.refs.is_empty());
        Ok(())
    }

    #[test]
    fn head_info_spawn_error_preserves_already_gathered_symbolic_ref() -> Result<()> {
        use perl_tdd_support::must_some;
        // Regression for the writer-admission fast-follow (#3957 W1): the
        // `git rev-parse` spawn-error arm of `gather_head_info` used to
        // build its early-return `HeadInfo` with `..Default::default()`,
        // which reset `symbolic_ref` back to `None` even though it had
        // already been successfully resolved by the prior `git
        // symbolic-ref` call. `head_info_spawn_error` is the exact helper
        // that arm calls, so this pins the fix at the unit the bug lived
        // in: the already-known `symbolic_ref` must survive a later
        // spawn error, not be silently discarded.
        let info = head_info_spawn_error(
            Some("refs/heads/impl/1234-feature".to_string()),
            "failed to spawn git rev-parse: boom".to_string(),
        );
        let symbolic_ref = must_some(info.symbolic_ref.clone());
        assert_eq!(symbolic_ref, "refs/heads/impl/1234-feature");
        let error = must_some(info.error.clone());
        assert_eq!(error, "failed to spawn git rev-parse: boom");
        assert_eq!(info.resolved_sha, None);
        assert!(!info.dangling);
        Ok(())
    }

    #[test]
    fn head_info_spawn_error_with_no_prior_symbolic_ref_stays_none() -> Result<()> {
        use perl_tdd_support::must_some;
        // Symmetric case: the `git symbolic-ref` spawn-error arm has
        // nothing gathered yet, so it correctly passes `None` through.
        let info =
            head_info_spawn_error(None, "failed to spawn git symbolic-ref: boom".to_string());
        assert_eq!(info.symbolic_ref, None);
        let error = must_some(info.error.clone());
        assert_eq!(error, "failed to spawn git symbolic-ref: boom");
        Ok(())
    }
}
