# Merge-ready receipt protocol

`merge-ready` is bound to a SHA-bound receipt, not to a historical "once green" state.

## Invariant

A PR is merge-ready only when all of these are true for the same decision:

- exact `head_sha`
- exact `base_sha`
- exact `gate_graph_version`
- required checks from `.ci/policies/required-checks.toml` are successful
- review evidence exists
- blocker labels are absent

## Receipt

Schema: `.ci/receipts/schemas/merge-readiness.schema.json`

Key fields:

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
- `verdict` (`valid`, `stale_head`, `stale_base`, `stale_gate_graph`, `blocked`, `missing`)
- `expires_when`

## Commands

```bash
cargo xtask merge-ready emit --pr <N> --receipt target/receipts/merge-readiness.json
cargo xtask merge-ready verify --pr <N>
cargo xtask merge-ready reconcile --dry-run
cargo xtask merge-ready reconcile --apply
```

For fixture-based verification during development:

```bash
cargo xtask merge-ready verify --fixture xtask/tests/fixtures/merge-ready/valid.json
```

## Rollout

Reconciler defaults to advisory/dry-run mode.
Apply mode is opt-in through manual workflow dispatch input.
