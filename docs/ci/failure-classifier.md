# Failure Classifier

`cargo xtask failure-classifier` classifies failing CI runs before any routing automation applies labels like `needs-ci-fix`.

## Commands

```bash
cargo xtask failure-classifier --snapshot target/queue/snapshot.json --receipt target/receipts/failure-classifier.json
cargo xtask failure-classifier --fixture xtask/tests/fixtures/failure-classifier/master-red.json
```

## Input signals

- PR current head SHA
- PR status/check rollup for a gate
- Latest master status for the same gate (when available)
- Merge-group status (when available)
- Known infra signatures
- Optional receipt artifact hints (`infra`, `flaky`, evidence text)

## Output receipt fields

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

## Classifications and routing

- `PR_OWNED` → route to `NEEDS_CI_FIX / builder`
- `STALE_BASE` → route to `NEEDS_CASCADE_UPDATE`
- `MASTER_RED` → route to master incident handling (no PR-owned label)
- `INFRA_FAILURE` → route to infra/tooling triage
- `FLAKY` → rerun and observe
- `UNKNOWN` → human classification

## Guardrails

- Classifier **does not** apply labels.
- Classifier **does not** update branches.
- Classifier **does not** merge PRs.
- `PR_OWNED` requires current-head evidence (failing status + changed-file evidence).
