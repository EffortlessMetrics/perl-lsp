# Required Workflow Final-Check Template

This template documents the final-aggregator pattern for required CI checks.

## Pattern

- The workflow itself should always trigger for the target events.
- Internal matrix or shard jobs may be skipped based on change scope.
- A final job must run with `if: always()`.
- The final job aggregates internal receipts using `cargo xtask aggregate-receipts`.
- The final job then executes `cargo xtask finalize-check`.
- The final job publishes the stable external check name consumed by branch/ruleset enforcement.

## Minimal YAML skeleton

```yaml
name: example-required-workflow

on:
  pull_request:
  push:
    branches: [main]

jobs:
  internal-job:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Produce receipt
        run: |
          mkdir -p target/internal-receipts
          echo '{"name":"internal-job","required":true,"selected":true,"verdict":"pass","classification":"unknown"}' > target/internal-receipts/internal-job.json

  final-check:
    name: Stable Final Check Name
    if: always()
    runs-on: ubuntu-latest
    needs: [internal-job]
    steps:
      - uses: actions/checkout@v4
      - name: Aggregate
        run: cargo xtask aggregate-receipts --check "Stable Final Check Name" --inputs target/internal-receipts --output target/receipts/stable-final-check.json
      - name: Finalize
        run: cargo xtask finalize-check --receipt target/receipts/stable-final-check.json
```

## Enforcement rollout

Observe the stable final check for a soak period first. After observation, point branch protection/ruleset enforcement to the final check name only (not internal matrix job names).

Workflow conversion for existing CI lanes is intentionally handled in follow-up PRs.
