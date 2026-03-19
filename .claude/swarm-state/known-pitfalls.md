# Known Pitfalls

Accumulated lessons from fixer agents and failed builds. Scouts and builders should read this before starting work to avoid repeating known mistakes.

This file is append-only during swarm operation. The janitor consolidates it periodically.

## Format

Each entry:
```
### <date> — <category>
**Source**: <branch or PR that discovered this>
**Pitfall**: <what went wrong>
**Fix**: <what the correct approach is>
**Affected crates**: <list>
```

## Entries

<!-- Agents append new entries below this line -->

### 2026-03-19 — Rebase Burns CI Queue

**Source**: Cycle 5 session learnings
**Pitfall**: Workers that rebase onto master before creating a PR trigger a new CI run on master. When multiple workers rebase in quick succession, each rebase push cancels the previous CI run, burning the CI queue and delaying validation of already-merged work.
**Fix**: Workers should NOT rebase. Only fix code and verify locally. The ops coordinator handles rebasing as a controlled batch operation when needed.
**Affected crates**: all

### 2026-03-19 — Worktree Contention

**Source**: Cycle 5 parallel agent conflicts
**Pitfall**: Two agents assigned to the same worktree directory can corrupt each other's work — uncommitted changes, staged files, and branch state all collide silently. This happens when worktree names overlap or when an agent is reassigned to a worktree still in use.
**Fix**: Every agent gets its own uniquely-named worktree. Never reuse a worktree across agents. Before spawning a worker, verify the target worktree does not already exist with `git worktree list`.
**Affected crates**: all

### 2026-03-19 — Built-But-Not-Wired Pattern

**Source**: Cycle 5 review findings
**Pitfall**: Agents implement new functions, structs, or modules that compile and pass tests, but are never called from any entry point. The code is dead on arrival — it exists in the crate but no execution path reaches it. This is hard to catch in diff-only review.
**Fix**: Before marking a PR complete, verify new public functions have at least one call site outside their definition module and tests. Grep for the function name. The `/verify-build` skill now includes a wiring check step.
**Affected crates**: all

### 2026-03-19 — Repurpose Idle Agents

**Source**: Cycle 5 capacity management
**Pitfall**: Agents that complete their task early sit idle, wasting capacity. Meanwhile other lanes (review, ops) may be bottlenecked. The idle agent's context is warm and could be repurposed, but the default behavior is to just stop.
**Fix**: When an agent finishes its task, it should report completion AND check if adjacent work exists in the same lane. If not, send a message to the coordinator offering to take on the next queued task. Coordinators should reassign idle agents to the highest-priority unblocked work rather than spawning new ones.
**Affected crates**: all
