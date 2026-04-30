# CI Hardening Status

> Human-owned snapshot for implementers. Update when CI hardening state changes.
> Last updated: 2026-04-30.

## Purpose

This page is the durable status checkpoint for CI hardening so builders do not need to reconstruct state from multiple PR threads.

## Landed items

- ✅ **Label-trigger cancellation fixed** — core CI workflows no longer restart from `labeled` / `unlabeled` churn; this was the cancellation-cascade fix.
- ✅ **Shared `pr-fast` runner landed** — PR Smoke uses shared `cargo xtask gates --tier pr-fast ...` execution rather than per-workflow shell duplication.
- ✅ **Per-gate timeout attribution landed** — `xtask gates` emits gate-level timeout attribution before workflow-level timeout swallows causality.
- ✅ **UX receipt classifier landed** — `xtask ux-regression-receipt` emits structured failure classes and repro metadata (#7386, #7394).
- ✅ **Status marker contract landed** — parser status marker contract is enforced as pre-merge discipline (see coverage added via #7162).

## Partial items

- 🟡 **UX receipt completeness** — classifier exists, but keep validating full failure-path coverage and any remaining receipt field wiring.
- 🟡 **Receipt-to-routing integration** — receipt quality improved, but full projection into reconciler routing remains incomplete.

## Open items

- 🔴 **Update-status streaming remains open** — tracked as **#7404**.
- 🔴 **Expected-skip normalizer remains open** — SKIPPED semantics still need explicit normalization (`expected_skip` vs `unexpected_skip`).
- 🔴 **Review receipts/reconciler projection remains open** — receipts are not yet fully projected into authoritative label/routing decisions.
- 🔴 **Merge-train protocol remains open** — batch validation / train operation is still a defined next-wave item.

## Exact verification commands

Run from repo root:

```bash
# Fast CI gate runner behavior and timeout attribution surface
cargo xtask gates --tier pr-fast --plan policy

# Full fast gate receipt (canonical pre-PR signal)
just pr-fast

# Queue-wide workflow trigger hygiene audit
cargo xtask ci-audit-workflows

# UX receipt classifier behavior and schema checks
cargo test -p xtask ux_regression_receipt

# Parser status marker contract coverage
cargo test -p xtask update_status
```

## Known non-goals (current wave)

- Do **not** treat workflow-level timeout as the primary timeout signal.
- Do **not** mark SKIPPED checks as pass/fail without policy context.
- Do **not** make Parser Ratchet required before CI status/receipt surfacing is stable.
- Do **not** rely on manual label hygiene where reconciler-derived truth should own state.

## Next-wave execution order

1. Close **#7404** (update-status streaming) so status evidence becomes continuously consumable.
2. Land expected-skip normalizer (`required_and_passed` / `expected_skip` / `unexpected_skip` / `pending` / `failed` / `stale`).
3. Project review receipts into reconciler routing outputs (authoritative projection path).
4. Publish merge-train / batch-validation protocol and operator checklist.

## Related planning doc

- Canonical backlog and wave sequencing: [../CI_HARDENING_NEXT_WAVES.md](../CI_HARDENING_NEXT_WAVES.md).
