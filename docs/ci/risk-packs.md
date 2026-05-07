# Risk Packs

Risk packs route extra proof to PRs that touch known-risky surfaces. Defined in
[`policy/ci-risk-packs.toml`](../../policy/ci-risk-packs.toml).

> Companion: [pr-plan.md](pr-plan.md), [policy-ledgers.md](policy-ledgers.md).

---

## Catalog

| Risk pack | Surface | Default lanes | Deep lanes (label-gated) |
|---|---|---|---|
| `parser` | parser, lexer, token, AST, tree-sitter | `pr_smoke`, `merge_gate_shards`, `ripr_advisory` | `mutation`, `fuzz`, `coverage` |
| `lsp_provider` | LSP, completion, diagnostics, navigation | `pr_smoke`, `merge_gate_shards`, `ux_tests`, `ripr_advisory` | `real_repo_latency`, `vscode_smoke_matrix` |
| `workspace_index` | module resolution, semantic facts, indexing | `merge_gate_shards`, `lsp_memory_smoke`, `windows_guardrails`, `ripr_advisory` | `memory_plateau`, `real_repo_latency` |
| `retained_state` | long-lived maps, caches, queues, sessions | `lsp_memory_smoke`, `ripr_advisory` | `memory_plateau` |
| `dap` | debug adapter, breakpoints, evaluate | `merge_gate_shards`, `ux_tests`, `ripr_advisory` | — |
| `vscode` | extension packaging, managed binary | `pr_smoke` | `vscode_smoke_matrix` |
| `path_security` | URI normalization, path traversal, sandbox | `merge_gate_shards`, `windows_guardrails`, `ripr_advisory` | — |
| `security` | sandbox, subprocess, exec, deserialization | `security_audit`, `windows_guardrails`, `ripr_advisory` | — |
| `manifest` | Cargo.toml/lock, toolchain | `pr_smoke`, `merge_gate_shards`, `security_audit` | `release_check` |
| `policy` | policy ledgers, gate policy, ripr config | `pr_smoke`, `merge_gate_shards` | — |
| `workflow` | GitHub Actions, CI scripts, xtask CI tasks | `pr_smoke`, `merge_gate_shards` | — |
| `docs_only` | prose / markdown / status | `docs_gate` | — |

---

## How risk packs activate

The PR Plan workflow walks the diff and matches each changed file against every risk
pack's `paths` (glob) and `keywords` (substring on lowercased path). Any match selects
the pack. Selected packs add their `lanes` to the per-PR plan and, when `full-ci` is
labeled, also their `deep_lanes`.

The `retained_state` pack additionally pairs with the PR template's *Retained State*
checklist. If a contributor checks the retained-state boxes, they should see the
memory smoke lane selected automatically by the planner.

---

## Validation

```bash
python3 scripts/ci/validate_risk_packs.py
```

Checks:

- Every `lanes` / `deep_lanes` reference resolves in `policy/ci-lanes.toml`.
- Every pack has at least one of `paths` or `keywords`.
- Labels are strings.

`--strict` exits 1 on any issue.

---

## Adding a new risk pack

1. Add a `[risk_pack.<id>]` entry to `policy/ci-risk-packs.toml`.
2. Set `paths` (preferred — explicit) and/or `keywords`.
3. Set `lanes` to the default-PR lanes that should activate when the pack matches.
4. Set `deep_lanes` to the lanes that activate only with `full-ci`.
5. Set `labels` to label names that, when applied to a PR, force the pack on.
6. Run `python3 scripts/ci/validate_risk_packs.py --strict`.
7. Add a row to the catalog above.

The PR Plan picks up the new pack on the next PR — no workflow edit needed.
