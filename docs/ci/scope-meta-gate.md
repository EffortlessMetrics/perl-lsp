# Scope Meta-Gate

`scope-meta-gate` protects lane-selection governance from silent narrowing.

## Command

```bash
cargo xtask scope-meta-gate --base <sha> --head <sha> --receipt target/receipts/scope-meta-gate.json
cargo xtask scope-meta-gate --fixture xtask/tests/fixtures/scope-meta-gate/remove-parser-lane-fail.json
```

## Trigger surface

Run this gate when any of these change:

- `xtask/src/tasks/ci_scope.rs`
- `xtask/src/tasks/gates.rs`
- `.ci/scope.d/**`
- `.ci/gates.d/**`
- `.github/workflows/**`
- `.ci/parser-ratchet/**`

## Verdict policy

- **fail**: protected scope files changed and at least one lane was removed.
- **warn**: protected scope files changed and scope only expanded.
- **pass**: no protected files changed, or protected files changed with no lane delta.

## Receipt schema

Schema path: `.ci/receipts/schemas/scope-meta-gate.schema.json`.

Receipt includes:

- `old_decision`
- `new_decision`
- `changed_lanes` (`removed`, `added`, `unchanged`)
- `verdict`
- `reason`
