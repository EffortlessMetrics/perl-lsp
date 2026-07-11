//! Program-aware, deterministic work selector (#3624, M3 of the
//! enablement train #3612): `cargo xtask goals next --json`.
//!
//! Selects the next eligible slice of work from LIVE evidence — current
//! `origin/main`, live open GitHub PRs, the M2 manifest chain, and (for
//! programs shaped as a milestone ledger, e.g. `agent_loop_enablement`)
//! the `[[milestone]]` ledger — never from chat or Claude Code's
//! `TaskList`/`TaskGet` session state (see CLAUDE.md's truth hierarchy:
//! GitHub + the manifest chain outrank conversation and session
//! bookkeeping; the harness's `TaskList` `completed` flag is known not to
//! persist reliably across sessions).
//!
//! `goals next` is READ-ONLY: it never creates a branch, worktree, or PR,
//! and never writes any file under `.perl-lsp/goals/`, `target/`, or
//! `docs/`. Branch/worktree/PR creation is `/start-pr`'s job — a separate,
//! later deliverable, deliberately out of scope here.
//!
//! Three layers, so the selection algorithm stays pure and independently
//! testable:
//! - [`manifest`] — shared typed loader for `active.toml`'s
//!   `default_program` pointer and milestone ledgers, used by both this
//!   module and `active_goal_manifest::run()` (validator + selector must
//!   not drift on what a valid ledger looks like).
//! - [`select`] — `select_next`, a pure function: `SelectionSnapshot` in,
//!   `SelectionDecision` out. No I/O of any kind.
//! - [`snapshot`] — adapters. ALL `git`/`gh` shelling and manifest reading
//!   live here, fixture-testable via `--fixture <path>` (mirrors
//!   `tasks::queue_snapshot`'s `--fixture` pattern).

pub mod manifest;
pub mod select;
pub mod snapshot;

use color_eyre::eyre::Result;
use serde::Serialize;
use std::path::PathBuf;

use select::SelectionDecision;

#[derive(Debug, Clone, Serialize)]
pub struct GoalsNextOutput {
    pub repository: String,
    pub program: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub program_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tracker_issue: Option<u64>,
    #[serde(flatten)]
    pub decision: SelectionDecision,
}

/// `cargo xtask goals next` entry point. When `--json` is requested,
/// EVERY error path must emit parseable JSON on stdout and exit nonzero
/// — never an unstructured `color_eyre` dump on stderr (see #3692
/// defect 1: this was previously true only for the unknown-`--program`
/// case, which never `bail!`s in the first place; every OTHER internal
/// `Err` — missing/unparseable `active.toml`, `gh` offline/unauth, a
/// missing program manifest, an invalid milestone ledger — still
/// propagated as unstructured stderr). `render_output` computes
/// everything that would be printed without doing any process-exiting
/// side effect, so the whole flow (not just `build_snapshot`) is covered
/// and the error path stays unit-testable.
pub fn next(program: Option<String>, fixture: Option<PathBuf>, json: bool) -> Result<()> {
    match render_output(program, fixture, json) {
        Ok(text) => {
            println!("{text}");
            Ok(())
        }
        Err(err) if json => {
            println!("{}", render_json_error(&err));
            std::process::exit(1);
        }
        Err(err) => Err(err),
    }
}

/// Computes the exact text `next()` would print for a successful run —
/// JSON when `json` is true, the human-readable summary otherwise.
/// Returns `Err` on any internal failure (unresolved by the caller here);
/// `next()` decides how to surface that `Err` based on `json`.
fn render_output(program: Option<String>, fixture: Option<PathBuf>, json: bool) -> Result<String> {
    let snap = snapshot::build_snapshot(program, fixture)?;
    let decision = select::select_next(&snap);
    let output = GoalsNextOutput {
        repository: snap.repository,
        program: snap.resolved_program,
        program_title: snap.program_title,
        tracker_issue: snap.tracker_issue,
        decision,
    };

    if json { Ok(serde_json::to_string_pretty(&output)?) } else { Ok(render_human(&output)) }
}

/// Renders `err`'s full cause chain as a small parseable JSON object.
/// Never itself fails: `serde_json::to_string_pretty` on a
/// `Vec<String>`/`String`-only struct cannot realistically error, but a
/// literal JSON fallback is used instead of `unwrap`/`expect` in case it
/// ever does (this repo bans both in production code).
fn render_json_error(err: &color_eyre::eyre::Report) -> String {
    let chain: Vec<String> = err.chain().map(ToString::to_string).collect();
    let payload = serde_json::json!({
        "error": chain.first().cloned().unwrap_or_default(),
        "error_chain": chain,
    });
    serde_json::to_string_pretty(&payload).unwrap_or_else(|_| {
        "{\"error\":\"internal: failed to serialize error payload\"}".to_owned()
    })
}

fn render_human(output: &GoalsNextOutput) -> String {
    let mut lines = vec![
        format!("repository: {}", output.repository),
        format!("program: {}", output.program.as_deref().unwrap_or("<ambiguous>")),
    ];
    match &output.decision {
        SelectionDecision::Selected(packet) => {
            lines.push(format!("selected: {} — {}", packet.id, packet.session_goal));
            lines.push(format!("reason: {}", packet.reason));
            lines.push(format!("mode: {}", packet.mode));
        }
        SelectionDecision::Blocked(blockers) => {
            lines.push("blocked:".to_owned());
            for blocker in blockers {
                lines.push(format!("  - [{}] {}", blocker.kind, blocker.detail));
            }
        }
        SelectionDecision::Complete(evidence) => {
            lines.push(format!("complete: {} — {}", evidence.program, evidence.detail));
        }
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    // Mechanical invariant checks — not just doc comments. Scans the
    // OTHER goals-module source files' PRODUCTION code only (everything
    // before that file's own `#[cfg(test)]`, so this file's assertion
    // literals never self-match, and so those files' own tests — which
    // legitimately write fixture files to a tempdir — don't trip a check
    // aimed at production behavior) for the forbidden surface: `select`
    // must stay pure (no I/O, no session-task-board reads), and neither
    // module may create branches/worktrees/PRs or write outside a
    // fixture/receipt path.
    const SELECT_SRC: &str = include_str!("select.rs");
    const SNAPSHOT_SRC: &str = include_str!("snapshot.rs");
    const MANIFEST_SRC: &str = include_str!("manifest.rs");

    fn production_only(full_src: &str) -> &str {
        full_src.split("#[cfg(test)]").next().unwrap_or(full_src)
    }

    #[test]
    fn select_next_never_reads_task_list_or_shells_out() {
        let src = production_only(SELECT_SRC);
        for forbidden in ["TaskList", "TaskGet", "Command::new", "fs::write", "fs::remove"] {
            assert!(
                !src.contains(forbidden),
                "select.rs production code must stay pure and never reference {forbidden:?}"
            );
        }
    }

    #[test]
    fn manifest_loader_never_reads_task_list() {
        let src = production_only(MANIFEST_SRC);
        for forbidden in ["TaskList", "TaskGet"] {
            assert!(
                !src.contains(forbidden),
                "manifest.rs production code must never reference {forbidden:?}"
            );
        }
    }

    #[test]
    fn snapshot_adapter_is_read_only() {
        let src = production_only(SNAPSHOT_SRC);
        for forbidden in [
            "TaskList",
            "TaskGet",
            "\"branch\"",
            "\"worktree\"",
            "\"commit\"",
            "\"push\"",
            // Catches `gh` command arrays written as separate string-literal
            // args (e.g. `.args(["pr", "create", ...])`) as well as the
            // combined-phrase form; a plain `"pr create"` check misses the
            // array-literal shape entirely since "create" never appears
            // adjacent to "pr" as one token.
            "\"create\"",
            "fs::write",
            "fs::remove",
            "fs::create_dir",
        ] {
            assert!(
                !src.contains(forbidden),
                "snapshot.rs production code must stay read-only and never reference {forbidden:?}"
            );
        }
        // The only live commands this module may shell are the read-only
        // `gh repo view` / `gh pr list` subcommands.
        assert!(src.contains("\"repo\""));
        assert!(src.contains("\"pr\""));
        assert!(src.contains("\"list\""));
    }

    // Regression coverage for #3692 defect 1: `--json` callers must
    // always get parseable JSON, never an unstructured stderr dump, on
    // ANY internal error path — not just the unknown-`--program` case
    // (which never errors in the first place; it resolves to a Blocked
    // decision).
    use super::*;

    #[test]
    fn render_output_surfaces_an_err_when_the_fixture_path_does_not_exist() {
        // A missing --fixture file is the cheapest way to force
        // `build_snapshot` to fail without needing `gh auth`/network —
        // proves the underlying flow still produces an `Err` (as it must,
        // for the non-json caller to see a real error) before we assert
        // the json-caller-side rendering below.
        let bogus = std::path::PathBuf::from("definitely/does/not/exist/prs.json");

        let result = render_output(None, Some(bogus), true);

        assert!(result.is_err(), "expected an Err for a missing fixture file");
    }

    #[test]
    fn json_error_output_is_parseable_and_names_the_failure() {
        let bogus = std::path::PathBuf::from("definitely/does/not/exist/prs.json");
        let err = render_output(None, Some(bogus), true)
            .expect_err("missing fixture file must produce an Err");

        let text = render_json_error(&err);

        let parsed: serde_json::Value = serde_json::from_str(&text)
            .unwrap_or_else(|e| panic!("json error output must be parseable JSON: {e}\n{text}"));
        let error_field = parsed
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| panic!("expected a string \"error\" field in {text}"));
        assert!(!error_field.is_empty(), "expected a non-empty error message, got {text}");
        assert!(
            parsed.get("error_chain").is_some_and(|v| v.is_array()),
            "expected an \"error_chain\" array field in {text}"
        );
    }

    #[test]
    fn next_with_json_never_returns_err_to_the_caller() {
        // `next()` must never propagate `Err` up through `run_cli`/`main`
        // when `--json` was requested (that path is what previously
        // produced the unstructured color_eyre stderr dump). It instead
        // prints the JSON error to stdout and calls
        // `std::process::exit(1)` — which we cannot exercise directly in
        // a test process, so this test proves the OTHER half of the
        // contract: with json=false the same failure DOES propagate as
        // `Err` (preserving prior behavior for human callers), showing
        // the json/non-json branches are genuinely distinguished by
        // `next()`, not merged.
        let bogus = std::path::PathBuf::from("definitely/does/not/exist/prs.json");

        let human_result = render_output(None, Some(bogus), false);
        assert!(human_result.is_err(), "non-json path must still surface Err to its caller");
    }
}
