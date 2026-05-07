# CI Actuals

`ci-actuals.json` records what each gate actually spent in CI. It is the actuals
counterpart to [`ci-plan.json`](pr-plan.md): the plan forecasts cost; the actuals
record what was spent. Together they let later PRs derive learned LEM estimates.

> Companion: [pr-plan.md](pr-plan.md), [lem-budgeting.md](lem-budgeting.md).

---

## What it does

`scripts/ci/emit_ci_actuals.py` walks `target/receipts/` for gate receipts produced by
`xtask`, extracts `gate_name`, `tier`, `status`, `duration_ms`, and runner, and converts
each receipt into a per-job actuals entry with:

- `actual_minutes` = `duration_ms / 60000`
- `actual_lem` = `actual_minutes × runner_multiplier`
- `estimated_lem` = `policy/ci-lanes.toml`'s `base_lem` for the matching lane id
- `delta_lem` = `actual_lem − estimated_lem`

It writes `target/ci/ci-actuals.json`. The schema is intentionally tolerant: missing
`duration_ms` results in `actual_lem: null`. Receipts without a matching lane entry get
`estimated_lem: null`. This avoids the actuals collector becoming a brittle gate.

---

## Wiring

The script is meant to run after `xtask gates` in CI workflows:

```yaml
- name: Run gates
  run: cargo xtask gates --receipt target/receipts/receipt.json

- name: Emit CI actuals
  if: always()
  run: |
    python3 scripts/ci/emit_ci_actuals.py \
      --receipts-dir target/receipts \
      --workflow "${GITHUB_WORKFLOW}" \
      --sha "${GITHUB_SHA}" \
      --pr "${{ github.event.pull_request.number || 0 }}" \
      --json-out target/ci/ci-actuals.json

- name: Upload ci-actuals
  if: always()
  uses: actions/upload-artifact@v4
  with:
    name: ci-actuals
    path: target/ci/ci-actuals.json
    retention-days: 30
    if-no-files-found: warn
```

The actual workflow wiring lands in a follow-up — this PR delivers the script and docs
without touching `.github/workflows/ci.yml` so the rollout stays low-risk.

---

## Output schema

```json
{
  "schema_version": 1,
  "repo": "perl-lsp",
  "sha": "abc",
  "pr": 8137,
  "workflow": "CI",
  "totals": {
    "actual_lem": 25.0,
    "estimated_lem": 36.0,
    "delta_lem": -11.0
  },
  "jobs": [
    {
      "gate_name": "pr_smoke",
      "tier": "pr_fast",
      "status": "pass",
      "runner": "ubuntu_24_04",
      "duration_ms": 120000,
      "actual_minutes": 2.0,
      "actual_lem": 2.0,
      "estimated_lem": 4.0,
      "source_path": "target/receipts/receipt.json"
    }
  ]
}
```

---

## Why receipts, not workflow timestamps

The repo already produces structured gate receipts via `xtask`. Reading those is more
durable than scraping GitHub Actions timestamps from inside a job (which require either
log parsing or extra calls to the Actions API). Receipts also distinguish "the gate
ran" from "the job took N seconds" — the gate's own duration is a cleaner signal than
job overhead like cache restore and toolchain setup.

---

## Roadmap

| PR | Change |
|---:|---|
| 08 | This file. Python script + docs; not wired into workflows yet. |
| 13 | Soft LEM warnings using actuals. |
| 16 | Use percentile actuals (`p50`, `p90`, `p95`) to derive learned estimates. |

The first phase is purely observational. Enforcement based on actuals comes only after
a calibration window has accumulated.
