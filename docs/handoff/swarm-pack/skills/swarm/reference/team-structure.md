# Portable Swarm Team Structure

## Roles

| Name | Role | Subagent Strategy |
|------|------|-------------------|
| `scout` | Discovery coordinator | One Explore worker per bucket or issue cluster |
| `builder` | Build coordinator | One worktree worker per PR-shaped change |
| `reviewer` | Review coordinator | One reviewer worker per PR |
| `ops` | Merge and CI coordinator | Sequential merges, one fixer per failure mode |
| `improver` | Docs/tests/devex coordinator | Small background stream of worktree workers |

## Execution Doctrine

- Coordinators are persistent; implementation workers are disposable.
- Every PR-shaped code change gets its own worktree.
- Every materially different context gets a fresh worker.
- Stable procedure belongs in skills and templates; volatile task state belongs in handoffs, worktrees, and PRs.
- Worker prompts must list the required skills explicitly; subagents do not inherit parent skill state.

## Context Shift Triggers

Spawn a new worker when any of these change:
- objective or hypothesis
- dominant file surface
- tool or permission profile
- verification command
- branch or PR target

## Data Flow

```
scout ──────→ TaskCreate ─────→ builder claims via TaskList
builder ────→ SendMessage ────→ reviewer
reviewer ───→ gh pr create ───→ ops (merge queue)
ops ────────→ gh pr merge ────→ ops (post-merge validation)
ops ────────→ SendMessage ────→ scout (queue low)
improver ───→ worktree subs ──→ improvement PRs
```
