# Serialize Merges and Cancellation

## The pattern

In a CI system that cancels superseded runs, concurrent merges or rebases on the same
branch cause a cancellation cascade: each push triggers a new CI run, which cancels the
still-running CI for the previous push.

The fix is to serialize: finish one CI cycle completely before starting the next. The
merge queue handles one change at a time; the next change does not enter the queue until
the first change CI checks have completed and it has been merged (or rejected).

## Why it matters

Cancellation cascades have two concrete costs:

1. **Wasted CI compute**: every cancelled run consumed machine time and produced no
   usable artifact.
2. **Stale check results**: a check that completed on a cancelled run may appear "green"
   in the UI but is associated with a superseded commit SHA. The green label on the PR
   does not reflect the current HEAD.

The second cost is the more dangerous one. A stale green label can allow a bad state
to be merged. The correct mental model is: CI results are live-truth signals tied to a
specific SHA, not persistent labels that survive subsequent pushes.

## The cascade mechanism

1. PR A and PR B are both merge-ready and both get rebased or merged approximately
   simultaneously.
2. Both pushes trigger new CI runs on the same CI runner pool.
3. The CI system cancellation policy causes one run to be cancelled when the other starts.
4. The external reporting step (e.g., a coverage upload, a notification webhook) never
   completes for the cancelled run.
5. The external system records a failure for the cancelled run, even though the code
   was correct.

This pattern is especially common for reporting steps that have their own timeouts
independent of the main CI job.

## The fix

Enforce a strict merge cadence: one change per CI cycle.

1. Merge one change. Wait for all required CI checks to complete (verified against the
   merge commit SHA, not a stale label).
2. Only after all checks are green on the merge commit, merge the next change.
3. If the CI system has a native merge queue feature, use it. If not, implement the
   cadence manually by not rebasing the next PR until the current one has merged.

For reporting steps that are particularly prone to cancellation (e.g., coverage uploads
that require several minutes to complete after the build finishes), add a gap between
consecutive merges large enough for the reporting step to complete.

## Tradeoff / caution

Serialization reduces throughput. In a low-volume change stream (a few PRs per day),
the cost is negligible. In a high-volume stream (dozens of PRs per hour), serialization
may become a bottleneck.

In high-volume streams, the correct response is not to abandon serialization but to
address the root cause: why are there so many concurrent merge-ready changes? The answer
is usually either a batch-review workflow that approves many PRs simultaneously, or a
fast-merging ops agent that does not wait for CI completion. Both are addressable at the
process level.

Do not add artificial sleeps to work around cancellation. Sleeps hide the symptom
without fixing the cause and make the pipeline harder to reason about. Use the monitor
pattern (wait for a specific completion signal) instead of polling.

## Parallel builds, paced merges

Serialization does not mean single-threaded work. The correct model is:

**Build in parallel** — multiple agents working different changes simultaneously is
correct and efficient, especially in multi-crate or independent-feature repos. Parallel
builds do not cancel each other's CI because they target different branches.

**Merge in paced batches** — merging is the step that causes CI cascades. Merge one
small batch (e.g., 3 PRs), wait for all required checks to be green on the merged SHAs,
then merge the next batch. Never rebase a PR while its CI run is in progress — that
cancels the in-progress run and restarts from zero.

The anti-pattern is rapid-fire merging: merging several PRs in quick succession causes
each merge's CI run to be cancelled by the next merge, leaving all of them stuck at
partial green. None reaches the required check count, and the queue stalls.

Concrete rules:
1. Never update-branch (rebase) a PR while its CI is running (checks < required green)
2. Merge in batches of 3 or fewer; wait for all required checks before the next batch
3. Do not race all open PRs simultaneously toward merge; focus on one priority at a time

## The config tension: high-velocity main + strict-up-to-date + slow CI

In a repo where:
- many independent changes merge rapidly (high-velocity main)
- branch-protection requires the PR's base to be up-to-date before merge
- CI is slow (15–30+ minutes)

...a slow PR's CI run completes while main has advanced, the branch is now "behind,"
and the PR must be rebased — which cancels the just-completed CI run. This is the
rebase treadmill: a PR can cycle indefinitely if main moves faster than CI completes.

The structural fix is not more serialization — it is configuration:

- **Enable the merge queue**: the queue handles up-to-date enforcement automatically,
  merging PRs in sequence without requiring manual rebase
- **Relax strict-up-to-date**: allow merging a PR that is slightly behind main (N commits)
  when all required checks are green — the risk is small and bounded; the throughput
  gain is large

Either fix removes the treadmill at the config layer. Manual serialization (holding
PRs to keep main still) is futile in a multi-thread repo where main advances
regardless.

## Relation to other patterns

- **Cache-aware agent lanes** (cache-aware-agent-lanes.md) -- independent concern;
  lanes are about cost optimization, serialization is about correctness.
- **Re-create over untangle** (re-create-over-untangle.md) -- if a branch became
  tangled through multiple concurrent agents, serialization would have prevented the
  tangle; re-creation is the recovery path.
- **Orchestrator substrate model** (`orchestrator-substrate-model.md`) -- rapid-merge
  starvation and the rebase treadmill are substrate failure modes caused by a wrong
  model of CI timing and branch-protection semantics; the config tension above is the
  substrate-level diagnosis.
