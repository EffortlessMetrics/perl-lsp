# CI Workflow Audit

Generated: 2026-01-25

## Summary

| Metric | Value |
|--------|-------|
| Total workflows | 21 |
| With timeouts | 16 |
| Without timeouts | 5 (release/publishing only) |
| With concurrency | 16 |
| Authoritative gate | `ci.yml` |

## Workflow Categories

### Required Gate (1)
| Workflow | File | Trigger | Timeout | Status |
|----------|------|---------|---------|--------|
| CI Gate | `ci.yml` | PR, manual | 30m | **Primary merge gate** |

### Quality Checks (Label-Gated) (5)
| Workflow | File | Label | Timeout |
|----------|------|-------|---------|
| Quality Checks | `quality-checks.yml` | Various ci:* labels | 10-60m |
| LSP Tests | `lsp-tests.yml` | `ci:lsp` | 30-45m |
| Property Tests | `property-tests.yml` | `ci:property` | 15-30m |
| Rust Strict | `rust-strict.yml` | `ci:strict` | 10-30m |
| CI (Expensive) | `ci-expensive.yml` | `ci:mutation`, `ci:bench` | 30-60m |

### Additional PR Workflows (3)
| Workflow | File | Trigger | Timeout | Notes |
|----------|------|---------|---------|-------|
| Tests | `test.yml` | labeled PR | 10-45m | `ci:tests` label |
| Comprehensive Tests | `comprehensive_tests.yml` | labeled PR | 30m | `ci:all-tests` label |
| Check Ignored | `check-ignored.yml` | path-triggered PR | 10m | Auto on test changes |

### Manual/Nightly (4)
| Workflow | File | Trigger | Timeout |
|----------|------|---------|---------|
| Benchmarks | `benchmark.yml` | nightly schedule, manual, labeled | 10-45m |
| Nightly Deep Tests | `nightly.yml` | manual | 30-45m |
| Nightly Aspirational | `nightly-aspirational.yml` | manual | 30m |
| Docs Truth | `docs-truth.yml` | manual, labeled | 15m |

### Redundant/Consolidation Targets (2)
| Workflow | File | Trigger | Issue |
|----------|------|---------|-------|
| Rust CI | `rust.yml` | manual | Duplicates `ci.yml`, same name as `rust-ci.yml` |
| Rust CI | `rust-ci.yml` | manual | Duplicates `ci.yml`, same name as `rust.yml` |

### Release/Publishing (5)
| Workflow | File | Trigger | Timeout |
|----------|------|---------|---------|
| Release | `release.yml` | tag push | None (low frequency) |
| Build Packages | `build-packages.yml` | tag push | None (low frequency) |
| Brew Bump | `brew-bump.yml` | release | None (low frequency) |
| Publish Extension | `publish-extension.yml` | release | None (low frequency) |
| VSCode Publish | `vscode-publish.yml` | release | None (low frequency) |

### Reusable Workflow (1)
| Workflow | File | Purpose |
|----------|------|---------|
| Rust Tier | `_rust-tier.yml` | Reusable tier runner with configurable timeout |

## Consolidation Plan (Week 3)

### Phase 1: Archive Redundant Workflows
1. **Archive `rust.yml`** - duplicates ci.yml functionality
2. **Archive `rust-ci.yml`** - duplicates ci.yml functionality
3. Both are manual-only, so disabling won't break PRs

### Phase 2: Consolidate by Purpose
Target state (6 workflows):
- `ci.yml` - Primary merge gate (required)
- `quality-checks.yml` - Label-gated quality checks
- `nightly.yml` - Scheduled comprehensive tests + benchmarks
- `release.yml` - All release/publishing (consolidate 5 into 1)
- `check-ignored.yml` - Auto-triggered policy check
- `_rust-tier.yml` - Reusable tier runner

### Items to Merge
- `test.yml` → absorb into `quality-checks.yml` under `ci:tests` label
- `comprehensive_tests.yml` → absorb into `quality-checks.yml`
- `lsp-tests.yml` → keep separate (distinct platform matrix)
- `benchmark.yml` → merge into `nightly.yml`
- `nightly-aspirational.yml` → merge into `nightly.yml`
- `property-tests.yml` → merge into `quality-checks.yml`
- `docs-truth.yml` → merge into `quality-checks.yml`

## Timeout Standards

| Job Type | Timeout |
|----------|---------|
| Format check | 10m |
| Clippy | 15-20m |
| Unit tests | 20-30m |
| LSP tests (platform matrix) | 30m |
| Coverage | 45m |
| Mutation testing | 60m |
| Benchmarks | 30-45m |
| Documentation | 15m |
| Security audit | 15m |

## Done Criteria

- [x] All PR-triggered workflows have timeouts
- [x] All manual/nightly workflows have timeouts
- [x] All workflows have concurrency cancellation
- [ ] `ci.yml` is the only required check
- [ ] Workflow count reduced to ~6
- [ ] All release workflows consolidated
