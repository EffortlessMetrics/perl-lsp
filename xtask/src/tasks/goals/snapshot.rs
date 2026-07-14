//! Adapters: the ONLY place in the `goals` module that shells `git`/`gh`
//! or reads manifests from disk. Builds a [`SelectionSnapshot`] for the
//! pure `select::select_next`. `--fixture <path>` replaces the live
//! `gh pr list` call with a JSON fixture (mirrors
//! `tasks::queue_snapshot::run_snapshot`'s `--fixture` pattern) so tests
//! never need `gh auth`/network; manifests are always read from the real
//! repo tree, since they ARE the system under test.
//!
//! Read-only guarantee: this module never creates a branch, worktree, or
//! PR, and never writes any file under `.perl-lsp/goals/`, `target/`, or
//! `docs/` — it only shells read-only `git`/`gh` subcommands (`rev-parse`,
//! `repo view`, `pr list`) and reads manifests. Branch/worktree/PR
//! creation is `/start-pr`'s job (a separate, later deliverable).

use crate::utils::project_root;
use color_eyre::eyre::{Context, Result, bail};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use toml::Value;

use super::manifest;
use super::select::{
    LiveOpenPr, MilestoneCandidate, MilestoneStatus, ProgramCandidate, ReconciliationFinding,
    SelectionSnapshot, ambiguity_detail, parse_status, reconcile_in_progress,
};

#[derive(Debug, Default, Deserialize)]
struct LivePrFixture {
    #[serde(default)]
    repository: Option<String>,
    #[serde(default)]
    prs: Vec<LiveOpenPr>,
    /// MERGED PRs, consumed only by `goals reconcile` (`load_merged_prs_for_candidates`).
    /// Absent in pre-existing `goals next` fixtures, which default to empty
    /// and are therefore unaffected by this addition.
    #[serde(default)]
    merged_prs: Vec<LiveOpenPr>,
}

pub fn build_snapshot(
    program_arg: Option<String>,
    fixture: Option<PathBuf>,
) -> Result<SelectionSnapshot> {
    build_snapshot_at(&project_root()?, program_arg, fixture)
}

/// Root-parameterized core of [`build_snapshot`], split out so tests (and
/// `build_reconciliation_report_at`) can point it at a temporary directory
/// instead of the real repository tree — mirrors the existing
/// root-parameterized pattern already used by `resolve_program` and
/// `load_candidates_for_program` below.
fn build_snapshot_at(
    root: &Path,
    program_arg: Option<String>,
    fixture: Option<PathBuf>,
) -> Result<SelectionSnapshot> {
    // Schema 3 is a portfolio, not a repository-global routing pointer.
    // Legacy fields remain parseable for compatibility but are deliberately
    // ignored here; only an explicit `--program` may select one program until
    // the portfolio candidate compiler is introduced.
    let default_program = None;

    let known_programs = discover_known_programs(root)?;

    // Fail closed rather than erroring: an explicitly requested program
    // that isn't discoverable under `.perl-lsp/goals/programs/` leaves
    // `resolved_program` unset so `select_next` reports the structured
    // `ambiguous_program_authority` blocker (with `requested_program`
    // naming what was asked for and `known_programs` listing valid ids)
    // instead of `goals next --json` exiting with a non-JSON error —
    // `--json` callers must always get parseable output.
    //
    // Legacy default_program selection was removed by the portfolio schema;
    // No legacy pointer is consulted by this portfolio path.
    // Explicit `--program` still runs through the same
    // `manifest::validate_program_id` check the static
    // program-id validation used by the compatibility validator, so
    // the two can never drift or fail open again.
    let resolved_program =
        resolve_program(root, program_arg.as_deref(), default_program.as_deref());
    let resolved_program = match (program_arg.as_deref(), resolved_program) {
        (Some(_), Some(program_id))
            if portfolio_program_enabled(root, &program_id)?.is_some_and(|enabled| !enabled) =>
        {
            None
        }
        (_, resolved_program) => resolved_program,
    };

    let (repository, live_open_prs, live_prs_available) = load_live_prs(root, fixture)?;
    let current_git_ref = current_git_ref(root);

    let mut snapshot = SelectionSnapshot {
        repository,
        requested_program: program_arg,
        default_program,
        known_programs,
        resolved_program: resolved_program.clone(),
        mode: "maintainer".to_owned(),
        board: None,
        program_title: None,
        tracker_issue: None,
        non_goals: Vec::new(),
        candidates: Vec::new(),
        live_open_prs,
        current_git_ref,
        live_prs_available,
    };

    if let Some(program_id) = &resolved_program {
        load_candidates_for_program(root, program_id, &mut snapshot)?;
    }

    Ok(snapshot)
}

/// Resolves an explicitly requested program for `goals next`:
/// `--program` wins when given (even if invalid — it never falls back to
/// the portfolio, matching the explicit `--program` contract),
/// portfolio state is never an implicit fallback. The explicit id must pass
/// `manifest::validate_program_id` (bare id, no path separators/`..`, and
/// an existing manifest under `.perl-lsp/goals/programs/`) or resolution
/// fails closed to `None` — `select_next` turns that into
/// `Blocked(ambiguous_program_authority)` rather than ever guessing or
/// loading an unvalidated path. Pure/fs-read-only (no shelling), so it is
/// unit-testable without a live `gh` call.
fn resolve_program(
    root: &Path,
    program_arg: Option<&str>,
    default_program: Option<&str>,
) -> Option<String> {
    let candidate = program_arg.or(default_program)?;
    manifest::validate_program_id(root, candidate).ok().map(|()| candidate.to_owned())
}

/// Returns the schema-3 portfolio enablement for an explicitly requested
/// program. Legacy manifests have no portfolio enablement authority and keep
/// their existing explicit-selection behavior.
fn portfolio_program_enabled(root: &Path, program_id: &str) -> Result<Option<bool>> {
    let path = root.join(".perl-lsp/goals/active.toml");
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error).with_context(|| format!("failed to read {}", path.display()));
        }
    };
    let value: Value =
        toml::from_str(&text).with_context(|| format!("failed to parse {}", path.display()))?;
    if value.get("schema").and_then(Value::as_integer) != Some(3) {
        return Ok(None);
    }

    let Some(programs) = value.get("program").and_then(Value::as_array) else {
        bail!("schema-3 portfolio in {} must contain a [[program]] array", path.display());
    };
    let enabled = programs.iter().find_map(|program| {
        let table = program.as_table()?;
        (table.get("id").and_then(Value::as_str) == Some(program_id))
            .then(|| table.get("enabled").and_then(Value::as_bool).unwrap_or(false))
    });
    Ok(Some(enabled.unwrap_or(false)))
}

/// Measures the actual local git ref this snapshot's evidence was read
/// from (see #3692 defect 6): `WorkPacket::inputs_used` previously
/// hardcoded the literal `"origin/main"` regardless of what was actually
/// checked out, misattributing the receipt's own evidence on a feature
/// branch. Falls back to `"unknown"` when `git` itself is unavailable or
/// fails (this must never turn into an `Err` — provenance is
/// best-effort, not load-bearing for selection correctness), and to the
/// short commit SHA when `HEAD` is detached (so the value still names
/// something concrete rather than the literal string `"HEAD"`).
fn current_git_ref(root: &Path) -> String {
    let branch = Command::new("git")
        .current_dir(root)
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .filter(|s| !s.is_empty());

    match branch {
        Some(name) if name != "HEAD" => name,
        _ => Command::new("git")
            .current_dir(root)
            .args(["rev-parse", "--short", "HEAD"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| format!("detached@{}", String::from_utf8_lossy(&o.stdout).trim()))
            .filter(|s| s != "detached@")
            .unwrap_or_else(|| "unknown".to_owned()),
    }
}

fn discover_known_programs(root: &Path) -> Result<Vec<ProgramCandidate>> {
    let dir = root.join(".perl-lsp/goals/programs");
    let mut programs = Vec::new();
    if dir.exists() {
        for entry in
            fs::read_dir(&dir).with_context(|| format!("failed to read {}", dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("toml")
                && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
            {
                programs.push(ProgramCandidate { id: stem.to_owned() });
            }
        }
    }
    programs.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(programs)
}

/// Returns `(repository, open PRs, live_prs_available)`. `--fixture` input
/// is always deterministic and therefore always "available"; a live `gh`
/// call that fails (unauthenticated, offline, rate-limited) now fails
/// CLOSED on availability rather than erroring the whole `goals next --json`
/// invocation (#3696 item B) — the caller (`select_next`'s Guard A) treats
/// unavailable live state as its own reconciliation blocker rather than
/// ever conflating "unknown" with "confirmed no open PR".
fn load_live_prs(root: &Path, fixture: Option<PathBuf>) -> Result<(String, Vec<LiveOpenPr>, bool)> {
    if let Some(fixture_path) = fixture {
        let text = fs::read_to_string(&fixture_path)
            .with_context(|| format!("failed to read fixture {}", fixture_path.display()))?;
        let parsed: LivePrFixture = serde_json::from_str(&text)
            .with_context(|| format!("failed to parse fixture {}", fixture_path.display()))?;
        return Ok((parsed.repository.unwrap_or_else(|| "fixture".to_owned()), parsed.prs, true));
    }

    let repo_output = Command::new("gh")
        .current_dir(root)
        .args(["repo", "view", "--json", "nameWithOwner", "--jq", ".nameWithOwner"])
        .output()
        .context("failed to execute gh repo view")?;
    let repository = if repo_output.status.success() {
        String::from_utf8_lossy(&repo_output.stdout).trim().to_string()
    } else {
        "unknown".to_string()
    };

    let output = Command::new("gh")
        .current_dir(root)
        .args([
            "pr",
            "list",
            "--state",
            "open",
            "--limit",
            "200",
            "--json",
            "number,title,body,url,isDraft",
        ])
        .output()
        .context("failed to execute gh pr list")?;
    if !output.status.success() {
        return Ok((repository, Vec::new(), false));
    }
    let raw = String::from_utf8_lossy(&output.stdout);
    let prs: Vec<LiveOpenPr> =
        serde_json::from_str(&raw).context("failed to parse gh pr list output")?;
    Ok((repository, prs, true))
}

fn load_candidates_for_program(
    root: &Path,
    program_id: &str,
    snapshot: &mut SelectionSnapshot,
) -> Result<()> {
    let manifest_path = manifest::program_manifest_path(program_id);
    let full_path = root.join(&manifest_path);
    if !full_path.exists() {
        bail!("program manifest not found: {manifest_path}");
    }
    let text = fs::read_to_string(&full_path)
        .with_context(|| format!("failed to read {manifest_path}"))?;

    if manifest::is_milestone_ledger(&text) {
        load_milestone_candidates(root, &manifest_path, snapshot)
    } else {
        load_lane_routing_candidates(&text, &manifest_path, snapshot)
    }
}

fn load_milestone_candidates(
    root: &Path,
    manifest_path: &str,
    snapshot: &mut SelectionSnapshot,
) -> Result<()> {
    let ledger = manifest::load_milestone_ledger(root, manifest_path)?;
    // Structural validation (status/dependency/cycle checks) must gate
    // selection the same way it gates `check-active-goal-manifest` — this
    // is the ONE typed loader shared by both so `goals next` can never
    // select from a ledger the validator would have rejected.
    let violations = manifest::validate_milestone_ledger(&ledger);
    if !violations.is_empty() {
        bail!("{manifest_path}: invalid milestone ledger:\n{}", violations.join("\n"));
    }
    snapshot.program_title = ledger.title.clone();
    snapshot.tracker_issue = ledger.tracker_issue;
    snapshot.non_goals = ledger.non_goals;
    snapshot.candidates = ledger
        .milestones
        .into_iter()
        .map(|m| MilestoneCandidate {
            id: m.id,
            title: m.title,
            status: parse_status(&m.status),
            issue: m.issue,
            depends_on: m.depends_on,
            exit_criteria: m.exit_criteria,
            lane: None,
            claim_boundary: None,
            ownership: Vec::new(),
            required_proof: Vec::new(),
        })
        .collect();
    Ok(())
}

/// Lane-routing program shape (e.g. `real_perl_editor_trust`): each
/// `[[work_item]]` becomes a candidate. Slug ids do not map to a live
/// GitHub issue number in schema 2 (a known gap — see the M3 design
/// background at `scratchpad/m3-selector-design.md` §1) so `issue` is left
/// `None`; `--fixture`/live PR matching for this program shape is
/// therefore a no-op until manifests carry issue numbers.
fn load_lane_routing_candidates(
    text: &str,
    manifest_path: &str,
    snapshot: &mut SelectionSnapshot,
) -> Result<()> {
    let value: toml::Value =
        toml::from_str(text).with_context(|| format!("failed to parse {manifest_path}"))?;
    let Some(table) = value.as_table() else {
        bail!("{manifest_path}: expected TOML table");
    };

    if let Some(claim_boundaries) = table.get("claim_boundaries").and_then(|v| v.as_array()) {
        snapshot.non_goals =
            claim_boundaries.iter().filter_map(|v| v.as_str().map(ToString::to_string)).collect();
    }
    if let Some(board) = table.get("status_pointer").and_then(|v| v.as_str()) {
        snapshot.board = Some(board.to_owned());
    }

    let Some(items) = table.get("work_item").and_then(|v| v.as_array()) else {
        return Ok(());
    };

    snapshot.candidates = items
        .iter()
        .filter_map(|item| item.as_table())
        .map(|item| {
            let id = item.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_owned();
            let status_raw = item.get("status").and_then(|v| v.as_str()).unwrap_or_default();
            let claim_boundary =
                item.get("claim_boundary").and_then(|v| v.as_str()).map(ToString::to_string);
            MilestoneCandidate {
                title: id.clone(),
                id,
                status: parse_status(status_raw),
                issue: None,
                depends_on: Vec::new(),
                exit_criteria: claim_boundary.clone().unwrap_or_default(),
                lane: item.get("lane").and_then(|v| v.as_str()).map(ToString::to_string),
                claim_boundary,
                ownership: item
                    .get("files")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter().filter_map(|v| v.as_str().map(ToString::to_string)).collect()
                    })
                    .unwrap_or_default(),
                required_proof: item
                    .get("commands")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter().filter_map(|v| v.as_str().map(ToString::to_string)).collect()
                    })
                    .unwrap_or_default(),
            }
        })
        .collect();

    Ok(())
}

/// `cargo xtask goals reconcile` (#3696 item B): diagnoses milestones whose
/// self-reported ledger `status` may have drifted from live GitHub reality
/// (a merged-not-open PR for an in-progress milestone) or that lack the
/// identity `select_next`'s Guard A needs. This is a SOFT, advisory
/// report — findings never mutate the ledger or any PR, and are
/// deliberately NOT folded into `manifest::validate_milestone_ledger`'s
/// hard violations, so `check-active-goal-manifest` is never red-CI'd by a
/// finding that isn't currently blocking selection.
pub fn build_reconciliation_report(
    program: Option<String>,
    fixture: Option<PathBuf>,
) -> Result<Vec<ReconciliationFinding>> {
    build_reconciliation_report_at(&project_root()?, program, fixture)
}

fn build_reconciliation_report_at(
    root: &Path,
    program: Option<String>,
    fixture: Option<PathBuf>,
) -> Result<Vec<ReconciliationFinding>> {
    let snapshot = build_snapshot_at(root, program, fixture.clone())?;
    let Some(_resolved_program) = snapshot.resolved_program.as_deref() else {
        return Ok(vec![ReconciliationFinding {
            milestone_id: "<program>".to_owned(),
            issue: None,
            kind: "ambiguous_program_authority".to_owned(),
            detail: format!(
                "no program resolved; reconciliation cannot inspect candidates ({})",
                ambiguity_detail(&snapshot)
            ),
            pr_number: None,
            pr_url: None,
        }]);
    };
    let merged_prs =
        load_merged_prs_for_candidates(root, &snapshot.candidates, fixture.as_deref())?;
    Ok(reconcile_in_progress(
        &snapshot.candidates,
        &snapshot.live_open_prs,
        &merged_prs,
        &snapshot.repository,
        snapshot.live_prs_available,
    ))
}

/// Fetches MERGED PRs referencing any in-progress candidate's issue — the
/// evidence `reconcile_in_progress`'s `merged_pr_but_still_in_progress`
/// finding needs and `select_next`'s open-PR-only Guard A never looks at.
/// `--fixture` (same JSON shape `load_live_prs` reads, plus `merged_prs`)
/// replaces the live search, exactly like `load_live_prs`'s `--fixture`
/// contract.
fn load_merged_prs_for_candidates(
    root: &Path,
    candidates: &[MilestoneCandidate],
    fixture: Option<&Path>,
) -> Result<Vec<LiveOpenPr>> {
    if let Some(fixture_path) = fixture {
        let text = fs::read_to_string(fixture_path)
            .with_context(|| format!("failed to read fixture {}", fixture_path.display()))?;
        let parsed: LivePrFixture = serde_json::from_str(&text)
            .with_context(|| format!("failed to parse fixture {}", fixture_path.display()))?;
        return Ok(parsed.merged_prs);
    }

    let mut merged = Vec::new();
    let mut seen_pr_numbers = std::collections::BTreeSet::new();
    for issue in candidates
        .iter()
        .filter(|c| c.status == MilestoneStatus::InProgress)
        .filter_map(|c| c.issue)
    {
        let output = Command::new("gh")
            .current_dir(root)
            .args([
                "pr",
                "list",
                "--state",
                "merged",
                "--search",
                &format!("{issue} in:body,title"),
                "--limit",
                "200",
                "--json",
                "number,title,body,url,isDraft",
            ])
            .output()
            .context("failed to execute gh pr list --state merged")?;
        if !output.status.success() {
            bail!(
                "gh pr list --state merged failed for issue #{issue}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        let raw = String::from_utf8_lossy(&output.stdout);
        let prs: Vec<LiveOpenPr> = serde_json::from_str(&raw)
            .context("failed to parse gh pr list --state merged output")?;
        for pr in prs {
            if seen_pr_numbers.insert(pr.number) {
                merged.push(pr);
            }
        }
    }
    Ok(merged)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_replaces_live_gh_call() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let fixture_path = temp.path().join("prs.json");
        fs::write(
            &fixture_path,
            r#"{"repository":"EffortlessMetrics/perl-lsp-swarm","prs":[{"number":1,"title":"t","body":"b","url":"u","isDraft":true}]}"#,
        )?;

        let (repository, prs, available) = load_live_prs(&project_root()?, Some(fixture_path))?;

        assert_eq!(repository, "EffortlessMetrics/perl-lsp-swarm");
        assert_eq!(prs.len(), 1);
        assert_eq!(prs[0].number, 1);
        assert!(prs[0].is_draft);
        assert!(available, "a fixture is always deterministic/available");
        Ok(())
    }

    // Regression coverage for #3692 defect 6 (factory-droid review thread,
    // PR #3701): `current_git_ref` was previously only exercised
    // indirectly via `inputs_used_reflects_the_actual_current_git_ref_not_a_hardcoded_literal`
    // in `select.rs`, which starts from an already-populated
    // `SelectionSnapshot` and never calls this function at all. Pin its
    // three branches directly against throwaway repos so a regression in
    // the fallback chain (git unavailable/failing -> "unknown"; detached
    // HEAD -> "detached@<sha>", never the literal "HEAD") is caught here
    // rather than only showing up as a confusing provenance string deep
    // in a JSON receipt.

    #[test]
    fn current_git_ref_returns_unknown_outside_a_git_repo() -> Result<()> {
        // Both `git rev-parse` invocations fail (not a git repo at all),
        // so the function must fall back to "unknown" rather than
        // propagating an `Err` — this value is best-effort provenance
        // metadata, never load-bearing for selection correctness.
        let temp = tempfile::tempdir()?;

        assert_eq!(current_git_ref(temp.path()), "unknown");
        Ok(())
    }

    #[test]
    fn current_git_ref_names_the_actual_branch_in_a_real_repo() -> Result<()> {
        // Pinned against a throwaway repo (rather than this crate's own
        // checkout, whose branch/detached state varies across local runs
        // and CI's frequently-detached PR checkouts) so the assertion is
        // deterministic.
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        init_throwaway_repo(root)?;

        assert_eq!(current_git_ref(root), "trunk");
        Ok(())
    }

    #[test]
    fn current_git_ref_names_the_short_sha_when_detached() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        init_throwaway_repo(root)?;
        run_git(root, &["checkout", "-q", "--detach", "HEAD"])?;

        let result = current_git_ref(root);
        assert!(
            result.starts_with("detached@") && result.len() > "detached@".len(),
            "expected \"detached@<sha>\", got {result:?}"
        );
        Ok(())
    }

    /// Runs a `git` subcommand in `root`, failing the test (via `Err`) if
    /// it exits nonzero.
    fn run_git(root: &Path, args: &[&str]) -> Result<()> {
        let status = std::process::Command::new("git").args(args).current_dir(root).status()?;
        if !status.success() {
            bail!("git {args:?} failed with {:?}", status.code());
        }
        Ok(())
    }

    /// Initializes a minimal one-commit repo on branch `trunk` in `root`,
    /// for `current_git_ref` tests that need a real (but throwaway) git
    /// history rather than this crate's own checkout.
    fn init_throwaway_repo(root: &Path) -> Result<()> {
        run_git(root, &["init", "-q", "-b", "trunk"])?;
        run_git(root, &["config", "user.email", "test@example.com"])?;
        run_git(root, &["config", "user.name", "Test"])?;
        fs::write(root.join("f.txt"), "x")?;
        run_git(root, &["add", "."])?;
        run_git(root, &["commit", "-q", "-m", "init"])?;
        Ok(())
    }

    #[test]
    fn lane_routing_candidates_leave_issue_unresolved() -> Result<()> {
        let text = r#"
claim_boundaries = ["Do not broaden."]
status_pointer = "docs/board.md"

[[work_item]]
id = "wi-1"
status = "active"
lane = "trust"
claim_boundary = "boundary text"
files = ["docs/board.md"]
commands = ["rtk cargo test"]
"#;
        let mut snapshot = SelectionSnapshot {
            repository: "r".to_owned(),
            requested_program: None,
            default_program: None,
            known_programs: Vec::new(),
            resolved_program: Some("p".to_owned()),
            mode: "maintainer".to_owned(),
            board: None,
            program_title: None,
            tracker_issue: None,
            non_goals: Vec::new(),
            candidates: Vec::new(),
            live_open_prs: Vec::new(),
            current_git_ref: "main".to_owned(),
            live_prs_available: true,
        };

        load_lane_routing_candidates(text, "programs/p.toml", &mut snapshot)?;

        assert_eq!(snapshot.candidates.len(), 1);
        assert_eq!(snapshot.candidates[0].issue, None);
        assert_eq!(snapshot.board.as_deref(), Some("docs/board.md"));
        assert_eq!(snapshot.non_goals, vec!["Do not broaden.".to_owned()]);
        Ok(())
    }

    fn blank_snapshot(resolved_program: &str) -> SelectionSnapshot {
        SelectionSnapshot {
            repository: "r".to_owned(),
            requested_program: None,
            default_program: None,
            known_programs: Vec::new(),
            resolved_program: Some(resolved_program.to_owned()),
            mode: "maintainer".to_owned(),
            board: None,
            program_title: None,
            tracker_issue: None,
            non_goals: Vec::new(),
            candidates: Vec::new(),
            live_open_prs: Vec::new(),
            current_git_ref: "main".to_owned(),
            live_prs_available: true,
        }
    }

    // Legacy default-program regressions remain covered by explicit-id tests:
    // covered by #3697's `resolve_program_rejects_an_unvalidated_default_program`
    // and `unvalidated_default_program_blocks_selection_with_ambiguous_program_authority`
    // tests below (landed on main in a7eccc885 before this branch was
    // rebased) — that PR's shared `manifest::validate_program_id` fully
    // subsumes the fail-closed filtering this PR originally added here
    // with a different `resolve_program` signature; dropped as a
    // duplicate rather than re-litigated. See PR #3701's reconciliation
    // note.

    #[test]
    fn unknown_requested_program_resolves_to_none_not_an_error() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let fixture_path = temp.path().join("prs.json");
        fs::write(&fixture_path, r#"{"repository":"r","prs":[]}"#)?;

        let snapshot = build_snapshot(
            Some("definitely-not-a-real-program-xyz".to_owned()),
            Some(fixture_path),
        )?;

        assert_eq!(
            snapshot.requested_program.as_deref(),
            Some("definitely-not-a-real-program-xyz")
        );
        assert_eq!(snapshot.resolved_program, None);
        assert!(snapshot.candidates.is_empty());
        Ok(())
    }

    #[test]
    fn disabled_portfolio_program_cannot_be_selected_explicitly() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let goals = temp.path().join(".perl-lsp/goals/programs");
        fs::create_dir_all(&goals)?;
        fs::write(goals.join("disabled.toml"), "id = \"disabled\"\n")?;
        fs::write(
            temp.path().join(".perl-lsp/goals/active.toml"),
            "schema = 3\n\n[[program]]\nid = \"disabled\"\nenabled = false\nmanifest = \".perl-lsp/goals/programs/disabled.toml\"\nkind = \"milestone_ledger\"\n",
        )?;
        let fixture_path = temp.path().join("prs.json");
        fs::write(&fixture_path, r#"{"repository":"r","prs":[]}"#)?;

        let snapshot =
            build_snapshot_at(temp.path(), Some("disabled".to_owned()), Some(fixture_path))?;

        assert_eq!(snapshot.requested_program.as_deref(), Some("disabled"));
        assert_eq!(snapshot.resolved_program, None);
        assert!(snapshot.candidates.is_empty());
        Ok(())
    }

    #[test]
    fn malformed_portfolio_program_shape_is_an_error() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let goals = temp.path().join(".perl-lsp/goals/programs");
        fs::create_dir_all(&goals)?;
        fs::write(goals.join("p.toml"), "id = \"p\"\n")?;
        fs::write(
            temp.path().join(".perl-lsp/goals/active.toml"),
            "schema = 3\nprogram = \"not-an-array\"\n",
        )?;
        let fixture_path = temp.path().join("prs.json");
        fs::write(&fixture_path, r#"{"repository":"r","prs":[]}"#)?;

        let error = build_snapshot_at(temp.path(), Some("p".to_owned()), Some(fixture_path))
            .expect_err("malformed schema-3 program structure must not look disabled");
        assert!(error.to_string().contains("must contain a [[program]] array"));
        Ok(())
    }

    #[test]
    fn portfolio_does_not_use_legacy_default_program_as_selection_authority() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let goals = temp.path().join(".perl-lsp/goals/programs");
        fs::create_dir_all(&goals)?;
        fs::write(goals.join("known.toml"), "id = \"known\"\n")?;
        fs::write(
            temp.path().join(".perl-lsp/goals/active.toml"),
            "schema = 2\nactive_program = \"known\"\ndefault_program = \"known\"\n",
        )?;
        let fixture_path = temp.path().join("prs.json");
        fs::write(&fixture_path, r#"{"repository":"r","prs":[]}"#)?;

        let snapshot = build_snapshot_at(temp.path(), None, Some(fixture_path))?;

        assert_eq!(snapshot.default_program, None);
        assert_eq!(snapshot.resolved_program, None);
        assert_eq!(snapshot.requested_program, None);
        Ok(())
    }

    #[test]
    fn resolve_program_rejects_an_unvalidated_default_program() -> Result<()> {
        // #3647 follow-up: `default_program` used to be assigned to
        // `resolved_program` straight from `active.toml` with none of the
        // known-programs/bare-id validation the explicit `--program` path
        // already applied — fail-open on a control-plane work-routing
        // authority. `resolve_program` must now reject an unknown id AND a
        // path-traversal-shaped one, exactly like `--program` does.
        let temp = tempfile::tempdir()?;
        let programs_dir = temp.path().join(".perl-lsp/goals/programs");
        fs::create_dir_all(&programs_dir)?;
        fs::write(programs_dir.join("known.toml"), "id = \"known\"\n")?;

        for bad_default in ["unknown-program-xyz", "../../../etc/passwd", "sub/dir"] {
            assert_eq!(
                resolve_program(temp.path(), None, Some(bad_default)),
                None,
                "default_program {bad_default:?} must not resolve"
            );
        }

        // A valid, known default_program still resolves normally.
        assert_eq!(resolve_program(temp.path(), None, Some("known")), Some("known".to_owned()));
        Ok(())
    }

    #[test]
    fn unvalidated_default_program_blocks_selection_with_ambiguous_program_authority() -> Result<()>
    {
        // Proves the fail-closed outcome end to end: an unresolved
        // `default_program` must reach `select_next` as
        // `Blocked(ambiguous_program_authority)` — never a panic, and
        // never a silently-selected candidate.
        use super::super::select::{SelectionDecision, select_next};

        let temp = tempfile::tempdir()?;
        let programs_dir = temp.path().join(".perl-lsp/goals/programs");
        fs::create_dir_all(&programs_dir)?;
        fs::write(programs_dir.join("known.toml"), "id = \"known\"\n")?;

        let mut snapshot = blank_snapshot("known");
        snapshot.resolved_program = resolve_program(temp.path(), None, Some("../../../etc/passwd"));

        match select_next(&snapshot) {
            SelectionDecision::Blocked(blockers) => {
                assert_eq!(blockers[0].kind, "ambiguous_program_authority");
            }
            other => bail!("expected Blocked(ambiguous_program_authority), got {other:?}"),
        }
        Ok(())
    }

    #[test]
    fn load_candidates_for_program_classifies_by_parsed_toml_not_substring() -> Result<()> {
        // A lane-routing manifest that merely *mentions* "[[milestone]]" in
        // a comment must still be parsed as lane-routing, not misclassified
        // as a (structurally empty, therefore invalid) milestone ledger.
        let temp = tempfile::tempdir()?;
        let programs_dir = temp.path().join(".perl-lsp/goals/programs");
        fs::create_dir_all(&programs_dir)?;
        fs::write(
            programs_dir.join("p.toml"),
            "# this program intentionally does not use [[milestone]] entries\n\n[[work_item]]\nid = \"wi-1\"\nstatus = \"active\"\n",
        )?;
        let mut snapshot = blank_snapshot("p");

        load_candidates_for_program(temp.path(), "p", &mut snapshot)?;

        assert_eq!(snapshot.candidates.len(), 1);
        assert_eq!(snapshot.candidates[0].id, "wi-1");
        Ok(())
    }

    #[test]
    fn load_candidates_for_program_rejects_an_invalid_milestone_ledger() -> Result<()> {
        // The shared `validate_milestone_ledger` checks (status vocabulary,
        // dangling depends_on, cycles, and the in-progress-requires-issue
        // rule) must gate `goals next` selection, not just
        // `check-active-goal-manifest` — otherwise the two can drift on
        // what a valid ledger looks like.
        let temp = tempfile::tempdir()?;
        let programs_dir = temp.path().join(".perl-lsp/goals/programs");
        fs::create_dir_all(&programs_dir)?;
        fs::write(
            programs_dir.join("p.toml"),
            "id = \"p\"\ntitle = \"t\"\n\n[[milestone]]\nid = \"M1\"\ntitle = \"one\"\nstatus = \"in_progress\"\nexit_criteria = \"x\"\n",
        )?;
        let mut snapshot = blank_snapshot("p");

        let err = load_candidates_for_program(temp.path(), "p", &mut snapshot)
            .expect_err("in_progress milestone with no issue must fail validation");
        assert!(
            err.to_string().contains("issue number"),
            "expected an issue-number violation in the error, got {err}"
        );
        Ok(())
    }

    #[test]
    fn reconcile_flags_merged_pr_for_in_progress_milestone() -> Result<()> {
        // The #3696 item B incident regression fixture. M3 in_progress
        // (#3624) has NO open PR but a MERGED one —
        // `build_reconciliation_report` must flag it as
        // `merged_pr_but_still_in_progress`, naming the merged PR. M4 is a
        // deliberately identity-less Pending sibling, exercising the soft
        // `pending_without_identity` finding in the same pass.
        let temp = tempfile::tempdir()?;
        let programs_dir = temp.path().join(".perl-lsp/goals/programs");
        fs::create_dir_all(&programs_dir)?;
        fs::write(temp.path().join(".perl-lsp/goals/active.toml"), "active_program = \"p\"\n")?;
        fs::write(
            programs_dir.join("p.toml"),
            r#"
id = "p"
title = "t"

[[milestone]]
id = "M3"
title = "three"
status = "in_progress"
issue = 3624
depends_on = []
exit_criteria = "x"

[[milestone]]
id = "M4"
title = "four"
status = "pending"
depends_on = ["M3"]
exit_criteria = "y"
"#,
        )?;

        let fixture_path = temp.path().join("prs.json");
        fs::write(
            &fixture_path,
            r#"{"repository":"r","prs":[],"merged_prs":[{"number":4242,"title":"feat: M3 (#3624)","body":"","url":"https://example.invalid/pull/4242","isDraft":false}]}"#,
        )?;

        let findings =
            build_reconciliation_report_at(temp.path(), Some("p".to_owned()), Some(fixture_path))?;

        let merged_findings: Vec<_> =
            findings.iter().filter(|f| f.kind == "merged_pr_but_still_in_progress").collect();
        assert_eq!(
            merged_findings.len(),
            1,
            "expected exactly one merged-PR finding, got {findings:?}"
        );
        assert_eq!(merged_findings[0].milestone_id, "M3");
        assert_eq!(merged_findings[0].pr_number, Some(4242));
        assert!(merged_findings[0].detail.contains("4242"));

        assert!(
            findings.iter().any(|f| f.milestone_id == "M4" && f.kind == "pending_without_identity"),
            "expected a soft pending_without_identity finding for M4, got {findings:?}"
        );
        Ok(())
    }

    #[test]
    fn reconcile_reports_unresolved_program_instead_of_no_findings() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let programs_dir = temp.path().join(".perl-lsp/goals/programs");
        fs::create_dir_all(&programs_dir)?;
        fs::write(
            temp.path().join(".perl-lsp/goals/active.toml"),
            "schema = 3\nmode = \"portfolio\"\n\n[[program]]\nid = \"p\"\nenabled = false\n",
        )?;
        fs::write(programs_dir.join("p.toml"), "id = \"p\"\ntitle = \"t\"\n")?;

        let fixture_path = temp.path().join("prs.json");
        fs::write(&fixture_path, r#"{"repository":"r","prs":[]}"#)?;

        let findings = build_reconciliation_report_at(temp.path(), None, Some(fixture_path))?;

        assert_eq!(findings.len(), 1, "expected an authority finding, got {findings:?}");
        assert_eq!(findings[0].kind, "ambiguous_program_authority");
        assert!(findings[0].detail.contains("no program resolved"));
        Ok(())
    }
}
