# Stale-Snapshot Regression Gate (proposed)

## The pattern

External AI agents (Codex, Hermes, Jules, Droid, Aider) generate PRs from prompts that include a *snapshot* of master at prompt time. By the time the PR opens, master has often moved — sometimes by several merges, sometimes by hours-old corrections to the very content the PR is rewriting.

The verification pipeline catches:
- Mechanical errors (accuracy-scout)
- External-claim drift (research-verifier)
- Approach issues (oppositional-planner, advocatus-diaboli)
- Structural fit (architecture-reviewer)
- Project alignment (maintainer-issue, maintainer-pr)
- Logic correctness (reviewer-deep)

The pipeline does **not** explicitly check:
- Does this PR remove content that was added on master within the last N hours/days?
- Does this PR change a value that was deliberately corrected in a recent merge?
- Is the PR's framing built on a master snapshot that's already stale?

## Concrete incident — 2026-04-30

The README ensemble #7616-#7619 (4-shot Codex burst) reached the curator with these "improvements":

| Field | Master state | PR change | Hours since master fix |
|---|---|---|---|
| Published crate count | `34 crates` (correct, post-#7591 follow-up) | `34` → `31` (regression) | ~3 |
| Editor UX scenarios | `27 scenario files` (matches actual file count) | `27` → `23` (regression) | ~3 |

The curator agent picked #7616 as ALIGNED based on aesthetic merits (hyphenation, terminology refinement) without flagging that two table values were being reverted. Only manual orchestrator review caught it; all 4 PRs were closed.

If the curator had auto-routed the winner to `needs-plan-review` and downstream agents had verified internal consistency only, the regression would have shipped.

## Why current gates miss it

- **Accuracy-scout** verifies the new value against external truth (filesystem count) — if the PR's stale value happens to match an outdated cached source like `editor_ux.json`, accuracy passes.
- **Maintainer-issue** checks alignment with project goals, not with last-edit context.
- **Reviewer-deep** verifies logic, not historical drift.
- **Diff-auditor** checks coherence and scope, not regression-against-recent-merges.

## Proposed gate

A pre-plan-review check that, for any PR touching files modified on master within the last N days, surfaces the recent commits as context to subsequent gates.

### Implementation sketch

```rust
// Pseudo-code for a new xtask: stale-snapshot-check
fn check_stale_snapshot(pr_number: u64) -> Result<StaleSnapshotReport> {
    let pr_files = gh::pr_files(pr_number)?;
    let mut hot_files = Vec::new();
    for file in pr_files {
        let recent_commits = git::log_for_file(&file, since: 7.days_ago())?;
        if !recent_commits.is_empty() {
            hot_files.push(StaleSnapshotEntry {
                path: file,
                recent_commits,
                base_sha: git::merge_base(pr_branch, master)?,
                staleness_hours: hours_since(recent_commits.last().date),
            });
        }
    }
    Ok(StaleSnapshotReport {
        hot_files,
        verdict: if hot_files.iter().any(|f| f.staleness_hours < 24) {
            "needs-context" // surface to plan-reviewer
        } else {
            "ok"
        }
    })
}
```

The output should be posted as a PR comment listing each hot file + the recent commits that touched it, so plan-reviewer (and downstream agents) can verify the PR doesn't undo recent intentional changes.

### Lighter-weight stop-gap

Until the xtask gate exists, ensemble-curator can compare the diff direction against `git log -- <file> --since=24h` and flag any PR that *removes* lines that were added recently. This catches the README revert pattern.

## Acceptance criteria

A scenario reproducing the README #7616 case (PR diff regresses a value corrected on master within last 24h) should:
1. Trip the stale-snapshot gate
2. Block `needs-plan-review` advancement until plan-reviewer explicitly acknowledges the regression in a comment

## Open questions

- What's the right staleness window? 24 hours is aggressive; 7 days catches more but adds noise.
- Should the gate fire on file-touching or on line-overlap? Line-overlap is more precise but harder to compute.
- How does this interact with intentional reverts (PRs whose job is to undo a recent merge)?

## Related

- Memory: `feedback_codex_stale_snapshot_regression.md`
- 2026-04-30 forensics: [`docs/forensics/2026-04-30-cascade-day.md`](../forensics/2026-04-30-cascade-day.md)
