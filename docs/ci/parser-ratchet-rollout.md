# Parser Ratchet rollout (scaffold → canary → enforce)

This document defines the safe rollout for the `Parser Ratchet` workflow and
its PR profile.

## Scope

- Workflow: `.github/workflows/parser-ratchet.yml`
- PR profile: `.ci/parser-ratchet/profiles/pr.toml`
- Final check/job name: **Parser Ratchet**

The workflow runs on:

- `pull_request`
- `merge_group`
- `push` to `master`

No path filters are used. Receipt upload is `if: always()`.

## Modes

### 1) `scaffold` (default)

Use when wiring and validating the plumbing.

Expected behavior:

- `selected = false`
- workflow is a no-op with `verdict = "pass"`
- exit code = 0
- receipt is still emitted

This is the default in `.ci/parser-ratchet/profiles/pr.toml`.

### 2) `canary`

Use after scaffold is proven stable.

Expected behavior:

- `selected = true`
- runs PR profile (`manifest = .ci/common-corpus-manifest.txt`)
- hard regressions are recorded as `verdict = "would_fail"` (or `warn` for non-regression execution anomalies)
- workflow still exits 0

### 3) `enforce`

Enable only after canary evidence is consistently clean.

Expected behavior:

- `selected = true` + hard regression => exit 1 (`verdict = "fail"`)
- `selected = false` => exit 0 (safe no-op)

### 4) `ruleset`

After multiple clean runs across PR, merge queue, and post-merge (`master`),
update branch protection/ruleset configuration explicitly in a dedicated change.

Do **not** silently change required-check settings from this workflow change.

## Guardrails

- Do not enable hard enforcement immediately without canary evidence.
- Do not include CPAN top-N in the PR profile.
- Do not add path filters to this workflow.
- Do not auto-edit GitHub ruleset settings in CI.

## Validation matrix

Use fixture regression scenarios before promotion:

1. `selected=false` path passes.
2. `canary` + `selected=true` + fixture regression exits 0 and records `would_fail`/`warn`.
3. `enforce` + `selected=true` + fixture regression exits 1.
