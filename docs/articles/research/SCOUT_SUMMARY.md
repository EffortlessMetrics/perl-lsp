# Scout Reports Summary — March 19, 2026

## Reports Generated

This directory contains three comprehensive scout reports conducted for project evaluation:

### 1. Test Coverage Gaps (`TEST_COVERAGE_GAPS_SCOUT.md`)

**Scope**: Critical test coverage gaps in 5 key crates before 0.12.0 public alpha  
**Status**: 10 impactful gaps identified, 4 marked CRITICAL

**Key Findings**:
- perl-lsp-completion: Missing malformed input recovery tests (25 tests needed)
- perl-lsp-navigation: Missing error recovery in broken files (20 tests needed)
- perl-dap: Missing UTF-8 breakpoint edge cases (15 tests needed)
- perl-lsp-completion: Missing cancellation boundary tests (18 tests needed)

**Risk Assessment**: 4 RED FLAGS that block public alpha launch
- Completion fails while typing in editor (broken code)
- Navigation can't work during refactoring
- DAP breakpoints wrong for non-ASCII code
- Large project completions timeout

**Deliverable**: Detailed test pattern examples + action items

---

### 2. Testing Infrastructure Gaps (`TESTING_INFRASTRUCTURE_GAPS_SCOUT.md`)

**Scope**: Property-based testing, snapshot testing, integration testing, benchmarks, coverage, mutation testing  
**Status**: 8 actionable gaps identified, 6 GitHub issues created

**GitHub Issues Created**:
1. **#2091** — Benchmark regression detection + baseline tracking (HIGH | 4h)
2. **#2093** — Flaky test detection + auto-quarantine (HIGH | 6h)
3. **#2096** — Test taxonomy (smoke/deep/slow tiers) (HIGH | 5h)
4. **#2099** — Branch coverage reporting & gating (MEDIUM | 4h)
5. **#2101** — Property-based testing generators (MEDIUM | 10h)
6. **#2104** — Snapshot testing for AST & errors (MEDIUM | 8h)

**Key Infrastructure Gaps**:
- No benchmark regression baseline tracking (Criterion available but unused)
- Flaky test infrastructure exists but is empty (0 tests tracked)
- 900+ integration tests all run on every PR (20+ min feedback time)
- Line coverage tracked, branch coverage not gated
- Property-based testing available (108 uses) but generators missing
- Snapshot testing barely used (5 LSP snapshots only)

**Quick Wins** (non-issue):
- Mutation time budget: Add `--timeout 10m` flag (15 min)
- Corpus validation: Add validation script (opportunistic)

**Effort Summary**: 37 total hours to address all 8 gaps

---

## Context & Methodology

Both scouts were conducted via:
1. **File enumeration**: Counted test files, source modules, test functions
2. **Grep pattern analysis**: Searched for specific tool usage (proptest, insta, criterion, cargo-llvm-cov)
3. **Configuration review**: Examined CLAUDE.md, debt-ledger.yaml, gate-policy.yaml, justfile
4. **Gap identification**: Compared "available tools" vs "actual usage"
5. **Impact analysis**: Ranked gaps by user-facing impact + effort to fix

---

## Handoff to Builders

Both scout reports are structured for easy handoff:
- Prioritized lists (use in PR/issue ordering)
- Specific acceptance criteria (builders know when done)
- Effort + complexity estimates (planning)
- Code examples/patterns (builders know what to build)
- Test counts (for effort validation)

Recommended action: Use issues #2091, #2093, #2096 for next build cycle (merge queue bottleneck).

---

## Recommendations for Next Cycle

1. **Immediate**: Create GitHub issue for test coverage gaps (use TEST_COVERAGE_GAPS_SCOUT.md)
   - Before public alpha launch
   - Focus on #1-4 (CRITICAL items)

2. **Current Issues**: Start on testing infrastructure issues (#2091-#2104)
   - Fix merge queue bottleneck first (#2096 test taxonomy)
   - Then flaky detection (#2093) and regression tracking (#2091)

3. **Deferred**: Property-based testing (#2101) and snapshots (#2104)
   - These are quality improvements, not blockers
   - Can be post-alpha

---

**Scout Date**: 2026-03-19  
**Reports**: 2 (Test Coverage, Testing Infrastructure)  
**Issues Created**: 6 (all labeled swarm-discovered + swarm-improve-tests)  
**Gaps Identified**: 18 (10 test coverage + 8 infrastructure)
