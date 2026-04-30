# Failure Classifier

`cargo xtask failure-classifier` classifies CI failures before any routing automation (for example, before a bot would apply `needs-ci-fix`).

## Goals

- Separate true PR-owned failures from shared incidents.
- Detect stale-base failures when master is already green.
- Route infra and flaky failures away from PR-owned queues.
- Return `UNKNOWN` when evidence is insufficient, instead of over-classifying.

## Commands

```bash
cargo xtask failure-classifier --snapshot target/queue/snapshot.json --receipt target/receipts/failure-classifier.json
cargo xtask failure-classifier --fixture xtask/tests/fixtures/failure-classifier/master-red.json
```

## Input shape (snapshot/fixture)

Top-level fields consumed by the classifier:

- `pr.number`
- `pr.head_sha`
- `pr.master_sha`
- `pr.behind_master`
- `pr.changed_files[]`
- `pr_checks[]` (failed checks for PR context)
- `master_checks[]` (latest master checks for same gate)
- `merge_group_checks[]` (optional)
- `known_infra_signatures[]`
- `receipt_artifacts[]`
- `affected_prs[]` (optional cluster context)

Each check may include:

- `name` or `signature`
- `sha`
- `conclusion`
- `files[]`
- `flaky`
- `attempts`
- `recent_outcomes[]`

## Receipt output

The output receipt follows `.ci/receipts/schemas/failure-classifier.schema.json` and includes:

- `check`
- `signature`
- `affected_prs`
- `master_sha`
- `master_same_signature`
- `classification`
- `recommended_action`
- `confidence`
- `evidence`

## Routing map

- `PR_OWNED` → `NEEDS_CI_FIX / builder`
- `STALE_BASE` → `NEEDS_CASCADE_UPDATE`
- `MASTER_RED` → `master incident / no PR-owned label`
- `INFRA_FAILURE` → `infra/tooling route`
- `FLAKY` → `rerun/observe`
- `UNKNOWN` → `human classification`

## Guardrails

- Does **not** apply labels.
- Does **not** update branches.
- Does **not** merge PRs.
- Does **not** classify PR-owned without current-head failing evidence.
