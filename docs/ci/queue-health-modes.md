# Queue health modes

`cargo xtask queue health` emits a queue-health receipt used by orchestrators to decide whether merge drain can continue.

## Commands

```bash
cargo xtask queue health --receipt target/receipts/queue-health.json
cargo xtask queue health --fixture xtask/tests/fixtures/queue-health/master-green.json
```

- `--receipt <path>` writes the receipt JSON to disk (recommended path: `target/receipts/queue-health.json`).
- `--fixture <path>` loads deterministic input for validation/testing.

## Modes

### GREEN

- merge drain allowed
- cascade update allowed
- green-ci promotion allowed

### PENDING

- read-only review/design allowed
- no merge-ready promotion unless candidate current
- no broad cascade final labels

### RED

- freeze merge drain
- classify shared blocker
- allow master-fix and read-only review only

## Receipt fields

The receipt schema lives at `.ci/receipts/schemas/queue-health.schema.json` and requires:

- `master_sha`
- `mode`
- `allowed_lanes`
- `blocked_lanes`
- `reasons`
- `verdict`

## Guardrails

The queue health task is classification-only. It must not:

- mutate labels
- merge PRs
- cancel workflows
- dispatch agents directly
