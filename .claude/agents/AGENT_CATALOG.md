# Agent Catalog

The swarm orchestrator calls agents. The agents are part of the swarm rather
than an optional historical sidecar.

Agent files carry the role-specific operating context for each lane:

- who the agent is
- what lane or responsibility it owns
- how it should communicate
- what its startup and active todo list should contain

Those todo items then call the specific skills or commands that hold the
step-by-step procedure for each phase of the work.

Keeping the mechanical substep instructions in skills matters operationally:
the agent can load the relevant skill when that substep becomes relevant
mid-run instead of forcing the orchestrator or agent file to front-load every
procedural detail up front.

That is the actual split:

- agent files define role, context, ownership, and expected todo shape
- skills define the reusable, load-on-demand instructions for the steps in
  those todos
- the orchestrator routes work through those agents instead of having to
  restate each role from scratch every time

This catalog documents the main swarm roster:
- 5 core coordinators (scout, builder, reviewer, ops, improver)
- 7 reusable workers (bootstrapper, fixer, validator, pr-responder, research-*)
- 40 specialist workers across implementation, quality, review, explore, scout,
  and docs/devex categories

Older archived agent-iteration directories elsewhere under `.claude/` are
also intentionally kept for historical comparison and analysis.
