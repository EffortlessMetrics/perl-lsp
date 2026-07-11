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

use super::manifest;
use super::select::{
    LiveOpenPr, MilestoneCandidate, ProgramCandidate, SelectionSnapshot, parse_status,
};

#[derive(Debug, Default, Deserialize)]
struct LivePrFixture {
    #[serde(default)]
    repository: Option<String>,
    #[serde(default)]
    prs: Vec<LiveOpenPr>,
}

pub fn build_snapshot(
    program_arg: Option<String>,
    fixture: Option<PathBuf>,
) -> Result<SelectionSnapshot> {
    let root = project_root()?;

    let pointer = manifest::load_active_pointer(&root)?;
    let default_program = pointer
        .default_program
        .clone()
        .or_else(|| (!pointer.active_program.is_empty()).then(|| pointer.active_program.clone()));

    let known_programs = discover_known_programs(&root)?;

    // Fail closed rather than erroring: an explicitly requested program
    // that isn't discoverable under `.perl-lsp/goals/programs/` leaves
    // `resolved_program` unset so `select_next` reports the structured
    // `ambiguous_program_authority` blocker (with `requested_program`
    // naming what was asked for and `known_programs` listing valid ids)
    // instead of `goals next --json` exiting with a non-JSON error —
    // `--json` callers must always get parseable output.
    //
    // The `default_program` arm used to assign it straight from
    // `active.toml` with no validation at all (#3647 follow-up finding,
    // #3696/#3697): an unknown, malformed, or path-traversal-shaped
    // `default_program` reached `resolved_program` unchecked whenever no
    // explicit `--program` was given — fail-OPEN on a control-plane
    // work-routing authority (this was also #3692 defect 5). This
    // `resolve_program` now runs BOTH sources through the same
    // `manifest::validate_program_id` check the static
    // `active_goal_manifest::validate_default_program` validator uses, so
    // the two can never drift or fail open again.
    let resolved_program =
        resolve_program(&root, program_arg.as_deref(), default_program.as_deref());

    let (repository, live_open_prs) = load_live_prs(&root, fixture)?;
    let current_git_ref = current_git_ref(&root);

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
    };

    if let Some(program_id) = &resolved_program {
        load_candidates_for_program(&root, program_id, &mut snapshot)?;
    }

    Ok(snapshot)
}

/// Resolves which program `goals next` selects against: an explicit
/// `--program` wins when given (even if invalid — it never falls back to
/// `default_program`, matching the pre-existing `--program` contract),
/// otherwise `active.toml`'s `default_program`. Either source must pass
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

fn load_live_prs(root: &Path, fixture: Option<PathBuf>) -> Result<(String, Vec<LiveOpenPr>)> {
    if let Some(fixture_path) = fixture {
        let text = fs::read_to_string(&fixture_path)
            .with_context(|| format!("failed to read fixture {}", fixture_path.display()))?;
        let parsed: LivePrFixture = serde_json::from_str(&text)
            .with_context(|| format!("failed to parse fixture {}", fixture_path.display()))?;
        return Ok((parsed.repository.unwrap_or_else(|| "fixture".to_owned()), parsed.prs));
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
        bail!("gh pr list failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    let raw = String::from_utf8_lossy(&output.stdout);
    let prs: Vec<LiveOpenPr> =
        serde_json::from_str(&raw).context("failed to parse gh pr list output")?;
    Ok((repository, prs))
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

        let (repository, prs) = load_live_prs(&project_root()?, Some(fixture_path))?;

        assert_eq!(repository, "EffortlessMetrics/perl-lsp-swarm");
        assert_eq!(prs.len(), 1);
        assert_eq!(prs[0].number, 1);
        assert!(prs[0].is_draft);
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
        }
    }

    // #3692 defect 5 ("stale default_program -> non-JSON error") is now
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
}
