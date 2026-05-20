# PR Plan

Advisory CI economics forecast. Runs once per PR via
[`.github/workflows/pr-plan.yml`](../../.github/workflows/pr-plan.yml) and writes
`target/ci/ci-plan.json` + a step summary.

> Companion: [lem-budgeting.md](lem-budgeting.md), [labels.md](labels.md).

---

## What it does

1. Reads `policy/ci-budget.toml`, `policy/ci-lanes.toml`,
   `policy/ci-risk-packs.toml`, and `policy/trust-lanes.toml`.
2. Computes changed files via `git diff --name-only $BASE...$HEAD`.
3. Classifies the diff into risk packs (parser, LSP, retained-state, etc).
4. Selects lanes from three independent sources, recording the **origin** of
   each selection:
   - `default-pr` — every lane with `default_pr = true` is selected for
     non-docs PRs (`docs_gate` is selected only for docs-only PRs).
   - `risk-pack:<id>` — lanes pulled in by a matched risk pack.
   - `label:<name>` — lanes pulled in by labels (`ci:*`, `full-ci`, etc).
   - `deep-lane:full-ci` — `deep_lanes` from matched risk packs when
     `full-ci` is set.
5. **Honors lane `paths:` filters.** A lane that has a `paths` field is only
   counted toward LEM when at least one changed file matches. Lanes that
   would have been selected but are skipped by paths-filter are reported in
   the `skipped_lanes` section so contributors can see they were considered.
6. Sums `base_lem` per selected lane.
7. Classifies the diff against advisory trust-lane classes from
   `policy/trust-lanes.toml`, recording the strongest class, required proof,
   skipped-by-policy checks, widening triggers, support-claim impact, and
   hosted-CI estimate.
8. Emits the band: `default` / `elevated` / `high` / `over_ceiling`.

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
  "trust_lanes": {
    "schema_version": 1,
    "policy": "trust-lanes",
    "status": "advisory",
    "spec": "docs/specs/PLSP-SPEC-0011-trust-lane-ci-routing.md",
    "strongest_class": {
      "id": "parser_runtime_fix",
      "risk_rank": 50,
      "claim_boundary": "Changes parser, lexer, AST, token, POD, regex, or source-position runtime behavior.",
      "required_checks": ["focused parser runtime tests for the changed grammar family"],
      "skipped_by_policy_checks": ["release proof unless release files changed"],
      "widening_triggers": ["AST shape changes"],
      "support_claim_impact": "Parser bucket or compatibility claims require fresh generated status evidence."
    },
    "changed_surface": ["parser, lexer, or parser-core runtime source"],
    "hosted_ci_estimate": {
      "estimated_lem": 42.0,
      "band": "elevated",
      "selected_lanes": 5
    }
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
- **Not support proof.** Trust-lane classification explains which proof a PR
  should buy for its claim class; it does not prove provider behavior or
  promote support tiers.

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

| PR | Change | Status |
|---:|---|---|
| 04 | Python prototype, advisory only. | landed |
| 07 | Lane origins + paths-filter + ripr summary section. | landed |
| 12 | Replace prototype with `cargo xtask ci plan`. | deferred |
| 13 | Hard-ceiling guard above 125 LEM. | landed |
| 16 | Aggregator + consumer scripts; PR Plan reads `.ci/metrics/ci-lane-history.json` when present. | landed |

The planner now consumes learned LEM estimates whenever the history file has
`learned: true` for a lane (≥ 5 samples in the rolling window). Sampled lanes
substitute `p50 × 1.15` (clamped to the static floor) for the static `base_lem`,
and the plan's `learned` block reports `lanes_using_learned` and
`delta_lem_vs_static`.
