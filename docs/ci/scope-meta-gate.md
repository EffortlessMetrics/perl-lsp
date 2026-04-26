# scope-meta-gate

`scope-meta-gate` protects CI gate selection logic from silent regressions.

## Purpose

When a PR modifies scope/gate policy inputs, we compare **old** and **new** lane decisions.
If a lane (for example `parser_ratchet`) becomes unselected, the meta-gate fails.

Sensitive paths:

- `xtask/src/tasks/ci_scope.rs`
- `xtask/src/tasks/gates.rs`
- `.ci/scope.d/**`
- `.ci/gates.d/**`
- `.github/workflows/**`
- `.ci/parser-ratchet/**`

## Commands

```bash
cargo xtask scope-meta-gate --base <sha> --head <sha> --receipt target/receipts/scope-meta-gate.json
cargo xtask scope-meta-gate --fixture xtask/tests/fixtures/scope-meta-gate/parser-lane-removed.json
```

## Fixture format

```json
{
  "old_decision": { "selected_lanes": ["parser_ratchet", "test_scoped"] },
  "new_decision": { "selected_lanes": ["test_scoped"] }
}
```

## Receipt shape

The receipt contains:

- `old_decision`
- `new_decision`
- `changed_lanes` (`removed`, `added`, `unchanged`)
- `verdict` (`pass`, `warn`, `fail`)
- `advisory`

Use `.ci/receipts/schemas/scope-meta-gate.schema.json` for validation.
