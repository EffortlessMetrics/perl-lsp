# GitHub Ruleset required-status-checks rollout plan

This repository currently enforces required CI checks by convention and operations process. This document defines the **manual** rollout plan for a GitHub Ruleset `required_status_checks` rule once final check names have proven stable.

> Scope: planning + verification only. This document does **not** authorize automatic ruleset mutation.

## Preconditions before enforcement

Do not add `required_status_checks` until all of the following are true:

1. Final aggregator check names are present and stable (same names across PR, `merge_group`, and `master` contexts).
2. The Methodology Gate reports reliably for all required trigger types.
3. The Parser Ratchet reports reliably for all required trigger types.
4. `.github/workflows/ci.yml` includes `merge_group` support when merge queue is intended.
5. `workflow-trigger-lint` passes and confirms required workflows trigger in all expected contexts.
6. No workflow that would become required uses path filters that can skip required reporting.

## Observation period (before any ruleset change)

Collect evidence first. Keep convention-based enforcement in place during observation.

Minimum evidence window:

- At least **3 consecutive clean days** of reporting.
- At least **10 pull requests** observed with clean final checks.
- At least **1 push to `master`** with clean final checks.
- If merge queue is enabled or planned, at least **1 `merge_group` event** with clean final checks before enforcement.

Use `scripts/github-ruleset-preview.sh` plus workflow run history to capture and retain evidence snapshots.

## Ruleset update plan (manual, staged)

When preconditions and observation criteria are satisfied:

1. Update the repository ruleset manually in GitHub UI to add a `required_status_checks` rule.
2. Add only the **final aggregator check names** as required checks (do not require every internal job).
3. If merge queue is desired, enable it conservatively and start with batch size **1-2**.
4. Observe for **2-3 additional clean days** and tune queue settings only after stable behavior.

## Rollback plan

If required checks enforcement causes regressions or blocked merges:

1. Remove the `required_status_checks` rule from the ruleset.
2. Disable merge queue (if it was enabled for this rollout).
3. Keep CI workflows running under conventional enforcement while issues are triaged.

## Explicit non-goals

- No automatic ruleset mutation from repository scripts.
- No `gh api PATCH`/`PUT` ruleset writes.
- No automatic merge queue enablement from repository automation.
