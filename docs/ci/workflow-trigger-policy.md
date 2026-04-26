# Workflow trigger policy lint

`cargo xtask workflow-trigger-lint` validates required GitHub workflows against a
conventional policy list in `.ci/policies/required-checks.toml`.

Until repository rulesets enforce `required_status_checks`, this policy file is the
source of truth for which workflows must satisfy trigger guarantees.

## Rules for `required = true` workflows

- `on.pull_request` must exist.
- `on.merge_group` must exist.
- `on.push.branches` must include `master`.
- No `paths` or `paths-ignore` filters may be present on the top-level `on` block or
  required event triggers.
- Top-level concurrency must be event-aware:
  - `cancel-in-progress: ${{ github.event_name == 'pull_request' }}`
- The workflow file referenced by policy must exist.

## Commands

```bash
cargo xtask workflow-trigger-lint \
  --policy .ci/policies/required-checks.toml \
  --receipt target/receipts/workflow-trigger-lint.json

cargo xtask workflow-trigger-lint \
  --fixture xtask/tests/fixtures/workflows/missing-merge-group.yml

cargo xtask workflow-trigger-lint --format json
```

The JSON receipt schema is defined at
`.ci/receipts/schemas/workflow-trigger-lint.schema.json`.
