# 10 Failure Stories from Building a Perl LSP with AI Agents

*The most instructive moments in the perl-lsp project were not the successes. They were the failures — the reverts, the phantom metrics, the security scares that turned out to be nothing, and the scaling walls that nobody predicted. Each of these stories encodes a lesson that changed how the project operates.*

---

## 1. Premature Optimization Reverts

### What Happened

During cycle 2 (March 2026), several agents were tasked with performance optimization work — reducing `.clone()` calls, implementing LRU caches, and introducing incremental parsing shortcuts. The PRs looked clean, passed tests, and merged.

Then regressions appeared. The LRU cache in the completion provider introduced stale results when files were edited rapidly. The `.clone()` elimination in the symbol table caused a use-after-free-equivalent (a borrow checker error on master that wasn't visible in the isolated worktree). Two PRs were reverted within hours of merging.

### The Lesson

Performance optimization PRs are the riskiest category for AI agents because:

1. They require understanding data flow across module boundaries — which agents in isolated worktrees cannot see.
2. The test suite validates correctness, not performance semantics like cache invalidation timing.
3. Benchmarks existed (Criterion) but had no regression baseline tracking — so there was no automated way to verify the optimization actually helped.

Issue #2091 was created to add benchmark regression detection. The rule became: **no performance PR merges without before/after benchmark data**.

### Data

- 2 reverted performance PRs out of 38 merged in cycle 4 session 1
- Cost: ~2 hours of debugging + revert + CI pipeline
- Fix: benchmark regression baselines (issue #2091, estimated 4h to implement)

---

## 2. Security Ratchet: 13 Vulnerabilities That Kept Coming Back

### What Happened

A security audit in cycle 5 (March 19, 2026) was launched expecting to find problems. Instead, the scout found enterprise-grade security posture: 3-layer path traversal prevention in `crates/perl-path-security/`, frame size limits in the DAP server, no `unsafe` in any security-critical path.

But the story behind that posture is less reassuring. Across cycles 2-4, the same categories of vulnerability kept appearing in new code:

- Path traversal in file-handling utilities (fixed 3 times in different crates)
- Unbounded input in regex parsing (fixed twice — budget guard `MAX_REGEX_BYTES = 64KB` in `crates/perl-lexer/`)
- Missing input validation in the DAP debug adapter (fixed in `crates/perl-dap/`)

Each individual fix was correct. But new agents writing new code in new crates didn't know about the security patterns established in other crates. The same class of vulnerability kept being introduced because agents don't inherit institutional memory about security patterns.

### The Lesson

Security is defense-in-depth, not fix-once. The solution was:

1. Extract security primitives into dedicated microcrates (`perl-path-security`, `perl-dap-security`)
2. Make the safe API the easy API — importing the microcrate gives you the safe version
3. Run security audits once per release cycle, not every session

The security scout confirmed these investments paid off: zero findings in the cycle 5 audit. But the path to zero was 13 separate vulnerability fixes across 4 cycles.

### Data

- 13 vulnerability instances across cycles 2-4
- 3 path traversal, 2 unbounded input, 8 missing validation
- 3 security microcrates extracted
- 0 findings in cycle 5 audit

---

## 3. The 54 Archived Agent Definitions

### What Happened

The `.claude/agents/` directory accumulated 54 agent definition files over cycles 1-4. Each was carefully written with a name, role description, tool permissions, and behavioral guidance. They represented dozens of hours of prompt engineering.

None of them were used.

In cycle 5's agent skill mix analysis, the finding was stark: the orchestrator writes inline prompts for every agent, because inline prompts can include the specific function name, line number, and fix approach discovered by a scout. Static agent definitions are generic ("you are a parser fix agent") while inline prompts are specific ("fix `consume_use_import_value` at `declarations.rs:952` where the parser fails on `use if $condition, 'Module'`").

The three actual agent patterns used were:
1. `Agent(subagent_type="Explore")` — for research/scouting
2. `Agent(isolation="worktree", mode="bypassPermissions")` — for code changes
3. `Agent(subagent_type="general-purpose")` — for GitHub operations

Everything else was ceremony.

### The Lesson

Agent definitions are the wrong abstraction level. What works is:
- **Prompt templates** — lightweight fill-in-the-blank patterns that the orchestrator completes with scout findings
- **Skills** — reusable procedures that agents invoke (`/verify-build`, `/corpus-ratchet`)
- **Hooks** — enforcement rules that fire automatically

The 54 definitions were archived. Three templates replaced them (builder, scout, reviewer). The lesson: **agents orchestrate, skills execute, hooks enforce**.

### Data

- 54 agent definitions written, 0 used in cycle 5
- 3 actual patterns replace all 54
- 47 commands in `.claude/commands/` — similarly most unused
- Referenced in: `feedback_agent_skill_mix.md`

---

## 4. Phantom Corpus Bucket: 83 Ghost Files

### What Happened

The CPAN corpus error classification system uses "semantic buckets" — substring matching rules in `xtask/src/tasks/parser_corpus_sweep.rs:33-84` that map raw parser error strings to human-readable category names.

During the cycle 5 scout analysis of error buckets, bucket #5 (`unexpected_rbrace_expr`: 83 files) was flagged for investigation. A scout traced the error emission path and discovered that **no parser code actually generates the exact error string** that the bucket's substring match was looking for.

The 83 files were real failures. But they were being misclassified — their actual errors matched a different pattern that happened to contain the substring `rbrace`. The bucket was a **classification artifact**, not a parser bug category. Fixing the "right" bucket wouldn't fix these files, because the bucket was pointing at a phantom.

The critical revision came after deeper investigation: the bucket was in fact real, just renamed. The scout's initial "phantom" hypothesis was wrong — but the investigation revealed that the SEMANTIC_BUCKETS mapping had drifted from the actual parser error strings over time, creating a gap between what the metrics said and what was true.

### The Lesson

Metrics can lie. The corpus classification system had accumulated technical debt:
- Bucket names didn't match parser error strings
- First-match-wins substring matching meant order-dependent classification
- No validation that bucket names correspond to actual error patterns

The fix: audit SEMANTIC_BUCKETS against actual parser error strings (`ParseError::unexpected()` in `crates/perl-parser-core/src/engine/parser/expressions/primary.rs:752`). Remove phantom buckets. Add validation that every bucket maps to a real error.

### Data

- 83 files in phantom bucket
- 457 files across buckets #4-9 (10.5% of corpus)
- SEMANTIC_BUCKETS defined at `xtask/src/tasks/parser_corpus_sweep.rs:33-84`
- Error generation at `crates/perl-parser-core/src/engine/parser/expressions/primary.rs:752`
- Issue #2189 created for investigation

---

## 5. Stale PR Branches: Correct Diff, Wrong Code

### What Happened

During cycle 5 review, three PRs (#1940, #1939, #1938) passed diff review — the changes shown in the GitHub diff view were correct and desirable. But when a review agent checked out the branches and tried to build them, the branches contained **completely different code** than expected.

The branches had drifted from master. Incorrect rebases had introduced unrelated changes, or the wrong branch had been pushed. The GitHub diff view showed only the delta, which looked correct. But the full branch state was broken.

### The Lesson

PR diffs can show correct changes while the branch is broken. Review agents must:

1. **Checkout and build**, not just read diffs
2. Verify the branch is based on recent master
3. Check for unrelated files in the branch diff

This is especially insidious for automated reviews: the diff-based review passes, CI might even pass on the branch, but the merge commit introduces unintended changes from the stale base.

The worktree drift problem (`feedback_worktree_base_drift.md`) is a systemic issue: agents create worktrees, do work, and the worktree's base commit drifts from master as other PRs merge. The longer a PR sits unmerged, the more likely its branch state diverges from what the diff shows.

### Data

- 3 PRs with correct diffs but broken branches (cycle 5)
- Root cause: worktree base drift from master
- Fix: review agents must checkout + build, not just read diffs
- Referenced in: `feedback_worktree_base_drift.md`

---

## 6. policy_checks: The Systemic Merge Blocker

### What Happened

Every PR that adds tests to the codebase changes the test count. The test count is recorded in `docs/project/CURRENT_STATUS.md`, which is generated by `scripts/update-current-status.py`. The `policy_checks` CI gate verifies that CURRENT_STATUS.md matches the actual test counts.

This means: every PR that adds tests fails CI unless the agent also regenerates CURRENT_STATUS.md. In cycle 5, 4 out of 5 PRs in the first review batch were blocked by this gate. It was the #1 source of merge friction.

### The Lesson

Computed documentation that is checked into the repo creates a coupling between unrelated changes. An agent working on parser fixes shouldn't need to know about the status documentation pipeline. Three possible fixes were identified:

1. Run `update-current-status.py` as part of the merge queue workflow (automation)
2. Make `policy_checks` advisory-only — warning, not failure (relaxation)
3. Have a bot auto-update CURRENT_STATUS.md on merge (automation)

The interim fix was adding the status update to the `/verify-build` skill so agents run it automatically. But the fundamental tension remains: coupling test work to documentation work creates unnecessary friction.

### Data

- 4/5 PRs blocked in review-batch-1 (cycle 5)
- #1 merge friction point across cycles 4-5
- Fix applied to `/verify-build` skill
- Root: `docs/project/CURRENT_STATUS.md` computed by `scripts/update-current-status.py`

---

## 7. Version String Invisibility

### What Happened

For the entire 0.12.0 release preparation, every version string in the repo said `0.11.0`. The binary output, all `Cargo.toml` files, `package.json` for the VSCode extension — all stale. Nobody caught it because version verification wasn't in any CI gate.

The version bump PR (#2035) was created in cycle 5, but the discovery highlighted a systematic gap: the project had no automation to verify version consistency across the workspace's 130+ Cargo.toml files.

### The Lesson

Anything not checked by CI will drift. Version strings are a classic example of "obviously someone checks this" assumptions. The fix:

1. A `just version-check` recipe that verifies all Cargo.toml versions match
2. Adding version verification to the release checklist
3. Considering a CI check for version consistency

The broader pattern: AI agents are excellent at writing code but poor at maintaining cross-cutting invariants that span the entire workspace. Version strings, license headers, dependency versions, and documentation links all tend to drift because no single agent owns them.

### Data

- All 130+ Cargo.toml files showed 0.11.0 during 0.12.0 prep
- PR #2035 created for version bump
- No CI gate existed for version consistency
- Referenced in: `feedback_cycle5_learnings.md` item 4

---

## 8. Diminishing Returns Above 50 Agents

### What Happened

Cycle 4 deployed approximately 100 concurrent agents — the largest swarm session. The scaling dynamics revealed a clear inflection point:

```
Agents 1-8:    Triage (highest ROI — prevented hours of conflict)
Agents 9-20:   Merge improvements (high ROI)
Agents 21-35:  Research scouts (very high ROI — root cause discovery)
Agents 36-50:  Targeted builders (high ROI — exact fixes from scout findings)
Agents 51-75:  Test coverage (medium ROI — fills gaps but not critical)
Agents 76-100: Diminishing returns (low ROI — many crates already well-tested)
```

The last 50 agents were test coverage additions to crates that already had adequate coverage. They produced valid PRs, but the marginal value was low. Meanwhile, they consumed merge queue capacity (3 PRs per CI cycle) and review attention.

### The Lesson

The merge queue is the bottleneck, not agent throughput. With a 3-wide merge queue and ~5 minute CI cycles, the maximum throughput is approximately 36 PRs per hour. Producing 100 PRs per hour with 100 agents just creates a backlog.

The optimal formula: `merge_queue_width x agent_work_time / merge_cycle_time = ~9 concurrent coding agents`. The remaining capacity should go to scouts, reviewers, and planners that don't generate PRs.

Cycle 5 confirmed this: 75 agents hit the platform team roster ceiling. The platform limit forced the discipline that the math already suggested.

### Data

- ~100 agents deployed in cycle 4
- Optimal coding agents: ~9 (merge queue math)
- Platform ceiling: ~75 named teammates
- Referenced in: `feedback_merge_queue_is_bottleneck.md`, `feedback_100_agent_session.md`

---

## 9. Duplicate Bug Reveals Better Architecture

### What Happened

Two agents independently fixed the same bug — prototype mode parsing in the Perl parser. PR #1903 applied a targeted patch: a conditional check in the specific function that was failing. PR #1906 introduced a proper `after_sub` state machine in the lexer that handled prototype mode transitions systematically.

Both PRs fixed the test case. Both passed CI. During review, the comparison revealed that #1906 was architecturally superior — it handled not just the reported case but an entire class of related failures. The state machine approach was inherently more robust.

### The Lesson

Running two agents on the same bug is not waste — it's parallel solution space exploration. The "cost" is one agent's compute time (~15 minutes). The benefit is discovering the superior approach that a single agent might not have found.

The pattern holds generally: constrained tasks (parser fixes with clear test cases) produce ~90% success rates. When two agents both succeed, the review comparison reveals which approach generalizes better. This is analogous to genetic algorithms: diversity in solutions leads to better outcomes.

The rule: don't prevent duplication when the area is not extremely narrow. Let both run, compare during review, keep the better solution, close the other with a note explaining why.

### Data

- PR #1903: targeted patch (simpler, narrower fix)
- PR #1906: state machine approach (broader, more robust)
- #1906 merged, #1903 closed
- Agent success rate on constrained tasks: ~90%
- Referenced in: `feedback_duplicate_agent_discovery.md`

---

## 10. False Security Alarm from Bot

### What Happened

During cycle 3 (late January 2026), `google-labs-jules[bot]` authored approximately 210 draft-PR commits between January 16 and January 30. The initial response from the team was alarm — were these unauthorized? Was the bot creating security risks?

Investigation revealed the commits were legitimate draft PRs from an authorized integration. But the surrounding merged history was full of follow-up work: Steven/Bolt/Sentinel/Palette supersedes, reverts, and selective merges. The bot's output required significant human curation.

The deeper lesson wasn't about security — it was about the **false alarm cost**. The time spent investigating the bot's provenance could have been spent reviewing its output. And the pattern of investigation-before-review became a template: when encountering unexpected agent output, check authorization first, then quality.

### The Lesson

External batch tools (Codex, Jules, other bots) produce output that looks alarming at first glance — many PRs, unfamiliar branch names, automated commit messages. The instinct to investigate provenance is correct, but it should be bounded:

1. Check authorization (5 minutes max)
2. If authorized, switch to triage mode
3. Cluster duplicate PRs (Codex generates 2-5 near-duplicates per topic)
4. Pick the best from each cluster, incorporate unique ideas from rest, close duplicates

Cycle 4 formalized this: after any external batch tool run, the FIRST action is triage, not investigation. The Codex cleanup pattern — cluster, compare, keep-best, close-rest — was codified as a potential `/triage-prs` skill.

### Data

- ~210 draft-PR commits from jules[bot] in January 2026
- Codex duplicates: 2-5 near-identical PRs per topic
- Cycle 4 triaged 50+ stale PRs in first 20 minutes using cluster pattern
- 55 duplicates closed, 38 merged from the clean set
- Referenced in: `feedback_codex_cleanup_as_session_start.md`, `feedback_codex_duplicate_prs.md`

---

## Common Threads

These 10 stories share patterns:

1. **Metrics can lie**: Phantom buckets, stale baselines, correct diffs on broken branches. Trust computed evidence, not cached snapshots.

2. **Enforcement beats suggestion**: Hooks enforce, prompts suggest. Version checks need CI gates. Security needs extracted microcrates.

3. **Scaling has cliffs**: 50 agents is the inflection point. 75 is the platform ceiling. The merge queue is 3-wide. Optimize for merge throughput, not agent count.

4. **Duplication has value**: Two agents on the same bug reveals architecture. Codex duplicates reveal the best approach per cluster. Parallel exploration beats serial exploration.

5. **Institutional memory doesn't transfer**: Security patterns, version conventions, documentation coupling — agents don't inherit knowledge from other agents. The solution is infrastructure (microcrates, hooks, CI gates), not documentation.
