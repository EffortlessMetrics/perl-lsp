# Parser Ratchet rollout

Issue: Refs #6847

## Rollout modes

Parser Ratchet is wired for a three-stage rollout controlled by the repository
variable `PARSER_RATCHET_MODE`.

- `scaffold`
  - `selected=false`
  - always records a receipt
  - `verdict=pass`
  - exits 0
- `canary` (**current rollout mode**)
  - `selected=true`
  - runs `.ci/parser-ratchet/profiles/pr.toml`
  - hard regressions are classified as `hard_regression` and reported as `would_fail`
  - non-hard failures are reported as `warn`
  - exits 0 (advisory)
- `enforce`
  - `selected=true` runs profile and classifies outcomes
  - hard regressions exit 1 with `verdict=fail`
  - `selected=false` (only scaffold mode) exits 0

`ruleset` is intentionally not automatic. After several clean `pull_request`,
`merge_group`, and `push` (`master`) runs in canary/enforce, update branch
protection/rulesets manually in a follow-up change.

## Workflow scope

`.github/workflows/parser-ratchet.yml` runs on:

- `pull_request`
- `merge_group`
- `push` to `master`

There are no path filters. Concurrency is event-aware to avoid stale duplicate
runs. Receipts are uploaded on every run (`if: always()`).

## PR profile

PR profile is intentionally constrained to the common/system corpus path and does
not run CPAN top-N workloads.

## Validation matrix

The workflow supports a simulation variable (`PARSER_RATCHET_SIMULATE`) to test
rollout behavior without introducing real regressions:

- `hard_regression`: simulates a hard regression (exit code 1)
- `warn_regression`: simulates a warning-grade failure
- `pass` (or unset): runs the real profile command

Expected outcomes:

1. `scaffold` + any simulation => `selected:false`, exit 0, `verdict:pass`
2. `canary` + `hard_regression` => exit 0, `verdict:would_fail`
3. `enforce` + `hard_regression` => exit 1, `verdict:fail`
