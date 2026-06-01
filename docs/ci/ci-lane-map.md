# CI Lane Map

Quick-reference mapping from policy lanes → workflow jobs → triggers → cost band.
Generated alongside [`policy/ci-lane-whitelist.toml`](../../policy/ci-lane-whitelist.toml)
and meant to stay in sync with it.

> Companion: [inventory.md](inventory.md).

---

## Default-PR lanes

| Lane id | Workflow | Job | Runner | Base LEM | Blocking? |
|---|---|---|---|---:|---:|
| `pr_plan` | `pr-plan.yml` | `plan` | `ubuntu-24.04` | 1 | no |
| `draft_guard` | `ci.yml` | `draft-pr-check` | `ubuntu-24.04` | 1 | yes |
| `preflight_latest` | `ci.yml` | `preflight-latest-check` | `ubuntu-24.04` | 1 | yes |
| `conflict_markers` | `ci.yml` | `conflict-markers` | `ubuntu-24.04` | 1 | yes |
| `pr_title_check` | `pr-title-check.yml` | `validate-title` | `ubuntu-24.04` | 1 | yes |
| `methodology_gate` | `methodology-gate.yml` | `methodology` | `ubuntu-24.04` | 2 | yes |
| `pr_smoke` | `ci.yml` | `pr-smoke` | `ubuntu-24.04` | 4 | yes |
| `merge_gate_shards` | `ci.yml` | `merge-gate-shards` | `ubuntu-24.04` | 24 | yes |
| `merge_gate_aggregate` | `ci.yml` | `merge-gate` | `ubuntu-24.04` | 1 | yes |
| `check_all_targets` | `ci.yml` | `check-all-targets` | `ubuntu-24.04` | 6 | yes |
| `ux_tests` | `ci.yml` | `ux-tests` | `ubuntu-24.04` | 8 | yes |
| `lsp_memory_smoke` | `ci.yml` | `lsp-memory-smoke` | `ubuntu-24.04` | 8 | yes |
| `windows_guardrails` | `ci.yml` | `windows-guardrails` | `windows-latest` | 20 (10m × 2.0) | yes |
| `ripr_advisory` | `ripr.yml` | `ripr` | `ubuntu-24.04` | 4 | **no** |
| `droid_auto_review` | `droid-review.yml` | `droid` | `self-hosted, linux, x64, em-ci, trusted-pr, workflow-nano` | 4 | no |

**Default-PR LEM sum (Linux+Windows weighted):** ≈ 93 LEM today. After PR 17 with
risk-pack routing, expected ordinary-PR LEM ≈ 30–40.

## Label-gated lanes

| Lane id | Trigger labels | Base LEM |
|---|---|---:|
| `mutation` | `ci:mutation`, `mutation`, `full-ci` | 60 |
| `coverage` | `ci:coverage`, `coverage`, `full-ci` | 45 |
| `fuzz` | `ci:fuzz`, `full-ci` | 60 |
| `real_repo_latency` | `ci:bench`, `ci:real-repo-latency`, `full-ci` | 30 |
| `memory_plateau` | `ci:memory`, `full-ci` | 35 |
| `perl_version_matrix` | `ci:perl-matrix`, `full-ci` | 40 |
| `vscode_smoke_matrix` | `ci:vscode-matrix`, `full-ci` | 35 |
| `security_audit` | `security-audit`, `ci:security`, `full-ci` | 15 |
| `release_check` | `release-check`, `full-ci` | 15 |

## Schedule-only lanes

`ci-nightly.yml` (mutation, coverage), `perl-version-matrix.yml`, scheduled passes
of `vscode-managed-binary-smoke.yml`, `ci-security.yml`, `flake-detection.yml`,
`triage-issues.yml`, `merge-ready-reconciler.yml`, `tokmd.yml`.

## Release-only lanes

`release.yml`, `release-orchestration.yml`, `publish-crates.yml`,
`publish-extension.yml`, `docker-publish.yml`, `*-bump.yml`,
`post-publish-smoke.yml`, `vscode-published-extension-smoke.yml`.
