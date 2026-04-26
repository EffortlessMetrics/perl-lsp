# Scope Meta-Gate

`cargo xtask scope-meta-gate` protects lane selection logic from silent narrowing.

## Commands

```bash
cargo xtask scope-meta-gate --base <sha> --head <sha> --receipt target/receipts/scope-meta-gate.json
cargo xtask scope-meta-gate --fixture xtask/tests/fixtures/scope-meta-gate/<fixture>.json
```

## Triggered on selector changes

- `xtask/src/tasks/ci_scope.rs`
- `xtask/src/tasks/gates.rs`
- `.ci/scope.d/**`
- `.ci/gates.d/**`
- `.github/workflows/**`
- `.ci/parser-ratchet/**`

## Algorithm

1. Compute old and new scope decisions (from SHA snapshots or fixture).
2. Compare selected lanes.
3. If a previously-selected lane is now unselected, return `fail`.
4. If only expansions are detected, return `warn`.
5. If no narrowing or expansion occurs, return `pass`.
6. Write a receipt containing `old_decision`, `new_decision`, `changed_lanes`, and `verdict`.

## Fixture contract

Fixture files are JSON objects with:

```json
{
  "old_decision": { "selected_lanes": ["parser_ratchet"] },
  "new_decision": { "selected_lanes": [] }
}
```

This enables deterministic unit/integration tests without comparing real git SHAs.
