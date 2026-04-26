# Queue health modes

`cargo xtask queue health` computes a queue-health receipt so orchestration can gate merge behavior without mutating labels or workflows.

## Commands

```bash
cargo xtask queue health --receipt target/receipts/queue-health.json
cargo xtask queue health --fixture xtask/tests/fixtures/queue-health/master-green.json
```

## Mode definitions

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

## Input model

The task accepts these inputs through a fixture JSON (`--fixture`) and falls back to conservative local defaults when no fixture is supplied:

- latest master CI state (`master_ci_state`: `green|pending|red`)
- pending/running checks (`pending_checks`, `running_checks`)
- failure classifier (`failure_classifier`) when available
- ruleset/gate policy (`gate_policy`) when available

## Receipt fields

Output receipt shape:

- `master_sha`
- `mode`
- `allowed_lanes`
- `blocked_lanes`
- `reasons`
- `verdict`

Schema: `.ci/receipts/schemas/queue-health.schema.json`.

## Non-goals

This command is intentionally read-only. It does **not**:

- mutate labels
- merge PRs
- cancel workflows
- dispatch agents directly
