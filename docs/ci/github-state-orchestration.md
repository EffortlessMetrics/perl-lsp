# GitHub-State Orchestration (Leases + Receipts)

This repository treats **GitHub state as the coordination substrate** for disconnected maintainership boxes.

## Protocol summary

1. Build a queue snapshot.
2. Create a bounded `agent_task`.
3. Emit a GitHub-visible `agent_lease` claim.
4. Re-verify lease winner deterministically.
5. Perform bounded work.
6. Emit `agent_receipt` evidence.
7. Reconciler derives canonical state and projects labels/routes.

## Command surface

```bash
cargo xtask agent lease create --task <task.json> --out <lease.json>
cargo xtask agent lease verify --lease <lease.json> --snapshot <snapshot.json>
cargo xtask agent receipt validate --receipt <receipt.json> --task <task.json> --snapshot <snapshot.json>
cargo xtask agent receipt status --receipt <receipt.json> --snapshot <snapshot.json>
```

## Deterministic duplicate lease winner

When multiple leases exist for the same `task_id + head_sha`, winner selection is deterministic:

- earliest non-expired lease wins
- tie-break by lexicographic `lease_id`

Losers are stale and MUST NOT mutate shared state.

## Scope boundaries

These primitives only validate local JSON protocol invariants and freshness against a snapshot.
They do not mutate labels, merge PRs, or push branches.
