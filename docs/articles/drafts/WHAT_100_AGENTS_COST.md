# What 100+ Agents Actually Cost: The Economics Nobody Publishes

*A benchmark from a real session, March 2026. Concrete numbers, not estimates.*

---

## The Number

$2.29.

That is the cost of a 23-minute block of a 100-agent development session on perl-lsp: a production Rust LSP server built entirely by AI agents under one human's strategic direction. In that 23 minutes, the agents processed roughly 4 million tokens. In the full session, they produced 64 merged pull requests.

$2.29 for 23 minutes.

The session as a whole consumed roughly 3% of a $200-250/month active development budget.

Most articles about AI agent economics cite pricing charts or theoretical projections. This one uses billing data from a ccusage export, cross-referenced against session logs and memory files. The numbers are real. The reactions to them tend to be either "that can't be right" or "so why is my bill so high?" The answer to both is the same: cache-read tokens.

---

## Why It Is Cheap

Of those 4 million tokens in the 23-minute block, here is the composition:

| Token type | Count | Share of total |
|---|---|---|
| Cache-read | 3.83 million | ~88% |
| Input (non-cached) | ~528K | ~12% |
| Output | ~5,816 | ~0.1% |

Across a full session with many agents sharing context, the cache-read fraction climbs further. The source billing data shows session-level cache-read ratios reaching 94.5% when context is fully warm.

Cache-read tokens are billed at roughly one-tenth of input token pricing. When 94.5% of your tokens are cache reads, your effective rate is not "input token pricing" — it is something closer to one-tenth of that for almost all your traffic.

The reason the cache dominates is architectural. In a well-built agent system, every agent reads the same large context on every invocation:

- Workspace instructions (CLAUDE.md): ~3-4K tokens
- Memory files (project state, learned patterns, failure history): ~10-15K tokens
- Skill definitions (builder steps, verify commands, PR templates): ~2-5K tokens per skill loaded
- Tool call results and session state

The first agent in a session loads all of this as input tokens. Every subsequent agent in the same session reads it as cache. A 100-agent session where every agent reads the same 20K-token shared context pays input rate once; the remaining 99 agents pay cache rate.

This is counterintuitive because token counts look large. "4 million tokens" sounds expensive until you understand that 3.8 million of them are at 10% cost. The session cost is driven almost entirely by how much agents *write* (output tokens), not how much they read.

At $2.29 for ~5,800 output tokens, the effective output rate was roughly $0.39 per thousand tokens. The "per agent cost" in a heavily cached session is essentially the cost of whatever that agent generates. An agent that writes a short PR description and a targeted code fix generates maybe 800 tokens of output. Cost: ~$0.30.

The marginal cost of "one more agent" in this system, when the context is already warm, is almost nothing.

---

## The Real Cost

The framing above will make some readers want to immediately replicate it. Before doing that, here is what the $2.29 number obscures.

The real cost center is not tokens. It is CI.

In this session, 64 merged PRs means 64 CI runs. The full gate for perl-lsp includes:

- `cargo clippy --workspace` (lint across all 128 crates)
- `cargo test --workspace --lib` (2,500+ tests)
- CPAN corpus check (4,355 real-world Perl module files)
- Format check

In CI infrastructure, a single full gate run is several minutes of compute. With 64 PRs, that is several hours of cumulative CI machine time. The token cost to generate the code was roughly $2-3 per agent session. The CI cost to validate each PR was potentially $3-8 per run, depending on infrastructure pricing.

The CI cost can exceed the token cost.

There is more. Sixty-four simultaneous pull requests create merge ordering problems. The protocol for this project is batches of three: merge three, wait for CI to complete, merge the next three. Rapid merges without waiting cancel each other's CI runs — a cascading CI cancellation problem. At 64 PRs, you have roughly 21 merge batches. At 5 minutes per batch, that is over an hour of pure merge operations, serialized.

Rebase cascades add to this. When one PR merges and changes shared infrastructure — a Cargo.toml, a module that multiple crates import, a test helper — subsequent open PRs may need rebasing. With 64 PRs in flight, a single merge to a shared file can trigger a dozen rebase operations, each of which requires a new CI run.

Git itself becomes a constraint at high concurrency. Above roughly 25 concurrent agents all writing to worktrees in the same repository, git's object lock contention becomes measurable. The git config lock (`.git/config.lock`) serializes certain git operations. The worktree disk footprint is also non-trivial: each agent worktree is a full checkout of the repository, which in a 550K-line Rust codebase is roughly 4-5GB per worktree. In one session, 52 orphaned worktrees needed cleanup — that was ~260GB of disk recovered.

The infrastructure engineering to run this reliably is the real cost, and it is not captured in billing dashboards.

---

## The Breakdown

Working from the billing data and session logs, here is the per-unit economics:

**Per agent session (one agent, one task):**
- Token cost: ~$2-5 (heavily cache-dominated)
- Agent runtime: 20-40 minutes
- Output: 0 or 1 PR (scouts produce issues, not PRs)

**Per solid merged PR (full pipeline):**
- ~2-4 agent sessions (scout + plan-review + builder + reviewer)
- Token cost: ~$15-30
- CI cost: ~$10-25 (2-5 CI runs including retries and reruns)
- Total: roughly $40 + CI infrastructure overhead

**Per session (full 100-agent session):**
- Token cost: ~$5-8 (one billing block at $2.29 represents one sub-block; full sessions run multiple blocks)
- Budget consumed: ~3% of monthly active development budget
- Output: 20-64 merged PRs depending on phase (merge-drain sessions produce more than research sessions)

**Model routing:**
The actual spend breakdown splits roughly as:
- Haiku: cheap support work (routing, triage, simple scouting, status checks) — low per-session cost, very high cache ratio
- Sonnet: plan-review, implementation, complex scouting — moderate per-session cost
- Opus: architectural reasoning, novel design work — highest per-session cost, lowest cache ratio

Observed monthly split: Haiku at roughly $45/month, Opus at roughly $639/month, with the remaining budget on Sonnet. The ratio ($639/$45 = 14x) illustrates where the budget actually concentrates: complex output work, not cheap reads.

The account-level picture for March 2026: total spend was $684.66, down 73% from February's $2,497.66 — while running more agents and producing more PRs than any previous month. The cost reduction came from better cache utilization, more Haiku routing for support work, and a tighter pipeline that wastes fewer tokens on vague-spec exploratory sessions.

---

## What Makes It Work

The 94.5% cache-read ratio does not happen by accident. It is the result of several specific architectural choices.

**Pipeline narrowness.** Each agent does exactly one thing. A scout reads code and files an issue. A builder takes a spec and implements it. A reviewer reads a PR diff and pushes improvements. No agent does all three. Narrow focus means short output sequences (the expensive part) and long shared context reads (the cheap part).

**Stable shared context.** CLAUDE.md, the skill system, the memory files — these are loaded on every agent invocation. Because they are stable across a session, they hit cache. An unstable context (one that changes every invocation) would have a much lower cache ratio and a proportionally higher bill.

**Context compounding.** The skill system and memory files grow in value over time. An agent in session 100 benefits from memory files encoding 99 sessions of lessons. The context load does not increase linearly with knowledge — it grows slowly (well-curated memory is compressed knowledge) while the value compounds rapidly. This is why the cost-per-PR actually *decreases* over time even as the project grows more complex.

**Microcrate architecture.** 128 workspace crates in this project means 128 independent contexts. Agents working on different crates do not conflict, do not need to coordinate, and do not need to read each other's work. The isolation that makes mass parallelism safe also keeps individual agent sessions focused and short.

---

## What Does Not Scale

**CI width.** The merge queue is the binding constraint, not agent count. The math: CI runs take ~5 minutes. Each batch of 3 merges needs one full CI cycle. 64 PRs / 3 per batch = ~21 batches = ~105 minutes of pure merge operations. Adding 100 more agents does not help if the merge pipe can only process 3 at a time. The optimal number of concurrent coding agents is roughly `merge_queue_width * agent_work_time / merge_cycle_time` = 3 * 15 / 5 = ~9. The other ~90 agents in a 100-agent session should be scouts, reviewers, and planners — not more builders.

**Git lock contention.** Above ~25 concurrent agents with active git operations, lock contention becomes measurable. The worktree approach (each agent in its own git worktree) mitigates this for the work itself, but operations that touch the shared `.git/` directory (config updates, large object operations) still serialize.

**Worktree disk.** Each worktree is a full checkout. In a large Rust repo, that is 4-5GB per worktree. 50 concurrent worktrees = 200-250GB. Cleanup is not automatic — orphaned worktrees accumulate if agents exit without cleanup. This is a systems administration problem that token costs do not capture.

**Control engineering time.** The methodology does not run itself. Someone has to design the pipeline, write the skill definitions, curate the memory files, decide which work to prioritize, and make the merge decisions. The billing dashboard shows $684.66 for March. It does not show the 150-200 hours of human strategic direction that makes those tokens produce useful output rather than churn. The human attention cost is real and is not on any invoice.

---

## The Comparison

Traditional software development: a senior Rust developer costs $90-125/hour productive time at current San Francisco market rates. Five developer-days of work (one week, one person) costs $3,600-5,000 in productive hours, more in fully-loaded cost. That produces perhaps 5-15 merged PRs depending on complexity.

This pipeline: one session, roughly $5-8 in tokens plus CI overhead, 20-64 merged PRs. Overnight. With 100 agents.

The comparison sounds absurd until you look at what it requires to achieve it. A pipeline that produces 64 merged PRs overnight requires:

- A codebase structured for parallel agent work (128 isolated crates, no circular dependencies)
- A scout-constrain-build pipeline (constrained agents succeed ~90% of the time; unconstrained agents succeed ~50% of the time)
- A skill system so agents know exactly what to do without being told
- Memory files so agents do not repeat mistakes from previous sessions
- Automated review agents so humans are not the bottleneck at review
- A CI infrastructure that can handle 64 parallel runs
- A merge discipline that serializes carefully to avoid cascade failures
- A human who understands the system well enough to direct it

The $40+CI per PR figure is not the cost of running an agent. It is the amortized cost of the entire pipeline, including all the infrastructure engineering, all the failed experiments, and all the human strategic direction that makes it work.

The pipeline is the cost. The $2.29 billing block is just the visible tip.

---

## Why Nobody Publishes This

Token count leaderboards make the numbers look scary. "4 million tokens" sounds expensive to someone accustomed to chatbot pricing. Nobody leads with "and 94.5% of those tokens were 10x cheaper than you think."

Cache-read economics are also invisible in most billing dashboards. They are a line item that requires drilling into the detailed breakdown, not the summary view. It is easy to look at total tokens and assume uniform pricing.

CI cost is distributed across infrastructure invoices that are separate from AI API invoices. If you run CI on GitHub Actions, the compute cost is on a GitHub invoice. If you run on a cloud provider, it is on a separate cloud invoice. Combining these numbers into a single per-PR cost requires deliberate accounting that most teams do not do.

And the honest number is, frankly, boring. "$2.29 for a 23-minute block" is not a headline. "$5,073 lifetime across 43 projects" is not a horror story — it is less than some teams spend on Slack in a year. The scary version of this article would cherry-pick total tokens without mentioning cache, cite gross billing without noting the 73% month-over-month cost reduction, and ignore that the expensive sessions were the poorly-constrained ones, not the well-engineered ones.

The real lesson of the $2.29 number is not that AI is cheap. It is that *the methodology is the asset*. The same session architecture with vague-spec agents and no memory system would have cost 4x more and produced one-tenth the output. February 2026 ($2,497.66) versus March 2026 ($684.66) is not a pricing change — it is the same agent platform, the same team, the same project. What changed was the pipeline discipline.

The benchmark is not $2.29. The benchmark is $684 for a month that produced more shipped output than any previous month. And the path from February's $2,497 to March's $684 was not cutting agents. It was making them work correctly.

---

*Data: ccusage export March 2026, session billing dashboard, memory files from project_cycle6_progress.md and project_cycle5_session3.md. CI cost estimates are infrastructure approximations. Model cost figures from billing dashboard. All figures are from perl-lsp development on the Claude API.*
