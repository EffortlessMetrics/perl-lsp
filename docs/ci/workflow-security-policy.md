# Workflow security policy lint

`cargo xtask workflow-policy-lint` enforces GitHub Actions safety invariants for PR execution boundaries.

## Commands

- `cargo xtask workflow-policy-lint --receipt target/receipts/workflow-policy.json`
- `cargo xtask workflow-policy-lint --fixture <workflow.yml>`

## Current enforced rules

Errors:
- `WF001`: fail if a `pull_request_target` workflow checks out PR head (`github.event.pull_request.head.*` / `github.head_ref`).
- `WF002`: fail if a `pull_request` workflow grants `contents: write` unless the workflow filename is explicitly allowlisted.
- `WF003`: fail on `permissions: write-all`.
- `WF004`: fail when untrusted PR checkout can run while non-`GITHUB_TOKEN` secrets are referenced.
- `WF005`: fail when a required-style workflow (push + pull_request) lacks `merge_group` unless allowlisted.
- `WF006`: fail when a required-style workflow path-filters itself.
- `WF007`: fail when blanket `cancel-in-progress: true` applies to `main`/`master` push or `merge_group` truth runs.

Warnings:
- `WF008`: warn on unpinned third-party actions (warning-first policy).

## Receipt

When `--receipt` is supplied, a JSON receipt is emitted and validated by `.ci/receipts/schemas/workflow-policy.schema.json`.
