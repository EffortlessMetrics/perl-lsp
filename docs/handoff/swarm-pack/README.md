# Swarm Pack

Drop-in infrastructure derived from the live swarm control plane for running
continuous, highly-parallel development swarms with Claude Code agent teams.

## What This Is

A portable pack of agent definitions, skills, slash commands, hooks, and a setup
script that gives any repo a `/swarm` control plane for continuous codebase
improvement. Designed for repos with many independent improvement opportunities.

## Architecture

```
Lead (orchestrator) — coordinates only, never writes code
  ├── scout      — DISCOVERY: find gaps, write handoffs and issues
  ├── builder    — BUILD: implement in worktrees
  ├── reviewer   — REVIEW: review, create PRs, address comments
  ├── ops        — MERGE + VALIDATE + FIX: merge green, validate, fix CI
  └── improver   — IMPROVE (~20% of capacity, always active)
```

**5 coordinator teammates. 4 persistence tiers.**

- Thin coordinator teammates + thick subagent fanout + worktree isolation
- Each coordinator spawns 3-8 fresh focused subagents in parallel
- Every coding subagent gets its own worktree
- ~20% of capacity always goes to background improvement
- Peak: 30-60 parallel workers

## Quick Start

```bash
# 1. Install the portable infrastructure
bash path/to/swarm-pack/setup.sh

# 2. In Claude Code: discover your codebase and generate domain agents
/bootstrap-agents

# 3. Start the swarm
/swarm all

# 4. Check in periodically
/swarm-status          # quick state view
/swarm-report          # daily summary

# 5. Shut down when done
/swarm-wind-down       # graceful: finish work, merge, clean up (~20 min)
/swarm-stop            # emergency: save state, halt (~5 min)
```

`setup.sh` gives you portable agents + skills + slash commands. `/bootstrap-agents`
explores YOUR codebase and generates ~25-30 domain-specific agents. Together:
~50 agents with full repo context pre-encoded, 5 named coordinator teammates,
GitHub labels, issue/PR templates, and a self-improving learning loop.

The pack now ships both a portable `.claude/skills/swarm/` tree and compatible
`.claude/commands/` files. Skills are the canonical control plane for modern
Claude Code installs; commands remain as the compatibility surface and manual
entrypoint.

## What Gets Installed

```
.claude/
  agents/
    scout.md              # Discovery coordinator
    builder.md            # Build/worktree coordinator
    reviewer.md           # Review coordinator
    ops.md                # Merge + validate + queue-health coordinator
    improver.md           # Docs/tests/devex/infra coordinator
    bootstrapper.md       # Codebase discovery → roster refresh
    fixer.md              # CI and branch failure repair worker
    validator.md          # Post-merge claim validation worker
    pr-responder.md       # Review feedback worker
    research-*.md         # Web/docs/verification helpers
  skills/
    swarm/
      SKILL.md           # skill-native /swarm control plane
      reference/
        team-structure.md
      templates/
        teammate-prompt-template.md
  commands/
    swarm.md              # /swarm — 5-coordinator orchestrator entrypoint
    bootstrap-agents.md   # /bootstrap-agents — discover codebase, mint domain agents
    swarm-protocol.md     # /swarm-protocol — behavioral rules
    coding-standards.md   # /coding-standards — project standards
    swarm-priorities.md   # /swarm-priorities — roadmap + P0-P4 tiers
    swarm-status.md       # /swarm-status — current state aggregation
    swarm-report.md       # /swarm-report — daily check-in summary
    green-merge.md        # /green-merge — drain passing PRs
    queue-scout.md        # /queue-scout — launch discovery agents
    status-drift.md       # /status-drift — fix computed metrics
    rebase-open.md        # /rebase-open — rebase conflicted PRs
    pr-respond.md         # /pr-respond — address review feedback
    salvage-worktrees.md  # /salvage-worktrees — save dirty worktrees
    swarm-stop.md         # /swarm-stop — emergency halt
    swarm-wind-down.md    # /swarm-wind-down — graceful shutdown
  hooks/
    teammate-idle.sh      # Keeps teammates working
    task-completed.sh     # Quality gate on task completion
    subagent-stop.sh      # Worker teardown metrics
  swarm-state/            # Tracked (committed, persists across sessions)
    known-pitfalls.md     # Failure knowledge base
    completed-slices.md   # Scout dedup log
    discovered-issues.md  # Agent-flagged leads
    findings.json         # Durable control-plane findings ledger
    findings.schema.json  # Machine-readable contract for findings.json
    swarm-queue.json      # Overlap tracking
  settings.json           # Hook registrations (PostToolUse, TeammateIdle,
                          # TaskCompleted, SubagentStart, SubagentStop,
                          # PreToolUse, SessionStart)
.ops/                     # Ephemeral runtime (gitignored)
  handoffs/               # Agent handoff files (scout → builder → reviewer)
  swarm-metrics.jsonl     # Performance data
  agent-patches/          # Self-improvement proposals
  salvage/                # Emergency worktree dumps
```

## Command And Skill Layer

The portable pack installs both `.claude/skills/swarm/` and compatible
`.claude/commands/` files. Treat them as one slash-entrypoint surface at the
call site. Skills become the better packaging when you need supporting files,
agent preloading, or frontmatter controls such as `disable-model-invocation`,
`user-invocable`, `allowed-tools`, `context: fork`, or hooks.

Key frontmatter fields for that repo-local skill layer:

```yaml
---
name: parser-fix
context: fork          # Runs isolated — doesn't pollute caller context
allowed-tools: Read, Edit, Write, Grep, Glob, Bash(cargo *), Bash(git *)
user-invocable: true
---
```

- `context: fork` — skill runs in isolated context (important for agent skills)
- `allowed-tools` — enforces tool restrictions at the framework level
- `user-invocable: false` — hides internal skills from the user's skill list
- `disable-model-invocation: true` — prevents the model from firing a
  side-effecting workflow automatically

## Hooks Architecture

Hooks read JSON from stdin (not env vars). All hooks registered in `.claude/settings.json`:

| Event | What It Enforces |
|-------|-----------------|
| `PostToolUse` (Edit/Write) | Auto-format + check edited source files |
| `TeammateIdle` | Detect idle agents with unclaimed work |
| `TaskCompleted` | Block ghost completions — verify deliverables exist |
| `SubagentStart` (builder/reviewer/fixer/etc.) | Auto-inject coding standards |
| `SubagentStop` (builder/reviewer/fixer/etc.) | Record worker teardown and handoff boundaries |
| `PreToolUse` (Bash) | Block dangerous commands |
| `SessionStart` (compact) | Inject context refresh after compaction |

`WorktreeCreate` and `WorktreeRemove` are intentionally not registered by
default in the shared settings template. In Claude Code those hooks replace the
default git worktree behavior, so they should only be added when the hook
script itself creates or removes the working copy.

## Customization

### Automatic: `/bootstrap-agents`

After `setup.sh`, run `/bootstrap-agents` in Claude Code. It discovers your codebase and generates ~25-30 domain-specific agents with your actual:
- Package paths and structure
- Test commands and patterns
- Error sources and baselines
- Coding standards and banned constructs
- CI gate configuration

It also customizes the portable agents (fills in `$PLACEHOLDER` values).

### Manual (if you prefer)

Three things to customize in the portable agents:

### 2. Verification Commands (all agents)

Find-and-replace these patterns:

| Placeholder | Rust (default) | Python | TypeScript | Go |
|------------|---------------|--------|------------|-----|
| Format check | `cargo fmt --all --check` | `ruff format --check .` | `prettier --check .` | `gofmt -l .` |
| Lint | `cargo clippy -p <pkg> --tests` | `ruff check .` | `eslint .` | `go vet ./...` |
| Test (unit) | `cargo test -p <pkg>` | `pytest tests/<pkg>/` | `vitest run <pkg>` | `go test ./<pkg>/...` |
| Test (all) | `cargo test --workspace` | `pytest` | `vitest run` | `go test ./...` |
| Fast check | `cargo check` | `python -m py_compile` | `tsc --noEmit` | `go build ./...` |

### 3. Drift Protocol (merge automation + `status-drift`)

Replace the drift commands with your repo's computed metrics:

| perl-lsp | Your equivalent |
|----------|----------------|
| `python3 scripts/update-current-status.py` | Your status regeneration command |
| `just corpus-sweep-update` | Your baseline ratchet command |
| `just cpan-corpus-ratchet` | Your manifest ratchet command |

## Prerequisites

- Claude Code with agent teams enabled:
  ```json
  // ~/.claude/settings.json
  { "env": { "CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS": "1" } }
  ```

## Design Principles

See [SWARM_DESIGN.md](../SWARM_DESIGN.md) for full rationale.

### Execution
1. **Coordinators don't code.** Teammates manage lanes. Subagents do work.
2. **Fresh beats stale.** New subagent > reused context. Agent definitions are the reusable part.
3. **Parallel beats sequential.** All independent subagents in one message.
4. **Worktrees for all code changes.** No file conflicts.
5. **Overlap by files, not count.** Unlimited agents if files don't overlap.

### Efficiency
6. **Skills over file reads.** `/swarm-protocol` not `Read .claude/skills/swarm/...`.
7. **Handoffs carry context.** Next agent reads previous agent's summary, not raw sources.
8. **Minimal subagent prompts.** 7 lines pointing to files, not 100 lines inline.
9. **Per-unit verification.** Test the package you changed, not the workspace.

### Quality
10. **Validate merges.** Ops verifies that work actually helped — regressions caught immediately.
11. **Every agent is a scout.** Discoveries outside scope become GitHub issues for fresh agents.
12. **~20% goes to improvement.** Docs, tests, devex, infra — always running, not just when idle.
13. **Review comments get addressed.** PR responder monitors and fixes feedback.

### Governance
14. **Priority-weighted discovery.** Scouts check roadmap, and the lead can spawn strategist-style analysis when drift appears.
15. **Self-improving.** Metrics analysis, agent patches, friction logs, ADRs — the system learns.
16. **4 persistence layers.** Handoffs (ephemeral) → ops files (session) → GitHub (permanent) → memories (cross-session).
17. **GitHub-native tracking.** Labels, issues, PR templates, auto-merge, `gh` CLI everywhere.

### Lifecycle
18. **Continuous, not batchy.** All lanes concurrent. Never batch-then-merge.
19. **Graceful shutdown.** `/swarm-wind-down` finishes work, `/swarm-stop` saves state.
20. **Session resumption.** Next `/swarm` picks up in-progress slices, open PRs, pending patches.
