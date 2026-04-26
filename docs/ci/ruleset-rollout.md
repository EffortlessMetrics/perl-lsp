# Ruleset Rollout Plan for Receipt-Driven CI

This plan documents how to align GitHub Rulesets with the receipt-driven control-plane architecture.

## Current repository reality

- The repository uses GitHub **Rulesets**, not classic branch protection.
- Required status checks are not currently configured at the ruleset layer.
- Therefore, references to “required checks” in this plan are a target/convention until rulesets are explicitly updated.

## Rollout principles

1. Required-style workflows must always run.
2. Required-style workflows must not rely on path filters.
3. Workflows should no-op internally when conditions do not apply.
4. Event coverage must include `pull_request`, `merge_group`, and `push` to `master`.
5. Final aggregators should provide the merge-facing truth signal.
6. `merge_group` is the final pre-merge validation truth.

## Event and concurrency guidance

- Trigger on:
  - `pull_request`
  - `merge_group`
  - `push` branches: `master`
- Use event-aware concurrency keys so:
  - PR updates supersede stale PR runs,
  - merge queue runs remain isolated,
  - push-to-master validation is preserved.

## Aggregation guidance

- Upstream jobs emit receipts and local statuses.
- A final aggregator evaluates all required invariants and emits a decisive result.
- Aggregator output is the principal merge-facing check signal.

## Label projection boundary

- Labels are UI-only projection.
- Labels must not be used as merge authority.
- State reconciliation from receipts is authoritative; labels mirror that state.

## Issue closeout hygiene

For rollout/scaffold PRs that only prepare infrastructure:

- Use `Refs #issue` or `Part of #issue`.
- Reserve `Closes`/`Fixes`/`Resolves` for PRs that fully satisfy acceptance criteria.
