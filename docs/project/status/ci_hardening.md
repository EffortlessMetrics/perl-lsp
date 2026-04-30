# CI Hardening Status

> Durable status page for CI hardening work so implementers can build on current `master` state without re-deriving lane history.

## Scope

This page tracks CI hardening items around PR smoke/gates, status signaling, and receipt quality. It is intentionally short and action-oriented.

## Landed

1. **Label-trigger cancellation fixed** — cancellation behavior no longer aborts intended CI runs on label events (landed; see CI hardening wave notes in #7404 context).
2. **Shared `pr-fast` runner landed** — PR smoke uses shared `cargo xtask gates --tier pr-fast ...` execution instead of ad-hoc per-workflow wiring.
3. **Per-gate timeout attribution landed** — gate-level timeout signaling is emitted so failures can be attributed to a specific gate instead of generic timeout noise.
4. **UX receipt classifier landed** — `cargo xtask ux-regression-receipt` emits structured classification (`failure_class`, `panic_location`, `repro`, `first_failing_line`, `route`). Landed in #7386 and #7394.
5. **Status marker contract landed** — parser/status marker contract is enforced by tests to prevent drift in status update semantics.

## Partial

1. **UX receipt classifier coverage** — base classifier is landed (#7386, #7394), but full CI-path coverage and artifact-path completeness are still being finished.

## Open

1. **`update-status` streaming remains open** — tracked in **#7404**.
2. **Expected-skip normalizer remains open** — normalize skip semantics so status/check gates produce stable outcomes across lanes.
3. **Review receipts / reconciler projection remains open** — project receipt outcomes into labels/comments in a consistent reconciler path.
4. **Merge-train protocol remains open** — define and document batch-validation / merge-train operating rules.

## Exact verification commands

Run these from repo root:

```bash
just pr-fast
cargo xtask gates --tier pr-fast --profile ci
cargo xtask ux-regression-receipt --help
cargo test -p xtask
cargo test -p perl-lsp-rs update_status
```

If you are validating pre-merge state locally, use the canonical full gate receipt:

```bash
nix develop -c just ci-gate
```

## Known non-goals

- This page does **not** restate all CI architecture or lane definitions (see `docs/project/CI.md` and `docs/project/CI_TEST_LANES.md`).
- This page does **not** claim post-#7404 streaming behavior is shipped.
- This page does **not** define new policy for parser/documentation truth systems; it only records current CI hardening state.

## Next-wave order (recommended)

1. Land **#7404 update-status streaming**.
2. Land **expected-skip normalizer** so skip behavior is deterministic and machine-checkable.
3. Land **review receipt -> reconciler projection** to close the loop from gate output to review UX.
4. Land **merge-train protocol** (including operator playbook and failure handling).
5. Return to **UX receipt completion** (artifact-path + edge-path coverage) once lane semantics are stable.

## References

- #7386 — UX receipt classifier base wiring.
- #7394 — UX receipt classifier follow-up/hardening.
- #7404 — `update-status` streaming (open).
- `docs/project/CI_HARDENING_NEXT_WAVES.md` — detailed backlog/order context.
