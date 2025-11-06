# Sprint A Immediate Action Plan
**Status**: 🟡 YELLOW CAUTION - Pre-Sprint Preparation Required
**Target Sprint Start**: 2025-11-18 (2-week deferral recommended)

---

## TL;DR for Leadership

**Sprint A is NOT ready for immediate kickoff.** While meta-coordination (issue #212) is excellent, critical technical specifications are 40-60% complete, with three **P0 blockers** requiring resolution:

1. **Infrastructure Audit** (4 hours) - Verify heredoc handling files exist
2. **Statement Tracker Architecture** (6 hours) - Design most underspecified component
3. **Test Inventory** (3 hours) - Identify 17 target tests for re-enablement

**Recommendation**: Defer Sprint A kickoff by 1-2 weeks to complete pre-sprint preparation. Alternative: Execute partial sprint focusing only on #185 (phase diagnostics) + #183 foundation (heredoc declaration), deferring complex components to Sprint A.5.

**Success Probability**:
- With Deferral: **75%** ✅
- Partial Sprint: **60%** ⚠️
- As-Planned: **25%** ❌

---

## Critical Path Reality Check

### Documented Plan (from #212)
```
Day 1-3: #183 Heredoc declaration →
Day 3-6: #184 Content collector →
Day 6-8: #182 Statement tracker →
Day 8-10: #144 Test re-enablement

Parallel: #185 (Day 4-5), #186 (Day 7-10)
```

### Realistic Assessment
```
Week 1: Infrastructure discovery + #183 partial + #185 complete
Week 2: #183 done + #184 start (optimistic)
Week 3: #184 done + #182 start (BLOCKER RISK)
Week 4: #182 partial + #186 detection + #144 partial
```

**Key Issue**: #182 (statement tracker) is **critically underspecified** and blocks entire critical path.

---

## Issue Readiness Scorecard

| Issue | Title | Readiness | Blocker Severity | Dependencies |
|-------|-------|-----------|------------------|--------------|
| #183 | Heredoc Declaration Parser | 🟡 60/100 | MEDIUM | None (but files may be missing) |
| #184 | Heredoc Content Collector | 🟡 55/100 | HIGH | Requires #183 + AST integration |
| #185 | Phase Diagnostics | 🟢 75/100 | **LOW** | None - **SAFE TO START** |
| #182 | Statement Tracker | 🔴 40/100 | **CRITICAL** | Requires #183 + #184 + new architecture |
| #186 | Edge Case Handler | 🟡 50/100 | MEDIUM | Independent but needs detection framework |
| #144 AC10 | Test Re-enablement | 🔴 35/100 | HIGH | Requires all above + test inventory |

**Safest Path Forward**: Start with #185 (green), stabilize #183 (yellow), defer #182 (red blocker).

---

## Three P0 Blockers (Must Resolve Before Sprint)

### 🚨 Blocker 1: Infrastructure Audit (Priority: P0)
**Problem**: Referenced files in issues may not exist or differ from documentation

**Impact**: Day 1 implementation could stall on missing infrastructure

**Resolution Steps**:
```bash
cd /home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs

# Run comprehensive audit
echo "=== Sprint A Infrastructure Audit ===" > audit_report.txt

# Check for heredoc files
find crates/ -name "*heredoc*" -type f >> audit_report.txt

# Verify statement tracking infrastructure
rg "StatementTracker|statement_tracker" crates/perl-parser/src/ >> audit_report.txt

# Check AST heredoc node definitions
rg "AstNode.*Heredoc" crates/perl-parser/src/ -A 3 >> audit_report.txt

# Verify ParseError types exist
rg "ParseError" crates/perl-parser/src/ -A 3 >> audit_report.txt

cat audit_report.txt
```

**Estimated Effort**: 4 hours
**Owner**: Parser Lead
**Deliverable**: `audit_report.txt` with gaps identified + mitigation plan

**Success Criteria**:
- ✅ Confirm `/crates/perl-parser/src/` has heredoc handling infrastructure OR
- ✅ Create stubs for missing files with TODO markers
- ✅ Identify AST node structure for heredoc placement
- ✅ List all ParseError variants needed for Sprint A

---

### 🚨 Blocker 2: Statement Tracker Architecture (Priority: P0)
**Problem**: Issue #182 is most underspecified with highest failure risk

**Impact**: Critical path blocks at Day 6; entire Sprint A fails if #182 stalls

**Resolution Steps**:

#### Step 1: Architecture Design Session (2 hours)
**Participants**: All Sprint A developers + parser architect

**Agenda**:
1. Draw statement tracker data flow (whiteboard/Mermaid diagram)
2. Define `StatementTracker` struct fields and methods
3. Specify block entry/exit detection mechanism
4. Document AST node hierarchy for heredocs in blocks
5. Design content spanning beyond block boundaries strategy

#### Step 2: Create Architecture Document (2 hours)
**File**: `docs/STATEMENT_TRACKER_ARCHITECTURE.md`

**Required Sections**:
- Data structures (StatementTracker, HeredocContext)
- Integration points with existing parser
- Block depth tracking algorithm
- Content collection strategy for nested blocks
- AST node placement rules
- Error handling for malformed block structures

#### Step 3: Implement Statement Tracker Stub (2 hours)
```rust
// crates/perl-parser/src/statement_tracker.rs (create if missing)

/// Tracks statement boundaries for heredoc content collection
pub struct StatementTracker {
    /// Current block nesting depth (0 = top-level)
    block_depth: usize,
    /// Heredoc declarations encountered with their context
    heredoc_contexts: Vec<HeredocContext>,
}

/// Context for a heredoc declaration within a statement
pub struct HeredocContext {
    /// Line number where heredoc was declared
    declaration_line: usize,
    /// Block depth at time of declaration
    block_depth_at_declaration: usize,
    /// Terminator label (e.g., "EOF", "END")
    terminator: String,
    /// Full heredoc declaration metadata (from #183)
    decl: HeredocDecl,
}

impl StatementTracker {
    pub fn new() -> Self {
        Self {
            block_depth: 0,
            heredoc_contexts: Vec::new(),
        }
    }

    pub fn enter_block(&mut self) {
        self.block_depth += 1;
    }

    pub fn exit_block(&mut self) {
        self.block_depth = self.block_depth.saturating_sub(1);
    }

    pub fn record_heredoc(&mut self, line: usize, decl: HeredocDecl) {
        self.heredoc_contexts.push(HeredocContext {
            declaration_line: line,
            block_depth_at_declaration: self.block_depth,
            terminator: decl.label.clone(),
            decl,
        });
    }

    pub fn collect_content(&mut self, lines: &[&str]) -> Vec<(HeredocContext, String)> {
        // TODO: Implement content collection algorithm
        // Allow content to extend beyond block boundaries
        // Associate heredoc with correct block in AST
        Vec::new()
    }
}
```

**Estimated Effort**: 6 hours total
**Owner**: Parser Architect + Senior Developer
**Deliverable**:
- `docs/STATEMENT_TRACKER_ARCHITECTURE.md` (architecture document)
- `crates/perl-parser/src/statement_tracker.rs` (stub implementation)
- Unit tests for basic block depth tracking

**Success Criteria**:
- ✅ Architecture document reviewed and approved by team
- ✅ Stub compiles without errors
- ✅ Basic unit tests pass (enter_block/exit_block/record_heredoc)
- ✅ Integration points with parser clearly documented

---

### 🚨 Blocker 3: Test Inventory for AC10 (Priority: P1)
**Problem**: Cannot validate Sprint A success without clear test targets

**Impact**: No objective measure of Sprint A completion

**Resolution Steps**:
```bash
cd /home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs

# Generate comprehensive test inventory
echo "Issue,Test File,Test Name,Current Status,Reason Ignored,Sprint A Target" > test_inventory.csv

# Find all ignored tests related to heredocs, phases, statements
rg "#\[ignore\]" crates/perl-parser/tests/ crates/perl-lexer/tests/ -B 5 -A 1 \
  | rg -i "heredoc|phase|statement|BEGIN|END|CHECK|INIT" \
  | tee test_inventory_raw.txt

# Check test compilation status
echo -e "\n=== Test Compilation Status ===" >> test_inventory_raw.txt
cargo test --workspace --no-run 2>&1 | tee -a test_inventory_raw.txt

# Manual classification step: Review test_inventory_raw.txt and populate test_inventory.csv
```

**Manual Classification Required**:
For each ignored test in `test_inventory_raw.txt`, fill out:
1. Which Sprint A issue addresses it? (#183, #184, #185, #182, #186)
2. What is current failure mode? (compile error, runtime panic, assertion failure)
3. What must be implemented to enable it?

**Example CSV Format**:
```csv
Issue,Test File,Test Name,Current Status,Reason Ignored,Sprint A Target
#183,heredoc_tests.rs,test_single_quote_heredoc,compile_error,missing HeredocStyle::SingleQuoted,Day 3
#184,heredoc_tests.rs,test_heredoc_content_collection,not_implemented,content collector missing,Day 6
#185,phase_tests.rs,test_end_block_warning,assertion_fail,missing END diagnostic,Day 5
#182,statement_tests.rs,test_heredoc_in_if_block,runtime_panic,statement tracker missing,Day 8
```

**Estimated Effort**: 3 hours
**Owner**: QA Lead
**Deliverable**:
- `test_inventory.csv` (17+ rows, one per ignored test)
- `test_enablement_plan.md` (mapping tests to Sprint A issues)

**Success Criteria**:
- ✅ All 17 target tests identified and categorized
- ✅ Each test mapped to specific Sprint A issue (#183-#186)
- ✅ Current failure mode documented for each test
- ✅ Clear acceptance criteria for test re-enablement

---

## Pre-Sprint Preparation Timeline

### Week 1 (Nov 5-11): Infrastructure + Architecture
**Goal**: Resolve all 3 P0 blockers

| Day | Activity | Owner | Deliverable | Hours |
|-----|----------|-------|-------------|-------|
| Day 1 (Tue) | Infrastructure Audit | Parser Lead | audit_report.txt | 4 |
| Day 2 (Wed) | Statement Tracker Architecture Session | All Devs | Whiteboard notes | 2 |
| Day 2 (Wed) | Create Architecture Document | Parser Architect | STATEMENT_TRACKER_ARCHITECTURE.md | 2 |
| Day 3 (Thu) | Implement Statement Tracker Stub | Senior Dev | statement_tracker.rs | 2 |
| Day 3 (Thu) | Generate Test Inventory | QA Lead | test_inventory.csv | 3 |
| Day 4 (Fri) | Review and Validate Blockers | All Leads | Readiness review meeting | 2 |

**Total Effort**: 15 hours (distributed across team)

### Week 2 (Nov 12-18): Documentation + Preparation
**Goal**: Close documentation gaps, prepare Sprint A environment

| Day | Activity | Owner | Deliverable | Hours |
|-----|----------|-------|-------------|-------|
| Day 5 (Mon) | Parser Architecture Overview | Parser Architect | PARSER_ARCHITECTURE.md | 4 |
| Day 6 (Tue) | Heredoc Lifecycle Diagram | Tech Writer | heredoc_lifecycle.png | 2 |
| Day 7 (Wed) | Complete ParseError Taxonomy | Parser Lead | error_types.rs updates | 2 |
| Day 8 (Thu) | Create Test Fixtures | QA Lead | CRLF, UTF-8, edge case samples | 4 |
| Day 9 (Fri) | Sprint Kickoff Checklist | PM | SPRINT_A_CHECKLIST.md | 1 |
| Day 9 (Fri) | Sprint A Go/No-Go Meeting | All | Final readiness decision | 1 |

**Total Effort**: 14 hours (distributed across team)

**Sprint A Start Date**: **Monday, November 18, 2025** ✅

---

## Alternative: Partial Sprint Option

**If deferral is not acceptable**, execute **Partial Sprint** with reduced scope:

### Partial Sprint Goals (Reduced Risk)
- ✅ **Complete #185** (phase diagnostics) - 2 days - **GREEN LIGHT**
- ✅ **Implement #183 foundation** (heredoc declaration parsing) - 4 days - **YELLOW CAUTION**
- ⏸️ **Defer #184** (content collector) to Sprint A.5 - **Requires AST clarity**
- ⏸️ **Defer #182** (statement tracker) to Sprint A.5 - **Requires architecture**
- ❌ **Cancel #186** (edge cases) - Move to Sprint C - **Not critical path**
- ❌ **Cancel #144 AC10** (test re-enablement) - Revisit after Sprint A.5

### Partial Sprint Timeline (10 Days)
```
Day 1-2: #185 Phase Diagnostics (COMPLETE)
Day 3-6: #183 Heredoc Declaration Parser (FOUNDATION ONLY)
Day 7-8: #183 Testing and Documentation
Day 9-10: Sprint Retrospective + Sprint A.5 Planning
```

### Partial Sprint Success Criteria
- ✅ #185 complete with all BEGIN/END/CHECK/INIT warnings improved
- ✅ #183 foundation compiles and passes basic unit tests
- ✅ HeredocDecl struct defined and integrated with lexer
- ✅ Manual parsing algorithm implemented (no backreferences)
- ⏸️ Content collection interface specified but not implemented
- ⏸️ Statement tracking deferred with clear requirements documented

**Probability of Success**: 60% (manageable risk with clear scope reduction)

---

## Sprint A Readiness Checklist

Use this checklist for Go/No-Go decision on **Friday, November 15**:

### P0 Blockers (Must Be Green)
- [ ] **Blocker 1**: Infrastructure audit complete, gaps documented
- [ ] **Blocker 2**: Statement tracker architecture approved, stub implemented
- [ ] **Blocker 3**: Test inventory CSV complete (17+ tests mapped)

### Documentation (Must Be Complete)
- [ ] Parser architecture overview document exists
- [ ] Heredoc lifecycle diagram created
- [ ] ParseError taxonomy complete
- [ ] Statement tracker integration points documented

### Code Preparation (Must Be Ready)
- [ ] Statement tracker stub compiles
- [ ] ParseError types defined for Sprint A scenarios
- [ ] Test fixtures created (CRLF, UTF-8, edge cases)
- [ ] CI pipeline validated (cargo test --workspace passes)

### Team Readiness (Must Be Confirmed)
- [ ] All developers have reviewed issue #212 (meta-coordination)
- [ ] All developers have reviewed issue specifications (#183-#186)
- [ ] All developers have reviewed STATEMENT_TRACKER_ARCHITECTURE.md
- [ ] QA lead has validated test inventory and acceptance criteria
- [ ] PM has confirmed no competing priorities during Sprint A

### Success Metrics (Must Be Agreed)
- [ ] Definition of "done" for each issue documented
- [ ] Mutation testing criteria defined (target: ≥75%, stretch: ≥85%)
- [ ] Performance baseline captured (must not regress by >5%)
- [ ] Code review process established (2 approvers required)

**Go/No-Go Decision Criteria**:
- **GO**: All P0 blockers green + ≥80% documentation complete + team confirmed ready
- **PARTIAL GO**: All P0 blockers green + execute Partial Sprint option
- **NO-GO**: Any P0 blocker red OR <60% documentation complete

---

## Risk Mitigation Strategies

### If #182 Stalls During Sprint (Contingency Plan)
**Trigger**: By Day 7, #182 implementation has not started or is blocked

**Action**:
1. **Immediate**: Convene architecture review meeting (2 hours)
2. **Options**:
   - **Option A**: Simplify #182 scope (handle only top-level blocks, defer nested)
   - **Option B**: Defer #182 to Sprint A.5, proceed with #186 + #144 partial
   - **Option C**: Extend Sprint A by 3 days (requires Sprint B deferral)
3. **Communication**: Update issue #212 with revised timeline + rationale

### If Test Re-enablement (#144 AC10) Falls Short
**Trigger**: By Day 10, <10 tests re-enabled (target was 17)

**Action**:
1. **Triage**: Categorize remaining ignored tests by blocker type
2. **Partial Success**: Document which tests were enabled and why
3. **Deferred Work**: Create Sprint A.5 with remaining test targets
4. **Success Redefinition**: Adjust exit criteria to "≥50% tests enabled" (≥8 tests)

### If Performance Regresses (>5% Baseline)
**Trigger**: Benchmark shows parsing time increased by >5%

**Action**:
1. **Profiling**: Run `cargo flamegraph` to identify hotspots
2. **Triage**: Determine if regression is due to #183, #184, or #182
3. **Options**:
   - **Option A**: Optimize critical path (delay Sprint A by 2 days)
   - **Option B**: Accept regression if <10% and creates tech debt ticket
   - **Option C**: Revert offending change and redesign

---

## Communication Plan

### Sprint A Stakeholder Updates

**Daily Standup** (15 minutes, 9:00 AM):
- What did I complete yesterday?
- What will I work on today?
- What blockers am I facing? (RED/YELLOW/GREEN status)
- Focus on #182 progress (critical path)

**Weekly Status Report** (Fridays):
- Issue completion status (#183, #184, #185, #182, #186, #144 AC10)
- Test re-enablement progress (X of 17 tests enabled)
- Performance benchmarks (vs. baseline)
- Risks and mitigation actions
- Next week priorities

**Sprint A Retrospective** (Day 11, 2 hours):
- What went well? (process, technical decisions)
- What didn't go well? (blockers, surprises)
- What should we do differently in Sprint B?
- Lessons learned for future sprint planning

---

## Success Metrics and Acceptance Criteria

### Quantitative Metrics
- **Test Re-enablement**: ≥8 tests (50% of 17 target) OR ≥17 tests (100% stretch goal)
- **Mutation Score**: ≥75% (realistic) OR ≥85% (stretch)
- **Performance**: <5% regression vs. baseline
- **Code Coverage**: ≥80% for new heredoc/phase/statement code
- **Zero Parser Crashes**: 1M fuzzing iterations without panic

### Qualitative Criteria
- ✅ All implemented code has comprehensive unit tests
- ✅ Statement tracker architecture approved and documented
- ✅ Heredoc lifecycle clearly explained in documentation
- ✅ Phase diagnostics provide actionable error messages
- ✅ Code review feedback incorporated (2 approvals per PR)
- ✅ Sprint B unblocked (no architectural technical debt)

---

## Next Steps (Immediate Actions)

### This Week (Nov 5-11)
1. **Tuesday PM**: Run infrastructure audit script (Blocker 1)
2. **Wednesday AM**: Statement tracker architecture session (Blocker 2)
3. **Wednesday PM**: Create statement tracker architecture document
4. **Thursday AM**: Implement statement tracker stub
5. **Thursday PM**: Generate test inventory (Blocker 3)
6. **Friday AM**: Review all 3 blockers with team
7. **Friday PM**: Go/No-Go decision for Sprint A vs. Partial Sprint vs. Full Deferral

### Next Week (Nov 12-18)
1. **Monday**: Parser architecture overview document
2. **Tuesday**: Heredoc lifecycle diagram + ParseError taxonomy
3. **Wednesday**: Test fixtures creation
4. **Thursday**: Pre-sprint environment setup (CI validation, dependency updates)
5. **Friday**: Sprint A kickoff checklist + final readiness review

### Sprint A Start (Nov 18+)
1. **Day 1**: #185 implementation (phase diagnostics)
2. **Day 2**: #185 testing and #183 start (heredoc declaration)
3. **Day 3-6**: #183 + #184 implementation (critical path)
4. **Day 6-8**: #182 implementation (statement tracker)
5. **Day 7-10**: #186 + #144 AC10 (edge cases + test re-enablement)
6. **Day 11**: Sprint retrospective + Sprint B planning

---

## Conclusion

**Sprint A has excellent meta-coordination (#212) but incomplete technical specifications.** The team should either:

1. **RECOMMENDED**: Defer by 1-2 weeks to resolve 3 P0 blockers (75% success probability)
2. **ALTERNATIVE**: Execute Partial Sprint (#185 + #183 foundation) and defer complex components (60% success probability)
3. **NOT RECOMMENDED**: Proceed as-planned (25% success probability, high failure risk)

**Next Decision Point**: Friday, November 15, 2025 - Go/No-Go meeting based on this checklist.

---

**Document Owner**: Sprint A PM
**Last Updated**: 2025-11-05
**Next Review**: 2025-11-15 (Go/No-Go meeting)
