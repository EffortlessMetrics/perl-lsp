# Swarm Agent Architecture for Claude Code

A portable design for running continuous, highly-parallel development swarms using Claude Code's agent teams, subagents, and worktree isolation. Designed for repos with many independent improvement opportunities (parser fixes, test gaps, dead code, etc.).

## The Core Insight

Claude Code's agent teams documentation recommends 3-5 teammates. That's fine for a review or a feature — but for continuous codebase improvement, you need a different model:

**Thin coordinator teammates + thick subagent fanout + worktree isolation per task.**

- **Teammates** are persistent lane operators. They manage work queues, not do the work.
- **Subagents** are fresh, focused workers. Spawn them aggressively with tight prompts.
- **Worktrees** give every coding subagent an isolated copy of the repo. No file conflicts.

This creates a multiplicative effect:
```
12 coordinator teammates × 3-8 subagents each = 30-60 parallel workers
```

Fresh subagents are more context-efficient than long-running teammates because:
1. Their prompts are focused on exactly one task
2. They don't accumulate irrelevant context
3. Good agent definitions (`.claude/agents/`) ensure consistent behavior
4. They exit when done — no idle token burn

## Architecture

```
                    ┌─────────────────────┐
                    │   Lead (Orchestrator)│
                    │   Coordinates only   │
                    └──────────┬──────────┘
                               │
        ┌──────────┬───────────┼───────────┬──────────┐
        ▼          ▼           ▼           ▼          ▼
   ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌────────┐ ┌────────┐
   │Scout    │ │Builder  │ │Reviewer │ │Merger  │ │Janitor │
   │Coord ×2 │ │Coord ×2 │ │Coord ×1 │ │     ×1 │ │     ×1 │
   └────┬────┘ └────┬────┘ └────┬────┘ └────────┘ └────────┘
        │           │           │
   ┌────┴────┐ ┌────┴────┐ ┌────┴────┐
   │Explore  │ │Build    │ │Review   │
   │subagents│ │subagents│ │subagents│
   │×5-8 each│ │×3-5 each│ │×3-5 each│
   │(read    │ │(worktree│ │(read    │
   │ only)   │ │ each)   │ │ diffs)  │
   └─────────┘ └─────────┘ └─────────┘
```

### Lane Flow (Continuous)

```
Scouts discover → Tasks created → Builders claim → Worktree subagents build
     ↑                                                        ↓
     │                                           Reviewers review → PRs created
     │                                                        ↓
     └── merger signals "queue low" ← Merger merges ← Green PRs
                                           ↓
                                    Janitor cleans up worktrees
```

All lanes run concurrently. Never stop one to wait for another.

## Design Decisions

### Why thin teammates + thick subagents (not many teammates)?

Agent teams teammates each carry a full context window. 20 teammates = 20 context windows accumulating noise. Instead, 7 coordinator teammates manage the flow while fresh subagents do focused work. Subagents start clean, do one thing, and exit.

### Why worktrees (not branches)?

Git worktrees give each subagent a physically separate working directory. Multiple subagents can modify different files simultaneously without `git stash`/`checkout` conflicts. The Agent tool's `isolation: "worktree"` handles creation and cleanup automatically.

### Why overlap detection by files (not agent count)?

The constraint isn't "too many agents." It's "two agents editing the same file." Every scout SLICE includes a `files_touched` field listing every file the builder will modify. The orchestrator checks set intersections before assigning work. No overlap = no conflict = unlimited parallelism.

### Why per-crate verification (not workspace)?

`cargo test --workspace` takes 3-5 minutes. `cargo test -p <crate>` takes 10-30 seconds. For small, focused PRs, crate-level verification is sufficient and 10x faster. Escalate to workspace verification only for cross-cutting changes (3+ crates).

### Why fresh agents instead of reusing?

A fresh subagent with a well-written prompt from `.claude/agents/swarm-builder.md` produces better results than a long-running agent that's accumulated conversation history about past tasks. The agent definition IS the reusable knowledge. The subagent instance is disposable.

### Why hooks for idle/completion?

`TeammateIdle` keeps coordinators working instead of going dormant. `TaskCompleted` enforces quality gates (cargo fmt check) automatically. These hooks make the swarm self-sustaining without manual intervention.

### Why handoff files (not just SLICE definitions)?

Context efficiency. The handoff chain is scout → builder → reviewer → merger. Without handoffs, each agent re-reads the same source files the previous agent already read. With handoffs:

- **Scout** reads 10 files, writes a handoff with code excerpts, test template, and fix strategy
- **Builder** reads ONLY the handoff file (not 10 source files), implements, then appends a reviewer briefing
- **Reviewer** reads the briefing + a focused diff scan (not the full diff cold)
- **Improvers** read handoff "Lesson Learned" sections to find friction and ADR candidates

Handoff files live in `.ops-<repo>/handoffs/<branch>.md` and are cleaned up by the janitor after merge.

**The efficiency hierarchy:**
1. Best: multiple context-chunked agents with effective handoffs (each reads only what the previous condensed)
2. Good: one agent with skills and cache hits (same context, no re-reading)
3. Worst: multiple agents re-reading the same full context with no sharing

The handoff protocol ensures we stay in category 1.

## Files to Create

### Agent Definitions (`.claude/agents/`)

| File | Role | Color | Persistence |
|------|------|-------|-------------|
| `swarm-scout.md` | Read-only gap finder, spawns Explore subagents | green | Coordinator teammate |
| `swarm-builder.md` | TDD implementation, spawns worktree subagents | blue | Coordinator teammate |
| `swarm-reviewer.md` | Diff review + PR creation, spawns review subagents | yellow | Coordinator teammate |
| `swarm-fixer.md` | Surgical CI failure repair | red | Coordinator teammate |
| `swarm-merger.md` | Sequential merge + drift handling | purple | Direct teammate |
| `swarm-janitor.md` | Worktree/branch cleanup | gray | Direct teammate |

Each agent file has:
- YAML frontmatter: `name`, `description`, `model` (sonnet for workers), `color`
- Operating mode: persistent teammate vs. one-shot subagent
- Subagent spawning patterns with `run_in_background: true`
- Communication patterns (SendMessage to other teammates)
- Structured output format for results

### Slash Commands (`.claude/commands/`)

| File | Purpose |
|------|---------|
| `swarm.md` | Main entry point — creates team, spawns all coordinators |
| `queue-scout.md` | Launch scouts across focus areas |
| `green-merge.md` | Drain all passing PRs |
| `rebase-open.md` | Rebase conflicted PRs onto master |
| `status-drift.md` | Fix computed metric drift |
| `salvage-worktrees.md` | Save dirty worktrees before cleanup |

### Hooks (`.claude/hooks/`)

| File | Event | Behavior |
|------|-------|----------|
| `teammate-idle.sh` | TeammateIdle | Nudges idle teammates to claim tasks or discover new work |
| `task-completed.sh` | TaskCompleted | Runs `cargo fmt --check` before allowing task completion |

### Settings (`.claude/settings.json`)

Add hook registrations:
```json
{
  "hooks": {
    "TeammateIdle": [{ "hooks": [{ "type": "command", "command": "bash .claude/hooks/teammate-idle.sh" }] }],
    "TaskCompleted": [{ "hooks": [{ "type": "command", "command": "bash .claude/hooks/task-completed.sh" }] }]
  }
}
```

### Queue Artifact (`.ops-<repo>/swarm-queue.json`)

```json
{
  "slices": [],
  "hot_files": []
}
```

Machine-facing file. Coordinators read/write this to track active slices and prevent file overlap.

## Adapting to Another Repo

### 1. Copy the infrastructure

```bash
# From the template repo
cp -r .claude/agents/swarm-*.md   <your-repo>/.claude/agents/
cp -r .claude/commands/swarm.md   <your-repo>/.claude/commands/
cp -r .claude/commands/green-merge.md <your-repo>/.claude/commands/
cp -r .claude/commands/rebase-open.md <your-repo>/.claude/commands/
cp -r .claude/commands/status-drift.md <your-repo>/.claude/commands/
cp -r .claude/commands/salvage-worktrees.md <your-repo>/.claude/commands/
cp -r .claude/hooks/              <your-repo>/.claude/hooks/
mkdir -p <your-repo>/.ops-<name>/
echo '{"slices":[],"hot_files":[]}' > <your-repo>/.ops-<name>/swarm-queue.json
```

### 2. Customize scout focus areas

Edit `swarm-scout.md` to list YOUR repo's gap sources:
- What are your test coverage gaps?
- Where are your error hotspots?
- What tool checks for unused deps in your language?
- Where is your technical debt tracked?

### 3. Customize verification commands

Replace `cargo fmt`, `cargo clippy`, `cargo test` with your language's equivalents:
- Python: `ruff check`, `pytest -x`
- TypeScript: `eslint`, `tsc --noEmit`, `vitest`
- Go: `gofmt`, `go vet`, `go test ./...`

### 4. Customize the task-completed hook

Replace `cargo fmt --check` with your language's format check.

### 5. Customize drift handling

Replace CURRENT_STATUS/corpus/CPAN with your repo's computed metrics:
- Coverage reports
- Bundle size
- API docs generation
- Changelog updates

### 6. Enable agent teams

```json
// ~/.claude/settings.json
{
  "env": {
    "CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS": "1"
  }
}
```

### 7. Run it

```
/swarm all
```

## Key Principles (Portable)

1. **Coordinators don't code.** Teammates manage lanes. Subagents do work.
2. **Fresh beats stale.** New subagent > reused context. Agent definitions are the reusable part.
3. **Parallel beats sequential.** Launch all independent subagents in one message.
4. **Worktrees for all code changes.** `isolation: "worktree"` prevents conflicts.
5. **Overlap by files, not count.** Cap file ownership, not agent count.
6. **Per-unit verification.** Test the crate/package you changed, not the whole workspace.
7. **Continuous, not batchy.** All lanes run concurrently. Don't batch-then-merge.
8. **Hooks keep it moving.** `TeammateIdle` prevents stalls. `TaskCompleted` enforces quality.
9. **Code first, docs later.** Scout for code/test/security gaps by default. Docs only when needed.
10. **Machine-facing queue.** Track slices and hot files in a JSON artifact, not in conversation.
