# Label projection from canonical PR state

`cargo xtask queue project-labels` projects GitHub labels from canonical PR state receipts.

## Default mode

The command is **dry-run by default**. If `--apply` is not passed, it only prints (and optionally writes) the projection receipt.

```bash
cargo xtask queue project-labels \
  --state target/receipts/queue-state.json \
  --dry-run \
  --receipt target/receipts/label-projection.json
```

## Apply mode

`--apply` is explicit and required for mutation.

```bash
cargo xtask queue project-labels \
  --state target/receipts/queue-state.json \
  --apply
```

Apply mode requirements:

- `GH_TOKEN` must be present and non-empty.
- `pr_number` must exist in the state receipt.
- Missing labels are **not created** unless `--create-missing-labels` is passed.

## MERGE_READY safety

For `MERGE_READY`, projection is refused unless the state payload confirms a valid merge-readiness receipt (`merge_ready_receipt_valid`, `has_merge_ready_receipt`, or `merge_readiness_receipt.valid`).

## Config

Rules are loaded from `.ci/state/label-projection.toml` by default.

## Receipt schema

Projection receipt fields:

- `current_labels`
- `projected_apply`
- `projected_remove`
- `skipped`
- `reason`
- `dry_run`
- `verdict`

Schema file: `.ci/receipts/schemas/label-projection.schema.json`.
