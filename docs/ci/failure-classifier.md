# CI Failure Classifier

`cargo xtask failure-classifier` classifies a failing CI gate *before* any routing labels are applied.

## Why

Repeated failures can indicate different root causes:

- broken PR diff (`PR_OWNED`)
- stale base (`STALE_BASE`)
- red master (`MASTER_RED`)
- infrastructure outage (`INFRA_FAILURE`)
- intermittent flake (`FLAKY`)
- insufficient data (`UNKNOWN`)

Classifying first prevents accidental `needs-ci-fix` labeling when evidence points elsewhere.

## Commands

```bash
cargo xtask failure-classifier --snapshot target/queue/snapshot.json --receipt target/receipts/failure-classifier.json
cargo xtask failure-classifier --fixture xtask/tests/fixtures/failure-classifier/master-red.json
```

## Input signals

The classifier consumes these signals when available:

- PR current head SHA and gate status rollup
- latest master status for the same gate/signature
- merge-group status
- known infra signatures
- prior receipt artifacts

## Receipt shape

The output receipt contains:

- `check` (`Failure Classifier`)
- `signature`
- `affected_prs`
- `master_sha`
- `master_same_signature`
- `classification`
- `recommended_action`
- `confidence`
- `evidence`

See schema: `.ci/receipts/schemas/failure-classifier.schema.json`.

## Routing map

- `PR_OWNED` → `NEEDS_CI_FIX / builder`
- `STALE_BASE` → `NEEDS_CASCADE_UPDATE`
- `MASTER_RED` → open master incident (no PR-owned label)
- `INFRA_FAILURE` → infra/tooling route
- `FLAKY` → rerun + observe
- `UNKNOWN` → human classification

## Guardrail

The classifier does **not** call failures PR-owned unless current-head evidence is present.
