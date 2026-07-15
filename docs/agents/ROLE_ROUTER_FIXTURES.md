# Role Router Fixtures

These examples are stable routing fixtures. They describe invocation shape, not
current issue, PR, branch, model, or portfolio state.

## Parent coordination fixture

Request:

```text
Coordinate the three supplied work items, compare their current GitHub and repository
evidence, challenge the plans, and return one next-transition recommendation for each.
Do not edit concurrently.
```

Expected route: parent orchestrator.

The parent reconstructs live state, preserves contradictions, dispatches bounded
read-only passes where useful, synthesizes their evidence, and decides the next
transition. It does not treat a remembered queue or subagent report as authority.

## Bounded worker fixture

Packet:

```text
Issue: supplied issue
Objective: review the named current PR head for the declared parser seam
Read scope: named files, linked spec, current PR, required checks
Write scope: none
Proof: cite exact-head findings and commands
Stop: return if the head changes or the named seam is outside scope
```

Expected route: bounded worker.

The worker reads only the packet's scope, performs the requested review, returns
concise evidence and uncertainty, and does not select unrelated work or delegate.

## Single-agent control fixture

Request:

```text
Fix the documented typo in the supplied file and report the focused diff.
```

Expected route: single-agent execution. No subagent is needed when isolation or
independence would not improve the result.
