# ADR-0038: Session Economics for Swarm Wind-Down Policy

**Status**: Accepted
**Date**: 2026-03-19
**Related**: [ADR-0033](0033-worktree-first-disposable-workers.md), [ADR-0032](0032-skill-scoping-and-hook-enforcement.md)

---

## Context

During cycle 5 (2026-03-19), a broadcast shutdown to 117 agents consumed 6% of
the orchestrator's context window. Each agent sent an acknowledgment message,
flooding the orchestrator with 117 inbound messages that provided no value. The
session nearly exhausted its context budget before final memory writes and status
checks could complete.

This incident exposed a gap in the swarm operating model: the lifecycle cost of
agent communication was not accounted for. Spawning, messaging, idling, and
stopping agents each have different token costs, and the wrong wind-down strategy
can waste significant context at exactly the moment it is most scarce.

---

## Decision

**Adopt explicit session economics: budget context like a finite resource, prefer
idle over stop, and never broadcast shutdown.**

### 1. Agent lifecycle cost model

Each agent lifecycle state has a distinct cost to the orchestrator:

| State | Orchestrator Cost | Notes |
|-------|-------------------|-------|
| **Spawn** | Cheap (seconds, minimal tokens) | One outbound message to create |
| **Running** | Moderate (tokens per message exchange) | Each send/receive consumes context |
| **Idle** | Zero | No messages = no context consumption |
| **Shutdown** | 1 outbound + 1 inbound per agent | Acknowledgment floods orchestrator |
| **Restart** | Expensive (full context re-establishment) | Lost state must be rebuilt from scratch |

The key insight: idle agents are free. They consume zero orchestrator context
when not messaged. Stopped agents may later incur restart cost if work resumes.
Shutdown messages are pure waste near session end.

### 2. Session budget model

The orchestrator context window is finite (~1M tokens for Opus). Every message
exchange consumes a portion of that budget:

- **Broadcasts** cost N messages (one per teammate). A broadcast to 100 agents
  costs 100 outbound + up to 100 inbound acknowledgments.
- **Scout reports** can be 1,000+ lines each. Pulling 10 scout reports into
  orchestrator context can consume 50,000+ tokens.
- **Status polls** (asking each agent for progress) scale linearly with agent
  count.
- **Memory writes** and **final status checks** at session end require reserved
  context.

Rule: be strategic about what enters orchestrator context. Not every agent result
needs to be pulled in. Use issues and PRs as out-of-band handoff artifacts.

### 3. Wind-down policy

When approaching session limits:

1. **Do NOT broadcast shutdown.** This is the single most important rule. A
   broadcast to N agents costs 2N messages (N out + N ack) for zero operational
   benefit.
2. **Stop sending new messages.** Agents idle naturally when they receive no
   work. Idle costs nothing.
3. **Let session end terminate agents.** Session termination cleans up all agents
   automatically with zero context cost.
4. **Only explicitly stop agents that are actively polling or looping.** An agent
   stuck in a retry loop will keep sending messages; these are the only ones
   worth stopping.
5. **Reserve context for final writes.** Memory updates, status checks, and
   handoff notes are high-value uses of remaining context. Do not waste budget
   on ceremony.

### 4. Scaling dynamics

Operational limits that inform session budgeting:

| Dimension | Limit | Constraint |
|-----------|-------|------------|
| Optimal coding agents | ~9 | Merge queue is 3-wide, each PR needs ~5 min CI |
| Maximum useful agents | ~50 | Diminishing returns above this count |
| Platform ceiling | ~75 teammates | Hard roster limit |
| Context ceiling | Variable | Depends on message volume, not agent count |

Agent count and context consumption are only loosely correlated. Ten chatty
agents can consume more context than fifty quiet ones. The binding constraint is
message volume, not headcount.

### 5. Anti-patterns

| Anti-pattern | Cost | Alternative |
|--------------|------|-------------|
| Mass shutdown broadcast | 2N messages wasted | Let agents idle |
| Polling all agents for status | N messages per poll | Check PRs/issues instead |
| Pulling full scout reports into orchestrator | 1,000+ lines per report | Have scouts write issues |
| Restarting stopped agents | Full context rebuild | Keep agents idle instead of stopping |
| Broadcasting non-critical updates | N messages for FYI | Only message agents that need to act |

---

## Consequences

### Positive

- **Context preserved for high-value operations.** Final memory writes, status
  checks, and handoff notes get the context they need.
- **Graceful degradation near limits.** Instead of a sudden context exhaustion
  from a broadcast storm, sessions wind down smoothly.
- **Simpler wind-down procedure.** "Stop talking" is simpler and cheaper than
  "tell everyone to stop."
- **Correct mental model.** Teams think of context as a budget, not an unlimited
  resource, leading to better message discipline throughout the session.

### Negative

- **Idle agents may hold stale state.** An agent that was mid-task when messaging
  stopped will retain its partial context. This is acceptable because session end
  cleans up all agents, and any important state should be in PRs or issues.
- **No explicit confirmation of wind-down.** The orchestrator does not get
  acknowledgment that agents have stopped. This is acceptable because there is
  nothing useful to do with that confirmation near session end.

---

## Evidence

- **2026-03-19 incident**: Broadcasting shutdown to 117 agents consumed ~6% of
  session context in acknowledgment messages alone, leaving insufficient budget
  for final memory writes.
- **Cycle 5 observation**: Sessions with 75+ agents that used targeted messaging
  (only contacting agents with actionable work) maintained healthy context
  budgets throughout.
- **Merge queue data**: With a 3-wide merge queue and ~5 min CI per PR, more
  than ~9 concurrent coding agents produce PRs faster than they can be merged,
  creating a backlog that does not benefit from additional agents.
