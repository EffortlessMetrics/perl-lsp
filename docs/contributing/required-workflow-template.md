# Required Workflow Template (Final Aggregator Pattern)

Use this template when introducing stable required checks for branch/ruleset enforcement.

## Design goals

- The workflow itself always triggers on the relevant events.
- Internal matrix/component jobs may be skipped depending on scope.
- A final aggregator job runs with `if: always()`.
- The final job collects receipts from internal jobs.
- The final job publishes the stable check name that policies should enforce.
- Branch/ruleset enforcement should target only that final job, and only after an observation period.

## Skeleton

```yaml
name: example-ci

on:
  pull_request:
  push:
    branches: [master]

jobs:
  internal-job-a:
    runs-on: ubuntu-latest
    outputs:
      receipt_path: ${{ steps.write.outputs.receipt_path }}
    steps:
      - uses: actions/checkout@v4
      - name: Run scoped checks
        run: echo "selected=true"
      - id: write
        name: Write receipt
        run: |
          mkdir -p target/receipts/internal
          cat > target/receipts/internal/job-a.json <<'JSON'
          {"check":"job-a","required":true,"selected":true,"verdict":"pass","classification":"unknown"}
          JSON
          echo "receipt_path=target/receipts/internal/job-a.json" >> "$GITHUB_OUTPUT"

  final-example-check:
    if: always()
    needs: [internal-job-a]
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Aggregate receipts
        run: cargo xtask aggregate-receipts --check "Example Final Check" --inputs target/receipts/internal --output target/receipts/example-final.json
      - name: Finalize stable check
        run: cargo xtask finalize-check --receipt target/receipts/example-final.json
```

## Rollout notes

1. Ship the final aggregator job first and observe results for stability.
2. Keep existing internal job names unchanged during this framework phase.
3. Update branch protection/rulesets to require only the final stable check after observation.
