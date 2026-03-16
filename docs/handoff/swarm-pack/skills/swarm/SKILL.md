---
name: swarm
description: Start a continuous swarm with thin persistent coordinators and disposable worktree workers.
user-invocable: true
argument-hint: "[focus] e.g. 'all', 'parser', 'tests', 'cleanup', 'improve'"
disable-model-invocation: true
---

# Swarm: Portable Control Plane

Start a continuous swarm. Focus: **$ARGUMENTS**

You are the lead. You coordinate only. You NEVER write production code.
Persistent coordinators own routing, review, merge control, and system
improvement. Disposable workers in isolated worktrees do all code mutation.

## Skill Scope

See [reference/team-structure.md](reference/team-structure.md) for the concrete
coordinator handoffs and data flow.

The pack ships both skills and compatible commands. Prefer this skill as the
canonical coordinator playbook. Use the command files as compatibility surfaces
and manual entrypoints.

## Execution Boundaries

Treat each layer as a different boundary:

1. **Worktree = write boundary**: every PR-shaped code change happens in its own worktree.
2. **Worker = context boundary**: spawn a fresh worker when objective, file surface, tool profile, permissions, verification loop, or branch changes materially.
3. **Skill = durable procedure boundary**: stable instructions live in skills, not in repeated inline prose.
4. **Hook = deterministic control boundary**: anything that must always happen belongs in hooks, not in agent memory.

If a coding task crosses into a different crate, file surface, or verification loop, do not stretch the current worker. Write or update the handoff and spawn a fresh worker in a fresh worktree.
Subagents do not inherit parent skills automatically. Every worker prompt must name the required skills explicitly, or the task itself should be packaged as a `context: fork` skill.
Each coordinator and worker should keep a local todo list. Every todo item should name the skill or command to invoke for that step so the procedure stays attached to the work, not to ambient memory.

## Team Structure

The persistent coordinator layer stays small:

| Name | Role | Subagent Strategy |
|------|------|-------------------|
| `scout` | Discovery coordinator | Spawns 5-8 Explore subagents/round |
| `builder` | Build coordinator | Spawns 3-5 worktree subagents/round |
| `reviewer` | Review + PR creation | Spawns 3-5 review subagents/round |
| `ops` | Merge + validate + fix CI | Sequential merges, spawns fix subagents |
| `improver` | Docs + tests + devex | Spawns 2-4 worktree subagents |

Use [templates/teammate-prompt-template.md](templates/teammate-prompt-template.md)
for standard spawn prompts.

## Operator Rules

- Mutation implies worktree.
- New context implies new worker.
- Stable procedure belongs in skills and templates.
- Completion and cleanup behavior belong in hooks and settings.
- Keep volatile task detail in handoffs, not in persistent coordinator memory.
