# Gate Policy ↔ Lane Economics Cross-Reference

`.ci/gate-policy.yaml` defines **what executes**; `policy/ci-lanes.toml` defines **what
it costs and why**. This doc is the cross-reference between the two. The mapping
itself lives in `scripts/ci/validate_gate_lane_mapping.py` so it can be checked by
running:

```bash
python3 scripts/ci/validate_gate_lane_mapping.py
```

> Companion: [policy-ledgers.md](policy-ledgers.md), [pr-plan.md](pr-plan.md),
> [ci-actuals.md](ci-actuals.md).

---

## Why this exists

The two files have orthogonal concerns:

| `.ci/gate-policy.yaml` | `policy/ci-lanes.toml` |
|---|---|
| Per-gate definitions | Per-lane economics |
| Tiers (`pr_fast`, `merge_gate`, …) | Bands (`default`, `elevated`, …) |
| `command`, `timeout_seconds`, `budgets.max_duration_ms` | `base_lem`, `runner`, `default_pr` |
| Source-of-truth for execution | Source-of-truth for cost forecast |

Without a cross-reference, the planner's lane forecast can drift from the gate runner's
actual behavior. The validator script ensures every gate has a known lane home; if a new
gate is added without a mapping, the validator flags it and forces an update to either
the lane policy or the validator's mapping table.

---

## Mapping rule

Many gates roll up under a single lane (e.g. all `pr_fast` gates contribute to the
`pr_smoke` lane's LEM). The mapping is therefore many-to-one for most lanes, but a small
number of gates (e.g. `lsp_tier_a`) span two lanes.

Current state (as of PR 09):

- 48 gates in `.ci/gate-policy.yaml`
- 23 lanes in `policy/ci-lanes.toml`
- 48 / 48 gates have at least one lane mapping
- 0 gates point at a non-existent lane

---

## Lane → gates

| Lane | Gates |
|---|---|
| `pr_smoke` | `fmt`, `release_history`, `readme_heading_check`, `publish_closure`, `publish_manifest_check`, `layer_check`, `published_crate_count_pr_fast`, `release_history_check`, `clippy_scoped`, `unit_scoped`, `check_tests_scoped`, `policy_checks`, `workflow_audit`, `nested_lock_check` |
| `merge_gate_shards` | `clippy_core`, `unit_core`, `perl_token_leaf_contract`, `clippy_full`, `unit_foundation_full`, `unit_parser_stack_full`, `unit_analysis_full`, `unit_lsp_full`, `unit_dap_support_full`, `common_corpus_clean`, `parser_corpus_ratchet`, `cpan_corpus_ratchet`, `parser_audit_closeout`, `v2_parity`, `v2_bundle_sync` |
| `check_all_targets` | `compile_all_targets` |
| `conflict_markers` | `check_conflict_markers` |
| `ux_tests` | `lsp_smoke`, `lsp_tier_a` |
| `docs_gate` | `docs_build` |
| `release_check` | `published_crate_count`, `release_build`, `version_sync`, `sbom_verify`, `determinism_check` |
| `security_audit` | `security_audit` |
| `mutation` | `mutation`, `corpus_validation`, `corpus_sweep` |
| `fuzz` | `fuzz` |
| `coverage` | `coverage` |
| `real_repo_latency` | `benchmarks`, `lsp_tier_a`, `lsp_tier_b` |
| `perl_version_matrix` | `full_matrix` |

Lanes without any gate mapping today: `pr_plan`, `draft_guard`, `preflight_latest`,
`merge_gate_aggregate`, `lsp_memory_smoke`, `windows_guardrails`, `ripr_advisory`,
`memory_plateau`, `vscode_smoke_matrix`, `droid_auto_review`, `methodology_gate`,
`pr_title_check`. These either have no `.ci/gate-policy.yaml` entry (workflow-level
controls, not gates) or run under standalone workflows.

---

## What this PR does not do

- Does **not** add fields to `.ci/gate-policy.yaml`. The mapping table lives in the
  validator script so the policy file's existing schema and parser are unchanged.
- Does **not** change CI behavior. The validator is informational; PR 11 (workflow
  policy lint extension) will optionally enforce it once the mapping has stabilized.
- Does **not** require running the validator in CI yet. Run it locally if you change
  gate-policy or lanes; it will catch drift in seconds.

---

## How to extend

When you add a new gate to `.ci/gate-policy.yaml`:

1. Open `scripts/ci/validate_gate_lane_mapping.py`.
2. Add an entry to `GATE_TO_LANE_MAP`:
   ```python
   "your_new_gate": {"lanes": ["lane_id_from_ci_lanes_toml"]},
   ```
3. Run `python3 scripts/ci/validate_gate_lane_mapping.py` to confirm.

When you add a new lane to `policy/ci-lanes.toml` and want one or more gates to roll up
under it, update the validator's mapping table. The lane key in `ci-lanes.toml` is the
`lanes:` value in the validator.
