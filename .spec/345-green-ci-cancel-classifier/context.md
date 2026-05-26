# Context: Green-CI Cancel Classification

## Problem

GitHub Actions concurrency groups cancel older jobs when a new push arrives. These cancellations have `conclusion: cancelled` with zero duration (`started_at == completed_at`). The green-ci agent currently treats ALL cancellations as RED failures, producing false negatives (green PR marked RED) when the concurrency kill triggers.

## Key Decision: 5-second threshold

- **INFRA-NOISE:** `started_at == completed_at` → zero-duration cancellation (concurrency group kill)
- **DEVELOPER-CANCEL:** `completed_at - started_at > 5s` → manual developer cancel via GitHub UI or API
- The 5s threshold is a safety margin; in practice, concurrency kills are instantaneous (microseconds).

## Alternatives rejected

1. **Ignore all cancellations:** Risk of missing developer-initiated cancels that warrant investigation.
2. **Parse check logs for "Cancellation reason" text:** GitHub API does not expose cancellation reason in check-runs endpoint; would require workflow run details API and post-hoc correlation.
3. **Consult workflow_run.event (check_suite.conclusion):** Unreliable; concurrency kills update check-runs independently of workflow_run status.

## Why this approach wins

- Uses only GitHub check-runs API (already queried in step 3)
- Pure duration logic, no string parsing or external correlation
- Preserves developer-initiated cancels (>5s) for investigation
- Simple, deterministic classification
- Zero false positives on actual CI failures

## Scope

- Modifies only skill markdown (green-ci-check.md, green-ci.md)
- No Rust code changes
- No new dependencies
- No test harness (skill markdown has no executable tests)
- Verification is empirical (next concurrency event proves it)

## Related

- Issue #345 (this issue)
- PR comment: plan-reviewer locked in spec; no red-tdd needed
- Blocked PRs: Any PR affected by concurrency cancellation false negatives (empirical validation on next event)
