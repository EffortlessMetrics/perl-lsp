# Skill and Agent Architecture Design

This document defines the current swarm execution model for `perl-lsp`.

These rules are the default for **swarm mode**: parallel, PR-shaped execution
with thin coordinators and disposable workers. They are intentionally stricter
than the default single-session flow. Quick targeted edits that stay in one
file surface and one verification loop can still stay in the main
conversation.

The important question is no longer "how many agents do we have?" It is:

- what boundary gets a new worktree
- what boundary gets a new worker
- what knowledge is pre-encoded
- what state is handed off
- what behavior is enforced mechanically

For the governing decision, see
[ADR-0033](../adr/0033-worktree-first-disposable-workers.md).

## Current Model

The swarm is built from four layers:

1. **Thin persistent coordinators**: `scout`, `builder`, `reviewer`, `ops`,
   `improver`
2. **Disposable specialists**: fresh workers spawned for a narrow task
3. **Structural knowledge**: skills, templates, hooks, `CLAUDE.md`
4. **Volatile task state**: handoffs, worktrees, PRs, issues, queue files

The persistent team is intentionally small. Most code mutation happens in
short-lived workers with isolated worktrees.
The specialist-worker inventory lives in
[`../../.claude/agents/agent-roster.json`](../../.claude/agents/agent-roster.json),
while [`../../.claude/agents/AGENT_CATALOG.md`](../../.claude/agents/AGENT_CATALOG.md)
summarizes the roster and records spawned-by, handoff-to, and first-entrypoint
metadata for every tracked specialist.

## Boundary Doctrine

### 1. Worktree Boundary

The worktree is the write-isolation boundary.

Default rule:
- any PR-shaped code change gets its own worktree

Create a new worktree when:
- the change should merge separately
- the change may need independent rebasing
- the verification loop differs from nearby work
- another worker may touch overlapping files

Read-only scouting can stay in the current session. Code mutation should not.

### 2. Worker Boundary

The worker is the context and permission boundary.

Spawn a new worker when any of these change:
- objective
- dominant crate or file surface
- tool or permission profile
- verification command
- PR/branch target
- root-cause hypothesis

This means "similar" work is not enough reason to reuse a worker. A parser fix
worker should not silently become a docs worker just because the same session
is available.

### 3. Coordinator Boundary

Persistent coordinators own routing, not implementation.

Coordinator responsibilities:
- `scout`: discover, dedup, create slices, write handoffs
- `builder`: claim tasks, spawn worktree workers, track branch state
- `reviewer`: review one PR at a time, create drafts, route feedback
- `ops`: merge green PRs, watch CI, validate recent merges, handle queue health
- `improver`: spend a bounded share of capacity on docs/tests/devex/infra

Coordinators should avoid carrying detailed implementation context from task to
task. Their job is to replace workers cheaply, not to become one.

## Where Information Lives

### Durable Knowledge

Stable, reusable instructions belong in structural artifacts:

- `CLAUDE.md`: repo-wide rules and commands
- skills and supporting files: reusable procedures and reference material
- hooks: deterministic enforcement
- templates: handoff/PR/report formats
- coordinator prompts: role boundaries and operating loops
- worktree-manager runtime: `.ops-perl-lsp/worktree-manager/state.json` for reusable slot lifecycle state

Durable knowledge is pre-encoded so it is not restated in every spawn prompt.
Stable swarm conclusions should also be recorded in the tracked findings ledger
at [`../../.claude/swarm-state/findings.json`](../../.claude/swarm-state/findings.json)
instead of being left in chat transcripts.

### Volatile State

Task-specific or branch-specific state belongs in:

- handoff files
- the worktree
- PRs and issues
- queue/state files
- short reviewer briefings and merge notes

Volatile state should move forward through handoffs, not by keeping a worker
alive indefinitely.

### Tracked Swarm Findings

Use `.claude/swarm-state/findings.json` for durable control-plane findings:

- roster invariants
- slash-entrypoint surface rules
- export-versus-trunk decisions
- hook lifecycle constraints
- other findings that should change how the swarm operates or documents itself

Do not put product bugs there. Product or codebase leads belong in
`discovered-issues.md`. Reusable failure lessons belong in `known-pitfalls.md`.

## Skills, Hooks, And Handoffs

### Skills

Skills hold reusable procedure and domain knowledge.

Use a skill when:
- the instructions are stable across runs
- multiple workers need the same procedure
- supporting files or templates help keep the hot prompt small

In this repo today, `/swarm`, `/swarm-protocol`, `/coding-standards`,
`/swarm-priorities`, `/plan-fix`, `/parser-fix`, `/verify-build`, and
`/worktree-manager` ship from `.claude/skills/`. Other slash entrypoints currently live under
`.claude/commands/`. They are interchangeable at the call site unless
frontmatter or bundled resources intentionally change the behavior. The runtime
docs should describe that surface accurately.

Subagents do not inherit the caller's loaded skills automatically. If a worker
needs repo procedure or domain knowledge, name the required skills in the
worker prompt or encode the task itself as a `context: fork` skill.
For stable startup procedure, prefer preloading those skills directly in agent
frontmatter instead of relying only on prose reminders.

Typical slash entrypoints in this repo today:
- `/coding-standards`
- `/swarm-protocol`
- `/swarm-priorities`
- `/parser-fix`
- `/verify-build`
- `/plan-fix`
- `/worktree-manager`

### Skill Frontmatter Cheat Sheet

- `disable-model-invocation: true`: the user can invoke the skill, but the
  model cannot trigger it automatically.
- `user-invocable: false`: hides the skill from the slash-command menu, but
  does not stop model invocation on its own.
- `allowed-tools`: grants the listed tools while the skill is active.
- `context: fork`: runs the skill in an isolated worker context instead of the
  current conversation.

### Hooks

Hooks hold behaviors that must happen every time.

Use a hook when:
- a completion gate must be enforced
- a dangerous command must be blocked
- context must be re-injected after compaction
- idle workers need deterministic nudging

Prompts express judgment. Hooks enforce invariants.

### Handoffs

Handoffs are the continuity mechanism across workers and lanes.

A handoff should carry:
- problem summary
- files or crate surface
- fix strategy or hypothesis
- known pitfalls
- verification command
- reviewer-facing notes after implementation

The handoff exists so the next worker does not need to reconstruct context from
raw source files.

### Local Todo Lists

Workers and coordinators should use the local todo or task tool for the current
slice or lane. Each item should name the skill or command to invoke for that
step:
- review handoff with `/plan-fix`
- implement with `/parser-fix`
- verify with `/verify-build`
- publish with `/pr-create`

This keeps procedure attached to the current task instead of relying on
ambient remembered instructions.

When a task produces a durable swarm finding, capture it in
`.claude/swarm-state/findings.json` and validate it with
`python3 scripts/validate_swarm_findings.py`.

## Spawn Rules

### New Worktree

Create a new worktree when the result should land as a separate PR or when the
change needs independent rebasing, validation, or review.

### New Worker

Create a new worker when the context shifts materially:
- new crate
- new PR
- new verification loop
- new permissions/tools
- new root-cause hypothesis

### New Skill

Create or expand a skill when the procedure is stable enough to reuse and is
likely to appear again.

### New Hook

Create or expand a hook when the behavior should be guaranteed, not merely
requested.

### No New Worker

Stay in the current worker only when the work is sequential, branch-local, and
shares the same:
- objective
- file surface
- verification loop
- tool profile

If those drift, retire the worker and spawn again.

## Prompt Design

### Good Spawn Prompt

A good worker prompt is short and specific. It should name:
- worktree
- branch
- file or crate surface
- goal
- verification command
- handoff path
- relevant skill(s)

Example:

```text
Worktree: agent-fix-heredoc-queue
Branch: fix/heredoc-queue
Crate: perl-parser
Files: crates/perl-parser/src/heredoc.rs crates/perl-parser/tests/heredoc.rs
Goal: fix queued heredoc replay after nested interpolation
Verify: cargo fmt --all && cargo clippy -p perl-parser --tests && cargo test -p perl-parser
Read .ops-perl-lsp/handoffs/fix-heredoc-queue.md, then invoke the slash entrypoints /coding-standards and /parser-fix
```

### Bad Spawn Prompt

Avoid:
- large inline rule dumps
- multiple unrelated goals
- vague file scope
- implicit verification
- "and while you're there" work

Those are signals that the task needs to be split.

## Operating Guidance

1. **Prefer fresh workers to clever reuse.**
2. **Prefer separate worktrees to shared mutation.**
3. **Prefer structural knowledge to repeated prompt prose.**
4. **Prefer handoffs to context resurrection.**
5. **Prefer hook gates to advisory wording.**

## Relationship To Older Swarm Generations

The repo still contains historical agent generations and experiments
(`.claude/agents2` through `.claude/agents6`). Those are useful as history and
donor material, but they are not the architectural center of gravity anymore.
Compatibility-only donor agents also live outside the archived roster under
`.claude/agents-compat/`.

The current control plane is defined by:
- [`.claude/README.md`](../../.claude/README.md)
- [`.claude/agents/README.md`](../../.claude/agents/README.md)
- [`.claude/commands/swarm.md`](../../.claude/commands/swarm.md)
- [`.claude/skills/swarm/SKILL.md`](../../.claude/skills/swarm/SKILL.md)
- [`.claude/skills/swarm/reference/team-structure.md`](../../.claude/skills/swarm/reference/team-structure.md)
- [`.claude/skills/swarm/templates/teammate-prompt-template.md`](../../.claude/skills/swarm/templates/teammate-prompt-template.md)
- [`.claude/settings.json`](../../.claude/settings.json)
- [`docs/handoff/SWARM_DESIGN.md`](../handoff/SWARM_DESIGN.md)

The pack under `docs/handoff/swarm-pack/` is a derived export for adoption in
other repos. It should mirror the live control plane, not compete with it.

[`docs/project/AGENT_SWARM_WORKFLOW.md`](../project/AGENT_SWARM_WORKFLOW.md) is
historical/operator reference material, not the live runtime contract.
