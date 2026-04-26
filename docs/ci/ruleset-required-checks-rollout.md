# GitHub Ruleset required-status-checks rollout plan

This runbook documents how to move from convention-enforced CI to an explicit GitHub Ruleset `required_status_checks` rule.

> Scope: **planning and verification only**. Do not apply Ruleset mutations from this document.

## Objective

Enable required status checks only after final check names are stable and observed as reliable across pull requests, merge groups (if enabled), and pushes to `master`.

## Preconditions (must all be true)

1. Final aggregator check names exist and are listed in `.ci/policies/required-checks.toml`.
2. `Methodology Gate` reports reliably.
3. `Parser Ratchet` reports reliably.
4. `.github/workflows/ci.yml` includes a `merge_group` trigger.
5. Workflow trigger linting passes (for example, a `workflow-trigger-lint` check in CI or an equivalent repository lint command).
6. No workflow that will become required uses path filters (`paths` / `paths-ignore`) that could suppress reporting.

## Observation period before enforcement

Collect evidence before updating Rulesets:

- At least **3 consecutive days** of clean reporting.
- At least **10 PRs** where all planned required checks report.
- At least **1 push to `master`** where all planned required checks report.
- If merge queue is enabled, at least **1 `merge_group` event** where all planned required checks report before enforcement.

Record evidence in the rollout issue (sample):

- Date window observed.
- Number of PRs sampled.
- Example PR links with full check reporting.
- Example `master` push link.
- Example `merge_group` run link (if applicable).

## Ruleset update plan (manual, conservative)

1. Open repository Rulesets and edit the active branch protection Ruleset manually.
2. Add `required_status_checks` using only the final aggregator check names from `.ci/policies/required-checks.toml`.
3. Do **not** include ephemeral or matrix-expanded job names.
4. If using merge queue, enable conservatively:
   - start with batch size **1–2**,
   - monitor queue health,
   - tune only after **2–3 additional clean days**.
5. Announce enforcement start time in the infra issue/PR so contributors know required checks are now platform-enforced.

## Rollback plan

If required-check enforcement causes merge blockage due to check non-reporting:

1. Remove the `required_status_checks` rule from the Ruleset.
2. Disable merge queue (if it was enabled for rollout).
3. Keep CI workflows running conventionally while fixing trigger/reporting reliability.
4. Re-enter observation mode and repeat enforcement only after the observation criteria are met again.

## Verification helpers

Use `scripts/github-ruleset-preview.sh` for a read-only snapshot of current Ruleset state and whether `required_status_checks` is configured.

```bash
bash scripts/github-ruleset-preview.sh
```

The script is intentionally read-only and must not apply any Ruleset changes.
