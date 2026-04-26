# Ruleset required-status-checks rollout plan

This document describes a **manual, read-only-first rollout** for enforcing GitHub Ruleset `required_status_checks` in `EffortlessMetrics/perl-lsp`.

> Scope: planning and operator runbook only. No automation in this repository should mutate rulesets.

## Objectives

- Move from convention/ops-enforced required CI to Ruleset-enforced required final checks.
- Enforce only stable **final aggregator check names** (not per-job internals).
- Keep rollback simple and fast if required checks become flaky or missing.

## Preconditions (must all be true before enforcement)

1. Final aggregator check names exist and are documented in `.ci/policies/required-checks.toml`.
2. **Methodology Gate** reports reliably.
3. **Parser Ratchet** reports reliably.
4. `ci.yml` supports `merge_group` events (for merge queue correctness).
5. `workflow-trigger-lint` passes consistently.
6. No workflow that contributes to required-style enforcement uses path filters that can skip reporting.

## Observation period (prove reliability before enabling enforcement)

Collect evidence in PR notes or ops logs before changing rulesets.

Minimum observation threshold:

- At least **3 consecutive clean days** of reporting.
- At least **10 PRs** with complete final-check reporting.
- At least **1 push to `master`** with complete final-check reporting.
- If merge queue is enabled or planned, at least **1 `merge_group` event** with complete final-check reporting **before** enforcement.

“Clean reporting” means each expected final check reports a terminal status (success/failure) and is not missing/skipped due to trigger gaps.

## Ruleset update plan (manual, staged)

1. Preview current rulesets with:
   - `scripts/github-ruleset-preview.sh`
2. Update rulesets manually in GitHub UI (or equivalent ops channel), adding `required_status_checks` for **final check names only** from `.ci/policies/required-checks.toml`.
3. If using merge queue, enable conservatively:
   - Start with batch size **1–2**.
   - Observe queue behavior and CI reporting stability.
4. Tune only after **2–3 additional clean days** post-enforcement.

## Rollback plan (manual)

If required checks block merges due to reporting instability:

1. Remove `required_status_checks` from the affected ruleset.
2. Disable merge queue (if enabled).
3. Keep workflows running conventionally while root-causing missing/flaky reporting.
4. Re-enter observation period before re-enabling enforcement.

## Non-goals / guardrails

- Do **not** mutate rulesets from repository scripts.
- Do **not** use `gh api` `PATCH`/`PUT` from rollout tooling.
- Do **not** enable merge queue automatically from repository scripts.

## Operator checklist

- [ ] Preconditions satisfied.
- [ ] Observation thresholds met.
- [ ] Current ruleset state captured via preview script.
- [ ] Manual ruleset enforcement applied to final check names only.
- [ ] Post-change monitoring window completed.
- [ ] Rollback steps validated and documented.
