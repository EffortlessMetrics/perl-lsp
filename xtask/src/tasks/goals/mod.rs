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

pub fn next(program: Option<String>, fixture: Option<PathBuf>, json: bool) -> Result<()> {
    let snap = snapshot::build_snapshot(program, fixture)?;
    let decision = select::select_next(&snap);
    let output = GoalsNextOutput {
        repository: snap.repository,
        program: snap.resolved_program,
        program_title: snap.program_title,
        tracker_issue: snap.tracker_issue,
        decision,
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        print_human(&output);
    }
    Ok(())
}

fn print_human(output: &GoalsNextOutput) {
    println!("repository: {}", output.repository);
    println!("program: {}", output.program.as_deref().unwrap_or("<ambiguous>"));
    match &output.decision {
        SelectionDecision::Selected(packet) => {
            println!("selected: {} — {}", packet.id, packet.session_goal);
            println!("reason: {}", packet.reason);
            println!("mode: {}", packet.mode);
        }
        SelectionDecision::Blocked(blockers) => {
            println!("blocked:");
            for blocker in blockers {
                println!("  - [{}] {}", blocker.kind, blocker.detail);
            }
        }
        SelectionDecision::Complete(evidence) => {
            println!("complete: {} — {}", evidence.program, evidence.detail);
        }
    }
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
}
