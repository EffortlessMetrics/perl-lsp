# CI Workflow Manifest

> **Purpose**: Document all CI workflows to support Issue #211 CI Pipeline Cleanup
> **Generated**: 2025-12-31
> **Total Workflows**: 21

---

## Summary

| Category | Count | Monthly Cost Estimate |
|----------|-------|-----------------------|
| Core (Merge Gate) | 2 | Low (~$10-20) |
| Extended Testing | 6 | Medium (~$30-60) |
| Quality Assurance | 4 | Medium (~$20-40) |
| Release/Deployment | 5 | Low (~$5-10) |
| Deprecated/Redundant | 4 | High waste (~$30-50) |

**Total Estimated Savings from Cleanup**: $720/year (per Issue #211)

---

## 1. Core (Merge Gate - Required for PRs)

These workflows are essential and must run on every PR.

### ci.yml
| Attribute | Value |
|-----------|-------|
| **Name** | CI |
| **Purpose** | Primary merge gate: fmt, clippy, core tests, LSP tests, docs build |
| **Trigger** | PR to master, push to ci-pilot/** |
| **Estimated Runtime** | 8-12 minutes |
| **Cost Tier** | Low |
| **Recommendation** | **KEEP** - Primary merge gate, well-optimized with adaptive threading |

### test.yml
| Attribute | Value |
|-----------|-------|
| **Name** | Tests |
| **Purpose** | Cross-platform tests (Ubuntu/Windows), clippy, format check, nextest integration |
| **Trigger** | Push/PR to main/master |
| **Estimated Runtime** | 15-20 minutes (matrix: 2 OS x 1 toolchain) |
| **Cost Tier** | Medium |
| **Recommendation** | **KEEP** - Essential cross-platform validation |

---

## 2. Extended Testing

These workflows provide deeper test coverage beyond the merge gate.

### lsp-tests.yml
| Attribute | Value |
|-----------|-------|
| **Name** | LSP Tests |
| **Purpose** | Comprehensive LSP testing across platforms (Ubuntu/Windows) with stable/beta/nightly toolchains |
| **Trigger** | Push/PR to master/main (path-filtered: crates/perl-parser, crates/perl-lexer, crates/perl-lsp) |
| **Estimated Runtime** | 20-30 minutes (matrix: 2 OS x 3 toolchains) |
| **Cost Tier** | High |
| **Recommendation** | **MERGE** - Consider merging label-gated coverage/benchmark jobs into ci-expensive.yml |

### comprehensive_tests.yml
| Attribute | Value |
|-----------|-------|
| **Name** | Comprehensive Test Suite |
| **Purpose** | Full test suite with feature verification and orphaned test detection |
| **Trigger** | Push to main/master OR label `ci:all-tests` |
| **Estimated Runtime** | 25-30 minutes |
| **Cost Tier** | Medium |
| **Recommendation** | **KEEP** - Valuable for main branch; label-gating prevents PR waste |

### property-tests.yml
| Attribute | Value |
|-----------|-------|
| **Name** | Property Tests |
| **Purpose** | Property-based testing (proptest) with standard and extended (256 cases) modes |
| **Trigger** | Push/PR to main/master, nightly schedule (3 AM UTC) |
| **Estimated Runtime** | 10-15 minutes (standard), 20-30 minutes (extended) |
| **Cost Tier** | Medium |
| **Recommendation** | **KEEP** - Critical for parser correctness |

### nightly.yml
| Attribute | Value |
|-----------|-------|
| **Name** | Nightly Deep Tests |
| **Purpose** | Deep property tests (2048 cases), corpus validation, edge case discovery |
| **Trigger** | Schedule (daily 3 AM UTC), manual dispatch |
| **Estimated Runtime** | 30-45 minutes |
| **Cost Tier** | Medium |
| **Recommendation** | **KEEP** - Valuable nightly regression detection |

### nightly-aspirational.yml
| Attribute | Value |
|-----------|-------|
| **Name** | Nightly - Aspirational Feature Matrix |
| **Purpose** | Tests experimental/future features (constant-advanced, qw-variants, error-classifier-v2, etc.) |
| **Trigger** | Schedule (daily 3:17 AM UTC), manual dispatch |
| **Estimated Runtime** | 15-25 minutes |
| **Cost Tier** | Low |
| **Recommendation** | **ARCHIVE** - Features appear dormant; runs regardless of feature existence |

### check-ignored.yml
| Attribute | Value |
|-----------|-------|
| **Name** | Check Ignored Tests |
| **Purpose** | Tracks ignored test count baseline to prevent regression |
| **Trigger** | Push/PR to main/master (path-filtered: tests, src, ci/ignored_baseline.txt) |
| **Estimated Runtime** | 2-3 minutes |
| **Cost Tier** | Low |
| **Recommendation** | **KEEP** - Cheap, valuable hygiene check |

---

## 3. Quality Assurance

### quality-checks.yml
| Attribute | Value |
|-----------|-------|
| **Name** | Quality Checks |
| **Purpose** | Coverage (label-gated), tautology detection, security audit, semver check, strict clippy, mutation testing, test metrics |
| **Trigger** | Push/PR to main/master, various label gates |
| **Estimated Runtime** | 5-60 minutes (depends on jobs triggered) |
| **Cost Tier** | Medium-High |
| **Recommendation** | **KEEP** - Well-structured with label gates; mutation testing is informational |

### rust-strict.yml
| Attribute | Value |
|-----------|-------|
| **Name** | Rust Strict Quality Checks |
| **Purpose** | Strict mode: -D warnings, strict clippy, doc link validation, cargo-deny, semver checks |
| **Trigger** | Push/PR to master/main (label-gated: `ci:strict`, `ci:semver`) |
| **Estimated Runtime** | 10-15 minutes |
| **Cost Tier** | Low |
| **Recommendation** | **KEEP** - Valuable strict mode for quality PRs |

### ci-expensive.yml
| Attribute | Value |
|-----------|-------|
| **Name** | CI (Expensive) |
| **Purpose** | Expensive operations: mutation testing, benchmarks (label-gated) |
| **Trigger** | PR labeled/synchronize/reopened with `ci:mutation` or `ci:bench` |
| **Estimated Runtime** | 30-60+ minutes |
| **Cost Tier** | High |
| **Recommendation** | **KEEP** - Properly label-gated for expensive operations |

### docs-truth.yml
| Attribute | Value |
|-----------|-------|
| **Name** | docs-truth |
| **Purpose** | Validates documentation stays in sync with receipts (state.json), quarantine guard, snapshot guard |
| **Trigger** | PR (label-gated: `ci:docs-truth`) |
| **Estimated Runtime** | 3-5 minutes |
| **Cost Tier** | Low |
| **Recommendation** | **KEEP** - Valuable documentation integrity check |

---

## 4. Release/Deployment

### release.yml
| Attribute | Value |
|-----------|-------|
| **Name** | Release |
| **Purpose** | Multi-platform binary builds (7 targets), creates GitHub releases with checksums |
| **Trigger** | Push tag v*.*.*, manual dispatch |
| **Estimated Runtime** | 20-30 minutes |
| **Cost Tier** | Low (runs only on release) |
| **Recommendation** | **KEEP** - Essential release automation |

### build-packages.yml
| Attribute | Value |
|-----------|-------|
| **Name** | Build Linux Packages |
| **Purpose** | Builds .deb and .rpm packages for Linux distributions |
| **Trigger** | Push tag v*, manual dispatch |
| **Estimated Runtime** | 10-15 minutes |
| **Cost Tier** | Low (runs only on release) |
| **Recommendation** | **KEEP** - Valuable for Linux distribution |

### brew-bump.yml
| Attribute | Value |
|-----------|-------|
| **Name** | Homebrew Auto-Bump |
| **Purpose** | Automatically updates Homebrew formula on release |
| **Trigger** | Release published, manual dispatch |
| **Estimated Runtime** | 5-10 minutes |
| **Cost Tier** | Low (runs only on release) |
| **Recommendation** | **KEEP** - Valuable release automation |

### publish-extension.yml
| Attribute | Value |
|-----------|-------|
| **Name** | Publish VSCode Extension |
| **Purpose** | Publishes VS Code extension to Marketplace and Open VSX |
| **Trigger** | Push tag v*, manual dispatch |
| **Estimated Runtime** | 5-10 minutes |
| **Cost Tier** | Low (runs only on release) |
| **Recommendation** | **MERGE** - Duplicate with vscode-publish.yml |

### vscode-publish.yml
| Attribute | Value |
|-----------|-------|
| **Name** | Publish VS Code Extension |
| **Purpose** | Publishes VS Code extension to Marketplace (nearly identical to publish-extension.yml) |
| **Trigger** | Push tag v*.*.*, manual dispatch |
| **Estimated Runtime** | 5-10 minutes |
| **Cost Tier** | Low (runs only on release) |
| **Recommendation** | **MERGE** - Duplicate with publish-extension.yml; consolidate into one |

---

## 5. Deprecated/Candidates for Removal

### rust-ci.yml
| Attribute | Value |
|-----------|-------|
| **Name** | Rust CI |
| **Purpose** | Tests, lint, security audit, build matrix, performance, documentation, publish |
| **Trigger** | Push/PR to main/develop/rust-conversion |
| **Estimated Runtime** | 30-45 minutes (large matrix: 3 OS x 3 Rust versions) |
| **Cost Tier** | High |
| **Recommendation** | **ARCHIVE** - References non-existent branches (develop, rust-conversion), uses xtask commands that may not work, overlaps heavily with ci.yml and test.yml |

### rust.yml
| Attribute | Value |
|-----------|-------|
| **Name** | Rust CI |
| **Purpose** | Test matrix (stable/beta/nightly), lint, benchmark, comparison, security, coverage |
| **Trigger** | Push/PR to main/develop |
| **Estimated Runtime** | 40-60 minutes (large matrix: 3 Rust versions + 6 jobs) |
| **Cost Tier** | High |
| **Recommendation** | **ARCHIVE** - References tree-sitter-perl directory (doesn't exist), duplicate functionality with rust-ci.yml and ci.yml |

### benchmark.yml
| Attribute | Value |
|-----------|-------|
| **Name** | Benchmarks |
| **Purpose** | Performance benchmarks, parser comparison, memory profiling |
| **Trigger** | Push/PR to master/main (label-gated: `ci:bench`), manual dispatch |
| **Estimated Runtime** | 15-30 minutes |
| **Cost Tier** | Medium |
| **Recommendation** | **MERGE** - Consolidate with ci-expensive.yml benchmark job |

### badges.yml
| Attribute | Value |
|-----------|-------|
| **Name** | Update Badges |
| **Purpose** | Updates repository badges after CI/Benchmarks complete |
| **Trigger** | workflow_run (CI, Benchmarks completed), manual dispatch |
| **Estimated Runtime** | 2-3 minutes |
| **Cost Tier** | Low |
| **Recommendation** | **ARCHIVE** - Badge updates rarely needed; can be done manually or via release workflow |

---

## Consolidation Recommendations

### Immediate Actions (Week 1-2)

1. **Archive rust-ci.yml** - Broken references, heavy overlap
2. **Archive rust.yml** - Broken references (tree-sitter-perl), heavy overlap
3. **Merge publish-extension.yml and vscode-publish.yml** - Near-identical functionality
4. **Archive badges.yml** - Low value, can be manual

### Short-term Actions (Week 2-4)

5. **Merge benchmark.yml into ci-expensive.yml** - Consolidate label-gated expensive jobs
6. **Review nightly-aspirational.yml** - Verify features exist before running
7. **Move lsp-tests.yml coverage/benchmark jobs to ci-expensive.yml** - Better organization

### Metrics After Cleanup

| Metric | Before | After |
|--------|--------|-------|
| Total Workflows | 21 | ~14 |
| Redundant Workflows | 4 | 0 |
| Estimated Monthly Savings | - | ~$60 |
| Estimated Annual Savings | - | ~$720 |

---

## Workflow Dependency Graph

```
                    [Release Event]
                          |
          +---------------+---------------+
          |               |               |
    release.yml    build-packages.yml  brew-bump.yml
          |                               |
          +-------------------------------+
                          |
              [publish-extension.yml OR vscode-publish.yml]
                          |
                    [badges.yml]

    [PR/Push Event]
          |
    +-----+-----+
    |           |
  ci.yml    test.yml
    |           |
    +-----+-----+
          |
    [Label-gated]
          |
    +-----+-----+-----+-----+
    |     |     |     |     |
ci-expensive  quality-checks  rust-strict  docs-truth

    [Schedule]
          |
    +-----+-----+
    |           |
nightly.yml  property-tests.yml (extended)
    |
nightly-aspirational.yml
```

---

## Cost Analysis by Trigger Type

| Trigger | Workflows | Frequency | Cost Impact |
|---------|-----------|-----------|-------------|
| Every PR | ci.yml, test.yml | High | High impact - optimize these |
| Label-gated | ci-expensive, quality-checks, rust-strict | Low | Controlled cost |
| Main branch only | comprehensive_tests, nightly series | Medium | Acceptable |
| Release only | release, build-packages, brew-bump, publish-extension | Rare | Low impact |
| Dead branches | rust-ci, rust | Never (broken) | Waste until removed |

---

## Appendix: File Locations

All workflows are located in `.github/workflows/`:

```
.github/workflows/
  badges.yml
  benchmark.yml
  brew-bump.yml
  build-packages.yml
  check-ignored.yml
  ci-expensive.yml
  ci.yml
  comprehensive_tests.yml
  docs-truth.yml
  lsp-tests.yml
  nightly-aspirational.yml
  nightly.yml
  property-tests.yml
  publish-extension.yml
  quality-checks.yml
  release.yml
  rust-ci.yml
  rust-strict.yml
  rust.yml
  test.yml
  vscode-publish.yml
```
