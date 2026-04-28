# Orchestration Glossary

This document is an **index**, not the source of truth. Each entry points to the reference
doc that defines the term in full. When definitions here conflict with a reference doc,
the reference doc wins.

Reference docs (reading order for someone new to the system):

1. [OCTOPUS_CLUSTER.md](OCTOPUS_CLUSTER.md) — umbrella framing: what the cluster is and why it takes this shape
2. [ORCHESTRATION_DOCTRINE.md](ORCHESTRATION_DOCTRINE.md) — design philosophy and the specific failures that motivated each direction
3. [PIPELINE_GATES.md](PIPELINE_GATES.md) — 7-gate model: skip criteria, within-gate sequencing, worked examples
4. [LIVE_SIGNALS_VS_LABELS.md](LIVE_SIGNALS_VS_LABELS.md) — live-truth principle and per-label classification
5. [CLAUDE.md](../../CLAUDE.md) — operational reference: agents, labels, routing queries, commands

---

## C

### Candidate

A PR that has been generated but not yet verified through the full conveyor. A possibility,
not a trusted change. The distinction matters: candidates may be wrong, incomplete, or
structurally unsound — that is expected and cheap to fail at this stage.

*Example: four Codex PRs for the same issue are four candidates. One becomes a trusted change.*

See [OCTOPUS_CLUSTER.md — Candidate vs Trusted Change](OCTOPUS_CLUSTER.md)

---

### Conveyor / Trust Conveyor

The 7-gate sequence that converts candidates into trusted changes. Organized into coarse
stages (gates), each with a clear exit condition and multiple agents working within it.
Variance-tolerant: trivial PRs skip irrelevant gates; complex PRs run the full sequence.

See [OCTOPUS_CLUSTER.md — The Trust Conveyor](OCTOPUS_CLUSTER.md) | [PIPELINE_GATES.md](PIPELINE_GATES.md)

---

## D

### Derived State

Queue state computed from facts, receipts, and live signals — not from accumulated manual
label bookkeeping. The reconciler produces derived state by querying live CI for each PR's
current HEAD SHA, resolving label contradictions via timeline precedence, and computing
which PRs are routing-blocked, review-blocked, or clear for merge.

*Example: "PR #7031 is merge-ready" is derived state. The raw inputs are live CI (green),
labels (deep-reviewed, diff-audited), and merge conflict status (none).*

See [OCTOPUS_CLUSTER.md — Receipts + Reconciler = Derived State](OCTOPUS_CLUSTER.md)

---

### Dirty Tail

The expensive remainder of large queues: stale, conflicted, or partially-reviewed PRs.
Default behavior is to classify for salvage (rescue cost vs. reimplementation cost) rather
than close-by-default. The tail is expensive, but it often contains value that is cheaper
to extract than to regenerate.

See [OCTOPUS_CLUSTER.md — Terminology Reference](OCTOPUS_CLUSTER.md)

---

## E

### Ensemble

Intentional generation of multiple candidates for one design item — typically 4 shots from
an external-agent burst (Codex, etc.). Candidates in an ensemble are not duplicates; they
pick different slices of the solution space. The ensemble is a parallel search over the
design space.

Related terms: **winner**, **loser**, **loser harvest**, **layer-diverse sibling**.

See [OCTOPUS_CLUSTER.md — Variance Is Search, Not Waste](OCTOPUS_CLUSTER.md)

#### Winner

The selected candidate in an ensemble. Closest to the right solution; often improved
further by cherry-picking tests and ideas from losers before merge.

#### Loser

A non-selected ensemble candidate. Closed after harvest. Not discarded — the diff is a
research artifact.

#### Loser Harvest

Extracting tests, edge cases, ideas, and alternative approaches from losing candidates
before closure. Ensures that variance converts to value even for candidates that don't
merge.

#### Layer-Diverse Sibling

An ensemble PR that touches a different layer of the stack than the winner (e.g., one PR
fixes the parser, a sibling fixes the LSP layer). Both may be valid; sort by file path
before calling "duplicate."

---

## F

### Frontdoor Proof

The first CI pass on every credible candidate. Scoped to the PR's actual blast radius
and thorough within that scope. Fast enough to run on every candidate (minutes, not hours).

Contrasted with **survivor-level verification**, which is expensive and runs only on
curated survivors post-curation.

*Example: `cargo test -p perl-parser` on a parser PR is frontdoor proof. The full mutation
test suite and CPAN corpus parse are survivor-level.*

See [OCTOPUS_CLUSTER.md — CI as Scoped Proof](OCTOPUS_CLUSTER.md)

---

## G

### Gate

A coarse stage in the Trust Conveyor with a clear entry condition, exit condition, and
one or more agents working within it. Gates may be skipped when not relevant to a given
PR's nature. Agents within a gate collectively satisfy the gate's exit condition; no single
agent sign-off is sufficient.

The 7 gates:

| Gate | Name | Purpose |
|------|------|---------|
| 1 | Identify | Accurate, builder-ready problem statement |
| 2 | Spec | Scoped, project-aligned proposed approach |
| 3 | Build | Well-tested, implemented PR |
| 4 | Review/improve | Three-axis verification |
| 5 | CI green | Live CI actually green on current HEAD SHA |
| 6 | Merge | Changes land on master |
| 7 | Learn | Captured learning consolidated into durable artifacts |

See [PIPELINE_GATES.md](PIPELINE_GATES.md) | [ORCHESTRATION_DOCTRINE.md — Gates are coarse stages; agents work within them](ORCHESTRATION_DOCTRINE.md)

---

## L

### Label Signal

A label applied by an agent to record that a gate pass occurred. Labels are bookkeeping
for agent activity — they record what an agent did, not what is currently true. For
state that has live ground truth (CI, mergeability), the live signal takes precedence.

Contrasted with **live signal**.

See [LIVE_SIGNALS_VS_LABELS.md](LIVE_SIGNALS_VS_LABELS.md)

---

### Live Signal

Ground truth that is queryable directly from the GitHub API at any moment: CI status
(`statusCheckRollup`), mergeability (`mergeStateStatus`), conflict status, diff content.
Where a live signal exists, it supersedes any label's claim about the same state.

*Example: live CI red overrules a `ci-green` label applied to an earlier HEAD SHA.*

See [LIVE_SIGNALS_VS_LABELS.md](LIVE_SIGNALS_VS_LABELS.md)

---

## M

### Maintainer-Orchestrator

The human role in the Octopus Cluster: doctrine, exception handling, economics tuning,
deciding when repeated failure becomes automation. Not a code reviewer in the traditional
sense — the agent pipeline handles review. The orchestrator routes work, calibrates
economics, and handles the decisions agents cannot make autonomously.

See [OCTOPUS_CLUSTER.md — Terminology Reference](OCTOPUS_CLUSTER.md)

---

### Master Bit-Rot Incident

A trunk failure affecting many unrelated PRs simultaneously. The diagnostic fingerprint:
N apparently-independent PRs failing identically at the same CI step. Correct response:
treat as infrastructure downtime — fix master once, cascade the fix to all blocked PRs
via `gh pr update-branch`. Not: investigate each PR independently.

*Example: a formatting rule change breaks `cargo xtask fmt` for 30+ unrelated PRs on the
same day. Fixing the rule in a single master PR unblocks the queue.*

See [OCTOPUS_CLUSTER.md — Master Stays Healthy](OCTOPUS_CLUSTER.md)

---

### Missing-Proof Routing

Route PRs based on which receipt (signoff label) is absent for their gate and risk profile,
not on which routing label (`needs-*`) is present. Routing labels accumulate and become
stale; missing receipts tell you exactly what gate work is still needed.

*Example: "which PRs are in Gate 4 review?" → query for PRs that have `plan-reviewed`
(Gate 2 done) and `builder-ready` converted to a PR, but are missing `review-reviewed`.*

See [OCTOPUS_CLUSTER.md — Terminology Reference](OCTOPUS_CLUSTER.md)

---

## O

### Octopus Cluster

The multi-box, GitHub-native delivery system: parallel candidate generation + shared-
substrate verification + reconciled merge. The name captures the shape: a central GitHub
substrate (the body) with many agent arms reaching out simultaneously. See also:
**substrate**, **trust conveyor**, **reconciler**.

See [OCTOPUS_CLUSTER.md](OCTOPUS_CLUSTER.md)

---

## R

### Receipt

Machine- and human-readable proof that a gate was performed against a specific PR at a
specific HEAD SHA. Applied as a GitHub label by the agent that completed the pass.
Receipts have two properties: they are auditable (any subsequent agent can query the trail)
and they age (a receipt against `abc123` does not prove anything about `def456`).

*Example: `deep-reviewed` is a receipt that reviewer-deep completed its correctness check.*

See [OCTOPUS_CLUSTER.md — Receipts + Reconciler = Derived State](OCTOPUS_CLUSTER.md)

#### SHA-Bound Receipt

A receipt whose validity is tied to the HEAD SHA at the time it was applied. CI-related
receipts (`ci-green`) are SHA-bound: a new commit invalidates the prior receipt. Non-CI
receipts (e.g., `deep-reviewed`) are not invalidated by new commits automatically —
the reconciler uses timeline precedence for those.

---

### Reconciler / Reconciliation Dividend

**Reconciler**: Automation that converts GitHub facts and receipts into current derived
queue state. Owns contradiction resolution (later-applied wins for no-live-signal labels),
staleness detection (strips stale CI labels based on live CI), and authoritative state
derivation. Implementation: `xtask/src/tasks/queue_reconciler.rs`.

**Reconciliation Dividend**: The gain from continuously stripping stale state and
re-deriving current routing from live signals. Without it, the visibility dividend
degrades into "confident wrong decisions from stale context." With it, agents can trust
what they read on the shared substrate.

See [OCTOPUS_CLUSTER.md — Reconciliation Dividend](OCTOPUS_CLUSTER.md) | [LIVE_SIGNALS_VS_LABELS.md](LIVE_SIGNALS_VS_LABELS.md)

---

## S

### Scoped-Deep CI

CI that is targeted to the changed crate or component and thorough within that scope.
Contrasted with wide-but-shallow CI, which touches many files but verifies nothing deeply.
Scoped-deep CI is cheaper, faster, and produces stronger signal. See **frontdoor proof**.

See [OCTOPUS_CLUSTER.md — CI as Scoped Proof](OCTOPUS_CLUSTER.md)

---

### Substrate

GitHub itself: branches, PRs, comments, reviews, checks, labels, SHAs, issues. The shared
surface that allows many concurrent agents to coordinate without any external coordination
layer. An agent in one worktree opens a PR; a second agent reads its labels; a third merges
it. No shared in-process state is required.

See [OCTOPUS_CLUSTER.md — Why GitHub Is the Substrate](OCTOPUS_CLUSTER.md)

---

### Survivor-Level Verification

Expensive verification checks that run only on candidates that have already passed
frontdoor proof and curation: mutation testing, long-form fuzzing, full CPAN corpus parse,
broad platform soak. Running these on every candidate would cost 10-50x more for the same
signal — most bad candidates are caught by cheaper frontdoor proof.

See [OCTOPUS_CLUSTER.md — CI as Scoped Proof](OCTOPUS_CLUSTER.md)

---

## T

### Three-Axis Triangulation

Gate 4's multi-agent review structure. A PR must pass all three axes to proceed:

- **Axis 1 — Right thing**: matches user/issue intent (reviewer, maintainer-pr)
- **Axis 2 — What the codebase needs**: matches project direction and architecture
  (architecture-reviewer, refactor-planner)
- **Axis 3 — Right way**: correct, idiomatic, regression-safe (reviewer-deep, diff-auditor,
  green-tdd)

A PR that clears one axis but fails another is not trustworthy and does not merge.
The triangulation exists because single-axis review historically missed the other two.

See [PIPELINE_GATES.md — Three-Axis Triangulation](PIPELINE_GATES.md) | [ORCHESTRATION_DOCTRINE.md — Three-axis triangulation in Gate 4 review](ORCHESTRATION_DOCTRINE.md)

---

### Trusted Change

A merged, reviewed, current-head-green PR — verified through the full conveyor. The
opposite of a candidate. Every step in the Trust Conveyor exists to make the conversion
from candidate to trusted change reliable and auditable.

See [OCTOPUS_CLUSTER.md — What Is an Octopus Cluster?](OCTOPUS_CLUSTER.md)

---

## V

### Visibility Dividend

The gain agents get from seeing the shared work surface. Each agent builds on prior work:
an accuracy-scout's file-path corrections inform the plan-reviewer; a green-tdd agent's
edge case findings inform the deep reviewer. No agent starts from zero.

The failure mode is stale visibility: agents reading outdated information and making
confident wrong decisions. The **reconciliation dividend** is what keeps the visibility
dividend trustworthy.

See [OCTOPUS_CLUSTER.md — The Visibility Dividend](OCTOPUS_CLUSTER.md)
