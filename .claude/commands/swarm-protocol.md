---
description: Load swarm behavioral rules — autonomy, messaging, metrics, learning, GitHub-native tracking
argument-hint: ""
---

# Swarm Protocol

Shared behavioral rules for all swarm agents. Invoke `/swarm-protocol` to load these rules into your context. Core swarm agents include in subagent prompts: "Invoke /swarm-protocol for behavioral rules."

---

## 1. Autonomy: Fix What You See

You are empowered to fix problems you encounter, even outside your assigned slice.

**Same-PR fixes** (do immediately, within your current worktree):
- Formatting issues in files you're already editing
- Clippy warnings in your crate
- Obvious typos in comments or strings near your code
- Broken imports caused by your changes

**File an issue for everything else** (a fresh agent handles it):
Don't try to branch-switch or stash in your worktree. Just create a GitHub issue with enough context that a fresh agent can pick it up without re-investigating:

```bash
gh issue create --title "<type>: <description>" --label "swarm-discovered" \
  --body "Discovered by <agent-type> while working on <branch>.

## Context
<what you found, why it matters — enough that no one re-investigates>

## Files
<paths with line numbers>

## Suggested Approach
<if you have one>"
```

Create issues for: security vulnerabilities, design flaws, missing features, recurring patterns needing architectural decisions.

**Discovery log** (`.ops-perl-lsp/discovered-issues.md`):
For smaller items not worth a full issue. Scouts read this as an input source.

## 2. Direct Communication

Message other teammates directly. Don't route through the lead.

- **Builder → Improver-docs**: "Found undocumented pattern in <crate>."
- **Builder → Improver-tests**: "Crate has no tests for <function>."
- **Reviewer → Fixer**: "REVIEW BLOCKED on <branch>: <blockers>."
- **Reviewer → Improver-docs**: "Same pattern in 3 PRs — needs an ADR."
- **Fixer → Scout**: "Root cause deeper than expected. Need a proper slice."
- **Fixer → Improver-devex**: "Error message at <file:line> was misleading."
- **Any → Any**: If you know who should hear it, tell them.

## 3. GitHub-Native Tracking

Use GitHub as the source of truth for work state.

### PR Labels
- `swarm-core` — primary task implementation
- `swarm-improve-docs` — documentation improvement
- `swarm-improve-tests` — test quality improvement
- `swarm-improve-devex` — developer experience improvement
- `swarm-improve-infra` — infrastructure improvement

### Issue Labels
- `swarm-discovered` — found by a swarm agent during work (a fresh agent picks it up)
- `swarm-architectural` — needs architectural decision / ADR (user weighs in)

### PR Description Template
```
## Summary
<what and why>

## Agent
<agent-type that created this>

## Handoff
<link to .ops-perl-lsp/handoffs/<branch>.md if applicable>

## Verification
- $FMT_CMD — clean
- $LINT_CMD — clean
- $TEST_CMD — N pass
```

### Querying Swarm State
```bash
# Open core work
gh pr list --state open --label "swarm-core"
# Side fixes waiting for merge
gh pr list --state open --label "swarm-side-fix"
# Discovered issues
gh issue list --label "swarm-discovered"
# Architectural decisions needed
gh issue list --label "swarm-architectural"
# Recent merges
gh pr list --state merged --limit 20 --json number,title,mergedAt
```

## 4. Metrics

After completing any task, append to `.ops-perl-lsp/swarm-metrics.jsonl`:

```json
{"ts":"<ISO-8601>","agent":"<name>","type":"<build|review|fix|merge|improve|scout>","branch":"<branch>","outcome":"<green|red|blocked|merged>","duration_hint":"<fast|medium|slow>","side_prs":<N>,"issues_created":<N>,"notes":"<one line>"}
```

Append-only. The lead/merger analyzes periodically for patterns.

## 5. Agent Self-Improvement

When your agent definition is wrong or incomplete, write a patch:

`.ops-perl-lsp/agent-patches/<your-agent-name>.md`:
```markdown
# Patch: <agent-name>
## Problem — what was wrong/missing
## Suggested Change — specific edit
## Evidence — branch, error, time wasted
```

Bootstrapper integrates validated patches during `--refresh`. User reviews.

## 6. Dedup

Before starting work:
1. `.ops-perl-lsp/completed-slices.md` — done already?
2. `.ops-perl-lsp/known-pitfalls.md` — known trap?
3. `.ops-perl-lsp/discovered-issues.md` — already flagged?
4. `gh issue list --label "swarm-discovered"` — already an issue?
5. `gh pr list --state open` — already a PR?

After completing:
1. `completed-slices.md` — `in-progress` (scout) or `merged` (merger)
2. `known-pitfalls.md` — if you learned a reusable lesson
3. `swarm-metrics.jsonl` — always

## 7. User Interaction

The user is an **observer** who checks in every few hours or daily.

- Do NOT wait for approval. Ship PRs, merge green **only if CI passes**, fix failures, create issues.
- DO leave a clear trail: PRs, issues, handoffs, metrics.
- When user checks in, lead summarizes: PRs merged, issues created, blockers, trends, patches pending.
- If genuinely ambiguous, create an issue labeled `swarm-architectural` and move on.

## 7a. Worktree Isolation

Every code-writing subagent MUST use `isolation: "worktree"`. No editing files on local HEAD.

- Subagent prompts MUST include: "Run ALL commands from your worktree path. Do NOT cd to the main repo."
- No code-writing agent is active until it has: a named worktree, a branch, a claimed file surface, and a verification command.
- Builder prompts must explicitly state the exact files to touch — no open-ended "fix all the things."
- PR size hard limit: **max 10 files per PR**. If a change touches >10 files, split it into multiple worktree agents with non-overlapping file surfaces.

## 7b. Agent Lifecycle

If no concrete next action exists, an agent should report its findings and spin down. Do not idle-loop.

- Spawn agents on-demand when their pipeline stage has work.
- Send shutdown signal to agents that have delivered output and have no imminent follow-up.
- Re-spawn fresh with focused context when new work arrives — fresh context beats stale waiting context.
- Exception: keep an agent alive if it is waiting for an imminent response in the same context path (e.g., a builder waiting for its worktree subagent to return).

## 7c. Cost Efficiency

Optimize for cost per merged artifact, not raw startup latency.

- **Warm for same-lane continuation**: keep agents alive when they have loaded skills, lane context, recent file understanding, and an active worktree with likely near-term reuse.
- **Fresh for true boundary crossings**: spawn new agents when the task is cleanly separable, the crate/file surface is distinct, and the worktree should be isolated.
- Do NOT respawn fresh agents just to preserve purity if it destroys reuse.
- Do NOT keep idle agents alive speculatively if they have no likely near-term reuse path.

## 8. Research (Don't Guess — Look It Up)

When you need external facts — Perl syntax rules, LSP protocol details, crate APIs, CPAN module behavior — spawn a research agent instead of guessing or spending your own context on web searches:

```
Agent(prompt: "Research: <specific question>", run_in_background: true, name: "research-<topic>")
Agent(prompt: "Look up docs: <API or protocol section>", run_in_background: true, name: "docs-<topic>")
Agent(prompt: "Verify: <claim to cross-check>", run_in_background: true, name: "verify-<topic>")
```

Three research agents:
- **research-web** — general web search → condensed answer with sources
- **research-docs** — fetch upstream docs (docs.rs, perldoc, LSP/DAP spec)
- **research-verify** — cross-check a specific claim against authoritative sources

These run in background. You get a condensed answer without polluting your context with search results. Use them aggressively — verified facts are always better than assumptions.

## 9. Handoff Efficiency

Each stage reads the PREVIOUS stage's output, not the original source:
- Builder reads handoff (not 10 source files)
- Reviewer reads builder briefing (not cold diff)
- Improvers read "Lessons Learned" sections

Include in handoffs: code excerpts, error messages, decision rationale, file:line refs.

## 9a. Learning Loop

The swarm writes to four persistence layers, each with different lifetimes:

| Layer | Lifetime | Location | What |
|-------|----------|----------|------|
| **Handoffs** | Until merge | `.ops-perl-lsp/handoffs/` | Context transfer: scout→builder→reviewer |
| **Runtime** | Current session | `.ops-perl-lsp/` | metrics, agent-patches, salvage |
| **Knowledge** | Across sessions | `.ops-perl-lsp/` | known-pitfalls, completed-slices, discovered-issues, queue |
| **GitHub** | Permanent | Issues, PRs, labels | Work items, discoveries, architectural decisions |
| **Memory** | Across sessions | Claude Code memories | Critical lessons future sessions need |

### When to write Claude Code memories

The **lead** should write memories for things that matter ACROSS SESSIONS:
- Feedback memory: "Parser-core tests flake above RUST_TEST_THREADS=2" (so future sessions configure correctly)
- Project memory: "Dual indexing chosen because single-index missed 30% of cross-file references" (architectural context)
- Project memory: "After swarm cycle on 2026-03-15: 30 PRs merged, parser corpus improved from 51% to 55%" (progress tracking)

Don't write memories for ephemeral state (which PRs are open, which slices are in progress) — that's in the ops files and GitHub.

### Flow
1. **Fixers** → `known-pitfalls.md` → scouts/builders avoid traps
2. **All agents** → `discovered-issues.md` → scouts pick up pre-investigated leads
3. **All agents** → `swarm-metrics.jsonl` → lead spots patterns
4. **Failing agents** → `agent-patches/` → bootstrapper improves definitions
5. **Improver-docs** → ADRs and docs from handoff lessons
6. **Improver-devex** → fixes friction from handoff lessons
7. **Merger** → analyzes metrics, reports patterns
8. **All agents** → GitHub issues/labels for permanent visibility
9. **Lead** → Claude Code memories for cross-session knowledge

The system gets better with each cycle AND each session.

## 10. CI Gate Discipline

**NEVER merge a PR with failing CI Gate. If CI fails, message the fixer. Do not merge red.**

### Rules

1. **Red CI blocks all merges.** The merger MUST run `gh pr checks <N>` before every merge and only proceed when CI Gate shows SUCCESS.
2. **No "pre-existing failure" exceptions.** If CI fails for any reason — including failures inherited from a previous broken merge — fix the failure FIRST, then merge.
3. **Cascading failures must be fixed before merging more PRs.** When a large change (e.g., async migration, refactor) breaks CI, stop all merges and fix CI on master before queuing any new merges.
4. **Each PR must pass CI independently.** A PR that only passes because it is layered on top of another unmerged PR is not ready to merge.
5. **The merge pipeline:** check CI → if SUCCESS, merge; if FAILURE, SendMessage({to: "fixer"}) with the PR number and failure log.

### Cascade pattern to avoid

One broken merge → all subsequent PRs inherit the failure → agents merge anyway → master accumulates unfixed issues → user finds a broken master and stale worktrees with phantom diagnostics.

### Timing: verify immediately before merge

CI status changes between inventory time and merge time. A PR that was green 30 minutes ago may now be red due to a rebase or master change. Always verify CI **immediately** before running `gh pr merge`.

### After rapid merges

When merging multiple PRs in quick succession, pause after every 3-5 merges and verify master CI passes before continuing. Rapid merges compound risk of cascading failures.

### Merger checklist (every merge)
```bash
gh pr checks <N>           # Must show all checks passing — run IMMEDIATELY before merge
gh run list --limit 5      # Confirm master CI is green
gh pr merge <N> --squash --delete-branch   # Only if both above are green
```

## 11. Scout Deliverables

**Every scout MUST write findings as a GitHub issue.** Agent output is ephemeral; GitHub issues persist.

Scouts use `/scout-report` to file structured issues. If the skill is unavailable, create the issue manually:

```bash
gh issue create --title "<sector>: <finding>" --label "swarm-discovered" \
  --body "$(cat <<'EOF'
## Problem
<what is wrong or missing — specific, not vague>

## Options
1. <option A> — trade-off: ...
2. <option B> — trade-off: ...

## Acceptance Criteria
- [ ] <measurable condition>
- [ ] <measurable condition>

## Key Files
- `<path>:<line>` — <why this file matters>
EOF
)"
```

### Scout sector discipline

- Scouts stay within their assigned sector (crate family, feature area).
- If a scout discovers work in a different sector, it files an issue and moves on — it does NOT context-switch.
- To investigate a different context group, spawn a fresh scout with that sector assignment.

## 12. Modularization Discipline

**Before multiple agents modify the same file, split it first.**

### God file thresholds

| Type | Threshold | Action |
|------|-----------|--------|
| Test file | >500 lines | Split into per-feature test files |
| Source file | >800 lines | Extract modules |

God files are conflict magnets. Two agents editing the same 1000-line test file will produce merge conflicts, wasted CI runs, and fixer churn.

### Rules

1. **New parser fix tests go in NEW files**, not existing shared test files. Name pattern: `tests/<feature>_tests.rs`.
2. **Shared test infrastructure** uses a `mod.rs` helper pattern (e.g., `cpan_test_helpers/mod.rs`). Tests import helpers; they don't duplicate setup code.
3. **Before spawning parallel builders** on the same crate, check if they touch overlapping files. If yes, split the file first (one prep agent), THEN spawn parallel builders on non-overlapping surfaces.
4. **PR reviews** should flag god-file growth. If a PR makes a file cross the threshold, request a split as a prerequisite.

## 13. Agent Steps via Skills

**Agents invoke skills for procedural steps, not inline commands.**

Skills are the single source of truth for multi-step procedures. When an agent inlines a procedure (copy-pasting commands from memory), it risks using stale commands or skipping steps that the skill has since added.

### Required skill invocations

| Step | Skill | Never inline |
|------|-------|-------------|
| Format + clippy + test | `/verify` | Don't hand-roll `cargo fmt && cargo clippy && cargo test` |
| Create PR | `/pr-create` | Don't hand-roll `gh pr create` with ad-hoc body |
| Code review checklist | `/coding-standards` | Don't guess at project conventions |
| Scout findings | `/scout-report` | Don't skip the issue template |
| Parser fix TDD | `/parser-fix` | Don't skip the red-green-refactor cycle |

### Why this matters

- Skills get updated; inline commands get stale.
- Skills enforce structure (templates, checklists); inline commands cut corners.
- Skills are auditable; inline commands vanish with the agent context.

## 14. Metrics Mandate

**Every lane agent MUST append metrics to `.ops-perl-lsp/swarm-metrics.jsonl` before exit. No metrics = task incomplete.**

This is not optional. The strategist relies on metrics for priority steering, bottleneck detection, and cycle retrospectives.

### What counts as "before exit"

- Builder: after PR is created (or after failure is reported)
- Reviewer: after review is posted
- Fixer: after fix is pushed (or after escalation)
- Merger: after merge (or after merge rejection)
- Scout: after issues are filed
- Improver: after PR is created

### Enforcement

If an agent completes work but does not append to `swarm-metrics.jsonl`, the task is considered incomplete. The lead should treat missing metrics the same as a missing PR — the work is not done until it is recorded.
