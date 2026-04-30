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
