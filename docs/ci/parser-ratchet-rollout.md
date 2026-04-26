# Parser Ratchet rollout

This workflow introduces the `Parser Ratchet` check with an explicit three-stage rollout so it can prove behavior before it becomes blocking.

## Scope

- Workflow: `.github/workflows/parser-ratchet.yml`
- Profile: `.ci/parser-ratchet/profiles/pr.toml`
- Triggers: `pull_request`, `merge_group`, and `push` on `master`
- No path filters
- Receipt artifact uploaded on every run

The PR profile intentionally uses the strict-clean manifest only and **does not** include CPAN top-N sweeps.

## Modes

The profile controls two things:

- `rollout.mode`: `scaffold`, `canary`, or `enforce`
- `rollout.selected`: whether the ratchet command executes

### 1) `scaffold`

- `selected=false` (default)
- No-op execution path
- `verdict=pass`
- Workflow exits `0`

Use this to wire job names, triggers, receipts, and artifacts safely.

### 2) `canary`

- `selected=true`
- Runs parser ratchet command against PR profile
- Hard regression classifies as `verdict=would_fail`
- Workflow still exits `0`

Use this to gather evidence that classification and receipts are correct before turning on enforcement.

### 3) `enforce`

- `selected=true`
- Hard regression classifies as `verdict=fail` and exits `1`
- `selected=false` still exits `0`

Only switch to this mode after clean canary evidence across PR, merge queue, and post-merge runs.

### 4) `ruleset`

After several clean runs in enforce mode, update repository protection/rulesets manually to require `Parser Ratchet`.

Do **not** auto-edit GitHub rulesets from CI.

## Validation matrix

Use these checks locally (or via CI fixture branches) before moving stages:

1. `selected:false` path passes.
2. Canary `selected:true` + fixture regression exits `0`, records `would_fail` (or warning classification).
3. Enforce `selected:true` + fixture regression exits `1`.

## Suggested promotion checklist

1. Merge scaffold wiring.
2. Flip profile to `mode="canary"`, `selected=true`.
3. Collect clean evidence over multiple `pull_request`, `merge_group`, and `push master` runs.
4. Flip to `mode="enforce"`, `selected=true`.
5. After additional clean evidence, make `Parser Ratchet` a required check in repository settings/rulesets.
