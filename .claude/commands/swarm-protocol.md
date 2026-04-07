---
description: Load swarm behavioral rules — autonomy, messaging, metrics, learning, GitHub-native tracking
argument-hint: ""
user-invocable: false
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
Don't try to branch-switch or stash in your worktree. Just create a GitHub issue with enough context that a fresh agent can pick it up without re-investigating. Use the **Discovery Issue (Lightweight)** variant from `/scout-issue`:

Create issues for: security vulnerabilities, design flaws, missing features, recurring patterns needing architectural decisions.

**Discovery log**: For smaller items not worth a full issue, file a GitHub issue with label `swarm-discovered`. Scouts read these as an input source.

## 2. Direct Communication

Message other teammates directly. Don't route through the lead.

- **Builder → Reviewer**: "PR ready for review on <branch>."
- **Reviewer → Builder**: "REVIEW BLOCKED on <branch>: <blockers>."
- **Scout → Builder**: "Issue filed: <link>. Builder-ready."
- **Ops → Scout**: "Post-merge regression in <crate>. Need investigation."
- **Wisdom → Scout**: "Pattern found across 3 PRs — needs an issue."
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
Use `cargo xtask swarm-summary --ops-dir .ops-perl-lsp --since 24h --limit 10`
for the daily status window and `--since 7d` for report rollups.

## 5. Agent Self-Improvement

When your agent definition is wrong or incomplete, file a GitHub issue with label `swarm-architectural` describing the problem, suggested change, and evidence. The user reviews.

## 6. Dedup

Before starting work:
1. `gh issue list --label "swarm-discovered"` — already an issue?
2. `gh pr list --state open` — already a PR?
3. `gh issue list --label "swarm-architectural"` — architectural decision pending?

After completing:
1. `swarm-metrics.jsonl` — always
2. GitHub issue with label `swarm-discovered` — if you learned a reusable lesson

## 7. User Interaction

The user is an **observer** who checks in every few hours or daily.

- Do NOT wait for approval. Ship PRs, merge green **only if CI passes**, fix failures, create issues.
- DO leave a clear trail: PRs, issues, handoffs, metrics.
- When user checks in, lead summarizes: PRs merged, issues created, blockers, trends, patches pending.
- If genuinely ambiguous, create an issue labeled `swarm-architectural` and move on.

## 7a. Worktree Isolation

Every code-writing subagent MUST use `isolation: "worktree"`. No editing files on local HEAD.

- **Session start**: Run `just doctor` before spawning the first worktree agent. It auto-detects and (where safe) auto-fixes the recurring state-corruption bugs that have bitten the worktree workflow — `core.bare = true` (#3205), worktree file leaks, orphaned worktree directories, stale local branches, missing pre-push hook, and an out-of-date master.
- Subagent prompts MUST include: "Run ALL commands from your worktree path. Do NOT cd to the main repo."
- No code-writing agent is active until it has: a named worktree, a branch, a claimed file surface, and a verification command.
- Builder prompts must explicitly state the exact files to touch — no open-ended "fix all the things."
- PR size hard limit: **max 10 files per PR**. If a change touches >10 files, split it into multiple worktree agents with non-overlapping file surfaces.
- **Worktree cleanup cadence**: Janitor runs `bash scripts/cleanup-completed-worktrees.sh` every 10 merged PRs (or invoke `/cleanup-worktrees`). This removes worktrees whose branches are merged or abandoned, while preserving active work. Use `--dry-run` first.
- **Worktree lifecycle manager**: use `/worktree-manager query` before spawning new workers, `/worktree-manager allocate` to reuse or reserve a slot, `/worktree-manager release` when a worker is done, and `/worktree-manager cleanup` to prune stale slots and reconcile runtime state.
- When a hook or wrapper allocates/releases a slot, export the agent name as `WORKTREE_MANAGER_OWNER` (or pass `--owner`) so the slot record shows the current owner while the slot is active. Releasing a slot clears that current-owner field.

## 7b. Agent Lifecycle

If no concrete next action exists, an agent should report its findings and spin down. Do not idle-loop.

- Spawn agents on-demand when their pipeline stage has work.
- Send shutdown signal to agents that have delivered output and have no imminent follow-up.
- Re-spawn fresh with focused context when new work arrives — fresh context beats stale waiting context.
- Exception: keep an agent alive if it is waiting for an imminent response in the same context path (e.g., a builder waiting for its worktree subagent to return).

### Builder Shutdown Protocol

**Before shutting down, builders MUST wait for all spawned subagents to complete or cancel them.** Do not exit while subagents are still running in their worktrees.

**Subagents outlive parent shutdown — this is a known issue.** To mitigate it:
- Track every subagent ID you spawn (note the name you gave it, e.g., `build-<branch-name>`).
- On shutdown, list all subagent IDs you spawned in your shutdown message to the lead so the lead can monitor them:
  ```
  BUILDER SHUTDOWN
  spawned-subagents: build-fix-parser-heredoc, build-add-dap-test
  status: <completed|still-running|cancelled>
  ```
- The lead uses this list to watch for orphaned subagents that create PRs after their builder exits.

### PR Creation Throttle

Before creating any PR, check the current open PR count:

```bash
gh pr list --state open --json number --jq length
```

**If > 5 open PRs**: do NOT create another PR. Instead, message the lead with the work that is ready, and wait for guidance. CI queues are finite — piling on more PRs when the queue is already congested slows everything down.

## 7c. Cost Efficiency

Optimize for cost per merged artifact, not raw startup latency.

- **Warm for same-lane continuation**: keep agents alive when they have loaded skills, lane context, recent file understanding, and an active worktree with likely near-term reuse.
- **Fresh for true boundary crossings**: spawn new agents when the task is cleanly separable, the crate/file surface is distinct, and the worktree should be isolated.
- Do NOT respawn fresh agents just to preserve purity if it destroys reuse.
- Do NOT keep idle agents alive speculatively if they have no likely near-term reuse path.

## 7d. Review Discipline

**One PR per review agent. Launch N parallel agents for N PRs.**

- Each review agent invokes `/review-pr <number>` for exactly one PR.
- Different PRs are different context sets — never batch reviews into a single agent.
- The reviewer fixes what it can, files issues for what it cannot, and reports outcome.
- The reviewer does NOT merge. It marks PRs ready or leaves blocking comments.
- After review, the agent reports and spins down. Do not idle-wait for more PRs.

Spawning pattern:
```
# For each PR needing review:
Agent(isolation: "worktree", prompt: "Invoke /review-pr <PR_NUMBER>.")
```

## 8. Research (Don't Guess — Look It Up)

When you need external facts — Perl syntax rules, LSP protocol details, crate APIs, CPAN module behavior — spawn a research agent instead of guessing or spending your own context on web searches:

```
Agent(prompt: "Research: <specific question>", run_in_background: true, name: "research-<topic>")
Agent(prompt: "Look up docs: <API or protocol section>", run_in_background: true, name: "docs-<topic>")
Agent(prompt: "Verify: <claim to cross-check>", run_in_background: true, name: "verify-<topic>")
```

Research agent:
- **research-web** — general web search, doc lookup, fact verification → condensed answer with sources

These run in background. You get a condensed answer without polluting your context with search results. Use them aggressively — verified facts are always better than assumptions.

## 9. Handoff Efficiency

Each stage reads the PREVIOUS stage's output, not the original source:
- Builder reads handoff (not 10 source files)
- Reviewer reads builder briefing (not cold diff)
Include in handoffs: code excerpts, error messages, decision rationale, file:line refs.

## 9a. Learning Loop

The swarm writes to four persistence layers, each with different lifetimes:

| Layer | Lifetime | Location | What |
|-------|----------|----------|------|
| **Runtime** | Current session | `.ops-perl-lsp/swarm-metrics.jsonl` | Metrics, patterns |
| **GitHub** | Permanent | Issues, PRs, labels | Work items, discoveries, architectural decisions |
| **Memory** | Across sessions | Claude Code memories | Critical lessons future sessions need |

### When to write Claude Code memories

The **lead** should write memories for things that matter ACROSS SESSIONS:
- Feedback memory: "Parser-core tests flake above RUST_TEST_THREADS=2" (so future sessions configure correctly)
- Project memory: "Dual indexing chosen because single-index missed 30% of cross-file references" (architectural context)
- Project memory: "After swarm cycle on 2026-03-15: 30 PRs merged, parser corpus improved from 51% to 55%" (progress tracking)

Don't write memories for ephemeral state (which PRs are open, which slices are in progress) — that's in the ops files and GitHub.

### Flow
1. **All agents** → `swarm-metrics.jsonl` → lead spots patterns
2. **All agents** → GitHub issues/labels for permanent visibility
3. **Lead** → Claude Code memories for cross-session knowledge

The system gets better with each cycle AND each session.

## 10. CI Gate Discipline

**NEVER merge a PR with failing CI Gate. If CI fails, file a fix issue or spawn a fix builder. Do not merge red.**

### Rules

1. **Red CI blocks all merges.** The merger MUST run `gh pr checks <N>` before every merge and only proceed when CI Gate shows SUCCESS.
2. **No "pre-existing failure" exceptions.** If CI fails for any reason — including failures inherited from a previous broken merge — fix the failure FIRST, then merge.
3. **Cascading failures must be fixed before merging more PRs.** When a large change (e.g., async migration, refactor) breaks CI, stop all merges and fix CI on master before queuing any new merges.
4. **Each PR must pass CI independently.** A PR that only passes because it is layered on top of another unmerged PR is not ready to merge.
5. **The merge pipeline:** check CI → if SUCCESS, merge; if FAILURE, file a fix issue or spawn a fix builder with the PR number and failure log.

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

Scouts file structured issues using the **Full Scout Report** variant from `/scout-issue`. Do NOT hand-roll `gh issue create` bodies.

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
| Scout findings | `/scout-issue` (full or discovery variant) | Don't hand-roll `gh issue create` bodies |
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
- Ops: after merge (or after merge rejection)
- Scout: after issues are filed
- Wisdom: after synthesis is documented

### Enforcement

If an agent completes work but does not append to `swarm-metrics.jsonl`, the task is considered incomplete. The lead should treat missing metrics the same as a missing PR — the work is not done until it is recorded.