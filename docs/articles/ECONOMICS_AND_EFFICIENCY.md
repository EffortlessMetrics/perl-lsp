# Economics and Efficiency of Agentic Development

## Session Benchmark: Era 7 Session 4 (2026-03-22)

| Metric | Value |
|--------|-------|
| Agents spawned | 246 |
| PRs created | 49 |
| PRs merged (this session) | 30 |
| Issues closed | 17+ |
| Critical bugs found & fixed | 7+ |
| System corpus improvement | 85.7% → 90.9% (ratcheted) + active fixes in queue |
| Weekly quota delta | +15 points (50% → 65%) |
| Session window consumed | 88% of 5h |

## Capacity Framing

The key benchmark is not the absolute weekly usage gauge, but the **change during the session**.

This session began at roughly **50% of weekly quota** and ended around **65%**, so it consumed about **15 percentage points of weekly plan capacity**. Over the same period it used about **88% of the current 5-hour session window**.

Those are different meters:

- **Weekly quota** is the broader capacity ceiling.
- **Session-window usage** is the local throttle on one working block.

Public reporting should keep them separate. "65% weekly" is a gauge reading. "+15% weekly this session" is the session cost.

## Two Scarcity Surfaces, Not One Cost Meter

The primary benchmark is **live quota share + output**, not backward-looking dollar logs.

### 1. Live Quota Share

Two different meters, never conflated:

- **Weekly %** — how much of the standard weekly Max 20x quota was consumed
- **Five-hour %** — how much of the current session bucket was consumed

Due to Anthropic's March 2026 off-peak promotion, the five-hour session budget is doubled during off-peak weekday hours, but the weekly cap is NOT doubled. These are separate scarcity surfaces.

### 2. What To Report Publicly (Four Things)

| Dimension | What to show |
|-----------|-------------|
| **Live quota share** | Weekly start/end/delta + five-hour % at named checkpoints |
| **Output** | Agents spawned, PRs created/merged, issues closed, corpus delta |
| **Scope exclusions** | Excludes Copilot CLI, Codex CLI, CI spend, other machines |
| **Promotion context** | Whether during March off-peak bonus window |

### 3. What NOT To Report

The March ccusage export is the wrong anchor. It mixes older work, other projects, broken export methodology, and none of the Codex/Copilot/CI side of the stack.

Avoid:
- "March spend was $684, therefore this session cost X"
- "Weekly is probably doubled right now" (only 5h doubles, not weekly)
- "This was exactly $50-70" (faux precision from polluted data)
- Treating live plan UI and backfilled export logs as the same instrument

## What Makes It Efficient

### 1. Plan-review kills bad work early

Every scout spec in this session was corrected by plan-review. Corrections included:

- Wrong file references (stale line numbers, renamed modules)
- Wrong root causes (symptoms misidentified as bugs)
- Already-fixed issues (17+ closed without building)
- Missing dependencies (crates not in Cargo.toml)
- Wrong architectural assumptions (duplicating existing infrastructure)
- Fabricated sub-patterns (scout's "8 gaps" was actually 2 real defects)

A haiku-tier plan-review pass costs a fraction of a sonnet-tier builder. Catching a wrong spec before a builder spends 30 minutes is the highest-ROI stage.

### 2. Deep review catches real bugs cheaply

Every deep review in this session found and fixed real issues:

| PR | Bug Found |
|----|-----------|
| #2728 | `sortText` field never serialized — feature silently inert |
| #2733 | `set -euo pipefail` + grep = exit 1 instead of fail-open 0 |
| #2103 | Semantic token legend desync — every token wrong in all clients |
| #2740 | Telemetry dedup + Cancelled exclusion + cleanup |
| #2736 | CRLF Unicode surrogate pair edge cases |
| #2737 | Latent UTF-8 panic on French/German text |
| #2769 | Heredoc guard too broad — regresses `print $fh <<END` |
| #2743 | URI normalization mismatch in semantic token cache |
| #2738 | NodeKind::Class not handled in type hierarchy |
| #2744 | Vacuous test assertions + trailing whitespace in hover card |
| #2761 | Docs PR silently carrying a full feature revert |

### 3. Queue compression is productive work

Closing issues produces real value:

| Outcome | Count | Why It Matters |
|---------|-------|----------------|
| Already-fixed | 8+ | Prevents builders from re-implementing existing features |
| Stale/invalid | 5+ | Removes false signals from backlog searches |
| Respecified | 5+ | Converts ambiguous issues into buildable specs |

The codebase was consistently more mature than the backlog describing it.

### 4. Microcrates + worktrees = safe parallelism

The 133-crate workspace means 246 agents can work simultaneously without merge conflicts. Each agent gets a git worktree (isolated copy of the repo) and works on a different microcrate.

### 5. Cache-read economics

94.5% cache-read tokens (from earlier export data) means the system is paying for context once and reusing it across agents. Each new agent reads from cache rather than regenerating understanding.

## What Limits Throughput

### What IS the bottleneck

1. **Weekly plan limit** — the real budget cap (+15 points for 246 agents and 49 PRs)
2. **CI queue** — merges need individual verification; rebasing after merges creates churn
3. **Control engineering** — stage ownership, label state, receipt freshness, worktree safety
4. **Git config lock contention** — at ~250 concurrent worktrees, lock contention becomes frequent
5. **Session time limit** — secondary constraint (88% consumed)

### What is NOT the bottleneck

- Model capability (agents are sufficient for their assigned tasks)
- Code generation speed (agents write code faster than reviewers can verify it)
- Individual agent quality (mediocre outputs get corrected by the pipeline)

## The Core Insight

> The pipeline is more valuable than the agents inside it.

The architecture absorbs error. Mediocre individual outputs get corrected. Strong outputs get sharpened. The system produces trusted change because verification is layered, not because generation is perfect.

> Even a modest slice of plan quota can produce a surprising amount of trusted output when the pipeline is right.

## Transferable Patterns

### "Built but not wired" is the highest-ROI discovery

This session found 17 unwired crates with 6,566 lines of code and 51 tests sitting unused. The wiring cost was minimal. This pattern is not specific to this project — it is a general AI-native development story.

### Queue-wide unlocks dominate local fixes

One semantic token legend fix made every token render correctly in all clients. One diagnostic test fix unblocked CI across 20+ PR branches. These global unlocks are worth more than ten local feature PRs.

### Classification before implementation

The `unexpected_token_in_expr` bucket (92 files) decomposed into 4 concrete sub-patterns with exact file:line fix locations. Without classification, a builder would have tried to fix "the bucket" and failed.

### Every deep review found a bug

Not "sometimes." Every single one. That makes the two-pass review (standards + deep) structural, not optional.

## Process Lessons

### Orchestrator must never merge directly

The orchestrator running `gh pr merge` directly bypasses all safety: no worktree isolation, no CI verification, no sequential green checks. Each merge needs its own ops agent on its own worktree processing one PR.

### One PR, one agent, one worktree, done

No batching PRs in a single agent. No keeping agents alive across PRs. No swapping worktrees. Each agent touches exactly one PR on exactly one worktree and then terminates. Fresh context = clean state.

### Spec freshness matters

Many failures in this session were freshness failures: already-fixed issues, stale root causes, stale file references, stale branch bases. A spec is not true forever. Verify against current master before building.
