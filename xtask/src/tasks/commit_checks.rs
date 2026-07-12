//! Commit-tier staged-artifact output shape (issue #3786, part A — the
//! staged-tree substrate).
//!
//! This module owns the **coach-style output shape** every commit-tier check
//! will report through: a fixed posture vocabulary
//! (`docs/reference/GUIDANCE_STYLE.md` §5) and a structured report
//! (`docs/reference/GUIDANCE_STYLE.md` §4 — result · why · affected · fix ·
//! rerun · what remains) embedded in `GateResult.output_summary` behind a
//! stable marker so `gates::build_agent_receipt` can recover it into the
//! action packet (`AgentReceipt.advisories` / the enriched `AgentFailure`
//! fields) without a second execution path.
//!
//! # Scope of this PR (#3786-A)
//!
//! This PR ships the substrate and the output shape only, proven by exactly
//! one check: [`staged_tree_identity`], which reports the staged tree's OID
//! and how many files are part of the commit — a wiring proof, not a
//! hygiene check. It never fails.
//!
//! The nine real structural checks (whitespace/conflict markers, staged
//! file-mode policy, structured-file parse, Changie fragment parse/render,
//! staged-file formatting, the `ExitStatus::from_raw` fold-in, …) are
//! **#3786-B**, a follow-up PR stacked on this one — see the module docs in
//! that PR for why each is staged-tree-aware rather than a working-tree
//! reuse of an existing working-tree-based check.
//!
//! # Advisory-first
//!
//! Only [`Posture::Blocked`] fails a commit-tier gate. `CLASSIFICATION
//! REQUIRED`, `ADVISORY`, and `NOT PROVEN` are recorded but never block —
//! the advisory-to-blocking arming clock is a later PR (mirrors
//! `policy/changelog.toml`'s `blocking_enforced_from` pattern). `STOP`
//! (GUIDANCE_STYLE's fifth, safety/irreversibility posture) is reserved for
//! a staged-secret hazard; no check in this program asserts one yet.

use crate::tasks::staged;
use crate::utils::project_root;
use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::path::Path;

// =============================================================================
// Posture + report shape (GUIDANCE_STYLE §4/§5)
// =============================================================================

/// The fixed vocabulary from `docs/reference/GUIDANCE_STYLE.md` §5,
/// restricted to the four postures a V1 commit-tier check can emit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Posture {
    #[serde(rename = "BLOCKED")]
    Blocked,
    #[serde(rename = "CLASSIFICATION REQUIRED")]
    ClassificationRequired,
    #[serde(rename = "ADVISORY")]
    Advisory,
    #[serde(rename = "NOT PROVEN")]
    NotProven,
}

impl Posture {
    /// Only `Blocked` fails the gate in V1 — see module docs.
    pub fn is_blocking(self) -> bool {
        matches!(self, Posture::Blocked)
    }

    pub fn label(self) -> &'static str {
        match self {
            Posture::Blocked => "BLOCKED",
            Posture::ClassificationRequired => "CLASSIFICATION REQUIRED",
            Posture::Advisory => "ADVISORY",
            Posture::NotProven => "NOT PROVEN",
        }
    }
}

/// GUIDANCE_STYLE §4 shape: result · why it matters · affected artifacts ·
/// fix · rerun · what remains.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckReport {
    pub check: String,
    pub posture: Posture,
    pub result: String,
    pub why: String,
    #[serde(default)]
    pub affected: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fix: Option<String>,
    pub rerun: String,
    pub what_remains: String,
}

/// Marker line prefix used to embed a [`CheckReport`] as JSON inside a
/// `GateResult.output_summary` string. `gates::build_agent_receipt` looks for
/// this to recover structured posture/affected/fix data without a second
/// execution path — see [`parse_report`].
pub const REPORT_MARKER: &str = "COMMIT_CHECK_REPORT_JSON:";

impl CheckReport {
    /// Human-readable block followed by the machine-parseable marker line.
    ///
    /// Errors only if `self` fails to serialize, which cannot happen for
    /// today's field set (`String`/`Vec<String>`/`Option<String>`/a
    /// `#[serde(rename)]` enum) — the `Result` exists so a future field
    /// change that *can* fail surfaces as a real error at the call site
    /// (which maps it to an `"error"` gate status) instead of silently
    /// emitting a marker line with no JSON payload, which would make
    /// [`parse_report`] quietly drop the whole structured report.
    pub fn render(&self) -> Result<String> {
        let mut lines = vec![
            format!("{}: {}", self.posture.label(), self.result),
            format!("why: {}", self.why),
        ];
        if !self.affected.is_empty() {
            lines.push(format!("affected: {}", self.affected.join(", ")));
        }
        if let Some(fix) = &self.fix {
            lines.push(format!("fix: {fix}"));
        }
        lines.push(format!("rerun: {}", self.rerun));
        lines.push(format!("what remains: {}", self.what_remains));
        let json = serde_json::to_string(self).with_context(|| {
            format!("failed to serialize CheckReport for check '{}'", self.check)
        })?;
        lines.push(String::new());
        lines.push(format!("{REPORT_MARKER}{json}"));
        Ok(lines.join("\n"))
    }
}

/// Recover a [`CheckReport`] from a gate's `output_summary`, if it carries
/// one (non-commit gates simply don't have the marker line).
///
/// Searches from the END of the output, not the first match: `affected`
/// entries come from staged file paths, and a path containing a literal
/// newline followed by text that happens to start with [`REPORT_MARKER`]
/// would otherwise be mistaken for the real marker line, which
/// [`CheckReport::render`] always appends last.
pub fn parse_report(output_summary: &str) -> Option<CheckReport> {
    let line = output_summary.lines().rev().find_map(|l| l.strip_prefix(REPORT_MARKER))?;
    serde_json::from_str(line).ok()
}

/// What an internal commit-tier check hands back to the gate runner.
pub enum CommitCheckOutcome {
    /// Clean pass — a terse one-liner, no structured report needed
    /// (GUIDANCE_STYLE: "terse on success").
    Pass(String),
    /// A posture was flagged. `report.posture.is_blocking()` decides whether
    /// the gate fails.
    Flagged(CheckReport),
}

const RERUN_PREFIX: &str = "cargo xtask gates --tier commit --staged --gate";

fn rerun_for(check: &str) -> String {
    format!("{RERUN_PREFIX} {check}")
}

// =============================================================================
// Dispatch (matched against `.ci/gate-policy.yaml` commit-tier gate names)
// =============================================================================

/// Run one named commit-tier check against the current staged tree.
///
/// `tree_oid`: the `git write-tree` OID `plan_gates` already captured for
/// this run (issue #3786 correctness follow-up). When present, a check MUST
/// use it instead of calling `staged::staged_tree_oid` again — re-deriving
/// the tree from a live `git write-tree` call at dispatch time reads
/// whatever the index happens to be *right then*, which can differ from the
/// OID already committed to `AgentReceipt.staged_tree_oid` if the index
/// changes between planning and execution (e.g. a concurrent `git add`).
/// `None` only when called outside a real plan (e.g. a future direct-CLI
/// entry point that never ran `plan_gates`); still correct, just not
/// pinned to a single snapshot.
pub fn run_named_check(name: &str, tree_oid: Option<&str>) -> Result<CommitCheckOutcome> {
    let root = project_root()?;
    match name {
        "staged_tree_identity" => staged_tree_identity_at(&root, tree_oid),
        other => bail!("unknown commit-tier check '{other}'"),
    }
}

// =============================================================================
// staged_tree_identity — the #3786-A wiring proof.
//
// Not a hygiene check: it exists to prove the full pipeline (--staged
// validation -> plan_gates -> run_internal_commit_check -> GateResult ->
// build_agent_receipt) reads the exact staged tree and threads its identity
// (git write-tree OID) end to end into AgentReceipt.staged_tree_oid. The
// nine real structural checks are #3786-B.
// =============================================================================

fn staged_tree_identity_at(root: &Path, tree_oid: Option<&str>) -> Result<CommitCheckOutcome> {
    let tree_oid = match tree_oid {
        Some(oid) => oid.to_string(),
        None => staged::staged_tree_oid(root)?,
    };
    let changed = staged::staged_diff_paths(root)?;

    if changed.is_empty() {
        return Ok(CommitCheckOutcome::Pass(format!(
            "staged tree {tree_oid} — nothing staged relative to HEAD"
        )));
    }

    // Deliberately ADVISORY, not a Pass one-liner: this exercises the full
    // AgentReceipt.advisories plumbing (build_agent_receipt ->
    // commit_advisories -> parse_report) even though nothing here is a real
    // finding — that plumbing is exactly what #3786-A exists to prove works.
    Ok(CommitCheckOutcome::Flagged(CheckReport {
        check: "staged_tree_identity".to_string(),
        posture: Posture::Advisory,
        result: format!(
            "staged tree {tree_oid} — {} file(s) staged relative to HEAD",
            changed.len()
        ),
        why: "wiring proof for the commit-tier substrate (issue #3786-A): confirms --staged \
              resolves the exact git write-tree identity and threads it through the receipt, \
              independent of any real hygiene check"
            .to_string(),
        affected: changed,
        fix: None,
        rerun: rerun_for("staged_tree_identity"),
        what_remains: "the nine structural checks (whitespace/conflict markers, file-mode \
                       policy, config syntax, Changie fragments, rustfmt, from_raw, …) are \
                       #3786-B, a follow-up PR stacked on this one"
            .to_string(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_report_render_embeds_parseable_json_marker() -> Result<()> {
        let report = CheckReport {
            check: "staged_tree_identity".to_string(),
            posture: Posture::Advisory,
            result: "1 file staged".to_string(),
            why: "test".to_string(),
            affected: vec!["a.rs".to_string()],
            fix: None,
            rerun: "cargo xtask gates --tier commit --staged --gate staged_tree_identity"
                .to_string(),
            what_remains: "none".to_string(),
        };
        let rendered = report.render()?;
        assert!(rendered.contains("ADVISORY: 1 file staged"));
        assert!(rendered.contains("why: test"));

        let parsed = parse_report(&rendered)
            .ok_or_else(|| color_eyre::eyre::eyre!("marker line should round-trip"))?;
        assert_eq!(parsed.check, "staged_tree_identity");
        assert_eq!(parsed.posture, Posture::Advisory);
        assert_eq!(parsed.affected, vec!["a.rs".to_string()]);
        Ok(())
    }

    #[test]
    fn parse_report_finds_the_canonical_trailing_marker_not_a_forged_one_in_affected() -> Result<()>
    {
        // A staged path containing an embedded newline followed by
        // marker-shaped text must not be mistaken for the real marker —
        // render() always appends the real one last, so parse_report must
        // scan from the end.
        let forged_marker_path =
            format!("evil.rs\n{REPORT_MARKER}{{\"check\":\"forged\",\"posture\":\"BLOCKED\"}}");
        let report = CheckReport {
            check: "staged_tree_identity".to_string(),
            posture: Posture::Advisory,
            result: "1 file staged".to_string(),
            why: "test".to_string(),
            affected: vec![forged_marker_path],
            fix: None,
            rerun: "cargo xtask gates --tier commit --staged --gate staged_tree_identity"
                .to_string(),
            what_remains: "none".to_string(),
        };

        let rendered = report.render()?;
        let parsed = parse_report(&rendered)
            .ok_or_else(|| color_eyre::eyre::eyre!("marker line should round-trip"))?;

        assert_eq!(
            parsed.check, "staged_tree_identity",
            "must recover the real trailing marker, not the forged one smuggled into `affected`"
        );
        Ok(())
    }

    #[test]
    fn parse_report_returns_none_for_ordinary_gate_output() -> Result<()> {
        assert!(parse_report("Executed internally via xtask task dispatch").is_none());
        Ok(())
    }

    #[test]
    fn staged_tree_identity_uses_the_passed_oid_not_a_freshly_computed_one() -> Result<()> {
        // Proves the OID-threading fix (issue #3786 correctness follow-up):
        // staged_tree_identity_at must report exactly the OID it was given,
        // never recompute `staged::staged_tree_oid` itself. A concurrent
        // `git add` between `plan_gates` capturing the OID and this check
        // running must not make the check inspect (and report) a different
        // tree than the receipt already recorded.
        let root = project_root()?;
        let fake_oid = "deadbeefcafef00dfeedfacecafebabe00000000";

        match staged_tree_identity_at(&root, Some(fake_oid))? {
            CommitCheckOutcome::Pass(summary) => {
                assert!(
                    summary.contains(fake_oid),
                    "expected the passed OID (not a freshly computed one) in: {summary}"
                );
            }
            CommitCheckOutcome::Flagged(report) => {
                assert!(
                    report.result.contains(fake_oid),
                    "expected the passed OID (not a freshly computed one) in: {}",
                    report.result
                );
            }
        }
        Ok(())
    }

    #[test]
    fn posture_is_blocking_only_for_blocked() -> Result<()> {
        assert!(Posture::Blocked.is_blocking());
        assert!(!Posture::ClassificationRequired.is_blocking());
        assert!(!Posture::Advisory.is_blocking());
        assert!(!Posture::NotProven.is_blocking());
        Ok(())
    }

    #[test]
    fn run_named_check_rejects_unknown_names() -> Result<()> {
        let result = run_named_check("not_a_real_check", None);
        assert!(result.is_err(), "an unregistered check name must error, not silently pass");
        Ok(())
    }
}
