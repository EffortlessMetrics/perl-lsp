# Merge-train protocol (operator-run, no auto-merge)

This protocol defines a **bounded merge-train check-and-merge flow** so batch/admin merges do not treat `master` as the first integration branch.

It is intentionally an **operator playbook**, not a merge bot:

- no automatic merging
- no admin bypass requirement
- no CI weakening
- reusable with existing `xtask`/`just` gates

## Inputs

For each candidate PR, collect:

- PR number
- head SHA (exact)
- labels/state snapshot
- mergeability status (clean/conflicting)
- latest CI verdict

Use existing queue tooling to gather queue context before train construction:

```bash
cargo xtask queue-snapshot --out target/queue-snapshot.json
cargo xtask queue-health --fixture xtask/tests/fixtures/queue-health/master-green.json # fixture mode example
cargo xtask merge-ready reconcile --dry-run
```

## Candidate requirements

A PR is train-eligible only if all required conditions hold at planning time:

1. **Current head SHA known** and recorded in the train plan.
2. **No active `needs-*` state** (including `needs-ci-fix`, `needs-builder-fix`, etc.).
3. **CI green**, or **expected-skip-normalized green** (path-conditioned skip that queue reconciler treats as merge-eligible).
4. **Mergeable now**, or explicitly marked as **intentionally ordered** behind a PR expected to resolve dependency/conflict ordering.

If any candidate violates these requirements, the train is not launched.

## Train sizing rules

Build trains using one of these bounded profiles:

- **3 PRs**: overlapping/high-risk cluster
- **5 PRs**: normal code cluster
- **10 PRs**: docs/leaf, non-overlapping cluster

Do not mix profiles inside one train run. If scope diverges, split into multiple trains.

## Train execution checks (required)

Start from **current green `master`** (or `main`, whichever is canonical in the repo mirror).

For the planned PR order:

1. Check out/update base to current green branch tip.
2. Apply/simulate each PR in order (local merge/cherry-pick simulation is sufficient).
3. Run conflict-marker check.
4. Run formatting check.
5. Run PR-fast gate with receipt.

Required commands:

```bash
# conflict markers
just check-conflict-markers

# fmt check
cargo xtask fmt --check

# shared PR-fast gate
cargo xtask gates --tier pr-fast --base origin/master --receipt
```

If a temporary integration branch/worktree is used, it must be disposable and never force-pushed over `master`.

## Stop conditions (hard fail)

Abort the train immediately on any of the following:

- merge/apply conflict
- stale candidate SHA (candidate head changed since plan creation)
- failed conflict-marker/fmt/pr-fast check
- unexpected skip outcome (skip not explained by expected path conditioning)
- base branch no longer green (new red `master` signal)

On stop, do not continue with remaining candidates.

## Required output: train receipt/summary

Each train run must emit a markdown summary (ticket comment, runbook entry, or PR-thread note) containing:

- candidate list with PR numbers + SHAs
- planned order
- executed checks and outcomes
- final verdict (`pass`/`stop`)
- explicit stop reason (if stopped)

Suggested template:

```md
## Merge Train Receipt

- Base: `master@<sha>` (green at plan time)
- Profile: `3-overlap | 5-normal | 10-docs-leaf`
- Candidates:
  1. #1234 @ <sha>
  2. #1235 @ <sha>
  3. #1236 @ <sha>
- Checks:
  - `just check-conflict-markers`: pass|fail
  - `cargo xtask fmt --check`: pass|fail
  - `cargo xtask gates --tier pr-fast --base origin/master --receipt`: pass|fail
- Verdict: pass|stop
- Stop reason: <none|conflict|stale-sha|failed-check|unexpected-skip|master-red>
```

## Recommended operator flow

1. Reconcile queue labels (`merge-ready`/`needs-*`) in dry-run mode.
2. Select candidates that satisfy requirements.
3. Choose train profile (3/5/10).
4. Build ordered plan with explicit SHAs.
5. Simulate/apply in a disposable integration branch/worktree from green `master`.
6. Run required checks.
7. If pass, perform manual merges in tested order.
8. Post train receipt summary.

This keeps `master` out of first-contact integration risk while preserving human control.
