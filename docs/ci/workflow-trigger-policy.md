# Workflow trigger policy (conventional required checks)

`cargo xtask workflow-trigger-lint` validates trigger and concurrency requirements for workflows listed in `.ci/policies/required-checks.toml`.

## Why this exists

The repository currently has no GitHub ruleset `required_status_checks` configured. Until ruleset enforcement lands, `.ci/policies/required-checks.toml` is the conventional source of truth for which checks are treated as required.

## Required-workflow lint rules

For each `required = true` check, the workflow must:

- define `pull_request`
- define `merge_group`
- define `push` with `branches: [master]`
- avoid top-level or event-level `paths` / `paths-ignore`
- define event-aware concurrency:
  - `cancel-in-progress: ${{ github.event_name == 'pull_request' }}`
- exist at the configured path

## Commands

- Policy run with receipt:
  - `cargo xtask workflow-trigger-lint --policy .ci/policies/required-checks.toml --receipt target/receipts/workflow-trigger-lint.json`
- Fixture validation:
  - `cargo xtask workflow-trigger-lint --fixture xtask/tests/fixtures/workflows/valid-required.yml`
- JSON output:
  - `cargo xtask workflow-trigger-lint --format json`

## CI workflow

`.github/workflows/workflow-trigger-lint.yml` runs this lint on `pull_request`, `merge_group`, and `push` to `master` with no path filters. The job is currently advisory while legacy required workflows are migrated.
