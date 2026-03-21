# What 100+ Agents Actually Cost

**Session data from March 2026 — concrete, defensible numbers**

---

## The Honest Version

Most "AI agent economics" articles cite theoretical pricing or cherry-picked runs. This one uses
actual billing data from a real development session, cross-referenced against a multi-project
account.

The short version: running 100+ agents across a 2-hour session costs roughly $2-3. The CI that
validates their output costs more than the agents.

---

## Session-Level Data: One 23-Minute Block

The most granular data point available:

| Metric | Value |
|--------|-------|
| Duration | 23 minutes |
| Total tokens | ~4 million |
| Cost | $2.29 |
| Input tokens | 528K |
| Output tokens | 5,816 |
| Cache-read tokens | 3.83M |

The math explains itself: 3.83M of those 4M tokens were cache reads. Cache-read pricing is roughly
10x cheaper than input pricing. When an agent session is 94-96% cache reads — which is typical when
agents share a large context (memory files, skill system, workspace index) — the marginal cost per
session is almost entirely the output tokens.

At ~5,800 output tokens for $2.29, that is roughly $0.39 per 1,000 output tokens. The actual
session cost is driven almost entirely by how much the agent *writes*, not how much it reads.

---

## Token Composition: Why Cache Dominates

Across the full ccusage export for this session:

- **Cache-read**: 94.5% of total tokens
- **Input**: ~0.5% of total tokens
- **Output**: ~5% of total tokens

This ratio is not accidental. It reflects an architecture where agents share large contexts:

- The CLAUDE.md workspace instructions (~3-4K tokens, loaded every invocation)
- Memory files (~10-15K tokens of project state)
- Skill system (~2-5K tokens per skill loaded)
- Workspace index and type information

The first time any of this context is loaded, it is billed as input. Every subsequent read in the
same session (or within the cache TTL window) is billed as cache-read at roughly 10% of the input
rate. A 100-agent session where every agent reads the same 15K-token context pays input rate once;
the remaining 99 agents pay cache rate.

This is why the headline "per session cost" looks misleadingly cheap. The first session of the day
primes the cache. Everything after is nearly free to *read*.

---

## Session-Level Economics: ~30 PRs in ~2 Hours

A typical productive session with 30-50 active agents running in parallel:

| Metric | Value |
|--------|-------|
| Session duration | ~2 hours |
| PRs merged | ~30 |
| Approximate session cost | ~$5-8 |
| Budget consumed | ~3% of weekly budget |

The "3% of weekly budget for ~30 merged PRs" framing reflects the actual session overhead relative
to a ~$200-250/month active development budget. A single session producing 30 merged PRs consumes
about what a Netflix subscription costs for a day.

This is not typical of all sessions. Sessions that involve heavy output generation (agents writing
large new files, running mutation testing, generating comprehensive test suites) cost more. The
cheap sessions are the ones where agents are reading, filing issues, and reviewing. The expensive
sessions are the ones where agents are writing lots of new code.

---

## Per-PR Economics at API Pricing

Rough unit economics:

| Metric | Value |
|--------|-------|
| Cost per flow run (one agent session) | ~$5 |
| Cost per solid PR at API pricing | ~$40 + CI |
| The CI caveat | significant |

The "$40 + CI" figure for a solid PR breaks down as:
- ~2-4 agent sessions (scout + plan-review + builder + reviewer) at ~$5 each
- ~$15-25 in CI costs (see below)

The CI cost note is not a throwaway qualifier. For this project, running the full gate — `just
ci-gate`, which includes `cargo clippy`, `cargo test --workspace`, CPAN corpus check, mutation
testing subset — takes 3-5 minutes locally. In CI, running multiple gates across concurrent PRs
with cache warmup, test isolation, and parallel jobs, the compute bill can exceed the token cost for
the agents that generated the code.

If you have 10 PRs in flight simultaneously, each triggering CI, the CI concurrency cost can easily
dwarf what you paid to write the code. This is the "CI can dwarf token cost" observation: the
bottleneck and the cost center have shifted from code generation to code validation.

---

## Model Economics: Haiku vs Opus

The cost breakdown by model reveals the actual usage pattern:

| Model | Monthly Cost | Role |
|-------|-------------|------|
| **Claude Haiku** | $45.36/month | Cheap support work |
| **Claude Opus** | $638.97/month | Expensive output generation |

Haiku carries the cheap work: routing decisions, issue triage, simple scouting, formatting checks,
status updates. These tasks run at very low output token counts and extremely high cache-read ratios.
Haiku's cost is mostly amortized context reads.

Opus carries the expensive work: plan-review, complex implementation, reasoning about architecture,
writing novel code. Opus output is substantially more expensive per token and these sessions tend to
have higher output-to-cache ratios because they are generating genuinely new content.

The ratio ($638/$45 = roughly 14x) means Opus is where the budget concentrates. If you want to
reduce costs, route more work to Haiku. If you want to maintain quality on complex changes, keep
Opus on the high-stakes tasks.

The practical model routing pattern that emerged:
- Scout (haiku): cheap exploration, file issues
- Plan-review (sonnet): spec completion, gap filling
- Build (sonnet): implementation
- Review (haiku/sonnet): validation, push fixes

Opus would appear on tasks requiring the deepest reasoning. The total allocation reflects actual
usage: most sessions are sonnet or haiku, not Opus.

---

## Full Account Spend: March 2026 in Context

| Period | Total Spend | Notes |
|--------|------------|-------|
| January 2026 | $1,634.08 | Earlier era, less efficient |
| February 2026 | $2,497.66 | Peak spend, less cache optimization |
| March 2026 | $684.66 | Down 73% from February |

The March reduction is not from doing less work — March included the peak 100-agent sessions and
the most PRs merged. The reduction comes from better cache utilization (shared large contexts),
more Haiku routing for cheap tasks, and a cleaner pipeline that wastes fewer tokens on failed or
redundant agent sessions.

February's higher spend includes a period of architectural churn where agents were less constrained
— more exploratory sessions, more dead-end branches, less cache reuse across sessions.

### Lifetime Account Economics

- **Total lifetime spend**: $5,073.22
- **Projects tracked**: 43 projects under one Claude root

The distribution is important context:

| Project | Approximate Spend |
|---------|------------------|
| flow-studio-swarm | ~$818 |
| tokmd | ~$675 |
| perl-lsp | (third or lower) |
| Other 40 projects | balance |

perl-lsp is not the majority of the account's total spend. Two other projects — flow-studio-swarm
and tokmd — have each consumed more than perl-lsp. This matters for the "what does 100 agents cost"
framing: the perl-lsp numbers are the development story of one product, but the real agent
operations span a portfolio of 43 projects under $5,100 total.

$5,073 lifetime across 43 projects over multiple months is roughly $118 per project average. That
is a rounding error in any professional development budget.

---

## What This Means for Replication

If you want to replicate this approach, the honest cost model is:

**Cheap version (small team, well-constrained tasks)**:
- Mostly Haiku + Sonnet
- Heavy cache reuse (shared CLAUDE.md, skill system, memory)
- $50-150/month for meaningful productivity

**Medium version (100-agent sessions, multiple PRs per day)**:
- Sonnet primary, Haiku for support
- $300-700/month (what March 2026 looks like)
- CI costs become significant — budget accordingly

**Expensive version (unconstrained exploration, lots of Opus)**:
- February 2026 pattern: $2,000-2,500/month
- Symptoms: lots of vague-spec builders, repeated explorations, low cache hit rate
- Fix: better scout-constrain-build pipeline, more Haiku routing

The March-to-February reduction (73% cost drop while maintaining productivity) is the clearest
evidence that **methodology matters more than raw agent count**. Running fewer, better-constrained
agents with higher cache reuse is cheaper and more productive than running many unconstrained agents.

---

## CI Is the Real Budget Variable

One data point that deserves its own section: CI cost can exceed agent token cost.

For this project the full gate includes:
- `cargo clippy --workspace` (catches any linting regression)
- `cargo test --workspace --lib` (full test suite, 2,500+ tests)
- CPAN corpus check (4,355 real-world Perl files)
- Mutation testing subset
- Format check

In CI infrastructure terms, a single PR gate run is several minutes of compute. With 30 PRs in
flight, the compute bill scales linearly. The agents that wrote the code might have cost $1-2 each;
the CI that validated each PR might cost $3-8 each depending on infrastructure pricing.

This is why optimizing CI is not a secondary concern. A faster, cheaper CI gate compounds across
every PR merged. Cutting CI from 5 minutes to 2 minutes per run, across 30 PRs, saves more real
dollars than optimizing which model routes to Haiku.

The practical observation: **the bottleneck has shifted**. Code generation is cheap. Code
validation is where the time and money go.

---

## Summary: The Honest Numbers

| Metric | Value |
|--------|-------|
| Cost for a 23-min block (4M tokens) | $2.29 |
| Cache-read % of total tokens | 94.5% |
| Cost per flow run | ~$5 |
| Cost per solid PR (tokens only) | ~$40 |
| CI cost per PR gate run | ~$3-8 (can exceed token cost) |
| Haiku monthly (support work) | $45.36 |
| Opus monthly (output work) | $638.97 |
| March 2026 total | $684.66 |
| February 2026 total | $2,497.66 |
| January 2026 total | $1,634.08 |
| Account lifetime total | $5,073.22 |
| Projects in account | 43 |
| perl-lsp share of lifetime | less than flow-studio-swarm ($818) or tokmd ($675) |

The headline for anyone building with 100+ agents: **the tokens are cheap; CI and infrastructure
validation is where you'll find the real cost**. Design your pipeline to
maximize cache reuse, route cheap work to cheap models, and invest in a fast CI gate. Those three
decisions will move your bill more than anything else.

---

*Data: ccusage export, March 2026 session billing. CI cost estimates are infrastructure approximations, not billing receipts. Model cost figures from billing dashboard.*
