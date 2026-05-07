# CI Policy Ledgers

Index of the machine-readable CI policy files under [`policy/`](../../policy/).

> See [cost-and-verification-policy.md](cost-and-verification-policy.md) for the
> operating thesis, [perl-lsp-rollout-plan.md](perl-lsp-rollout-plan.md) for the
> rollout sequence.

---

## Files

| File | Role | Read by |
|---|---|---|
| `policy/ci-budget.toml` | LEM bands, runner multipliers, label conventions | PR Plan, policy lint |
| `policy/ci-lanes.toml` | Lane economics (intent, base LEM, default-PR flag) | PR Plan, policy lint |
| `policy/ci-risk-packs.toml` | When extra proof is relevant (paths/keywords → lanes) | PR Plan |
| `policy/ci-lane-whitelist.toml` | Why each CI lane exists, owner, expiry | Policy lint |
| `policy/ci-whitelist-exceptions.toml` | Whitelist-specific debt ledger | Policy lint |
| `policy/ci-exceptions.toml` | General CI policy deviations | Policy lint |
| `policy/ci-non-rust-allowlist.toml` | First-class non-Rust CI surfaces | PR Plan, policy lint |
| `policy/ripr-suppressions.toml` | Ripr finding suppressions | `ripr.toml` |

---

## Governance pattern

All ledgers follow the same shape:

```toml
schema_version = 1
policy = "..."
owner = "EffortlessMetrics"
status = "active" | "advisory"
updated = "YYYY-MM-DD"
```

All ledger entries (`[[exception]]`, `[[allow]]`, `[[suppress]]`, `[[lane]]`) require:

- `id` — stable identifier
- `owner` — accountable team or person
- `reason` — why this deviation or entry exists
- `issue` — tracking issue (or `"TODO"` if not yet filed)
- `created` — date entry was added
- `review_after` — date by which this entry must be re-reviewed
- `expires` — hard expiry; entry becomes invalid past this date

This mirrors the existing pattern in `.ci/debt-ledger.yaml` and the strict-lint
exception ledgers.

---

## Lifecycle

```
unknown → whitelisted → measured → learned-estimate → routed → enforced
```

A lane should not be routed (PR 12) or enforced (PR 13+) until it has been:

1. Whitelisted (PR 02 — this file's contents).
2. Inventoried (PR 03 — `docs/ci/inventory.md`).
3. Measured (PR 08 — `target/ci/ci-actuals.json`).

Skipping steps produces brittle policy. The point of the ledgers is that every
deviation is reviewable and dated.

---

## Editing

When adding a new CI workflow or job:

1. Add a `[[lane]]` entry to `policy/ci-lane-whitelist.toml` with intent,
   failure mode, proof obligation, evidence, runner, base LEM.
2. Add or extend a `[lane.<id>]` entry in `policy/ci-lanes.toml` for the
   economics-side metadata.
3. If the lane is `default_pr = true` and `expensive = true`, add a
   corresponding `[[exception]]` entry to
   `policy/ci-whitelist-exceptions.toml` with `review_after` and `expires`.
4. If the lane uses non-Rust tooling, ensure
   `policy/ci-non-rust-allowlist.toml` covers its paths.

The policy lint (PR 11) will reject workflows whose jobs lack lane entries.
