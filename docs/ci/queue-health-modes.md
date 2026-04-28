# Queue Health Modes

`cargo xtask queue health` classifies master queue safety for orchestrator actions without mutating labels, merging PRs, cancelling workflows, or dispatching agents.

## Commands

```bash
cargo xtask queue health --receipt target/receipts/queue-health.json --fixture xtask/tests/fixtures/queue-health/master-green.json
cargo xtask queue health --fixture xtask/tests/fixtures/queue-health/master-pending.json
cargo xtask queue health --fixture xtask/tests/fixtures/queue-health/master-red.json
```

## Modes

- **GREEN**
  - Merge drain allowed
  - Cascade update allowed
  - Green-CI promotion allowed
- **PENDING**
  - Read-only review/design allowed
  - No merge-ready promotion unless candidate is current
  - No broad cascade final labels
- **RED**
  - Freeze merge drain
  - Classify shared blocker
  - Allow master-fix and read-only review only

## Receipt fields

Written JSON includes:

- `master_sha`
- `mode`
- `allowed_lanes`
- `blocked_lanes`
- `reasons`
- `verdict`

Schema: `.ci/receipts/schemas/queue-health.schema.json`.

## Input fixture shape

The fixture JSON accepts:

- `master_sha`
- `ci_state` (`green`, `pending`, `red`)
- `pending_checks` (array)
- `running_checks` (array)
- `failed_checks` (array)
- `failure_classifier.shared_blocker` (optional)
- `failure_classifier.summary` (optional)
- `gate_policy.pending_allows_merge_ready_if_candidate_current` (optional)
