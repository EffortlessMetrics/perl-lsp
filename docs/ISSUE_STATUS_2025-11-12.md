# GitHub Issues Comprehensive Status Report
<!-- Generated: 2025-11-12 -->
<!-- Research: 30 open issues analyzed and updated -->

> **⚠️ SNAPSHOT DISCLAIMER**: Status snapshot as of 2025-11-12. For live status, treat GitHub issues & milestones as canonical. This document provides comprehensive analysis at a point in time but may not reflect subsequent changes.

## Executive Summary

**Total Open Issues**: 30
**Research Complete**: ✅ All issues analyzed and posted to GitHub
**Critical Blockers**: 3 issues (P0-CRITICAL)
**MVP Completion**: 70-75% (2-3 weeks remaining)
**Production Readiness**: 85-90% (11-13 weeks to v1.0)

---

## 🔬 Semantic & LSP Definition Status (2025-11-20)

**Parser (Phase 1 Complete)**:
- ✅ Semantic Phase 1: 12/12 handlers implemented and compiling
  - `VariableListDeclaration`, `Ternary`, `ArrayLiteral`, `HashLiteral`, `Try`, `PhaseBlock`, `ExpressionStatement`, `Do`, `Eval`, `VariableWithAttributes`, `Unary`, `Readline`
- ✅ SemanticModel API: `build()`, `tokens()`, `symbol_table()`, `hover_info_at()`, `definition_at()`
- ✅ Tests: `semantic_smoke_tests.rs` (13 Phase-1 tests passing, 8 Phase-2/3 ignored as designed)
- ✅ Export: Public API via `pub use semantic::SemanticModel;` in `lib.rs`

**LSP Integration**:
- ✅ `textDocument/definition`: Handler implemented via `SemanticAnalyzer::find_definition()`
- ✅ Tests: `crates/perl-lsp/tests/semantic_definition.rs` (4 scenarios: scalar, sub, lexical scope, package-qualified)
- ⏳ Local execution: Tests compile cleanly; actual execution limited on current dev machine due to resource constraints (WSL2 + low RAM)
- ✅ CI wiring: `just ci-lsp-def` target exists and is wired into `ci-gate`

**Next Actions**:
- [ ] Run `just ci-lsp-def` on higher-capacity machine or GitHub Actions (when billing restored)
- [ ] Verify all 4 semantic definition tests pass (see `docs/ci/MERGE_CHECKLIST_188_phase1.md` for exact commands)
- [ ] Mark `ci-lsp-def` as "stable" in docs once tests execute successfully

**Blockers**: GitHub Actions billing (for automated CI), or access to higher-capacity dev machine for local validation

---

## 🔴 P0-CRITICAL (3 issues) - IMMEDIATE ACTION REQUIRED

### Issue #211: CI Pipeline Cleanup
- **Status**: 🔴 **Open** - Blocking #210
- **Priority**: P0-CRITICAL
- **Effort**: 3 weeks (baseline → optimization → enablement)
- **Cost Impact**: **$720/year savings** (88% reduction: $68/month → $10-15/month)
- **Blockers**: None - can start immediately
- **Dependencies**: Blocks Issue #210 (Merge-Blocking Gates)

**Key Actions**:
1. Create `.ci/scripts/measure-ci-time.sh` for timing infrastructure
2. Run baseline measurement on clean master
3. Consolidate redundant workflows (21 workflows, 3,079 lines → ~1,500 lines)
4. Feature branch validation (10 test PRs)
5. Master branch enablement

**GitHub**: https://github.com/EffortlessMetrics/tree-sitter-perl-rs/issues/211

---

### Issue #210: Formalize Merge-Blocking Gates
- **Status**: 🟡 **Blocked by #211** - Ready for implementation post-#211
- **Priority**: P0-CRITICAL (after #211)
- **Effort**: 8 weeks post-#211 completion
- **Implementation**: 4 phases (Foundation, Scenario Execution, CI Integration, Polish)

**Components**:
- `.ci/gate-policy.yaml` configuration
- LSP scenario harness (`xtask lsp-gates`)
- Receipt generation system (`receipt.json`)
- GitHub Check Runs API integration
- Golden snapshot comparison

**Timeline**:
- Week 1-2: Foundation (policy, harness, receipts)
- Week 3-4: Scenario execution (golden snapshots, performance)
- Week 5-6: CI integration (Check Runs, branch protection)
- Week 7-8: Polish & rollout (informational → soft → hard enforcement)

**GitHub**: https://github.com/EffortlessMetrics/tree-sitter-perl-rs/issues/210

---

###Issue #182: Miscellaneous TODOs (Statement Tracker)
- **Status**: 🔴 **0% Complete** - CRITICAL BLOCKER for Sprint A
- **Priority**: P0-CRITICAL
- **Effort**: 1.5-2.5 days (Days 6-8)
- **Blocks**: Issue #144 AC10 (17 test re-enablement)

**Critical TODO**:
- **Statement Tracker** (`heredoc_parser.rs:473`) - Architecture undefined
- Requires: Issue #183 + #184 completion first
- **URGENT**: 2-hour architecture design session needed

**Additional TODOs** (67+ total):
- 8 refactoring operations (3-4 weeks)
- 2 LSP capabilities (15 minutes)
- 4 parser infrastructure items (4-6 days)
- 14 test infrastructure fixes
- 9 DAP Phase 2/3 items (8-12 weeks, future)

**GitHub**: https://github.com/EffortlessMetrics/tree-sitter-perl-rs/issues/182

---

## 🟠 P1-HIGH (10 issues) - SPRINT A & B

### Sprint B Meta (Issue #213)
- **Status**: 🟡 **Blocked by Sprint A**
- **Priority**: P1-HIGH
- **Effort**: 9 days (21 story points)
- **Target**: 93%+ LSP coverage (from 91%)

**Sprint B Components**:
1. **Issue #180**: Name Spans (3 points, Days 11-13)
2. **Issue #188**: Semantic Analyzer ⭐ (12 points, Days 11-16) - **LARGEST**
3. **Issue #181**: Workspace Features (3 points, Days 11-15)
4. **Issue #191**: Document Highlighting (3 points, Days 16-19)

**Risk**: 40% chance of 2-3 day delay due to semantic analyzer scope

**GitHub**: https://github.com/EffortlessMetrics/tree-sitter-perl-rs/issues/213

---

### Sprint A Meta (Issue #212)
- **Status**: 🟡 **35-40% Complete** (Day 7+ of 10-day sprint)
- **Priority**: P1-HIGH
- **Timeline**: 2-3 weeks realistic (was 10 days)

**Sprint A Components**:
1. **Issue #183**: Heredoc Backreferences - 70% complete (Days 1-3)
2. **Issue #184**: Heredoc Content - 75% complete (Days 3-6)
3. **Issue #185**: Phase Diagnostics - 70% complete (Days 4-5)
4. **Issue #186**: Edge Cases - 15% complete (Days 7-10)
5. **Issue #182**: Statement Tracker - 0% **BLOCKER**
6. **Issue #144**: Test Re-enablement - Blocked

**Critical Path**: #183 → #184 → #182 → #144

**GitHub**: https://github.com/EffortlessMetrics/tree-sitter-perl-rs/issues/212

---

### Issue #208: Batteries Included
- **Status**: 🟢 **80% Infrastructure Complete**
- **Priority**: P1-HIGH
- **Effort**: 3.5 weeks (4 phases)

**Current State**:
- ✅ Perl::Tidy integration (75% complete)
- ✅ Perl::Critic integration (85% complete)
- ✅ Import optimization (90% complete)
- ✅ LSP infrastructure (91% functional)

**Remaining Work**:
- Phase 1: Wire LSP formatting handler (1 week)
- Phase 2: Linting integration (1 week)
- Phase 3: Import on-save (0.5 weeks)
- Phase 4: Polish & packaging (1 week)

**Impact**: Matches Perl Navigator UX with superior performance

**GitHub**: https://github.com/EffortlessMetrics/tree-sitter-perl-rs/issues/208

---

### Issue #204: Eliminate unreachable!() Macros
- **Status**: ✅ **COMPLETE** (PR #205 merged October 2025)
- **Priority**: P1-HIGH (was)
- **Remaining**: Post-MVP audit (67 test/bench instances, 6-12 hours)

**Achievement**:
- Zero unreachable!() in production code
- 100% test pass rate (272/272)
- Enterprise-grade defensive error handling

**Recommendation**: Close issue or update to reflect completion

**GitHub**: https://github.com/EffortlessMetrics/tree-sitter-perl-rs/issues/204

---

### Issue #191: Document Highlighting
- **Status**: 🟡 **Ready for Sprint B** (Days 16-19)
- **Priority**: P1-HIGH
- **Effort**: 2-3 hours
- **Missing**: 17-25 NodeKind handlers

**Implementation**:
- Add missing get_children() handlers
- Leverage #188 work (code reuse opportunity)
- 10+ comprehensive tests

**Impact**: Complete symbol highlighting in try/catch/given/when constructs

**GitHub**: https://github.com/EffortlessMetrics/tree-sitter-perl-rs/issues/191

---

### Issue #188: Handle All AST Node Types
- **Status**: 🟡 **Ready for Sprint B** (Days 11-16)
- **Priority**: P1-HIGH ⭐ **CORNERSTONE**
- **Effort**: 15-16 hours (12 story points)
- **Missing**: 43 NodeKind handlers (27% → 100% coverage)

**6-Phase Implementation**:
1. Expression Wrappers (8 handlers, 2h)
2. Variable Declarations (6 handlers, 3h)
3. Methods/Classes (5 handlers, 2h)
4. Control Flow (8 handlers, 2h)
5. Literals/Operators (8 handlers, 2h)
6. Module System (8 handlers, 2h)

**Risk**: 40% probability of 1-2 day delay

**GitHub**: https://github.com/EffortlessMetrics/tree-sitter-perl-rs/issues/188

---

### Issues #183-186: Sprint A Components
**See Sprint A Meta (#212) above for details**

- **#183**: Heredoc Backreferences (70% complete)
- **#184**: Heredoc Content (75% complete)
- **#185**: Phase Diagnostics (70% complete)
- **#186**: Edge Cases (15% complete)

---

### Issue #180: Implement Missing Parser Features
- **Status**: 🟡 **Ready for Sprint B** (Days 11-13)
- **Priority**: P1-HIGH
- **Effort**: 2-3 hours

**3 TODO Items**:
1. Set proper name_span (1-2 hours) - Blocks #181 Phase 3
2. Context-aware parsing (30-45 minutes)
3. Subroutine creation from inline completions (30 minutes)

**GitHub**: https://github.com/EffortlessMetrics/tree-sitter-perl-rs/issues/180

---

### Issue #181: Complete LSP Feature Implementation
- **Status**: 🟡 **Ready for Sprint B** (Days 11-15)
- **Priority**: P1-HIGH
- **Effort**: 2.5-3 hours

**3 TODO Items**:
1. Track containing package/class (1 hour)
2. Add code action capabilities (15 minutes) - SOURCE_ORGANIZE_IMPORTS already done!
3. Calculate name range in call hierarchy (1-1.5 hours) - Depends on #180

**GitHub**: https://github.com/EffortlessMetrics/tree-sitter-perl-rs/issues/181

---

## 🟡 P2-MEDIUM (11 issues)

### Issue #198: Stabilize Test Infrastructure
- **Status**: 🟡 **779 Ignored Tests** (not 17 as originally stated)
- **Priority**: P2-MEDIUM
- **Effort**: 5 weeks

**Root Causes**:
- 678 tests (87%): BrokenPipe errors (LSP initialization)
- 5 tests: Double-initialization
- 8 tests: Cancellation timeouts
- 88 tests: Various flakiness

**Critical Gap**: CI guardrail doesn't monitor `perl-lsp/tests/`

**Implementation**:
- Week 1: Fix LSP initialization sequence
- Week 2: Validation & smoke tests
- Week 3-4: Progressive test re-enablement
- Week 5: Cleanup & guardrails

**GitHub**: https://github.com/EffortlessMetrics/tree-sitter-perl-rs/issues/198

---

### Issue #197: Add Missing Documentation
- **Status**: 🟡 **484 Violations Baseline**
- **Priority**: P2-MEDIUM
- **Effort**: 8 weeks (phased)

**Infrastructure**: ✅ Complete (PR #160/SPEC-149)
- `#![warn(missing_docs)]` enabled
- 12 acceptance criteria test suite
- Quality gates operational

**Phased Implementation**:
- Phase 1 (Weeks 1-2): Core parser (484 → 350 violations, 27% reduction)
- Phase 2 (Weeks 3-4): LSP providers (350 → 180, 63% total)
- Phase 3 (Weeks 5-6): Advanced features (180 → 50, 90% total)
- Phase 4 (Weeks 7-8): Supporting infrastructure (50 → 0, 100%)

**GitHub**: https://github.com/EffortlessMetrics/tree-sitter-perl-rs/issues/197

---

### Issue #200: Flaky LSP Test Timeout
- **Status**: 🟡 **Open** - Root cause identified
- **Priority**: P2-MEDIUM
- **Effort**: 1 day (immediate fix available)

**Root Cause**: Missing adaptive timeout in `await_index_ready()`
- Current: 500ms fixed timeout
- Issue: CI needs 500ms+ for initialization
- Fix: Adaptive timeout (500ms → 3s for CI environments)

**NOT** a Unicode processing performance issue!

**GitHub**: https://github.com/EffortlessMetrics/tree-sitter-perl-rs/issues/200

---

### Issue #187: Improve Symbol Extraction
- **Status**: 🟢 **Production-Ready** - Optimization opportunities identified
- **Priority**: P2-MEDIUM
- **Effort**: 7-12 days (3 phases)

**Optimization Potential**: 15-40% performance gains

**3-Phase Approach**:
- Phase 1: Low-risk string optimizations (1-2 days, 15% faster)
- Phase 2: String interning (3-5 days, 30-40% memory reduction)
- Phase 3: Advanced optimizations (5-7 days, SIMD, profile-guided)

**GitHub**: https://github.com/EffortlessMetrics/tree-sitter-perl-rs/issues/187

---

### Issue #179: Complete Refactoring Features
- **Status**: 🟡 **8 TODO Items**
- **Priority**: P2-MEDIUM
- **Effort**: 3-4 weeks (16-20 dev days + 4-5 test days)

**TODO Breakdown**:
1. Cleanup backup directories (2-3 hours)
2. Validation logic (1-2 days) **HIGH COMPLEXITY**
3. Backup creation (4-6 hours)
4. Workspace-wide rename (2-3 days) **HIGH PRIORITY**
5. Scoped rename (1 day)
6. Method extraction (3-4 days) **VERY HIGH COMPLEXITY**
7. Code movement (2 days)
8. Inlining (2-3 days)

**GitHub**: https://github.com/EffortlessMetrics/tree-sitter-perl-rs/issues/179

---

### Issue #201: 3 Failing Mutation Tests
- **Status**: 🟡 **Open** - Root cause identified
- **Priority**: P2-MEDIUM
- **Effort**: 1-2 days (~1 hour implementation)

**Failing Tests**:
1. `test_parameter_validation_comprehensive`
2. `test_file_path_extraction_validation`
3. `test_file_not_found_error_structure`

**Root Cause**: Incomplete error message contextualization
- Fragile string parsing of canonicalization errors
- Path context lost during error handling

**Fix**: Check file existence BEFORE calling `canonicalize()`

**Impact**: +6-9% mutation score improvement

**GitHub**: https://github.com/EffortlessMetrics/tree-sitter-perl-rs/issues/201

---

### Issue #193: Apply Lexer Optimizations
- **Status**: 🟢 **Already Highly Optimized** - Post-MVP enhancements
- **Priority**: P2-MEDIUM (deferred)
- **Effort**: 5 weeks (phased)

**Current Status**: 20-30% recent speedup achieved

**Optimization Opportunities**:
- SIMD optimizations (10-30% potential gain)
- Regex caching (5-15% on heredoc-heavy files)
- Memory allocation optimization (2-5%)

**Expected Cumulative**: 15-35% improvement (conservative)

**Recommendation**: Defer pending Sprint A/B completion

**GitHub**: https://github.com/EffortlessMetrics/tree-sitter-perl-rs/issues/193

---

### Issue #190: Default Value Logging
- **Status**: 🟡 **Post-MVP Enhancement**
- **Priority**: P2-MEDIUM (deferred)
- **Effort**: 24-36 hours

**Context**: Log when parser substitutes default values during error recovery

**Current State**:
- Documentation exists but no implementation
- Logging infrastructure already available (tracing crate)
- LSP logging support via `--log` flag

**Implementation**: 3 phases
- Phase 1: Infrastructure setup (4-8h)
- Phase 2: Parser integration (8-16h)
- Phase 3: Testing (8-12h)

**GitHub**: https://github.com/EffortlessMetrics/tree-sitter-perl-rs/issues/190

---

### Issue #189: Make Inlay Hints Work
- **Status**: 🟡 **Post-MVP** - Same root cause as #191
- **Priority**: P2-MEDIUM (deferred)
- **Effort**: 3-5 hours (reduced from 5-7h by leveraging #191)

**Root Cause**: Incomplete AST traversal (same as #191)
- Missing ~17-25 NodeKind handlers
- **Code reuse opportunity** from #191

**Implementation**:
- Wait for #191 completion (Sprint B)
- Leverage `get_node_children()` enhancements
- Update `inlay_hints_provider.rs` (2-3 hours additional)

**GitHub**: https://github.com/EffortlessMetrics/tree-sitter-perl-rs/issues/189

---

### Issues #185-186: Sprint A Components
**See Sprint A Meta (#212) above**

---

## 🟢 P3-LOW (6 issues)

### Issue #203: Upgrade xtask to Rust 2024
- **Status**: ✅ **COMPLETE** (PR #175 merged Oct 28, 2025)
- **Priority**: P3-LOW (was)

**Resolution**:
- Workspace upgraded to Rust edition 2024
- Let-chain syntax enabled and functional
- Zero compilation errors

**Recommendation**: **CLOSE ISSUE**

**GitHub**: https://github.com/EffortlessMetrics/tree-sitter-perl-rs/issues/203

---

### Issue #202: Update Deprecated rand::Rng::gen_range
- **Status**: ✅ **COMPLETE** (commit e768294f, Oct 1, 2025)
- **Priority**: P3-LOW (was)

**Resolution**:
- Migrated `gen_range` → `random_range`
- Zero deprecation warnings remaining
- All tests passing

**Recommendation**: **CLOSE ISSUE**

**GitHub**: https://github.com/EffortlessMetrics/tree-sitter-perl-rs/issues/202

---

### Issue #196: Production Roadmap
- **Status**: 🟢 **85-90% Production Ready**
- **Priority**: P3-LOW (planning document)
- **Timeline**: 6-10 months to full production maturity

**Production Readiness**:
- Parser: 95% (excellent)
- LSP Core: 91% (strong)
- Performance: 98% (excellent)
- Testing: 99.6% (excellent)
- Security: 95% (strong)
- Documentation: 70% (good, 484 violations)
- CI/CD: 40% (needs work)

**Timeline**:
- Production v1.0 (MVP + Phase 1): 11-13 weeks
- Production v1.1 (+ Phase 2): 23 weeks
- Production v1.2 (+ Phase 3): 31 weeks
- Production v2.0 (+ Phase 4): 43 weeks

**GitHub**: https://github.com/EffortlessMetrics/tree-sitter-perl-rs/issues/196

---

### Issue #195: MVP Roadmap
- **Status**: 🟡 **70-75% Complete**
- **Priority**: P3-LOW (planning document)
- **Timeline**: 2-3 weeks to MVP

**MVP Components (4/6 complete)**:
- ✅ Robust Parsing (95%, heredoc edge cases remaining)
- ✅ Accurate Diagnostics (100%)
- ✅ Basic Code Completion (100%)
- ✅ Go-to-Definition (100%)
- 🔄 Sprint A (Parser Foundation) - In progress
- 🔄 Sprint B (LSP Polish) - Blocked by Sprint A

**Critical Path**: Sprint A → Sprint B → MVP Launch

**GitHub**: https://github.com/EffortlessMetrics/tree-sitter-perl-rs/issues/195

---

### Issue #194: Implement type_hierarchy_provider
- **Status**: ✅ **ALREADY IMPLEMENTED** - Misleading title!
- **Priority**: P3-LOW (was)

**Discovery**: Type hierarchy is FULLY IMPLEMENTED (v0.8.8+)
- 564 lines of production code
- 11+ comprehensive tests
- All Perl inheritance mechanisms supported
- **Issue**: Tests marked `#[ignore]` due to CI flakiness (not feature defects)

**Recommendation**: Update issue title to "Enable Type Hierarchy Tests in CI"

**GitHub**: https://github.com/EffortlessMetrics/tree-sitter-perl-rs/issues/194

---

### Issue #192: Address Deprecated compat.rs
- **Status**: 🟢 **Ready for Removal** (v0.9.0 target)
- **Priority**: P3-LOW
- **Effort**: 1 sprint (2 weeks)

**Current State**:
- 12 deprecated items (100% migration complete)
- Zero current usage in codebase
- Test-only feature (`test-compat` flag)

**Removal Strategy**: 3 phases
- Phase 1: Validation (1 week)
- Phase 2: Escalation (2 weeks)
- Phase 3: Removal in v0.9.0

**GitHub**: https://github.com/EffortlessMetrics/tree-sitter-perl-rs/issues/192

---

## 📊 Summary Statistics

### By Priority
| Priority | Count | Status |
|----------|-------|--------|
| P0-CRITICAL | 3 | 1 blocked, 2 open |
| P1-HIGH | 10 | 3 blocked, 6 ready, 1 complete |
| P2-MEDIUM | 11 | 8 open, 3 identified solutions |
| P3-LOW | 6 | 3 complete, 3 planning/ready |

### By Sprint
| Sprint | Issues | Status |
|--------|--------|--------|
| Sprint A | 6 | 35-40% complete, Day 7+ of 10 |
| Sprint B | 4 | Blocked by Sprint A |
| Post-MVP | 8 | Deferred appropriately |
| Infrastructure | 3 | P0 priority |
| Complete | 3 | Can be closed |

### Overall Health
- **MVP Timeline**: 2-3 weeks (realistic estimate)
- **Production v1.0**: 11-13 weeks
- **Critical Blockers**: 3 (all identified and actionable)
- **Test Coverage**: 99.6% (828/830 tests passing)
- **Ignored Tests**: 779 (87% due to LSP initialization)
- **Documentation**: 484 violations (infrastructure complete)

---

## 🎯 Recommended Immediate Actions

### This Week
1. ✅ **Close Issues #203, #202** - Already resolved
2. 🔴 **Statement Tracker Architecture Session** (#182) - 2 hours
3. 🔴 **Fix CI Guardrail** (#198) - Monitor perl-lsp tests
4. 🟡 **Merge PR #214** - CI infrastructure (Sprint A blocker)
5. 🟡 **Complete #183** - Heredoc declaration (Days 1-3)

### Next 2 Weeks (Sprint A)
1. Complete #184, #185, #186
2. Implement #182 (statement tracker)
3. Re-enable tests (#198 Phase 1)
4. Begin #211 (CI pipeline optimization)

### Weeks 3-4 (Sprint B)
1. Execute #180, #188, #181, #191
2. Achieve 93%+ LSP coverage
3. Prepare for Production v1.0

---

## 📚 Research Methodology

**Research Conducted**: 2025-11-12
**Agent**: github-pr-issue-researcher (30 deployments)
**Coverage**: 100% of open issues (30/30)
**Analysis Depth**: ~100,000+ words across all reports
**GitHub Comments**: 30 comprehensive reports posted

**Validation**:
- Cross-referenced with CLAUDE.md
- Integrated with codebase analysis
- Aligned with Sprint A/B planning (Issues #212, #213)
- Validated against production roadmap (#196)

---

*This report provides complete intelligence on all open GitHub issues, enabling informed decision-making and efficient sprint planning for the perl-lsp project.*

*Last Updated: 2025-11-12*
*Next Review: After Sprint A completion*
