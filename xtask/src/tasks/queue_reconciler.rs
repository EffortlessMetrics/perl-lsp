//! Queue-wide label contradiction reconciler.
//!
//! # Model
//!
//! Labels fall into two categories based on whether live ground truth exists:
//!
//! ## Labels with live ground truth — CI status
//!
//! `ci-green` and `needs-ci-fix` are informational about **agent activity**, not
//! the actual CI state. **Live CI (statusCheckRollup) is the ground truth:**
//!
//! - Live CI GREEN → strip `needs-ci-fix` (the action it flagged for is moot)
//! - Live CI SKIPPED → strip `needs-ci-fix` (gate is not applicable — path-conditioning)
//! - Live CI RED → leave `needs-ci-fix` in place; dispatch green-ci to fix
//! - Live CI PENDING → leave both alone
//!
//! "SKIPPED" arises from path-conditioning: docs-only or CI-config-only diffs
//! skip the Rust build lanes, so the merge-gate job is never required for those
//! PRs. The reconciler treats SKIPPED the same as GREEN for `needs-ci-fix`
//! resolution — the gate obligation is satisfied because it was never applicable.
//!
//! For `merge-ready + needs-ci-fix` specifically:
//! - Live CI GREEN → strip `needs-ci-fix`, keep `merge-ready`
//! - Live CI SKIPPED → strip `needs-ci-fix`, keep `merge-ready` (gate not applicable)
//! - Live CI RED → strip `merge-ready` (live CI blocks merge)
//! - Live CI PENDING → leave both
//!
//! ## Labels without live ground truth — "later-applied wins"
//!
//! These pairs don't have a live equivalent; timeline is the best signal:
//! - `deep-reviewed + needs-deep-review` → whichever was applied later wins
//! - `diff-audited + needs-diff-fix` → whichever was applied later wins
//! - `review-reviewed + needs-builder-fix` → whichever was applied later wins
//! - `maintainer-pr-reviewed + needs-builder-fix` → whichever was applied later wins
//!
//! When both labels were applied within 5 seconds of each other (simultaneous),
//! sign-off beats routing. When no timeline is available, use the conservative
//! fallback: keep the sign-off, strip the routing label.
//!
//! ## merge-ready + non-CI needs-* labels
//!
//! CLAUDE.md doctrine: any `needs-*` blocks merge. So `merge-ready + needs-builder-fix`
//! → strip `merge-ready`. The only exception is `merge-ready + needs-ci-fix`, which is
//! handled by the live CI rule above.
//!
//! # Idempotency guarantee
//!
//! A second run on unchanged label state is always a no-op. All contradiction detectors
//! only fire when BOTH labels in a pair are simultaneously present.
//!
//! # Commenting policy
//!
//! Every label change produces a structured `## Reconciler action` comment on the PR.
//! Per memory `feedback_comment_trail_over_overwrite`: always post — the trail teaches
//! future agents.

use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write as IoWrite;
use std::path::PathBuf;
use std::process::Command;

use crate::utils::project_root;

/// Default output path for the queue reconciliation receipt.
const DEFAULT_QUEUE_RECEIPT_PATH: &str = "target/receipts/queue-reconcile.json";

/// Within this many seconds, treat two label-applied timestamps as simultaneous.
/// When simultaneous, sign-off labels beat routing (needs-*) labels.
const SIMULTANEOUS_THRESHOLD_SECS: i64 = 5;

// ---------------------------------------------------------------------------
// Label name constants
// ---------------------------------------------------------------------------

const MERGE_READY: &str = "merge-ready";
const CI_GREEN: &str = "ci-green";
const DEEP_REVIEWED: &str = "deep-reviewed";
const DIFF_AUDITED: &str = "diff-audited";
const REVIEW_REVIEWED: &str = "review-reviewed";
const MAINTAINER_PR_REVIEWED: &str = "maintainer-pr-reviewed";

const NEEDS_CI_FIX: &str = "needs-ci-fix";
const NEEDS_DEEP_REVIEW: &str = "needs-deep-review";
const NEEDS_DIFF_FIX: &str = "needs-diff-fix";
const NEEDS_BUILDER_FIX: &str = "needs-builder-fix";

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

/// A label event from the GitHub timeline API.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LabelEvent {
    pub label: String,
    /// RFC 3339 timestamp at which the label was applied.
    pub applied_at: String,
    pub applied_by: String,
}

impl LabelEvent {
    fn epoch_secs(&self) -> i64 {
        chrono::DateTime::parse_from_rfc3339(&self.applied_at).map(|dt| dt.timestamp()).unwrap_or(0)
    }
}

/// A label that should be stripped, with the reason.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Contradiction {
    /// The label to KEEP (or keep by virtue of being ground truth).
    pub keep: String,
    /// The label to STRIP.
    pub strip: String,
    /// Human-readable explanation.
    pub reason: String,
}

/// Actions taken or planned for a single PR.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrAction {
    pub pr_number: u64,
    pub contradictions: Vec<Contradiction>,
    /// True when this action was actually applied (not just planned).
    pub applied: bool,
}

/// Summary receipt emitted to `target/receipts/queue-reconcile.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueReconcileReceipt {
    pub reconciled_at: String,
    pub total_prs_scanned: usize,
    pub prs_with_contradictions: usize,
    pub total_labels_stripped: usize,
    pub applied: bool,
    pub actions: Vec<PrAction>,
}

/// Minimal representation of an open PR from the gh CLI.
#[derive(Debug, Clone)]
pub struct OpenPr {
    pub number: u64,
    pub labels: Vec<String>,
    /// The commit SHA at the head of the PR branch.
    /// Populated for future use in stale-signoff detection (PR2 scope).
    #[allow(dead_code)]
    pub head_ref_oid: String,
}

/// Live CI state for a PR head SHA.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum CiOutcome {
    Success,
    Failure,
    Pending,
    /// All present checks were SKIPPED — path-conditioning excluded the gate
    /// (e.g. docs-only diff). Treated like `Success` for `needs-ci-fix`
    /// resolution: the gate obligation is satisfied because it was inapplicable.
    /// Does NOT trigger the live-CI-red `merge-ready` strip.
    Skipped,
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Run the full queue reconciliation.
///
/// When `apply` is `true`, label changes are pushed to GitHub and structured
/// comments are posted. When `false`, performs a dry-run scan and writes a
/// receipt (with `applied: false`) without GitHub mutations.
pub fn reconcile_queue(
    apply: bool,
    pr_filter: Option<u64>,
    receipt_path: Option<PathBuf>,
) -> Result<()> {
    let prs = fetch_open_prs(pr_filter)?;
    let total = prs.len();
    let run_timestamp = chrono::Utc::now().to_rfc3339();
    let mut actions: Vec<PrAction> = Vec::new();

    for pr in &prs {
        // Fetch timeline events (best-effort; empty → conservative label-only fallback).
        let events = fetch_label_timeline(pr.number).unwrap_or_default();

        // Query live CI state for this PR.
        let ci_outcome = query_live_ci_state(pr.number).unwrap_or(CiOutcome::Pending);

        let contradictions = detect_contradictions(pr, &events, ci_outcome);

        if contradictions.is_empty() {
            continue;
        }

        actions.push(PrAction { pr_number: pr.number, contradictions, applied: false });
    }

    let mut prs_touched = 0usize;
    let mut labels_stripped = 0usize;

    for action in &mut actions {
        if apply {
            let mut strips: Vec<String> = Vec::new();
            for c in &action.contradictions {
                match strip_label(action.pr_number, &c.strip) {
                    Ok(()) => {
                        strips.push(c.strip.clone());
                        labels_stripped += 1;
                    }
                    Err(e) => {
                        eprintln!(
                            "warn: failed to strip `{}` from PR #{}: {e}",
                            c.strip, action.pr_number
                        );
                    }
                }
            }

            if !strips.is_empty() {
                prs_touched += 1;
                action.applied = true;

                // Post structured comment for every label change (audit trail).
                let comment = build_comment(&action.contradictions, &strips, &run_timestamp);
                if !comment.is_empty() {
                    // Propagate comment errors as warnings; don't fail the whole reconcile run.
                    let post_result = post_comment(action.pr_number, &comment);
                    if let Err(e) = post_result {
                        eprintln!("warn: failed to post comment on PR #{}: {e}", action.pr_number);
                    }
                }
            }
        } else {
            prs_touched += 1;
            labels_stripped += action.contradictions.len();
        }
    }

    if apply {
        println!(
            "reconcile-queue: applied — {} PRs touched, {} labels stripped",
            prs_touched, labels_stripped
        );
    } else {
        println!(
            "reconcile-queue: dry-run — {} PRs would be touched, {} labels would be stripped",
            prs_touched, labels_stripped
        );
        for action in &actions {
            for c in &action.contradictions {
                println!("  #{}: would strip `{}` — {}", action.pr_number, c.strip, c.reason);
            }
        }
    }

    let receipt = QueueReconcileReceipt {
        reconciled_at: run_timestamp,
        total_prs_scanned: total,
        prs_with_contradictions: actions.len(),
        total_labels_stripped: labels_stripped,
        applied: apply,
        actions,
    };

    let root = project_root()?;
    let out = receipt_path.unwrap_or_else(|| root.join(DEFAULT_QUEUE_RECEIPT_PATH));
    write_receipt(&out, &receipt)?;
    println!("wrote receipt: {}", out.display());

    Ok(())
}

// ---------------------------------------------------------------------------
// Core contradiction detection
// ---------------------------------------------------------------------------

/// Detect all label contradictions for a PR, using live CI state and timeline events.
///
/// This is the authoritative entry point for detection. It dispatches to
/// CI-specific logic (live ground truth) and timeline-based logic (no ground truth).
pub fn detect_contradictions(
    pr: &OpenPr,
    events: &[LabelEvent],
    ci_outcome: CiOutcome,
) -> Vec<Contradiction> {
    let mut out = Vec::new();

    let has = |name: &str| pr.labels.iter().any(|l| l == name);

    // --- CI-specific: live state is ground truth ---

    // needs-ci-fix: strip when live CI is definitively green OR all checks were SKIPPED
    // (path-conditioning — e.g. docs-only diff excludes the gate entirely).
    if has(NEEDS_CI_FIX) && matches!(ci_outcome, CiOutcome::Success | CiOutcome::Skipped) {
        let reason = if ci_outcome == CiOutcome::Skipped {
            format!(
                "live CI checks are all SKIPPED (path-conditioning) — \
                 gate obligation is not applicable; {NEEDS_CI_FIX} is stale, stripping it"
            )
        } else {
            format!("live CI Gate is SUCCESS — {NEEDS_CI_FIX} is stale, stripping it")
        };
        out.push(Contradiction {
            keep: CI_GREEN.to_string(),
            strip: NEEDS_CI_FIX.to_string(),
            reason,
        });
    }

    // merge-ready + needs-ci-fix:
    // - Live CI green or all SKIPPED: strip needs-ci-fix, keep merge-ready (handled above)
    // - Live CI red: strip merge-ready (live CI blocks merge)
    // - Live CI pending: leave both
    if has(MERGE_READY) && has(NEEDS_CI_FIX) && ci_outcome == CiOutcome::Failure {
        out.push(Contradiction {
            keep: NEEDS_CI_FIX.to_string(),
            strip: MERGE_READY.to_string(),
            reason: format!(
                "live CI Gate is FAILURE — {MERGE_READY} must be stripped (red always blocks merge)"
            ),
        });
    }

    // --- merge-ready + non-CI needs-* labels ---
    // CLAUDE.md doctrine: any needs-* blocks merge.
    let non_ci_needs: Vec<String> = pr
        .labels
        .iter()
        .filter(|l| l.starts_with("needs-") && l.as_str() != NEEDS_CI_FIX)
        .cloned()
        .collect();

    if has(MERGE_READY) && !non_ci_needs.is_empty() {
        out.push(Contradiction {
            keep: non_ci_needs.first().cloned().unwrap_or_default(),
            strip: MERGE_READY.to_string(),
            reason: format!(
                "{MERGE_READY} contradicts {} — per CLAUDE.md doctrine, any needs-* blocks merge",
                non_ci_needs.join(", ")
            ),
        });
    }

    // --- Non-CI pairs: "later-applied wins" via timeline ---
    let non_ci_pairs: &[(&str, &str)] = &[
        (DEEP_REVIEWED, NEEDS_DEEP_REVIEW),
        (DIFF_AUDITED, NEEDS_DIFF_FIX),
        (REVIEW_REVIEWED, NEEDS_BUILDER_FIX),
        (MAINTAINER_PR_REVIEWED, NEEDS_BUILDER_FIX),
    ];

    let mut stripped_routing: Vec<&str> = Vec::new(); // track to avoid duplicates

    for &(signoff, routing) in non_ci_pairs {
        if !has(signoff) || !has(routing) {
            continue;
        }
        // Avoid emitting the same routing-label strip twice (e.g., both review-reviewed
        // and maintainer-pr-reviewed with needs-builder-fix).
        if stripped_routing.contains(&routing) {
            continue;
        }

        let c = resolve_non_ci_pair(signoff, routing, events);
        stripped_routing.push(routing);
        out.push(c);
    }

    out
}

/// Resolve a contradicting pair that has no live ground truth using timeline events.
///
/// Rule: whichever label was applied later wins. If within the simultaneous threshold,
/// sign-off beats routing. If no timeline data, conservative fallback: keep sign-off.
fn resolve_non_ci_pair(signoff: &str, routing: &str, events: &[LabelEvent]) -> Contradiction {
    let find_latest = |name: &str| -> Option<i64> {
        events.iter().filter(|e| e.label == name).map(|e| e.epoch_secs()).max()
    };

    let signoff_ts = find_latest(signoff);
    let routing_ts = find_latest(routing);

    let strip = match (signoff_ts, routing_ts) {
        (Some(st), Some(rt)) => {
            let diff = st - rt;
            if diff.abs() <= SIMULTANEOUS_THRESHOLD_SECS {
                // Simultaneous → keep sign-off.
                routing.to_string()
            } else if st > rt {
                // Sign-off is newer → strip routing.
                routing.to_string()
            } else {
                // Routing is newer → strip sign-off.
                signoff.to_string()
            }
        }
        // Conservative fallback: no timeline data → keep sign-off.
        _ => routing.to_string(),
    };

    let keep = if strip == signoff { routing.to_string() } else { signoff.to_string() };

    let (newer_ts, older_ts, newer_label, older_label) = if strip == routing {
        (signoff_ts.unwrap_or(0), routing_ts.unwrap_or(0), signoff, routing)
    } else {
        (routing_ts.unwrap_or(0), signoff_ts.unwrap_or(0), routing, signoff)
    };

    let reason = if signoff_ts.is_none() || routing_ts.is_none() {
        format!(
            "{routing} contradicts {signoff}; no timeline data — conservative: keeping sign-off"
        )
    } else {
        format!(
            "`{newer_label}` applied later (ts={newer_ts}) supersedes `{older_label}` (ts={older_ts}) — later wins"
        )
    };

    Contradiction { keep, strip, reason }
}

/// Fallback: detect contradictions using only the label list (no timeline, no live CI).
///
/// Conservative: always keeps sign-off labels and strips routing labels.
/// Used when timeline API is unavailable and live CI is not queried.
/// Available as a public test helper and for offline/fixture-based testing.
#[allow(dead_code)]
pub fn detect_contradictions_from_labels(labels: &[String]) -> Vec<Contradiction> {
    let pr = OpenPr { number: 0, labels: labels.to_vec(), head_ref_oid: String::new() };
    // Use conservative pending CI and no events → conservative sign-off preference.
    detect_contradictions(&pr, &[], CiOutcome::Pending)
}

// ---------------------------------------------------------------------------
// Live CI query (via gh CLI)
// ---------------------------------------------------------------------------

/// Query the current-head CI state for a PR via `gh pr view`.
///
/// Returns `CiOutcome::Pending` when any check is still in progress or on error.
/// Returns `CiOutcome::Failure` when any check definitively failed.
/// Returns `CiOutcome::Success` when all checks passed.
pub fn query_live_ci_state(pr_number: u64) -> Result<CiOutcome> {
    let root = project_root()?;
    let pr_str = pr_number.to_string();

    let output = Command::new("gh")
        .current_dir(&root)
        .args(["pr", "view", &pr_str, "--json", "statusCheckRollup"])
        .output()
        .context("failed to execute gh pr view for CI state")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("gh pr view failed for #{pr_number}: {}", stderr.trim());
    }

    let raw = String::from_utf8_lossy(&output.stdout);
    let val: serde_json::Value =
        serde_json::from_str(&raw).context("failed to parse gh pr view JSON")?;

    let checks = val
        .get("statusCheckRollup")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();

    if checks.is_empty() {
        return Ok(CiOutcome::Pending);
    }

    let mut any_definitive_success = false;
    let mut any_failure = false;

    for check in &checks {
        // GitHub uses `conclusion` for completed runs (SUCCESS, FAILURE, SKIPPED, etc.)
        // and `state` as a fallback for older check-suite entries. When `conclusion`
        // is present as a JSON null (check still in progress), fall through to `state`.
        let conclusion_val = check.get("conclusion");
        let state_val = if conclusion_val.and_then(serde_json::Value::as_str).is_none() {
            check.get("state")
        } else {
            conclusion_val
        };
        let state = state_val.and_then(serde_json::Value::as_str).unwrap_or("UNKNOWN");

        match state.to_uppercase().as_str() {
            "SUCCESS" | "NEUTRAL" => {
                any_definitive_success = true;
            }
            "SKIPPED" => {
                // Path-conditioned skip — counts neither as success nor failure.
                // If ALL checks land here we return Skipped below.
            }
            "IN_PROGRESS" | "QUEUED" | "WAITING" | "PENDING" => {
                return Ok(CiOutcome::Pending);
            }
            _ => {
                any_failure = true;
            }
        }
    }

    if any_failure {
        return Ok(CiOutcome::Failure);
    }
    if any_definitive_success {
        return Ok(CiOutcome::Success);
    }
    // No failures, no pending, no explicit successes: all present checks were
    // SKIPPED (path-conditioning). Report as Skipped — not applicable.
    Ok(CiOutcome::Skipped)
}

// ---------------------------------------------------------------------------
// GitHub API helpers (via gh CLI)
// ---------------------------------------------------------------------------

/// Fetch label timeline events for a PR via the GitHub REST API.
///
/// Returns empty on failure so callers fall back gracefully.
fn fetch_label_timeline(pr_number: u64) -> Result<Vec<LabelEvent>> {
    let root = project_root()?;
    let endpoint = format!("repos/{{owner}}/{{repo}}/issues/{pr_number}/timeline?per_page=100");

    let output = Command::new("gh")
        .current_dir(&root)
        .args(["api", &endpoint, "--paginate"])
        .output()
        .context("failed to execute gh api timeline")?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let raw = String::from_utf8_lossy(&output.stdout);
    let events_json: Vec<serde_json::Value> = serde_json::from_str(&raw).unwrap_or_default();

    let label_events: Vec<LabelEvent> = events_json
        .into_iter()
        .filter(|e| e.get("event").and_then(serde_json::Value::as_str) == Some("labeled"))
        .filter_map(|e| {
            let label = e
                .get("label")
                .and_then(|l| l.get("name"))
                .and_then(serde_json::Value::as_str)?
                .to_string();
            let applied_at = e.get("created_at").and_then(serde_json::Value::as_str)?.to_string();
            let applied_by = e
                .get("actor")
                .and_then(|a| a.get("login"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown")
                .to_string();
            Some(LabelEvent { label, applied_at, applied_by })
        })
        .collect();

    Ok(label_events)
}

fn fetch_open_prs(pr_filter: Option<u64>) -> Result<Vec<OpenPr>> {
    let root = project_root()?;

    let output = Command::new("gh")
        .current_dir(&root)
        .args([
            "pr",
            "list",
            "--state",
            "open",
            "--limit",
            "500",
            "--json",
            "number,labels,headRefOid",
        ])
        .output()
        .context("failed to execute gh pr list")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("gh pr list failed: {}", stderr.trim());
    }

    let raw = String::from_utf8_lossy(&output.stdout);
    let prs_json: Vec<serde_json::Value> =
        serde_json::from_str(&raw).context("failed to parse gh pr list JSON")?;

    let prs: Vec<OpenPr> = prs_json
        .into_iter()
        .filter_map(|pr| {
            let number = pr.get("number").and_then(serde_json::Value::as_u64)?;
            if pr_filter.is_some_and(|filter| number != filter) {
                return None;
            }
            let labels = pr
                .get("labels")
                .and_then(serde_json::Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(|l| {
                            l.get("name").and_then(serde_json::Value::as_str).map(String::from)
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let head_ref_oid = pr
                .get("headRefOid")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string();
            Some(OpenPr { number, labels, head_ref_oid })
        })
        .collect();

    Ok(prs)
}

fn strip_label(pr_number: u64, label: &str) -> Result<()> {
    let root = project_root()?;
    let pr_str = pr_number.to_string();

    let output = Command::new("gh")
        .current_dir(&root)
        .args(["pr", "edit", &pr_str, "--remove-label", label])
        .output()
        .context("failed to execute gh pr edit")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("gh pr edit --remove-label failed for PR #{pr_number}: {}", stderr.trim());
    }

    Ok(())
}

fn post_comment(pr_number: u64, body: &str) -> Result<()> {
    let root = project_root()?;
    let pr_str = pr_number.to_string();

    let output = Command::new("gh")
        .current_dir(&root)
        .args(["pr", "comment", &pr_str, "--body", body])
        .output()
        .context("failed to execute gh pr comment")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("gh pr comment failed for PR #{pr_number}: {}", stderr.trim());
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Comment building
// ---------------------------------------------------------------------------

/// Build a structured reconciler comment for every label change.
///
/// Per memory `feedback_comment_trail_over_overwrite`: always post — the trail
/// teaches future agents. Uses `## Reconciler action` headers so agents can grep
/// for prior reconciliation events.
pub fn build_comment(
    contradictions: &[Contradiction],
    strips: &[String],
    run_timestamp: &str,
) -> String {
    if strips.is_empty() {
        return String::new();
    }

    let mut buf = String::new();

    for c in contradictions {
        if !strips.contains(&c.strip) {
            continue;
        }

        buf.push_str("## Reconciler action\n\n");
        buf.push_str(&format!("**Stripped**: `{}`\n", c.strip));
        buf.push_str(&format!("**Reason**: {}\n", c.reason));
        buf.push_str("**Evidence**:\n");
        buf.push_str(&format!("  - `{}` kept\n", c.keep));
        buf.push_str(&format!("  - `{}` stripped\n", c.strip));
        buf.push_str(&format!(
            "**Receipt**: `target/receipts/queue-reconcile.json` (run: {run_timestamp})\n"
        ));
        buf.push_str("\n---\n");
        buf.push_str("*Reconciler — sign-off-as-routing enforcement, fixed-point pass.*\n\n");
    }

    buf
}

// ---------------------------------------------------------------------------
// Receipt I/O
// ---------------------------------------------------------------------------

fn write_receipt(path: &PathBuf, receipt: &QueueReconcileReceipt) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create receipt directory: {}", parent.display()))?;
    }

    let json = serde_json::to_string_pretty(receipt).context("failed to serialize receipt")?;

    let mut file = fs::File::create(path)
        .with_context(|| format!("failed to create receipt file: {}", path.display()))?;
    file.write_all(json.as_bytes())
        .with_context(|| format!("failed to write receipt: {}", path.display()))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    fn make_pr(number: u64, label_names: &[&str]) -> OpenPr {
        OpenPr { number, labels: labels(label_names), head_ref_oid: format!("sha-{number}") }
    }

    fn make_event(label: &str, applied_at: &str) -> LabelEvent {
        LabelEvent {
            label: label.to_string(),
            applied_at: applied_at.to_string(),
            applied_by: "test-agent".to_string(),
        }
    }

    const TEST_TS: &str = "2026-04-27T00:00:00Z";

    // -----------------------------------------------------------------------
    // CI-specific detection (live state is ground truth)
    // -----------------------------------------------------------------------

    #[test]
    fn strips_needs_ci_fix_when_live_ci_is_green() {
        let pr = make_pr(1, &[CI_GREEN, NEEDS_CI_FIX]);
        let c = detect_contradictions(&pr, &[], CiOutcome::Success);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].strip, NEEDS_CI_FIX);
        assert!(c[0].reason.contains("SUCCESS"), "reason should mention SUCCESS");
    }

    #[test]
    fn does_not_strip_when_live_ci_is_failing() {
        // Live CI red: needs-ci-fix should stay, no strip.
        let pr = make_pr(1, &[CI_GREEN, NEEDS_CI_FIX]);
        let c = detect_contradictions(&pr, &[], CiOutcome::Failure);
        // No needs-ci-fix strip (only merge-ready would be stripped if present).
        assert!(c.iter().all(|x| x.strip != NEEDS_CI_FIX));
    }

    #[test]
    fn does_not_strip_when_live_ci_is_pending() {
        let pr = make_pr(1, &[CI_GREEN, NEEDS_CI_FIX]);
        let c = detect_contradictions(&pr, &[], CiOutcome::Pending);
        assert!(c.iter().all(|x| x.strip != NEEDS_CI_FIX));
    }

    // -----------------------------------------------------------------------
    // merge-ready + needs-ci-fix: live CI decides
    // -----------------------------------------------------------------------

    #[test]
    fn merge_ready_plus_needs_ci_fix_live_green_strips_needs_ci_fix_not_merge_ready() {
        let pr = make_pr(1, &[MERGE_READY, NEEDS_CI_FIX]);
        let c = detect_contradictions(&pr, &[], CiOutcome::Success);
        // Live CI green → strip needs-ci-fix, keep merge-ready.
        let strips: Vec<&str> = c.iter().map(|x| x.strip.as_str()).collect();
        assert!(strips.contains(&NEEDS_CI_FIX), "should strip needs-ci-fix");
        assert!(!strips.contains(&MERGE_READY), "should NOT strip merge-ready when CI is green");
    }

    #[test]
    fn merge_ready_plus_needs_ci_fix_live_red_strips_merge_ready() {
        let pr = make_pr(1, &[MERGE_READY, NEEDS_CI_FIX]);
        let c = detect_contradictions(&pr, &[], CiOutcome::Failure);
        let strips: Vec<&str> = c.iter().map(|x| x.strip.as_str()).collect();
        assert!(strips.contains(&MERGE_READY), "should strip merge-ready when CI is red");
        assert!(!strips.contains(&NEEDS_CI_FIX), "should NOT strip needs-ci-fix when CI is red");
    }

    #[test]
    fn merge_ready_plus_needs_ci_fix_pending_leaves_both() {
        let pr = make_pr(1, &[MERGE_READY, NEEDS_CI_FIX]);
        let c = detect_contradictions(&pr, &[], CiOutcome::Pending);
        let strips: Vec<&str> = c.iter().map(|x| x.strip.as_str()).collect();
        assert!(!strips.contains(&MERGE_READY), "should not strip merge-ready when pending");
        assert!(!strips.contains(&NEEDS_CI_FIX), "should not strip needs-ci-fix when pending");
    }

    // -----------------------------------------------------------------------
    // CiOutcome::Skipped — path-conditioned PRs (docs-only, CI-config-only)
    // -----------------------------------------------------------------------

    #[test]
    fn strips_needs_ci_fix_when_live_ci_is_skipped() {
        // Path-conditioned PR: all checks SKIPPED — gate is not applicable.
        // needs-ci-fix should be treated as stale and stripped.
        let pr = make_pr(1, &[CI_GREEN, NEEDS_CI_FIX]);
        let c = detect_contradictions(&pr, &[], CiOutcome::Skipped);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].strip, NEEDS_CI_FIX);
        assert!(
            c[0].reason.contains("SKIPPED") || c[0].reason.contains("path-conditioning"),
            "reason should mention SKIPPED or path-conditioning, got: {}",
            c[0].reason
        );
    }

    #[test]
    fn merge_ready_plus_needs_ci_fix_skipped_strips_needs_ci_fix_not_merge_ready() {
        // Path-conditioned PR with merge-ready: SKIPPED means gate inapplicable,
        // so strip needs-ci-fix but keep merge-ready (same as Success path).
        let pr = make_pr(1, &[MERGE_READY, NEEDS_CI_FIX]);
        let c = detect_contradictions(&pr, &[], CiOutcome::Skipped);
        let strips: Vec<&str> = c.iter().map(|x| x.strip.as_str()).collect();
        assert!(strips.contains(&NEEDS_CI_FIX), "should strip needs-ci-fix when all checks SKIPPED");
        assert!(
            !strips.contains(&MERGE_READY),
            "should NOT strip merge-ready when CI is SKIPPED (gate inapplicable)"
        );
    }

    #[test]
    fn idempotent_with_skipped_ci() {
        // Second pass on a path-conditioned PR must be a no-op.
        let mut state = labels(&[MERGE_READY, CI_GREEN, NEEDS_CI_FIX]);
        let pass1 = apply_pass(&mut state, &[], CiOutcome::Skipped);
        // Should strip needs-ci-fix only.
        assert!(pass1.contains(&NEEDS_CI_FIX.to_string()), "pass1 should strip needs-ci-fix");
        assert!(!pass1.contains(&MERGE_READY.to_string()), "pass1 should not strip merge-ready");
        let pass2 = apply_pass(&mut state, &[], CiOutcome::Skipped);
        assert!(pass2.is_empty(), "second pass must be no-op on path-conditioned PR");
    }

    #[test]
    fn clean_pr_with_all_skipped_ci_produces_no_strips() {
        // A PR with only merge-ready + ci-green and all CI SKIPPED: nothing to strip.
        let mut state = labels(&[MERGE_READY, CI_GREEN, DEEP_REVIEWED, DIFF_AUDITED]);
        let pass1 = apply_pass(&mut state, &[], CiOutcome::Skipped);
        assert!(pass1.is_empty(), "clean path-conditioned PR should produce no strips");
    }

    // -----------------------------------------------------------------------
    // merge-ready + non-CI needs-* (doctrine: strip merge-ready)
    // -----------------------------------------------------------------------

    #[test]
    fn merge_ready_plus_needs_builder_fix_strips_merge_ready() {
        let pr = make_pr(1, &[MERGE_READY, NEEDS_BUILDER_FIX]);
        let c = detect_contradictions(&pr, &[], CiOutcome::Pending);
        let strips: Vec<&str> = c.iter().map(|x| x.strip.as_str()).collect();
        assert!(strips.contains(&MERGE_READY));
    }

    #[test]
    fn merge_ready_plus_needs_deep_review_strips_merge_ready() {
        let pr = make_pr(1, &[MERGE_READY, NEEDS_DEEP_REVIEW]);
        let c = detect_contradictions(&pr, &[], CiOutcome::Pending);
        let strips: Vec<&str> = c.iter().map(|x| x.strip.as_str()).collect();
        assert!(strips.contains(&MERGE_READY));
    }

    #[test]
    fn merge_ready_alone_no_contradiction() {
        let pr = make_pr(1, &[MERGE_READY, CI_GREEN, DEEP_REVIEWED]);
        let c = detect_contradictions(&pr, &[], CiOutcome::Success);
        assert!(c.is_empty(), "no contradictions on clean merge-ready PR");
    }

    // -----------------------------------------------------------------------
    // Non-CI pairs: "later-applied wins" via timeline
    // -----------------------------------------------------------------------

    #[test]
    fn newer_signoff_strips_routing_when_timeline_available() {
        let pr = make_pr(1, &[DEEP_REVIEWED, NEEDS_DEEP_REVIEW]);
        let events = vec![
            make_event(NEEDS_DEEP_REVIEW, "2026-04-23T10:00:00Z"),
            make_event(DEEP_REVIEWED, "2026-04-26T22:00:00Z"),
        ];
        let c = detect_contradictions(&pr, &events, CiOutcome::Pending);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].strip, NEEDS_DEEP_REVIEW, "newer signoff wins");
    }

    #[test]
    fn newer_routing_strips_signoff_when_timeline_available() {
        let pr = make_pr(1, &[DEEP_REVIEWED, NEEDS_DEEP_REVIEW]);
        let events = vec![
            make_event(DEEP_REVIEWED, "2026-04-23T10:00:00Z"),
            make_event(NEEDS_DEEP_REVIEW, "2026-04-26T22:00:00Z"),
        ];
        let c = detect_contradictions(&pr, &events, CiOutcome::Pending);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].strip, DEEP_REVIEWED, "newer routing wins");
    }

    #[test]
    fn simultaneous_labels_prefer_signoff() {
        let pr = make_pr(1, &[DEEP_REVIEWED, NEEDS_DEEP_REVIEW]);
        let events = vec![
            make_event(DEEP_REVIEWED, "2026-04-26T22:00:00Z"),
            make_event(NEEDS_DEEP_REVIEW, "2026-04-26T22:00:03Z"), // 3s — within threshold
        ];
        let c = detect_contradictions(&pr, &events, CiOutcome::Pending);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].strip, NEEDS_DEEP_REVIEW, "simultaneous: signoff wins");
    }

    #[test]
    fn conservative_fallback_keeps_signoff_when_no_timeline() {
        let pr = make_pr(1, &[REVIEW_REVIEWED, NEEDS_BUILDER_FIX]);
        let c = detect_contradictions(&pr, &[], CiOutcome::Pending);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].keep, REVIEW_REVIEWED);
        assert_eq!(c[0].strip, NEEDS_BUILDER_FIX);
    }

    #[test]
    fn no_duplicate_needs_builder_fix_strip_when_both_signoffs_present() {
        // Both review-reviewed and maintainer-pr-reviewed with needs-builder-fix.
        // Should strip needs-builder-fix exactly once.
        let pr = make_pr(1, &[REVIEW_REVIEWED, MAINTAINER_PR_REVIEWED, NEEDS_BUILDER_FIX]);
        let c = detect_contradictions(&pr, &[], CiOutcome::Pending);
        let strips: Vec<_> = c.iter().filter(|x| x.strip == NEEDS_BUILDER_FIX).collect();
        assert_eq!(strips.len(), 1, "exactly one strip of needs-builder-fix");
    }

    // -----------------------------------------------------------------------
    // Idempotency
    // -----------------------------------------------------------------------

    /// Apply contradictions to a label set (simulate one reconciler pass).
    fn apply_pass(labels: &mut Vec<String>, events: &[LabelEvent], ci: CiOutcome) -> Vec<String> {
        let pr = OpenPr { number: 1, labels: labels.clone(), head_ref_oid: "sha".to_string() };
        let contradictions = detect_contradictions(&pr, events, ci);
        let strips: Vec<String> = contradictions.iter().map(|c| c.strip.clone()).collect();
        labels.retain(|l| !strips.contains(l));
        strips
    }

    #[test]
    fn idempotent_after_single_resolution() {
        let mut state = labels(&[MERGE_READY, NEEDS_BUILDER_FIX]);
        let pass1 = apply_pass(&mut state, &[], CiOutcome::Pending);
        assert!(!pass1.is_empty());
        let pass2 = apply_pass(&mut state, &[], CiOutcome::Pending);
        assert!(pass2.is_empty(), "second pass must be no-op");
    }

    #[test]
    fn idempotent_complex_state() {
        let mut state = labels(&[
            "in-review",
            MERGE_READY,
            CI_GREEN,
            NEEDS_CI_FIX,
            DEEP_REVIEWED,
            NEEDS_DEEP_REVIEW,
            "size/M",
        ]);
        let events = vec![
            make_event(DEEP_REVIEWED, "2026-04-26T22:00:00Z"),
            make_event(NEEDS_DEEP_REVIEW, "2026-04-23T10:00:00Z"),
        ];
        // Live CI green → strip needs-ci-fix; also deep-reviewed newer → strip needs-deep-review;
        // merge-ready + needs-ci-fix with live green: strip needs-ci-fix (not merge-ready).
        let pass1 = apply_pass(&mut state, &events, CiOutcome::Success);
        assert!(!pass1.is_empty());
        let pass2 = apply_pass(&mut state, &events, CiOutcome::Success);
        assert!(pass2.is_empty(), "second pass must be no-op, got: {pass2:?}");
    }

    #[test]
    fn idempotent_with_red_ci() {
        let mut state = labels(&[MERGE_READY, CI_GREEN, NEEDS_CI_FIX]);
        let pass1 = apply_pass(&mut state, &[], CiOutcome::Failure);
        // Should strip merge-ready.
        assert!(pass1.contains(&MERGE_READY.to_string()));
        let pass2 = apply_pass(&mut state, &[], CiOutcome::Failure);
        assert!(pass2.is_empty(), "second pass must be no-op");
    }

    #[test]
    fn clean_pr_produces_no_strips() {
        let mut state = labels(&[
            MERGE_READY,
            CI_GREEN,
            DEEP_REVIEWED,
            DIFF_AUDITED,
            REVIEW_REVIEWED,
            MAINTAINER_PR_REVIEWED,
        ]);
        let pass1 = apply_pass(&mut state, &[], CiOutcome::Success);
        assert!(pass1.is_empty(), "clean PR should produce no strips");
    }

    // -----------------------------------------------------------------------
    // Comment building
    // -----------------------------------------------------------------------

    #[test]
    fn build_comment_empty_when_no_strips() {
        let comment = build_comment(&[], &[], TEST_TS);
        assert!(comment.is_empty());
    }

    #[test]
    fn build_comment_structured_format() {
        let contradictions = vec![Contradiction {
            keep: NEEDS_CI_FIX.to_string(),
            strip: MERGE_READY.to_string(),
            reason: "test reason".to_string(),
        }];
        let strips = vec![MERGE_READY.to_string()];
        let comment = build_comment(&contradictions, &strips, TEST_TS);
        assert!(comment.contains("## Reconciler action"));
        assert!(comment.contains("**Stripped**: `merge-ready`"));
        assert!(comment.contains("**Reason**:"));
        assert!(comment.contains("**Evidence**:"));
        assert!(comment.contains("Reconciler"));
    }

    #[test]
    fn build_comment_has_one_section_per_strip() {
        let contradictions = vec![
            Contradiction {
                keep: CI_GREEN.to_string(),
                strip: NEEDS_CI_FIX.to_string(),
                reason: "ci fixed".to_string(),
            },
            Contradiction {
                keep: DEEP_REVIEWED.to_string(),
                strip: NEEDS_DEEP_REVIEW.to_string(),
                reason: "review done".to_string(),
            },
        ];
        let strips = vec![NEEDS_CI_FIX.to_string(), NEEDS_DEEP_REVIEW.to_string()];
        let comment = build_comment(&contradictions, &strips, TEST_TS);
        let count = comment.matches("## Reconciler action").count();
        assert_eq!(count, 2, "one section per strip");
    }

    // -----------------------------------------------------------------------
    // Receipt round-trip
    // -----------------------------------------------------------------------

    #[test]
    fn write_and_read_receipt_round_trip() -> color_eyre::eyre::Result<()> {
        let receipt = QueueReconcileReceipt {
            reconciled_at: "2026-04-27T00:00:00Z".to_string(),
            total_prs_scanned: 10,
            prs_with_contradictions: 2,
            total_labels_stripped: 3,
            applied: false,
            actions: vec![],
        };

        let tmp = tempfile::NamedTempFile::new()?;
        let path = tmp.path().to_path_buf();
        write_receipt(&path, &receipt)?;

        let raw = std::fs::read_to_string(&path)?;
        let loaded: QueueReconcileReceipt = serde_json::from_str(&raw)?;
        assert_eq!(loaded.total_prs_scanned, 10);
        assert_eq!(loaded.prs_with_contradictions, 2);
        assert_eq!(loaded.total_labels_stripped, 3);
        assert!(!loaded.applied);
        Ok(())
    }
}
