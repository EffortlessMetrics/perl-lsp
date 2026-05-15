# Label projection from canonical queue state

`cargo xtask queue project-labels` projects UI labels from canonical PR state receipts.

## Modes

- Default mode is **dry-run**.
- Use `--apply` to reconcile labels against GitHub.
- `--apply` requires `GH_TOKEN` and `GITHUB_REPOSITORY`.

## Commands

```bash
cargo xtask queue project-labels \
  --state target/receipts/queue-state.json \
  --dry-run \
  --receipt target/receipts/label-projection.json

cargo xtask queue project-labels \
  --state target/receipts/queue-state.json \
  --apply \
  --receipt target/receipts/label-projection.json
```

## Input state shape

The command accepts either:

- a single object with `state`, `current_labels`, optional `pr_number`, and optional `has_merge_ready_receipt`; or
- an object containing a list under `pull_requests`, `prs`, `entries`, or `items`.

The projector does **not** infer canonical state from labels when a state receipt is present.

## Safety rules

- `MERGE_READY` projection is skipped unless `has_merge_ready_receipt` (or alias field) is true.
- Only configured projection labels are added/removed.
- The task does not create missing labels.

## Receipt fields

Receipts (written in both dry-run and `--apply` modes when `--receipt` is supplied) include per-PR entries with:

- `current_labels`
- `projected_apply`
- `projected_remove`
- `skipped`
- `reason`
- `dry_run`
- `verdict`
