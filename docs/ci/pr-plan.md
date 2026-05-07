# PR Plan

Advisory CI economics forecast. Runs once per PR via
[`.github/workflows/pr-plan.yml`](../../.github/workflows/pr-plan.yml) and writes
`target/ci/ci-plan.json` + a step summary.

> Companion: [lem-budgeting.md](lem-budgeting.md), [labels.md](labels.md).

---

## What it does

1. Reads `policy/ci-budget.toml`, `policy/ci-lanes.toml`,
   `policy/ci-risk-packs.toml`.
2. Computes changed files via `git diff --name-only $BASE...$HEAD`.
3. Classifies the diff into risk packs (parser, LSP, retained-state, etc).
4. Selects lanes:
   - PR Plan, draft-guard, preflight, conflict-markers always run for non-docs PRs.
   - Rust default gates run for non-docs PRs.
   - Risk packs add their associated lanes.
   - Labels (`ci:*`, `full-ci`, etc) trigger label-gated lanes.
   - `full-ci` additionally pulls in `deep_lanes` for matched risk packs.
5. Sums `base_lem` (or `base_minutes × runner_multiplier`) per lane.
6. Emits the band: `default` / `elevated` / `high` / `over_ceiling`.

---

## Output schema

```json
{
  "schema_version": 1,
  "repo": "perl-lsp",
  "base_sha": "abc",
  "head_sha": "def",
  "labels": ["full-ci"],
  "posture": "rust",
  "budget": {
    "estimated_lem": 42.0,
    "band": "elevated",
    "default_limit_lem": 35,
    "elevated_limit_lem": 75,
    "hard_limit_lem": 125,
    "estimated_usd": 0.336
  },
  "changed": {
    "files": ["..."],
    "areas": ["parser"],
    "docs_only": false
  },
  "selection": {
    "risk_packs": ["parser"],
    "lanes": [
      {"id": "pr_smoke", "intent": "frontdoor mechanical proof",
       "runner": "ubuntu_24_04", "base_lem": 4,
       "default_pr": true, "blocking": true}
    ]
  },
  "warnings": []
}
```

---

## What it is not

- **Not blocking.** PR 04 lands the planner advisory-only. Hard ceiling guard
  arrives in PR 13.
- **Not a billing source.** `estimated_usd` is display only.
- **Not a runtime gate.** It does not trigger or skip downstream workflows. PR 12
  adds `xtask ci plan` which downstream lanes can read for `if:` conditions.

---

## Running locally

```bash
python3 scripts/ci/pr_plan.py \
  --base origin/master --head HEAD \
  --json-out target/ci/ci-plan.json
cat target/ci/ci-plan.json | jq .budget
```

---

## Roadmap

| PR | Change |
|---:|---|
| 04 | This file. Python prototype, advisory only. |
| 07 | Wire `ripr` selection and risk-pack outputs visible in summary. |
| 12 | Replace prototype with `cargo xtask ci plan`. |
| 13 | Add hard-ceiling guard above 125 LEM. |
| 16 | Use historical actuals to derive learned estimates. |
