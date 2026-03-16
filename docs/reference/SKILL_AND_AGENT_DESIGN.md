# Skill and Agent Architecture Design

This document captures the design of the skill and agent architecture for the perl-lsp swarm: how skills are scoped, how agents are structured, how hooks enforce rules, and how context flows through the pipeline.

## Table of Contents

1. [Skill Scoping Model](#1-skill-scoping-model)
2. [Agent Architecture](#2-agent-architecture)
3. [Hook Enforcement Layer](#3-hook-enforcement-layer)
4. [Context Flow Design](#4-context-flow-design)
5. [Task-Based Coordination](#5-task-based-coordination)
6. [Design Principles](#6-design-principles)

---

## 1. Skill Scoping Model

Skills live in `.claude/commands/`. There are 32 skills, organized into three categories based on who invokes them.

### 1.1 Orchestrator-Only Skills

These skills are invoked manually by the user or the lead orchestrator. Agents **must not** auto-trigger these — they start long-running processes or affect shared infrastructure.

| Skill | File | Purpose |
|-------|------|---------|
| `/swarm` | `swarm.md` | Start the full swarm team |
| `/green-merge` | `green-merge.md` | Drain the merge queue |
| `/rebase-open` | `rebase-open.md` | Rebase all conflicting PRs |
| `/swarm-stop` | `swarm-stop.md` | Emergency stop |
| `/swarm-wind-down` | `swarm-wind-down.md` | Graceful shutdown |
| `/swarm-report` | `swarm-report.md` | Daily summary for user |
| `/bulk-pr` | `bulk-pr.md` | PR all worktrees with changes |
| `/salvage-worktrees` | `salvage-worktrees.md` | Save dirty worktrees before cleanup |
| `/health-check` | `health-check.md` | Quick codebase health scan (stdout only) |
| `/corpus-ratchet` | `corpus-ratchet.md` | Sweep corpus and update baseline |
| `/wave` | `wave.md` | Launch a parallel agent wave |
| `/bootstrap-agents` | `bootstrap-agents.md` | Discover codebase and generate domain agents |

**Frontmatter marker**: These skills carry no special frontmatter flag — they are distinguished by the orchestrator's discipline of not passing them to subagent prompts.

### 1.2 Agent-Only Skills

These skills are background knowledge that agents load into their own context. Users do not type these at the prompt. They are tagged `user-invocable: false` in frontmatter.

| Skill | File | Purpose |
|-------|------|---------|
| `/swarm-protocol` | `swarm-protocol.md` | Behavioral rules: autonomy, messaging, metrics, dedup |
| `/coding-standards` | `coding-standards.md` | Banned constructs, patterns, commit format, verification |
| `/swarm-priorities` | `swarm-priorities.md` | Roadmap alignment, priority tiers P0–P4 |

**Frontmatter**:
```yaml
user-invocable: false
```

**Design intent**: These are "load into context" skills. An agent that needs coding standards invokes `/coding-standards` as its first action — this is more reliable than pasting standards inline into every prompt, and more targeted than always loading them via a hook.

### 1.3 Dual-Use Skills

Both users and agents can invoke these. They are task-oriented commands. They carry default frontmatter (no `user-invocable: false`, no special restriction).

**Discovery**:
| Skill | File | Purpose |
|-------|------|---------|
| `/scout` | `scout.md` | Single-area deep dive |
| `/find-issues` | `find-issues.md` | Open-ended issue discovery |
| `/audit` | `audit.md` | Deep audit of a specific crate |
| `/compare` | `compare.md` | Compare two approaches |
| `/queue-scout` | `queue-scout.md` | Launch broad multi-area scouts |

**Build and Review**:
| Skill | File | Purpose |
|-------|------|---------|
| `/parser-fix` | `parser-fix.md` | TDD parser fix workflow |
| `/pr-create` | `pr-create.md` | Create a PR from current branch |
| `/pr-cleanup` | `pr-cleanup.md` | Clean up branch for review |
| `/pr-respond` | `pr-respond.md` | Respond to PR review comments |

**Workflow**:
| Skill | File | Purpose |
|-------|------|---------|
| `/worktree-pr` | `worktree-pr.md` | PR a worktree's changes |
| `/scout-report` | `scout-report.md` | Write scout findings as GitHub issues |
| `/status-drift` | `status-drift.md` | Fix computed metric drift |
| `/cycle-transition` | `cycle-transition.md` | Manage cycle boundaries |

**Monitoring**:
| Skill | File | Purpose |
|-------|------|---------|
| `/swarm-status` | `swarm-status.md` | Show PRs, issues, metrics, queue depth |
| `/ci-watch` | `ci-watch.md` | Monitor CI runs |
| `/changelog` | `changelog.md` | Generate or update changelog |

---

## 2. Agent Architecture

Agent definitions live in `.claude/agents6/` (the current active generation). There are 56 agents organized into functional groups. Agents are loaded via `subagent_type: "<agent-name>"` when spawning subagents.

Each agent definition file has YAML frontmatter:
```yaml
---
name: <agent-name>
description: <one-line purpose — used for routing>
model: sonnet | opus
color: <terminal color for teammate display>
---
```

### 2.1 Swarm Team Agents

Persistent teammates that run as long-lived coordinator processes. They do not write code themselves — they spawn subagents for code work.

| Agent | File | Role |
|-------|------|------|
| `swarm-builder` | `swarm-builder.md` | Claims build tasks, spawns worktree subagents |
| `swarm-merger` | `swarm-merger.md` | Drains the merge queue |
| `swarm-scout` | `swarm-scout.md` | Finds gaps, creates task slices |
| `swarm-reviewer` | `swarm-reviewer.md` | Reviews PRs in the review queue |
| `swarm-fixer` | `swarm-fixer.md` | Fixes CI failures and review feedback |
| `swarm-validator` | `swarm-validator.md` | Validates completed work quality |
| `swarm-strategist` | `swarm-strategist.md` | Tracks roadmap alignment, steers priorities |
| `swarm-improver-docs` | `swarm-improver-docs.md` | Continuously improves documentation |
| `swarm-improver-tests` | `swarm-improver-tests.md` | Continuously improves test coverage |
| `swarm-improver-devex` | `swarm-improver-devex.md` | Improves developer experience |
| `swarm-improver-infra` | `swarm-improver-infra.md` | Improves CI and infrastructure |
| `swarm-janitor` | `swarm-janitor.md` | Cleans up stale worktrees, branches, cruft |
| `swarm-pr-responder` | `swarm-pr-responder.md` | Responds to PR comments from external contributors |

### 2.2 Domain Specialist Agents

These are used as `subagent_type` when a builder spawns a focused implementation agent. They encode domain knowledge about specific subsystems.

**Parser domain**:
| Agent | File | Specialty |
|-------|------|-----------|
| `parser-fix-engine` | `parser-fix-engine.md` | Parser expression engine fixes |
| `parser-fix-constructs` | `parser-fix-constructs.md` | Specific Perl construct parsing |
| `parser-lexer` | `parser-lexer.md` | Lexer and tokenizer work |
| `parser-test` | `parser-test.md` | Parser test coverage |
| `parser-corpus` | `parser-corpus.md` | Corpus sweep and baseline |

**LSP domain**:
| Agent | File | Specialty |
|-------|------|-----------|
| `lsp-feature` | `lsp-feature.md` | LSP feature providers |
| `lsp-provider` | `lsp-provider.md` | LSP provider implementations |
| `lsp-navigation` | `lsp-navigation.md` | Go-to-definition, references |
| `lsp-test` | `lsp-test.md` | LSP integration tests |

**DAP domain**:
| Agent | File | Specialty |
|-------|------|-----------|
| `dap-feature` | `dap-feature.md` | DAP feature implementation |
| `dap-test` | `dap-test.md` | DAP integration tests |

**Semantic and workspace**:
| Agent | File | Specialty |
|-------|------|-----------|
| `semantic-analysis` | `semantic-analysis.md` | Semantic analyzer |
| `workspace-index` | `workspace-index.md` | Workspace symbol indexing |
| `module-resolution` | `module-resolution.md` | Module path resolution |
| `refactoring` | `refactoring.md` | Rename, extract, restructure |

### 2.3 Review Agents

Spawned by `swarm-reviewer` to perform focused code review passes.

| Agent | File | Review Focus |
|-------|------|-------------|
| `review-api` | `review-api.md` | Public API surface, semver impact |
| `review-performance` | `review-performance.md` | Allocations, hot paths, benchmarks |
| `review-scope` | `review-scope.md` | PR scope creep, unintended changes |
| `review-security` | `review-security.md` | Security and supply chain issues |
| `review-standards` | `review-standards.md` | Coding standards compliance |

### 2.4 Research Agents

Used for background investigation without code changes.

| Agent | File | Purpose |
|-------|------|---------|
| `research-web` | `research-web.md` | Web research for Perl specs, RFC lookups |
| `research-docs` | `research-docs.md` | Documentation research within the codebase |
| `research-verify` | `research-verify.md` | Verify claims and reproduce bugs |

### 2.5 Infrastructure Agents

Long-running infrastructure improvement agents.

| Agent | File | Purpose |
|-------|------|---------|
| `ci-gate` | `ci-gate.md` | Run and interpret CI gate results |
| `security-audit` | `security-audit.md` | `cargo-audit`, supply chain review |
| `dep-cleaner` | `dep-cleaner.md` | Remove unused dependencies |
| `dead-code` | `dead-code.md` | Dead code detection and removal |
| `fuzz-tester` | `fuzz-tester.md` | Fuzz target coverage |
| `mutant-killer` | `mutant-killer.md` | Mutation testing improvement |
| `coverage-filler` | `coverage-filler.md` | Coverage gap filling |
| `flaky-fixer` | `flaky-fixer.md` | Fix flaky and ignored tests |
| `test-quality` | `test-quality.md` | Test quality improvements |
| `baseline-ratchet` | `baseline-ratchet.md` | Ratchet corpus and metric baselines |
| `friction-logger` | `friction-logger.md` | Log developer friction as improvement backlog |
| `adr-writer` | `adr-writer.md` | Architectural decision records |
| `api-docs` | `api-docs.md` | Rustdoc and API documentation |
| `changelog-writer` | `changelog-writer.md` | Generate and maintain changelog |

### 2.6 Explorer Agents

Used as `subagent_type: "Explore"` for read-only investigation.

| Agent | File | Purpose |
|-------|------|---------|
| `explore-codebase` | `explore-codebase.md` | General codebase exploration |
| `explore-deps` | `explore-deps.md` | Dependency graph analysis |
| `explore-issues` | `explore-issues.md` | GitHub issue triage and categorization |
| `scout-parser` | `scout-parser.md` | Parser gap discovery |
| `scout-dap` | `scout-dap.md` | DAP gap discovery |
| `scout-security` | `scout-security.md` | Security issue discovery |

---

## 3. Hook Enforcement Layer

Hooks enforce rules that prompts cannot reliably enforce. Hooks live in `.claude/hooks/` and are registered in `.claude/settings.json`.

### 3.1 PostToolUse Hook

**Trigger**: After every `Edit`, `Write`, or `NotebookEdit` tool call.

**Actions**:
1. If the file is a `.rs` file, runs `cargo fmt` on it immediately.
2. Runs `cargo check --quiet --message-format=short` and surfaces the first 20 lines.

**Effect**: Agents always work on formatted code. Compilation errors surface before the next tool call, not at the end of a long build session.

**Configuration** (`.claude/settings.json`):
```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Edit|Write|NotebookEdit",
        "hooks": [
          {
            "type": "command",
            "command": "if [[ \"$CLAUDE_FILE_PATH\" == *.rs ]]; then cargo +1.92.0 fmt -- \"$CLAUDE_FILE_PATH\" 2>/dev/null || cargo fmt -- \"$CLAUDE_FILE_PATH\" 2>/dev/null || true; fi"
          },
          {
            "type": "command",
            "command": "if [[ \"$CLAUDE_FILE_PATH\" == *.rs ]]; then cargo check --quiet --message-format=short 2>&1 | head -20 || true; fi"
          }
        ]
      }
    ]
  }
}
```

### 3.2 TaskCompleted Hook

**File**: `.claude/hooks/task-completed.sh`

**Trigger**: When any agent attempts to mark a task as `completed`.

**Gate**: Runs `cargo fmt --all -- --check`. If the check fails, the hook exits with code 2, which **blocks task completion** and returns feedback to the agent.

**Effect**: Tasks cannot be marked complete while formatting is dirty. An agent cannot "finish" and hand off to the reviewer with inconsistent formatting.

**Why hooks, not prompts**: Cycle 2 had 30 PRs merged with 0 metrics entries. Prompts that say "write metrics before completing" are suggestions. A hook that blocks completion is enforcement.

```bash
#!/bin/bash
if ! cargo fmt --all -- --check 2>/dev/null; then
  echo "Task completion blocked: cargo fmt check failed. Run 'cargo fmt --all' before marking complete."
  exit 2
fi
exit 0
```

### 3.3 TeammateIdle Hook

**File**: `.claude/hooks/teammate-idle.sh`

**Trigger**: When a teammate thread goes idle.

**Actions**: Tracks idle state in `/tmp/claude-swarm-idle-state/`. On first idle transition, records the state. On repeated idle ticks, suppresses output to reduce noise.

**Effect**: Deduplicates repeated idle notifications in long-running swarms.

### 3.4 SubagentStart Hook (Planned)

**Not yet implemented.** Design intent:

**Trigger**: At the start of every subagent invocation.

**Actions**:
1. Inject condensed coding standards (banned constructs, commit format, patterns) into the subagent's initial context.
2. Inject known pitfalls from `.ops-perl-lsp/known-pitfalls.md`.

**Effect**: Every subagent gets a baseline without needing "Invoke /coding-standards" in its prompt. The full `/coding-standards` skill remains available for agents that need deeper detail.

---

## 4. Context Flow Design

The key insight: **context should be scoped, not broadcast**. Each layer adds specificity. Agents only receive the layers relevant to their task.

```
Layer 1: CLAUDE.md (always loaded, project-wide)
         ↓ available to everyone
Layer 2: PostToolUse hook (auto-runs on file edits)
         ↓ enforces formatting on every code change
Layer 3: Agent definition (.claude/agents6/<agent>.md)
         ↓ system prompt when spawned as subagent_type
Layer 4: Skills invoked by agent (/swarm-protocol, /coding-standards)
         ↓ loaded only when the agent invokes them
Layer 5: Handoff files (.ops-perl-lsp/handoffs/<branch>.md)
         ↓ task-specific context passed between pipeline stages
Layer 6: Source code (only files relevant to the task)
         ↓ read last, guided by the handoff
```

| Consumer | Layers Seen |
|----------|-------------|
| Orchestrator (lead) | 1, 2 |
| Swarm teammate | 1, 2, 3, 4 |
| Build subagent | 1, 2, 3, 4, 5, 6 |
| Explore subagent | 1, 2, 3 |

### Handoff Protocol

Each pipeline stage reads the **previous stage's output**, not the original source:

```
Scout subagent        → writes handoff to .ops-perl-lsp/handoffs/<branch>.md
                         (includes: problem, code excerpts, fix strategy, test template)
  ↓
Builder subagent      → reads handoff FIRST, reads source only for gaps the handoff didn't cover
                         (appends: builder → reviewer briefing with files changed, test results)
  ↓
Reviewer subagent     → reads handoff FIRST (builder briefing), scans diff only for verification
```

**Goal**: A builder reads ONLY the handoff and has everything needed to write the test and fix. For small/medium slices, no source file re-reading should be needed.

---

## 5. Task-Based Coordination

All swarm agents use the task system (`TaskCreate`/`TaskList`/`TaskUpdate`) for coordination. This is the shared queue that connects scouts to builders to reviewers.

### Lifecycle

```
Scout finds gap
  → writes handoff to .ops-perl-lsp/handoffs/<branch>.md
  → TaskCreate (status: pending, description: SLICE)
  → appends to .ops-perl-lsp/completed-slices.md

Builder polls TaskList
  → claims task (TaskUpdate: status: in_progress, owner: self)
  → spawns worktree subagent
  → subagent implements, updates handoff, commits, creates PR
  → TaskUpdate: status: completed
  → messages reviewer teammate

Reviewer picks up PR
  → reads handoff (builder briefing)
  → reviews diff
  → approves or sends back to fixer

Merger drains queue
  → gh pr list --state open --label swarm-core
  → verifies CI green before merging
```

### Deduplication

Before creating a task, the scout checks:
- `.ops-perl-lsp/completed-slices.md` — skip if already done or in-progress
- `files_touched` overlap with existing in-progress tasks — skip if conflicting

Two tasks that touch the same files cannot run concurrently. The `files_touched` field in each SLICE is the overlap-detection key.

### Deduplication Format (completed-slices.md)

Each entry records what was done so future scouts don't re-discover the same work:

```
## fix/sub-prototype-parsing
- status: completed
- pr: #1234
- files_touched: crates/perl-parser/src/parser/expressions.rs
- merged: 2026-03-15
```

---

## 6. Design Principles

### Principle 1: Hooks Enforce, Prompts Suggest

Critical rules go in hooks, not agent prompts. A prompt that says "run cargo fmt before completing" will be skipped when an agent is rushing to finish. A hook that blocks completion cannot be skipped.

Apply this principle when: a rule has been violated before despite being in a prompt.

### Principle 2: Skills Scope Context

Each agent loads only the context it needs by invoking the appropriate skill. This keeps agent context windows focused and avoids wasting tokens on irrelevant standards.

- Build agents: invoke `/coding-standards` and `/swarm-protocol`
- Scout agents: invoke `/swarm-protocol` and `/swarm-priorities`
- Review agents: invoke `/coding-standards` only

### Principle 3: Orchestrator Routes, Agents Work

The lead never writes production code. Every code change = an agent. Every investigation = an explore subagent. The orchestrator's job is to spawn the right agent with the right context and route results to the next stage.

### Principle 4: Fresh Context Over Stale

Spawn a new agent for a new context. Do not reuse a stale agent context that has accumulated noise from a previous task. Agent contexts drift. Worktree isolation enforces fresh state at the file level; spawning fresh agents enforces fresh state at the context level.

See: [feedback_scout_context_boundaries.md](../../.claude/agents6/) — same sector = resume; different sector = fresh agent.

### Principle 5: Task List Is the Queue

The shared task list (`TaskCreate`/`TaskList`/`TaskUpdate`) is the only coordination mechanism between scouts, builders, reviewers, and mergers. No direct state sharing, no shared files as queues. The task list provides atomic claim semantics: a builder marks a task `in_progress` before starting, so two builders don't claim the same work.

### Principle 6: Handoffs Carry Context

Each pipeline stage reads the PREVIOUS stage's output. A builder does not re-read everything the scout read. A reviewer does not re-read everything the builder read. The handoff file is the condensed, stage-appropriate summary.

This is a context budget optimization: subagent context windows are finite. Handoffs carry the 10% of information that matters, not the 100% that was read during discovery.

### Principle 7: GitHub Is the Source of Truth

PRs, issues, and labels are permanent record. Ops files (`.ops-perl-lsp/`) are ephemeral coordination artifacts. When a swarm cycle ends, the PRs and issues survive. The ops files are rebuilt at the start of the next cycle.

Scouts create GitHub issues (`gh issue create --label swarm-discovered`) for out-of-scope discoveries. These survive agent restarts, cycle boundaries, and context resets.

---

## Appendix A: Skill Frontmatter Reference

```yaml
---
description: <shown in /help and used for routing>
argument-hint: "<shown in autocomplete>"
user-invocable: false    # omit for dual-use; set false for agent-only
---
```

## Appendix B: Agent Frontmatter Reference

```yaml
---
name: <agent-name>          # used in subagent_type field
description: <routing hint>  # used by orchestrator when choosing agent type
model: sonnet | opus         # model to use
color: blue | green | red | yellow | purple | cyan
---
```

## Appendix C: Worktree Isolation Pattern

Every code-writing subagent uses `isolation: "worktree"`:

```
Agent(
  prompt: "...",
  isolation: "worktree",
  run_in_background: true,
  mode: "auto",
  name: "build-<branch-name>"
)
```

Worktree isolation:
- Gives the subagent its own git worktree (separate working directory)
- Prevents concurrent agents from conflicting on the same files
- Allows parallel builds on non-overlapping slices
- The `target/` directory is symlinked (configured in `.claude/settings.json`) to avoid redundant compilation

```json
{
  "worktree": {
    "symlinkDirectories": ["target"]
  }
}
```

## Appendix D: Agent Generation History

The `.claude/` directory contains multiple agent generations:
- `agents/` — generation 1: research agents only (research-docs, research-verify, research-web)
- `agents2/` — generation 2: categorized by role (generative, integration, review)
- `agents3/`, `agents4/` — generation 3–4: pipeline-oriented (issue-to-draft, draft-to-pr, pr-to-merge)
- `agents5/` — generation 5: lifecycle-focused with finalizer pattern
- `agents6/` — **current**: domain-specialist + swarm-team model (56 agents)

When spawning subagents, always use `agents6/` definitions. Earlier generations are retained for historical reference.
