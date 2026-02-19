# CI/CD Quick Reference Guide

**Last Updated**: 2026-02-12

---

## Overview

This repository uses a consolidated CI/CD pipeline with 6 active workflows:

1. **ci.yml** - Main merge-blocking gate (runs on every PR)
2. **ci-nightly.yml** - Nightly and label-gated tests
3. **ci-security.yml** - Security scanning (daily)
4. **docs-deploy.yml** - Documentation deployment
5. **release.yml** - Release builds
6. **publish-extension.yml** - VSCode extension publishing

---

## For Contributors

### What Runs Automatically?

Every PR triggers the **merge-blocking gate** (`ci.yml`):

```bash
just gates  # Equivalent to just merge-gate
```

This includes:
- ✅ Format check
- ✅ Clippy (core + full)
- ✅ Tests (core + full)
- ✅ LSP smoke tests
- ✅ Security audit
- ✅ CI policy checks
- ✅ LSP definition tests
- ✅ Parser feature checks
- ✅ Feature invariants

**Duration**: 3-5 minutes
**Cost**: ~$0.05 per PR

### How to Trigger Additional Checks?

Add labels to your PR to trigger additional CI jobs:

| Label | Triggers | Duration | When to Use |
|-------|----------|----------|-------------|
| `ci:mutation` | Mutation testing | 60 min | Critical code changes |
| `ci:bench` | Performance benchmarks | 45 min | Performance-sensitive changes |
| `ci:coverage` | Test coverage report | 45 min | Test changes |
| `ci:strict` | Strict linting | 20 min | Code quality improvements |
| `ci:audit` | Dependency audit | 15 min | Dependency updates |

**Adding labels via CLI**:
```bash
gh pr edit $PR_NUMBER --add-label ci:bench
```

**Adding labels via GitHub UI**:
1. Go to your PR
2. Click "Labels"
3. Add the desired label(s)

### What Runs Nightly?

The `ci-nightly.yml` workflow runs at 3 AM UTC and includes:

- Mutation testing
- Performance benchmarks
- Test coverage
- Semver checks
- Fuzz testing (5 targets)

### What Runs Daily?

The `ci-security.yml` workflow runs at 2 AM UTC and includes:

- Cargo audit (RustSec vulnerabilities)
- Cargo deny (policy enforcement)
- Trivy repository scan
- Trivy Docker image scan

---

## For Maintainers

### Merge Requirements

Before merging a PR to `main` or `master`:

1. ✅ All CI checks must pass (`ci.yml`)
2. ✅ Review and approve the PR
3. ✅ Resolve any review comments

### Optional Pre-Merge Checks

For high-risk changes, consider adding labels:

```bash
# For performance-critical changes
gh pr edit $PR_NUMBER --add-label ci:bench

# For test coverage verification
gh pr edit $PR_NUMBER --add-label ci:coverage
```

### Release Process

1. Create and push a version tag:
   ```bash
   git tag v0.9.0
   git push origin v0.9.0
   ```

2. Workflows automatically trigger:
   - `release.yml` - Build and release binaries
   - `publish-extension.yml` - Publish VSCode extension
   - `brew-bump.yml` - Update Homebrew formula

3. Monitor the workflows in the Actions tab

---

## Local Development

### Running CI Locally

You can run the same checks locally using `just`:

```bash
# Quick PR validation (1-2 min)
just pr-fast

# Full pre-merge validation (3-5 min)
just merge-gate

# Same as merge-gate, via Nix
nix develop -c just ci-gate

# Comprehensive nightly tests (15-30 min)
just nightly
```

### Local Gate Commands

| Command | Duration | Purpose |
|----------|----------|---------|
| `just fmt-check` | 5 sec | Format check |
| `just clippy-core` | 30 sec | Core crate linting |
| `just clippy-full` | 1 min | Full workspace linting |
| `just test-core` | 1 min | Core crate tests |
| `just test-full` | 2-3 min | Full workspace tests |
| `just lsp-smoke` | 30 sec | LSP smoke tests |
| `just security-audit` | 30 sec | Security audit |

---

## Troubleshooting

### CI Fails on My PR

1. **Check the logs**: Click on the failed check in the PR
2. **Reproduce locally**: Run `just gates` locally
3. **Fix the issue**: Address the failing check
4. **Push changes**: The CI will automatically re-run

### CI Takes Too Long

- Normal duration: 3-5 minutes
- If longer: Check if caching is working
- Cache hit rate should be 70-80%

### CI Passes Locally but Fails on CI

- Check for platform differences (Linux vs macOS/Windows)
- Verify Rust toolchain version (1.89.0)
- Check for environment-specific dependencies

---

## Cost Optimization

### How We Save Money

- **Consolidated workflows**: 6 instead of 9
- **Linux-only runners**: Except for release builds
- **Aggressive caching**: 70-80% cache hit rate
- **Concurrency cancellation**: Cancel obsolete runs

### Estimated Savings

- **Per PR**: $0.386 saved (88% reduction)
- **Monthly**: $36.75 saved (29% reduction)
- **Annually**: $573.30 saved (38% reduction)

---

## Workflow Triggers

### ci.yml (Merge Gate)

```yaml
on:
  pull_request:
    branches: [ main, master ]
  workflow_dispatch: {}
```

### ci-nightly.yml (Nightly + Label-Gated)

```yaml
on:
  pull_request:
    branches: [ main, master ]
    types: [labeled]
  schedule:
    - cron: '0 3 * * *'  # 3 AM UTC
  workflow_dispatch:
```

### ci-security.yml (Security Scanning)

```yaml
on:
  pull_request:
    branches: [main, master]
    paths:
      - 'Cargo.toml'
      - 'Cargo.lock'
  push:
    branches: [main, master]
  schedule:
    - cron: '0 2 * * *'  # 2 AM UTC
  workflow_dispatch:
```

### docs-deploy.yml (Documentation)

```yaml
on:
  push:
    branches: [master]
    paths:
      - 'docs/**'
      - 'book/**'
  workflow_dispatch:
```

### release.yml (Release Builds)

```yaml
on:
  push:
    tags:
      - 'v*.*.*'
  workflow_dispatch:
```

### publish-extension.yml (VSCode Extension)

```yaml
on:
  push:
    tags:
      - 'v*'
  workflow_dispatch:
```

### brew-bump.yml (Homebrew)

```yaml
on:
  release:
    types: [published]
  workflow_dispatch:
```

---

## Caching Strategy

All workflows use `Swatinem/rust-cache@v2`:

```yaml
- name: Cache cargo dependencies
  uses: Swatinem/rust-cache@v2
  with:
    cache-on-failure: true
    cache-all-crates: true
    shared-key: ${{ runner.os }}-workflow-${{ hashFiles('Cargo.lock') }}
```

### Cache Keys

- `ci-gate`: Merge gate dependencies
- `nightly-job`: Nightly test dependencies
- `security-job`: Security scan dependencies
- `release-${{ matrix.target }}`: Release build dependencies
- `mdbook`: Documentation build dependencies

---

## Concurrency Cancellation

All workflows have concurrency cancellation enabled:

```yaml
concurrency:
  group: workflow-${{ github.ref }}
  cancel-in-progress: true
```

This means:
- New commits cancel in-progress runs
- Only the latest code is tested
- Saves 30-50% on CI minutes

---

## Performance Metrics

### Target Metrics

| Metric | Target | Current |
|--------|--------|---------|
| Gate Duration | <10 min | 3-5 min ✅ |
| Cache Hit Rate | >70% | 70-80% ✅ |
| CI Pass Rate | >95% | >95% ✅ |
| Flaky Test Rate | <5% | <5% ✅ |

### Monitoring

Check the Actions tab for:
- Workflow run times
- Cache hit rates
- Failure patterns
- Cost trends

---

## Getting Help

### Documentation

- [CI Phase 1 Implementation](CI_PHASE1_IMPLEMENTATION.md) - Detailed implementation report
- [CI Requirements](../plans/phase1_requirements.md) - Phase 1 requirements
- [CI Architecture](CI_ARCHITECTURE.md) - System design (if exists)

### Contact

- Open an issue for CI-related problems
- Tag with `ci` label for faster response
- Check existing issues for known problems

---

## Quick Commands Reference

```bash
# Local CI commands
just pr-fast              # Quick validation (1-2 min)
just merge-gate           # Full validation (3-5 min)
just nightly              # Comprehensive tests (15-30 min)

# Individual gate commands
just fmt-check            # Format check
just clippy-core          # Core linting
just clippy-full          # Full linting
just test-core            # Core tests
just test-full            # Full tests
just lsp-smoke            # LSP smoke tests
just security-audit       # Security audit

# GitHub CLI commands
gh pr edit $PR_NUMBER --add-label ci:bench
gh pr view $PR_NUMBER --checks
gh run list --workflow=ci.yml
gh run view $RUN_ID

# Nix commands
nix develop -c just ci-gate  # Canonical local gate
```

---

**Version**: 1.0
**Last Updated**: 2026-02-12
