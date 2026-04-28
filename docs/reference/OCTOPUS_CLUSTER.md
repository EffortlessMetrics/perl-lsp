# Octopus Cluster

> For the design philosophy behind the architecture described here, see [ORCHESTRATION_DOCTRINE.md](ORCHESTRATION_DOCTRINE.md).
> For the 7-gate model with skip criteria and triangulation, see [PIPELINE_GATES.md](PIPELINE_GATES.md).
> For the live-truth principle that governs label semantics, see [LIVE_SIGNALS_VS_LABELS.md](LIVE_SIGNALS_VS_LABELS.md).

---

## What Is an Octopus Cluster?

An **Octopus Cluster** is a multi-box, GitHub-native software delivery system where cheap candidate generation runs in parallel, while shared state, receipts, CI, curation, and reconciliation convert that raw variance into trusted merged change.

The name captures the shape: a central GitHub substrate with many agent arms reaching out simultaneously — generating candidates, running verification passes, posting receipts, and routing work. The arms operate in parallel. The substrate holds the shared state.

The crucial distinction that organizes everything else:

> **A PR is a candidate. A merged, reviewed, current-head-green PR is a trusted change.**

The Octopus Cluster's single purpose is to convert candidates into trusted changes — efficiently, verifiably, and in a way that makes the system smarter after each conversion.

This document is the umbrella concept layer. It explains what the cluster is, why it takes the shape it does, and what we're learning from running it. The detailed mechanics live in the reference docs linked above.

---

## Why GitHub Is the Substrate — and Why That Isn't Enough

GitHub provides everything a multi-agent delivery system needs as primitive operations: branches (isolated work surfaces), pull requests (candidate containers), comments (structured communication), reviews and checks (verification proof), labels (bookkeeping), and the merge operation itself (trust landing).

These primitives are enough to coordinate work across many concurrent agents without any external coordination layer. An agent in one worktree can open a PR; a second agent in another worktree can read that PR's labels and decide whether to run a review pass; a third agent can query live CI state and decide whether to merge. No shared in-process state is required. The substrate handles it.

But GitHub's primitives alone are not enough. Three gaps emerge at scale:

**Gap 1: Labels drift.** An agent applies `ci-green`. A new commit pushes. CI turns red. The label does not update itself. Agents reading the label later make decisions on stale state. At low throughput this is an annoyance. At 30+ concurrent PRs it becomes a systematic routing failure.

**Gap 2: Labels contradict.** An agent applies `deep-reviewed`. Another applies `needs-deep-review`. Both exist simultaneously. GitHub has no semantics for "which one wins." Agents disagree about what to do next, and the contradiction accumulates.

**Gap 3: Labels lie for CI.** CI state is a live, queryable signal. A label claiming "CI is green" is not the same thing as CI being green. The label records what was true when an agent checked, not what is true now.

The **reconciler** closes all three gaps. It runs continuously, queries live signals where they exist (CI state, mergeability, conflict status), and derives current routing state from facts rather than accumulated label bookkeeping. This is the move from a label-driven system to a **derived state** system.

The reconciler is not a human. It is automation that continuously strips stale state, resolves contradictions using timeline precedence, and grounds CI-related labels in live CI truth. Agents propose state changes by applying labels. The reconciler disposes: it decides what the current authoritative state actually is.

---

## The Visibility Dividend, Then the Reconciliation Dividend

Running dozens of agents against the same GitHub substrate produces two distinct gains:

### Visibility Dividend

Every agent that posts a comment, applies a label, or writes a review leaves a record that all subsequent agents can read. An accuracy-scout's file-path corrections inform the plan-reviewer. A green-tdd agent's edge case findings inform the deep reviewer. A reviewer's scope concern gets read by the diff-auditor. No agent starts from zero — each one builds on the shared work surface.

This is the visibility dividend: agents make better decisions because they can see what earlier agents found. A system where each agent works in isolation and communicates only through code cannot get this benefit. The shared substrate makes it free.

### Reconciliation Dividend

The visibility dividend has a failure mode: stale visibility. If a green-ci agent's comment from three days ago says "CI is green" but a new commit broke a test since then, subsequent agents reading that comment are misled rather than helped. Stale state is worse than no state — it produces confident wrong decisions.

The reconciliation dividend is what you get by continuously stripping stale state and re-deriving current routing from live signals. After reconciliation runs, agents can trust what they read. The visibility dividend compounds: more agents, longer history, higher confidence — because the reconciler keeps the record current.

The reconciler implementation lives at `xtask/src/tasks/queue_reconciler.rs`. The principle it encodes: **where ground truth exists as a queryable live signal, query it. Labels record agent activity; they do not record what is currently true.** See [LIVE_SIGNALS_VS_LABELS.md](LIVE_SIGNALS_VS_LABELS.md) for the full classification.

---

## Variance Is Search, Not Waste

The canonical example: for a given implementation problem, Codex generates four candidate PRs simultaneously (a 4-shot ensemble). They are not duplicates — they pick different slices of the solution space. One might focus on the core data structure change. Another might add more test coverage. A third might find an adjacent cleanup. The fourth might take a different architectural approach entirely.

This is intentional. The candidates are a parallel search over the design space. The question isn't "which one do we merge?" but rather "what does the best solution look like, and which PR is closest to it?"

The vocabulary:

- **Ensemble**: intentional multiple candidates for one design item
- **Winner**: the selected candidate — closest to the right solution, often improved further during review
- **Losers**: non-selected candidates — they are closed, but not discarded

**Loser harvest** is the value-preservation mechanism. Before closing a losing candidate, the curator extracts:
- Tests the winner didn't include
- Edge cases the winner didn't cover
- Ideas worth filing as follow-up issues
- Alternative approaches worth documenting

The losing PR's diff is a research artifact. Harvesting it before closure is how variance converts to value even for the candidates that don't merge. In practice this means a winning PR often gets 2-3 additional tests cherry-picked from losing PRs before it merges.

The anti-pattern to avoid: treating ensemble generation as accidental duplication and closing all but one without reading the others. That discards the search value. Ensemble diversity is cheap to generate and expensive to recreate from scratch — harvest it.

---

## CI as Scoped Proof

Every credible candidate gets a CI pass. This is **frontdoor proof**: the minimum viable verification that establishes whether a candidate is worth curating at all. Frontdoor proof is:

- **Scoped**: it runs against the PR's actual blast radius, not a wide-but-shallow generic check
- **Deep within scope**: within the selected scope, it is thorough — not a surface skim
- **Fast enough to run on every candidate**: measured in minutes, not hours

The "scoped and deep" framing matters. A CI check that runs wide-and-shallow (touches everything, verifies nothing deeply) provides weak signal. A check that runs within the blast radius of the actual change and verifies that blast radius thoroughly provides strong signal. The goal is to fail bad candidates quickly and cheaply, not to run a big-but-thin validation theater.

After curation — after the ensemble is sorted, winners are selected, and losers are harvested and closed — **survivor-level verification** runs on the curated set:

- Mutation testing (verifies test quality, not just coverage)
- Long-form fuzzing (verifies robustness against unexpected inputs)
- Full CPAN corpus parse (verifies parser changes don't regress the real-world baseline)
- Broad platform soak (verifies Windows/Linux/Mac behavior consistency)

Survivor-level verification is expensive. It runs only on candidates that have already passed frontdoor proof and passed curation. Running it on every candidate would cost 10-50x more and provide the same signal — because most candidates that fail frontdoor proof are caught by much cheaper checks.

---

## Receipts + Reconciler = Derived State

An agent completes a review pass and applies a label: `deep-reviewed`. This label is a **receipt** — machine- and human-readable proof that the agent performed its gate against a specific PR at a specific HEAD SHA.

Receipts have two properties that make them useful:

1. **They are auditable**: any subsequent agent can query the label history and know that a specific pass happened. The trail is transparent.
2. **They age**: a receipt applied against HEAD SHA `abc123` does not prove anything about HEAD SHA `def456`. A new commit invalidates CI-related receipts. The reconciler detects this and strips or flags stale receipts.

The **reconciler** is the engine that converts facts + receipts into current queue state. It does not trust labels at face value. It:

- Queries live CI for each PR's current HEAD SHA
- Compares live CI to `ci-green`/`needs-ci-fix` labels (stripping stale ones)
- Applies timeline precedence for no-live-signal labels (later applied wins when contradictions exist)
- Computes which PRs are routing-blocked, review-blocked, or clear for merge

**Derived state** is the output of this process: routing state computed from facts and receipts, not from accumulated manual bookkeeping. When operators query "which PRs are merge-ready?", they are querying derived state. When agents ask "has this PR been deep-reviewed?", the answer comes from derived state.

The phrase "agents propose; the reconciler disposes" captures the division of labor. Agents focus on their gate: do the review, post the finding, apply the signoff. The reconciler handles the messy bookkeeping: contradiction resolution, staleness detection, authoritative state derivation.

See [LIVE_SIGNALS_VS_LABELS.md](LIVE_SIGNALS_VS_LABELS.md) for the precise classification of which labels have live ground truth and how each is treated.

---

## The Trust Conveyor (Gates)

The sequence that converts candidates into trusted changes is called the **Trust Conveyor**. It is organized into 7 gates — coarse stages, each with a clear exit condition and multiple agents working within it.

The 7 gates in brief:

| Gate | Name | What it establishes |
|------|------|---------------------|
| 1 | Identify | Accurate, builder-ready problem statement |
| 2 | Spec | Scoped, project-aligned proposed approach |
| 3 | Build | Well-tested, implemented PR |
| 4 | Review/improve | Three-axis verification (right thing × codebase needs × right way) |
| 5 | CI green | Live CI actually green on current HEAD SHA |
| 6 | Merge | Changes land on master |
| 7 | Learn | Captured learning consolidated into durable artifacts |

Key properties of the gate model:

- **Gates are coarse; agents are the workers within them.** One gate may have 5 agents. A trivial docs PR may skip 3 gates entirely. The gate framing captures the *kind* of work, not the exact number of steps.
- **Three-axis triangulation in Gate 4** is what makes review trustworthy. Multiple agents cross-check: building the right *thing* (matches user intent), building what the *codebase needs* (matches architecture and project direction), and building it *right* (correct, idiomatic, regression-safe). A PR that clears one axis but fails another does not merge.
- **Gate 7 is mandatory.** Learning captured throughout all gates is consolidated here into durable memory, doctrine updates, and follow-up issues. Skipping Gate 7 under throughput pressure degrades the system — it is where the conveyor improves itself.

See [PIPELINE_GATES.md](PIPELINE_GATES.md) for the full 7-gate model with skip criteria, within-gate sequencing rules, and worked examples.

---

## What We're Gaining

Running an Octopus Cluster over several months of intensive development has produced observable, measurable gains across five dimensions.

### Bad PRs fail cheaply

Frontdoor proof catches structural problems before expensive review passes run. In practice: a PR with a hallucinated API (Codex generating code against a module that doesn't exist) fails `cargo check` in 2 minutes and gets closed with a note. The same problem discovered after a full review cycle would cost 10-15 agent-minutes to identify and address.

The ensemble pattern amplifies this: running frontdoor proof on 4 candidates simultaneously takes the same wall-clock time as running it on 1. Bad candidates in the ensemble are identified and closed cheaply. Only credible candidates advance to curation.

An example from this session: the master fmt cascade fix (#7090) was discovered because a cluster of 12 apparently-unrelated PRs failed identically at the same step. This is the fingerprint of a master bit-rot incident — not per-PR failure but infrastructure failure that affects many unrelated PRs simultaneously. Identifying this pattern and treating it as infrastructure downtime (fix master once, cascade the fix to all blocked PRs) is far cheaper than investigating 12 PRs independently.

### Good PRs merge safely

Three-axis review in Gate 4 catches problems that single-axis review misses. In a documented incident, PR #5543 reached full signoff under a `docs:` title carrying 24 changed files and 3082 additions spanning UX, async, and code-actions — because each reviewing agent checked only one axis. The title-vs-diff scope rule in the diff-auditor closes this gap by adding a structural check that no other agent in the gate performs.

Multi-axis triangulation means a PR that passes one reviewer's check is not assumed to be safe. It has to pass all three axes. This raises the bar in a targeted way: the three axes are specifically designed to catch the failure modes that single-axis review historically missed.

### Duplicates close cheaply

Ensemble curation is the mechanism: read all diffs, identify the winner, harvest tests and ideas from losers, close losers with cross-references. The economics work because reading a diff is cheap (a few seconds per file, automated tooling assists), while re-implementing a missed edge case after merge is expensive.

A practical measure: in one session, 90+ PR closures and 66+ issue closures happened cleanly. The key insight was sorting candidates by file path (not by title), which surfaces structural similarity even when titles diverge. Two PRs touching the same files are likely handling the same problem.

### Master stays healthy

The reconciler + master bit-rot incident handling keeps the trunk from degrading. Two mechanisms:

First, the reconciler continuously strips stale labels that would cause agents to route incorrectly — a PR incorrectly labeled `ci-green` when CI is actually red would flow toward merge without this check.

Second, master bit-rot incidents are treated as infrastructure events rather than per-PR failures. In the session that produced the fmt cascade fix (#7090), a formatting rule change broke `cargo xtask fmt` for a large number of PRs simultaneously. The correct response was: identify the pattern (same failure across N unrelated PRs), fix master once, run `gh pr update-branch` to cascade the fix to all affected PRs. Not: investigate each PR independently.

The test panic blocker fixed in #7031 is another example: a test introduced in an earlier PR was failing in a way that silently blocked subsequent PRs. Identifying this as a master-level issue (not a per-PR issue) and fixing it in isolation unblocked the entire downstream queue.

### The system learns every cycle

Gate 7 is not optional. Every agent-wrapup produces a memory candidate. The wisdom agent consolidates memory candidates into durable MEMORY.md entries. Those entries inform future agents' prompts. The loop is: work → learning artifact → consolidated memory → better future agent behavior.

The structured comment trail is the carrier: `## Lesson:` and `## Pattern:` sections in PR comments create a searchable audit trail. Future agents reading a PR can see not just what happened, but what was learned from it. This is how the system accretes knowledge across sessions without requiring human synthesis at each step.

---

## Anti-Patterns We've Abandoned

Running the system at scale has produced a clear list of patterns that seemed reasonable but produce bad outcomes in practice.

**Stale next-step labels as routing primitives.** Labels like `needs-deep-review` seem useful but become toxic at scale. They are applied and not stripped. They accumulate. An agent routing based on "which PRs have `needs-deep-review`?" sees the set of PRs that ever had this label, not the set that currently need deep review. The reconciler replaces this pattern: route based on which receipt is *missing*, not on which routing label is *present*.

**"Wide but shallow" CI.** A CI check that touches 200 files but only verifies surface-level compilation provides less signal than a check that runs the full test suite for the specific crate changed. The "wide but shallow" pattern seems thorough but it's expensive theater. Scoped-deep CI is cheaper, faster, and produces stronger signal.

**One-axis review.** Any single agent checking any single dimension is not sufficient for a feature PR. An agent checking "is this correct Rust?" cannot tell you whether it's implementing the right feature. An agent checking "does this match the spec?" cannot tell you whether it introduces a regression. The three-axis triangulation requirement is not bureaucracy — it is the minimum structure to catch what single-axis review historically missed.

**Agents as label janitors.** Agents manually managing label contradictions (checking for conflicting labels, deciding which to strip) is a dead end. Each agent does it differently, none do it consistently, and the problem recurs after every agent pass. The reconciler handles this. Agents focus on their gate; they do not manage label hygiene.

---

## Terminology Reference

These terms should be used consistently across all orchestration documentation. When a term from this list appears in a document, it should carry exactly the meaning defined here.

| Term | Definition |
|------|------------|
| **Octopus Cluster** | The multi-box, GitHub-native delivery system: parallel candidate generation + shared-substrate verification + reconciled merge |
| **Substrate** | GitHub itself — branches, PRs, comments, reviews, labels, checks, SHAs, issues |
| **Trust Conveyor** | The 7-gate sequence that converts candidates into trusted changes |
| **Candidate** | A PR that has been generated but not yet verified — a possibility, not a trusted change |
| **Trusted Change** | A merged, reviewed, current-head-green PR — verified through the full conveyor |
| **Receipt** | Machine- and human-readable proof that a gate was performed against a specific HEAD SHA |
| **Reconciler** | Automation that converts GitHub facts + receipts into current queue state; owns derived state |
| **Derived State** | Queue state computed from facts, receipts, and live signals — not from manually applied labels |
| **Missing-Proof Routing** | Route PRs based on which receipt is absent for their gate + risk profile, not on which routing label is present |
| **Visibility Dividend** | The gain agents get from seeing the shared work surface — each subsequent agent builds on prior work |
| **Reconciliation Dividend** | The gain from continuously stripping stale state and re-deriving current routing — the visibility dividend without the staleness failure mode |
| **Ensemble** | Intentional generation of multiple candidates for one design item (typically 4-shot) |
| **Winner** | The selected candidate in an ensemble — the closest to the right solution, often improved during review |
| **Loser** | A non-selected ensemble candidate — closed after harvest |
| **Loser Harvest** | Extracting tests, ideas, edge cases, and alternative approaches from losing candidates before closure |
| **Frontdoor Proof** | The first CI pass on every credible candidate — scoped to the PR's actual blast radius, deep within that scope |
| **Scoped-Deep CI** | CI that is targeted to the changed crate/component and thorough within that scope (contrast: wide-but-shallow) |
| **Survivor-Level Verification** | Expensive checks (mutation, long fuzz, full corpus, platform soak) that run only on curated survivors post-curation |
| **Dirty Tail** | The expensive remainder of large queues: stale, conflicted, partially reviewed PRs — often still valuable via salvage |
| **Master Bit-Rot Incident** | A trunk failure affecting many unrelated PRs simultaneously — treat as infrastructure downtime, not per-PR failure |
| **Maintainer-Orchestrator** | The human role: doctrine, exception handling, economics tuning, deciding when repeated failure becomes automation |

### When to use which term

- **"Octopus Cluster"** — the umbrella system. Use when describing the overall architecture or explaining what we're doing to someone new.
- **"Trust Conveyor"** — the gate sequence specifically. Use when describing how PRs move from candidate to trusted change.
- **"Reconciler"** — the specific automation component. Use when describing label cleanup, derived state, or the `queue_reconciler.rs` implementation.
- **"Derived State"** — the output of reconciliation. Use when emphasizing that routing state is computed, not manually maintained.
- **"Frontdoor Proof"** — the first CI pass on a candidate. Use to distinguish from survivor-level verification.
- **"Master Bit-Rot Incident"** — a trunk failure pattern. Use specifically when N unrelated PRs fail identically, indicating infrastructure-level failure.

### Preferred phrasing for common patterns

| Instead of... | Prefer... |
|---------------|-----------|
| "the pipeline" | "the Trust Conveyor" or "the gate sequence" (more precise) |
| "the agent" | "the [agent-name] agent" (precision prevents confusion at scale) |
| "it's in review" | "it's in Gate 4 review" or "review-reviewed is pending" (cites the state precisely) |
| "CI passed" | "frontdoor proof passed" (candidate) or "CI is green on current HEAD" (live check) |
| "we have duplicate PRs" | "we have an ensemble; triage for winner and harvest losers" |
| "close the stale PRs" | "classify for salvage, harvest value, then close" |
| "the label says X" | "derived state is X" (emphasizes reconciler derivation) or "the label records X" (emphasizes it's an audit trail) |

---

## Reading Order

For someone new to the system:

1. **This document** — what the system is and why it takes this shape
2. [ORCHESTRATION_DOCTRINE.md](ORCHESTRATION_DOCTRINE.md) — the design philosophy and specific failure modes that motivated each direction
3. [PIPELINE_GATES.md](PIPELINE_GATES.md) — the 7-gate model with skip criteria, worked examples, and routing logic
4. [LIVE_SIGNALS_VS_LABELS.md](LIVE_SIGNALS_VS_LABELS.md) — the live-truth principle and its implications for every label in the system
5. [CLAUDE.md](../../CLAUDE.md) — the operational reference: specific agents, labels, routing queries, and commands

Each subsequent document is more detailed and more tactical than the one before. Reading in order builds context before detail.
