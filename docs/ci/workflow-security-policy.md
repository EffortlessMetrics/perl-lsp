# Workflow security policy lint

`cargo xtask workflow-policy-lint` enforces GitHub Actions policy checks for
public PR safety boundaries.

## Commands

- `cargo xtask workflow-policy-lint --receipt target/receipts/workflow-policy.json`
- `cargo xtask workflow-policy-lint --fixture xtask/tests/fixtures/workflow-policy/pull_request_read_only.yml`

## Policy rules

The lint emits blocking errors for:

1. `pull_request_target` workflows that checkout `pull_request.head.*`.
2. `pull_request` workflows with `contents: write` unless explicitly allowlisted.
3. `permissions: write-all`.
4. Untrusted PR execution paths that access `secrets.*`.
5. Required-style workflows missing `merge_group` trigger.
6. Required-style workflows path-filtering `pull_request`.
7. `concurrency.cancel-in-progress: true` blanket cancellation.

The lint emits warnings for unpinned third-party actions (`uses: owner/repo@tag`).

## Required-style workflow marker

A workflow is treated as required-style only when it opts in:

```yaml
x-workflow-policy:
  required-style: true
```

This avoids forcing every workflow into merge-queue semantics.
