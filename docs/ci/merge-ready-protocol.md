# Merge-ready receipt protocol

`merge-ready` is now SHA-bound evidence, not a historical "was green" signal.

A PR is merge-ready only when one receipt simultaneously proves:

- the exact `head_sha` passed,
- against the exact `base_sha` candidate,
- under the exact `gate_graph_version` derived from CI policy inputs.

## Receipt source of truth

- Required checks are sourced from `.ci/policies/required-checks.toml`.
- Gate graph version includes deterministic content hashing of:
  - `.ci/policies/required-checks.toml`
  - `.ci/policies/**`
  - `.ci/gates.d/**` (if present)
  - workflow files referencing required-style check names

The hash excludes runtime metadata (timestamps, run IDs, nondeterministic ordering).

## Receipt schema

Schema: `.ci/receipts/schemas/merge-readiness.schema.json`

Required fields:

- `check` (`merge-readiness`)
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

## Commands

```bash
cargo xtask merge-ready emit --pr <N> --receipt target/receipts/merge-readiness.json
cargo xtask merge-ready verify --pr <N>
cargo xtask merge-ready reconcile --dry-run
cargo xtask merge-ready reconcile --apply
```

For deterministic fixture checks:

```bash
cargo xtask merge-ready verify --fixture xtask/tests/fixtures/merge-ready/valid.json
```

## Verification statuses

- `valid`
- `stale_head`
- `stale_base`
- `stale_gate_graph`
- `blocked`
- `missing`

## Reconciler rollout mode

Workflow: `.github/workflows/merge-ready-reconciler.yml`

- runs every 15 minutes, on push to `master`, and via manual dispatch
- defaults to advisory `--dry-run`
- `--apply` is opt-in through manual dispatch input
- apply mode is responsible for removing stale `merge-ready` and posting a reason comment with receipt evidence
