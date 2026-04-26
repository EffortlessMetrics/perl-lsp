# Ruleset Rollout Notes for Control-Plane Modernization

This document clarifies how rollout guidance maps onto current GitHub governance.

## Current repository reality

- The repository uses **GitHub Rulesets** (not classic branch protection).
- Required status checks are **not currently configured** at the Ruleset layer.
- Therefore, references to "required checks" in this rollout are future/conventional until Rulesets are updated.

## Rollout implications

- Build workflows as if they are required-style gates:
  - Always run on supported events.
  - No-op internally when conditions do not apply.
  - Avoid path filtering for required-style workflows.
- Include `pull_request`, `merge_group`, and `push` (`master`) triggers to preserve event parity.
- Use event-aware concurrency to prevent cross-event cancellation ambiguity.
- Use final aggregators to define one authoritative control-plane result per event.

## Merge-group as final pre-merge truth

`merge_group` should be treated as the final pre-merge truth source:

- It validates integrated state after queue composition.
- It is the right place to enforce final aggregator invariants.
- It should consume the same receipt evidence model as pull-request flows.

## State and configuration hygiene

- Prefer sharded configuration and derived state over shared mutable state files.
- Keep contract-bearing artifacts explicit (`.ci/receipts/...`) and runtime evidence ephemeral (`target/receipts/...`).
- Ensure rollout PRs use partial-close keywords (`Refs` / `Part of`) unless completion criteria are fully satisfied.
