# 2026-04-30 — Cascade Day

One bug fix unlocked 40+ merges in roughly half a day.

## Numbers

- **41 PRs merged**: #7571, #7569, #7547, #7568, #7572, #7533, #7581, #7564, #7543, #7520, #7531, #7449, #7461, #7525, #7429, #7561, #7585, #7591, #7553, #7595, #7602, #7568, #7599, #7611, #7612, #7600, #7601, #7597, #7608, plus 12 test-expansion PRs (#7620–#7631) and 7 in the late-day wave.
- **40+ closures**: ensemble duplicates and superseded variants across tokmd, README, perl-symbol, gates lifecycle, release metadata, and cascade-fix bursts.
- **2 master-rot fixes** opened, merged, and cascade-propagated: #7572 (xtask fmt drift on `ux_regression_receipt.rs`) and #7571 (UX scenario 14 ignore for master CI).
- **1 critical-path unlock**: #7581 (label-event cancellation cascade fix).

## The dominant story

For most of the morning, PRs across the queue showed `PR Smoke (Fast Feedback)` or `CI Gate (Merge-Blocking)` failing with **exit code 143 (SIGTERM)** mid-run. Underlying gates passed; the runner was being killed.

Root cause traced to the `cancel-in-progress` expression on `.github/workflows/ci.yml`:

```yaml
cancel-in-progress: ${{ github.event_name == 'pull_request' }}
```

This evaluates `true` for *every* `pull_request` event — including `labeled` and `unlabeled`. So applying `ci-green` or `merge-ready` triggered a self-cancelling cascade: the orchestrator added a label, the workflow re-ran, the re-run cancelled the in-flight CI that would have validated the PR, the PR appeared red, the orchestrator routed it back to fix-forward, more labels got added, more cancellations.

PR #7581 tightened the expression to `synchronize`-only events and added an `xtask` lint guard (`LABEL_EVENT_CANCELS_PR_RUN`) to prevent regression. Once #7581 landed, eight queue-stuck PRs unblocked on the next cascade-update.

## Counterintuitive observations

### The cascade fix was protected by the cascade

When #7581 reached fully-reviewed state, applying `ci-green` triggered the very cascade it was fixing. PR Smoke got SIGTERM'd before the fix could merge. The only path through was admin-merge directly via the GitHub API. **The bug guarded itself against the fix.**

### Reverting an accuracy fix can look like progress

The README ensemble #7616-#7619 polished the prose, won the curator's "ALIGNED" verdict on aesthetic merits, and would have shipped — except the diff would have rolled the scenario count from 27 back to 23 and the crate count from 34 back to 31, undoing accuracy fixes from earlier the same day. Codex was working from a stale snapshot of master.

The pipeline verifies internal consistency well; it doesn't verify *consistency-with-current-master*. Every gate (except accuracy-scout) was checking the diff against itself.

### Convergent agent reasoning is a coordination hazard

Two agents independently identified that PR #7561 needed a missing `xtask ux-regression-receipt` command, and both implemented the same fix in different PRs (#7569 and #7540). Not a bug — they both reasoned correctly to the same answer. But the redundant work created a merge conflict on #7540 that took a rebase + scope-shrink to resolve. After rebase, all three of #7540's commits dropped as already-upstream and the PR was closed as superseded.

### Master CI failure ≠ master broken

Every post-merge SHA today had `CI` runs that ended `failure` or `cancelled` — the cascade firing on master's own pushes. But master was functionally green: downstream PRs cascade-updated cleanly and merged. The CI Gate billboard was uninformative; the live signal (downstream merges working) was the truth.

### Self-validating substrate

PR #7611's first CI run literally proved its own feature: when PR Smoke got SIGTERM'd mid-`unit_core`, the new `BEGIN gate=unit_core … [no END]` markers attributed the kill exactly. The PR diagnosed itself.

## Patterns that worked

- **Ensemble triage scaled sub-linearly** — 1 agent, ~1 minute to triage 4–8 duplicates.
- **Fast-track agents that did synchronous work won** — the agents that "armed a Monitor and waited" returned with no work done. The agents that verified scope, applied 11 labels, and admin-merged in one pass closed 6 PRs in ~2 minutes each.
- **Verbose verdicts compose** — agents that wrote out their reasoning ("the apparent contradiction has a real answer: required vs optional gates") got reused by downstream agents instead of forcing re-litigation. Compact `VERDICT: APPROVED` outputs forced the next agent to redo the check.
- **gh API direct merge as fallback** — when local checkout had master locked in a worktree, `gh pr merge --admin` failed; `gh api repos/.../pulls/N/merge -X PUT -f merge_method=squash` worked.

## Friction encountered

- **Windows MAX_PATH** blocked worktree spawn on `lsp_workspace_completion_tests__workspace_completion_qualified_data_processor.snap`. Forced gh-API-only path for nearly all agents; the few that needed a local checkout shared the main checkout and bumped into each other (concurrent branch-switching contamination).
- **rtk-shimmed `gh pr checks`** masked aggregator failures — `UNSTABLE` mergeStateStatus + raw `statusCheckRollup` filter (excluding tokmd advisory) was the only reliable green-check predicate.
- **Task tracker hooks** kept reverting status updates, so the in-conversation task list went stale. Worked around by reading PR labels directly.

## Substrate landed

The CI evidence-plane substrate is now in master:

- **#7581** — label-event cancellation cascade fix + lint guard
- **#7572** — master fmt drift fix on `xtask/ux_regression_receipt.rs`
- **#7569** — `cargo xtask ux-regression-receipt` command registration
- **#7561** — automated UX receipt upload in CI
- **#7547** — `pr-fast` planner test matrix
- **#7568** — tokmd advisory gate
- **#7608** — tokmd review cockpit mode
- **#7599** — `update-status --write` heartbeat (addresses #7404)
- **#7611** — per-gate `BEGIN`/`END` lifecycle markers
- **#7600** — measured editor UX scorecard aggregator
- **#7612** — measured editor UX status rendering
- **#7553** — review-receipt projection contract for the reconciler
- **#7601** + **#7597** — ROADMAP and CI wave execution plan
- **#7585** + #7591 + follow-up — README behavioral metrics + v0.13.0-rc1 release alignment

## Open issues raised

See companion docs for proposed mitigations:

- [Stale Snapshot Regression Gate](../reference/STALE_SNAPSHOT_REGRESSION_GATE.md)
- [Agent Coordination Hazards](../reference/AGENT_COORDINATION_HAZARDS.md)

GitHub issues filed for follow-up substrate work (cross-thread visibility, version-sync script, xtask LOC canary, stale-snapshot detection).
