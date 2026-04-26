# Workflow Security Policy Lint

`cargo xtask workflow-policy-lint` enforces repository workflow security and execution-separation rules.

## Commands

- `cargo xtask workflow-policy-lint --receipt target/receipts/workflow-policy.json`
- `cargo xtask workflow-policy-lint --fixture <workflow.yml>`

## Enforced policy

- Fails when a `pull_request_target` workflow checks out PR head.
- Fails when `pull_request` workflows request `contents: write` unless explicitly allowlisted.
- Fails on `permissions: write-all`.
- Fails when untrusted PR code can access secrets.
- Fails when required-style workflows omit `merge_group`.
- Fails when required-style workflows path-filter themselves.
- Fails when blanket `cancel-in-progress: true` applies to `main`/`master` push or `merge_group` truth runs.
- Warns on unpinned third-party actions.

## Receipt schema

Receipts are emitted as JSON and validated against:

- `.ci/receipts/schemas/workflow-policy.schema.json`
