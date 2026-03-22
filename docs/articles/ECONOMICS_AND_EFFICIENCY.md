# Economics and Efficiency of Agentic Development

## The Headline Numbers

Era 7 Session 4 (2026-03-22):

| Metric | Value |
|--------|-------|
| Agents spawned | 110+ |
| PRs created | 23 |
| PRs merged | 3 |
| Issues closed | 11+ |
| Plan-reviews completed | 20+ |
| Critical bugs found & fixed | 5 |
| System corpus improvement | 85.7% → 93.7% |
| Weekly plan usage | ~7% |
| Session time usage | ~27% of 5h limit |

The full morning (multiple sessions) consumed roughly 20% of a Claude Max 20x weekly plan and moved a few hundred PRs through the pipeline.

## Two Scarcity Surfaces, Not One Cost Meter

The primary benchmark is **live quota share + output**, not backward-looking dollar logs.

### 1. Live Quota Share

Two different meters, never conflated:

- **Weekly %** — how much of the standard weekly Max 20x quota was consumed
- **Five-hour %** — how much of the current session bucket was consumed

Due to Anthropic's March 2026 off-peak promotion, the five-hour session budget is doubled during off-peak weekday hours, but the weekly cap is NOT doubled. These are separate scarcity surfaces.

At checkpoint (100 agents spawned):
- **~7% of weekly plan** — 100+ agents, 23 PRs, corpus 85.7% → 93.7%
- **~27% of five-hour window** — same checkpoint

By session end: 125+ agents, 40+ PRs merged.

### 2. What To Report Publicly (Four Things)

| Dimension | What to show |
|-----------|-------------|
| **Live quota share** | Weekly % + five-hour % at named checkpoints |
| **Output** | Agents spawned, PRs created/merged, issues closed, corpus delta |
| **Scope exclusions** | Excludes Copilot CLI, Codex CLI, CI spend, other machines |
| **Promotion context** | Whether during March off-peak bonus window |

### 3. What NOT To Report
| Opus 4.6 | $638.97 | 93.3% |
| Haiku 4.5 | $45.36 | 6.6% |

The token breakdown tells the real story: **94.5% cache-read tokens** (6.87B of 7.27B total). This is why the marginal cost of each additional agent is lower than raw token counts suggest. The system is reading cached context, not regenerating it.

Export dollars and live quota are different meters. They should not be conflated.

### 3. Per-PR Unit Cost

From reimplementation benchmarks and observed data:

- **$1–$5** per Flow 3 run (automated pipeline execution)
- **~$40 + CI** per solid working PR (full pipeline: scout → plan-review → build → review → deep-review → merge)

The per-PR cost includes the plan-reviews that kill bad specs, the deep reviews that catch bugs, and the queue compression that closes stale issues. Those are not overhead — they are the reason the output is trusted.

## What Makes It Efficient

### 1. Plan-review kills bad work early

Every scout spec in Era 7 was corrected by plan-review. Corrections included:

- Wrong file references (stale line numbers, renamed modules)
- Wrong root causes (symptoms misidentified as bugs)
- Already-fixed issues (11+ closed without building)
- Missing dependencies (crates not in Cargo.toml)
- Wrong architectural assumptions (duplicating existing infrastructure)

A haiku-tier plan-review pass costs a fraction of a sonnet-tier builder. Catching a wrong spec before a builder spends 30 minutes on it is the highest-ROI stage in the pipeline.

### 2. Deep review catches real bugs cheaply

Every deep review in Era 7 found real bugs:

| PR | Bug Found |
|----|-----------|
| #2728 | `sortText` field never serialized — feature was silently inert |
| #2733 | `set -euo pipefail` + grep no-match = exit 1 instead of fail-open 0 |
| #2103 | Semantic token legend desynchronized — every token rendered wrong in all clients |
| #2740 | Telemetry payload needed deduplication, Cancelled exclusion, cleanup |
| #2736 | CRLF edge cases with Unicode surrogate pairs |

A review pass is cheaper than a production bug. Two passes (standards + deep) is cheaper than one pass that tries to do everything.

### 3. Queue compression is productive work

Closing issues produces real value:

| Outcome | Count | Why It Matters |
|---------|-------|----------------|
| Already-fixed | 6+ | Prevents builders from re-implementing existing features |
| Stale/invalid | 4+ | Removes false signals from backlog searches |
| Respecified | 5+ | Converts ambiguous issues into buildable specs |
| Duplicate | 1+ | Reduces scout noise |

This is not housekeeping. It is one of the ways the system gets faster over time.

### 4. Microcrates + worktrees = safe parallelism

The 128-crate workspace means 110 agents can work simultaneously without merge conflicts. Each agent gets a git worktree (isolated copy of the repo) and works on a different microcrate. The architecture was designed for this.

### 5. Cache-read economics

94.5% cache-read tokens means the system is paying for context once and reusing it across agents. The CLAUDE.md, agent definitions, skill files, and codebase context are cached. Each new agent reads from cache rather than regenerating understanding.

## What Limits Throughput

The binding constraints are not what most people expect.

### What IS the bottleneck

1. **Weekly plan limit** — the real budget cap
2. **CI queue** — merges in batches of 3 to avoid cancellation cascades
3. **Control engineering** — stage ownership, label state, receipt freshness, worktree safety
4. **Shared file overlap** — rare but occurs when multiple builders touch the same file
5. **Session time limit** — secondary constraint (~5 hours)

### What is NOT the bottleneck

- Model capability (Opus/Sonnet/Haiku are all sufficient for their assigned tasks)
- Code generation speed (agents write code faster than reviewers can verify it)
- Individual agent quality (mediocre outputs get corrected by the pipeline)

This is the key insight: **the pipeline is more valuable than the agents inside it**. The architecture absorbs error. Mediocre individual outputs get corrected. Strong outputs get sharpened. The system produces trusted change because verification is layered, not because generation is perfect.

## The Core Economic Insight

> Token spend is real, but the scaling pain is mostly CI, queueing, and control engineering.

The expensive resource is not compute. It is:

- **Human attention** — reviewing evidence rather than diff-auditing raw output
- **Trusted change** — verified, tested, reviewed code that can be merged with confidence
- **Backlog truthfulness** — knowing which issues are real, which are stale, and which are already done

The pipeline converts cheap compute into these expensive things. That is the value proposition.

## Transferable Patterns

### "Built but not wired" is the highest-ROI discovery

Every session keeps finding features that are:
- Mostly implemented
- Locally tested
- Not connected to the user path
- Activatable in tens of lines, not hundreds

This session found **17 unwired crates** with 6,566 lines of code and 51 tests — all sitting unused. The wiring cost is minimal. This pattern is not specific to this project.

### Queue-wide unlocks dominate local fixes

One deadlock fix unblocked CI across the entire branch universe. One enforcement fix prevented a class of spec errors. These global unlocks are worth more than ten local feature PRs.

### Classification before implementation

For decomposed error buckets and broad cleanup work, the first successful output is classification, not code. The `unexpected_token_in_expr` bucket (92 files) decomposed into 4 concrete sub-patterns — each with exact file:line fix locations. Without classification, a builder would have tried to fix "the bucket" and failed.

## What This Means for the Article

The strongest framing is not "AI coding is cheap." It is:

> A well-staged pipeline can turn a modest slice of plan usage into a surprising amount of trusted output.

The numbers support it. The pipeline mechanics explain it. The bugs caught prove the verification is real, not theater.
