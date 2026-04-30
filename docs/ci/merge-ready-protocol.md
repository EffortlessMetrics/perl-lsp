# Merge-ready receipt protocol

`merge-ready` is bound to a receipt for an exact PR head, exact base lineage, and exact gate graph version.

## Receipt

Receipt JSON uses `.ci/receipts/schemas/merge-readiness.schema.json` and includes:

- `check`: `merge-readiness`
- `schema_version`
- `event`
- `pr`
- `head_sha`
- `base_sha`
- `gate_graph_version`
- `required_checks`
- `review_evidence`
- `blocker_labels_absent`
- `verdict`
- `expires_when`

## Required checks source

This repository uses rulesets. Conventional required checks are read from `.ci/policies/required-checks.toml` first.

## Gate graph versioning

`gate_graph_version` is a deterministic hash over:

- `.ci/policies/required-checks.toml`
- `.ci/policies/**`
- `.ci/gates.d/**` (when present)
- required-style workflow files under `.github/workflows/**`

Inputs are normalized for line endings and sorted to exclude nondeterministic ordering.

## xtask commands

```bash
cargo xtask merge-ready emit --pr <N> --receipt target/receipts/merge-readiness.json
cargo xtask merge-ready verify --pr <N>
cargo xtask merge-ready verify --fixture xtask/tests/fixtures/merge-ready/valid.json
cargo xtask merge-ready reconcile --dry-run
cargo xtask merge-ready reconcile --apply
```

Verification statuses:

- `valid`
- `stale_head`
- `stale_base`
- `stale_gate_graph`
- `blocked`
- `missing`

## Rollout mode

Reconciliation defaults to advisory dry-run. Apply mode can be enabled explicitly.

## Merge-train operator protocol (bounded, no auto-merge)

This protocol is for batch/admin throughput without making `master` the first integration branch.
It is intentionally operator-driven and does **not** auto-merge PRs.

### Candidate requirements

Every PR candidate must be revalidated immediately before train planning with current API data
(`gh pr view --json ...` + latest-per-check filter):

- PR head SHA is captured and frozen in the plan.
- No active `needs-*` routing label.
- CI is green for the current head SHA, or green after expected-skip normalization (latest-per-check status only).
- PR is mergeable now, or intentionally sequenced behind a dependency in the same train.

### Train sizes

Use bounded cluster sizes to limit blast radius:

- **3 PRs**: overlapping/high-interaction changes.
- **5 PRs**: normal code changes.
- **10 PRs**: docs/leaf non-overlapping changes.

### Train check procedure

For each train, start from the latest known-green `master` SHA and execute in order:

1. Confirm `master` is currently green (latest CI run on `master` is successful).
2. Materialize an ordered candidate list with head SHA pins.
3. Apply/simulate each PR in order (local branch/rehearsal path is acceptable).
4. Run conflict-marker guard:
   - `just check-conflict-markers`
5. Run formatting gate:
   - `cargo xtask fmt --check`
6. Run fast merge gate:
   - `cargo xtask gates --tier pr-fast --base origin/master --receipt`

If simulation cannot be performed for a candidate (tooling/API limitation), mark the candidate as
"not simulated" and do not treat it as merge-approved.

### Stop conditions (hard halt)

Stop the train immediately when any of the following is observed:

- Conflict while applying candidate order.
- Candidate head SHA changed from planned value (stale SHA).
- Any required check fails.
- Unexpected skip state (non-normalized skip, missing required signal, or ambiguous check outcome).
- `master` turns red at any point in the train window.

### Required output (train receipt)

Each train execution must emit a markdown or JSON receipt containing:

- Baseline green `master` SHA.
- Candidate list with pinned head SHAs.
- Planned order (and dependency notes when applicable).
- Check commands executed and pass/fail verdict per step.
- Final train verdict (`ready`, `blocked`, `partial`) and explicit stop reason when blocked.

Minimum markdown template:

```md
## Merge Train Receipt
Baseline master SHA: <sha>

### Candidates
1. #1234 @ <sha> (mergeable: yes)
2. #1235 @ <sha> (depends on #1234)

### Checks
- conflict markers: pass/fail
- fmt: pass/fail
- pr-fast: pass/fail

### Verdict
- status: ready|blocked|partial
- stop reason: <none|conflict|stale-sha|failed-check|unexpected-skip|red-master>
```

### Non-goals / guardrails

- No automatic merges by default.
- No requirement for `--admin` merges.
- No CI policy weakening or bypass.
- No global hooks or cron formatting as primary enforcement.
