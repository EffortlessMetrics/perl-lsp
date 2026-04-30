# CI Hardening Status (Control Plane Operability)

> Human-owned snapshot for CI operability work. Keep this focused on bounded, testable control-plane changes.

## Scope and Goal

This status page tracks the CI operability wave goal:

- make CI failures self-explaining;
- make merge-state interpretation reliable;
- avoid broad CI redesign or gate weakening.

Non-goals for this wave:

- no branch-protection changes;
- no bulk-close queue operations;
- no full merge bot;
- no global pre-push hook policy shift.

## Landed

- **UX label-trigger cancellation fix** landed (`#7378`).
- **Shared `pr-fast` runner flow** is in use via `cargo xtask gates --tier pr-fast --base origin/master --receipt`.
- **Per-gate timeout attribution** landed (`#7398`) with timeout classification fields in gate receipts.
- **UX regression receipt classifier CLI** landed (`#7386`) via `cargo xtask ux-regression-receipt`.
- **Parser/status marker contract** landed (`#7162`).
- **PR Smoke timeout/budget false-fail fix** landed (`#7403`).

## Partial / Needs Hardening

- **Per-gate timeout behavior proof**: add synthetic timeout tests to prevent regression to opaque job-level termination failure modes.
- **UX receipt workflow integration**: ensure CI captures UX logs, generates receipt artifacts, and publishes actionable pointers in summaries.

## Open

- **`update-status --write` inactivity timeout** remains open (`#7404`): stream progress or tee output so long status-generation steps cannot go silent for ~5+ minutes.
- **Expected-skip status normalizer** remains open: distinguish `expected_skip` vs `unexpected_skip` for merge/readiness logic.
- **Review receipt projection into reconciler** remains open (`#7287`): reconciler owns label projection from current-SHA review receipts.
- **Merge-train protocol/planner** remains open (`#7288`): bounded train checks before batch merges.

## Verification Commands

Use these commands for reproducible checks during this wave:

```bash
# Status generation and drift
cargo run -p xtask -- update-status --write
cargo xtask update-status --check
cargo xtask update-status --write --only parser

# Gate runner / receipts
cargo xtask gates --tier pr-fast --base origin/master --receipt
cargo test -p xtask

# UX receipt classifier
cargo xtask ux-regression-receipt --input <log> --receipt target/receipts/ux-regression.json --sha <sha>
```

## Recommended Next-Wave Order

1. `#7404` update-status streaming
2. gate timeout synthetic tests (`#7398` hardening)
3. UX workflow receipt wiring (`#7386` integration)
4. `pr-fast` planner tests hardening
5. expected-skip normalizer
6. review-receipt projection (`#7287`)
7. merge-train protocol (`#7288`)

## Operator Notes

- Prefer one scoped change per PR.
- Preserve current gate strictness; improve observability and diagnosis.
- Keep `target/receipts/` as the canonical artifact location for CI debugging receipts.
