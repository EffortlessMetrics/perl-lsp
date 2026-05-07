# CI Lane Map

Quick-reference mapping from policy lanes → workflow jobs → triggers → cost band.
Generated alongside [`policy/ci-lane-whitelist.toml`](../../policy/ci-lane-whitelist.toml)
and meant to stay in sync with it.

> Companion: [inventory.md](inventory.md).

---

## Default-PR lanes

| Lane id | Workflow | Job | Runner | Base LEM | Blocking? |
|---|---|---|---|---:|---:|
| `draft-pr-guard` | `ci.yml` | `draft-pr-check` | `ubuntu-24.04` | 1 | yes |
| `preflight-latest-sha` | `ci.yml` | `preflight-latest-check` | `ubuntu-24.04` | 1 | yes |
| `conflict-marker-check` | `ci.yml` | `conflict-markers` | `ubuntu-24.04` | 1 | yes |
| `pr-title-check` | `pr-title-check.yml` | `validate-title` | `ubuntu-24.04` | 1 | yes |
| `methodology-gate` | `methodology-gate.yml` | `methodology` | `ubuntu-24.04` | 2 | yes |
| `pr-smoke` | `ci.yml` | `pr-smoke` | `ubuntu-24.04` | 4 | yes |
| `merge-gate-shards` | `ci.yml` | `merge-gate-shards` | `ubuntu-24.04` | 24 | yes |
| `merge-gate-aggregate` | `ci.yml` | `merge-gate` | `ubuntu-24.04` | 1 | yes |
| `ux-regression` | `ci.yml` | `ux-tests` | `ubuntu-24.04` | 8 | yes |
| `lsp-memory-smoke` | `ci.yml` | `lsp-memory-smoke` | `ubuntu-24.04` | 8 | yes |
| `windows-guardrails` | `ci.yml` | `windows-guardrails` | `windows-latest` | 20 (10m × 2.0) | yes |
| `ripr-advisory` | `ripr.yml` | `ripr` | `ubuntu-24.04` | 4 | **no** |
| `droid-auto-review` | `droid-review.yml` | `droid` | `ubuntu-latest` | 4 | no |

**Default-PR LEM sum (Linux+Windows weighted):** ≈ 87 LEM today. After PR 17 with
risk-pack routing, expected ordinary-PR LEM ≈ 30–40.

## Label-gated lanes

| Lane id | Trigger labels | Base LEM |
|---|---|---:|
| `mutation-nightly` | `ci:mutation`, `mutation`, `full-ci` | 60 |
| `coverage` | `ci:coverage`, `coverage`, `full-ci` | 45 |
| `perl-version-matrix` | `ci:perl-matrix`, `full-ci` | 40 |
| `vscode-managed-binary-smoke` | `ci:vscode-matrix`, `full-ci` | 35 |
| `ci-security` | `security-audit`, `ci:security`, `full-ci` | 15 |

## Schedule-only lanes

`ci-nightly.yml` (mutation, coverage), `perl-version-matrix.yml`, scheduled passes
of `vscode-managed-binary-smoke.yml`, `ci-security.yml`, `flake-detection.yml`,
`triage-issues.yml`, `merge-ready-reconciler.yml`, `tokmd.yml`.

## Release-only lanes

`release.yml`, `release-orchestration.yml`, `publish-crates.yml`,
`publish-extension.yml`, `docker-publish.yml`, `*-bump.yml`,
`post-publish-smoke.yml`, `vscode-published-extension-smoke.yml`.
