# Queue health modes

`cargo xtask queue health` computes a queue health receipt for orchestration safety decisions.

## Commands

```bash
cargo xtask queue health --fixture xtask/tests/fixtures/queue-health/master-green.json
cargo xtask queue health --fixture xtask/tests/fixtures/queue-health/master-green.json --receipt target/receipts/queue-health.json
```

## Mode semantics

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

The receipt JSON follows `.ci/receipts/schemas/queue-health.schema.json` and includes:

- `master_sha`
- `mode`
- `allowed_lanes`
- `blocked_lanes`
- `reasons`
- `verdict`

## Safety boundaries

This command is read-only and **must not**:
- mutate labels
- merge PRs
- cancel workflows
- dispatch agents directly
