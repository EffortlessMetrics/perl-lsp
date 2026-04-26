# Required Workflow Template (Stable Final Check Pattern)

Use this template when converting a CI workflow to stable required checks.

## Principles

1. The workflow itself should always trigger.
2. Internal jobs may be skipped based on scope/routing.
3. The final job must use `if: always()`.
4. The final job aggregates sub-job receipts into one final receipt.
5. The final job owns the stable check name used for required checks.
6. Branch/ruleset enforcement should point to the final job **only after** an observation window.

## Template

```yaml
name: ci-example

on:
  pull_request:
  push:
    branches: [main]

jobs:
  internal-a:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: echo '{"name":"internal-a","required":true,"selected":true,"verdict":"pass"}' > .ci/receipts/incoming/internal-a.json

  internal-b:
    runs-on: ubuntu-latest
    if: ${{ false }}
    steps:
      - run: echo "skipped"

  final-check:
    name: Stable Final Check Name
    runs-on: ubuntu-latest
    if: always()
    needs: [internal-a, internal-b]
    steps:
      - uses: actions/checkout@v4
      - run: |
          cargo xtask aggregate-receipts \
            --check "Stable Final Check Name" \
            --inputs .ci/receipts/incoming \
            --output target/receipts/stable-final-check.json
      - run: cargo xtask finalize-check --receipt target/receipts/stable-final-check.json
```

## Rollout guidance

- Do not refactor every workflow/job name in one PR.
- Convert one workflow lane at a time.
- Keep legacy internal job names until the final-check pattern is proven in production.
- After observation confirms stability, point branch/ruleset enforcement to the final check name only.
