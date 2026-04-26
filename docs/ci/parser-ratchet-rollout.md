# Parser Ratchet rollout

This document defines the staged rollout for the `Parser Ratchet` workflow and
its expected behavior on `pull_request`, `merge_group`, and `push` to `master`.

## Rollout modes

The workflow reads `PARSER_RATCHET_MODE` from repository variables.
Selection can be forced with `PARSER_RATCHET_SELECTED=true|false` (default:
`auto`).

### 1) scaffold (default)

- `selected=false` by default.
- `verdict=pass`.
- Job exits `0`.
- Receipt is always uploaded.

Use this phase to prove signal plumbing (workflow trigger, receipt path,
artifact upload, summary format) without affecting mergeability.

### 2) canary

- `selected=true` (workflow override in canary mode; profile default remains
  `selected=false` for scaffold safety).
- PR profile runs.
- Hard regressions classify as `would_fail` (or `warn` for soft signals).
- Job exits `0`.
- Receipt is always uploaded.

Use this phase to gather evidence that classification is correct and stable
before turning on enforcement.

### 3) enforce

- `selected=true` runs PR profile and enforces hard regressions.
- Hard regressions exit `1`.
- `selected=false` still exits `0`.
- Receipt is always uploaded.

Only enable this mode after canary evidence demonstrates clean, stable runs.

### 4) ruleset

After several clean `pull_request`, `merge_group`, and `master` runs in canary
and enforce modes, update repository required checks/ruleset settings manually.

> Do not silently change ruleset settings in workflow code.

## Validation matrix

Use `PARSER_RATCHET_FIXTURE=regression` to force a known regression fixture.

1. `selected:false` path passes.
2. Canary + `selected:true` + fixture regression exits `0` and records
   `would_fail` or `warn` in the receipt.
3. Enforce + `selected:true` + fixture regression exits `1`.

## Non-goals

- No path filters on this workflow.
- No automatic GitHub ruleset mutation.
- No CPAN top-N inclusion in the PR profile.
