# CI Workflow Consolidation Plan

> **Purpose**: Reduce workflow count, eliminate redundancy, and improve CI maintainability
> **Created**: 2026-01-24
> **Related**: Issue #211 CI Pipeline Cleanup, `.ci/WORKFLOW_MANIFEST.md`

---

## 1. Current State Summary

### Workflow Inventory (20 workflows)

| Workflow | Trigger | Purpose | Runtime | Status |
|----------|---------|---------|---------|--------|
| `ci.yml` | PR to main/master | Primary merge gate (just gates) | 8-12m | **KEEP - Primary** |
| `test.yml` | PR + label `ci:tests` | Cross-platform tests, nextest | 15-20m | CONSOLIDATE |
| `lsp-tests.yml` | PR + label `ci:lsp` + paths | LSP tests, coverage, benchmark | 20-30m | CONSOLIDATE |
| `comprehensive_tests.yml` | PR + label `ci:all-tests` | Full test suite | 25-30m | CONSOLIDATE |
| `property-tests.yml` | PR + label `ci:property` | Property/fuzz tests | 10-30m | CONSOLIDATE |
| `nightly.yml` | manual | Deep property tests, corpus | 30-45m | **KEEP - Nightly** |
| `nightly-aspirational.yml` | manual | Aspirational features | 15-25m | ARCHIVE |
| `check-ignored.yml` | PR/push + paths | Ignored test count | 2-3m | CONSOLIDATE |
| `quality-checks.yml` | PR + labels | Coverage, audit, mutation | 5-60m | CONSOLIDATE |
| `rust-strict.yml` | PR + label `ci:strict` | Strict clippy, semver | 10-15m | CONSOLIDATE |
| `ci-expensive.yml` | PR + labels | Mutation, benchmarks | 30-60m | **KEEP - Expensive** |
| `docs-truth.yml` | PR + label `ci:docs-truth` | Documentation sync | 3-5m | CONSOLIDATE |
| `benchmark.yml` | PR + label `ci:bench` | Performance benchmarks | 15-30m | MERGE into expensive |
| `rust-ci.yml` | manual only | Legacy tests (xtask) | 30-45m | **ARCHIVE** |
| `rust.yml` | manual only | Legacy tests (tree-sitter) | 40-60m | **ARCHIVE** |
| `release.yml` | tag v*.*.* | Multi-platform builds | 20-30m | **KEEP - Release** |
| `build-packages.yml` | tag v* | Linux deb/rpm packages | 10-15m | **KEEP - Release** |
| `brew-bump.yml` | release published | Homebrew formula update | 5-10m | **KEEP - Release** |
| `publish-extension.yml` | tag v* | VS Code publish | 5-10m | MERGE |
| `vscode-publish.yml` | tag v*.*.* | VS Code publish (duplicate) | 5-10m | MERGE |

### Identified Redundancies

1. **`publish-extension.yml` + `vscode-publish.yml`**: Nearly identical, different tag patterns
2. **`rust-ci.yml` + `rust.yml`**: Dead workflows (broken references, manual-only)
3. **`benchmark.yml` + `ci-expensive.yml` benches job**: Duplicate benchmark functionality
4. **`test.yml` + `lsp-tests.yml` + `comprehensive_tests.yml`**: Overlapping test jobs
5. **`quality-checks.yml` + `rust-strict.yml`**: Overlapping strict clippy/audit

### Current CI Metrics (Last 30 Days)

From `.ci/ci_baseline.json`:

| Metric | Value |
|--------|-------|
| Total Runs | 200 |
| Total Billable Minutes | 970m |
| Overall Success Rate | 2.8% (many failures/skips) |

**Top Consumers:**
- Quality Checks: 736m (75.9%)
- Nightly Deep Tests: 118m (12.2%)
- Property Tests: 43m (4.4%)
- Nightly Aspirational: 36m (3.7%)

---

## 2. Target State

### Consolidated Workflow Structure

```
.github/workflows/
├── pr-checks.yml        # PR-fast tier (required for merge)
├── merge-gate.yml       # Post-merge validation (main branch)
├── extended.yml         # Label-gated extended testing
├── nightly.yml          # Scheduled deep tests
├── release.yml          # Release automation (tag-triggered)
└── manual.yml           # Manual dispatch utilities
```

### Tier Definitions

#### Tier 1: PR-Fast (`pr-checks.yml`) - REQUIRED
- **Trigger**: Every PR
- **Runtime Target**: < 10 minutes
- **Jobs**:
  - `gate`: fmt, clippy (first-party), core tests via `just gates`
  - `ignored-check`: Verify ignored test baseline (cheap, 2-3m)

#### Tier 2: Merge-Gate (`merge-gate.yml`) - Post-Merge
- **Trigger**: Push to main/master only
- **Runtime Target**: < 15 minutes
- **Jobs**:
  - `full-tests`: Workspace tests with nextest
  - `lsp-smoke`: LSP server smoke test
  - `docs-check`: Documentation link validation

#### Tier 3: Extended (`extended.yml`) - Label-Gated
- **Trigger**: PR with specific labels
- **Labels**:
  - `ci:tests` -> Cross-platform tests (Ubuntu/Windows)
  - `ci:lsp` -> Full LSP test suite
  - `ci:property` -> Property tests (standard + extended)
  - `ci:strict` -> Strict clippy, tautology, determinism
  - `ci:coverage` -> Coverage report
  - `ci:audit` -> Security audit + semver check
  - `ci:docs-truth` -> Documentation sync validation

#### Tier 4: Expensive (`ci-expensive.yml`) - Label-Gated, Heavy
- **Trigger**: PR with `ci:mutation` or `ci:bench`
- **Jobs**:
  - `mutation`: Mutation testing (cargo-mutants)
  - `benchmarks`: Full benchmark suite with comparison

#### Tier 5: Nightly (`nightly.yml`) - Scheduled
- **Trigger**: Daily 3 AM UTC + manual
- **Jobs**:
  - `deep-property`: 2048 case proptest runs
  - `corpus-validation`: Full corpus lint and stats
  - `edge-discovery`: Edge case exploration

#### Tier 6: Release (`release.yml`) - Tag-Triggered
- **Trigger**: Push tag `v*.*.*` + manual
- **Jobs**:
  - `build`: Multi-platform binary builds (7 targets)
  - `packages`: Linux deb/rpm
  - `vscode`: VS Code extension publish
  - `homebrew`: Formula bump PR

---

## 3. Consolidation Mapping

### Workflows to Archive (Remove)

| Workflow | Reason | Action |
|----------|--------|--------|
| `rust-ci.yml` | Dead (references develop/rust-conversion branches, manual-only) | Delete |
| `rust.yml` | Dead (references tree-sitter-perl dir, manual-only) | Delete |
| `nightly-aspirational.yml` | Features dormant, 100% failure rate | Archive to `.archive/` |

### Workflows to Merge

| Source | Target | Notes |
|--------|--------|-------|
| `publish-extension.yml` | `release.yml` | Add as job, use `v*.*.*` pattern |
| `vscode-publish.yml` | `release.yml` | Merge into publish-extension job |
| `benchmark.yml` | `ci-expensive.yml` | Already has `benches` job; merge extra jobs |
| `test.yml` | `extended.yml` | Under `ci:tests` label |
| `lsp-tests.yml` | `extended.yml` | Under `ci:lsp` label |
| `comprehensive_tests.yml` | `extended.yml` | Under `ci:all-tests` (alias to `ci:tests`) |
| `check-ignored.yml` | `pr-checks.yml` | Add as fast job |
| `quality-checks.yml` | `extended.yml` | Split across labels |
| `rust-strict.yml` | `extended.yml` | Under `ci:strict` label |
| `docs-truth.yml` | `extended.yml` | Under `ci:docs-truth` label |

### Matrix Reduction Strategy

**Current Problem**: `lsp-tests.yml` runs 2 OS x 3 toolchains = 6 matrix cells

**Target**:
- PR-fast: Ubuntu stable only (1 cell)
- Extended `ci:tests`: Ubuntu + Windows, stable only (2 cells)
- Extended `ci:lsp`: Ubuntu stable + nightly (2 cells, nightly experimental)
- Nightly: Full matrix with beta/nightly on Ubuntu only

---

## 4. Reusable Components

### Composite Actions to Create

#### `.github/actions/rust-setup/action.yml`
```yaml
name: 'Rust Setup'
description: 'Install Rust toolchain with caching'
inputs:
  toolchain:
    default: 'stable'
  components:
    default: 'rustfmt, clippy'
runs:
  using: composite
  steps:
    - uses: dtolnay/rust-toolchain@stable
      with:
        toolchain: ${{ inputs.toolchain }}
        components: ${{ inputs.components }}
    - uses: Swatinem/rust-cache@v2
      with:
        cache-on-failure: true
```

#### `.github/actions/lockfile-check/action.yml`
```yaml
name: 'Lockfile Validation'
description: 'Validate Cargo.lock and --locked policy'
runs:
  using: composite
  steps:
    - name: Validate Cargo.lock
      shell: bash
      run: |
        if [ ! -f Cargo.lock ]; then
          echo "::error::Cargo.lock required"
          exit 1
        fi
        cargo metadata --format-version=1 --locked >/dev/null
    - name: Check --locked policy
      shell: bash
      run: |
        if grep -RIn --exclude-dir=target --include="*.yml" \
           -E 'cargo (build|test|install|nextest|bench|clippy|doc|run|check|publish)\b' \
           .github/workflows | grep -v '\-\-locked' | grep -q .; then
          echo "::error::Found cargo without --locked"
          exit 1
        fi
```

#### `.github/actions/nextest-run/action.yml`
```yaml
name: 'Nextest Run'
description: 'Run tests with nextest and CI guards'
inputs:
  package:
    default: '--workspace'
  test-threads:
    default: '2'
  skip-patterns:
    default: 'crash_reproducer_test fuzz_regression_test'
runs:
  using: composite
  steps:
    - uses: taiki-e/install-action@nextest
    - name: Run tests
      shell: bash
      env:
        CARGO_BUILD_JOBS: 2
        RUSTFLAGS: "-Cdebuginfo=0 -Copt-level=1 --cfg ci"
      run: |
        SKIP_ARGS=""
        for pattern in ${{ inputs.skip-patterns }}; do
          SKIP_ARGS="$SKIP_ARGS --skip $pattern"
        done
        cargo nextest run --locked ${{ inputs.package }} \
          --test-threads=${{ inputs.test-threads }} \
          -- $SKIP_ARGS
```

### Reusable Workflows (workflow_call)

#### `.github/workflows/_test-matrix.yml`
```yaml
name: Test Matrix (Reusable)
on:
  workflow_call:
    inputs:
      os-matrix:
        type: string
        default: '["ubuntu-22.04"]'
      toolchain-matrix:
        type: string
        default: '["stable"]'
      package:
        type: string
        default: '--workspace'

jobs:
  test:
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: ${{ fromJson(inputs.os-matrix) }}
        toolchain: ${{ fromJson(inputs.toolchain-matrix) }}
    steps:
      - uses: actions/checkout@v4
      - uses: ./.github/actions/rust-setup
        with:
          toolchain: ${{ matrix.toolchain }}
      - uses: ./.github/actions/nextest-run
        with:
          package: ${{ inputs.package }}
```

### Shared Caching Strategy

**Key Format**: `${{ runner.os }}-${{ matrix.toolchain || 'stable' }}-${{ hashFiles('Cargo.lock') }}`

**Cache Separation**:
- `rust-cache`: Cargo registry + target directory
- Upload artifacts with 1-day retention for PR, 7-day for release

---

## 5. Migration Plan

### Phase 1: Create New Workflows (Week 1)

**Deliverables**:
- [ ] Create `.github/actions/rust-setup/action.yml`
- [ ] Create `.github/actions/lockfile-check/action.yml`
- [ ] Create `.github/actions/nextest-run/action.yml`
- [ ] Create `pr-checks.yml` (mirrors current `ci.yml` + ignored check)
- [ ] Create `extended.yml` with all label-gated jobs

**Validation**:
- Run new workflows on a test branch
- Compare output with existing workflows
- Verify receipt generation still works

### Phase 2: Parallel Operation (Week 2)

**Deliverables**:
- [ ] Enable `pr-checks.yml` as non-required check
- [ ] Enable `extended.yml` as non-required check
- [ ] Monitor both old and new workflows side-by-side
- [ ] Create `merge-gate.yml` for post-merge validation

**Validation**:
- No regression in test coverage
- Receipt artifacts match
- No increase in CI minutes

### Phase 3: Branch Protection Redirect (Week 3)

**Deliverables**:
- [ ] Update branch protection to require `pr-checks.yml` instead of `ci.yml`
- [ ] Document label usage in CONTRIBUTING.md
- [ ] Merge duplicate VS Code publish workflows into `release.yml`
- [ ] Archive `rust-ci.yml`, `rust.yml`, `nightly-aspirational.yml`

**Validation**:
- All PRs use new workflow
- No broken merges
- Release workflow still works

### Phase 4: Cleanup (Week 4)

**Deliverables**:
- [ ] Delete archived workflows (move to `.archive/` first)
- [ ] Delete superseded workflows (`test.yml`, `lsp-tests.yml`, etc.)
- [ ] Update `WORKFLOW_MANIFEST.md`
- [ ] Update CI baseline metrics

**Validation Checkpoints**:
1. Gate receipt still generated correctly
2. All tests that passed before still pass
3. CI minutes reduced by target amount
4. No manual intervention needed for PRs

---

## 6. Cost Controls

### Concurrency Settings (All Workflows)

```yaml
concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true
```

### Path Filters

| Workflow | Path Filter |
|----------|-------------|
| `pr-checks.yml` | None (always run) |
| `extended.yml` `ci:lsp` | `crates/perl-parser/**`, `crates/perl-lsp/**` |
| `check-ignored` job | `crates/*/tests/**`, `ci/ignored_baseline.txt` |
| `docs-truth` job | `docs/**`, `*.md`, `artifacts/**` |

### Label-Gated Heavy Jobs

| Label | Jobs Enabled | Typical Runtime |
|-------|--------------|-----------------|
| `ci:tests` | Cross-platform test matrix | 15-20m |
| `ci:lsp` | Full LSP test suite | 20-30m |
| `ci:property` | Property tests (256 cases) | 10-15m |
| `ci:strict` | Strict clippy + determinism | 15-20m |
| `ci:coverage` | Coverage report | 10-15m |
| `ci:mutation` | Mutation testing | 30-60m |
| `ci:bench` | Full benchmarks | 15-30m |
| `ci:docs-truth` | Documentation sync | 3-5m |
| `ci:semver` | Semver compatibility | 5-10m |
| `ci:audit` | Security audit | 3-5m |

### Timeout Settings

| Tier | Timeout |
|------|---------|
| PR-fast | 15 minutes |
| Extended | 30 minutes |
| Expensive | 90 minutes |
| Nightly | 60 minutes |
| Release | 45 minutes per job |

---

## 7. Expected Outcomes

### Workflow Count Reduction

| Metric | Before | After | Change |
|--------|--------|-------|--------|
| Total Workflows | 20 | 6 | -70% |
| PR-triggered | 12 | 2 | -83% |
| Label-gated | 8 | 1 (multi-job) | -87% |
| Release | 5 | 1 (multi-job) | -80% |
| Nightly | 2 | 1 | -50% |

### Minutes Reduction Estimate

| Category | Before (monthly) | After (monthly) | Savings |
|----------|------------------|-----------------|---------|
| PR workflows | ~400m | ~150m | ~250m |
| Quality checks | ~736m | ~200m | ~536m |
| Nightly | ~154m | ~100m | ~54m |
| **Total** | **~1290m** | **~450m** | **~840m (65%)** |

**Annual Savings Estimate**: ~$840-1000 (at $0.008/min)

### Improved Maintainability

1. **Single source of truth**: One workflow per tier
2. **Reusable components**: DRY principle via composite actions
3. **Clear labeling**: Predictable behavior based on PR labels
4. **Documented triggers**: WORKFLOW_MANIFEST.md auto-updated
5. **Fast feedback**: PR-fast tier under 10 minutes

---

## 8. Risk Mitigation

### Rollback Plan

Each phase has a rollback:
- **Phase 1**: Delete new files, no impact
- **Phase 2**: Disable new workflows, keep old
- **Phase 3**: Restore branch protection rules
- **Phase 4**: Restore from `.archive/`

### Monitoring

- Compare CI minutes weekly during migration
- Track PR merge time (should decrease)
- Monitor test pass rate (should improve or stay same)

### Communication

- Announce label system in PR template
- Document in CONTRIBUTING.md
- Add label descriptions to GitHub repository settings

---

## Appendix A: Workflow Trigger Reference

### Current Trigger Patterns

```yaml
# Every PR
on: pull_request: branches: [main, master]

# Label-gated PR
on: pull_request: types: [labeled]
jobs: job: if: contains(github.event.pull_request.labels.*.name, 'ci:label')

# Post-merge
on: push: branches: [main, master]

# Tag release
on: push: tags: ['v*.*.*']

# Scheduled
on: schedule: - cron: '0 3 * * *'

# Manual
on: workflow_dispatch: {}
```

### Target Trigger Patterns

```yaml
# pr-checks.yml
on:
  pull_request:
    branches: [main, master]
  workflow_dispatch: {}

# extended.yml
on:
  pull_request:
    branches: [main, master]
    types: [labeled, synchronize, reopened]
  workflow_dispatch:
    inputs:
      labels:
        description: 'Labels to simulate (comma-separated)'
        type: string

# nightly.yml
on:
  schedule:
    - cron: '0 3 * * *'
  workflow_dispatch: {}

# release.yml
on:
  push:
    tags: ['v*.*.*']
  workflow_dispatch:
    inputs:
      tag:
        description: 'Release tag'
        required: true
```

---

## Appendix B: Label Quick Reference

| Label | Effect | When to Use |
|-------|--------|-------------|
| `ci:tests` | Full cross-platform tests | Changes to core crates |
| `ci:lsp` | LSP-specific tests | LSP feature changes |
| `ci:property` | Property/fuzz tests | Parser changes |
| `ci:strict` | Strict linting | Pre-release cleanup |
| `ci:coverage` | Coverage report | Coverage tracking |
| `ci:mutation` | Mutation testing | Test quality audit |
| `ci:bench` | Benchmarks | Performance changes |
| `ci:docs-truth` | Doc sync check | Documentation changes |
| `ci:semver` | API compatibility | Public API changes |
| `ci:audit` | Security audit | Dependency updates |
| `ci:all-tests` | Alias for ci:tests | Convenience |

---

*Document generated by CI consolidation analysis*
