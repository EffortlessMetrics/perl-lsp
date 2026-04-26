# GitHub-state orchestration

This repo treats GitHub as the shared coordination substrate for disconnected maintainership.

## Model

- GitHub queue snapshot is the read model.
- Agent tasks define bounded allowed and forbidden mutations.
- Agent leases provide deterministic claim/winner behavior.
- Agent receipts are durable evidence.
- Reconciler projects labels/routes from canonical state.

## Deterministic lease winner

For a `(task_id, head_sha)` cohort:

1. Filter out expired leases.
2. Earliest `claimed_at` wins.
3. Ties resolve by lexicographic `lease_id`.

Losers must not mutate state.

## Freshness rules

- Head SHA mismatch marks lease/receipt stale.
- Base SHA mismatch is advisory unless policy says otherwise.
- Expired lease is stale/advisory.

These primitives are intentionally local validators and do not apply labels or merge.
