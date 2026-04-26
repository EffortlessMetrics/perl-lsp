# Agent Leases, Receipts, and Freshness Rules

This document defines local validation rules for GitHub-backed maintainership receipts.

## Scope

The goal is to make disconnected workers safe by validating claim and output artifacts before any projection step.

This protocol does **not**:

- merge PRs,
- mutate labels directly,
- push branches,
- coordinate via local worktree directories.

## JSON shapes

### Task (`kind: agent_task`)

Required fields:

- `schema_version`
- `kind`
- `task_id`
- `snapshot_id`
- `lane`
- `pr`
- `head_sha`
- `base_sha`
- `allowed_mutations`
- `forbidden_mutations`
- `expires_at`

### Lease (`kind: agent_lease`)

Required fields:

- `schema_version`
- `kind`
- `task_id`
- `lease_id`
- `owner`
- `pr`
- `head_sha`
- `base_sha`
- `allowed_mutations`
- `expires_at`

### Receipt (`kind: agent_receipt`)

Required fields:

- `schema_version`
- `kind`
- `task_id`
- `lease_id`
- `lane`
- `pr`
- `head_sha`
- `base_sha`
- `verdict`
- `classification`
- `evidence`

## Validation and status rules

### Hard rejection

Reject receipt/lease when:

- schema is invalid,
- prohibited mutation appears in requested mutation set,
- task and receipt identity fields do not match (`task_id`, `pr`, `lane`).

### Stale/advisory classification

Mark stale or advisory when:

- `head_sha` differs from current PR head in snapshot,
- lease is expired,
- `base_sha` differs and lane policy does not allow base drift.

### Supersession

For the same tuple (`task_id`, `lane`, `head_sha`), newer receipts supersede older receipts.

### Duplicate lease resolution

For same (`task_id`, `head_sha`):

1. ignore expired leases,
2. earliest remaining lease wins,
3. tie-break with lexicographic `lease_id`.

## Command contract (proposed)

```bash
cargo xtask agent lease create --task <task.json> --out target/receipts/agent-lease.json
cargo xtask agent lease verify --lease <lease.json> --snapshot <snapshot.json>
cargo xtask agent receipt validate --receipt <receipt.json> --task <task.json> --snapshot <snapshot.json>
cargo xtask agent receipt status --receipt <receipt.json> --snapshot <snapshot.json>
```

## Queue snapshot prerequisite

All workers should derive freshness from a common queue snapshot shape that contains current PR head/base/status check rollups.

```bash
cargo xtask queue snapshot --out target/queue/open-prs.json
cargo xtask queue snapshot --fixture <fixture.json> --out target/queue/open-prs.json
```

## Safety invariant

Agents emit evidence; reconcilers derive canonical state.

Any stale, duplicate, or late worker output must degrade to no-op state projection rather than unsafe mutation.
