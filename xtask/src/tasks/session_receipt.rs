//! `cargo xtask session-receipt` — M5 phase 4 (#3777): a machine-produced
//! session-start receipt capturing checkout identity (repo/branch/SHA
//! relative to `origin/main`) plus an advisory staleness liveness check.
//!
//! Motivation: this session hit the stale-source trap TWICE — a local
//! checkout tens of commits behind `origin/main` produced a false
//! `unrecognized subcommand 'goals'` because the local `xtask` binary
//! predated the `goals` subcommand entirely. Building/reasoning from a
//! stale checkout silently produces wrong answers. This command makes the
//! checkout's freshness a machine-checkable fact instead of something an
//! agent has to remember to check.
//!
//! Scope boundary (see #3777 phase 4 claim boundary): this is a
//! **receipt + advisory liveness check**, not a build gate. It always
//! exits `0` — a stale checkout prints a `WARNING`, never a hard failure.
//! Build-lease enforcement (M5 phase 3) and stage-boundary checks (M5
//! phase 5) are separate, later deliverables.
//!
//! Read-only except for `git fetch origin main` (a read from the remote)
//! and writing the receipt JSON to `--out`: never creates/mutates a
//! branch, worktree, PR, or ledger.
//!
//! **Uniform fail-closed contract**: every field this receipt cannot
//! actually verify reports `null`/`None`, never a value that LOOKS like a
//! confirmed fact (a false "clean", a mismatched repo, an unvalidated
//! program id). A receipt that silently reports a wrong default when it
//! couldn't check is worse than one that says "unknown" -- that's the
//! entire point of a *trustworthy* machine receipt (post-review
//! hardening, deep-review + factory-droid P3 findings on PR #3866).

use crate::utils::project_root;
use color_eyre::eyre::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Advisory default threshold (commits behind `origin/main`) before the
/// staleness WARNING fires. Chosen from the concrete incident this
/// command guards against: the stale-source trap hit this session was
/// ~48 commits behind. A handful of commits behind is normal for a
/// long-lived worktree mid-task; double digits is the danger zone this
/// check exists to surface. Advisory only — never a hard gate.
pub const DEFAULT_WARN_THRESHOLD: u32 = 5;

pub const RECEIPT_KIND: &str = "session_start";
pub const SCHEMA_VERSION: u32 = 1;
pub const DEFAULT_OUT_PATH: &str = "target/receipts/session-start.json";

/// Machine-readable session-start receipt. Field shape mirrors
/// `.ci/receipts/schemas/session-start.schema.json` — keep them in sync
/// (a unit test below cross-checks this struct's serialized keys against
/// the schema's `required`/`properties`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionReceipt {
    pub schema_version: u32,
    pub kind: String,
    pub captured_at: String,
    pub timestamp_source: String,
    /// `owner/repo`, derived from `git config --get remote.origin.url`
    /// (the SAME source `git fetch origin` resolves against) -- never
    /// from `gh`'s auth context, which can silently name a DIFFERENT repo
    /// than `origin` actually points to. `None` when the remote is
    /// missing or unparseable (fail-closed, not a guess).
    pub repo: Option<String>,
    pub branch: String,
    pub head_sha: String,
    /// Whether `git fetch origin main` succeeded. When `false`,
    /// `origin_main_sha`/`behind_origin_main`/`ahead` are all `None` --
    /// fail-CLOSED on staleness rather than ever silently reporting
    /// 0-behind when the comparison is actually unavailable.
    pub fetch_ok: bool,
    pub origin_main_sha: Option<String>,
    pub behind_origin_main: Option<u32>,
    pub ahead: Option<u32>,
    /// `None` when `git status --porcelain` itself failed to run --
    /// fail-CLOSED rather than ever reporting a false "clean". See
    /// `dirty_check_ok` for the underlying success flag.
    pub dirty: Option<bool>,
    /// Whether the `git status --porcelain` call that produced `dirty`
    /// succeeded. `dirty` is only meaningful when this is `true`.
    pub dirty_check_ok: bool,
    pub toolchain_version: String,
    /// Resolved program id, validated via
    /// `goals::manifest::validate_program_id` (bare id, no path
    /// separators/`..`, and a manifest that actually exists) -- an
    /// invalid/garbage `default_program` in `active.toml` never passes
    /// through unfiltered. `None` when unresolved or invalid; see
    /// `program_note` for why.
    pub program: Option<String>,
    pub lane: Option<String>,
    /// Populated when `program` is `None` because the candidate id
    /// failed validation (as opposed to simply being absent).
    pub program_note: Option<String>,
    /// The threshold that was in effect when `stale_warning` was computed
    /// (provenance for the advisory decision).
    pub warn_threshold: u32,
    /// Populated only when `behind_origin_main` exceeds `warn_threshold`.
    pub stale_warning: Option<String>,
}

/// `cargo xtask session-receipt` entry point. Never returns an `Err` for
/// staleness itself (advisory, not a gate) -- only for genuine I/O
/// failure building the receipt (e.g. the receipt directory cannot be
/// created).
pub fn run(
    json: bool,
    program: Option<String>,
    lane: Option<String>,
    out: Option<PathBuf>,
    warn_threshold: u32,
) -> Result<()> {
    let root = project_root()?;
    let receipt = build_receipt(&root, program, lane, warn_threshold)?;

    let out_path = out.unwrap_or_else(|| root.join(DEFAULT_OUT_PATH));
    write_receipt(&out_path, &receipt)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&receipt)?);
    } else {
        println!("{}", render_human(&receipt));
    }

    // Always print advisory signals to stderr, independent of --json, so
    // they are never buried inside a JSON blob a downstream parser might
    // discard -- visibility is the entire point of this advisory check.
    if let Some(warning) = &receipt.stale_warning {
        eprintln!("WARNING: {warning}");
    } else if !receipt.fetch_ok {
        eprintln!(
            "WARNING: could not fetch origin/main -- staleness relative to origin is UNKNOWN (fail-closed, #3777)."
        );
    }
    if !receipt.dirty_check_ok {
        eprintln!(
            "WARNING: could not run `git status --porcelain` -- dirty-state is UNKNOWN (fail-closed, #3777)."
        );
    }
    if receipt.repo.is_none() {
        eprintln!(
            "WARNING: could not resolve repo from `git config --get remote.origin.url` -- repo identity is UNKNOWN (fail-closed, #3777)."
        );
    }

    Ok(())
}

fn write_receipt(out_path: &Path, receipt: &SessionReceipt) -> Result<()> {
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(out_path, serde_json::to_string_pretty(receipt)?)?;
    Ok(())
}

fn render_human(receipt: &SessionReceipt) -> String {
    let dirty_text = match (receipt.dirty, receipt.dirty_check_ok) {
        (Some(dirty), true) => dirty.to_string(),
        _ => "unknown (check failed)".to_owned(),
    };

    let mut lines = vec![
        format!("repo:             {}", receipt.repo.as_deref().unwrap_or("unknown")),
        format!("branch:           {}", receipt.branch),
        format!("head_sha:         {}", receipt.head_sha),
        format!("dirty:            {dirty_text}"),
        format!("toolchain:        {}", receipt.toolchain_version),
        format!("program:          {}", receipt.program.as_deref().unwrap_or("(none)")),
        format!("lane:             {}", receipt.lane.as_deref().unwrap_or("(none)")),
    ];

    if let Some(note) = &receipt.program_note {
        lines.push(format!("program_note:     {note}"));
    }

    if receipt.fetch_ok {
        lines.push(format!(
            "origin/main:      {} (behind={}, ahead={})",
            receipt.origin_main_sha.as_deref().unwrap_or("unknown"),
            receipt.behind_origin_main.map(|n| n.to_string()).unwrap_or_else(|| "?".to_owned()),
            receipt.ahead.map(|n| n.to_string()).unwrap_or_else(|| "?".to_owned()),
        ));
    } else {
        lines.push("origin/main:      unavailable (fetch failed)".to_owned());
    }

    lines.join("\n")
}

fn build_receipt(
    root: &Path,
    program_arg: Option<String>,
    lane_arg: Option<String>,
    warn_threshold: u32,
) -> Result<SessionReceipt> {
    let repo = repo_name(root);
    let branch = current_branch(root);
    let head_sha = git_output(root, &["rev-parse", "HEAD"]).unwrap_or_else(|| "unknown".to_owned());

    let status_output = git_output(root, &["status", "--porcelain"]);
    let (dirty, dirty_check_ok) = dirty_from_status(status_output.as_deref());

    let fetch_ok = fetch_origin_main(root);
    let (origin_main_sha, behind_origin_main, ahead) = if fetch_ok {
        let sha = git_output(root, &["rev-parse", "origin/main"]);
        let counts =
            git_output(root, &["rev-list", "--left-right", "--count", "origin/main...HEAD"]);
        let (behind, ahead) =
            counts.as_deref().map(parse_left_right_counts).unwrap_or((None, None));
        (sha, behind, ahead)
    } else {
        (None, None, None)
    };

    let toolchain_version = rustc_version().unwrap_or_else(|| "unknown".to_owned());

    let (program, program_note) = match program_arg {
        Some(explicit) => (Some(explicit), None),
        None => resolve_default_program(root),
    };
    let lane = lane_arg;

    // Distinguishes a session-start receipt captured under a CI runner
    // (whose ambient clock/env is externally controlled) from one
    // captured on an interactive developer/agent machine -- both use the
    // same `chrono::Utc::now()` call, but the provenance differs for
    // audit purposes.
    let timestamp_source =
        if std::env::var("CI").is_ok() { "ci_environment" } else { "system_clock" }.to_owned();

    let stale_warning = compute_stale_warning(behind_origin_main, warn_threshold);

    Ok(SessionReceipt {
        schema_version: SCHEMA_VERSION,
        kind: RECEIPT_KIND.to_owned(),
        captured_at: chrono::Utc::now().to_rfc3339(),
        timestamp_source,
        repo,
        branch,
        head_sha,
        fetch_ok,
        origin_main_sha,
        behind_origin_main,
        ahead,
        dirty,
        dirty_check_ok,
        toolchain_version,
        program,
        lane,
        program_note,
        warn_threshold,
        stale_warning,
    })
}

/// Pure: the advisory WARNING message, or `None` when the checkout isn't
/// stale enough to warrant one (or the comparison is unavailable).
/// `> threshold`, not `>=` -- exactly-at-threshold is still considered
/// within the normal range for a long-lived worktree.
fn compute_stale_warning(behind_origin_main: Option<u32>, threshold: u32) -> Option<String> {
    let behind = behind_origin_main?;
    if behind <= threshold {
        return None;
    }
    Some(format!(
        "local checkout is {behind} commits behind origin/main — build/run from a CURRENT \
         checkout (fresh worktree off origin/main) before trusting tooling output \
         (stale-source trap, #3777)."
    ))
}

/// Pure: derives `(dirty, dirty_check_ok)` from `git status --porcelain`
/// output. `None` input means the underlying `git status` command itself
/// failed to run/exit successfully -- fail-CLOSED to `(None, false)`
/// rather than ever inferring a false "clean" from a command that never
/// actually reported anything (factory-droid P3 finding on PR #3866).
fn dirty_from_status(status_output: Option<&str>) -> (Option<bool>, bool) {
    match status_output {
        Some(text) => (Some(!text.trim().is_empty()), true),
        None => (None, false),
    }
}

/// Resolves the receipt's `program` the same way `goals::snapshot`'s
/// `resolve_program` does: `.perl-lsp/goals/active.toml`'s
/// `default_program` (falling back to `active_program` when unset), but
/// ALWAYS run through `goals::manifest::validate_program_id` -- a bare id
/// with no path separators/`..` whose manifest actually exists under
/// `.perl-lsp/goals/programs/`. An invalid/garbage `default_program`
/// (or a missing/unparseable pointer file) fails CLOSED to `(None,
/// Some(reason))`/`(None, None)` rather than ever passing through
/// unvalidated (deep-review finding on PR #3866: this previously matched
/// `goals::snapshot::build_snapshot_at`'s OLD unvalidated read, not its
/// current validated `resolve_program`).
fn resolve_default_program(root: &Path) -> (Option<String>, Option<String>) {
    let Ok(pointer) = crate::tasks::goals::manifest::load_active_pointer(root) else {
        return (None, None);
    };
    let candidate = pointer
        .default_program
        .clone()
        .or_else(|| (!pointer.active_program.is_empty()).then(|| pointer.active_program.clone()));

    let Some(candidate) = candidate else {
        return (None, None);
    };

    match crate::tasks::goals::manifest::validate_program_id(root, &candidate) {
        Ok(()) => (Some(candidate), None),
        Err(reason) => {
            (None, Some(format!("default_program {candidate:?} failed validation: {reason}")))
        }
    }
}

fn fetch_origin_main(root: &Path) -> bool {
    Command::new("git")
        .current_dir(root)
        .args(["fetch", "origin", "main"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn git_output(root: &Path, args: &[&str]) -> Option<String> {
    Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn current_branch(root: &Path) -> String {
    match git_output(root, &["rev-parse", "--abbrev-ref", "HEAD"]) {
        Some(name) if !name.is_empty() && name != "HEAD" => name,
        _ => match git_output(root, &["rev-parse", "--short", "HEAD"]) {
            Some(sha) if !sha.is_empty() => format!("detached@{sha}"),
            _ => "unknown".to_owned(),
        },
    }
}

/// Resolves `owner/repo` from `git config --get remote.origin.url` --
/// deliberately NOT `gh repo view`, whose answer depends on `gh`'s own
/// auth context and can name a DIFFERENT repository than `origin` (the
/// same remote `git fetch origin` reads) actually points to (deep-review
/// finding on PR #3866: `gh` reported `EffortlessMetrics/perl-lsp`
/// instead of `perl-lsp-swarm` when origin was misconfigured). `None`
/// when the remote is missing or its URL is unparseable -- fail-closed,
/// never a mismatched guess.
fn repo_name(root: &Path) -> Option<String> {
    let url = git_output(root, &["config", "--get", "remote.origin.url"])?;
    parse_repo_from_remote_url(&url)
}

/// Pure: parses `owner/repo` out of a git remote URL, handling both the
/// scp-like SSH form (`git@host:owner/repo.git`) and URL forms
/// (`https://host/owner/repo.git`, `ssh://git@host/owner/repo.git`).
/// Returns `None` for anything that doesn't resolve to a non-empty
/// `owner` and `repo` segment -- never a partial/garbage guess.
fn parse_repo_from_remote_url(url: &str) -> Option<String> {
    let trimmed = url.trim();

    let path = if let Some(scheme_idx) = trimmed.find("://") {
        // scheme://host/owner/repo(.git)?
        let rest = &trimmed[scheme_idx + 3..];
        let host_end = rest.find('/')?;
        rest[host_end + 1..].to_owned()
    } else if let Some(colon_idx) = trimmed.rfind(':') {
        // scp-like: git@host:owner/repo(.git)?
        trimmed[colon_idx + 1..].to_owned()
    } else {
        return None;
    };

    let path = path.trim_end_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path);

    let (owner, repo) = path.rsplit_once('/')?;
    if repo.is_empty() || owner.is_empty() {
        return None;
    }
    Some(format!("{owner}/{repo}"))
}

fn rustc_version() -> Option<String> {
    Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// Pure: parses `git rev-list --left-right --count A...B` output
/// (`"<left>\t<right>"`) into `(left, right)`. Returns `(None, None)` on
/// any unparseable input rather than erroring -- the caller already
/// treats an unavailable comparison as `(None, None)` via `fetch_ok`.
fn parse_left_right_counts(text: &str) -> (Option<u32>, Option<u32>) {
    let mut parts = text.split_whitespace();
    let left = parts.next().and_then(|part| part.parse::<u32>().ok());
    let right = parts.next().and_then(|part| part.parse::<u32>().ok());
    (left, right)
}

#[cfg(test)]
mod tests {
    use super::*;
    use color_eyre::eyre::eyre;
    use std::collections::BTreeSet;

    fn sample_receipt() -> SessionReceipt {
        SessionReceipt {
            schema_version: SCHEMA_VERSION,
            kind: RECEIPT_KIND.to_owned(),
            captured_at: "2026-07-11T00:00:00+00:00".to_owned(),
            timestamp_source: "system_clock".to_owned(),
            repo: Some("EffortlessMetrics/perl-lsp-swarm".to_owned()),
            branch: "impl/3777-session-receipt-m5-phase4".to_owned(),
            head_sha: "abc123".to_owned(),
            fetch_ok: true,
            origin_main_sha: Some("def456".to_owned()),
            behind_origin_main: Some(0),
            ahead: Some(1),
            dirty: Some(false),
            dirty_check_ok: true,
            toolchain_version: "rustc 1.93.1".to_owned(),
            program: Some("agent_loop_enablement".to_owned()),
            lane: None,
            program_note: None,
            warn_threshold: DEFAULT_WARN_THRESHOLD,
            stale_warning: None,
        }
    }

    #[test]
    fn parse_left_right_counts_reads_tab_separated_pair() {
        assert_eq!(parse_left_right_counts("3\t5"), (Some(3), Some(5)));
    }

    #[test]
    fn parse_left_right_counts_reads_space_separated_pair() {
        assert_eq!(parse_left_right_counts("48 2"), (Some(48), Some(2)));
    }

    #[test]
    fn parse_left_right_counts_handles_empty_input() {
        assert_eq!(parse_left_right_counts(""), (None, None));
    }

    #[test]
    fn parse_left_right_counts_handles_garbage_input() {
        assert_eq!(parse_left_right_counts("not-a-number"), (None, None));
    }

    #[test]
    fn compute_stale_warning_none_when_within_threshold() {
        assert_eq!(compute_stale_warning(Some(3), DEFAULT_WARN_THRESHOLD), None);
    }

    #[test]
    fn compute_stale_warning_none_at_exact_threshold() {
        // Boundary: exactly-at-threshold is still within the normal range,
        // not yet a WARNING (`>`, not `>=`).
        assert_eq!(
            compute_stale_warning(Some(DEFAULT_WARN_THRESHOLD), DEFAULT_WARN_THRESHOLD),
            None
        );
    }

    #[test]
    fn compute_stale_warning_fires_above_threshold() {
        let warning = compute_stale_warning(Some(48), DEFAULT_WARN_THRESHOLD);
        let warning = warning.unwrap_or_default();
        assert!(warning.contains("48 commits behind"), "got: {warning}");
        assert!(warning.contains("#3777"), "got: {warning}");
    }

    #[test]
    fn compute_stale_warning_none_when_comparison_unavailable() {
        // fetch_ok == false surfaces its own separate "unavailable"
        // message at the `run()` layer -- `stale_warning` itself stays
        // `None` when there is nothing concrete to warn about.
        assert_eq!(compute_stale_warning(None, DEFAULT_WARN_THRESHOLD), None);
    }

    #[test]
    fn dirty_from_status_reports_clean_when_output_is_empty() {
        assert_eq!(dirty_from_status(Some("")), (Some(false), true));
    }

    #[test]
    fn dirty_from_status_reports_dirty_when_output_is_nonempty() {
        assert_eq!(dirty_from_status(Some(" M src/main.rs\n")), (Some(true), true));
    }

    #[test]
    fn dirty_from_status_fails_closed_when_command_failed() {
        // The critical regression guard: a failed `git status` must NEVER
        // be reported as clean (`Some(false)`) -- it must be `None` with
        // `dirty_check_ok == false`.
        assert_eq!(dirty_from_status(None), (None, false));
    }

    #[test]
    fn parse_repo_from_remote_url_handles_ssh_scp_form() {
        assert_eq!(
            parse_repo_from_remote_url("git@github.com:EffortlessMetrics/perl-lsp-swarm.git"),
            Some("EffortlessMetrics/perl-lsp-swarm".to_owned())
        );
    }

    #[test]
    fn parse_repo_from_remote_url_handles_https_form_with_git_suffix() {
        assert_eq!(
            parse_repo_from_remote_url("https://github.com/EffortlessMetrics/perl-lsp-swarm.git"),
            Some("EffortlessMetrics/perl-lsp-swarm".to_owned())
        );
    }

    #[test]
    fn parse_repo_from_remote_url_handles_https_form_without_git_suffix() {
        assert_eq!(
            parse_repo_from_remote_url("https://github.com/EffortlessMetrics/perl-lsp-swarm"),
            Some("EffortlessMetrics/perl-lsp-swarm".to_owned())
        );
    }

    #[test]
    fn parse_repo_from_remote_url_handles_ssh_url_form() {
        assert_eq!(
            parse_repo_from_remote_url("ssh://git@github.com/EffortlessMetrics/perl-lsp-swarm.git"),
            Some("EffortlessMetrics/perl-lsp-swarm".to_owned())
        );
    }

    #[test]
    fn parse_repo_from_remote_url_fails_closed_on_garbage_input() {
        assert_eq!(parse_repo_from_remote_url("not a url"), None);
        assert_eq!(parse_repo_from_remote_url(""), None);
    }

    #[test]
    fn resolve_default_program_fails_closed_when_pointer_missing() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let (program, note) = resolve_default_program(temp.path());
        assert_eq!(program, None);
        assert_eq!(note, None);
        Ok(())
    }

    #[test]
    fn resolve_default_program_fails_closed_on_invalid_program_id() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let goals_dir = temp.path().join(".perl-lsp/goals");
        fs::create_dir_all(&goals_dir)?;
        fs::write(
            goals_dir.join("active.toml"),
            "active_program = \"\"\ndefault_program = \"../escape\"\n",
        )?;
        let (program, note) = resolve_default_program(temp.path());
        assert_eq!(program, None, "an invalid program id must never pass through unfiltered");
        assert!(note.is_some(), "an invalid candidate must leave a note explaining why");
        Ok(())
    }

    #[test]
    fn resolve_default_program_accepts_a_validated_manifest() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let programs_dir = temp.path().join(".perl-lsp/goals/programs");
        fs::create_dir_all(&programs_dir)?;
        fs::write(programs_dir.join("demo_program.toml"), "")?;
        fs::write(
            temp.path().join(".perl-lsp/goals/active.toml"),
            "active_program = \"\"\ndefault_program = \"demo_program\"\n",
        )?;
        let (program, note) = resolve_default_program(temp.path());
        assert_eq!(program, Some("demo_program".to_owned()));
        assert_eq!(note, None);
        Ok(())
    }

    #[test]
    fn render_human_reports_unavailable_when_fetch_failed() {
        let mut receipt = sample_receipt();
        receipt.fetch_ok = false;
        receipt.origin_main_sha = None;
        receipt.behind_origin_main = None;
        receipt.ahead = None;
        let text = render_human(&receipt);
        assert!(text.contains("unavailable (fetch failed)"), "got: {text}");
    }

    #[test]
    fn render_human_reports_behind_and_ahead_counts() {
        let mut receipt = sample_receipt();
        receipt.behind_origin_main = Some(7);
        receipt.ahead = Some(2);
        let text = render_human(&receipt);
        assert!(text.contains("behind=7"), "got: {text}");
        assert!(text.contains("ahead=2"), "got: {text}");
    }

    #[test]
    fn render_human_reports_unknown_dirty_when_check_failed() {
        let mut receipt = sample_receipt();
        receipt.dirty = None;
        receipt.dirty_check_ok = false;
        let text = render_human(&receipt);
        assert!(text.contains("unknown (check failed)"), "got: {text}");
    }

    #[test]
    fn render_human_reports_repo_unknown_when_unresolved() {
        let mut receipt = sample_receipt();
        receipt.repo = None;
        let text = render_human(&receipt);
        assert!(text.contains("repo:             unknown"), "got: {text}");
    }

    /// Cross-checks the struct's serialized field set against
    /// `.ci/receipts/schemas/session-start.schema.json`'s
    /// `required`/`properties` -- every required schema field must be
    /// present on the receipt, and (since the schema declares
    /// `additionalProperties: false`) the receipt must never emit a key
    /// the schema doesn't know about. Keeps the two definitions from
    /// drifting the way #3692 flagged for `WorkPacket::inputs_used`.
    #[test]
    fn emitted_receipt_matches_schema_required_and_property_set() -> Result<()> {
        let root = project_root()?;
        let schema_path = root.join(".ci/receipts/schemas/session-start.schema.json");
        let schema_text = fs::read_to_string(&schema_path)?;
        let schema: serde_json::Value = serde_json::from_str(&schema_text)?;

        let required: Vec<String> = schema
            .get("required")
            .and_then(serde_json::Value::as_array)
            .map(|items| items.iter().filter_map(|v| v.as_str().map(ToOwned::to_owned)).collect())
            .unwrap_or_default();
        let properties: BTreeSet<String> = schema
            .get("properties")
            .and_then(serde_json::Value::as_object)
            .map(|object| object.keys().cloned().collect())
            .unwrap_or_default();

        assert!(!required.is_empty(), "schema declares no required fields");
        assert!(!properties.is_empty(), "schema declares no properties");

        let receipt = sample_receipt();
        let value = serde_json::to_value(&receipt)?;
        let object =
            value.as_object().ok_or_else(|| eyre!("receipt did not serialize to an object"))?;

        for key in &required {
            assert!(object.contains_key(key), "receipt missing schema-required field {key}");
        }
        for key in object.keys() {
            assert!(
                properties.contains(key),
                "receipt field {key} is not declared in schema properties"
            );
        }
        Ok(())
    }
}
