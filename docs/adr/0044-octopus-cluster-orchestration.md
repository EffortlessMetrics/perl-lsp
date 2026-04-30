# ADR-0044: Octopus Cluster Orchestration Architecture

**Status**: Accepted
**Date**: 2026-04-28
**Related**: [ADR-0033](0033-worktree-first-disposable-workers.md), [OCTOPUS_CLUSTER.md](../reference/OCTOPUS_CLUSTER.md), [ORCHESTRATION_DOCTRINE.md](../reference/ORCHESTRATION_DOCTRINE.md), [PIPELINE_GATES.md](../reference/PIPELINE_GATES.md), [LIVE_SIGNALS_VS_LABELS.md](../reference/LIVE_SIGNALS_VS_LABELS.md), [FAILURE_MODES.md](../reference/FAILURE_MODES.md)

---

## Context

### The Problem

perl-lsp receives PR candidates from multiple concurrent sources — Codex bursts, Jules, Claude
agents, and internal swarm workers. At low volume, the bottleneck is generation. At the volume
this project operates, the bottleneck shifted to curation: distinguishing candidates worth
landing from those that should be harvested-and-closed, identifying contradictions in label
state, and verifying that CI is actually green — not merely labeled green — before merge.

Three gaps emerged that neither a linear pipeline nor a bare GitHub-labels system handles:

1. **Label drift**: an agent applies `ci-green`, a new commit pushes, CI turns red, and the
   label does not update itself. Subsequent agents make routing decisions on stale state.

2. **Label contradiction**: `deep-reviewed` and `needs-deep-review` exist simultaneously on
   the same PR. GitHub has no semantics for which wins.

3. **Label vs live truth for CI**: the label records what was true when an agent checked.
   The GitHub API can answer what is true *now* for the current HEAD SHA. These are not
   the same thing.

Additionally, task state does not survive across sessions or across boxes. When dozens of agents
are running in parallel, there is no shared in-process state: each agent sees only what GitHub
exposes. The coordination substrate must be durable, queryable, and multi-writer by design.

---

## Decision

We adopt the **Octopus Cluster** model: a GitHub-native, multi-agent delivery architecture
where candidate generation is cheap and parallel, shared state lives in GitHub primitives
(PRs, comments, checks, labels, SHAs), and a reconciler derives authoritative routing state
from live signals rather than accumulated label bookkeeping.

### 1. GitHub is the coordination substrate

GitHub branches, pull requests, check runs, comments, labels, and the merge operation are
the primitives. Agents coordinate by reading and writing these — not by sharing in-process
state. This makes the system durable across sessions and boxes.

No external coordination layer is required. An agent in one worktree can open a PR. A second
agent in another worktree can read that PR's labels and decide whether to run a review pass.
A third can query live CI state and decide whether to merge. The substrate handles it.

### 2. Routing state is derived from substrate facts, not labels alone

Labels record what agents did. They are sign-off receipts, not authoritative ground truth for
CI state, mergeability, or conflict status. A reconciler runs continuously to:

- Strip stale labels when the underlying condition changed (new commit after `ci-green`)
- Resolve contradictions using timeline precedence
- Ground CI-related labels in live CI truth (current HEAD SHA checks, not prior run results)

Agents propose state changes by applying labels. The reconciler disposes: it decides what the
current authoritative routing state actually is.

### 3. The pipeline is gates, not a strict linear sequence

The pipeline is organized into **7 coarse gates** with multiple agents working within each:

| Gate | Purpose | Key agents |
|------|---------|-----------|
| **1. Identify** | Accurate, builder-ready problem statement | scout, accuracy-scout, research-verifier |
| **2. Spec** | Scoped, project-aligned proposed approach | plan-reviewer, oppositional-planner, advocatus-diaboli, architecture-reviewer, maintainer-issue, spec-planner |
| **3. Build** | Well-tested, implemented PR | red-tdd, builder, green-tdd |
| **4. Review/improve** | Right thing × what codebase needs × right way | reviewer, maintainer-pr, refactor-planner, green-refactor, reviewer-deep, diff-auditor |
| **5. CI green** | Live CI actually green (not just a label) | green-ci, pr-responder |
| **6. Merge** | Land it | ops |
| **7. Learn** | Consolidate captured learning into durable artifacts | wisdom, memory-recalibrator |

Sequencing within a gate is preferred when agents build on each other's output. Parallel
agents within a gate are fine when they do not depend on each other. Gates may be skipped when
not relevant to a PR's nature (a 1-line fmt fix skips Gates 1 and 2).

Gate 4 applies three-axis triangulation: right thing (does it solve the actual problem),
what the codebase needs (fits architecture and style), right way (implementation correctness).
Agents in Gate 4 read each other's output — sequencing matters here more than in other gates.

### 4. Live signals beat labels where ground truth exists

CI status, mergeability, conflict state, and diff content are queryable as live signals. Where
live ground truth exists, the live signal is authoritative. Labels are bookkeeping for agent
activity, not state machines that compete with live truth.

`statusCheckRollup` for the current HEAD SHA is the authoritative CI answer. `ci-green` is
informational — it records that green-ci ran a pass, not that CI is green right now.

### 5. Variance is search; ensemble curation extracts winners

High-volume candidate generation from multiple sources is not a problem to eliminate — it is
a search strategy. Each burst of 3-5 candidates per issue surfaces approaches that a single
agent would not generate. The curation pipeline (Gate 4) extracts winners, harvests edge cases
and tests from near-misses, and closes duplicates with cross-references.

The bottleneck is curation, not generation. Capping fanout to reduce duplication misidentifies
the constraint.

### 6. Learning is continuous; Gate 7 consolidates

Every agent in every gate captures learning artifacts when something novel is encountered.
Gate 7 (Learn) is the dedicated consolidation layer: it shapes captured artifacts into durable
memory, updated doctrine, and follow-up issues. The system improves between cycles, not just
within them. An orchestration model that only produces output without producing learning
degrades over time.

---

## Consequences

### Positive

- **Scales across boxes and sessions**: all coordination state lives in GitHub; no shared
  in-process memory is required. Agents in different worktrees, different machines, and
  different sessions coordinate correctly.
- **Self-improving**: Gate 7 consolidation + continuous per-agent learning means the system
  gets better at routing, curation, and building over time.
- **Durable audit trail**: every agent pass leaves a record (comment, label, check) that all
  subsequent agents can read. Accuracy scouts inform plan-reviewers. Reviewer findings inform
  deep reviewers. No agent starts from zero.
- **Cheap variance becomes trusted change**: the curation pipeline converts high-volume
  candidates into a small number of trusted merges, without requiring expensive manual triage.
- **Operator capacity redirected**: the operator's attention shifts from manual coordination
  (routing, status tracking, labeling) to architecture and exception handling. Routine pipeline
  mechanics run without intervention.

### Negative

- **Vocabulary surface area**: the model introduces a nontrivial shared vocabulary (gates,
  agents-within-gates, reconciler, live-signals-vs-labels, three-axis triangulation, variance-
  as-search). New contributors and agents must load this vocabulary before they can reason
  about routing decisions correctly.
- **Reconciler infrastructure required**: the live-signals-vs-labels principle only works if
  the reconciler runs continuously. An unmaintained or idle reconciler degrades the system
  back toward the label-drift failure mode.
- **Operator must hold the architecture in mind**: the gate model is coarse enough to be
  navigable, but the orchestrator must understand which agents belong in which gate, what the
  skip criteria are, and when to deviate from the default sequence. The system does not enforce
  this mechanically.

### Neutral

- **Does not replace human review**: the architecture redirects human attention from manual
  coordination to architecture and exception handling. Human judgment remains in the loop for
  structural decisions, unusual PRs, and exception cases.
- **Gate sequencing is advisory**: gates may be skipped and within-gate sequencing is
  preferred but not strict. This gives flexibility but places more responsibility on the
  orchestrator to make the right skip/parallel calls.

---

## Alternatives Considered

### Strict linear pipeline

**Rejected.** A fixed sequence of agents on every PR regardless of nature is rigid and
wasteful. A 1-line fmt fix should not pay the same gate cost as a 3000-line feature. More
importantly, a strict linear model does not adapt when agents' conclusions contradict each
other — it has no mechanism for handling the label-contradiction or label-drift failure modes.

### Agent-managed labels as the authoritative state machine

**Rejected.** Labels rot. Contradictions accumulate. CI-related labels compete with live CI
truth. A system where labels are the authoritative state machine — not just bookkeeping for
agent activity — degrades over time as stale labels accumulate and agents make decisions based
on outdated state. Operating the perl-lsp pipeline at scale demonstrated this failure mode
directly. The reconciler-plus-live-signals design is the response.

### Bare GitHub with no orchestration layer

**Rejected.** GitHub's primitives are necessary but not sufficient. The three gaps identified
in the Context section (label drift, label contradiction, label vs live truth for CI) do not
self-correct without automation that queries live signals and derives current routing state.
Using GitHub without the reconciler and gate model produces the failure modes at scale, not
just occasionally.

### Capping fanout to reduce duplicate PRs

**Rejected.** Fanout reduction addresses the wrong constraint. The bottleneck is curation, not
generation. Cheap candidate generation via Codex/Jules/Claude bursts is a search strategy:
each burst surfaces approaches that a single agent would not generate. Limiting fanout reduces
search without proportionally reducing curation cost. The correct lever is better curation
(ensemble-detect, cluster-triage, hallucination-check), not smaller bursts.

---

## References

- [OCTOPUS_CLUSTER.md](../reference/OCTOPUS_CLUSTER.md) — umbrella concept: what the cluster is, why variance is search, what we're gaining
- [ORCHESTRATION_DOCTRINE.md](../reference/ORCHESTRATION_DOCTRINE.md) — mentality, direction, and north star for orchestration design
- [PIPELINE_GATES.md](../reference/PIPELINE_GATES.md) — full gate model: skip criteria, within-gate ordering, three-axis triangulation
- [LIVE_SIGNALS_VS_LABELS.md](../reference/LIVE_SIGNALS_VS_LABELS.md) — live-truth principle and label classification audit table
- [FAILURE_MODES.md](../reference/FAILURE_MODES.md) — catalog of recurring failure patterns the architecture is designed to prevent
- [ADR-0033](0033-worktree-first-disposable-workers.md) — disposable workers and worktree isolation (the execution model this architecture routes into)
- Issues [#7084](https://github.com/EffortlessMetrics/perl-lsp/issues/7084), [#7079](https://github.com/EffortlessMetrics/perl-lsp/issues/7079), [#7071](https://github.com/EffortlessMetrics/perl-lsp/issues/7071), [#7078](https://github.com/EffortlessMetrics/perl-lsp/issues/7078) — related design issues
