# Swarm Methodology Improvements Analysis

**Analysis Date:** 2026-03-19
**Sessions Analyzed:** Cycles 3-5 (6 weeks, 100+ agents, 56 PRs, 80+ issues)
**Evidence Sources:** Memory files (106), hook system, skills, agent patterns, CI/merge queue metrics

---

## Executive Summary

The swarm has demonstrated strong empirical patterns: 90% success on constrained tasks (parser fixes, TDD), 50% on unconstrained features, 3-wide merge queue bottleneck, and evidence of process gaps (test assertion bugs, phantom error buckets, version drift). This analysis identifies **7 critical improvements** and **12 tactical enhancements** that will compound across future sessions.

---

## 1. CRITICAL IMPROVEMENTS (High Impact, Moderate-High Effort)

### 1.1 Auto-Update CURRENT_STATUS.md on Test Changes

**Impact:** Eliminates the #1 merge blocker (`policy_checks` gate failures)
**Effort:** 1-2 hours
**Evidence:**
- Cycle 5: 4/5 review-batch-1 PRs blocked by stale test counts
- Root cause: Adding tests changes count, but CURRENT_STATUS.md is computed and must be regenerated
- Current state: `update-current-status.py --check` fails CI, agents know to run `--write` but it's a manual step

**Implementation Options:**
1. **Hook-based (preferred):** Add PostToolUse hook that auto-runs `python3 scripts/update-current-status.py --write` after any Edit/Write that adds tests
2. **CI-based:** Modify policy_checks to auto-compute and update CURRENT_STATUS.md if tests changed (simpler but less feedback)
3. **Agent-built-in:** Add `update-current-status.py --write` to `/verify-build` skill verification step

**Recommendation:** Implement #1 (hook) + #3 (skill). The hook catches accidental misses; the skill ensures intentional runs.

---

### 1.2 Post-Merge Issue Closure Automation

**Impact:** Removes stale issue tracker entries (8 refactoring issues were already resolved)
**Effort:** 2-3 hours
**Evidence:**
- Cycle 5: All 8 open refactoring issues had already merged — issue tracker was stale
- Each issue that should be closed wastes orchestrator time on dedup scouts
- GitHub PR `Fixes #N` directive auto-closes issues but agents don't always use it

**Implementation:**
1. Add hook: `PostToolUse` on `gh pr merge` — auto-close associated issues
2. Add `/issue-cleanup` skill — scans open issues, checks if their linked PR merged, auto-closes
3. Modify agent PR templates to always include "Fixes #N" in body

**Expected ROI:** Saves 10-15 min per session on dedup work. 80+ issues in backlog need closure audit.

---

### 1.3 Worktree Isolation Enforcement (Prevent Contention)

**Impact:** Eliminates shared-worktree conflicts that cause file reverts and retry cascades
**Effort:** 3-4 hours
**Evidence:**
- `readme-polish-012` worktree: 5+ agents shared, caused branch switching conflicts, file reverts, wasted retries
- Version bump agent: "Concurrent agents switching branches caused file reverts"
- Multiple agents reported worktree state inconsistency

**Implementation:**
1. **Hard requirement:** Modify agent spawn instructions to ALWAYS use `isolation: "worktree"`
2. **Hook validation:** PreToolUse hook that fails if agent uses `git checkout` without `isolation: "worktree"`
3. **Cleanup:** Add `/cleanup-worktrees` skill to prune abandoned worktree branches
4. **Naming convention:** `.claude/worktrees/<issue-number>-<description>` prevents collisions

**Current State:** Settings already support symlinked target dirs. Hook enforcement missing.

---

### 1.4 CI Queue Awareness in Merge Sequencing

**Impact:** Prevent merge queue from starving due to CI backlog
**Effort:** 4-5 hours
**Evidence:**
- Merge queue is 3-wide (rapid merges cancel CI)
- Cycle 5: 75 agents generating 50+ PRs created merge backlog
- Optimal agent count ≈ 3 (queue width) × 15 min (avg work time) / 5 min (merge cycle) = 9 agents
- With 100 agents, merge queue became the bottleneck

**Implementation:**
1. `/merge-queue` skill: Check `gh run list --workflow=ci.yml --limit=10` before attempting merge
2. If CI queue has >5 pending runs, agent parks and tries again in 30s
3. Dashboard: Log queue depth into swarm-metrics.jsonl for trending
4. Feedback: Optimal concurrent agents ≈ 9 for this queue width

**Expected Impact:** Prevent cascading merge delays. Allow safe scaling to 50-75 agents without starvation.

---

### 1.5 Corpus Ratchet as Post-Merge Automation

**Impact:** Free 3-4% corpus improvement (249 files in Cycle 5 were already fixed but not ratcheted)
**Effort:** 1-2 hours
**Evidence:**
- Cycle 5: Error buckets #2 (140 files) and #3 (109 files) were already fixed on master
- Tests pass, code is in place, but baseline was never ratcheted
- Ratchet is manual, gets forgotten
- One `/corpus-ratchet` skill run would have added 3-4% immediately

**Implementation:**
1. Post-merge hook: After parser fix PRs merge, trigger `/corpus-ratchet`
2. Add `just cpan-corpus-ratchet` to post-merge operations
3. Skill should verify results and create a companion commit if corpus improves

**Expected Value:** 2-3% corpus improvement per merge wave with zero code effort.

---

### 1.6 Test Assertion Bug Fix (assert_clean_parse Case Sensitivity)

**Impact:** Fix false positives in test infrastructure (30 silent test passes)
**Effort:** 1 hour
**Evidence:**
- Test helper checks for `(error` and `(Error` but `to_sexp()` emits `(ERROR`
- Result: 30 tests in `fix_expected_colon.rs` silently pass despite parser errors
- This is a "when receipts lie" example — test infrastructure can't fail when it should

**Implementation:**
1. Fix: Add `(ERROR` to the case-insensitive match in `cpan_test_helpers/mod.rs`
2. Run tests: Discover which 30 tests now fail
3. Fix: Correct the 30 newly-visible test failures
4. Verify: Run full test suite, ensure no regressions

**Root Cause Pattern:** Test helpers should have comprehensive coverage of output formats. Consider: are there other S-expression variants we're missing?

---

### 1.7 Phantom Error Bucket Audit (Corpus Misclassification)

**Impact:** Fix inflated corpus metrics (bucket #5 doesn't actually exist in parser)
**Effort:** 2-3 hours
**Evidence:**
- Cycle 5: Bucket #5 (`unexpected_rbrace_expr`: 83 files) is defined in mapping but never emitted by parser
- This is a classification artifact from substring matching
- Corpus metrics may be overstated due to phantom buckets

**Implementation:**
1. Audit SEMANTIC_BUCKETS mapping in xtask against actual parser error strings
2. Remove phantom buckets, update classifier
3. Re-run corpus analysis — identify which files misclassified
4. Recalculate corpus %; adjust baseline if needed
5. Create builder issues for the REAL error buckets now visible

**Expected Outcome:** Honest corpus metrics. Possibly discover new buckets that need fixing.

---

## 2. TACTICAL IMPROVEMENTS (High Impact, Low-Moderate Effort)

### 2.1 Constrain Feature Tasks via Scout Phase

**Impact:** Improve feature agent success from 50% to 80%+
**Effort:** Process change, no code
**Evidence:**
- Constrained tasks (parser fixes, TDD): ~90% success
- Feature work (unconstrained): ~50% success
- Scout output that identifies exact file paths, function signatures, APIs improves success dramatically

**How to Apply:**
For any feature work >30 lines, require a scout phase first:
1. Scout: Identify exact APIs, file paths, function signatures, test patterns
2. Scaffold: Create module/file with stubs
3. Implementation: Fill in one function at a time with tests
4. Integration: Wire into LSP server

Each phase = separate agent with clear verification.

**Skill Update Needed:** Enhance `/scout-then-build` to explicitly separate these phases.

---

### 2.2 Rebase Timing: Only at Merge, Not Speculatively

**Impact:** Reduce CI queue waste, speed up merges
**Effort:** Process change, instruction update
**Evidence:**
- Cycle 5: 6 draft PR fixers rebased speculatively, triggered 6 CI runs that competed with merge CI
- Each rebase = CI run. Rebasing hours before merge is pure waste.
- GitHub merge can handle non-rebased PRs if no conflicts

**How to Apply:**
1. Draft PR fixer agents: Verify tests locally, fix code, but DO NOT rebase unless merge conflicts
2. Merge queue agent: Rebase right before merging (part of merge pipeline)
3. Review agents: Read diff (shows correct changes regardless of base), don't checkout stale branches

**Expected Savings:** ~5-10 min per merge wave (5-15% of CI time).

---

### 2.3 Don't Broadcast Shutdown to Idle Agents

**Impact:** Stop wasting 6% of session context on acknowledgment messages
**Effort:** Process change
**Evidence:**
- Cycle 5 end: Broadcasting shutdown to 117 agents consumed 6% of context window
- Idle agents don't consume context unless woken up
- Shutdown via broadcast generates N acknowledgment messages

**How to Apply:**
- Never broadcast shutdown. Idle agents are free — just stop sending messages.
- Agents will idle naturally; terminate automatically at session end.
- Only explicit shutdown for actively looping agents consuming resources.

**Expected Impact:** Preserves 5-10% context budget per session.

---

### 2.4 Structured Task Lists for Agents (Not Monolithic Prose)

**Impact:** Improve feature agent clarity and debug-ability
**Effort:** Agent prompt template update
**Evidence:**
- Monolithic prose prompts: "Implement X — check this, build that, verify" = 50% success
- Structured task lists with skill invocations: Each step has clear success/failure → 80%+ success
- Agents make design decision in step 3, don't discover failure until step 8

**Implementation:**
Create task-list agent prompt template:
```
1. Invoke /coding-standards
2. Read crates/X/src/Y.rs (understand current API)
3. TaskCreate: "Add module with function Z"
4. Write the module (one file, one function)
5. Invoke /verify <crate>
6. TaskCreate: "Wire module into provider"
7. Edit to call module
8. Invoke /verify <crate>
9. Invoke /pr-create
```

**Benefit:** If step 5 fails, agent knows exactly what broke (step 4's output). Verification after each step catches errors early.

---

### 2.5 Dedup Scout Before Building (10-minute investment saves 40 minutes)

**Impact:** Eliminate wasted builder agents on already-completed work
**Effort:** Add dedup step to orchestrator workflow
**Evidence:**
- Cycle 5: 4 builder agents discovered their work was already done (merged PRs)
- Each wasted builder = 15-20 min agent time + merge queue slot
- Dedup scout: `gh pr list --state merged --search "fixes #N"` takes 30 seconds

**How to Apply:**
1. Before launching builders, run dedup scout
2. Cross-reference every issue against merged PRs
3. Only launch builders for issues with no existing PR
4. Saves 10 min planning, prevents 40 min builder waste

---

### 2.6 Version Drift Detection

**Impact:** Catch release blockers before shipping (all versions were stale in Cycle 5)
**Effort:** 1-2 hours
**Evidence:**
- Cycle 5: Binary output, Cargo.toml, package.json all had stale versions
- Nobody caught it because no CI gate for version consistency
- Version 0.11.0 vs 0.12.0 mismatch

**Implementation:**
1. Add `just version-check` recipe that verifies all Cargo.toml versions match
2. Add to release checklist and pre-push hooks
3. Consider CI gate in policy_checks

**Expected Impact:** Zero shipping bugs from version mismatch.

---

### 2.7 Memory System Consolidation

**Impact:** Reduce context load, improve signal-to-noise
**Effort:** 2-3 hours
**Evidence:**
- 106 memory files accumulated
- MEMORY.md index approaching 200-line truncation limit
- Some memories contradict each other or are outdated
- No expiry mechanism — old advice persists even if superseded

**How to Apply:**
1. Audit all 106 memories: consolidate contradictions
2. Remove outdated memories (Cycle 3 learnings superseded by Cycle 5)
3. Consolidate similar feedback into families (e.g., 5 rebase-related memories → 1)
4. Add expiry dates to time-sensitive advice
5. Re-organize by topic, not chronologically

**Expected Result:** 50-70 focused memories, faster loading, clearer guidance.

---

## 3. MISSING SKILLS (High ROI, Moderate Effort)

### 3.1 `/corpus-ratchet` Skill

**Status:** Mentioned in CLAUDE.md but not implemented as skill
**Use Case:** Post-parser-fix automation
**Effort:** 2-3 hours
**Template:**
```bash
just cpan-corpus-ratchet
# Verify improvement
cargo build --release
# If improved, commit and note results
```

---

### 3.2 `/issue-cleanup` Skill

**Status:** Missing
**Use Case:** Post-merge issue closure
**Effort:** 2-3 hours
**Template:**
```bash
gh issue list --state open --limit 100 | while read issue; do
  pr=$(gh issue view $issue --json body | grep -o "#[0-9]*" | head -1)
  if gh pr view $pr --json state | grep -q '"MERGED"'; then
    gh issue close $issue --reason "Completed by PR $pr"
  fi
done
```

---

### 3.3 `/pr-dedup-scout` Skill

**Status:** Part of /scout but should be standalone
**Use Case:** Before launching builders
**Effort:** 1-2 hours

---

### 3.4 `/verify-tests-match-status` Skill

**Status:** Missing; currently manual check
**Use Case:** Verify test counts match CURRENT_STATUS.md
**Effort:** 1 hour
**Template:**
```bash
actual_count=$(cargo test --lib 2>&1 | grep "test result" | grep -o "[0-9]* passed")
status_count=$(grep "test count" docs/project/CURRENT_STATUS.md)
if [ "$actual_count" != "$status_count" ]; then
  python3 scripts/update-current-status.py --write
fi
```

---

### 3.5 `/merge-queue-aware` Skill

**Status:** Missing
**Use Case:** Check CI queue depth before merge
**Effort:** 2-3 hours

---

## 4. MISSING HOOKS (Enforcement & Automation)

### 4.1 PostToolUse: Auto-Update CURRENT_STATUS.md

**Trigger:** Edit|Write on .rs files that match test patterns
**Action:** Run `python3 scripts/update-current-status.py --write`
**Rationale:** Eliminates policy_checks failures from stale test counts

---

### 4.2 PostToolUse: Close Issues After PR Merge

**Trigger:** `gh pr merge` command
**Action:** Parse `Fixes #N` from PR body, auto-close issues
**Rationale:** Keeps issue tracker in sync with merged PRs

---

### 4.3 PostToolUse: Trigger Corpus Ratchet After Parser Merges

**Trigger:** PR merged with changes to `crates/perl-parser/src/`
**Action:** Run `/corpus-ratchet` skill
**Rationale:** Capture corpus improvements automatically, don't forget

---

### 4.4 PreToolUse: Validate Worktree Isolation

**Trigger:** Bash command with `git checkout` or `git switch`
**Action:** Verify agent is in a worktree (isolation: "worktree")
**Rationale:** Prevent shared-worktree contention

---

### 4.5 PreToolUse: Warn on Speculative Rebase

**Trigger:** Agent rebases branch without immediate merge intention
**Action:** Warn "Rebase detected. Only rebase if PR is next in merge queue."
**Rationale:** Prevent CI queue waste

---

## 5. PROCESS UPDATES NEEDED

### 5.1 Agent Prompt Templates

**Current Gap:** Monolithic prose prompts for features (50% success)
**Needed:** Numbered task lists with skill invocations (80%+ success)

**Action:** Create 3 prompt templates in `.claude/skills/`:
1. `/parser-fix-template` — TDD loop
2. `/feature-template` — Scout → Scaffold → Implement → Wire
3. `/test-template` — Comprehensive test harness

---

### 5.2 Scout-to-Builder Handoff Format

**Current Gap:** Scouts produce prose issues; builders re-investigate
**Needed:** Scouts output structured handoffs with exact file paths, line numbers, APIs

**Action:** Create `/scout-handoff-template` with required fields:
- File paths (with line:column)
- Function signatures
- Test patterns
- Error examples
- Builder constraints

---

### 5.3 Release Checklist

**Current Gap:** Version drift, no pre-release verification
**Needed:** Formal release process

**Action:** Create `.ops-perl-lsp/RELEASE_CHECKLIST.md`:
```
- [ ] All Cargo.toml versions match (run: just version-check)
- [ ] CURRENT_STATUS.md updated (run: python3 scripts/update-current-status.py --check)
- [ ] Changelog updated
- [ ] Core tests pass (cargo test --lib)
- [ ] CPAN corpus at target % (corpus-ratchet done)
- [ ] All open PRs merged or closed
- [ ] CI green on master
```

---

## 6. SCALABILITY & TEAM CEILING ISSUES

### 6.1 Platform Team Roster Ceiling (~75 named teammates)

**Evidence:** Cycle 5 hit platform limit at ~75 concurrent agents
**Impact:** Can't spawn more agents after roster full

**Workarounds (already working):**
1. Use GitHub issues as overflow queue
2. Route completed agents to pick up new issues (repurposing via SendMessage)
3. Reserve 10 agent slots for late-cycle routing

**Mitigation:** Encode this ceiling in documentation. For future swarms:
- Max 75 concurrent agents
- Excess capacity → scouts, planners, reviewers (non-coding agents)
- OR use issues as primary queue, agents pick from there

---

### 6.2 Merge Queue Bottleneck (3-wide limit)

**Evidence:** Optimal concurrent coding agents ≈ 9 for 3-wide queue
**Impact:** Beyond 9 agents, they compete for merge slots

**Solution:** Formalize agent budget:
- Coding agents: 9 max
- Scouts/planners/reviewers: 20-30
- Total: 30-40 agents active, 10+ reserved for overflow

---

## 7. QUALITY RATCHETS MISSING

### 7.1 Corpus Baseline Ratchet ✓ (exists)

**Status:** Implemented, needs post-merge automation

---

### 7.2 Test Count Ratchet (Suggested)

**Rationale:** Test count should only increase, never decrease
**Implementation:** policy_checks: verify `actual_tests >= baseline_tests`

---

### 7.3 Clippy Warning Ratchet

**Rationale:** Merged PRs create clippy failures; should ratchet down
**Implementation:** Add `clippy --all --deny warnings` to CI, track baseline

---

### 7.4 Feature Coverage Ratchet

**Rationale:** Don't lose LSP capabilities
**Implementation:** features.toml: track implemented vs planned, verify no regressions

---

### 7.5 Documentation Coverage Ratchet

**Rationale:** Docs shouldn't regress
**Implementation:** Track % of public APIs documented, fail if < baseline

---

## 8. PRIORITY ROADMAP (Next 3 Sessions)

### Session 1 (Immediate)

**P0 — Blocking current progress:**
1. Fix assert_clean_parse bug (1 hour)
2. Auto-update CURRENT_STATUS.md on test changes (2 hours)
3. Implement `/corpus-ratchet` as post-merge automation (2 hours)
4. Create `/pr-dedup-scout` skill (1 hour)

**Total:** 6 hours. Unblocks all subsequent builders.

---

### Session 2 (Next 2 weeks)

**P1 — High ROI process improvements:**
1. Audit & fix phantom error buckets (2 hours)
2. Add version-check CI gate (1 hour)
3. Worktree isolation enforcement hooks (3 hours)
4. Issue cleanup automation (2 hours)
5. Structured agent task list templates (2 hours)

**Total:** 10 hours.

---

### Session 3 (Next 4 weeks)

**P2 — Infrastructure scaling:**
1. CI queue awareness in merge sequencing (4 hours)
2. Memory system consolidation (2 hours)
3. Missing skill implementations (8 hours total):
   - `/issue-cleanup`
   - `/merge-queue-aware`
   - `/verify-tests-match-status`
4. Release checklist formalization (1 hour)

**Total:** 15 hours.

---

## 9. EXPECTED OUTCOMES

| Improvement | Impact | Timeline |
|---|---|---|
| Auto-update CURRENT_STATUS.md | Eliminates #1 merge blocker | P0 (6h) |
| Corpus ratchet automation | Free 3-4% corpus per merge wave | P0 (2h) |
| Fix assert_clean_parse | Fix false positives in tests | P0 (1h) |
| Constrain feature tasks | 90% success vs 50% | Process (0h) |
| Rebase timing | 5-10 min CI savings per wave | Process (0h) |
| Don't broadcast shutdown | 5-10% context preservation | Process (0h) |
| Structured task lists | 80%+ feature success | P1 (2h) |
| Dedup scout first | Prevent 40 min builder waste | Process (0h) |
| Phantom bucket audit | Honest corpus metrics | P1 (2h) |
| Worktree isolation hooks | Prevent contention bugs | P1 (3h) |
| CI queue awareness | Safe scaling to 50+ agents | P2 (4h) |
| Memory consolidation | Faster context loading | P2 (2h) |

---

## 10. ANTI-PATTERNS TO AVOID

1. **Speculative rebasing** — Only rebase at merge time
2. **Shared worktrees** — Always use `isolation: "worktree"`
3. **Monolithic prose prompts** — Use numbered task lists + skills
4. **Broadcast shutdown** — Let idle agents idle naturally
5. **Manual CURRENT_STATUS.md updates** — Make it automatic
6. **No dedup before building** — Scout for existing PRs first
7. **Phantom error buckets** — Audit classifier before trusting metrics
8. **Test assertions that can't fail** — Verify helpers catch all output formats

---

## CONCLUSION

The swarm has proven 90% success on constrained, well-scoped work. The improvements above address:
- **Process automation** (CURRENT_STATUS, corpus ratchet, issue closure) → 3-5 hours per session saved
- **Quality infrastructure** (test helper bug, phantom buckets, version checking) → Honest metrics, fewer surprises
- **Scaling limits** (merge queue, team roster, worktree contention) → Safe 50+ agent operations
- **Developer experience** (structured prompts, scout-build constrain pattern) → 80%+ feature success

Total effort: ~40 hours across 3 sessions. Projected ROI: 15-20% throughput improvement, 10-20% quality improvement, 2-3% corpus gain.

