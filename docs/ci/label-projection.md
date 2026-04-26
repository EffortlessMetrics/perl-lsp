# Label projection from canonical PR state

`cargo xtask queue project-labels` projects GitHub labels from canonical queue state receipts.

## Design

- Labels are projections of canonical state; labels are not treated as source-of-truth state.
- Default mode is dry-run.
- Apply mode is opt-in with `--apply` and requires `GH_TOKEN`.
- `merge-ready` is never applied unless a valid merge-ready receipt is present in state input.

## Commands

```bash
cargo xtask queue project-labels --state target/receipts/queue-state.json --dry-run --receipt target/receipts/label-projection.json
cargo xtask queue project-labels --state target/receipts/queue-state.json --apply
```

## Input state shape (minimum)

```json
{
  "canonical_state": "NEEDS_BUILDER_FIX",
  "current_labels": ["review-reviewed", "ci-green", "merge-ready"],
  "pr": {
    "number": 1234,
    "repo": "EffortlessMetrics/perl-lsp"
  },
  "merge_ready_receipt": {
    "valid": false
  }
}
```

## Receipt fields

- `current_labels`
- `projected_apply`
- `projected_remove`
- `skipped`
- `reason`
- `dry_run`
- `verdict`

Schema: `.ci/receipts/schemas/label-projection.schema.json`.
