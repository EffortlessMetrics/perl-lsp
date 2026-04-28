# Agent receipts and freshness

`agent_receipt` freshness is evaluated against queue snapshot truth:

1. `head_sha` mismatch => stale.
2. `base_sha` mismatch => stale advisory.
3. Expired or missing lease => stale advisory.
4. Forbidden mutation request => reject.

`agent_lease` winner selection is deterministic:

- filter leases by same `task_id` + `head_sha`,
- pick earliest `created_at` among non-expired leases,
- tie-break by lexicographic `lease_id`.

The reconciler can consume these outcomes and project labels/routes while keeping direct agent mutations out of the state machine.
