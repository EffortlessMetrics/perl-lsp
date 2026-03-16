# ADR-008: Swarm v2 Agent Architecture

**Status**: Accepted
**Date**: 2026-03-16
**Decision Makers**: Swarm Orchestrator (Cycle 3)

---

## Context

Cycle 3 audited the swarm infrastructure and found several gaps where the
implementation did not match platform capabilities or intended behavior:

1. **Hooks silently failing**: `TaskCompleted` and `TeammateIdle` hooks read
   agent context from environment variables (`$TASK_SUBJECT`, `$TEAMMATE_ID`)
   that Claude Code never populates. Hooks ran but had no effect — builder
   accountability and idle detection were dead code.

2. **12-teammate teams**: The team roster grew to 12 named teammates
   (scout, builder-1 through builder-4, reviewer-1 through reviewer-4,
   ops, etc.). The recommended range is 3–5. Token overhead was high and
   most teammates were idle most of the time.

3. **Skills as flat documentation files**: Skills in `.claude/commands/` were
   plain markdown files read by agents. They had no isolation, no tool
   restrictions, and no `context:` field. A skill could accidentally
   modify the repo even when invoked for read-only research.

4. **Missing hook types**: `SubagentStart`, `SubagentStop`, `PreToolUse`,
   and `SessionStart` hooks were not configured. There was no mechanism to
   auto-inject coding standards into builder subagents or to block
   dangerous commands globally.

5. **No deliverable verification**: Builders could complete tasks without
   creating a branch or PR. The `TaskCompleted` hook checked `cargo fmt`
   but not whether a PR actually existed.

6. **Agent definitions were thick**: Agents in `agents6/` contained inline
   instructions that duplicated skill content. Any update to a workflow
   required editing both the skill and every agent that referenced it.

7. **Permission rules used colon syntax**: Settings used `Bash(gh:*)` which
   is not the correct Claude Code format; the correct format is `Bash(gh *)`.

---

## Decision

Align the swarm infrastructure with the official Claude Code platform
capabilities as documented in Cycle 3:

### 1. Hooks read JSON from stdin

All hooks now parse stdin JSON using `jq` to extract context. The hook
protocol delivers a JSON payload; environment variable injection is not
supported for hook context.

```bash
# Before (silently empty)
SUBJECT="$TASK_SUBJECT"

# After (correct protocol)
SUBJECT=$(echo "$HOOK_INPUT" | jq -r '.task.subject // empty')
```

### 2. Skills use `context: fork` and `allowed-tools`

Skills gain frontmatter fields for isolation and tool restriction:

```yaml
---
name: verify-build
description: Verify branch, tests, and PR exist
context: fork
allowed-tools: Bash(cargo *), Bash(git *)
---
```

`context: fork` runs the skill in an isolated subagent so it cannot
accidentally modify the main conversation state. `allowed-tools` enforces
tool restrictions at the platform level, not via prompt.

### 3. Team restructured to 5 coordinators

Replace 12 flat teammates with 5 coordinator roles:

| Role | Responsibility |
|------|---------------|
| scout | Broad exploration, writes GitHub issues |
| builder | Code changes in worktrees, one per task |
| reviewer | One PR per agent, draft review loop |
| ops | Merging, rebasing, CI watching |
| improver | Docs, tests, devex, infra (~20% capacity) |

Each coordinator spawns 3–8 focused subagents in parallel. Total capacity
is unchanged; token overhead is reduced by eliminating idle named teammates.

### 4. Agent definitions are thin orchestration loops

Agents invoke skills instead of containing inline instructions:

```markdown
# Before: inline
1. Read the file
2. Check for unwrap() calls
3. Fix each one
4. Run cargo clippy
...

# After: skill invocation
/coding-standards
/verify-build
```

This makes agent definitions ~50 lines each. Improving a skill benefits
all agents that invoke it without requiring agent edits.

### 5. Added hook types

- **SubagentStart**: Auto-injects `/coding-standards` into builder subagents
- **Stop**: Reminds agents to verify deliverables before stopping
- **PreToolUse**: Blocks `git push --force`, `rm -rf`, `cargo publish` without
  explicit authorization
- **SessionStart/compact**: Re-injects critical context after context compaction

### 6. Builder accountability via TaskCompleted hook

`TaskCompleted` now verifies a branch and PR exist before allowing completion:

```bash
BRANCH=$(git branch --show-current)
if [[ "$BRANCH" == "master" || "$BRANCH" == "main" ]]; then
  echo "Error: builder completed on master — no worktree branch created"
  exit 2
fi
PR=$(gh pr list --head "$BRANCH" --json number --jq '.[0].number' 2>/dev/null)
if [[ -z "$PR" ]]; then
  echo "Error: no PR found for branch $BRANCH — create one before marking complete"
  exit 2
fi
```

### 7. Permission rules use space syntax

Corrected from `Bash(gh:*)` to `Bash(gh *)` throughout `settings.json`.

---

## Consequences

### Positive

- **Hooks now enforce rules**: The `TaskCompleted`, `TeammateIdle`, and new
  `SubagentStart`/`PreToolUse` hooks actually read the data they need and
  take meaningful action.
- **Skills are isolated**: `context: fork` prevents skills from accidentally
  modifying conversation state. `allowed-tools` provides hard enforcement
  beyond prompt-level instructions.
- **~60% token reduction**: 5 coordinators vs. 12 flat teammates eliminates
  idle teammate overhead.
- **Agent definitions are maintainable**: ~50-line agents that invoke skills
  are far easier to read and update than 200-line agents with inline workflows.
- **Skills compound**: Every skill improvement automatically benefits all
  agents that invoke it. The marginal cost of adding a new agent decreases
  as the skill library grows.
- **Dangerous command blocking**: `PreToolUse` on `Bash` blocks force-push,
  rm -rf of tracked directories, and unauthorized `cargo publish` at the
  platform level.

### Negative

- **Migration cost**: Existing `agents6/` definitions need updating to use
  skill invocations instead of inline instructions. This is a one-time cost.
- **`context: fork` adds latency**: Forked subagents have startup overhead
  (~2–5s). Acceptable for skills that enforce safety; not appropriate for
  trivial utility skills.
- **Hook complexity**: JSON parsing via `jq` in bash hooks adds a `jq`
  dependency. This is already present in the Nix dev shell.

---

## Alternatives Considered

### Keep 12-teammate team with better assignment

**Rejected**: The root problem is not assignment logic — it is that idle
named teammates consume tokens even when not working. Reducing to
coordinators-with-subagent-fanout is architecturally cleaner.

### Prompt-based enforcement (no hooks)

**Rejected**: Cycle 3 demonstrated that prompt instructions are ignored
under load. Builders completed tasks on master without creating PRs.
Platform-level enforcement via hooks is the only reliable mechanism.

### Monolithic skill files without `context: fork`

**Rejected**: Without isolation, skills invoked for research can
accidentally trigger writes. The `context: fork` overhead is worth the
safety guarantee.

---

## Related

- ADR-001: Agent Architecture Specialization (original agent design)
- `.claude/hooks/` — hook implementations
- `.claude/commands/` — skill definitions
- `docs/project/AGENT_SWARM_WORKFLOW.md` — operational workflow
- `docs/handoff/SWARM_DESIGN.md` — portable design reference
