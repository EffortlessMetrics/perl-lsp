# Label projection from canonical queue state

`cargo xtask queue project-labels` treats labels as a projection of canonical queue state receipts.

## Commands

```bash
cargo xtask queue project-labels \
  --state target/receipts/queue-state.json \
  --dry-run \
  --receipt target/receipts/label-projection.json

cargo xtask queue project-labels \
  --state target/receipts/queue-state.json \
  --apply
```

## Behavior

- Default mode is dry-run (no GitHub mutations).
- `--apply` is required for label mutations.
- `--apply` requires `GH_TOKEN` in the environment.
- `MERGE_READY` will not project `merge-ready` unless `merge_ready_receipt_valid` is `true` in state input.
- Label creation is disabled by default; use `--create-labels` to opt in.

## State input (initial fixture shape)

```json
{
  "canonical_state": "NEEDS_BUILDER_FIX",
  "current_labels": ["review-reviewed", "ci-green", "merge-ready"],
  "pull_request": {
    "owner": "EffortlessMetrics",
    "repo": "perl-lsp",
    "number": 6853
  },
  "merge_ready_receipt_valid": false
}
```

## Receipt output

See schema: `.ci/receipts/schemas/label-projection.schema.json`.
