# Orchestrator Substrate Model

## The thesis

An autonomous agent fleet's throughput is gated by the **orchestrator's model of the
operating substrate**, not by agent quality or code difficulty. Agents optimize *within*
the model they're given. They cannot self-correct the model.

When the model is wrong, the fleet thrashes — not because the agents are bad, but because
they are faithfully executing instructions built on false assumptions about their environment.

## What "the substrate" means

The substrate is everything the fleet runs on top of:

- **CI timing** — how long checks take, which checks are required vs advisory, what
  "skipping" means (satisfied, not skipped)
- **Cache semantics** — warm-lane TTL, what sharing happens across invocations
- **Merge-queue and branch-protection rules** — strict-up-to-date requirements, queue
  enabled/disabled, admin-merge behavior
- **Concurrency model** — how many threads are active, whether main moves while one PR
  is in flight, whether actions on one PR affect others
- **Config correctness** — base refs, required check names, workflow trigger conditions

None of these are visible in the code. They are environmental facts that the orchestrator
must hold as a model.

## Failure modes from a wrong substrate model

Each wrong model produces a characteristic failure pattern:

| Wrong model | Symptom |
|-------------|---------|
| "main is stable while I work a PR" (multi-thread repo) | Futile serialization; held PRs rot while main advances anyway |
| "merge promptly to keep velocity" | Rapid-merge starvation: PRs bump each other behind, CI restarts, none reaches green |
| "this check failed, re-run it" | Cancelled-run cascade; check was stale not incorrect |
| "cached tokens cost the same as fresh" | 10x over-spend on cold-spawn agents |
| "the CI gate base ref is origin/master" (main repo) | Exit-128 on every CI run; repo-wide stall |
| "mid-CI update-branch is safe" | In-progress CI cancelled; PR restarts from zero |
| "a long-running lane is still on the right task" | Work finishes into a changed world; results are stale |

In each case the agent behavior is locally rational, given the model. The failure is the
model, not the agent.

## Why agents cannot self-correct the model

An agent can observe its own task, read code, run tests. It cannot observe:

- Whether another thread merged something while it was running
- Whether the CI it sees is green on the current SHA or a stale prior run
- Whether the merge queue was disabled or re-enabled
- Whether the "required checks" list changed
- Whether the base ref it's diffing against is the right one for this repo

These are *external* facts about the system configuration that require a vantage point
the agent doesn't have. The orchestrator holds that vantage point — but only correctly
if its substrate model is accurate.

## The human's role: model correction

In a human-plus-fleet system, the substrate model is implicitly held by the human
operator. When the fleet thrashes, the human's irreplaceable contribution is diagnosing
which model assumption is wrong, correcting it, and encoding the correction so the
next orchestrator inherits it.

This is different from code review (which the agents can do), spec clarification
(which the agents can do), or bug fixing (which the agents can do). Substrate-model
correction requires environmental knowledge that is structurally unavailable to the
agents.

The signal that a human correction is needed is not "the code is wrong" — it is
"the agents are thrashing despite doing correct work."

## The compounding tax

The substrate tax is paid on **every change by every thread**. A wrong base ref costs
CI time on every PR. A wrong concurrency model wastes every PR that gets held. A cold
spawn instead of a warm lane charges 10x tokens for every invocation.

This is why the deepest shift-left is shifting the substrate-model correction left:
encode each correction as it is discovered, so the next orchestrator inherits it rather
than re-thrashing on the same wrong model.

Individual-change optimizations (a faster builder, a tighter spec) save once. Substrate
fixes save on every change that follows.

## Investing in the substrate

The highest-leverage investments in an agentic system are substrate improvements:

- **CI speed** — reduces the per-PR tax on every agent in every thread
- **Merge-queue configuration** — correct strict-up-to-date + queue settings remove the
  rebase treadmill entirely for slow-CI repos with high-velocity main
- **Config correctness** — wrong base refs, stale check names, and incorrect required-set
  definitions have repo-wide blast radius; fix them first
- **The learning loop** — each human substrate correction should become a durable encoded
  rule, not a one-off fix; this is the mechanism by which the fleet learns

A fleet with high-quality agents on a bad substrate will underperform a fleet with
average agents on a well-understood substrate.

## Relation to other patterns

- **Serialize merges** (`serialize-merges-and-cancellation.md`) — one specific substrate
  model failure: rapid merges cancel each other. The fix is a pacing rule encoded in
  the substrate model, not agent-level behavior.
- **Cache-aware agent lanes** (`cache-aware-agent-lanes.md`) — another substrate fact
  (cache TTL, warm/cold cost ratio) that the orchestrator must model correctly.
- **Human corrects substrate** (`human-corrects-substrate.md`) — the division of labor:
  agents do domain work; the human corrects the model of the environment.
- **Re-create over untangle** (`re-create-over-untangle.md`) — a tangle is often caused
  by a wrong concurrency-model assumption (two agents, one branch); encoding the
  one-owner rule is a substrate fix.
