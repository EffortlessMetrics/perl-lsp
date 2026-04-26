# GitHub queue snapshot

`cargo xtask queue snapshot` writes a stable JSON snapshot suitable for disconnected boxes.

## Commands

- `cargo xtask queue snapshot --out target/queue/open-prs.json`
- `cargo xtask queue snapshot --fixture <fixture.json> --out target/queue/open-prs.json`

## Snapshot properties

- `snapshot_id`, `captured_at`, repository metadata.
- `prs[]` with head/base SHA, labels, merge-state, rollup checks.
- Derived buckets:
  - `merge_ready`
  - `ci_green`
  - `needs_ci_fix`
  - `needs_builder_fix`
  - `needs_diff_fix`
  - `diff_audited_waiting_ci`
  - `stale_or_dirty`
  - `draft`
  - `blocked_unknown`

## Notes

- Comments are evidence, not authoritative CI truth.
- Head SHA + status-check rollup are the freshness anchors.
- This command is read-only and does not mutate labels/routes.
