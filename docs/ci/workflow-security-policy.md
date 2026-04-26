# Workflow security policy lint

`cargo xtask workflow-policy-lint` enforces workflow guardrails for PR safety and privilege separation.

## Commands

```bash
cargo xtask workflow-policy-lint --receipt target/receipts/workflow-policy.json
cargo xtask workflow-policy-lint --fixture xtask/tests/fixtures/workflow-policy/pull_request_read_only.yml
```

## Rules

The lint currently enforces:

- fails if a `pull_request_target` workflow checks out PR head code
- fails if a `pull_request` workflow uses `contents: write` without an explicit allowlist entry
- fails on `permissions: write-all`
- fails if untrusted PR code can access secrets in `pull_request_target`
- fails if required-style workflows (PR + push-to-main/master) omit `merge_group`
- fails if required-style workflows path-filter themselves
- fails if required-style workflows blanket `cancel-in-progress: true`
- warns on unpinned third-party actions

## Receipt

JSON receipts are emitted to the provided `--receipt` path using schema:

- `.ci/receipts/schemas/workflow-policy.schema.json`
