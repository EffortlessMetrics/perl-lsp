# Testing Infrastructure Scout Report: Gaps & Opportunities

**Date**: 2026-03-19  
**Scope**: Property-based testing, snapshot testing, integration testing, benchmarks, fuzz coverage, code coverage  
**Status**: 8 actionable gaps identified

## Summary

Testing infrastructure is partially mature but missing key regression tools, weak in property-based testing strategies, and lacks systematic benchmark regression tracking.

---

## Gap 1: No Benchmark Regression Baseline Tracking

**Current State**:
- ✓ Criterion framework installed in 5 key crates (perl-dap, perl-lexer, perl-lsp, perl-lsp-tooling, perl-parser)
- ✓ `just benchmarks` command runs benchmarks and saves to `benchmarks/results/raw-output.txt`
- ✗ **NO**: Baseline storage, comparison, or delta reporting
- ✗ **NO**: Regression detection in CI (benchmarks don't block merges)
- ✗ **NO**: Critcmp integration for head-to-head comparison

**Impact**: Performance regressions silently accumulate; users first discover slowdowns in production

**Fix**: 
1. Generate baseline on release commits
2. Compare current run to baseline
3. Fail CI if >10% regression
4. Store baselines in `.ci/benchmark-baselines/`

**Effort**: 3-4 hours  
**Complexity**: Medium (Criterion API + CI integration)

---

## Gap 2: Property-Based Testing Strategies Underutilized

**Current State**:
- ✓ Proptest 1.9.0 available in 20+ crates
- ✓ 108 proptest strategy usages found in tests
- ✗ **NO**: Documented strategies or patterns
- ✗ **NO**: Generators for domain objects (Variable, Module, AST node, etc.)
- ✗ **NO**: Property test coverage in high-value areas

**Missing Coverage**:
- Parser: No proptest for arbitrary Perl code generation
- Lexer: No property tests for position/token stream invariants
- Module resolution: No fuzzing of path combinations
- Completion: No property tests for rank/sort ordering

**Impact**: Manual test cases miss random corner cases; parser mutations not validated

**Fix**:
1. Create `perl-test-generators` crate with Arbitrary impls
2. Add property tests for parser invariants (parse→AST→print roundtrip)
3. Add property tests for position mapper (UTF-16 offset invariants)
4. Document patterns in `docs/reference/PROPERTY_TESTING.md`

**Effort**: 8-10 hours  
**Complexity**: High (requires domain understanding)

---

## Gap 3: Snapshot Testing Framework Barely Used

**Current State**:
- ✓ Insta 1.46.1 available in perl-lsp, perl-parser
- ✓ 5 capability snapshots stored in `crates/perl-lsp-rs/tests/snapshots/`
- ✗ **NO**: Snapshot-driven tests for AST structure
- ✗ **NO**: Snapshot regression detection in CI
- ✗ **NO**: Systematic use for error message validation

**Untested Areas**:
- Parser AST snapshots (structure changes invisible)
- LSP capability changes (evolution untracked)
- Error message formatting (no baseline to catch drift)
- Semantic token color scheme (visual changes not caught)

**Impact**: Silent regressions in core data structures; user-visible output changes not noticed

**Fix**:
1. Add snapshot tests for parser error recovery (10-15 snapshots)
2. Add snapshots for AST structure of CPAN edge cases
3. Add semantic token color scheme snapshots
4. Integrate snapshot review into PR workflow

**Effort**: 6-8 hours  
**Complexity**: Medium (workflow+tooling integration)

---

## Gap 4: No Systematic Flaky Test Detection Infrastructure

**Current State**:
- ✓ Debt ledger supports flaky test tracking (`.ci/debt-ledger.yaml`)
- ✓ Budget system in place (max 10 quarantined tests)
- ✗ **NO**: Flaky test detection automation
- ✗ **NO**: Re-run logic in CI (run failed tests 3x)
- ✗ **NO**: Flaky test categorization by root cause

**Observation**: 0 flaky tests currently tracked (debt ledger is empty)

**Problem**: Tests that fail 1/100 runs never get caught; they accumulate until they fail main

**Fix**:
1. Add flakey-tests job to CI (re-runs failed tests 3x)
2. Auto-quarantine tests that flake >2% of runs
3. Create flake root-cause categories (race condition, timing, resource leak)
4. Add flaky test metrics dashboard

**Effort**: 5-6 hours  
**Complexity**: Medium (CI scripting + metrics)

---

## Gap 5: Code Coverage Tracks Lines, Not Branches

**Current State**:
- ✓ cargo-llvm-cov integrated in CI and justfile
- ✓ HTML coverage reports generated (`target/coverage/`)
- ✓ LCOV export for CI integration
- ✗ **NO**: Branch coverage reporting
- ✗ **NO**: Coverage exclusion rules (.coveragerc)
- ✗ **NO**: Coverage trend tracking
- ✗ **NO**: Coverage-gated PRs (coverage must improve or stay same)

**Impact**: 90% line coverage can hide untested branches; recovery paths often untested

**Fix**:
1. Enable branch coverage in llvm-cov
2. Set branch coverage minimum (80%+)
3. Fail PR if coverage decreases
4. Track trends in `.ci/coverage-baseline.txt`

**Effort**: 3-4 hours  
**Complexity**: Low (llvm-cov flag configuration)

---

## Gap 6: Integration/E2E Test Taxonomy Missing

**Current State**:
- ✓ 900 integration test files found
- ✓ 300 E2E test files found
- ✓ 180 golden/transcript test files found
- ✗ **NO**: Documented taxonomy (which tests are which tier)
- ✗ **NO**: Clear ownership (who maintains what category)
- ✗ **NO**: Selective CI runs (slow tests don't run on every PR)

**Problem**: With 900+ integration tests, PR CI takes 20+ min; can't distinguish smoke vs deep tests

**Fix**:
1. Create `tests/TAXONOMY.md` defining test tiers
2. Add `#[test_tier("smoke"|"deep"|"slow")]` attributes
3. Run smoke tier on every PR (<2 min)
4. Run deep tier on merge commit (<10 min)
5. Run slow tier on nightly (<30 min)

**Effort**: 4-5 hours  
**Complexity**: Medium (attribute parsing + CI matrix)

---

## Gap 7: Mutation Testing Run Times Unbounded

**Current State**:
- ✓ cargo-mutants integrated
- ✓ `just mutation-subset` command
- ✓ Mutation regression harnesses
- ✗ **NO**: Time budget enforcement (runs can take hours)
- ✗ **NO**: Incremental/targeted mutation (full workspace every time)
- ✗ **NO**: Mutation coverage reporting

**Observation**: Mutation testing blocked from merge gate due to time

**Fix**:
1. Set 10-min time budget for mutation tests
2. Only mutate changed files (not full workspace)
3. Run mutation tests on nightly gate (not merge gate)
4. Report survivor rate per crate

**Effort**: 3-4 hours  
**Complexity**: Low (cargo-mutants already supports --timeout)

---

## Gap 8: No Test Data Seeding / Corpus Validation

**Current State**:
- ✓ CPAN corpus in `test_corpus/`
- ✓ Tree-sitter corpus in `tree-sitter-perl/test/corpus/`
- ✗ **NO**: Corpus validity checks (are files well-formed?)
- ✗ **NO**: Corpus coverage metrics (which Perl idioms are tested)
- ✗ **NO**: Automated corpus expansion (find uncovered patterns)

**Impact**: Corpus is static; gaps in coverage are invisible

**Fix**:
1. Add `just corpus-validate` command (parses all files, reports errors)
2. Generate coverage report showing which Perl patterns are tested
3. Create issue for each uncovered idiom (closures, tie, smartmatch, etc.)
4. Ratchet coverage baseline upward with each PR

**Effort**: 5-7 hours  
**Complexity**: Medium (analysis + metrics collection)

---

## Priority Breakdown for GitHub Issues

| Gap | Priority | Blocking | Effort | Complexity |
|-----|----------|----------|--------|-----------|
| #1: Benchmark regression | HIGH | No | 4h | Medium |
| #2: Property-based testing | MEDIUM | No | 10h | High |
| #3: Snapshot testing | MEDIUM | No | 8h | Medium |
| #4: Flaky test detection | HIGH | Yes | 6h | Medium |
| #5: Branch coverage | MEDIUM | No | 4h | Low |
| #6: Test taxonomy | HIGH | No | 5h | Medium |
| #7: Mutation time budget | LOW | No | 4h | Low |
| #8: Corpus validation | MEDIUM | No | 7h | Medium |

---

## Recommended Action: Create 6 GitHub Issues

Issues to create (in priority order):

1. **"Implement benchmark regression detection and baseline tracking"** [enhancement]
   - Details: Add critcmp baseline storage, CI comparison, fail on >10% regression

2. **"Add flaky test detection and auto-quarantine infrastructure"** [enhancement]  
   - Details: Re-run failed tests 3x in CI, auto-quarantine flaky tests, track by root cause

3. **"Document and enforce test taxonomy (smoke/deep/slow)"** [enhancement]
   - Details: Create TAXONOMY.md, add tier attributes, configure CI matrix

4. **"Enable branch coverage reporting and gating"** [enhancement]
   - Details: Switch to branch coverage in llvm-cov, set 80%+ minimum, fail on decrease

5. **"Expand property-based testing with generated test data"** [enhancement, swarm-improve-tests]
   - Details: Create perl-test-generators crate, add parser/lexer property tests, document patterns

6. **"Systematize snapshot testing for AST and error messages"** [enhancement, swarm-improve-tests]
   - Details: Add snapshot tests for parser recovery, LSP capabilities, semantic tokens

---

## Non-Issue Recommendations (Quick Wins)

- **Gap #7** (Mutation time budget): Just update justfile to use `--timeout 10m` (15 min fix)
- **Gap #8** (Corpus validation): Can be added as a script, not blocking (opportunistic)

