# Failure Classifier (pre-routing)

`green-ci` should run failure classification before applying `needs-ci-fix`.

## Purpose

This classifier distinguishes between:

- `PR_OWNED`
- `STALE_BASE`
- `MASTER_RED`
- `INFRA_FAILURE`
- `FLAKY`
- `UNKNOWN`

The goal is to avoid incorrectly routing ecosystem-wide incidents as PR-owned failures.

## CLI

```bash
cargo xtask failure-classifier --snapshot target/queue/snapshot.json --receipt target/receipts/failure-classifier.json
cargo xtask failure-classifier --fixture xtask/tests/fixtures/failure-classifier/master-red.json
```

## Input signals

- PR current head SHA
- PR check rollup / gate status
- Latest master status for the same gate when available
- Merge group status when available
- Known infra failure signatures
- Existing receipt artifacts and signatures

## Receipt output

The classifier emits a receipt matching:

- `.ci/receipts/schemas/failure-classifier.schema.json`

Fields:

- `check` (`Failure Classifier`)
- `signature`
- `affected_prs`
- `master_sha`
- `master_same_signature`
- `classification`
- `recommended_action`
- `confidence`
- `evidence`

## Routing table

- `PR_OWNED` -> `NEEDS_CI_FIX / builder`
- `STALE_BASE` -> `NEEDS_CASCADE_UPDATE`
- `MASTER_RED` -> master incident workflow (no PR-owned label)
- `INFRA_FAILURE` -> infra/tooling route
- `FLAKY` -> rerun/observe
- `UNKNOWN` -> human classification

## Guardrails

- The classifier does **not** apply labels.
- The classifier does **not** update branches.
- The classifier does **not** merge PRs.
- `PR_OWNED` requires current-head evidence (observed failing head must match PR current head and overlap changed/failed files).
