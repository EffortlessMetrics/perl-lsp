# Workflow Trigger Policy (Conventional Required Checks)

`cargo xtask workflow-trigger-lint` enforces trigger and concurrency constraints for workflows listed in `.ci/policies/required-checks.toml`.

## Why this exists

GitHub Rulesets are in use, but `required_status_checks` are not yet configured as the authoritative source.
Until ruleset enforcement lands, `.ci/policies/required-checks.toml` is the conventional required-check list.

## Required workflow rules (`required = true`)

- Must include `pull_request` trigger.
- Must include `merge_group` trigger.
- Must include `push` trigger with `branches: [master]` (or equivalent containing `master`).
- Must not define `paths` or `paths-ignore` filters on the trigger.
- Must define top-level `concurrency.cancel-in-progress` exactly as:
  - `${{ github.event_name == 'pull_request' }}`
- Workflow file must exist.

## Commands

```bash
cargo xtask workflow-trigger-lint --policy .ci/policies/required-checks.toml --receipt target/receipts/workflow-trigger-lint.json
cargo xtask workflow-trigger-lint --fixture xtask/tests/fixtures/workflows/valid-required.yml
cargo xtask workflow-trigger-lint --format json
```

Fixture mode validates one workflow file and exits non-zero on violations.
