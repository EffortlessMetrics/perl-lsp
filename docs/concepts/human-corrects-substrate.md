# Human Corrects Substrate

## The division of labor

In a human-plus-agent-fleet system, two fundamentally different kinds of work happen
in parallel:

**Domain work** — code analysis, spec writing, test design, implementation, review.
Agents do this well. It is describable, verifiable, and improvable by iteration.

**Substrate-model correction** — diagnosing when the fleet's model of its own
operating environment (CI, cache, merge-queue, config, concurrency) is wrong, and
correcting it. This is the human's irreplaceable contribution.

The distinction is not about skill level. It is structural: agents cannot observe the
system configuration that their tasks run inside. The human can.

## The signal to escalate

The signal that substrate correction is needed is not "the code has a bug." It is:

> "The agents are doing correct work, and the fleet is still thrashing."

When agents write tests that pass, build implementations that compile, submit PRs that
look right — and the fleet still spins without making progress — the problem is almost
certainly not in the domain work. It is in the model of the environment.

Common patterns:

- Agents repeatedly hold work "to keep the base stable," but the base keeps moving
  anyway → wrong concurrency model
- Agents keep re-running checks that keep failing, despite correct code → stale CI
  result, wrong required-check name, or cancelled run
- Warm-lane cost savings never materialize despite using lanes → cache TTL assumption
  wrong or lane is idle beyond the window
- PRs keep getting rebased into CI re-runs → wrong model of when rebasing is safe

In each case, the correct human action is not "help the agent with the code" — it is
"correct the model, then encode the correction."

## Encoding the correction

A substrate correction has two steps:

1. **Patch the model** — tell the current session what is actually true
   ("main moves regardless; stop serializing on stillness"; "100k cached tokens cost
   10k fresh tokens; warm the lane"; "mid-CI rebase cancels the run; wait for green first")

2. **Encode the correction durably** — write the corrected model into the repo as a
   rule, a doc, or a config change, so the next orchestrator session inherits it
   rather than re-thrashing on the same wrong assumption

Step 1 without step 2 is a one-time fix that costs the same next session. Step 2 is
what makes the correction compound: every subsequent agent and every subsequent session
benefits without paying the discovery cost again.

## Why the human holds unique leverage here

An agent can verify that a function returns the right value. It cannot verify:

- Whether the required checks list changed since the PR was opened
- Whether the merge queue was disabled by a user action
- Whether the base ref is `origin/main` or `origin/master` for this particular repo
- Whether CI is genuinely green on the current HEAD SHA or a stale prior run
- Whether the cache window was already exceeded before a lane invocation

These require environmental context that the agent structurally lacks. The human, who
can look at the GitHub UI, the repo settings, the CI dashboard, and prior session
history simultaneously, holds the vantage point needed for diagnosis.

This is the human's highest-leverage contribution in an agentic system — not code
review (agents do that), not spec writing (agents do that), but the periodic "is my
model of the environment right?" diagnostic that the agents cannot perform on themselves.

## The recurring check

The human should ask "is the fleet's substrate model still accurate?" as a standing
check at the start of each long session, and whenever the fleet is thrashing without
apparent cause.

The questions to ask:

1. Has CI configuration changed since the last session? (required checks, base refs,
   workflow triggers)
2. Has branch-protection changed? (strict-up-to-date, merge-queue enabled/disabled)
3. Has the merge-queue behavior changed? (queue enabled, admin-merge rules)
4. Are any model assumptions from the last session still correct? (concurrency model,
   cache economics, timing)
5. What did the fleet thrash on last session? (Are those corrections encoded, or will
   they need to be re-discovered?)

## Relation to other patterns

- **Orchestrator substrate model** (`orchestrator-substrate-model.md`) — the full
  framing: fleet throughput is gated by the substrate model, not agent quality; this
  doc gives the human-side role in that model
- **Model conformance** (`model-conformance.md`) — when multiple agents have divergent
  substrate models, the conformance discipline finds the inconsistency; the human then
  decides which model to promote and encodes it
- **Shift-left ladder** (`shift-left-ladder.md`) — encoding substrate corrections is
  a form of shift-left for process hazards: the correction moves from "re-discovered
  each session" to "inherited by the next orchestrator"
