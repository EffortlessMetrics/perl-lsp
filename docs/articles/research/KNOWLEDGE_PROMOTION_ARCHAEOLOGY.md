# Knowledge Promotion Archaeology

## Question

How does this repo turn raw session output into durable knowledge instead of
letting it disappear with the run that produced it?

## Short Answer

The current swarm does not treat every artifact as the same kind of memory.

It promotes knowledge through layers:

- volatile execution state
- durable swarm-state ledgers
- preserved scout research
- operator summaries and wind-down checkpoints
- synthesized archaeology notes
- article-evidence maps for publication-facing claims

That promotion path is the difference between "the swarm remembers things" and
"the repo compounds what the swarm learns."

## 1. The Protocol Explicitly Tells Agents To Promote Durable Conclusions

The strongest current rule is in
[`.claude/skills/swarm-protocol/SKILL.md`](../../../.claude/skills/swarm-protocol/SKILL.md).

It does not treat transcript memory as proof. Instead it tells agents to use:

- `discovered-issues.md` for product leads
- `known-pitfalls.md` for repeatable failure lessons
- `findings.json` for durable control-plane conclusions

It also says:

- check the queue and state before creating new work
- leave durable receipts after each task
- update the findings ledger instead of hiding conclusions in chat or one-off
  handoffs

That is already a promotion model. The repo is telling agents that some things
must be elevated out of transient conversation and into tracked surfaces.

## 2. `swarm-state` Is The First Durable Promotion Layer

[`.claude/swarm-state/README.md`](../../../.claude/swarm-state/README.md)
defines the first durable promotion boundary.

It separates:

- overlap state in `swarm-queue.json`
- dedup and lifecycle in `completed-slices.md`
- out-of-scope leads in `discovered-issues.md`
- reusable traps in `known-pitfalls.md`
- stable control-plane conclusions in `findings.json`

This is not generic note keeping. It is typed promotion.

The same event can therefore be routed differently depending on what it became:

- a temporary execution fact
- a reusable lesson
- a durable decision

That routing is what makes later sessions cheaper.

## 3. Wind-Down Turns Session Output Into Recoverable Next-Session State

[`.claude/commands/swarm-wind-down.md`](../../../.claude/commands/swarm-wind-down.md)
shows the next promotion step.

At shutdown, the swarm is told to:

- stop new work cleanly
- finish PR creation and merge what is green
- run cleanup/report commands
- write Claude memories for session results, roadmap progress, agent
  performance, and unfinished work

The command also lists what gets preserved for the next session:

- `known-pitfalls.md`
- `completed-slices.md`
- `.ops-perl-lsp/swarm-metrics.jsonl`
- pending agent patches
- GitHub issues labeled `swarm-discovered`
- open PRs with auto-merge enabled
- Claude Code memories

That is promotion by policy. The session is not allowed to end as pure chat
residue.

## 4. Status And Report Commands Re-Surface The Promoted Knowledge

The promotion chain matters because the repo actually reads it back.

[`.claude/commands/swarm-status.md`](../../../.claude/commands/swarm-status.md)
surfaces:

- discovery counts from `discovered-issues.md`
- finding counts from `findings.json`
- in-progress slice counts from `completed-slices.md`
- runtime patch counts from `.ops-perl-lsp/agent-patches/`

[`.claude/commands/swarm-report.md`](../../../.claude/commands/swarm-report.md)
then builds a user-facing summary from:

- merged and open PRs
- discovered issues
- pending patches
- swarm metrics

So the promoted state is not archival dead weight. It becomes the input to the
next operator summary and the next round of orchestration.

## 5. Scout Logs Preserve Investigations Before Full Synthesis

The tracked scout logs add a second promotion boundary.

[SCOUT_LOG_ARCHAEOLOGY.md](SCOUT_LOG_ARCHAEOLOGY.md) already showed that the
March 19 scout logs were promoted into versioned repo history on purpose.

The preserved logs:

- [`.claude/logs/scouts/2026-03-19-v0.12.0-readiness.md`](../../../.claude/logs/scouts/2026-03-19-v0.12.0-readiness.md)
- [`.claude/logs/scouts/2026-03-19-install-experience.md`](../../../.claude/logs/scouts/2026-03-19-install-experience.md)

are not live queue state and not final doctrine. They are dated research runs
kept after their useful findings were absorbed into higher-level docs.

That means the repo has a promotion tier for:

- not yet just transient
- not yet final doctrine
- still worth preserving as evidence

## 6. The Repo Distinguishes Canonical Runtime From Derived Exports

[`.claude/README.md`](../../../.claude/README.md) sharpens the next boundary.

It says the canonical runtime surfaces are:

- `.claude/agents/`
- `.claude/skills/`
- `.claude/commands/`
- `.claude/settings.json`
- `.claude/swarm-state/`

and that `docs/handoff/swarm-pack/` is a derived export, not a co-equal design
source.

That matters historically because it means the repo is not just storing more
files. It is naming which surfaces are authoritative and which are generated,
portable, or explanatory.

Promotion here is not only about persistence. It is also about authority.

## 7. Archaeology Notes And Evidence Maps Are The Publication Layer

The final promotion step in this repo is from tracked internal memory into
publication-facing history.

[ARTICLE_EVIDENCE_LINEAGE_ARCHAEOLOGY.md](ARTICLE_EVIDENCE_LINEAGE_ARCHAEOLOGY.md)
is explicit that future article claims should be backed by exact issue, PR, and
doc chains.

That is a different job from `swarm-state`:

- `swarm-state` helps the swarm operate
- scout logs preserve session research
- archaeology notes synthesize historical meaning
- evidence maps constrain future public claims

This is how the repo keeps launch articles from drifting into nice-sounding but
unrecoverable summary.

## 8. Why This Matters

Many repos have memory. Fewer have promotion rules.

This one preserves a more interesting pattern:

- transient work gets routed into typed durable ledgers
- shutdown explicitly writes forward-looking memory
- status/report commands consume that memory again
- scout research can be promoted into tracked logs
- archaeology notes digest it into historical interpretation
- article evidence maps turn that interpretation into source-linked claims

That is not just "good documentation." It is a real knowledge promotion system.

## Strongest Evidence-Backed Claims

1. The current swarm protocol explicitly tells agents to move durable
   conclusions out of chat and into tracked ledgers.
2. `swarm-state` is the first durable promotion boundary, and it classifies
   memory by job instead of dumping everything into one file.
3. `swarm-wind-down` makes session preservation an operating requirement, not
   an optional cleanup step.
4. `swarm-status` and `swarm-report` prove the promoted state is actively
   re-consumed by later orchestration.
5. Scout logs are a distinct preserved-research layer between live swarm memory
   and polished archaeology.
6. Article evidence maps are the final promotion layer that constrains public
   launch claims to exact source chains.

## See Also

- [KNOWLEDGE_COMPOUNDING_ARCHAEOLOGY.md](KNOWLEDGE_COMPOUNDING_ARCHAEOLOGY.md)
- [SCOUT_LOG_ARCHAEOLOGY.md](SCOUT_LOG_ARCHAEOLOGY.md)
- [SWARM_MEMORY_TAXONOMY_ARCHAEOLOGY.md](SWARM_MEMORY_TAXONOMY_ARCHAEOLOGY.md)
- [ARTICLE_EVIDENCE_LINEAGE_ARCHAEOLOGY.md](ARTICLE_EVIDENCE_LINEAGE_ARCHAEOLOGY.md)
- [CONTROL_PLANE_ARCHAEOLOGY.md](CONTROL_PLANE_ARCHAEOLOGY.md)
