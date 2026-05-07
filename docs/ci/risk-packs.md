# CI Risk Packs

Risk packs route extra verification proof to PRs that touch known-risky surfaces.
They are the perl-lsp-specific layer between `policy/ci-lanes.toml` (which lanes
exist) and the PR planner (which lanes run for *this* PR).

**Source of truth:** [`policy/ci-risk-packs.toml`](../../policy/ci-risk-packs.toml)
**Consumer:** [`scripts/ci/pr_plan.py`](../../scripts/ci/pr_plan.py)
**Validation:** [`scripts/ci/validate_risk_packs.py`](../../scripts/ci/validate_risk_packs.py)

## How risk packs are selected

For each PR, the planner walks every entry in `[risk_pack.*]`:

1. **Path match** — if any changed file matches a glob in `paths`, the pack
   is selected.
2. **Keyword match** — if any changed file path (lowercased) contains any
   substring in `keywords`, the pack is selected.

A single PR can match multiple packs. Each matched pack contributes lanes
from its `lanes` list. When the `full-ci` label is present, lanes from
`deep_lanes` are added too.

## Schema

```toml
[risk_pack.<id>]
description = "Free-form description shown in the PR Plan summary."
paths       = ["crates/foo/**", "src/lib.rs"]   # globs over changed files
keywords    = ["session", "cache"]              # case-insensitive substring match
lanes       = ["pr_smoke", "merge_gate_shards"] # selected when pack matches
deep_lanes  = ["mutation"]                      # added on `full-ci`
labels      = ["ci:foo"]                        # informational; documents related labels
```

Lane keys must exist in `policy/ci-lanes.toml`. The validation script
(`scripts/ci/validate_risk_packs.py`) enforces this.

## The 12 packs

| Pack             | Surface                                                                              | Default lanes                                                       | Deep lanes (`full-ci`)            |
| ---------------- | ------------------------------------------------------------------------------------ | ------------------------------------------------------------------- | --------------------------------- |
| `parser`         | parser, lexer, token, AST, tree-sitter, incremental parsing, line index, POD, pragma | `pr_smoke`, `merge_gate_shards`, `ripr_advisory`                    | `mutation`, `fuzz`, `coverage`    |
| `lsp_provider`   | LSP server, providers, diagnostics, refactoring, perltidy, UX tests                  | `pr_smoke`, `merge_gate_shards`, `ux_tests`, `ripr_advisory`        | `real_repo_latency`, `vscode_smoke_matrix` |
| `workspace_index`| workspace, modules, semantic facts, indexing, symbol resolution, corpus              | `merge_gate_shards`, `lsp_memory_smoke`, `windows_guardrails`, `ripr_advisory` | `memory_plateau`, `real_repo_latency` |
| `retained_state` | long-lived maps, caches, queues, sessions, document lifecycle, subprocess state      | `lsp_memory_smoke`, `ripr_advisory`                                 | `memory_plateau`                  |
| `dap`            | DAP server, breakpoints, stepping, evaluate, launch/attach, session lifecycle        | `merge_gate_shards`, `ux_tests`, `lsp_memory_smoke`, `ripr_advisory`| —                                 |
| `vscode`         | VS Code extension packaging, managed binary, extension-host                          | `pr_smoke`                                                          | `vscode_smoke_matrix`             |
| `path_security`  | URI normalization, path traversal, sandbox enforcement                               | `merge_gate_shards`, `windows_guardrails`, `ripr_advisory`          | —                                 |
| `security`       | sandbox, subprocess, eval/exec, deserialization, dependency churn                    | `security_audit`, `windows_guardrails`, `ripr_advisory`             | —                                 |
| `manifest`       | `Cargo.toml`/`Cargo.lock`, toolchain, release metadata                               | `pr_smoke`, `merge_gate_shards`, `security_audit`                   | `release_check`                   |
| `policy`         | `policy/*.toml`, `.ci/gate-policy.yaml`, `ripr.toml`                                 | `pr_smoke`, `merge_gate_shards`                                     | —                                 |
| `workflow`       | `.github/workflows/**`, `scripts/ci/**`, `xtask/src/tasks/{ci,workflow,gate}_*`      | `pr_smoke`, `merge_gate_shards`                                     | —                                 |
| `docs_only`      | prose, markdown, generated status                                                    | `docs_gate`                                                         | —                                 |

## Path-security vs. security

The two packs deliberately overlap. `security` is the broad supply-chain and
runtime-execution surface (subprocess, eval, dependency upgrades). `path_security`
is the narrow path-handling surface (URI normalization, traversal, sandbox
fail-closed) where Windows-specific regressions cluster — splitting it out
keeps `windows_guardrails` selection precise even when `Cargo.lock` hasn't
moved.

## Retained-state keywords

The `retained_state` pack matches both crate paths (runtime, workspace,
subprocess) and keyword fragments in *any* changed file path: `cache`,
`session`, `queue`, `background`, `evict`, `close`, `delete`, `uri`,
`workspace_folder`, `stream`, `subprocess`. This intentionally over-matches —
a 3 LEM `lsp_memory_smoke` run is cheaper than missing a retained-state
regression.

## Adding or changing a risk pack

1. Edit `policy/ci-risk-packs.toml`. Keep `description` accurate; update
   `lanes` only after confirming the lane exists in `policy/ci-lanes.toml`.
2. Run `python3 scripts/ci/validate_risk_packs.py` — it checks parseability,
   lane resolution, glob form, and unknown fields.
3. Run a planner spot-check:
   ```bash
   python3 scripts/ci/pr_plan.py \
     --base origin/master --head HEAD \
     --json-out target/ci/ci-plan.json
   ```
   and confirm the new pack appears in `selection.risk_packs` for a representative diff.
4. Update this document's pack table.

## What risk packs do *not* do

- They do not block merges. Lane blocking is decided by `policy/ci-lanes.toml`.
- They do not invent lanes. They only select among lanes already declared.
- They do not bypass `default_pr = true` lanes — those run regardless.
- They do not run runtime mutation testing on every match — `mutation` is a
  `deep_lane` reached only via `full-ci` (or the `ci:mutation` label).

## Roadmap

- **PR 12** ports the planner from Python to a Rust `xtask ci plan` command.
  Risk packs continue to be defined in TOML; the consumer changes language.
- **PR 16** adds learned LEM estimates from `ci-actuals.json` history.
  Risk packs may gain a `historical_p50` shadow field for visibility.
