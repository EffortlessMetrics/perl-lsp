# ADR-0032: Skill Scoping and Hook Enforcement for Swarm Orchestration

**Status**: Accepted
**Date**: 2026-03-16
**Related**: PR #1707, [SKILL_AND_AGENT_DESIGN.md](../reference/SKILL_AND_AGENT_DESIGN.md)

---

## Context

In swarm cycle 2, the orchestration model had three recurring failure modes:

1. **Context window pollution**: The orchestrator loaded agent-scoped skills (`/swarm-protocol`, `/coding-standards`, `/swarm-priorities`) into its own context. These skills contain behavioral rules for coding agents — irrelevant to the orchestrator — and consumed context window budget that should be reserved for routing and state tracking.

2. **Metrics compliance failure**: Agents were instructed via prompts to write entries to `.ops-perl-lsp/swarm-metrics.jsonl` after completing tasks. Zero of 30 merged PRs contained a metrics entry. Prompt instructions are advisory; agents skip them under time or context pressure.

3. **Prompt bloat and skip rate**: Every agent prompt included "Invoke /coding-standards" as boilerplate. This added prompt size overhead and was frequently skipped, meaning agents produced code without coding standards loaded.

The underlying pattern: **prompt instructions are unreliable for behavioral enforcement**. Anything that must happen needs structural enforcement, not textual requests.

---

## Decision

### Decision 1: Skill Scope Separation

Skills are explicitly categorized by who may invoke them:

**Orchestrator-scoped skills** (the orchestrator invokes these; agents do not):
- `/swarm-status`
- `/green-merge`
- `/health-check`
- `/swarm-report`
- `/rebase-open`
- `/corpus-ratchet`

**Agent-scoped skills** (agents invoke these; the orchestrator does not):
- `/swarm-protocol`
- `/coding-standards`
- `/swarm-priorities`

The orchestrator **never** invokes agent-scoped skills. Loading behavioral rules meant for subagents into the orchestrator's context is waste — that context budget belongs to routing, state, and coordination.

### Decision 2: Hook Enforcement Over Prompt Instructions

Three hooks replace prompt-based behavioral requests:

| Hook | Trigger | Enforcement |
|------|---------|-------------|
| `TaskCompleted` | Agent marks a task done | Blocks completion unless a metrics entry exists in `swarm-metrics.jsonl` |
| `SubagentStart` | Any subagent initializes | Auto-injects condensed coding standards and known pitfalls into the subagent's context |
| `TeammateIdle` | A teammate goes idle | Blocks idle state if the teammate has in-progress tasks not yet completed |

Hooks execute unconditionally. Unlike prompt instructions, they cannot be skipped under pressure.

### Decision 3: Skill Frontmatter for Access Control

Skills use frontmatter fields to encode their intended audience:

```yaml
# Orchestrator-only: user types the command; model cannot invoke it autonomously
disable-model-invocation: true

# Agent-only: Claude auto-loads when relevant; not a user-facing command
user-invocable: false

# Dual-use: both the user and agents can invoke (default)
# (no frontmatter needed)
```

This makes access control explicit in the skill definition rather than enforced only by convention.

### Decision 4: Layered Context Injection

Instead of every agent prompt repeating "Invoke /coding-standards", context is injected in layers:

- **Layer 0 (automatic)**: `SubagentStart` hook injects a condensed version of coding standards — banned constructs, key patterns — into every subagent at startup. Zero prompt overhead required.
- **Layer 1 (on demand)**: Full `/coding-standards` skill remains available for agents that need the complete reference during a task.
- **Layer 2 (persistent)**: Known pitfalls from `.claude/swarm-state/known-pitfalls.md` are auto-injected alongside standards by the `SubagentStart` hook.

---

## Consequences

**Positive:**

- Orchestrator context stays clean. Agent behavioral docs no longer pollute it.
- Metrics compliance is structural. The `TaskCompleted` hook enforces entries; prompt requests were 0% reliable.
- Every subagent gets coding standards automatically. No prompt overhead, no skip risk.
- Skill audience is self-documenting via frontmatter, reducing the chance of orchestrators accidentally loading agent skills.

**Negative / Trade-offs:**

- Hook failures block agent progress. A misconfigured `TaskCompleted` hook can prevent task completion. Hooks must be tested before deployment.
- Condensed standards injected by `SubagentStart` may diverge from the full `/coding-standards` skill if one is updated without the other. Both need to be kept in sync.
- Agents that need the full standards must still invoke `/coding-standards` explicitly — the auto-injected layer is a summary, not a replacement.

---

## Divergence from Original Design

As of 2026-03-21, the following items described in this ADR were not implemented.
Future agents should treat this section as the ground truth, not the decision tables above.

| Hook | ADR Promise | Actual State |
|------|------------|--------------|
| `SubagentStart` | Auto-injects condensed coding standards and known pitfalls from `.claude/swarm-state/known-pitfalls.md` | Implemented as a bare `echo` command in `settings.json`; `.claude/swarm-state/` directory and `subagent-start.sh` do not exist |
| `TaskCompleted` | Blocks completion unless a metrics entry exists in `swarm-metrics.jsonl` | Only checks `cargo fmt` and `CURRENT_STATUS.md`; no metrics gate implemented |
| `TeammateIdle` | Registered in `settings.json` to block idle state if in-progress tasks exist | Hook script exists in `docs/handoff/swarm-pack/hooks/` but is **not registered** in live `settings.json`. **Formally retracted 2026-03-22** — see retraction note below |
| `PreToolUse` | (not in original ADR) | Inline in `settings.json` as of ADR write date; extracted to `.claude/hooks/pre-tool-use.sh` in PR #2297 |

**Consequence for builders:** The metrics compliance guarantee (Decision 2) was never enforced. The original motivation — zero of 30 PRs contained a metrics entry — was not resolved by this ADR's implementation. As of 2026-03-22, `TaskCompleted` now writes a passive metrics entry (observability, not blocking gate) — lifecycle ordering makes a blocking gate unimplementable at that hook point.

**TeammateIdle — formally retracted (2026-03-22):** The `teammate-idle.sh` hook in `docs/handoff/swarm-pack/hooks/` unconditionally exits 2, which would create an infinite busy-loop when a teammate's task list is genuinely empty. The hook was not registered across 6 sessions of 50-100 agents and its absence caused no observed failures. This hook is formally withdrawn from the design. The swarm relies on task-list discipline and orchestrator routing rather than idle-state blocking.

## Files Changed

| File | Change |
|------|--------|
| `.claude/commands/swarm.md` | Added Skill Scope section; removed orchestrator-level agent skill invocations |
| `.claude/agents-compat/swarm-*.md` (13 files) | Added `TaskList`/`TaskUpdate`/`TaskCreate` references; added metrics mandate |
| `.claude/hooks/task-completed.sh` | Enhanced with metrics gate |
| `.claude/hooks/subagent-start.sh` | New hook; auto-injects coding standards and known pitfalls |
| `.claude/hooks/teammate-idle.sh` | Enhanced with in-progress task check |
| `.claude/settings.json` | Registered `SubagentStart` hook |
