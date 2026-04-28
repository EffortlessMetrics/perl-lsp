# 2026-04-25 — Multi-Axis Scaling Design

**Window**: Correction emitted near end of the 2026-04-25 session
**Audience**: Future orchestrators reasoning about throughput, throttling, and idle plan activation
**Purpose**: Document a structural property of the methodology that earlier session synthesis had been mis-modeling

This doc captures a property the operator made explicit when correcting an orchestrator analysis. The methodology is **horizontal-by-default** — it scales across boxes, plans, and repos, not just within a single session. Prior session retrospectives (including the sibling `2026-04-25-process-meta-learnings.md` "sustainability question") implicitly assumed a single operator on a single box on a single repo, and reached conclusions that don't survive that assumption.

---

## The mis-model

Earlier session synthesis framed the steady-state arithmetic as:

> If Codex bursts produce ~40 PRs/day and the verification ladder closes/merges ~125/day at full intensity, the queue shrinks by ~85/day. [...] sustainable cadence is closer to 1 such session per 2-3 days.

And drew the implicit conclusion:

> The next operator decision point is whether to keep the high-intensity drain pattern or shift toward upstream throttling (e.g., fewer simultaneous Codex threads, longer per-thread research time to reduce hallucination rate).

Both statements assume a fixed downstream capacity. The first treats "1 session per 2-3 days" as a ceiling; the second treats "throttle upstream" as the only response to upstream-burst overload.

**Both are wrong** because the downstream is not a single fixed unit. It scales on three independent axes.

---

## The three independent scaling axes

### Axis 1 — Within-session parallelism

Multiple agents inside one orchestration session. Already proven in the 2026-04-25 sessions: 25-agent waves dispatched repeatedly within a single 5-hour window. The Wave 4 mass triage produced ~38 PR closures from a single ensemble agent in one wave; total session output was 13 merges + 111 closures.

**Limit**: per-session quota (Anthropic 5h session + weekly cap), per-machine GitHub rate limits, and worktree disk overhead (~10-30 GB across 173+ worktrees in one session).

**Where it breaks**: Wave 7 of the 2026-04-25 session, when org monthly cap blocked sub-agent spawns mid-wave.

---

### Axis 2 — Multi-box parallelism

Multiple Claude Code instances on different machines, each running its own orchestration session. Different plans = different quotas, no contention.

Per the operator: "we can go wider on review and improve and merge. We're designed for it and can already in theory via other boxes and other plans, both of which are already configured."

The key word is *configured*. The hardware and account separation already exist; activation cost is starting the session, not provisioning the environment.

**Plans currently sitting idle (per operator)**:

| Plan | Status | Notes |
|---|---|---|
| GLM Coding Plan | configured, idle | not yet wired into orchestration |
| Fireworks Firepass | configured, idle | not yet wired into orchestration |
| Minimax Token Plan | configured, idle | not yet wired into orchestration |
| OpenCode Go | configured, idle | not yet wired into orchestration |

Activation cost estimated by operator at ~1 week of setup time for full integration. That is a configuration-time cost, not a methodology-design cost.

**Limit**: cross-box coordination. Multiple sessions on the same repo can produce duplicate work without explicit coordination tooling. Discussed in the open-questions section below.

---

### Axis 3 — Multi-repo parallelism

When one repo's queue hits a flood threshold, the same methodology shards across other repos. Per the operator: "we can spread across other repos (because this one got a little too flooded at 500 PRs)".

Per operator, the methodology has been operating across multiple projects for ~1 year. perl-lsp is one of N projects under the methodology; its forensics docs are one shard of a multi-project knowledge base. The patterns documented in this repo's `docs/forensics/` are field-validated across projects, not hypothetical.

**Limit**: per-operator attention. Even with infinite boxes/plans, the operator's working memory across N concurrent repos has finite bandwidth. This is the binding constraint at high scale, not compute.

---

## The "Codex throttling" misconception

The earlier "shift toward upstream throttling" framing assumes downstream capacity is fixed and upstream rate is the lever. The actual design is the inverse:

**Upstream burst rate determines downstream parallelism requirement.** If Codex (across all operator threads, across all repos) produces 100 PRs/hour, the orchestration scales to match by spinning up additional boxes/repos. Throttling Codex would throw away the cheap-generation half of the asymmetric workflow.

The asymmetric two-LLM workflow documented in `2026-04-25-process-meta-learnings.md` § 1 explicitly relies on upstream being cheap and abundant. Throttling upstream to match downstream is solving the wrong problem.

**Correct framing**: per-repo concentration is the operational variable, not total throughput.

---

## Per-repo concentration as the actual operational metric

Observed pattern from 2026-04-25:

- ~500 open PRs in one repo = upper edge of operational tractability
- More open PRs in one repo means: more agent-collisions on the same scope (4-6% collision rate observed at ~327 open PRs scales worse), longer `gh pr list` queries, more git history to reason about per dispatch, larger worktree disk footprint
- The 410 → 327 trajectory across the 2026-04-25 sessions illustrates this — the repo was at the upper edge and is now back in normal range (250-300)

**Right response when concentration hits this**: shift Codex bursts to other repos in the methodology, let this one drain back to the 250-300 range.

**NOT the right response**: throttle Codex itself. That sacrifices upstream cheapness for a problem that load-balances away.

This recasts the "open question" from `2026-04-25-process-meta-learnings.md` § 10. The question wasn't "queue-zero or steady-state-low" — it was "is the per-repo concentration threshold the operational variable we should be tracking instead of total queue size".

---

## Layer diversity argument (independent of throughput)

The current verification ladder is mostly Anthropic models downstream (Sonnet for plan-review, deep-review, green-refactor; Haiku for accuracy, research, oppositional, architecture, maintainer, standards, diff-audit, green-CI). Defense-in-depth has a known correlated-failure risk:

> If Sonnet has a blind spot for some Perl construct (e.g., indirect method syntax, prototype declarations, tied variables), all the downstream Anthropic agents miss it the same way. The catch rate stays high *for things Sonnet is good at* while quietly dropping toward zero for things Sonnet is bad at.

The claim "deep-review catches ~100% of bugs" from prior forensics is true *conditional on the bug class being one Sonnet can see*. We have no observation set for the bug class Sonnet can't see — by construction, those would slip past silently.

**Mitigation**: layer diversity by model family. Activating the idle plans (GLM, Fireworks, Minimax, OpenCode) for *different downstream layers* decorrelates failure modes in a way that activating a second Anthropic instance can't.

Sketch of a layer-diversified ladder (illustrative, not prescribed):

| Layer | Current model | Diversification candidate |
|---|---|---|
| accuracy-scout | Anthropic Haiku | OpenCode Go (different family entirely) |
| research-verifier | Anthropic Haiku | Fireworks-hosted model |
| architecture-reviewer | Anthropic Haiku | GLM (different training data distribution) |
| deep-review | Anthropic Sonnet | (kept on Sonnet — final correctness gate) |

Even at current per-repo volume, decorrelating downstream layers across model families would catch a class of bugs the all-Anthropic ladder structurally cannot see. **This argument is independent of the throughput argument.** Even if throughput were not a concern, layer diversity is a structural risk reduction.

The activation cost (per operator estimate, ~1 week) is paid once and amortizes across every future session.

---

## What this changes about long-term planning

| Old framing | Corrected framing |
|---|---|
| "Sustainable cadence is 1 session per 2-3 days" | Per-box cadence; total cadence scales with box count |
| "Throttle upstream Codex to match downstream" | Shard upstream across repos when per-repo concentration hits threshold |
| "Steady-state queue is ~200 PRs" | Steady-state per-repo concentration is ~250-300; total across all repos is N × that |
| "Verification ladder catches ~100% of bugs" | Catches ~100% of bugs in the class Sonnet can see; layer diversity needed for the class Sonnet can't see |
| "Quota is the binding constraint" | Per-operator attention is the binding constraint at scale; quota is per-axis |

Concretely, future planning should:

1. Track per-repo PR concentration as one operational metric among several, not the only metric.
2. Document which plans are configured vs. idle, and the activation cost, so layer-diversity is reachable as a deliberate decision.
3. Treat "this repo is flooded" as a routing problem (shift Codex to other repos), not a capacity problem (throttle Codex).
4. When considering a throughput target, multiply by the number of active boxes × repos rather than treating it as a single-session metric.

---

## The cross-operator coordination problem (genuinely open)

At higher scale (5 operators on 5 boxes running 5 repos = 25 active orchestration units), the open question is:

- Who decides which Codex bursts go where?
- How do operators avoid duplicating each other's work?
- How do simultaneous sessions on the same repo coordinate (cross-session collisions are the multi-box analog of within-session collisions)?

The forensics doc layer (this directory) partially solves cross-session coordination: a session reads prior sessions' forensics to pick up routing decisions from data, not by re-discovering. This is sufficient for sequential single-operator sessions and probably sufficient for parallel sessions on different repos.

For parallel sessions on the *same* repo, explicit cross-session coordination tooling will be needed. Candidates (none built):

- Shared label state as the source of truth (already partially true via the pipeline state labels)
- Per-session leases on PR ranges ("operator A handles PRs created in 24h window X, operator B handles window Y")
- Real-time messaging between sessions (shared channel, periodic state broadcasts)

This is the next frontier problem. The methodology has not yet been stress-tested at 5+ concurrent operators on the same repo — the current scale is 1 operator on 1 repo at a time, with multi-repo handled by serial attention rather than parallel boxes.

---

## Generalization

The methodology has been operating across multiple projects for ~1 year (per operator). The patterns documented in `docs/forensics/` are field-validated across projects, not hypothetical. perl-lsp is one of N projects under the methodology; its forensics docs are one shard of a multi-project knowledge base.

This implies that operational patterns observed here (collision rates, ladder catch rates, master bit-rot cascade detection, ensemble-curator economics) are likely transferable to other projects under the same methodology. The reverse is also true: patterns observed in other projects' forensics likely apply here. The methodology's knowledge base lives across project boundaries, not within any single repo.

**Implication for this repo's forensics**: the docs in this directory should not assume single-repo context. Patterns named here should be named in transferable terms. Repo-specific facts (commit SHAs, PR numbers, file paths) belong in the supporting prose; the named pattern itself should generalize.

---

## Applies to

Future orchestrators should reference this doc when:

- **Reasoning about throughput targets.** Don't plan around a single-session capacity. Plan around per-repo concentration limits and multi-axis scaling response.
- **Considering throttling vs. fan-out tradeoffs.** Default response to "queue is too long" is fan-out (more boxes, more repos), not throttle-upstream.
- **Evaluating idle plan activation.** Two independent justifications: (a) throughput when concentration is uniformly high across repos, (b) layer diversity to decorrelate downstream failure modes. The second justification doesn't require the first.
- **Synthesizing operational metrics.** Per-repo PR count is one metric among several. Total open PRs across all methodology repos is a different and possibly more relevant metric.
- **Planning a session that may exceed single-box capacity.** Recognize this is a routing decision (which axes to activate), not a "do less" decision.
- **Writing forensics docs.** Don't assume single-operator-single-box-single-repo context. Name patterns transferably; let prose carry the repo-specific facts.

---

## Cross-references

Sibling docs from the 2026-04-25 session:

- `2026-04-25-3day-arc-economics-and-learnings.md` — quantitative metrics for the 3-day arc (single-repo, single-operator framing)
- `2026-04-25-orchestration-anatomy.md` — wave composition and within-session parallelism (Axis 1 detail)
- `2026-04-25-process-meta-learnings.md` — process patterns; § 10 is the entry this doc partially supersedes (the "sustainability question" was framed in single-axis terms)

The "sustainability question" in `2026-04-25-process-meta-learnings.md` § 10 should be read with this doc as a corrective overlay. The arithmetic in that section is correct *for one box on one repo*; the conclusion that throttling upstream is the response to overload is incorrect once Axes 2 and 3 are in scope.
