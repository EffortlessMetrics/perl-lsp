# GitHub queue snapshot

`cargo xtask queue snapshot` captures a stable JSON snapshot of open PR state.

## Fields

- top-level: `snapshot_id`, `captured_at`, `repository`, `default_branch`, `master_sha`, `ruleset_summary`
- per PR: `number`, `title`, `head_sha`, `base_sha`, `is_draft`, `merge_state_status`, `labels`, `status_check_rollup`, `updated_at`, `author`, `review_decision`
- derived buckets: `merge_ready`, `ci_green`, `needs_ci_fix`, `needs_builder_fix`, `needs_diff_fix`, `diff_audited_waiting_ci`, `stale_or_dirty`, `draft`, `blocked_unknown`

## Behavioral rules

- comments are evidence, not authoritative CI state,
- current `head_sha` and status check rollup are freshness truth,
- labels are projected UI state, not canonical state.
