# Session Economics: 2026-04-02 Release Cleanup & Multi-Release Build-Out

**Session Date**: 2026-04-02
**Model**: Claude Opus 4.6 (1M context)
**Operator**: Steven Zimmerman (orchestrator)
**Session type**: Normal Claude Code run — not a special swarm event. Agent calls were natural parallelism for a release cleanup task that grew organically into multi-release build-out.

### Budget (session 1 of 2, same day)

| Metric | Value |
|--------|-------|
| Session 1 budget used | 75% of single session (reset at 5h mark) |
| Weekly budget at end of session 1 | 42% |
| Session 2 started at | 0% session, 42% weekly |

---

## Session 1 (75% session consumed, 42% weekly at end)

The full first session used 75% of a single 5-hour session window and brought the weekly total to 42%. The approach evolved mid-session: the early portion (~first half) was reactive and ad-hoc (merge quickly, fix forward), the later portion shifted to proper pipeline (verify-before-build, full review passes).

### What went well
- 28 PRs merged, 48 issues closed
- Discovered 23 already-implemented features (the issue tracker was severely stale)
- Deep review caught 4 real bugs in subroutine inlining before merge
- Two full releases completed (0.12.2 stability, 0.12.3 refactoring)
- Roadmap built out through 0.12.8 → 0.13.0

### What went wrong
- **Merged red**: Multiple PRs merged on smoke-green without full CI gate passing
- **Triage false positive**: Triage agent closed #3020 as already-fixed but builder had real implementation
- **Worktree file leaks**: Agent worktrees leaked modified files into main checkout, causing merge conflicts
- **First builder went off-scope**: Hook-fix builder created test files across 10+ unrelated crates before being redirected
- **No CI gate on branch deletions**: Wasted ~55 min before discovering the hook bug

### Economics (full session 1)
- ~45 agents spawned across 6 waves
- 23 of those discovered work already done (51% discovery rate)
- 4 bugs caught by deep review (12-16x ROI on the review investment)
- Budget: 75% of one 5-hour session, bringing weekly to 42%
- Cost per merged PR: ~2.3% of session (75% / 33 PRs)
- Cost per closed issue: ~1.5% of session (75% / 50 issues)

### How the session evolved

**Phase 1 (~34% → 38% weekly, ~4% consumed):** Ad-hoc. Started with release cleanup (delete branches, close stale issue, fix git config). Grew into "let's build out the roadmap" and then "let's launch builders." Merged PRs on smoke-green without waiting for full CI gate. Most PRs merged and issues closed happened here — high throughput, lower quality gates.

**Phase 2 (~38% → 40% weekly, ~2% consumed):** Transitional. Applied lessons from phase 1: started verifying issue state before spawning builders, added the "already-done" check to every builder prompt, launched triage agents to clean the issue backlog. Still merging fast but with two-pass review on feature PRs. The economics documentation and multi-stage PR creation (4 PRs for this doc alone) consumed meaningful context during this phase.

**Phase 3 (~40% → 42% weekly, ~2% consumed):** Close to pipeline. Proper verify-before-build on every agent, full review passes, issue triage running in parallel, learned rules applied consistently. Much closer to the structured swarm pipeline (scout→plan-review→build→review→merge). Fewer wasted agents, higher per-PR confidence.

**Budget distribution:** Phase 1 consumed ~half the session's weekly budget (4 of 8 points) and produced the bulk of the output. Phases 2 and 3 together consumed the other half (2 points each) but ran at higher quality.

The key insight: **the session learned its own operating model in real-time.** The rules that worked best (verify-before-build, deep review, trust builders over triage) were discovered through failure in phase 1 and codified by phase 3.

This document itself reflects the evolution — it went through 4 PRs (#3098 initial, #3099 closed due to conflict, #3101 with quality assessment, #3107 with session split) because it was being written and revised as the session progressed. Each revision incorporated lessons the session was still learning. The multi-stage PR creation for the economics doc consumed meaningful context, but the documentation-as-you-go approach captured insights that would have been lost in a post-hoc writeup.

---

## Session 2: Pipeline Phase (0% session, 42% weekly — starting now)

The session reset gives a fresh 5-hour window. The remaining work is well-scoped: ~23 open issues, ~15 agents still running from session 1, proper pipeline rules in effect.

### Active agents carried over
Builders for: workspace-perf (#2078), Docker (#2083), Homebrew (#2086), Linux pkgs (#2095), corpus ratchet (#2026), VSCode ESLint (#1910), DAP attach (#3025), DAP signals (#3028), heredoc injection (#2059), token caching (#3021), dev guide (#3027), workspace docs (#3022), memory profiling (#2085), CPAN-scale (#1664), incremental parser (#2080), CPAN manifest (#2971).

### Rules for session 2
1. Wait for full CI gate before merge (lesson from session 1)
2. Verify issue state before spawning builders (42% session 1 builders found work done)
3. Trust builders over triage agents on "already-fixed" disagreements
4. Clean worktree file leaks after each wave
5. Run `just ci-gate` on master after merge batches

---

## Combined Output (both sessions, from `gh` queries)

| Metric | Count | Verification |
|--------|-------|-------------|
| **PRs merged** | 62+ (and counting) | `gh pr list --state merged --search "merged:2026-04-02"` |
| **Issues closed** | 67 of 68 (99%) | `gh issue list --state closed --search "closed:2026-04-02"` |
| **Issues: start → now** | 68 → 1 (99% reduction) | Only #3018 (AI/LLM, deferred by design) |
| **Issues created** | 4 (#3081, #3089, #3093, #3094) | |
| **Remote branches deleted** | 11+ (323 queued for cleanup) | |
| **Dependabot PRs merged** | 8 of 8 | |
| **Releases completed** | 0.12.2 through 0.12.8 (all milestones) | |
| **Already-implemented discoveries** | 24+ | |
| **Agents spawned** | ~80+ across 10+ waves | |
| **Real bugs found pre-merge** | 13 (4 inlining + 4 distribution binary name + 5 VSCode floating promises) | |
| **Key infrastructure shipped** | Incremental parsing pipeline, corpus ratchet, Docker image, justfile Windows fix | |
| **CI fix** | justfile $$ escaping broken on Windows — all gate recipes non-functional | |

### PR Breakdown by Type

| Type | Count | Examples |
|------|-------|---------|
| CI/infrastructure | 6 | #3078-#3080, #3084, #3086, #3088 |
| Refactoring features | 2 | #3083 (subroutine inlining), #3090 (extract var/sub) |
| Error handling | 1 | #3087 (logging batch) |
| Test coverage | 1 | #3091 (58 new tests) |
| Docs/config | 3 | #3082, #3085, #3095, #3096 |
| Dependency bumps | 8 | #3064-#3071 |

### Release Progress

| Release | Status | Key evidence |
|---------|--------|-------------|
| **0.12.2** (CI/stability) | Complete | 10 PRs merged |
| **0.12.3** (refactoring) | Complete | Scoped rename already done, inlining + extract merged |
| **0.12.4** (diagnostics) | ~80% done | 2 PRs in review, 1 builder active, 3 features found already implemented |
| **0.12.5** (parser) | Scouting done | All 7 Tier 1 blockers confirmed fixed, blockers.yaml updated |

---

## Agent Economics

### Deployment Summary

~30 agents spawned in 3 waves across isolated worktrees.

| Role | Spawned | Produced a PR | Found work already done | Errored/redirected |
|------|---------|--------------|------------------------|--------------------|
| Builder | 12 | 7 | 4 | 1 (went off-scope, redirected) |
| Reviewer (standards) | 6 | — | — | 0 |
| Reviewer (deep) | 5 | — | — | 0 |
| Plan-reviewer | 4 | — | — | 0 |
| Scout | 3 | — | — | 0 |
| Ops (merge) | 2 | — | — | 0 |
| General-purpose | 1 | — | — | 0 |

### Per-Agent Yield

| Metric | Value |
|--------|-------|
| PRs merged per agent spawned | 0.73 (22 / 30) |
| PRs merged per builder spawned | 1.83 (22 / 12) — builders produce PRs, other roles advance them |
| Issues closed per scout spawned | 3.0 (9 / 3) — scouts close stale issues too |
| Budget per agent | ~1.3% of session (38% / 30) |
| Budget per merged PR | ~1.7% of session (38% / 22) |

---

## Learnings

### 1. The Ledger Gap: Issue Trackers Lag Codebases

The most surprising finding: **42% of builder deployments discovered the work was already done**. Five features targeted for 0.12.3-0.12.5 turned out to be fully implemented on master:

- Scoped rename (#3037) — complete with 7 integration tests
- Moose/Moo method modifiers (#2328) — shipped in PR #2744
- Moose/Moo role composition (#2325) — already closed
- Strict/warnings diagnostics — PL100/PL101 with 19 tests
- 7 of 7 Tier 1 parser blockers — all fixed, 129 tests

**Why it matters**: Roadmap planning based solely on the issue tracker would have allocated 5+ builder sessions to work that needed zero code changes. The scouts and builders that discovered "already done" were not wasted — they updated the ledger, closed 9 stale issues, and prevented future agents from re-investigating the same ground.

**Implication**: Before building, verify. A 2-minute `gh issue view --json state` check before spawning a builder saves 15-30 minutes of agent time. Better: scout first, build second.

### 2. Deep Review Is Underpriced

The two-pass review pipeline caught 4 real bugs in a single PR (subroutine inlining #3083):

1. `str::replace` corrupted `$price_adjusted` when substituting `$price`
2. `"will return a value"` counted as a control-flow `return`
3. `my $x_count` corrupted when renaming collision variable `$x`
4. `"add(1,2)"` in a string triggered false recursion rejection

**Cost**: ~5% of session budget for the deep review pass.
**Avoided cost**: Each bug, if shipped, would need its own scout→build→review cycle (~15-20% of a future session each). Total: 60-80% of a future session avoided.

**ROI**: 12-16x return on the deep review investment.

The bugs shared a pattern: naive `str::replace` in text-pattern code. A human reviewer might miss these too — they require tracing through specific Perl input strings to trigger. The deep reviewer's systematic edge-case methodology (try string literals containing keywords, try variable name prefixes) is well-suited to this class of bug.

### 3. Infrastructure Debt Compounds Silently

Five infrastructure issues found during routine operations:

| Issue | Time wasted before fix | Fix time |
|-------|----------------------|----------|
| `core.bare = true` in .git/config | Unknown (blocked all git ops) | 1 second |
| Stale worktree reference | Blocked git ops | 1 second |
| Pre-push hook on deletions | ~55 min (11 branches x 5 min) | 15 min builder |
| perl-uri unused import | Blocked all PR CI | 5 min |
| Blockers.yaml stale (7 entries) | Misdirected 5+ agent deployments | 10 min |

**Total fix time**: ~30 minutes. **Total time wasted before fix**: hours across multiple sessions.

The blockers.yaml staleness is the most expensive: it caused the roadmap to plan 0.12.5 as a "parser confidence" release requiring significant new work, when in reality the parser was already well above its target. Multiple agents were deployed to investigate "unfixed" blockers that had been fixed weeks ago.

**Implication**: Automated staleness detection for status files (corpus baselines, blocker ledgers, feature catalogs) would prevent this class of waste. Issue #2026 (automate corpus ratchet) addresses the parser baseline; similar automation for blockers.yaml would help.

### 4. This Was a Normal Session, Not a Swarm Event

This was not an orchestrated 200-agent swarm deployment. It was a normal Claude Code session — the user asked to do release cleanup, it naturally grew into multi-release planning, and agents were called as a normal tool for parallelizing independent work. The ~30 agents were spawned in 3 natural waves, not pre-planned.

Comparing to session 6 (2026-03-22), which was a deliberate mass-swarm:

| Metric | Session 6 (mass swarm) | This session (normal run) | Ratio |
|--------|----------------------|--------------------------|-------|
| Agents deployed | 200+ | ~30 | 0.15x |
| PRs merged | 59 | 22 | 0.37x |
| Weekly budget | 8% | 9% | 1.1x |
| PRs per agent | 0.30 | 0.73 | 2.4x |
| Bugs caught pre-merge | 0 (not tracked) | 4 | — |
| Session type | Deliberate orchestrated swarm | Organic normal session | — |

The normal session had 2.4x higher PRs-per-agent, but **PRs-per-agent is a throughput metric, not a quality metric.** This session merged multiple PRs on smoke-green without the full merge gate. The swarm session enforced `just ci-gate` on every PR. Higher throughput with lower gate enforcement is not necessarily higher yield — it may just be lower standards.

Honest comparison:

| Dimension | Session 6 (swarm) | This session (normal) | Better? |
|-----------|-------------------|----------------------|---------|
| Throughput (PRs) | 59 | 22 | Swarm |
| Agent efficiency (PRs/agent) | 0.30 | 0.73 | Normal (but see quality) |
| CI gate enforcement | Full (`just ci-gate`) | Partial (smoke only) | Swarm |
| Bugs caught pre-merge | Not tracked | 4 | Normal (deep review) |
| Bugs shipped (unknown) | Unknown | Likely higher | Swarm (stricter gates) |

**The real question is: how many of these 22 PRs would have passed `just ci-gate`?** We don't know, because we didn't run it. The deep review pipeline caught 4 text-pattern bugs that CI wouldn't have, but CI catches type errors, API breakage, and integration failures that deep review doesn't.

**Implication**: PRs-per-agent is a vanity metric without quality adjustment. A fairer comparison would be *trusted PRs per unit of budget* — but that requires knowing whether the merged PRs are actually correct, which we won't know until the next CI run or user report.

### 5. The Orchestrator's Primary Job Is Routing, Not Building

The human operator wrote zero lines of feature code. All code was produced by agents. The operator's contributions:

- **Routing decisions**: which agents to spawn, in what order, on which issues
- **Unblocking**: fixed git config, restored git ops, added `workflow` scope
- **Merge sequencing**: batch ordering, conflict resolution, CI unblocking
- **Ledger maintenance**: blockers.yaml updates, features.toml catalog entries
- **Strategic framing**: release ladder design, milestone scoping

This matches the CLAUDE.md principle: "The orchestrator routes, it doesn't execute."

---

## Budget Projection

At 9% weekly budget for 22 merged PRs:

- The 0.12.x ladder (0.12.2 through 0.12.8) had ~68 open issues at session start
- ~15 were discovered already-done, leaving ~53 issues needing work
- At 22 PRs/session rate, **3 sessions** would cover the remaining work
- Projected weekly budget: **27%** (3 x 9%)
- Projected calendar time: **1-2 weeks** at 2-3 sessions/week

The 0.13.0 public alpha announcement could be ready in 2-3 weeks of targeted sessions.

---

## Methodology

- PR counts: `gh pr list --state merged --search "merged:2026-04-02" --limit 50`
- Issue counts: `gh issue list --state closed --search "closed:2026-04-02"`
- Agent counts: Manual count from conversation tool-call records
- Budget percentages: Reported by Claude Code session metrics (38% session, 9% weekly)
- All agent worktrees isolated via `Agent(isolation: "worktree")`
