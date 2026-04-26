# Required Workflow Template (Final Aggregator Pattern)

Use this template when adding a stable required CI check name without exposing internal matrix/job churn.

## Design goals

- The workflow **always triggers** for pull requests.
- Internal jobs may be conditionally skipped based on path filters or scopes.
- A final aggregator job runs with `if: always()`.
- The final job reads subreceipts and writes one aggregator receipt.
- The final job is the stable check name for future branch/ruleset enforcement.
- Branch/ruleset enforcement should target this final check **only after an observation period**.

## Workflow skeleton

```yaml
name: test-gate

on:
  pull_request:
  merge_group:

jobs:
  internal-a:
    runs-on: ubuntu-latest
    steps:
      - name: Emit receipt
        run: |
          mkdir -p .ci/receipts/test-gate
          cat > .ci/receipts/test-gate/internal-a.json <<'JSON'
          {
            "check": "internal-a",
            "selected": true,
            "skipped": false,
            "required": true,
            "verdict": "pass",
            "classification": null
          }
          JSON

  internal-b:
    if: ${{ needs.preflight.outputs.run_internal_b == 'true' }}
    runs-on: ubuntu-latest
    steps:
      - name: Emit receipt
        run: |
          mkdir -p .ci/receipts/test-gate
          # write internal-b.json

  final-test-gate:
    if: always()
    needs: [internal-a, internal-b]
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Aggregate receipts
        run: |
          cargo xtask aggregate-receipts \
            --check "Test Gate" \
            --inputs .ci/receipts/test-gate \
            --output target/receipts/test-gate.json
      - name: Finalize stable check
        run: |
          cargo xtask finalize-check --receipt target/receipts/test-gate.json
```

## Rollout note

Do not point branch protection/rulesets to internal jobs. Observe the final check behavior first, then enforce only the stable final job name.
