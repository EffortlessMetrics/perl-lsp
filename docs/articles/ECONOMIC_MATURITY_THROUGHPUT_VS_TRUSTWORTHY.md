# Economic Maturity: From Throughput to Trustworthy Throughput

*The next 20% efficiency gain is not more agents or faster cycles — it is queue hygiene,
trustworthy review routing, and reducing the cost of the last few PRs in each wave.*

---

## The Three-Metric Evolution

Every engineering organization moves through a predictable progression as it matures.
The perl-lsp swarm is no exception.

**Phase 1 — Raw throughput** (Era 6-7, 2025 Q3-Q4): The metric was PRs merged per session.
Maximizing the number was the goal. The constraint was agent availability and spec quality.
A "good session" meant 20+ merges.

**Phase 2 — Trustworthy throughput** (Era 7, 2026 Q1-Q2): The metric shifted to PRs merged
that did not subsequently require rollback, follow-up fix, or master cleanup. The 2-pass
review design (haiku standards + sonnet deep) became the architectural center because
raw throughput without trust is just expensive tech debt accumulation. A "good session"
meant every merged PR had been independently verified by at least two review agents.

**Phase 3 — Tail cost awareness** (current): The third evolution is recognizing that not all
PRs cost the same. The first 40 PRs in a wave are clean, fresh, merge-on-first-review.
The last 10-15 carry compounding liabilities: stale branches, rebase conflicts, CI state
that has rotated since the PR opened, and review labels that no longer reflect the current
HEAD. This is the "dirty tail" — and it costs 5-10× more per merge than early-cycle PRs.

The question for the current phase is: can we reduce the dirty tail without slowing early-
cycle velocity?

---

## The Four Metrics to Track Going Forward

### 1. Cost per actionable outcome

"PRs created" is a lagging indicator of effort, not a leading indicator of value. The
useful numerator is:

- Merges (value delivered)
- Duplicate closes with documentation (queue drained without wasted build slots)
- Real deep-review bug fixes (quality gates working as designed)
- Master-fire unblocks (cascades fixed, cluster of N PRs re-enabled)

The useful denominator is agent-hours (session budget consumed).

Cost per PR is a fine headline metric. Cost per actionable outcome is the diagnostic
metric. When these diverge — high PR count, low actionable outcomes — it usually means
the session produced a lot of intermediate work products (specs, plans, re-routes) without
completing them. This pattern is detectable early by tracking the ratio of "in-build" issues
to "build complete" outcomes at session end.

### 2. Value per PR (scope and impact, not size)

Not all PRs carry the same value even when they cost the same. A PR that fixes a root-
cause parser bug unblocking a CPAN corpus regression carries more value than a PR that
adds a docstring.

The current proxy for value is `size/S`, `size/M`, `size/L` labels. These measure scope
(number of files, lines changed), which correlates with difficulty and risk but not directly
with impact. A more useful decomposition:

| Dimension | Proxy measure |
|-----------|---------------|
| Difficulty | Files changed, crates touched |
| Risk | Production path vs. test-only vs. doc-only |
| Impact | Corpus coverage delta, features enabled, issues closed |
| Durability | Whether the change adds a test that would have caught it earlier |

The "durability" dimension is the least tracked and the most predictive of long-term
quality. PRs that fix a bug without adding a regression test are net-negative on quality
debt even when they are net-positive on functionality.

### 3. Queue depth slope

The queue (open PRs) behaves like a leaky bucket. Ingest rate (new PRs created) determines
the fill rate; merge rate determines the drain rate. The slope of the queue — is it growing
or shrinking? — is the health signal.

**2026-04-24 observation**: The session merged 75 PRs and closed 156 (non-merge), resolving
231 total. But Codex waves generated new PRs mid-session, and the queue finished at 388 —
higher than it started. The queue slope was positive despite record closure numbers.

This is not inherently bad. A positive slope during a productive session is expected when
new work is discovered faster than old work is completed. The warning sign is a queue that
grows despite declining merge rates — that pattern indicates the work is piling up at
review bottlenecks, not at the supply end.

Tracking queue depth slope per pipeline stage (how many PRs are stuck waiting for each
label type) makes the bottleneck visible. If 80 PRs are waiting for `deep-reviewed` and
only 20 are waiting for `review-reviewed`, the deep-review stage is the constraint.

### 4. Dirty-tail cost

The "dirty tail" is the last ~15% of PRs in any wave. They carry compounding liabilities:

- **Branch staleness**: The branch was cut from a master commit that no longer exists.
  Rebase needed before merge. Rebase time + CI re-run time.
- **Review label staleness**: `review-reviewed` was set on a SHA that has since been
  amended. The label is technically wrong; a new review pass is needed.
- **Context loss**: The original scout's spec is no longer accurate for the current
  codebase. Parts of the fix already landed via a superseding PR. Builder has to
  re-research before completing.
- **Conflict cascades**: If multiple dirty-tail PRs touch the same file (historically
  `scope_analyzer.rs` and `Cargo.toml` workspace sections), the first merge forces
  manual conflict resolution on all subsequent ones.

Rough cost multiplier observed across sessions: dirty-tail PRs (age > 5 days, rebase
required) cost 5-10× more per successful merge than same-day PRs. The cost is mostly in
CI time, not agent time — but CI time burns the merge-batch capacity.

**Mitigation approaches that work:**
- Batch merges by dependency graph, not by PR number. PRs touching the same files should
  merge in the same batch, back-to-back, before other PRs can rebase-conflict them.
- Run `gh pr update-branch` on the top 20 waiting PRs at session start, before spawning
  builders. Freshening branches early prevents the staleness accumulation.
- Close PRs older than 14 days if no builder is actively working them. A closed-with-
  comment PR can be re-opened; a perpetually-stale open PR is invisible drag on the queue.

---

## Cost Numbers (Verified: 2026-04-24)

The following cost model is based on verified subscription costs and observed per-PR metrics.

**Subscription baseline:**
- Claude Max: $200/month (20× usage vs. Free; this is the compute budget for the swarm)
- ChatGPT Pro: $200/month (Codex + planning; separate budget)
- Combined: $400/month flat subscription, no per-token charges at this tier

**Per-PR CI cost:**
- ~$0.339/PR on average, based on CI compute time per run
- Dominated by the Rust compilation step; parallel test runs are cheaper
- Dirty-tail PRs run CI 2-3× (initial + post-rebase + post-fix) = $0.68-$1.02/PR

**Per-PR agent amortization:**
- ~$0.38/PR averaged across the full pipeline (scout + verify + plan + build + review × 2)
- Deep review adds ~$0.08/PR on top of the haiku pass
- Ensemble curator (reading 3-5 Codex variants per issue) adds ~$0.12/PR when used

**Combined per-PR cost (approximate):**
- First-pass clean PRs: $0.50-0.75/PR all-in
- Dirty-tail PRs: $1.50-2.50/PR all-in after rebase, re-review, and CI retriggers

At these numbers, the economic argument for dirty-tail prevention is clear: avoiding one
dirty-tail rebase cascade saves the cost of 3-5 clean PRs.

---

## The Four Biggest Cost Sinks

### 1. Red master cascades

When master is red — compilation failure, formatting failure, or test regression — every
PR waiting to merge is blocked. More precisely, every PR that passes its own CI gets a
stale green signal. When master is fixed, all those PRs need fresh CI runs before they
can merge.

The 2026-04-24 session saw four master-side fixes (#5749, #5751/#5783, #5965, #5986) that
together unblocked roughly 60 PRs. Each fix required: identifying the root cause on master,
building a narrow fix, merging it, then running `gh pr update-branch` across the affected
cluster. Estimated cost: 2-3 hours of session time, 4 CI runs on master, 60+ CI re-runs
across the cluster.

Prevention: The `just pr-fast` gate runs before every push. The gate exists specifically
to prevent master from going red. When it fails, the failure must be addressed immediately,
not merged around. Two violations of this pattern in a session indicate a process gap, not
a code quality gap.

### 2. Dirty-tail rebase work

Described above. The key operational note: the cost is often invisible because it is
distributed across many small CI retriggers, each of which looks inexpensive individually.
The aggregate is where the damage shows.

### 3. Shallow-label review waves

When a wave of PRs goes through haiku review without also dispatching sonnet deep review,
the haiku reviews create a false signal: the PRs look reviewed and labeled, but they have
not been checked for semantic correctness. If they then merge in bulk, correctness bugs
reach master.

The pattern that produces this failure: a session spawns 20 haiku reviewers, they run in
parallel, they all complete, the queue looks "done." But the `deep-reviewed` label is
absent on all 20 PRs. If the orchestrator routes to ops at this point — or if an agent
applies `merge-ready` prematurely — correctness bugs ship.

Mitigation: The pipeline state machine requires `deep-reviewed` as a prerequisite for
`merge-ready`. But this invariant is enforced by convention, not by code. An orchestrator
that is not reading labels carefully can violate it. The documented fix: `reviewer-deep`
should not invoke `/pr-ready` at end (see `feedback_deep_reviewer_premature_merge_ready.md`).

### 4. Environment friction

Two persistent sources of environment cost:

**RTK hook not installed**: The `rtk` tool reduces token consumption 60-90% on standard
commands. When the hook is not installed (`/!\ No hook installed — run rtk init -g`),
every `cargo test`, `git log`, `gh pr list` returns full unfiltered output. This is not
a correctness issue, but it increases context window consumption per agent, which reduces
the number of useful steps an agent can take before hitting context limits.

**Windows path issues**: Windows-specific path normalization bugs (`std::fs::canonicalize`
expanding 8.3 short names, `CARGO_BIN_EXE_*` dropping backslashes, `MAX_PATH` failures
in deep worktrees) create a consistent class of CI failures that only appear on Windows
runners. These failures block the PRs that trigger them and require investigation time
that Linux-only development would not incur. The current mitigation is `xtask`-based
harness code (cross-platform Rust vs. platform-specific shell scripts), but the migration
is not complete.

---

## Looking Forward

The swarm has passed the "throughput" phase. The metrics that matter now are downstream
of raw count:

1. What fraction of merged PRs needed a follow-up fix within 14 days? (Quality durability)
2. What is the average age of PRs at merge? (Queue health)
3. What fraction of deep reviews caught at least one real bug? (Review effectiveness)
4. What is the cost delta between first-quartile and last-quartile PRs in each session?
   (Dirty-tail detection)

None of these metrics require new tooling. They require reading the data already in the
GitHub API and computing derived values instead of headline counts.

---

_Related: `docs/articles/SESSION_2026_04_24_ECONOMICS.md`, `docs/articles/SESSION_7_ECONOMICS.md`, `docs/articles/COST_ROI.md`_
