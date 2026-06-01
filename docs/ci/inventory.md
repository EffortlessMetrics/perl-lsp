# CI Workflow Inventory

Map every workflow and job to **one** intent. This is the inventory step that
PR 02's whitelist depends on. Where a row says "duplicate of," the resolution
is decided in PR 15+ once actuals confirm the overlap.

> Companion: [ci-lane-map.md](ci-lane-map.md), [policy-ledgers.md](policy-ledgers.md).
> Source of truth for execution: [`.ci/gate-policy.yaml`](../../.ci/gate-policy.yaml).
> Source of truth for governance: [`policy/ci-lane-whitelist.toml`](../../policy/ci-lane-whitelist.toml).

---

## Repo shape

| | |
|---|---|
| Default branch | `master` |
| Rust workspace | resolver `3`, MSRV `1.92.0` (per `rust-toolchain.toml`) |
| Workspace members | 134 (per `cargo metadata --no-deps`) |
| Required branch checks | `merge-gate` (aggregate), `pr-title-check`, `methodology-gate`, `workflow-policy` |
| Tiers | `pr_fast`, `merge_gate`, `nightly`, `release` (see `.ci/gate-policy.yaml`) |

---

## Workflow inventory

### Default-PR workflows

| Workflow | Job(s) | Trigger | Blocking? | Runner | Intent | Est. LEM | Whitelist id | Duplicate of | Disposition |
|---|---|---|---:|---|---|---:|---|---|---|
| `pr-plan.yml` | `plan` | `pull_request`, `workflow_dispatch` | no | `ubuntu-24.04` | LEM forecast and lane selection | 1 | `pr_plan` | — | keep |
| `ci.yml` | `pr-smoke` | `pull_request`, `push`, `merge_group` | yes | `ubuntu-24.04` | Fast scoped Rust proof | 4 | `pr_smoke` | — | keep |
| `ci.yml` | `merge-gate-shards` | `pull_request`, `push`, `merge_group` | yes | `ubuntu-24.04` (×N) | Bounded merge-gate shards | 24 | `merge_gate_shards` | — | keep |
| `ci.yml` | `merge-gate` | `pull_request`, `push`, `merge_group` | yes | `ubuntu-24.04` | Aggregate shard results | 1 | `merge_gate_aggregate` | — | keep |
| `ci.yml` | `check-all-targets` | `pull_request`, `push`, `merge_group` | yes | `ubuntu-24.04` | Compile all targets | 6 | `check_all_targets` | — | keep |
| `ci.yml` | `ux-tests` | `pull_request`, `push`, `merge_group` | yes | `ubuntu-24.04` | LSP UX regression smoke | 8 | `ux_tests` | `workflow:ux-regression-gate` | **decide PR 15** |
| `ci.yml` | `lsp-memory-smoke` | `pull_request`, `push`, `merge_group` | yes | `ubuntu-24.04` | Retained-state regression | 8 | `lsp_memory_smoke` | `nightly:memory_plateau` | keep, exception logged |
| `ci.yml` | `windows-guardrails` | `pull_request`, `push`, `merge_group` | yes | `windows-latest` | Windows path / sandbox regression | 20 (10m × 2.0) | `windows_guardrails` | — | keep, exception logged |
| `ci.yml` | `conflict-markers` | `pull_request`, `push`, `merge_group` | yes | `ubuntu-24.04` | Reject committed conflict markers | 1 | `conflict_markers` | `gate:check_conflict_markers` | **decide PR 15** |
| `ci.yml` | `draft-pr-check` | `pull_request` | yes | `ubuntu-24.04` | Skip draft PRs | 1 | `draft_guard` | — | keep |
| `ci.yml` | `preflight-latest-check` | `pull_request`, `push` | yes | `ubuntu-24.04` | Skip superseded SHAs | 1 | `preflight_latest` | — | keep |
| `ripr.yml` | `ripr` | `pull_request` (Rust paths), `workflow_dispatch` | no | `ubuntu-24.04` | Static oracle-gap detection | 4 | `ripr_advisory` | `nightly:mutation` | keep |
| `methodology-gate.yml` | `methodology` | `pull_request` | yes | `ubuntu-24.04` | PR-shape and methodology lint | 2 | `methodology_gate` | — | keep |
| `pr-title-check.yml` | `validate-title` | `pull_request` | yes | `ubuntu-24.04` | Real issue ref enforcement | 1 | `pr_title_check` | — | keep |
| `workflow-policy.yml` | `workflow-policy-lint` | `pull_request`, `workflow_dispatch` | yes | `ubuntu-24.04` | Workflow trigger / policy lint | 2 | _add in PR 11_ | — | keep, extend in PR 11 |
| `workflow-trigger-lint.yml` | `workflow-trigger-lint` | `pull_request` | yes | `ubuntu-24.04` | Trigger policy lint (legacy) | 1 | _add in PR 11_ | `workflow:workflow-policy` | **decide PR 15** |
| `droid-review.yml` | `droid` | `pull_request` (opened/ready/reopened) | no | `self-hosted, linux, x64, em-ci, trusted-pr, review-nano, droid-review` | External AI review | 4 | `droid_auto_review` | — | keep |
| `pipeline-labels.yml` | `apply-pipeline-labels` | `pull_request`, `pull_request_target` | no | `ubuntu-24.04` | Apply pipeline state labels | 1 | _add in PR 11_ | — | keep |
| `flake-detection.yml` | `flake-detect` | `pull_request`, `schedule` | no | `ubuntu-24.04` | Detect flaky tests | varies | _add in PR 11_ | — | keep |

### Nightly / scheduled workflows

| Workflow | Trigger | Blocking? | Runner | Intent | Est. LEM | Whitelist id | Disposition |
|---|---|---:|---|---|---:|---|---|
| `ci-nightly.yml` (mutation) | `schedule`, `workflow_dispatch`, label | no | `ubuntu-24.04` | Mutation testing | 60 | `mutation` | keep |
| `ci-nightly.yml` (test-coverage) | `schedule`, `workflow_dispatch`, label | no | `ubuntu-24.04` | Coverage | 45 | `coverage` | keep |
| `ci-nightly.yml` (fuzz) | `schedule`, `workflow_dispatch` | no | `ubuntu-24.04` | Bounded fuzz sweep | 60 | `fuzz` | keep |
| `ci-nightly.yml` (real-repo-latency) | `schedule`, `workflow_dispatch`, label | no | `ubuntu-24.04` | Real-repo latency | 30 | `real_repo_latency` | keep |
| `ci-nightly.yml` (memory-plateau) | `schedule`, `workflow_dispatch`, label | no | `ubuntu-24.04` | Memory plateau | 35 | `memory_plateau` | keep |
| `publish-dry-run.yml` | `pull_request` (release paths), `workflow_dispatch` | no | `ubuntu-24.04` | Release dry-run | 15 | `release_check` | keep |
| `ci-security.yml` | `schedule`, `pull_request` (security paths), `workflow_dispatch` | no | `ubuntu-24.04` | audit / deny / Trivy | 15 | `security_audit` | keep |
| `perl-version-matrix.yml` | `schedule`, `workflow_dispatch`, label | no | `ubuntu-24.04` | Perl 5.8–5.40 compat | 40 | `perl_version_matrix` | keep |
| `vscode-managed-binary-smoke.yml` | `schedule`, `workflow_dispatch`, paths, label | no | mixed (Linux/Win/macOS) | VS Code extension smoke | 35 | `vscode_smoke_matrix` | keep |
| `vscode-published-extension-smoke.yml` | `schedule`, `workflow_dispatch` | no | mixed | Post-publish extension smoke | varies | _add in PR 11_ | keep |
| `post-merge-corpus-ratchet.yml` | `push` (master) | no | `ubuntu-24.04` | Auto-ratchet CPAN corpus | 5 | _add in PR 11_ | keep |
| `post-merge-status.yml` | `push` (master) | no | `ubuntu-24.04` | Regenerate status docs | 3 | _add in PR 11_ | keep |
| `post-publish-smoke.yml` | `release`, `workflow_dispatch` | no | mixed | Post-publish smoke | varies | _add in PR 11_ | keep |
| `tokmd.yml` | `schedule`, `workflow_dispatch` | no | `ubuntu-24.04` | Token usage / metrics | varies | _add in PR 11_ | keep |
| `merge-ready-reconciler.yml` | `schedule`, `workflow_dispatch` | no | `ubuntu-24.04` | Reconcile merge-ready labels | 1 | _add in PR 11_ | keep |
| `triage-issues.yml` | `schedule`, `issues` | no | `ubuntu-24.04` | Issue triage automation | 1 | _add in PR 11_ | keep |
| `ci-gate-self-tests.yml` | `pull_request` (gate paths), `workflow_dispatch` | no | `ubuntu-24.04` | Validate gate definitions | 3 | _add in PR 11_ | keep |

### Release / publishing workflows

| Workflow | Trigger | Blocking? | Runner | Intent | Disposition |
|---|---|---:|---|---|---|
| `release.yml` | `release`, `workflow_dispatch` | yes (release-only) | mixed | Build and ship release artifacts | keep |
| `release-orchestration.yml` | `workflow_dispatch` | yes (release-only) | mixed | Coordinate release lifecycle | keep |
| `publish-crates.yml` | `release`, `workflow_dispatch` | yes | `ubuntu-24.04` | Publish to crates.io | keep |
| `publish-extension.yml` | `release`, `workflow_dispatch` | yes | `ubuntu-24.04` | Publish VS Code extension | keep |
| `publish-dry-run.yml` | `pull_request` (release paths), `workflow_dispatch` | no | `ubuntu-24.04` | Pre-release validation | keep |
| `docker-publish.yml` | `release`, `workflow_dispatch` | yes | `ubuntu-24.04` | Push Docker images | keep |
| `docs-deploy.yml` | `push` (master, docs paths), `workflow_dispatch` | no | `ubuntu-24.04` | Deploy mkdocs site | keep |
| `version-bump.yml` | `workflow_dispatch` | no | `ubuntu-24.04` | Workspace version bump PR | keep |
| `brew-bump.yml` | `release`, `workflow_dispatch` | no | `ubuntu-24.04` | Update Homebrew formula | keep |
| `chocolatey-bump.yml` | `release`, `workflow_dispatch` | no | `windows-latest` | Update Chocolatey package | keep |
| `scoop-bump.yml` | `release`, `workflow_dispatch` | no | `windows-latest` | Update Scoop manifest | keep |
| `winget-bump.yml` | `release`, `workflow_dispatch` | no | `windows-latest` | Update WinGet manifest | keep |

### Standalone / utility workflows

| Workflow | Trigger | Blocking? | Intent | Disposition |
|---|---|---:|---|---|
| `droid.yml` | `workflow_dispatch` | no | Standalone Droid command | keep, document in PR 14 |
| `ux-regression-gate.yml` | `pull_request` (UX paths), `workflow_dispatch` | yes (paths) | Path-gated UX regression | **decide PR 15** (vs `ci.yml::ux-tests`) |

---

## Proof-obligation map (current coverage and gaps)

| Failure mode | Cheapest lane today | Deep lane today | Coverage | Gap / action |
|---|---|---|---|---|
| Rust compile break | `pr-smoke` | `merge-gate-shards`, `check-all-targets` | yes | none |
| Format drift | `pr-smoke` (xtask fmt) | — | yes | none |
| Lint / banned-pattern violation | `pr-smoke` (clippy) | strict clippy | yes | none |
| Changed behavior lacks oracle | — | `mutation-nightly` | **no on PR** | **PR 06: add `ripr` advisory** |
| LSP UX regression | `ci.yml::ux-tests` | full UX harness, real-repo latency | yes | duplicate ownership with `ux-regression-gate.yml` (PR 15) |
| Retained-state regression | `lsp-memory-smoke` | `memory_plateau` | yes | LEM not measured (PR 08) |
| Windows path / sandbox regression | `windows-guardrails` | platform matrix | yes | LEM not measured (PR 08) |
| Extension regression | — | `vscode-managed-binary-smoke` | partial | consider Linux smoke default (PR 17) |
| Dependency vulnerability | `ci-security.yml` | scheduled audit / Trivy | yes | cache policy likely save-heavy (PR 05) |
| Public API break | none direct | release dry-run | weak | follow-up issue |
| Schema/serialization break | covered via parser tests | corpus | partial | none |
| Conflict markers | `conflict-markers` job + `gate:check_conflict_markers` | — | duplicate | resolve in PR 15 |

---

## Duplicate-intent flags (action items)

| Pair | Resolution PR |
|---|---|
| `ci.yml::ux-tests` vs `ux-regression-gate.yml` | PR 15 |
| `ci.yml::conflict-markers` vs `gate:check_conflict_markers` | PR 15 |
| `workflow-policy.yml` vs `workflow-trigger-lint.yml` | PR 15 |
| `ripr-advisory` vs `mutation-nightly` (related, not duplicate — see verification ladder) | document, keep both |

---

## Open questions

1. Which `ci.yml::*` jobs are actually included in the required `merge-gate`
   summary today, vs. running independently in branch protection? (Inventory
   captures intent; the merge-gate summary's exact membership lives in
   `.github/workflows/ci.yml`.)
2. Do any nightly workflows accidentally trigger on PRs without a label?
   (Policy lint in PR 11 will check this.)
3. Is `droid.yml` (standalone) still in active use, or superseded by
   `droid-review.yml`? (Tune in PR 14.)
4. Should `vscode-managed-binary-smoke.yml` Windows/macOS legs be moved off
   default-PR triggering after PR 08 actuals? (PR 17.)
