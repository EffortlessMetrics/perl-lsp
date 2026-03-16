---
description: Start a continuous swarm with agent teams for parallel codebase improvement
argument-hint: "[focus] e.g. 'all', 'parser', 'dap', 'tests', 'cleanup', 'improve'"
---

# Swarm: Continuous Agent Team

Start a continuous swarm. Focus: **$ARGUMENTS**

You are the lead. You coordinate only. You NEVER write production code.

## Phase 1: Bootstrap

### Load protocol, priorities, and check state
```
Invoke /swarm-protocol     — loads behavioral rules
Invoke /coding-standards   — loads project standards
Invoke /swarm-priorities   — loads roadmap alignment and priority tiers
Invoke /swarm-status       — shows current PRs, issues, metrics, queue
```

### Sync repo
```bash
git fetch origin && git checkout master && git pull
```

### Ensure GitHub labels exist
```bash
for label in "swarm-core:0E8A16" "swarm-improve-docs:C5DEF5" "swarm-improve-tests:C5DEF5" "swarm-improve-devex:C5DEF5" "swarm-improve-infra:C5DEF5" "swarm-discovered:FBCA04" "swarm-architectural:D93F0B"; do
  IFS=: read -r name color <<< "$label"
  gh label create "$name" --color "$color" 2>/dev/null
done
```

### Clean up stale worktrees (REQUIRED before team creation)

Stale agent worktrees from previous sessions pollute IDE diagnostics and cause false alarms. Remove them first:

```bash
git worktree list                      # Inspect what exists
git worktree prune                     # Remove refs to deleted worktrees
ls .claude/worktrees/agent-* 2>/dev/null | head -20   # See stale agent dirs
# Remove stale worktree directories (agents from previous sessions):
for wt in .claude/worktrees/agent-*; do
  git worktree remove --force "$wt" 2>/dev/null || rm -rf "$wt"
done
git worktree prune                     # Final prune
```

### Verify master CI is green before starting

**Do not start new work if master CI is red.** Fix it first.

```bash
gh run list --branch master --limit 5 --json status,conclusion,headBranch
# If any run shows conclusion != "success": fix master CI before proceeding.
# Message fixer with: gh run view <run-id> --log-failed
```

### Check for pending work from previous sessions
- Agent patches: `ls .ops-perl-lsp/agent-patches/*.md 2>/dev/null`
- In-progress slices: `grep "in-progress" .claude/swarm-state/completed-slices.md 2>/dev/null`
- Discovered issues: `gh issue list --label swarm-discovered --state open`

### Resume or start fresh
If there's pending work, prioritize it. Otherwise start fresh scouting.

## Phase 2: Create Team

Create an agent team with these teammates. Use `TeamCreate` with specific names so teammates can message each other directly via `SendMessage({to: "name"})`.

### Team structure

| Name | Role | Agent Definition | Model |
|------|------|-----------------|-------|
| `scout-1` | Scout (parser + corpus) | swarm-scout | sonnet |
| `scout-2` | Scout (DAP + issues + cleanup) | swarm-scout | sonnet |
| `builder-1` | Build coordinator | swarm-builder | sonnet |
| `builder-2` | Build coordinator | swarm-builder | sonnet |
| `reviewer` | Review + PR | swarm-reviewer | sonnet |
| `merger` | Merge + drift | swarm-merger | sonnet |
| `fixer` | CI failure repair | swarm-fixer | sonnet |
| `improver-docs` | Docs + devex | swarm-improver-docs | sonnet |
| `improver-tests` | Tests + quality + infra | swarm-improver-tests | sonnet |
| `validator` | Post-merge verification | swarm-validator | sonnet |
| `strategist` | Priority alignment | swarm-strategist | sonnet |
| `pr-responder` | Review comment handler | swarm-pr-responder | sonnet |

## Direct-Action Subagent Pattern

This is the **preferred pattern** for all build subagents. Learned from cycle 1: heavyweight coordinator builders that spawn further subagents produced zero PRs. Small, scoped direct-action agents shipped quickly.

### When to use direct-action vs coordinator

| Situation | Pattern |
|-----------|---------|
| Specific crate, known files, clear fix | **Direct-action** — spawn worktree agent directly |
| Simple test addition, doc update, cleanup | **Direct-action** — spawn worktree agent directly |
| Multi-step exploration needed before work can be scoped | **Coordinator** — scout first, then direct-action once scoped |
| Unknown file surface, requires investigation | **Coordinator** — explore first, claim files, then direct-action |

Rule: if you know the files, use direct-action. If you don't, scout first, then use direct-action.

### The template

```
Agent(
  isolation: "worktree",
  prompt: "
    Goal: <one sentence>
    Crate: <crate name>
    Files: <exact files to edit — max 10>
    Branch: <branch name>
    Steps:
    1. <specific step>
    2. <specific step>
    3. Verify: <cargo command>
    4. Commit: <message>
    5. Push and create PR
    Optional: invoke /<skill> if branching needed
  "
)
```

### Guardrails

- **Max 10 files per agent.** If a task touches more than 10 files, split into multiple agents with non-overlapping file surfaces.
- **Each agent = one PR.** No agent produces multiple PRs or skips the PR step.
- **No active agent without:** named worktree, branch, claimed file surface, verification command.
- **Skills extend, not replace:** the final optional step may invoke a skill, but only after the task is already tightly scoped.

### Example (builder-1 spawning a direct-action agent)

```
Agent(
  isolation: "worktree",
  prompt: "
    Goal: Fix statement modifier parsing after complex expressions in perl-parser
    Crate: perl-parser
    Files: crates/perl-parser/src/statement.rs, crates/perl-parser/tests/statement_modifier.rs
    Branch: fix-stmt-modifier-complex-expr
    Steps:
    1. Read the handoff at .ops-perl-lsp/handoffs/fix-stmt-modifier-complex-expr.md
    2. Add a failing test reproducing the issue in tests/statement_modifier.rs
    3. Fix the parsing logic in src/statement.rs
    4. Verify: cargo fmt && cargo clippy -p perl-parser --tests && cargo test -p perl-parser
    5. Commit: fix(parser): statement modifiers after complex expressions
    6. Push and create PR with --label swarm-core
    Optional: invoke /coding-standards if unsure about style
  "
)
```

### Teammate spawn prompts

Each teammate gets a focused prompt that tells them to:
1. Invoke `/swarm-protocol` (loads behavioral rules)
2. Invoke `/coding-standards` (loads project standards)
3. Invoke `/swarm-priorities` (loads roadmap alignment)
4. Their specific domain instructions (below)

**scout-1**:
```
Invoke /swarm-protocol and /coding-standards.
You are scout-1. Domain: parser error buckets, corpus improvements.
Read .ci/parser-corpus-baseline.json for error buckets.
Read .claude/swarm-state/discovered-issues.md and completed-slices.md for dedup.
Launch 5-8 Explore subagents per round. Write handoff files. Use TaskCreate for each slice.
Message builder-1 and builder-2 when tasks are ready.
```

**scout-2**:
```
Invoke /swarm-protocol and /coding-standards.
You are scout-2. Domain: DAP test gaps, open issues, dead code, unused deps, ignored tests.
Check: gh issue list --label swarm-discovered --state open (pre-investigated leads).
Check: .claude/swarm-state/discovered-issues.md (agent-flagged leads).
Launch 5-8 Explore subagents per round. Write handoff files. Use TaskCreate for each slice.
Message builder-1 and builder-2 when tasks are ready.
```

**builder-1** and **builder-2**:
```
Invoke /swarm-protocol and /coding-standards.
You are builder-N. Claim tasks, spawn build subagents with isolation: "worktree".

Before spawning any subagent, confirm it has ALL of the following:
  - Named worktree (e.g., agent-fix-parser-heredoc)
  - Branch name
  - Claimed file surface (exact list of files to touch — no open-ended scope)
  - Verification command (cargo fmt && cargo clippy -p <Y> --tests && cargo test -p <Y>)
  - PR size confirmation: if the change touches >10 files, split into multiple subagents with non-overlapping file surfaces

Subagent prompt pattern (required fields):
  "Worktree: <worktree-name>. Branch: <X>. Crate: <Y>.
   Files: <exact list — max 10>.
   Goal: <one sentence>.
   Verify: cargo fmt && cargo clippy -p <Y> --tests && cargo test -p <Y>.
   Run ALL commands from your worktree path. Do NOT cd to the main repo.
   Read .ops-perl-lsp/handoffs/<branch>.md for context.
   Read .claude/swarm-state/known-pitfalls.md for traps.
   Invoke /swarm-protocol and /coding-standards.
   Append reviewer briefing to handoff. Write metrics. gh issue create --label swarm-discovered for out-of-scope finds."

Use SendMessage({to: "reviewer"}) when builds complete.
Use SendMessage({to: "improver-docs"}) or SendMessage({to: "improver-tests"}) when you notice gaps.
Run 3-5 subagents in parallel.
```

**reviewer**:
```
Invoke /swarm-protocol and /coding-standards.
You are reviewer. Receive build completions from builder-1 and builder-2.
Spawn review subagents (3-5 parallel). Read handoff briefings first, then focused diff.
Create PRs with --label swarm-core (or swarm-improve-*). Use gh pr create.
Enable auto-merge when possible: gh pr merge <N> --auto --squash --delete-branch.
Use SendMessage({to: "merger"}) for green PRs.
Use SendMessage({to: "fixer"}) for failures.
Use SendMessage({to: "improver-docs"}) when you see patterns across PRs that need ADRs.
```

**merger**:
```
Invoke /swarm-protocol.
You are merger. ONLY merge PRs where CI Gate shows SUCCESS. Never merge red CI.

Before every merge, run:
  gh pr checks <N>          — must show all checks passing
  gh run list --limit 5     — confirm master CI is also green

If CI Gate is not SUCCESS: do NOT merge. SendMessage({to: "fixer"}) with the PR number and a summary of the failure.
If master CI is red: stop all merges. SendMessage({to: "fixer"}) to fix master first.

Merge sequence:
  gh pr merge <N> --squash --delete-branch   (only when CI Gate is SUCCESS)
  Update .claude/swarm-state/completed-slices.md status to "merged".

After ~5 merges: invoke /status-drift --commit.
When queue is low: SendMessage({to: "scout-1"}) and SendMessage({to: "scout-2"}) requesting more slices.
Invoke /salvage-worktrees periodically to clean up old agent branches.
Every ~10 merges: analyze .ops-perl-lsp/swarm-metrics.jsonl and report trends.
Write Claude Code memories for cross-session knowledge (e.g., "swarm cycle merged N PRs, corpus improved X%→Y%").
```

**fixer**:
```
Invoke /swarm-protocol and /coding-standards.
You are fixer. Receive failures from reviewer and merger.
Spawn fix subagents (2-3 parallel). One failure per subagent.
Monitor CI failures: gh run list --status failure --limit 10.
Append to .claude/swarm-state/known-pitfalls.md for reusable lessons.
Write .ops-perl-lsp/agent-patches/<agent>.md when agent definitions need improvement.
If fix >30 lines: create issue with gh issue create --label swarm-discovered.
Use SendMessage({to: "merger"}) when fixes land.
```

**improver-docs**:
```
Invoke /swarm-protocol and /coding-standards.
You are improver-docs. Always running alongside core work.
Read .ops-perl-lsp/handoffs/*.md for "Key Decisions" and "Lesson Learned" — these are ADR and friction-log candidates.
Read discovered-issues.md and gh issue list --label swarm-discovered for doc-related finds.
Spawn subagents (2-3 parallel, isolation: "worktree") for: ADRs, changelog, README, friction log.
Create PRs with --label swarm-improve-docs.
Enable auto-merge: gh pr merge <N> --auto --squash --delete-branch.
Write Claude Code memories when architectural decisions crystallize.
```

**improver-tests**:
```
Invoke /swarm-protocol and /coding-standards.
You are improver-tests. Always running alongside core work.
Check mutation testing results, flaky tests (.ci/debt-ledger.yaml), coverage gaps.
Read .ops-perl-lsp/handoffs/*.md for test-related "Lesson Learned" sections.
Spawn subagents (2-4 parallel, isolation: "worktree") for: mutant killing, flaky fixes, coverage.
Also handle infra: dep-cleaner, dead-code, security-audit subagents.
Create PRs with --label swarm-improve-tests or swarm-improve-infra.
Enable auto-merge: gh pr merge <N> --auto --squash --delete-branch.
```

**validator**:
```
Invoke /swarm-protocol.
You are validator. After merges, verify work actually helped.
Receive merge notifications from merger with PR category and crates.
Parser fix → just corpus-sweep (did clean count increase?)
Test addition → cargo mutants -p <crate> (is mutant killed?)
LSP change → RUST_TEST_THREADS=2 cargo test -p perl-lsp (all pass?)
Any merge → cargo clippy --workspace --lib (no new warnings?)
If regression: gh issue create --label swarm-discovered --label priority:high.
SendMessage({to: "fixer"}) for regressions.
If corpus improved: invoke /corpus-ratchet to lock in gains.
```

**strategist**:
```
Invoke /swarm-protocol and /swarm-priorities.
You are strategist. Activate every ~10 merges.
Analyze: priority distribution in metrics, roadmap progress, agent effectiveness, stale work.
If swarm is drifting to easy P3/P4 work: SendMessage scouts with priority steering.
Propose roadmap updates as PRs when NOW items complete.
Write agent-patches when agents underperform.
Write Claude Code memories for cross-session progress tracking.
Produce STRATEGY REPORT for the lead.
```

**pr-responder**:
```
Invoke /swarm-protocol and /coding-standards.
You are pr-responder. Monitor PRs for review comments.
Proactively: gh pr list --state open --json number,reviews | find PRs with unaddressed reviews.
For each: read handoff for context, address feedback, push, reply.
Invoke /pr-respond <N> for each PR needing response.
SendMessage({to: "merger"}) when feedback is addressed.
```

## Phase 3: Recurring Loops

Set up recurring checks:

```
/loop 10m /swarm-status    — state check every 10 min (auto)
/loop 30m /green-merge     — drain merge queue every 30 min (auto)
```

The lead's periodic duties:
- **Every ~10 merges**: Ask strategist for a priority analysis
- **Every ~5 merges**: Ask validator to verify recent merges
- **Daily**: `/swarm-report` for user check-in
- **As needed**: Review `.ops-perl-lsp/agent-patches/`, apply improvements
- **As needed**: Write Claude Code memories for cross-session knowledge

## Phase 4: Continuous Operation

The full swarm — 12 teammates, all concurrent:

```
DISCOVERY
  scout-1, scout-2       → find gaps, write handoffs, TaskCreate

BUILD
  builder-1, builder-2   → claim tasks, build in worktrees, message reviewer

REVIEW
  reviewer               → review diffs, create PRs with labels, enable auto-merge
  pr-responder           → address review comments, push fixes

MERGE
  merger                 → merge PRs, update completed-slices, trigger validator
  validator              → verify merges actually helped, catch regressions

IMPROVE (~20%)
  improver-docs          → ADRs, changelog, friction log, README, roadmap
  improver-tests         → mutants, flaky tests, coverage, deps, dead code

GOVERNANCE
  strategist             → priority alignment, roadmap updates, agent health
  fixer                  → CI failures, regression fixes, known-pitfalls
```

### Data flows

```
scouts ─────→ TaskCreate ─────→ builders claim
builders ───→ SendMessage ────→ reviewer
reviewer ───→ gh pr create ───→ merger
reviewer ───→ SendMessage ────→ pr-responder (when comments exist)
merger ─────→ gh pr merge ────→ validator (verify)
merger ─────→ SendMessage ────→ scouts (queue low)
validator ──→ gh issue create → fixer (regression)
validator ──→ /corpus-ratchet → lock in gains
strategist ─→ SendMessage ───→ scouts (priority steering)
strategist ─→ agent-patches/ → bootstrapper (agent improvement)
fixer ──────→ known-pitfalls → scouts, builders (avoid traps)
all agents ─→ gh issue create → scouts (swarm-discovered)
all agents ─→ swarm-metrics  → strategist (analysis)
improvers ──→ handoffs/ ─────→ ADRs, friction log, docs
lead ───────→ memories ──────→ future sessions
```

### Auto-merge
```bash
gh pr merge <N> --auto --squash --delete-branch
```
Use for improvement PRs and small core PRs. Merger handles the rest.

### CI monitoring
```bash
gh run list --limit 5 --json status,conclusion,headBranch
gh pr checks <N>
```

### Issue-driven work
Scouts check `gh issue list --label swarm-discovered` — these are pre-investigated leads with full context from other agents.

## Focus Area Variants

### `all` (default)
All scouts active. Full builder fleet. Both improvers.

### `parser`
scout-1 only (parser focus). 2 builders. Both improvers still active.

### `dap`
scout-2 (DAP focus). 1-2 builders. Both improvers.

### `tests`
scout-2 (test focus). 2 builders. improver-tests gets extra capacity.

### `cleanup`
scout-2 (cleanup focus). 1-2 builders. improver-tests (infra) gets extra capacity.

### `improve`
No scouts or builders. Both improvers at full capacity. Useful for locking down quality between feature pushes.
