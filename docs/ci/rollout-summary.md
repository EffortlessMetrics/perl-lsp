# CI Economics Rollout — Summary

End-of-rollout retrospective for the perl-lsp CI economics rollout described in
[`perl-lsp-rollout-plan.md`](perl-lsp-rollout-plan.md).

> Status: **15 of 18 plan PRs landed.** The remaining 3 are intentionally deferred
> until calibration data accumulates.

---

## What landed

### Doctrine and ledgers (PR 01–03, 10)

- `docs/ci/cost-and-verification-policy.md`, `lem-budgeting.md`, `verification-ladder.md`,
  `labels.md`, `perl-lsp-rollout-plan.md`, `policy-ledgers.md`, `inventory.md`,
  `ci-lane-map.md`, `risk-packs.md`.
- `policy/` ledgers: `ci-budget.toml`, `ci-lanes.toml`, `ci-risk-packs.toml`,
  `ci-lane-whitelist.toml`, `ci-whitelist-exceptions.toml`, `ci-exceptions.toml`,
  `ci-non-rust-allowlist.toml`, `ripr-suppressions.toml`.
- PR template extended with CI cost / verification checklist.

### Forecast and routing (PR 04, 07, 13)

- `.github/workflows/pr-plan.yml` — advisory PR Plan workflow.
- `scripts/ci/pr_plan.py` — Python planner with:
  - Risk-pack routing.
  - Lane-selection origin tracking (`default-pr`, `risk-pack:<id>`, `label:<name>`,
    `deep-lane:full-ci`).
  - `paths:` filter honored — lanes are skipped (visibly) when the diff doesn't match.
  - Soft warnings per band, hard guard at 125 LEM (override via `ci-budget-override`
    or `full-ci`).
  - **Learned-estimate consumer** (delivered with this finalize PR): reads
    `.ci/metrics/ci-lane-history.json` when present and substitutes learned
    `p50 × 1.15` (clamped to static floor) for sampled lanes.

### Cache hygiene (PR 05)

`save-if: master-only` applied across every `Swatinem/rust-cache` invocation in
PR-capable workflows. Cache write traffic drops to one save per master push instead of
one per PR push.

### Oracle-gap detection (PR 06)

- `ripr.toml` config + `policy/ripr-suppressions.toml` ledger.
- `.github/workflows/ripr.yml` advisory workflow (`continue-on-error: true`).
- `scripts/ci/ripr_summary.py` — JSON → step summary with path normalization,
  markdown pipe escaping, UTF-8 explicit encoding.

### Telemetry (PR 08, 09, 16)

- `scripts/ci/emit_ci_actuals.py` — receipts → `ci-actuals.json`.
- `scripts/ci/validate_gate_lane_mapping.py` — every gate in `.ci/gate-policy.yaml`
  maps to a lane in `policy/ci-lanes.toml` (48 / 48 mapped).
- `scripts/ci/aggregate_lane_history.py` — actuals → per-lane p50/p90/p95.
- `scripts/ci/learned_estimate.py` — lane id → learned estimate.
- `.ci/metrics/ci-lane-history.json` — initial empty history (23 lanes).

### Governance enforcement (PR 11)

`cargo xtask workflow-policy-lint --check-lane-whitelist` — opt-in advisory check that
every workflow has a `[[lane]]` entry in `policy/ci-lane-whitelist.toml` or is in
`ALLOWLIST_WORKFLOW_LANE_MISSING`.

### Trim / advisory (PR 14, 15)

- `droid-review.yml` marked `continue-on-error: true` so external AI review failures
  don't block unrelated CI signal.
- UX regression lane ownership decided: `ci.yml::ux-tests` is canonical.
  `ux-regression-gate.yml` slated for retirement after PR 17.

---

## Pipeline state

```
Static base_lem  →  ci-plan.json  (PR 04, 07, 13)
                         ↓
                      PR runs
                         ↓
              gate receipts (target/receipts/)
                         ↓
   scripts/ci/emit_ci_actuals.py  →  target/ci/ci-actuals.json   (PR 08)
                         ↓
   [missing wiring: upload ci-actuals.json + scheduled aggregator]
                         ↓
   scripts/ci/aggregate_lane_history.py  →  .ci/metrics/ci-lane-history.json   (PR 16)
                         ↓
   scripts/ci/learned_estimate.py  /  pr_plan.py  →  learned ci-plan.json   (this PR)
```

The wiring step in the middle is the only piece the rollout did not deliver: a CI
workflow change to upload `ci-actuals.json` artifacts and a scheduled job to download
recent ones and run the aggregator. PR Plan already consumes the history file when
present; the next contributor only needs to keep the file fresh.

---

## What did not land (and why)

### PR 12 — `xtask ci plan` (Rust port of the Python prototype)

Skipped. The Python prototype works and matches the planner's expected output. A Rust
port via `xtask` would let the planner reuse the existing `ci-scope` changed-file
classifier, but the Python version is small, tested, and replaceable. Worth doing
later as a code-quality improvement, not a blocker.

### PR 17 — Conditional lane cleanup

Cannot land without actuals. The plan's specific candidates (`lsp-memory-smoke`,
`windows-guardrails`, `vscode-managed-binary-smoke`, security scans) require evidence
that PR Plan's static estimates are conservative or that the lanes' failure rate
justifies their default-PR cost. Once the calibration window closes (≥ 2 weeks of
actuals), open a follow-up PR per candidate lane.

### PR 18 — `ripr` soft-gate promotion

Cannot land without `ripr` advisory data. The narrow soft-gate rule requires:

- Several weeks of `ripr` findings on real diffs.
- Confidence that high-confidence `reachable_unrevealed` / `weakly_exposed`
  classifications correlate with real oracle gaps.
- Suppressions ledger populated with reviewed entries.

Once data is available, gate promotion lands as a one-line change in `ripr.toml`.

---

## Calibration milestones

| When | What | Who |
|---|---|---|
| **Now** | All 15 PRs merged. Static estimates active. ripr advisory active. | — |
| **+1–2 weeks** | First `ci-actuals.json` artifacts. Sample counts climbing. | follow-up: wire upload + aggregator |
| **+2 weeks** | Diff `ci.yml::ux-tests` vs `ux-regression-gate.yml` evidence. | PR 15 action item → PR 17 |
| **+2–4 weeks** | First lanes cross `MIN_SAMPLES_FOR_LEARNED = 5`. | PR Plan starts using learned estimates automatically |
| **+4 weeks** | Conditional lane cleanup ready (PR 17). | open follow-up PR per candidate |
| **+4–6 weeks** | `ripr` advisory data sufficient for soft-gate rule (PR 18). | open follow-up |

---

## Operational reminders

- **PR Plan failure does not block merges.** The workflow is intentionally not a
  required check; the budget guard is a visibility tool, not a merge gate.
- **`cargo xtask workflow-policy-lint --check-lane-whitelist`** is opt-in and emits
  warnings only. Promotion to error is deferred until the whitelist is stable.
- **`policy/ci-whitelist-exceptions.toml`** entries expire 2026-08-07. PR 17 must
  resolve them or extend with rationale.
- **`policy/ripr-suppressions.toml`** entries expire 2026-08-07. Review before then
  or remove the suppressions.
- **`policy/ci-non-rust-allowlist.toml`** entries expire 2026-11-07.
- **Cache save-only-master**: first master push after this rollout repopulates the
  canonical cache. PRs from then on restore-only.

---

## What this rollout did not change

- **Branch protection.** Required check is still `merge-gate` aggregate.
- **Merge-gate shards.** Same gates, same tier mapping, same blocking behavior.
- **Existing receipt schema** (`.ci/receipt.schema.json`). The actuals collector reads
  it as-is.
- **Existing gate definitions** in `.ci/gate-policy.yaml`. The cross-reference doc
  records mapping but does not modify the file.

---

## Audit-trail PRs

| # | PR | Subject |
|---:|---|---|
| 01 | #8134 | Verification economics doctrine |
| 02 | #8135 | Policy TOML ledgers |
| 03 | #8136 | CI workflow inventory |
| 04 | #8137 | Advisory PR Plan workflow |
| 05 | #8138 | Restore-only cache policy |
| 06 | #8139 | `ripr` advisory |
| 07 | #8142 | PR Plan: lane origins + paths-filter + ripr summary |
| 08 | #8143 | `ci-actuals` from gate receipts |
| 09 | #8144 | Gate ↔ lane economics cross-reference |
| 10 | #8153 | Risk-pack catalog validator |
| 11 | #8154 | `xtask` lane-whitelist check |
| 13 | #8145 | Soft LEM warnings + 125-LEM hard ceiling |
| 14 | #8146 | Droid review advisory |
| 15 | #8147 | UX-regression ownership decision |
| 16 | #8155 | Learned-LEM-estimate scaffolding |
| **finalize** | this PR | Wire learned-estimate consumer + retrospective |
