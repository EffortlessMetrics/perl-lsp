# perl-lsp CI Economics Rollout Plan

This is the multi-PR rollout plan for instrumenting `perl-lsp`'s existing CI conveyor
with cost visibility, oracle-gap detection, and policy-driven routing. It does not
replace the conveyor — it adds the missing economics layer.

> Companion: [cost-and-verification-policy.md](cost-and-verification-policy.md),
> [lem-budgeting.md](lem-budgeting.md), [verification-ladder.md](verification-ladder.md).
> Anchors: [`.ci/gate-policy.yaml`](../../.ci/gate-policy.yaml),
> [../reference/CI_ARCHITECTURE.md](../reference/CI_ARCHITECTURE.md).

---

## Order and status

| PR | Name | Purpose | Status | Tracking |
|---:|---|---|---|---|
| 01 | CI economics docs | Encode verification economics doctrine and contributor framing. | landed | #8134 |
| 02 | Policy TOML ledgers | Map every CI item to intent, cost, trigger, owner. | landed | #8135 |
| 03 | CI inventory | Inventory current workflows; flag duplicates and gaps. | landed | #8136 |
| 04 | PR Plan advisory | Emit per-PR LEM estimate and selected lanes. | landed | #8137 |
| 05 | Cache save-only-master | Stop PR cache write churn. | landed | #8138 |
| 06 | `ripr` advisory | Add static oracle-gap signal. | landed | #8139 |
| 07 | PR Plan: `ripr` + LEM wiring | Make planner aware of `ripr` and risk packs. | landed | #8142 |
| 08 | `ci-actuals` | Convert receipts into cost telemetry. | landed (upload wiring deferred) | #8143 (wiring → #8166) |
| 09 | LEM in gate policy | Cross-reference `.ci/gate-policy.yaml` with lane economics. | landed | #8144 |
| 10 | Risk-pack routing metadata | Parser/LSP/workspace/DAP/security/memory risk packs. | landed | #8153 |
| 11 | Workflow policy lint | Lint workflows against `policy/ci-lane-whitelist.toml`. | landed | #8154 |
| 12 | `xtask ci plan` | Move PR Plan from Python to Rust. | deferred | #8166 |
| 13 | Soft LEM warnings | Warn above 35 / 75 LEM, hard guard above 125. | landed | #8145 |
| 14 | External AI review tuning | Make Droid review advisory and LEM-visible. | landed | #8146 |
| 15 | UX ownership clarification | Resolve `ci.yml` UX vs `ux-regression-gate.yml` overlap. | landed | #8147 |
| 16 | Learned estimate scaffolding | Use observed actuals for LEM estimates. | landed | #8155 |
| 17 | Conditional lane cleanup | Tune Windows / memory / UX / external review after actuals. | data-gated | #8166 |
| 18 | `ripr` soft-gate promotion | Require acknowledgement for high-confidence new gaps. | data-gated | #8166 |

End-of-rollout retrospective: [`rollout-summary.md`](rollout-summary.md).

> Tracking-column "landed" PR numbers are the audit-trail PRs from the retrospective; deferred rows (12, 17, 18 + actuals wiring) are tracked together in #8166. Cross-link added via #8670.

---

## Sequencing constraints

- **PR 02 must land before PR 04.** PR Plan reads `policy/*.toml`.
- **PR 04, 06, 08 are independent.** PR Plan, `ripr`, and `ci-actuals` can land in any
  order after PR 02.
- **PR 11 depends on PR 02.** Policy lint reads the whitelist.
- **PR 12 depends on PR 04.** xtask version replaces the Python prototype.
- **PR 13 depends on PR 08.** Soft warnings need a calibration window of actuals.
- **PR 16 depends on PR 13.** Learned estimates need actuals + warnings.
- **PR 17, 18 depend on PR 16.** Lane cleanup and `ripr` promotion need calibration.

---

## What this rollout will not do

- Will not weaken merge-gate shards.
- Will not remove Windows guardrails or memory smoke before actuals.
- Will not make `ripr` blocking until PR 18.
- Will not hard-enforce a 35 LEM budget before calibration.
- Will not require matrix leaf checks in branch protection.
- Will not treat skipped optional lanes as passed without a policy reason.

---

## Implementer handoff

Every PR in the rollout must include:

- Estimated LEM impact.
- Affected workflows.
- Whether branch protection changes (default: **no**).
- What failure mode the lane catches.
- What cheaper signal was considered.
- Rollback path.

Use the PR template's "CI cost / verification note" section.
