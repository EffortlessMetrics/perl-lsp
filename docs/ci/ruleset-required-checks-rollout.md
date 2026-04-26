# GitHub Ruleset required-status-checks rollout plan

This document defines how to move from convention/ops-enforced CI requirements to a GitHub Ruleset `required_status_checks` rule.

> Scope: planning and operational verification only. This plan does **not** auto-apply repository settings.

## Preconditions (must all be true)

Before any enforcement update is proposed, verify all of the following:

1. Final aggregator check names exist and are stable.
2. **Methodology Gate** reports reliably.
3. **Parser Ratchet** reports reliably.
4. `ci.yml` supports `merge_group` events.
5. `workflow-trigger-lint` passes.
6. No workflow that is expected to be required (including style/policy workflows) uses path filters that could skip reporting.

## Observation period (evidence gate)

Collect evidence before enabling required checks:

- At least **3 consecutive days** of clean reporting.
- At least **10 PRs** where final check names are reported consistently.
- At least **1 push to `master`** where final check names report consistently.
- If merge queue is enabled (or planned), at least **1 `merge_group` event** with consistent reporting before enforcement.

### Suggested observation log fields

- Date (UTC)
- Event type (`pull_request`, `push`, `merge_group`)
- PR number / commit SHA
- Observed final check names
- Missing checks (if any)
- Notes / remediation

## Ruleset update plan (manual, staged)

After preconditions and observation targets are met:

1. Add a GitHub Ruleset `required_status_checks` rule for **final check names only**.
   - Do not require intermediate job names.
   - Keep required list minimal and stable.
2. If merge queue is used, enable conservatively.
   - Start with merge queue batch size **1-2**.
3. Operate for **2-3 additional clean days**, then tune queue settings and required-check list only if needed.

## Rollback plan

If required-check enforcement causes merge blocking or noisy false negatives:

1. Remove the `required_status_checks` rule from the Ruleset.
2. Disable merge queue (if it was enabled during rollout).
3. Keep workflows running conventionally while investigating trigger/reporting gaps.

## Non-goals / safety constraints

- Do **not** mutate Rulesets automatically from repo scripts.
- Do **not** use `gh api` write operations (`PATCH`, `PUT`, `POST`, `DELETE`) in rollout helpers.
- Do **not** enable merge queue from automation in this repository.

## Read-only inspection helper

Use `scripts/github-ruleset-preview.sh` to inspect current Ruleset state in a read-only way.

