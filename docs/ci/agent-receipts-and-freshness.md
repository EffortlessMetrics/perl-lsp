# Agent receipts and freshness

`cargo xtask agent` adds local validation primitives for GitHub-backed coordination.

## Commands

- `cargo xtask agent lease create --task <task.json> --out <lease.json> [--owner <owner>]`
- `cargo xtask agent lease verify --lease <lease.json> --snapshot <snapshot.json>`
- `cargo xtask agent receipt validate --receipt <receipt.json> --task <task.json> --snapshot <snapshot.json>`
- `cargo xtask agent receipt status --receipt <receipt.json> --snapshot <snapshot.json>`

## Behavior summary

- Forbidden mutations in receipts are rejected.
- Head SHA drift marks receipts stale.
- Base SHA drift is advisory.
- Expired task/lease timestamps are stale/advisory.

The output of each validator is a small JSON status payload intended for CI or reconcilers.
