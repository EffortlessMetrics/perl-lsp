# CI Hardening Status

_Last updated: 2026-04-30_

This page is the durable implementation snapshot for CI hardening work. It records what is
landed in `master`, what is partially landed, and what remains open so implementers can pick
next work without re-deriving state from issue threads.

## Landed (in `master`)

- **Label-trigger cancellation fixed**: core CI workflows no longer use PR `labeled` /
  `unlabeled` events that caused cancellation cascades during label churn.
- **Shared `pr-fast` runner**: PR smoke path runs shared `xtask` gates (`cargo xtask gates --tier pr-fast ...`) instead of drift-prone duplicated shell steps.
- **Per-gate timeout attribution**: `xtask gates` enforces/attributes timeouts at gate level so failures identify the exact gate lane that timed out (not only workflow timeout).
- **UX receipt classifier**: `cargo xtask ux-regression-receipt` emits structured failure classification and routing hints (landed via #7386 and #7394).
- **Status marker contract**: marker integrity checks for generated status docs are enforced with tests so marker drift fails early (pre-merge signal).

## Partial (landed core, follow-ups still open)

- **UX receipt classifier follow-through**: classifier is landed, but additional enrichment/wiring follow-ups remain (for example, complete artifact-path + full failure-path coverage where applicable).
- **Status semantics work**: baseline behavior exists, but policy normalization for all `SKIPPED` cases is not fully complete (see open expected-skip item below).

## Open items

- **Update-status streaming remains open**: tracked in **#7404**.
- **Expected-skip normalizer remains open**: normalize `SKIPPED` states into expected vs unexpected skip classes for reconciler-safe routing.
- **Review receipts / reconciler projection remains open**: project receipt conclusions into queue labels/state with explicit derivation rules.
- **Merge-train protocol remains open**: define and document batch/merge-train operating contract.

## Exact verification commands

Use these commands on current `master` while implementing CI hardening slices:

```bash
# Fast lane used by contributors and PR smoke
just pr-fast

# Shared gate runner directly (same tier used by PR smoke)
cargo xtask gates --tier pr-fast --receipt

# Local merge gate
just ci-gate

# Status marker contract + status regeneration integrity
cargo test -p xtask post_merge_status_regen_tests
```

## Known non-goals (for this wave)

- Do **not** treat global pre-push hooks as the primary CI control plane.
- Do **not** rely on admin-merge bypass as routine flow.
- Do **not** mark parser-ratchet evidence lanes as required before CI status semantics are stable.
- Do **not** collapse `SKIPPED` into pass/fail without policy context.

## Next-wave order (recommended)

1. **Close #7404 (update-status streaming)** so status evidence is continuously attributable.
2. **Land expected-skip normalizer** so CI state derivation is policy-correct.
3. **Project review receipts into reconciler labels/state** (clear mapping + precedence).
4. **Codify merge-train protocol** (batch size, rollback trigger, revalidation contract).

## Source references

- Active plan/backlog narrative: `docs/project/CI_HARDENING_NEXT_WAVES.md`
- CI workflow and runner structure: `.github/workflows/ci.yml`, `.github/ci-config.yml`
- Shared gates runner: `xtask/src/tasks/gates.rs`
- Status marker contract checks: `xtask/tests/post_merge_status_regen_tests.rs`
