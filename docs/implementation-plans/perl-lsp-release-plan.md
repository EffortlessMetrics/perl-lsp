# perl-lsp CI/CD and Release Process Assessment

**Context:** v0.12.2 shipped, 113 issues filed. This document assesses the current CI/CD infrastructure and identifies gaps for managing high-volume issue resolution through the release pipeline.

**Generated:** 2026-04-09

---

## 1. Current CI/CD Assessment

### 1.1 Workspace Structure

**130+ workspace crates** organized in 7 dependency tiers:
- **Tier 1**: Leaf crates (no workspace deps) — 11 crates
- **Tier 1b-1c**: Single-dependency crates — 3 crates  
- **Tier 2**: Core infrastructure — 12 crates
- **Tier 3**: Analysis and indexing — 11 crates
- **Tier 4**: LSP providers — 20 crates
- **Tier 5**: Application crates — 10 crates
- **Tier 6**: Module resolution — 15 crates
- **Tier 7**: Top-level application — 2 crates (`perl-lsp-rs`, `perllsp`)

**Current version:** 0.12.3 (workspace-level version sync across all crates)

### 1.2 CI Workflow Architecture

The CI is organized into **tiered gates** with clear cost/quality tradeoffs:

| Tier | Trigger | Duration | Cost | Purpose |
|------|---------|----------|------|---------|
| **PR Smoke** | Every PR push | ~1-2 min | ~$0.03 | Fast feedback: fmt, clippy-core, test-core |
| **Merge Gate** | `merge-ready` label or push to main | ~3-5 min | ~$0.05 | Full validation: clippy-full, test-full, LSP smoke, security, policy |
| **UX Regression** | LSP/DAP path changes | ~5 min | ~$0.02-0.05 | First-5-minutes user experience validation |
| **Nightly** | Schedule (3am UTC) | ~15-30 min | ~$0.30 | Comprehensive: benchmarks, mutation, coverage, fuzz |

### 1.3 Key Workflow Files

| Workflow | Purpose | Gaps Identified |
|----------|---------|-----------------|
| `ci.yml` | Main CI with tiered gates | ✅ Well-structured, uses receipt system |
| `ci-nightly.yml` | Expensive checks (mutation, fuzz, coverage, benchmarks) | ✅ Label-gated, good resource control |
| `release-orchestration.yml` | Master release dispatcher | ✅ Comprehensive skip flags for recovery |
| `publish-crates.yml` | crates.io publishing with topological sort | ✅ Tarjan SCC for dev-deps, sparse-index verification |
| `version-bump.yml` | Automated version bump + changelog | ✅ Uses cargo-release + git-cliff |
| `ux-regression-gate.yml` | First-5-minutes UX validation | ⚠️ Harness detection is graceful-fail (not blocking) |
| `flake-detection.yml` | Flaky test detection + auto-quarantine | ✅ Good classification, creates PRs |
| `triage-issues.yml` | Automated issue labeling | ✅ Runs every 6 hours |

### 1.4 Current Release Process (Well-Documented)

The release process is **fully automated** with three entry points:

1. **Turnkey (Recommended):** `cargo xtask release-turnkey <0.x.y>`
2. **Manual PR Flow:** Version bump PR → Merge → Release orchestration
3. **Recovery:** Individual workflow dispatch with skip flags

**Distribution channels:**
- GitHub Releases (multi-platform binaries)
- crates.io (130+ crates in topological order)
- VSCode Marketplace + Open VSX
- Docker Hub + GHCR
- Homebrew, Scoop, Chocolatey, Winget

---

## 2. Gaps for Managing 113 Issue Fixes

### 2.1 Issue Batching Strategy (Gap Identified)

**Current State:** No explicit batching strategy documented for high-volume issue periods.

**Recommended Approach:**

| Release | Target | Content | Timing |
|---------|--------|---------|--------|
| **v0.12.4** | Patch release | Critical bug fixes, regressions from v0.12.2 | Immediate (next 1-2 weeks) |
| **v0.13.0-alpha** | Public alpha | Architecture implementations, new features | After v0.12.4 stabilizes |
| **v0.13.0** | Minor release | All v0.13.0-alpha validated features | 4-6 weeks after alpha |
| **v0.14.0** | Next minor | Advanced features (post-architecture completion) | TBD |

**Issue Triage Labels for Batching:**
- `priority/P0` — Blocker, fix immediately → v0.12.4
- `priority/P1` — Important, batch for v0.13.0
- `priority/P2` — Nice to have, v0.14.0 or later
- `type/regression` — Must go in next patch release
- `type/architecture` — Batch for v0.13.0-alpha

### 2.2 Testing Strategy for Large PR Volume

**Current Strengths:**
- ✅ Tiered CI keeps cost low for most PRs (~$0.03 per PR)
- ✅ Label-gated expensive jobs (mutation, coverage, benchmarks)
- ✅ Flake detection with auto-quarantine
- ✅ UX regression gate for user-facing changes

**Identified Gaps:**

| Gap | Impact | Recommendation |
|-----|--------|----------------|
| No PR batching for dependency updates | High volume of dependabot PRs can overwhelm CI | Group dependency updates weekly |
| Limited parallelization in merge gate | 30min timeout may be tight with 130 crates | Consider job matrix split by tier |
| No automated performance regression on PR | Performance issues may slip through | Enable `ci:bench` label by default on parser/LSP PRs |

**Recommended Testing Safeguards:**

```yaml
# Add to ci.yml - Batch dependency update detection
jobs:
  batch-check:
    runs-on: ubuntu-latest
    outputs:
      is-batchable: ${{ steps.check.outputs.batchable }}
    steps:
      - id: check
        run: |
          # If >5 PRs from dependabot[bot] open, suggest batching
          count=$(gh pr list --author "dependabot[bot]" --json number | jq length)
          if [ "$count" -gt 5 ]; then
            echo "::warning::$count dependabot PRs open — consider batching"
            echo "batchable=true" >> $GITHUB_OUTPUT
          fi
```

### 2.3 Backward Compatibility Concerns

**Current State:**
- ✅ `cargo-semver-checks` runs in nightly CI (label-gated: `ci:semver`)
- ✅ Workspace uses unified version (all crates move together)
- ⚠️ No automated semver check on PR by default

**Recommendations:**

1. **Enable semver check on PRs touching public APIs:**
   - Parser crates: `perl-parser`, `perl-lexer`, `perl-parser-core`
   - LSP protocol: `perl-lsp-protocol`
   - Module resolution: `perl-module-resolution`

2. **Breaking Change Documentation:**
   - Current: Conventional commits with `!` marker
   - Gap: No automated breaking change report in PR
   - Fix: Add breaking change summary comment to PRs with `!` commits

### 2.4 Performance Regression Detection

**Current State:**
- ✅ Benchmarks run nightly (label-gated: `ci:bench`)
- ✅ Baseline comparison with `benchmarks/baselines/v*.json`
- ✅ Alert generation for critical regressions
- ⚠️ No PR-level performance gate by default

**Identified Gaps:**

| Gap | Current | Recommended |
|-----|---------|-------------|
| Parser/LSP PRs | No perf gate | Run `ci:bench` on parser/LSP path changes |
| Benchmark storage | 30-day retention | Extend to 90 days for trend analysis |
| Regression threshold | Fixed threshold | Adaptive threshold based on variance |

**Implementation:**

```yaml
# In ci-nightly.yml - Add parser/LSP path trigger
benchmark:
  if: |
    github.event_name == 'schedule' ||
    contains(github.event.pull_request.labels.*.name, 'ci:bench') ||
    (
      github.event_name == 'pull_request' &&
      (
        contains(github.event.pull_request.changed_files, 'crates/perl-parser/') ||
        contains(github.event.pull_request.changed_files, 'crates/perl-lsp/') ||
        contains(github.event.pull_request.changed_files, 'crates/perl-lexer/')
      )
    )
```

---

## 3. Review of Existing GitHub Actions

### 3.1 Test Coverage Reporting

| Aspect | Status | Notes |
|--------|--------|-------|
| Tool | `cargo-llvm-cov` | Nightly only |
| Upload | Codecov | `fail_ci_if_error: false` (non-blocking) |
| Coverage | Branch-aware | Uses `-Z branchcoverage` |
| Trigger | Label-gated (`ci:coverage`) | Good cost control |

**Gap:** No PR-level coverage diff (showing coverage change from baseline).

**Recommendation:** Add coverage diff comment on PRs:
```yaml
- name: Coverage diff
  run: |
    # Compare to main branch coverage, post PR comment with +/-
```

### 3.2 Benchmark Tracking

| Aspect | Status | Notes |
|--------|--------|-------|
| Framework | Criterion | Standard Rust benchmarking |
| Extraction | Python scripts | `extract-criterion.py`, `compare.sh` |
| Baselines | Stored in repo | `benchmarks/baselines/` |
| Alerts | Generated | Posted as PR comments |

**Gap:** No automated benchmark history visualization.

**Recommendation:** Consider uploading to a benchmark tracking service (e.g., Bencher, or self-hosted).

### 3.3 Documentation Generation

| Aspect | Status | Notes |
|--------|--------|-------|
| Docs.rs | Configured | Per-crate metadata in `Cargo.toml` |
| Changelog | git-cliff | Automated with conventional commits |
| API docs | `cargo doc` | Standard Rust documentation |
| Deploy | Not automated | Manual docs deployment only |

**Gap:** No automated docs deployment on release.

**Recommendation:** Add `docs-deploy.yml` workflow trigger on release:
```yaml
on:
  release:
    types: [published]
```

### 3.4 Crate Publishing

| Aspect | Status | Notes |
|--------|--------|-------|
| Order | Topological sort | Tarjan SCC algorithm for dev-deps |
| Rate limiting | Handled | 13s sleep between publishes |
| Verification | Sparse index | Direct crates.io index verification |
| Recovery | Incremental | Failed crates retried, passed crates skipped |

**Current Limitations:**
- 130 crates × 13s = ~28 minutes minimum (within 360-min timeout ✓)
- Dev-dependency cycles handled by stripping dev-deps during publish
- No parallel publishing (sequential required for dependency order)

---

## 4. Recommended Release Strategy for Architecture Implementations

### 4.1 Release Train Model

For the 113 issues including architecture implementations, adopt a **release train** model:

```
v0.12.4 (patch) ──► v0.13.0-alpha ──► v0.13.0 (minor) ──► v0.14.0-dev
     │                    │                  │                  │
   Weekly              Bi-weekly          Monthly          Continuous
   Hotfixes            Feature preview    Stable release    Integration
```

### 4.2 v0.12.4 — Critical Fixes Only

**Scope:**
- P0 regressions from v0.12.2/v0.12.3
- Security fixes
- Crash fixes

**Process:**
1. Label issues `priority/P0` + `type/regression`
2. Fast-track through PR review
3. One-week stabilization period
4. Release via normal orchestration

### 4.3 v0.13.0-alpha — Architecture Preview

**Scope:**
- New parser architecture features
- LSP provider improvements
- Breaking changes allowed

**Process:**
1. Create `alpha` branch from main
2. Merge architecture PRs to alpha
3. Weekly alpha releases (manual dispatch)
4. Community testing period
5. Merge to main when stable

**CI Adaptation:**
```yaml
# Add alpha-specific workflow
on:
  push:
    branches: [alpha]
  workflow_dispatch:
```

### 4.4 v0.13.0 — Stable Minor Release

**Scope:**
- All validated v0.13.0-alpha features
- No breaking changes from alpha
- Complete documentation

**Entry Criteria:**
- [ ] All P0 issues resolved
- [ ] Alpha testing period complete (2+ weeks)
- [ ] Benchmarks show no regression
- [ ] Semver checks pass
- [ ] UX regression tests pass

---

## 5. Testing/Performance Safeguards

### 5.1 Recommended Additional Checks

| Check | Current | Recommended | Priority |
|-------|---------|-------------|----------|
| Semver on PR | Nightly only | Parser/LSP PRs | High |
| Benchmark on PR | Label-gated | Parser/LSP changes | High |
| Coverage diff | None | PR comment with diff | Medium |
| Binary size | None | Track in release | Medium |
| MSRV check | Nightly only | Every PR | Low |

### 5.2 CI Cost Controls for High Volume

Current cost: ~$0.03/PR (smoke) + ~$0.05/merge (gate) = ~$0.08 per PR

For 100 PRs/week: ~$8/week = ~$416/year (acceptable)

**Optimizations:**
1. **Cancel in-flight runs:** Already implemented (`cancel-in-progress: true`)
2. **Cache aggressively:** Rust cache + sccache already configured
3. **Label-gate expensive jobs:** Already implemented
4. **Skip CI for docs-only:** Consider `paths-ignore` for markdown changes

```yaml
on:
  pull_request:
    paths-ignore:
      - 'docs/**'
      - '**.md'
```

### 5.3 Performance Regression Thresholds

Recommended thresholds for benchmark alerts:

| Metric | Warning | Critical |
|--------|---------|----------|
| Parse time | +10% | +25% |
| Memory usage | +15% | +30% |
| Binary size | +5% | +10% |
| Compile time | +20% | +50% |

---

## 6. Documentation Update Requirements

### 6.1 Release-Specific Documentation

| Document | Current Status | Required Updates |
|----------|---------------|------------------|
| `CHANGELOG.md` | ✅ Automated | Verify conventional commits are clean |
| `docs/RELEASE_PROCESS.md` | ✅ Comprehensive | No changes needed |
| `docs/PUBLISHING.md` | ✅ Clear | No changes needed |
| `docs/project/ROADMAP.md` | ⚠️ Exists | Update for v0.13.0 scope |
| `docs/project/RELEASE_CHECKLIST.md` | ⚠️ Exists | Verify checkboxes work for batch releases |

### 6.2 New Documentation Needed

1. **Release Batching Guide** (`docs/project/RELEASE_BATCHING.md`)
   - How to batch issues for releases
   - Label conventions for batching
   - Merge queue strategy

2. **Performance Regression Playbook**
   - How to investigate benchmark regressions
   - When to block a release
   - Escalation path

3. **Issue Triage Guide** (update existing)
   - P0/P1/P2 definitions
   - Architecture vs bug fix distinction
   - Assignment guidelines

### 6.3 Changelog Improvements

Current `cliff.toml` is well-configured with:
- Conventional commit parsing
- Breaking change detection
- Grouped sections (Features, Bug Fixes, etc.)

**Recommended additions:**

```toml
# Add to cliff.toml
commit_parsers = [
  # ... existing parsers ...
  
  # Issue reference extraction
  { pattern = '#(\d+)', replace = "[#$1](https://github.com/EffortlessMetrics/perl-lsp/issues/$1)" },
]
```

---

## 7. Summary and Recommendations

### 7.1 Current State Assessment

| Area | Rating | Notes |
|------|--------|-------|
| CI Architecture | ⭐⭐⭐⭐⭐ | Tiered gates, excellent cost control |
| Release Automation | ⭐⭐⭐⭐⭐ | Fully automated, well-documented |
| Testing Strategy | ⭐⭐⭐⭐ | Good coverage, minor gaps in PR-level checks |
| Issue Management | ⭐⭐⭐⭐ | Automated triage, needs batching strategy |
| Documentation | ⭐⭐⭐⭐⭐ | Comprehensive, well-organized |
| Performance Monitoring | ⭐⭐⭐ | Nightly only, needs PR-level integration |

### 7.2 Priority Actions

**High Priority (This Week):**
1. Define and document issue batching strategy for v0.12.4/v0.13.0
2. Enable semver checks on parser/LSP PRs
3. Create `RELEASE_BATCHING.md` guide

**Medium Priority (Next 2 Weeks):**
1. Add benchmark runs on parser/LSP path changes
2. Extend benchmark retention to 90 days
3. Create performance regression playbook

**Low Priority (Ongoing):**
1. Add coverage diff PR comments
2. Automate docs deployment on release
3. Consider benchmark tracking service

### 7.3 Release Plan Summary

| Version | Target Date | Content | CI Adaptations |
|---------|-------------|---------|----------------|
| v0.12.4 | +1-2 weeks | Critical fixes only | None needed |
| v0.13.0-alpha | +2-4 weeks | Architecture features | Add alpha branch workflow |
| v0.13.0 | +4-6 weeks | Stable minor release | Enhanced gates |
| v0.14.0 | TBD | Post-architecture | TBD |

---

## Appendix: File References

### Key CI/CD Files
- `/Cargo.toml` — Workspace configuration, publish allowlist
- `/.github/workflows/ci.yml` — Main CI workflow
- `/.github/workflows/release-orchestration.yml` — Release dispatcher
- `/.github/workflows/publish-crates.yml` — crates.io publishing
- `/.github/workflows/ci-nightly.yml` — Expensive checks
- `/.github/ci-config.yml` — Shared CI configuration

### Documentation Files
- `/docs/RELEASE_PROCESS.md` — Complete release workflow
- `/docs/PUBLISHING.md` — crates.io publishing guide
- `/docs/CHANGELOG_WORKFLOW.md` — Changelog automation
- `/cliff.toml` — git-cliff configuration

### Release Management
- `/docs/project/RELEASE_CHECKLIST.md` — Release checklist
- `/docs/project/ROADMAP.md` — Current roadmap
- `/docs/project/RELEASE_RUNBOOK_0_12_3.md` — v0.12.3 runbook

---

*This document was generated as part of the perl-lsp release process assessment. For questions or updates, refer to the release documentation in `/docs/RELEASE_PROCESS.md`.*
