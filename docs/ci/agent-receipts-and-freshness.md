# Agent Receipts and Freshness

`agent_receipt` is durable evidence. It is not a direct state mutation.

## Freshness checks

Receipt validation enforces:

- `head_sha` mismatch => `stale`
- `base_sha` mismatch => `advisory`
- forbidden requested mutation => `rejected`
- newer receipt for same `task/lane/head` => `superseded`

## Idempotency markers

Recommended GitHub-visible prefixes for comments/check payloads:

- `[agent-lease:<lane>:<task_id>]`
- `[agent-receipt:<lane>:<task_id>:terminal]`

## Relationship to labels

Labels are UI projections from reconciled canonical state.
Agents should emit receipts and let reconciler logic project labels/routes.
