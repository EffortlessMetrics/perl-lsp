# Linux-Equivalent Minutes (LEM)

LEM is a normalized cost unit for CI lanes. It lets every lane and every PR be compared
on the same axis regardless of which runner type or OS the work runs on.

> See [cost-and-verification-policy.md](cost-and-verification-policy.md) for the
> operating thesis and [verification-ladder.md](verification-ladder.md) for which lanes
> sit at which cost band.

---

## Definition

```text
LEM = wall_clock_minutes × runner_multiplier
```

Runner multipliers are defined in [`policy/ci-budget.toml`](../../policy/ci-budget.toml).
Initial values:

| Runner | Multiplier |
|---|---:|
| `ubuntu-24.04` / `ubuntu-latest` | 1.0 |
| `windows-latest` | 2.0 |
| `macos-latest` | 10.0 |
| `docker_build` | 6.0 |
| `external_ai_review` | 4.0 |

For standard GitHub-hosted runners (`ubuntu-*`, `windows-*`, `macos-*`) these reflect
GitHub's documented per-minute billing weights. For composite or specialized lanes
(`docker_build`, `external_ai_review`, `self_hosted_gpu`) the multipliers are internal
cost estimates rather than documented billing weights, since GitHub does not bill those
as a single runner type. The exact rates are kept in TOML so they can be tuned without
edits to docs.

---

## Bands

Default planning bands:

| Band | LEM | Meaning |
|---|---:|---|
| pennies | 0–12 | docs, metadata, light checks |
| default | 13–35 | ordinary Rust PR |
| elevated | 36–75 | risk-expanded PR |
| high | 76–125 | explicit expensive PR |
| over ceiling | >125 | requires `full-ci` or `ci-budget-override` |

With a planning rate of `$0.008/Linux minute`:

| LEM | Planning $ |
|---:|---:|
| 35 | ~$0.28 |
| 75 | ~$0.60 |
| 125 | ~$1.00 |

The dollar figures are display-only; **billing truth lives in GitHub Actions usage
reports**, not in this policy.

---

## Estimation strategy

Three phases:

1. **Static** (PR 04). Each lane declares `base_lem` in
   [`policy/ci-lanes.toml`](../../policy/ci-lanes.toml). The PR Plan sums selected lanes.
2. **Actuals** (PR 08). Receipts emit `target/ci/ci-actuals.json` with measured runtime
   and runner multiplier. No enforcement.
3. **Learned** (PR 16). Planner uses recent percentile actuals:

   ```text
   estimate     = max(static_floor, p50_recent_actual × 1.15)
   warning      = p90_recent_actual
   hard_planning = p95_recent_actual
   ```

   Strict enforcement of learned estimates is deferred until a calibration window has
   accumulated.

---

## Soft warnings, hard guard

After PR 13:

| Estimated LEM | Behavior |
|---:|---|
| 0–35 | green summary |
| 36–75 | warning |
| 76–125 | high warning, suggest `ci-budget-ack` |
| >125 | fails unless `full-ci` or `ci-budget-override` |

Sub-125-LEM PRs never hard-fail on cost.

---

## What LEM is not

- **Not a billing source.** GitHub usage and Blacksmith billing reports are authoritative.
- **Not a per-author throttle.** It is a per-PR planning signal.
- **Not a substitute for receipts.** Receipts (`target/receipts/`) remain the primary
  evidence for whether a gate passed.

LEM exists to make spend *visible*, not to replace the existing gate evidence layer.
