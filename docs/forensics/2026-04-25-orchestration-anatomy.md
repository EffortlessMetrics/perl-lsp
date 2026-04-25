# 2026-04-25 — Orchestration Anatomy

**Window**: Operational patterns from the 2026-04-25 high-volume session
**Audience**: Future operators planning sessions of comparable scale
**Purpose**: Concrete data on wave composition, collision rates, and operator-orchestrator interaction patterns

This doc complements the economics report (#6757) and meta-learnings (#6761) with **operational anatomy** — what the actual dispatched waves looked like, where parallelism broke down, and how the operator-orchestrator dialogue shaped session output.

---

## Wave-design taxonomy

The session dispatched 7 distinct wave shapes across ~5 hours. Each had a different composition and target:

### Wave 1 — Initial broad sweep (13 agents)
- 1 ops-promote (the 3 stale "merge-ready" PRs)
- 1 promotion sweep (3 near-ready PRs)
- 4 green-CI batches (10 PRs each = 40 PRs)
- 3 diff-audit batches (7 PRs each = 21 PRs)
- 2 needs-ci-fix classification (10 PRs each = 20 PRs)
- 1 needs-diff-fix address (8 PRs)
- 1 label reconciliation (cross-PR drift)

**Outcome**: discovery wave. Identified the master cascade signal, the .hermes contamination problem, the diff-audit coverage gap. Produced the 92-PR-merge-conflict cluster. Cost: large but justified by information yield.

### Wave 2 — Standards/maintainer/reviewer-deep deep (20 agents)
- 2 standards review batches
- 4 maintainer-PR batches
- 4 green-CI continuation batches
- 2 diff-audit completion batches
- 1 needs-ci-fix completion
- 1 needs-diff-fix completion
- 2 ensemble triage agents (parser cluster, misc clusters)
- 1 diff-audit-B finding fixer
- 2 reviewer-deep batches
- 1 master bit-rot scout

**Outcome**: verification depth wave. Most label promotions happened here. Caught the false master-cascade (it was per-PR fmt issues). 8 ensemble closures. Cost-efficient — most agents had narrow scope.

### Wave 3 — Promote+ops drain focused (20 agents)
- 1 ops drain
- 2 bulk-rebase batches (17 conflict-blocked PRs)
- 1 incremental_document fix (turned out to be a no-op)
- 1 UX regression investigation
- 2 green-CI continuation
- 2 maintainer-PR continuation
- 3 reviewer-deep batches (high-value clusters)
- 1 hermes artifact sweep
- 1 aged-PR triage
- 1 editor-docs ensemble
- 1 label correction
- 1 master CI freshness resample
- 1 worktree slot recycling
- 1 diff-audit fresh batch
- 1 promote sweep

**Outcome**: drain-throughput wave. 4 PRs merged. Discovered master root rebuild explains hard-conflict cluster. Cost-effective for closures.

### Wave 4 — Mass triage (20 agents)
- Multiple ensemble closures (lexer/parser/incremental/workspace/refactor)
- Multiple maintainer-PR + standards reviews
- Reviewer-deep on remaining queue

**Outcome**: highest single-wave PR closure count (~38 from lexer/parser ensemble alone). Wave that produced the headline session number.

### Wave 5 — Closure + review focused (5 agents, after policy correction)
- 1 corrective .hermes attribution audit (per user policy clarification)
- 1 ops drain follow-up #2
- 1 promote sweep #3
- 1 reviewer-deep on remaining parser PRs
- 1 reviewer-deep on remaining DAP PRs

**Outcome**: surgical wave. Corrected a prior policy execution error. Added 2 more merges.

### Wave 6 — "Push to 95% session" (5 agents, late session)
- Reviewer-deep on test infra
- Quick-fix agent for #6246/#5938 bare assert!
- Branch contamination verification (the false-positive catch)
- Final ops drain attempt
- Session synthesis + handoff doc generator

**Outcome**: rate-limit-hit wave. Most agents returned BLOCKED quickly. The verification agent produced the most valuable single output (proving maintainer-pr-L was wrong on all 8 PRs).

### Wave 7 — Local-work-only (5 agents, after org-monthly cap)
- Memory consolidation review
- Cargo warning analysis on master
- Worktree cleanup analysis
- Architectural review of perl-symbol boundaries
- Next-session work plan generator

**Outcome**: org cap hit mid-wave. Some agents got through (architectural review produced the perl-symbol clean-boundary verification). Most returned with "You've hit your org's monthly usage limit".

### Direct-orchestrator finishing pass (no agents, 16 gh CLI calls)
After waves and weekly reset, the orchestrator executed the queued label/closure operations directly:
- 6 maintainer-pr-reviewed labels
- 1 needs-diff-fix strip + diagnostic comment
- 2 ci-green labels
- 3 needs-ci-fix labels
- 3 PR closures
- 1 promotion + 1 admin-merge of #5320

**Outcome**: highest tokens-per-merge efficiency of the session. Produced the 11th merge.

### Docs PR finishing pass (no agents, 2 PRs)
- #6757 (4 docs files) → admin-merged
- #6761 (1 docs file) → admin-merged

**Outcome**: 12th and 13th merges. Forensics layer fully populated.

---

## Wave composition heuristics (validated by 7-wave sample)

| Goal | Composition pattern that worked |
|---|---|
| **Discovery** | Many narrow-scope agents, broad coverage, accept some noise |
| **Verification depth** | Multiple verification ladder rungs in parallel + 1-2 ensemble agents |
| **Drain throughput** | Ops + bulk-rebase + promote sweeps + targeted reviewer-deep on near-ready PRs |
| **Mass triage** | Heavy on ensemble + diff-audit + maintainer-PR; light on reviewer-deep |
| **Surgical correction** | 5 agents, each scoped to 1-2 PRs, follow a specific user policy |
| **Burn-budget late session** | Mix of quick-return (CI, label) + slow-return (deep-review on hard PRs) |
| **Local-work-only** | Synthesis, analysis, memory consolidation — no GitHub API needed |
| **Direct CLI finishing** | Mechanical labels/closures with predetermined rationale |

**Operating insight**: the dominant wave shape should match the queue state's dominant problem. Wave 1 was discovery because the queue's structure was unknown. Wave 4 was mass triage because clusters had been identified. Wave 7 was local-only because GitHub was exhausted. Forcing the wrong shape (e.g., dispatching reviewer-deep when the queue is full of hard merge conflicts) wastes tokens.

---

## Agent collision empirics

Across ~80 agents dispatched in the session, observed collisions:

| Type | Count | Example |
|---|---|---|
| Two agents reviewing the same PR with conflicting verdicts | 2 | #5403 (maintainer-pr fixed, deep-review SEND-BACK same issues), #6090 (closed by ensemble, then reviewed by deep-review) |
| Wrong-closure recovered by parallel agent within same wave | 1 | Parser closeout C12 — #5989/#5990 reopened |
| False-positive verdict from cheap-model agent on metadata | 1 | Maintainer-pr-L "branch contamination" on 8 PRs — all CLEAN per REST API |
| Label race (label set then immediately read by another agent) | ~3-5 | Promotion sweeps reading mid-application label state |

**Collision rate**: 4-6% (~3-5 collisions / ~80 agents). The collision *cost* in this sample was low: net zero work loss (recovered by parallel agents or verifier-of-verifier passes).

**Mitigation patterns that worked**:
1. Narrow agent prompts with explicit "skip already-covered: #N1, #N2, #N3" lists (most effective)
2. "Check PR state at start" as standard preamble in reviewer prompts
3. Stagger ensemble vs. reviewer dispatches by ~30s to reduce simultaneous queries

**Mitigation patterns that didn't help**:
1. SendMessage coordination between running agents (added complexity without preventing the collision; more useful for chaining than for collision-avoidance)

---

## Operator-orchestrator dialogue patterns

The session had ~25 user messages. Their distribution reveals operator behavior:

| Intent type | Count | Examples |
|---|---|---|
| Direction (continue/stop/pivot) | 8 | "continue reviewing and improving and merging the prs", "Don't need much ensemble sweeps", "Standing down" |
| Quota telemetry shared by operator | 6 | "74% used 5 hour session", "91% weekly", "0% session 0% week" (later corrected to 100%) |
| Volume control | 5 | "Call another 20", "Call 5 more", "5 more agents" |
| Policy correction mid-execution | 3 | ".hermes only strip if contamination", "if it's just specs for the current pr, then it's just specs and should remain" |
| Coordination request | 2 | "Coordinate the agents so they don't accidentally burn gh rate limits" |
| Specific-PR pointer | 1 | "PR #5455 needs to be finished up" |
| Note for documentation | 4 | "log the economics", "if you see anything interesting in context history, document it" |

**Pattern**: operator behavior was high-engagement / quota-aware / policy-iterative. The operator stayed in the loop, shared real-time quota numbers, and corrected policies as they observed agent behavior. This is **fundamentally different** from a fire-and-forget orchestration model.

**Implication for orchestration design**: the orchestrator should be set up to *receive* policy corrections and propagate them mid-wave (e.g., the .hermes correction needed to update the in-flight hermes-strip agent's behavior, but couldn't because the agent was already dispatched). A "policy bus" that running agents check before destructive actions could prevent this class of post-hoc correction.

---

## The "policy elicited mid-execution" problem (case study)

The .hermes attribution policy was articulated by the operator only **after** the hermes-strip agent had already executed the wrong policy on PR #5870 (deleted 9 .hermes files without checking work-id attribution).

**Sequence**:
1. Wave 4 dispatched: hermes-sweep agent applies `needs-diff-fix` to 8 PRs with `.hermes/` artifacts
2. Hermes-strip agent (same wave) deletes `.hermes/` files from #5870 (the only non-draft target)
3. Operator clarifies policy: "only strip hermes if it's contamination ... if it's just specs for the current pr, then it's just specs and should remain"
4. Wave 5 dispatched: corrective .hermes attribution audit confirms #5870 strip was correct (work-id mismatch = real contamination), but #5750 would have been a false-positive strip (work-id matches branch)

**Cost**: 1 wave dedicated to verification + correction. No real damage — #5870 turned out to be correctly stripped.

**Could have prevented by**: pre-eliciting the policy. A "before dispatching agents that take destructive action on .hermes/, confirm policy: [keep self-attributed | strip all | strip cross-contamination only]" check in the orchestrator's pre-dispatch checklist.

**Generalization**: every recurring policy area (hermes attribution, scope drift threshold, conservative-close criterion, fix-forward authorization) should be elicited or confirmed before the first agent in a wave touches it. Mid-wave policy is too late for already-dispatched agents.

---

## Cross-day narrative arc (Thursday → Friday → Saturday)

Each day in the 3-day arc had a distinct dominant narrative:

| Day | Dominant narrative | What forced it |
|---|---|---|
| Thursday 04-23 | Tier-wiring fix-forward | Backlog of tier-wiring work from prior cycle |
| Friday 04-24 | Throughput + master root rebuild | Aggressive merge cadence + the 12:07 EDT root rebuild as forcing function |
| Saturday 04-25 | PR queue drain via ensemble triage | Accumulated Codex bursts had built up dense duplicate clusters |

**Pattern**: the dominant narrative shifts as the queue state shifts. This is operationally useful — the operator can predict next session's likely narrative by looking at the closing queue state of the prior session. Saturday's closing state (327 open, 2 docs PRs in queue, perl-dap UX cluster #6715 unresolved) suggests next session's likely narrative will be perl-dap correctness investigation + general drain continuation.

**Operating insight**: don't try to force a narrative against the queue state. Saturday session would have failed if dispatched as a Thursday-style "tier-wiring fix-forward" because the queue's actual state was duplicate clusters needing triage.

---

## #5455 partial-supersession discovery

At one point the operator pointed to PR #5455 as something to "finish up". Investigation revealed master commit `5676a2dae` already contained a SUPERSET of #5455's content (master's version: 220 lines; PR's version: 185 lines). The user's IDE was showing the master copy, not the PR copy.

**Pattern**: a forensics doc landed via PR-A can be partially superseded by content added to a sibling PR-B that lands on master with extended content. The PR-A becomes redundant without anyone explicitly closing it. Without a verification step (compare PR's content vs. current master content for that file), it can sit open forever as zombie work.

**Generalization**: for forensics docs and other write-once files, a periodic "is this PR's contribution already on master via a different route?" sweep is worth running.

---

## What this session demonstrated about the orchestration model

Three claims the session validated empirically:

1. **High-volume orchestration is sustainable when the verification ladder stays intact**. The session burned 91% weekly quota and produced 13 merges + 111 closures + 5 forensics docs without any merged PR being later reverted. The cost-effective ratio is real.

2. **Cheap-model agents need expensive verification on high-blast-radius decisions**. Maintainer-pr batch L's "branch contamination" verdict on 8 PRs would have caused mass closures if acted on. A single verifier-of-verifier agent ($N$ tokens, ~5 min) caught the false positive and saved 8 PRs of work.

3. **The orchestrator-as-coordinator + orchestrator-as-executor dual role works**. When sub-agents are blocked, the orchestrator's direct execution of the queued action items is the cheapest possible finisher. The model where the orchestrator is *only* a router and never executes is worse.

---

## Cross-references

Sibling docs from this session:
- `2026-04-25-3day-arc-economics-and-learnings.md` (#6757) — quantitative metrics
- `2026-04-25-process-meta-learnings.md` (#6761) — pattern-level analysis
- `2026-04-25-pr-queue-drain-session.md` (#6757) — Saturday session report
- `2026-04-25-session-final-state.md` (#6757) — closing snapshot
- `2026-04-26-session-priorities.md` (#6757) — next-session work plan

Together these 5 docs form the complete session-end retrospective: economics + patterns + anatomy + state + plan.
