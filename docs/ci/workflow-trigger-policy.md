# Workflow trigger policy lint

`cargo xtask workflow-trigger-lint` enforces trigger/concurrency policy for workflows listed in `.ci/policies/required-checks.toml` as `required = true`.

## Conventional source of truth

This repository currently treats `.ci/policies/required-checks.toml` as the conventional required-check list.

Why: GitHub Rulesets currently do not have `required_status_checks` configured, so classic branch protection is **not** the source of truth for this policy.

## Required-workflow rules

For each policy entry with `required = true`, the workflow must:

- Exist on disk.
- Include `pull_request` trigger.
- Include `merge_group` trigger.
- Include `push` trigger that covers branch `master`.
- Avoid top-level `pull_request.paths` and `pull_request.paths-ignore` filters.
- Configure event-aware concurrency:
  - `cancel-in-progress: ${{ github.event_name == 'pull_request' }}`

## Commands

- Full policy run + receipt:
  - `cargo xtask workflow-trigger-lint --policy .ci/policies/required-checks.toml --receipt target/receipts/workflow-trigger-lint.json`
- Fixture-based linting:
  - `cargo xtask workflow-trigger-lint --fixture xtask/tests/fixtures/workflows/missing-merge-group.yml`
- JSON output:
  - `cargo xtask workflow-trigger-lint --format json`

Receipt schema: `.ci/receipts/schemas/workflow-trigger-lint.schema.json`.
