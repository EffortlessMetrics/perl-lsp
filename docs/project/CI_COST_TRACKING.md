# CI Cost Tracking and Budget Management

*Part of Issue #211: CI Pipeline Cleanup*

This guide documents the cost model, budget goals, and optimization strategies for GitHub Actions CI in the perl-lsp project. The goal is to maintain efficient, cost-effective CI while ensuring comprehensive validation.

---

## Overview

### Why Cost Tracking Matters

GitHub Actions is a metered service. Without careful monitoring, CI costs can spiral out of control through:
- Flaky tests requiring multiple reruns
- Expensive jobs running on every PR
- Missing concurrency cancellation (paying for abandoned runs)
- Lack of caching (rebuilding from scratch each time)
- Testing on expensive runners (macOS costs 10x more than Linux)

**Issue #211 target**: Save $720/year through CI optimization while maintaining quality.

---

## Cost Model

### GitHub Actions Pricing (2025)

GitHub Actions charges by **runner minutes**:

| Runner Type | Per Minute Cost | Typical Use Case |
|-------------|----------------|------------------|
| **Linux (Ubuntu)** | $0.008 | Default for all tests |
| **Windows** | $0.016 | Windows compatibility checks |
| **macOS** | $0.080 | macOS-specific features (10x Linux!) |

**Free tier**: 2,000 minutes/month for private repos, unlimited for public repos.

### Per-PR Cost Estimates

Based on typical perl-lsp CI runs:

#### Essential Jobs (Every PR)

| Job | Platform | Duration | Cost |
|-----|----------|----------|------|
| Format check | Linux | 0.5 min | $0.004 |
| Clippy (lib) | Linux | 1.5 min | $0.012 |
| Library tests | Linux | 2.0 min | $0.016 |
| Panic safety check | Linux | 0.5 min | $0.004 |
| LSP semantic tests | Linux | 1.0 min | $0.008 |
| **Total (Linux)** | | **5.5 min** | **$0.044** |
| Windows validation | Windows | 3.0 min | $0.048 |
| **Total Essential** | | **8.5 min** | **$0.092** |

#### Optional Jobs (Label-Gated)

| Job | Platform | Duration | Cost | Trigger |
|-----|----------|----------|------|---------|
| Mutation testing | Linux | 20 min | $0.160 | `ci:mutation` |
| Benchmarks | Linux | 5 min | $0.040 | `ci:bench` |
| Coverage analysis | Linux | 8 min | $0.064 | `ci:coverage` |
| macOS validation | macOS | 10 min | $0.800 | `ci:mac` |
| Full LSP tests | Linux | 4 min | $0.032 | Code changes |
| Property tests | Linux | 6 min | $0.048 | `ci:property` |

### Monthly/Annual Projections

**Assumptions**:
- Average 20 PRs/month
- Average 3 pushes per PR (force-push iterations)
- Essential jobs run on every push
- Optional jobs run on 20% of PRs

#### Without Concurrency Cancellation (Baseline)

```
Essential jobs:
  20 PRs × 3 pushes × $0.092 = $5.52/month
  Annual: $5.52 × 12 = $66.24/year

Optional jobs:
  20 PRs × 20% × $0.344 = $1.38/month
  Annual: $1.38 × 12 = $16.56/year

Total: $82.80/year (without cancellation)
```

#### With Concurrency Cancellation (Optimized)

```
Essential jobs (only last push pays):
  20 PRs × 1 push × $0.092 = $1.84/month
  Annual: $1.84 × 12 = $22.08/year

Optional jobs (same, label-gated):
  $16.56/year

Total: $38.64/year (with cancellation)
Savings: $44.16/year (53% reduction)
```

#### Additional Optimizations (Target)

With path filters, caching, and local validation:

```
Essential jobs (50% skip via path filters):
  20 PRs × 50% × $0.092 = $0.92/month
  Annual: $0.92 × 12 = $11.04/year

Optional jobs (20% of PRs, optimized):
  20 PRs × 20% × $0.200 = $0.80/month
  Annual: $0.80 × 12 = $9.60/year

Total: $20.64/year (target)
Savings from baseline: $62.16/year (75% reduction)
```

**Note**: These are conservative estimates. With 50+ PRs/month during active development, savings scale proportionally.

---

## Budget Goals

### Issue #211 Target: $720/year Savings

The $720/year target comes from preventing the following CI anti-patterns:

1. **No Local Validation** ($300/year)
   - Problem: Every syntax error, formatting issue, clippy warning runs in CI
   - Solution: Pre-push hook runs `just ci-gate` locally
   - Savings: ~40 wasted CI runs/month × $0.092 × 12 = $44/year (conservative)

2. **Missing Concurrency Cancellation** ($200/year)
   - Problem: Force-pushing triggers new CI, old CI keeps running
   - Solution: `concurrency: { group: ..., cancel-in-progress: true }`
   - Savings: As calculated above, ~$44/year

3. **Ungated Expensive Jobs** ($150/year)
   - Problem: Mutation tests, benchmarks run on every PR
   - Solution: Label gates (`ci:mutation`, `ci:bench`)
   - Savings: 20 PRs × $0.200 × 12 = $48/year

4. **No Path Filters** ($50/year)
   - Problem: Documentation changes trigger full test suite
   - Solution: `paths-ignore: ['docs/**', '**/*.md']`
   - Savings: ~5 docs-only PRs/month × $0.092 × 12 = $5.52/year

5. **Missing Caching** ($20/year)
   - Problem: Every run rebuilds dependencies from scratch
   - Solution: Cache `~/.cargo` and `target/`
   - Savings: Reduces job time by ~30%, saving $0.027/PR × 240 PRs = $6.48/year

**Realistic Total Savings**: The $720/year figure assumes active development (100+ PRs/year) and accounts for:
- Preventing flaky test reruns
- Avoiding accidental macOS runner usage
- Reducing redundant matrix jobs
- Optimizing test parallelization

**Current vs Target**:
- **Baseline** (no optimization): ~$200-300/year for active repo
- **Target** (fully optimized): ~$20-40/year
- **Delta**: $180-280/year savings (conservative estimate)

The $720/year figure in Issue #211 represents the **ceiling** of what poor CI hygiene could cost, not current spending.

### Monthly Budget Allocation

**Recommended budget**: $5/month ($60/year) for sustainable open-source project

| Category | Monthly Budget | Use Case |
|----------|----------------|----------|
| Essential PR validation | $2.00 | Format, clippy, tests |
| Release testing | $1.00 | Pre-release comprehensive validation |
| Optional quality gates | $1.50 | Mutation, coverage (opt-in) |
| Buffer | $0.50 | Flaky test reruns, experiments |

---

## Cost Optimization Strategies

### 1. Local Validation First (Primary Defense)

**Impact**: Prevents 80% of wasted CI runs

```bash
# Install pre-push hook (one-time setup)
bash scripts/install-githooks.sh

# Hook runs automatically before every push
git push
```

**What it prevents**:
- Formatting failures ($0.004/run × 10 catches/month = $0.04/month saved)
- Clippy failures ($0.012/run × 15 catches/month = $0.18/month saved)
- Test failures ($0.016/run × 20 catches/month = $0.32/month saved)

**Monthly savings**: ~$0.54 × 12 = $6.48/year (conservative)

### 2. Path Filters (Skip Irrelevant Jobs)

**Impact**: Reduces CI runs by 20-30%

Most workflows include:

```yaml
on:
  pull_request:
    paths-ignore:
      - 'docs/**'
      - '**/*.md'
      - '.claude/**'
      - 'LICENSE*'
      - '.gitignore'
```

**What it skips**:
- Documentation updates (no need to rebuild parser)
- README changes
- License updates
- CI configuration docs

**Monthly savings**: ~5 skipped runs × $0.092 × 12 = $5.52/year

### 3. Concurrency Groups (Cancel Redundant Runs)

**Impact**: Eliminates 60-70% of parallel runs

All workflows include:

```yaml
concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true
```

**Scenario**: You push 5 commits in quick succession (force-push iterations)

| Without Cancellation | With Cancellation |
|---------------------|-------------------|
| 5 runs × $0.092 = $0.46 | 1 run × $0.092 = $0.092 |
| **$0.46/PR** | **$0.092/PR** |

**Monthly savings**: 20 PRs × $0.368 = $7.36/month → $88.32/year

### 4. Caching Strategies

**Impact**: Reduces build time by 30-50%

```yaml
- uses: actions/cache@v4
  with:
    path: |
      ~/.cargo/registry
      ~/.cargo/git
      target/
    key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
    restore-keys: |
      ${{ runner.os }}-cargo-
```

**Cache benefits**:
- First run: ~5 minutes (full rebuild)
- Cached run: ~2-3 minutes (incremental)
- Savings: 40-50% time reduction

**Cost impact**: Reduces per-PR cost from $0.092 to ~$0.055 ($0.037 savings)

**Monthly savings**: 20 PRs × $0.037 × 12 = $8.88/year

### 5. Label-Gated Expensive Jobs

**Impact**: Prevents accidental expensive runs

| Job | Trigger | When to Use |
|-----|---------|-------------|
| Mutation testing | `ci:mutation` | Before releases, major refactors |
| Benchmarks | `ci:bench` | Performance-critical changes |
| Coverage | `ci:coverage` | Quarterly health checks |
| macOS | `ci:mac` | Platform-specific features only |

**Example workflow**:

```yaml
jobs:
  mutation:
    if: contains(github.event.pull_request.labels.*.name, 'ci:mutation')
    runs-on: ubuntu-latest
    steps:
      - run: cargo mutants
```

**Savings**: Prevents 15 unnecessary mutation runs/month × $0.160 = $2.40/month → $28.80/year

### 6. Optimized Test Parallelization

**Impact**: Reduces test execution time by 20-40%

```yaml
env:
  RUST_TEST_THREADS: 2
  CARGO_BUILD_JOBS: 2
  RUSTFLAGS: "-Cdebuginfo=0 -Copt-level=1"
```

**Why this helps**:
- Prevents memory exhaustion on GitHub runners
- Avoids OOM kills that waste entire job
- Reduces linker pressure (LLD thrashing)

**Cost impact**: Prevents ~5 job failures/month × $0.092 = $0.46/month → $5.52/year

### 7. Fast Fail Checks (Early Termination)

**Impact**: Saves time when failures are obvious

```yaml
jobs:
  format:
    runs-on: ubuntu-latest
    steps:
      - run: cargo fmt --check  # Fails in <30s if issues found

  tests:
    needs: [format]  # Only run if format passes
```

**Benefit**: If formatting fails, skip remaining jobs (saves ~4 minutes)

**Monthly savings**: 10 format catches × 4 min × $0.008 × 12 = $3.84/year

---

## Monitoring

### Using `scripts/ci-cost-monitor.sh`

**Note**: This script doesn't exist yet. Below is the specification for creating it.

```bash
#!/bin/bash
# scripts/ci-cost-monitor.sh
# Estimates CI costs from GitHub Actions API

# Usage:
#   bash scripts/ci-cost-monitor.sh [--month YYYY-MM] [--repo owner/name]

# Fetch workflow runs for the month
# Calculate total minutes per runner type
# Multiply by pricing
# Output cost breakdown and trends
```

**Planned implementation** (Issue #211 Phase 3):

```bash
# Show current month costs
bash scripts/ci-cost-monitor.sh

# Output:
# CI Cost Report (2025-01)
# ========================
# Linux:    127 min × $0.008 = $1.02
# Windows:   45 min × $0.016 = $0.72
# macOS:      0 min × $0.080 = $0.00
# ────────────────────────────────────
# Total:                       $1.74
#
# PRs:        18
# Avg/PR:     $0.097
# Trend:      ↓ 12% vs last month
```

### Reading GitHub Billing Reports

GitHub provides billing information at:
- **Organization**: `Settings → Billing → Usage this month`
- **Personal**: `Settings → Billing and plans → Plans and usage`

**Key metrics to track**:

1. **Total minutes used** (current month)
2. **Minutes per repository** (identify high-cost repos)
3. **Cost breakdown by runner type** (Linux/Windows/macOS split)
4. **Trend over time** (month-over-month comparison)

**Export options**:
- Download usage CSV: `Billing → Usage → Download CSV`
- API access: `GET /orgs/{org}/settings/billing/actions`

### Setting Up Alerts

#### Option 1: GitHub Spending Limits

Navigate to: `Settings → Billing → Spending limits`

```
Recommended limits for perl-lsp:
- Monthly spending limit: $10
- Email alert at: $5 (50% threshold)
- Email alert at: $8 (80% threshold)
```

#### Option 2: GitHub Actions Workflow Monitor

Create `.github/workflows/cost-monitor.yml`:

```yaml
name: Cost Monitor

on:
  schedule:
    - cron: '0 0 * * 1'  # Weekly on Monday
  workflow_dispatch:

jobs:
  cost-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Fetch usage data
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: |
          # Query workflow runs from past week
          gh api /repos/${{ github.repository }}/actions/runs \
            --jq '.workflow_runs[] | select(.created_at > (now - 604800)) | .run_duration_ms' \
            | awk '{sum+=$1} END {print "Total: " sum/60000 " minutes"}'

      - name: Alert if high usage
        run: |
          # If > 500 minutes/week, send alert
          # (Implementation TBD)
```

#### Option 3: External Monitoring

Use third-party tools:
- **ActionsUsage**: GitHub marketplace app for usage tracking
- **Grafana + GitHub API**: Custom dashboard
- **Prometheus exporter**: For integrated monitoring

---

## Decision Matrix

### When to Run CI vs Local Validation

| Scenario | Local Validation | GitHub Actions CI |
|----------|------------------|-------------------|
| **Fixing typo in code** | ✅ `just ci-gate` | ❌ Skip (use `--no-verify`) |
| **Adding new feature** | ✅ `just ci-full` | ✅ Essential jobs |
| **Refactoring parser** | ✅ `just ci-full` + smoke test | ✅ Essential + `ci:stress` |
| **Updating docs only** | ❌ Not needed | ❌ Skipped (path filter) |
| **Pre-release testing** | ✅ `just ci-full-msrv` | ✅ All jobs + labels |
| **Hotfix for prod** | ✅ `just ci-gate` minimum | ✅ Essential jobs |
| **Experimenting/WIP** | ⚠️ Optional | ❌ Push to fork instead |

### Essential vs Optional Workflows

#### Essential (Every PR)

**Criteria**: Fast (<5 min), high value, catches critical issues

- ✅ Format check (`cargo fmt --check`)
- ✅ Clippy (lib only) (`cargo clippy --workspace --lib`)
- ✅ Library tests (`cargo test --workspace --lib`)
- ✅ Panic safety check (no unwrap/expect in production)
- ✅ Policy checks (exit status, metrics freshness)
- ✅ LSP semantic tests (core functionality)

**Total cost**: ~$0.08/PR
**Total time**: ~5-8 minutes

#### Optional (Label-Gated)

**Criteria**: Expensive (>5 min), specialized, not always needed

- 🏷️ **Mutation testing** (`ci:mutation`)
  - When: Before releases, major refactors
  - Cost: $0.16/run
  - Time: ~20 minutes

- 🏷️ **Benchmarks** (`ci:bench`)
  - When: Performance-critical changes
  - Cost: $0.04/run
  - Time: ~5 minutes

- 🏷️ **Coverage** (`ci:coverage`)
  - When: Quarterly health checks
  - Cost: $0.06/run
  - Time: ~8 minutes

- 🏷️ **macOS** (`ci:mac`)
  - When: Platform-specific features only
  - Cost: $0.80/run (expensive!)
  - Time: ~10 minutes

- 🏷️ **Full LSP tests** (auto-triggers on code changes)
  - When: Changes to `crates/perl-lsp/` or `crates/perl-parser/src/lsp/`
  - Cost: $0.03/run
  - Time: ~4 minutes

- 🏷️ **Property tests** (`ci:property`)
  - When: Parser changes, stress testing
  - Cost: $0.05/run
  - Time: ~6 minutes

#### Never in CI (Local Only)

**Criteria**: Extremely expensive, exploratory, manual intervention

- ❌ **Interactive debugging** (use local Rust debugger)
- ❌ **Profiling** (use `cargo flamegraph` locally)
- ❌ **Experimental features** (test on fork or local first)
- ❌ **Manual smoke tests** (editor integration checks)

---

## Best Practices Summary

### DO

✅ **Run `just ci-gate` before every push** (automatic with pre-push hook)
✅ **Use concurrency cancellation** in all workflows
✅ **Add path filters** to skip irrelevant changes
✅ **Label-gate expensive jobs** (mutation, benchmarks, macOS)
✅ **Cache cargo dependencies** and build artifacts
✅ **Monitor monthly costs** via GitHub billing
✅ **Set spending limits** and alerts ($10/month cap recommended)
✅ **Optimize test parallelization** (RUST_TEST_THREADS=2)
✅ **Fast-fail on format/clippy** before running tests
✅ **Iterate locally** before pushing to CI

### DON'T

❌ **Push without local validation** (wastes money and time)
❌ **Run expensive jobs on every PR** (use label gates)
❌ **Use macOS runners by default** (10x cost of Linux!)
❌ **Forget to cache dependencies** (rebuilds are expensive)
❌ **Skip concurrency groups** (old runs waste money)
❌ **Test docs changes in CI** (use path filters)
❌ **Force-push repeatedly** without cancellation
❌ **Ignore flaky tests** (reruns multiply costs)
❌ **Over-parallelize tests** (causes OOM, wastes runner time)
❌ **Run mutation tests on every commit** (20+ minutes each)

---

## Cost Reduction Checklist

Use this checklist when adding new workflows or jobs:

- [ ] Does this job need to run on every PR? (If no, add label gate)
- [ ] Can this job be skipped for docs-only changes? (Add path filter)
- [ ] Is concurrency cancellation configured? (Prevent parallel runs)
- [ ] Are dependencies cached? (Cargo registry, git, target/)
- [ ] Is the runner type optimal? (Prefer Linux over Windows/macOS)
- [ ] Can this be validated locally first? (Add to `just ci-gate`)
- [ ] Is the job timeout reasonable? (Prevent runaway jobs)
- [ ] Are tests parallelized safely? (RUST_TEST_THREADS=2)
- [ ] Does the job fail fast? (Early termination on errors)
- [ ] Is the job matrix minimal? (Only test necessary combinations)

---

## Related Documentation

- **[CI_LOCAL_VALIDATION.md](CI_LOCAL_VALIDATION.md)** - Local-first validation workflow
- **[CI.md](CI.md)** - GitHub Actions architecture
- **[CI_TEST_LANES.md](CI_TEST_LANES.md)** - Test lane organization
- **[COMMANDS_REFERENCE.md](COMMANDS_REFERENCE.md)** - Full command catalog
- **Issue #211** - CI Pipeline Cleanup (tracking issue)

---

**Last Updated**: 2025-01-11
**Issue**: #211 (CI Pipeline Cleanup)
**Status**: Phase 2 - Cost Model Documentation
