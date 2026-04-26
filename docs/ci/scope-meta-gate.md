# Scope Meta Gate

`scope-meta-gate` checks whether lane-selection logic changes accidentally drop lanes.

## Command

- `cargo xtask scope-meta-gate --base <sha> --head <sha> --receipt target/receipts/scope-meta-gate.json`
- `cargo xtask scope-meta-gate --fixture xtask/tests/fixtures/scope-meta-gate/parser-lane-dropped.json`

## Trigger files

This gate is intended for changes under:

- `xtask/src/tasks/ci_scope.rs`
- `xtask/src/tasks/gates.rs`
- `.ci/scope.d/**`
- `.ci/gates.d/**`
- `.github/workflows/**`
- `.ci/parser-ratchet/**`

## Receipt

Receipt fields:

- `old_decision`
- `new_decision`
- `changed_lanes`
- `verdict`

Schema: `.ci/receipts/schemas/scope-meta-gate.schema.json`.
