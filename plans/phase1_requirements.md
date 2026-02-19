# Phase 1: CI/CD Optimization - Requirements Document

**Date**: 2026-02-12
**Version**: 0.8.8 → 1.0.0
**Status**: Requirements Defined
**Timeline**: 8 weeks

---

## Executive Summary

Phase 1 focuses on **CI/CD pipeline optimization** to reduce costs, improve reliability, and establish merge-blocking gates. Based on Phase 0 baseline analysis, the project has an opportunity to save **$720/year** through CI optimization while maintaining comprehensive validation.

**Key Objectives:**
- Reduce monthly CI cost from $68 to $10-15 (88% reduction)
- Consolidate 21 workflows to 5-8 workflows (~1,500 lines)
- Implement merge-blocking gates for all PRs
- Optimize runner usage and caching
- Establish performance baseline tracking

---

## 1. Current State Analysis

### 1.1 Baseline Metrics

| Metric | Current | Target | Gap |
|--------|---------|--------|-----|
| **Active Workflows** | 10 | 5-8 | -2 to -5 |
| **Workflow Lines** | 3,079 | ~1,500 | -1,579 |
| **Monthly CI Cost** | $68 | $10-15 | -$53 to -$58 |
| **Annual Savings Opportunity** | N/A | $720 | $720 |
| **Merge-Blocking Gates** | Partial | Complete | 🚧 Needed |
| **Local Gate Duration** | 2-5 min | <5 min | ✅ Target met |
| **Merge Gate Duration** | 3-5 min | <10 min | ✅ Target met |

### 1.2 Current Workflows

| Workflow | Status | Purpose | Cost Impact |
|----------|--------|---------|-------------|
| **CI** | Active | Main CI gate | High |
| **CI (Expensive)** | Label-gated | Extended tests | Medium |
| **Tests** | Active | Platform-specific tests | High |
| **Quality Checks** | Active | Linting, security, mutation | Medium |
| **LSP Tests** | Active | LSP integration tests | High |
| **Property Tests** | Active | Property-based testing | Medium |
| **Benchmarks** | Label-gated | Performance benchmarks | Low |
| **Fuzz** | Label-gated | Fuzz testing | Low |
| **Quality Checks (strict)** | Label-gated | Strict linting | Low |
| **Docs Deploy** | Active | Documentation deployment | Low |

### 1.3 Cost Breakdown

| Runner Type | Per Minute | Essential Jobs | Optional Jobs |
|-------------|------------|----------------|---------------|
| Linux (Ubuntu) | $0.008 | 5.5 min | 20 min |
| Windows | $0.016 | 3.0 min | 0 min |
| macOS | $0.080 | 0 min | 10 min |

**Per-PR Cost Estimates:**
- Essential jobs: $0.092 per PR
- Optional jobs: $0.344 per PR (label-gated)
- Annual without cancellation: $82.80
- Annual with cancellation: $38.64
- Potential savings: $44.16/year (53% reduction)

---

## 2. Requirements

### 2.1 Functional Requirements

#### FR-1: Workflow Consolidation

**Priority**: P0 (Critical)
**Description**: Consolidate redundant workflows to reduce complexity and cost.

**Acceptance Criteria:**
- [ ] Reduce active workflows from 10 to 5-8
- [ ] Reduce total workflow lines from 3,079 to ~1,500
- [ ] Maintain all existing test coverage
- [ ] Preserve all quality checks
- [ ] Ensure no functionality is lost

**Implementation Approach:**
1. Audit all workflows for redundancy
2. Merge related workflows (e.g., Tests + LSP Tests)
3. Use job matrices for platform-specific tests
4. Use conditional execution for optional checks
5. Remove archived workflows

**Success Metrics:**
- Workflow count: 5-8
- Workflow lines: ~1,500
- Test coverage: 100% maintained
- Quality checks: 100% maintained

#### FR-2: Merge-Blocking Gates

**Priority**: P0 (Critical)
**Description**: Implement merge-blocking gates for all PRs to ensure quality.

**Acceptance Criteria:**
- [ ] All PRs must pass merge gate before merging
- [ ] Merge gate includes all essential checks
- [ ] Merge gate fails fast on first failure
- [ ] Merge gate status is clearly visible in PR UI
- [ ] Merge gate can be bypassed with explicit approval

**Implementation Approach:**
1. Define merge gate requirements (format, clippy, tests)
2. Create dedicated merge-gate workflow
3. Configure branch protection rules
4. Add merge gate status check to PR template
5. Document merge bypass procedure

**Success Metrics:**
- Merge gate: 100% of PRs
- Merge gate pass rate: >95%
- Merge gate duration: <10 min
- Bypass rate: <5%

#### FR-3: Runner Usage Optimization

**Priority**: P1 (High)
**Description**: Optimize runner usage to reduce cost while maintaining quality.

**Acceptance Criteria:**
- [ ] Use Linux runners for all tests (where possible)
- [ ] Limit Windows runners to Windows-specific tests only
- [ ] Limit macOS runners to macOS-specific tests only
- [ ] Use self-hosted runners for long-running jobs (if available)
- [ ] Implement job caching to reduce build time

**Implementation Approach:**
1. Audit all jobs for platform requirements
2. Move platform-agnostic tests to Linux
3. Consolidate Windows tests into single job
4. Consolidate macOS tests into single job
5. Implement aggressive caching (cargo, target, registry)

**Success Metrics:**
- Linux runner usage: >80%
- Windows runner usage: <10%
- macOS runner usage: <10%
- Cache hit rate: >70%

#### FR-4: Caching Strategy

**Priority**: P1 (High)
**Description**: Implement comprehensive caching to reduce build time and cost.

**Acceptance Criteria:**
- [ ] Cache cargo registry
- [ ] Cache cargo index
- [ ] Cache target directory
- [ ] Cache dependencies
- [ ] Cache workflow artifacts
- [ ] Cache expires after 7 days
- [ ] Cache key includes Rust toolchain

**Implementation Approach:**
1. Use `actions/cache` for cargo caching
2. Use `actions/cache` for target directory
3. Use `Swatinem/rust-cache@v2` for comprehensive caching
4. Configure cache keys with toolchain version
5. Implement cache warming for common dependencies

**Success Metrics:**
- Cache hit rate: >70%
- Build time reduction: >30%
- Cache size: <5 GB

#### FR-5: Performance Baseline Tracking

**Priority**: P2 (Medium)
**Description**: Establish performance baseline tracking to detect regressions.

**Acceptance Criteria:**
- [ ] Run benchmarks on every merge to main
- [ ] Store benchmark results in JSON format
- [ ] Compare current results to baseline
- [ ] Alert on performance regression >10%
- [ ] Store historical results for trend analysis

**Implementation Approach:**
1. Create benchmark workflow
2. Run `cargo bench` on merge
3. Store results in `benchmarks/results/`
4. Compare to baseline in `benchmarks/baselines/v0.9.0.json`
5. Create alert workflow for regressions

**Success Metrics:**
- Benchmarks run: 100% of merges
- Regression detection: <10% threshold
- Alert accuracy: >90%
- Historical data: Complete

#### FR-6: Concurrency Cancellation

**Priority**: P1 (High)
**Description**: Ensure concurrency cancellation is enabled for all workflows.

**Acceptance Criteria:**
- [ ] All workflows have concurrency cancellation enabled
- [ ] Concurrency group includes branch name
- [ ] In-progress runs are cancelled on new push
- [ ] Cancelled runs are clearly marked in UI

**Implementation Approach:**
1. Add `concurrency` section to all workflows
2. Use `ci-${{ github.ref }}` as group name
3. Set `cancel-in-progress: true`
4. Test cancellation behavior

**Success Metrics:**
- Concurrency cancellation: 100% of workflows
- Cancelled runs: >50% of total runs
- Cost savings: >30%

### 2.2 Non-Functional Requirements

#### NFR-1: Cost Reduction

**Priority**: P0 (Critical)
**Description**: Reduce monthly CI cost from $68 to $10-15.

**Acceptance Criteria:**
- [ ] Monthly CI cost: $10-15
- [ ] Annual savings: $720
- [ ] Cost per PR: < $0.05
- [ ] Cost tracking: Automated

**Success Metrics:**
- Monthly cost: $10-15
- Annual savings: $720
- Cost per PR: < $0.05

#### NFR-2: Gate Duration

**Priority**: P1 (High)
**Description**: Ensure gates complete in reasonable time.

**Acceptance Criteria:**
- [ ] Local gate duration: <5 min
- [ ] Merge gate duration: <10 min
- [ ] PR-fast gate duration: <2 min
- [ ] Nightly gate duration: <30 min

**Success Metrics:**
- Local gate: <5 min
- Merge gate: <10 min
- PR-fast: <2 min
- Nightly: <30 min

#### NFR-3: Test Coverage

**Priority**: P1 (High)
**Description**: Maintain 100% test coverage during optimization.

**Acceptance Criteria:**
- [ ] All tests run in merge gate
- [ ] Test coverage: 95.9% maintained
- [ ] No tests removed during consolidation
- [ ] Test execution time: <10 min

**Success Metrics:**
- Test coverage: 95.9%
- Tests run: 100%
- Test execution: <10 min

#### NFR-4: Reliability

**Priority**: P1 (High)
**Description**: Ensure CI is reliable and provides consistent results.

**Acceptance Criteria:**
- [ ] CI pass rate: >95%
- [ ] Flaky test rate: <5%
- [ ] False positive rate: <2%
- [ ] False negative rate: <1%

**Success Metrics:**
- CI pass rate: >95%
- Flaky test rate: <5%
- False positive: <2%
- False negative: <1%

#### NFR-5: Maintainability

**Priority**: P2 (Medium)
**Description**: Ensure CI workflows are easy to understand and modify.

**Acceptance Criteria:**
- [ ] Workflow structure: Clear and logical
- [ ] Job dependencies: Minimal
- [ ] Reusable workflows: Used where appropriate
- [ ] Documentation: Complete and up-to-date

**Success Metrics:**
- Workflow lines: <200 per workflow
- Job dependencies: <3 per workflow
- Reusable workflows: >50%
- Documentation: 100%

---

## 3. Implementation Plan

### 3.1 Week-by-Week Schedule

#### Week 1: Workflow Audit and Planning

**Goals:**
- Audit all current workflows
- Identify redundancies and optimization opportunities
- Create consolidation plan
- Define merge gate requirements

**Tasks:**
- [ ] Document all current workflows
- [ ] Identify redundant jobs
- [ ] Identify platform-specific jobs
- [ ] Create consolidation plan
- [ ] Define merge gate requirements
- [ ] Get approval for consolidation plan

**Deliverables:**
- Workflow audit report
- Consolidation plan
- Merge gate requirements document

#### Week 2: Workflow Consolidation (Part 1)

**Goals:**
- Consolidate core workflows (CI, Tests)
- Implement merge-blocking gate
- Add concurrency cancellation

**Tasks:**
- [ ] Consolidate CI and Tests workflows
- [ ] Create merge-gate workflow
- [ ] Add concurrency cancellation to all workflows
- [ ] Test consolidated workflows
- [ ] Update documentation

**Deliverables:**
- Consolidated CI workflow
- Merge-gate workflow
- Updated documentation

#### Week 3: Workflow Consolidation (Part 2)

**Goals:**
- Consolidate remaining workflows (Quality, LSP, Property)
- Optimize runner usage
- Implement caching strategy

**Tasks:**
- [ ] Consolidate Quality Checks workflow
- [ ] Consolidate LSP Tests workflow
- [ ] Consolidate Property Tests workflow
- [ ] Optimize runner usage (Linux-only where possible)
- [ ] Implement comprehensive caching
- [ ] Test optimized workflows

**Deliverables:**
- Consolidated workflows
- Optimized runner usage
- Comprehensive caching

#### Week 4: Performance Baseline Tracking

**Goals:**
- Implement performance baseline tracking
- Create benchmark workflow
- Implement regression detection

**Tasks:**
- [ ] Create benchmark workflow
- [ ] Store benchmark results
- [ ] Compare to baseline
- [ ] Implement regression alerting
- [ ] Test performance tracking

**Deliverables:**
- Benchmark workflow
- Performance baseline tracking
- Regression alerting

#### Week 5: Testing and Validation

**Goals:**
- Test all consolidated workflows
- Validate test coverage
- Measure cost reduction
- Validate gate duration

**Tasks:**
- [ ] Run all consolidated workflows
- [ ] Validate test coverage (95.9%)
- [ ] Measure cost reduction
- [ ] Measure gate duration
- [ ] Fix any issues

**Deliverables:**
- Validated workflows
- Cost reduction report
- Gate duration report

#### Week 6: Documentation and Training

**Goals:**
- Update all CI/CD documentation
- Create contributor guide
- Train contributors on new workflows

**Tasks:**
- [ ] Update CI_README.md
- [ ] Update CI_COST_TRACKING.md
- [ ] Create contributor guide for CI
- [ ] Train contributors
- [ ] Update PR template

**Deliverables:**
- Updated documentation
- Contributor guide
- Trained contributors

#### Week 7: Rollout and Monitoring

**Goals:**
- Roll out new workflows
- Monitor for issues
- Fix any problems
- Gather feedback

**Tasks:**
- [ ] Roll out new workflows
- [ ] Monitor for issues
- [ ] Fix any problems
- [ ] Gather feedback from contributors
- [ ] Make adjustments as needed

**Deliverables:**
- Rolled out workflows
- Monitoring dashboard
- Feedback report

#### Week 8: Final Review and Handoff

**Goals:**
- Final review of all changes
- Document lessons learned
- Hand off to maintainers
- Celebrate success

**Tasks:**
- [ ] Final review of all changes
- [ ] Document lessons learned
- [ ] Create handoff documentation
- [ ] Hand off to maintainers
- [ ] Celebrate success

**Deliverables:**
- Final review report
- Lessons learned document
- Handoff documentation

### 3.2 Risk Assessment

| Risk | Probability | Impact | Mitigation |
|------|-------------|---------|------------|
| **Workflow consolidation breaks tests** | Medium | High | Thorough testing before rollout |
| **Cost reduction not achieved** | Low | High | Continuous cost monitoring |
| **Gate duration exceeds target** | Medium | Medium | Parallelize jobs, optimize caching |
| **Merge gate blocks valid PRs** | Low | High | Clear bypass procedure |
| **Contributors resist changes** | Low | Medium | Early communication, training |

---

## 4. Success Criteria

### 4.1 Phase 1 Success Criteria

| Criterion | Target | Status |
|-----------|--------|--------|
| **Workflows consolidated** | 5-8 workflows | 🚧 Pending |
| **Workflow lines reduced** | ~1,500 lines | 🚧 Pending |
| **Monthly CI cost** | $10-15 | 🚧 Pending |
| **Annual savings** | $720 | 🚧 Pending |
| **Merge-blocking gates** | 100% of PRs | 🚧 Pending |
| **Test coverage maintained** | 95.9% | 🚧 Pending |
| **Gate duration** | <10 min | 🚧 Pending |
| **Concurrency cancellation** | 100% of workflows | 🚧 Pending |

### 4.2 Validation Commands

```bash
# Check workflow count
ls -la .github/workflows/*.yml | wc -l

# Check workflow lines
find .github/workflows -name "*.yml" -exec wc -l {} + | tail -1

# Run merge gate
just merge-gate

# Run local gate
nix develop -c just ci-gate

# Check test coverage
cargo test --workspace --lib

# Check cost
# View GitHub Actions billing page
```

---

## 5. Dependencies

### 5.1 Internal Dependencies

- Phase 0 baseline metrics report ✅ Complete
- Phase 0 documentation gap analysis ✅ Complete

### 5.2 External Dependencies

- GitHub Actions (platform)
- Rust toolchain 1.92.0
- just command runner
- nix (for local development)

---

## 6. Resources

### 6.1 Documentation References

- **CI Cost Tracking**: [`docs/CI_COST_TRACKING.md`](../docs/CI_COST_TRACKING.md)
- **CI Readme**: [`docs/CI_README.md`](../docs/CI_README.md)
- **CI Hardening**: [`docs/CI_HARDENING.md`](../docs/CI_HARDENING.md)
- **CI Audit**: [`docs/CI_AUDIT.md`](../docs/CI_AUDIT.md)
- **Local CI**: [`docs/LOCAL_CI.md`](../docs/LOCAL_CI.md)
- **Production Readiness Roadmap**: [`plans/production_readiness_roadmap.md`](production_readiness_roadmap.md)
- **Phase 0 Baseline Metrics**: [`plans/phase0_baseline_metrics_report.md`](phase0_baseline_metrics_report.md)

### 6.2 Tools and Commands

```bash
# Just commands
just ci-gate              # Run canonical local gate
just merge-gate           # Run merge gate
just pr-fast              # Run PR-fast gate
just nightly              # Run nightly gate
just debt-report          # Show debt status
just status-check        # Verify metrics

# Cargo commands
cargo fmt --check         # Format check
cargo clippy --workspace # Linting
cargo test --workspace    # Tests
cargo bench              # Benchmarks

# CI commands
gh workflow list          # List workflows
gh workflow view         # View workflow
gh run list             # List runs
gh run view             # View run details
```

---

## 7. Appendix

### 7.1 Workflow Consolidation Plan

#### Current Workflows

| Workflow | Lines | Jobs | Platform | Cost |
|----------|--------|-------|-----------|-------|
| CI | 84 | 1 | Linux | Low |
| CI (Expensive) | 150 | 5 | Linux | Medium |
| Tests | 200 | 4 | Linux/Windows | High |
| Quality Checks | 250 | 8 | Linux | Medium |
| LSP Tests | 180 | 4 | Linux/Windows/macOS | High |
| Property Tests | 120 | 3 | Linux | Medium |
| Benchmarks | 100 | 2 | Linux | Low |
| Fuzz | 80 | 2 | Linux | Low |
| Quality Checks (strict) | 60 | 3 | Linux | Low |
| Docs Deploy | 50 | 2 | Linux | Low |
| **Total** | **1,274** | **34** | **Mixed** | **High** |

#### Proposed Consolidated Workflows

| Workflow | Lines | Jobs | Platform | Cost |
|----------|--------|-------|-----------|-------|
| CI (consolidated) | 200 | 5 | Linux | Low |
| Merge Gate | 150 | 3 | Linux | Low |
| Quality Checks | 200 | 5 | Linux | Medium |
| Extended Tests (label-gated) | 300 | 8 | Linux/Windows/macOS | Medium |
| Benchmarks (label-gated) | 150 | 3 | Linux | Low |
| **Total** | **1,000** | **24** | **Mixed** | **Medium** |

**Reductions:**
- Workflows: 10 → 6 (-40%)
- Lines: 1,274 → 1,000 (-22%)
- Jobs: 34 → 24 (-29%)
- Cost: High → Medium (-50%+)

### 7.2 Merge Gate Requirements

#### Required Checks

| Check | Command | Duration | Fail Fast |
|-------|----------|-----------|------------|
| Format check | `cargo fmt --check` | 30s | Yes |
| Clippy (core) | `cargo clippy --workspace -p perl-parser -p perl-lexer` | 60s | Yes |
| Tests (core) | `cargo test --workspace --lib` | 120s | Yes |
| LSP semantic tests | `cargo test -p perl-lsp --test semantic_definition` | 60s | Yes |
| Security audit | `cargo audit` | 30s | Yes |

**Total Duration**: ~5 min

#### Optional Checks (label-gated)

| Check | Label | Command | Duration |
|-------|--------|----------|-----------|
| Clippy (full) | `ci:clippy-full` | `cargo clippy --workspace` | 120s |
| Tests (full) | `ci:tests-full` | `cargo test --workspace` | 300s |
| Mutation testing | `ci:mutation` | `cargo test -p perl-parser --test mutation_hardening_tests` | 600s |
| Benchmarks | `ci:bench` | `cargo bench` | 300s |

---

**Document Generated**: 2026-02-12
**Next Review**: After Week 4 (mid-phase review)
**Status**: ✅ Requirements Defined - Ready for Implementation
