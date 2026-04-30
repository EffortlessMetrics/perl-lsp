# Orchestration Doctrine

> **This doc is the north star for orchestration design decisions. When an implementation pass hits ambiguous decisions, resolve by reading this doc.**
>
> For the umbrella concept and vocabulary — what an Octopus Cluster is, why variance is search not waste, and what we're gaining — see [OCTOPUS_CLUSTER.md](OCTOPUS_CLUSTER.md).

This document captures the mentality, direction, why, and design behind the perl-lsp orchestration model. The tactical roadmap issues are implementation phases; this doc captures the principles those tactics serve. It is a reference doc, not a how-to.

---

## Mentality

> The conveyor is a variance-tolerant, self-improving loop, not a processor.

Three implications:

1. **Variance-tolerant**: PRs vary widely in nature — trivial fmt-fix to architecture-shifting feature. The conveyor adapts; it does not impose one-size-fits-all process. A 1-line cleanup does not pay the same gate cost as a 3000-line feature.

2. **Self-improving**: every cycle leaves the system smarter. Throughput matters; so does learning. The system that only produces output without producing learning degrades over time.

3. **Loop, not processor**: feedback flows back into doctrine, agents, skills, and tooling — not just into the next PR. Learning captured in Gate 7 shapes the next Gate 1.

The opposite of this mentality:
- Rigid pipeline that runs every gate on every PR regardless of nature
- Process that produces output without producing learning
- Tooling that requires constant manual care to stay accurate

If a proposed change reinforces the opposite, it is the wrong change.

---

## Direction

We are moving toward:

### 1. Live truth where it exists, labels for the rest

CI status, mergeability, conflict state, and diff content are all queryable as live signals. Labels are bookkeeping for agent activity, not state machines that compete with live truth. Where live ground truth exists, query it — do not duplicate it into labels.

See [LIVE_SIGNALS_VS_LABELS.md](LIVE_SIGNALS_VS_LABELS.md) for the full classification and implications.

### 2. Gates are coarse stages; agents work within them

The pipeline is gates (coarse stages) with multiple agents working within each gate. Sequencing within a gate is preferred because each agent reads the prior, but sequencing is not strict. Some gates may be skipped when not relevant to a given PR's nature.

The six gates:

| Gate | Purpose | Agents (within-gate) |
|------|---------|----------------------|
| **1. Identify** | Get the issue right — file an accurate, builder-ready problem statement | scout, accuracy-scout, research-verifier |
| **2. Spec** | Accurate, scoped, project-aligned proposed approach | plan-reviewer, oppositional-planner, advocatus-diaboli, architecture-reviewer, maintainer-issue, spec-planner |
| **3. Build** | Well-tested and implemented PR | red-tdd, builder, green-tdd |
| **4. Review/improve** | Right thing × what codebase needs × right way | reviewer, maintainer-pr, refactor-planner, green-refactor, reviewer-deep, diff-auditor |
| **5. CI green** | Live CI actually green (not just the label) | green-ci, pr-responder (iterations) |
| **6. Merge** | Land it | ops |

A seventh layer (Gate 7 — learning consolidation) is cross-cutting: learning happens throughout all gates, and Gate 7 consolidates captured artifacts into durable memory, doctrine, and follow-up work.

### 3. The reconciler is the authoritative label-state engine

Agents stop manually managing label contradictions. The reconciler:
- Grounds CI-pair decisions in live state (where live truth exists)
- Uses GitHub timeline for no-live-signal labels ("later applied wins")
- Is the only thing that authoritatively strips labels

Agents propose; the reconciler disposes.

### 4. Learning happens continuously throughout all gates

Gate 7 is the dedicated consolidation layer — it shapes captured artifacts into durable memory, doctrine, and follow-up work. It is not where learning starts. Every gate captures learning; Gate 7 makes it durable.

Skipping Gate 7 "because we're busy" degrades the system. It is not optional.

### 5. Three-axis triangulation in Gate 4 review

Multiple agents within Gate 4 cross-check three axes:

- **Building the right *thing*** — matches user/issue intent (reviewer, maintainer-pr)
- **Building what the *codebase needs*** — matches project direction and architecture (architecture-reviewer if Gate 2 was thin, refactor-planner)
- **Building it *right*** — correctness, idiomatic, regression-safe (reviewer-deep, diff-auditor, green-tdd)

A PR that clears one axis but fails another is not trustworthy and does not merge. The triangulation is what makes the conveyor produce trustworthy output.

### 6. Automation over manual care

If agents have to remember to do X, eventually X will be forgotten. Automate the bookkeeping; reserve agent attention for novel decisions. Crons, reconcilers, and CI gates encode the invariants so human attention can focus on judgment.

---

## Why Each Direction Came From a Specific Failure

Each direction is the converse of an observed failure, not a speculative improvement:

| Direction | Failure that motivated it |
|-----------|---------------------------|
| Live truth over labels | #5365/#5353 sat 4 days on stale `needs-deep-review` while CI was actually green — label drift blocked a valid merge |
| Gates over strict sequencing | Trivial fmt-fix PRs paid the full 6-agent verification ladder cost because the linear framing made "skip the irrelevant pass" feel like rule-breaking |
| Reconciler as authoritative | Agents reporting label changes that didn't land (~80% silent-failure on one cluster); contradictions accumulating with no cleanup path |
| Learning throughout | A 24h master blocker (test panic from PR #5985) wasn't surfaced because no agent had a structured way to flag "this test is broken" between sessions |
| Three-axis triangulation | PR #5543 reached full 4-signoff with title `docs:` and 24 files / 3082 additions across UX/async/code-actions because each agent checked only one axis |
| Automation over manual care | Multiple sessions losing time to label drift that a 15-min cron reconciler eliminates |

Each failure is documented as a memory entry or a closed/active issue. The directions are not speculative.

---

## Design

### Shape: gates with agents, three axes, two layers of state

```
                  ┌─── Live truth (CI, mergeable, diff) ───────┐
                  │                                              │
Issue → Gate 1 → Gate 2 → Gate 3 → Gate 4 → Gate 5 → Gate 6 → Gate 7
        Identify  Spec    Build    Review   CI      Merge    Learn
                                   ↓↓↓
                                 Three axes:
                                 - right thing
                                 - what codebase needs
                                 - right way
                  │                                              │
                  └─── Bookkeeping (sign-off labels) ───────────┘
                       ↓
                  Reconciler (authoritative, grounds in live truth)
```

### Principles embedded in the shape

- **Gates** are coarse stages, not atomic steps. Skip when not relevant to the PR's nature.
- **Agents within gates** triangulate. Multiple agents per gate, each covering different ground.
- **Live truth** is queried, not duplicated into labels.
- **Labels** record agent activity. They are informational; the reconciler keeps them clean.
- **Reconciler** is the only thing that authoritatively strips labels. Agents propose; reconciler disposes.
- **Gate 7** consolidates captured learning. Learning itself is continuous throughout all gates.

### Anti-patterns this design forbids

- Agents using labels as a state machine when live truth exists
- One agent's signoff being treated as gate-complete (triangulation requires multiple agents within the gate)
- Skipping Gate 7 under throughput pressure (it is where the conveyor improves)
- Treating sequencing within a gate as strict (it is a preferred default, not a rule)
- Adding more agents to compensate for gaps (often the right answer is to remove a noisy agent, not add another)
- Agents manually managing label contradictions instead of delegating to the reconciler

---

## Tactical Work

The tactical issues that implement this doctrine, grouped by theme:

**Gate model documentation and implementation:**
- Gate-model docs: introduce the 6-gate structure in `docs/reference/` and update CLAUDE.md
- Gate-model implementation: make routing logic gate-aware (skip irrelevant gates by PR nature)

**Reconciler maturity:**
- Reconciler PR1: queue-wide contradiction reconciler with live-signal grounding
- Reconciler roadmap: subsequent phases (cron frequency, observability, typed contradiction rules)

**Label hygiene:**
- Deprecate `needs-*` family after reconciler is stable
- Split `needs-builder-fix` into typed labels (reconciler-derived, per known failure patterns)

**Three-axis review:**
- Diff-audit title-vs-diff-size scope rule (closes a 3-axis gap where a docs-titled PR carries code)
- CI receipt fidelity improvements

**Infrastructure:**
- Add `merge_group` trigger (closes the "merge into red master" hole)
- Salvage-classify skill (typed routing for stale/dirty PRs)

**Continuous learning:**
- Continuous-learning-capture roadmap (how Gate 7 consolidation works at scale)
- Conveyor metrics and observability (measuring learning throughput, not just code throughput)

---

## What This Doctrine Is NOT

- Not a unit of implementation work — it is the design rationale behind implementation work
- Not a vague aspiration — every direction is grounded in an observed failure
- Not immutable — see the "How to Update" section below

---

## How to Update This Doctrine

Doctrine evolves as the system encounters new failure modes and resolves old ones. The process:

1. **File an issue** describing the proposed change to a direction or design principle. The issue must link to the specific failure mode that motivates the change — doctrine changes without a concrete failure story are speculative and should be rejected.

2. **Get sign-off from the maintainer.** Doctrine changes affect every downstream agent's behavior. A maintainer sign-off ensures the change is intentional and scoped.

3. **Update both this doc AND issue #7084**, which stays open as the design conversation log. Issue #7084 is where the rationale accumulates; this doc is where the current state lives. Both must be updated together — the issue is not a substitute for the doc, and the doc is not a substitute for the issue trail.

Additions are preferred over removals. If a direction's failure mode disappears, the direction may be marked "superseded by X" rather than deleted — the history of why it existed remains useful.

---

## Memory References

This doctrine consolidates and elevates:

- `project_conveyor_doctrine` — the six conveyor principles (variance-tolerant, self-improving, loop-not-processor, bad-fail-cheaply, good-merge-safely, system-learns)
- `feedback_labels_as_state_machine` — labels as resumable state machine (refined here: labels record agent activity; live truth takes priority)
- `feedback_live_signals_vs_label_signals` — live truth over labels (expanded in [LIVE_SIGNALS_VS_LABELS.md](LIVE_SIGNALS_VS_LABELS.md))
- `feedback_take_judgment_on_verdicts` — synthesis over voting (verification agents are lenses, not votes)
- `feedback_orchestrator_follows_pipeline` — never bypass the gates, but skip gates when not relevant to a PR's nature
