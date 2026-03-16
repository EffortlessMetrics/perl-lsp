# ADR-0033: Worktree-First Disposable Worker Execution for Swarm Orchestration

**Status**: Accepted
**Date**: 2026-03-16
**Related**: [ADR-0032](0032-skill-scoping-and-hook-enforcement.md), [SWARM_DESIGN.md](../handoff/SWARM_DESIGN.md), [AGENT_SWARM_WORKFLOW.md](../project/AGENT_SWARM_WORKFLOW.md)

---

## Context

The swarm has already adopted small persistent coordinator teams, skill-scoped
behavior, and hook-based enforcement. The remaining architectural question is
how aggressively to lean into fresh workers, worktree isolation, and
pre-encoded procedure.

This ADR defines the default operating model for **swarm mode**: continuous,
parallel, PR-shaped execution. It is not a universal rule for every Claude Code
interaction. Small sequential edits that stay in one file surface and one
verification loop can still stay in the main conversation.

The default "small team, occasional delegation" model is safe and broadly
applicable, but it under-optimizes for a repo that routinely decomposes into
dozens of independent PR-shaped units of work. In that environment, the main
failure modes are not lack of intelligence; they are:

1. **Stale worker context**: long-lived implementation agents drift into
   adjacent tasks, retain irrelevant state, and carry the wrong verification
   loop into later work.
2. **Shared-write collisions**: concurrent coding without worktree isolation
   creates branch churn, merge conflicts, and cleanup cost.
3. **Prompt bloat**: stable conventions are restated in every spawn prompt
   instead of being encoded structurally in skills, hooks, or templates.
4. **Weak context boundaries**: a worker that starts on one crate or one PR can
   silently expand into another when the next task "looks similar enough."
5. **Over-persistent teams**: execution responsibility sticks to named
   teammates instead of being pushed out to cheap disposable workers.

For high-throughput coding, the architecture needs sharper boundaries.

---

## Decision

**All code mutation in swarm mode happens in disposable workers running in
isolated git worktrees; persistent coordinators own routing, review, merge
control, and system improvement.**

### 1. The worktree is the write-isolation boundary

Every PR-shaped code change runs in its own git worktree. Shared mutation in a
single checkout is not the default.

Use a new worktree when any of the following is true:
- the change should land in its own PR
- the change may need independent rebasing
- the verification loop differs from other in-flight changes
- another agent may touch overlapping parts of the main checkout

For read-only scouting or analysis, a worktree is optional. For code mutation,
it is the default.

### 2. The worker is the context and permission boundary

Spawn a fresh worker whenever the work context changes materially. A context
shift includes any of:
- a different objective or hypothesis
- a different crate or dominant file surface
- a different tool or permission profile
- a different verification command or gate
- a different PR target or branch

The swarm does **not** optimize for reusing implementation workers. It
optimizes for replacing them cheaply.

### 3. Persistent teammates form a thin control plane

Persistent teammates stay small in number and narrow in role:
- `scout`
- `builder`
- `reviewer`
- `ops`
- `improver`

These coordinators route work, claim tasks, read status, aggregate results, and
spawn disposable specialists. They do not act as the main long-lived carriers
of implementation context.

### 4. Durable knowledge is pre-encoded; volatile state is handed off

Stable, reusable knowledge belongs in structural artifacts:
- `CLAUDE.md`
- skills and supporting files
- hooks
- prompt templates
- agent/coordinator definitions

Volatile task state belongs in:
- handoff files
- the worktree itself
- the PR/issue
- queue or state files

Do not keep transient task detail alive by reusing the same worker when a fresh
worker plus a handoff would be clearer.

Subagents do not inherit the caller's loaded skills automatically. If a worker
needs repo procedure or domain knowledge, list the required skills explicitly
in the worker prompt or package the task as a `context: fork` skill.

### 5. Hooks own guarantees; prompts own judgment

If a behavior must happen every time, it belongs in a hook or another
deterministic layer. Prompts should express judgment and local procedure, not
attempt to enforce invariants such as:
- completion gates
- dangerous command blocking
- mandatory logging
- state refresh after compaction

In the live repo control plane, the primary lifecycle hooks that operationalize
this model are `SessionStart`, `SubagentStart`, `SubagentStop`,
`TeammateIdle`, and `TaskCompleted`. Those are the mechanical boundaries for
state refresh, provisioning, cleanup, queue bookkeeping, metrics, and
completion gates.

`WorktreeCreate` and `WorktreeRemove` remain available hook boundaries, but
they are intentionally not registered in the shared project settings because
they replace Claude Code's default worktree provisioning and teardown behavior.
Treat them as reserved hooks to adopt only when the repo explicitly wants to
own that lifecycle itself.

### 6. Handoffs are the continuity mechanism

When a worker finishes or when a context shift requires retirement, it writes
or updates a handoff so the next worker starts from condensed context instead of
reconstructing it from scratch.

The handoff is the continuity mechanism. Worker reuse is not.

---

## Consequences

### Positive

- **Higher parallel throughput**: independent PR-shaped changes can proceed
  without shared-write interference.
- **Cleaner prompts**: stable rules move into reusable structures instead of
  being restated in every spawn message.
- **Lower context drift**: workers die at context boundaries instead of
  accumulating unrelated history.
- **Better review boundaries**: one worktree and one worker map naturally to
  one reviewable diff.
- **Cheaper failure recovery**: a bad worker attempt is discarded and replaced
  without poisoning the rest of the swarm.

### Negative

- **More branch/worktree churn**: the system creates more short-lived
  worktrees and branches, so cleanup discipline matters.
- **More handoff writing**: some context that might have lived implicitly in a
  reused worker now needs to be recorded.
- **Stricter slicing pressure**: vague or oversized tasks become obvious
  because they resist clean worktree and worker boundaries.

---

## Alternatives Considered

### Reuse long-lived implementation teammates

**Rejected**: this saves spawn overhead but reintroduces stale context,
scope creep, and muddled verification loops.

### Keep worktrees optional for code changes

**Rejected**: optional isolation works for occasional delegation, but not for
the swarm's normal operating mode. Mutation without isolated filesystem state
is fragile under parallel load.

### Encode everything in prompts

**Rejected**: stable knowledge and mandatory behavior drift too easily when
kept only in prompt prose.

---

## Operating Rules

1. **New worktree** when the change should review or merge independently.
2. **New worker** when objective, file surface, tool profile, or verification
   loop changes.
3. **New skill** when the instructions are stable enough to reuse across runs.
4. **New hook** when the behavior must be guaranteed rather than requested.
5. **No new worker** for branch-local, sequential work that shares the same
   objective, files, and verification loop.

---

## Related Files

- [`.claude/commands/swarm.md`](../../.claude/commands/swarm.md)
- [`.claude/skills/swarm/SKILL.md`](../../.claude/skills/swarm/SKILL.md)
- [`.claude/skills/swarm/reference/team-structure.md`](../../.claude/skills/swarm/reference/team-structure.md)
- [`.claude/skills/swarm/templates/teammate-prompt-template.md`](../../.claude/skills/swarm/templates/teammate-prompt-template.md)
- [`docs/reference/SKILL_AND_AGENT_DESIGN.md`](../reference/SKILL_AND_AGENT_DESIGN.md)
