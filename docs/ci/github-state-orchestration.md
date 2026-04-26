# GitHub-State Orchestration for Disconnected Maintainers

## Core model

GitHub is the shared distributed state. Agent local filesystems are not.

```text
multiple boxes / agents / maintainers
        ↓
read GitHub state
        ↓
claim or inspect bounded work
        ↓
emit GitHub-visible evidence
        ↓
reconciler derives canonical state
        ↓
labels/checks/comments/routes update
```

In this model:

- GitHub comments and check artifacts carry **leases** and **receipts**.
- Receipts are durable evidence, not direct state mutation.
- Labels are projected UI based on canonical reconciled state.
- PR head/base SHAs provide freshness boundaries.
- Rulesets and merge queue become final enforcement once enabled.

## Why this replaces local worktree coordination

A local worktree allocator assumes shared filesystem state and direct box-to-box coordination.
That does not hold for disconnected maintainership.

The repo control plane should therefore:

- avoid `.claude/worktrees` as the coordination substrate,
- use GitHub-visible claim/receipt records,
- make stale/duplicate work harmless by enforcing freshness at reconciliation.

## Protocol primitives

### 1. Queue snapshot

Each worker starts from a GitHub snapshot document (`snapshot_id`, captured PR metadata, status rollups).

Authoritative freshness data should come from:

- current `head_sha`,
- current `base_sha`,
- status check rollups.

Comments are evidence, not source of truth for CI status.

### 2. Task

A bounded task includes:

- `task_id`, `lane`, `pr`, `head_sha`, `base_sha`,
- `allowed_mutations` and `forbidden_mutations`,
- expiration timestamp.

### 3. Lease

A lease is a GitHub-visible claim over a task/head pair.

Idempotency prefix:

```text
[agent-lease:<lane>:<task_id>]
```

Deterministic winner rule:

1. earliest non-expired lease for the same `task_id` + `head_sha` wins,
2. ties break by lexicographic `lease_id`,
3. losers do not mutate.

### 4. Receipt

A receipt records lane output and evidence.

Idempotency prefix:

```text
[agent-receipt:<lane>:<task_id>:terminal]
```

Reconciler checks:

- receipt head/base freshness,
- lease validity and expiry,
- mutation policy compliance,
- supersession by newer receipt for same task/lane/head.

Only valid, current receipts can project labels/routes.

## Suggested phased rollout

1. **Phase 1**: snapshots + leases + receipts + projected labels by convention.
2. **Phase 2**: Methodology/Parser/CI gates always report machine-readable state.
3. **Phase 3**: merge-ready receipts required operationally.
4. **Phase 4**: required status checks enabled in GitHub rulesets.
5. **Phase 5**: merge queue becomes final landing truth.

## Operator loop

```text
1. Pull current GitHub state.
2. Build local snapshot.
3. Pick bounded work from canonical state.
4. Claim task in GitHub state.
5. Re-read GitHub state and verify claim still wins.
6. Do bounded work.
7. Verify head/base/state freshness before mutation.
8. Emit receipt.
9. Reconciler projects labels/routes.
```
