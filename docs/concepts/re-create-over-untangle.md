# Re-Create Over Untangle

## The pattern

When a branch has accreted changes from multiple agents or multiple rounds of review
feedback, the branch often becomes a tangle: commits from different intents interleaved,
merge conflicts resolved in ways that mixed concerns, review feedback applied on top of
other review feedback until the diff no longer cleanly represents any single intention.

In this situation, re-creating the change fresh from the spec is often cheaper and
safer than untangling the existing branch.

## Signals that re-creation is cheaper

- The branch has commits from more than one agent (different agents wrote different
  parts of the fix, or one agent applied the fix and another applied the tests).
- Cherry-picking the "right" commits would require non-trivial conflict resolution,
  because the commits touch overlapping files in inconsistent ways.
- The accumulated diff contains changes that belong to this PR AND changes that drifted
  in from adjacent concerns (a reviewer suggestion that went further than the spec, a
  convenience fix added while the agent had the file open, a test extracted to a common
  module that now pulls in unrelated refactoring).
- The CI results on the branch are unreliable because intermediate commits left the
  build in an inconsistent state.
- It is no longer clear which commits represent the intended change and which are
  workaround-on-top-of-workaround.

## How to re-create

1. Read the original spec (the issue body, the plan-reviewer comment, the acceptance
   criteria). Not the accumulated PR body, which may have drifted from the spec.
2. Create a new branch from the current main.
3. Implement the spec cleanly, without carrying over any code from the tangled branch.
   Treat the tangled branch as reference only -- it shows what approaches were tried,
   which may save time, but it should not be cherry-picked wholesale.
4. Write the tests first (or in parallel), per the spec acceptance criteria.
5. Open a new PR citing the original issue and noting that it supersedes the tangled PR.
6. Close the tangled PR with a comment linking to the replacement.

## The one-owner principle

The root cause of a tangled branch is almost always one of:

- **Multiple agents, one branch**: two or more agents checked out the same branch and
  each committed their own changes. Even if each agent changes were individually correct,
  the interleaving is incoherent.
- **Successive round-trips**: the PR went through many rounds of "reviewer finds a
  problem, builder applies a fix" without a clean re-read of the full diff after each
  round. Accumulated fixes often undo each other or introduce new inconsistencies.

The structural fix is the one-owner principle: at any given time, one agent owns one
branch. A branch is handed off from one agent to the next (with an explicit handoff
message), not concurrently worked by multiple agents. A reviewer that finds a problem
hands the branch back to the builder; the builder re-reads the full spec and fixes
forward, not just the pointed-at line.

## Tradeoff / caution

Re-creation has a fixed cost: the builder must re-read the spec, re-implement the
change, and re-run verification. This cost is well-defined and bounded.

Untangling a complex branch has an uncertain cost: it may take multiple rounds of
cherry-picks and conflict resolutions, each of which can introduce new inconsistencies.
The cost is unbounded in the worst case.

Re-creation is the correct choice when the tangled branch complexity is high (many
interleaved commits, large diff, multiple unrelated concerns). Untangling is the correct
choice when the tangle is shallow (one or two commits, a clear separation of concerns,
minimal conflict risk).

Never re-create a branch by force-pushing to the existing branch name. Create a new
branch. This preserves the history of the tangled branch for reference and avoids
confusing anyone who had the old branch checked out.

## Relation to other patterns

- **Serialize merges** (serialize-merges-and-cancellation.md) -- serialization prevents
  the concurrent-agent scenario that causes tangles; re-creation is the recovery path
  when serialization was not applied.
- **Cache-aware agent lanes** (cache-aware-agent-lanes.md) -- a lane pattern with clear
  handoffs between agents prevents tangles; the lane single-owner structure is the
  preventive equivalent of re-creation curative path.

## 2026-06 refinement

Re-create is a **salvage threshold**, not a preference. The decision criterion: if branch state is
more expensive to understand than to recreate, recreate. Untangle when history contains useful
REASONING (prior approaches, root-cause analysis, edge-case discovery); recreate when the branch
mostly contains contaminated MECHANICS (interleaved commits, conflict resolutions that mixed
concerns, workaround-on-top-of-workaround). The artifacts that matter — the spec, the tests, the
patch, the proof, the learning — can be extracted from a tangled branch without carrying the
contaminated mechanics forward. A re-created branch is not a loss of history; it is a promotion of
the useful artifacts into a clean implementation.
