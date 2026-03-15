# Swarm Pack

Drop-in infrastructure for running continuous, highly-parallel development swarms with Claude Code agent teams.

## What This Is

A portable pack of agent definitions, slash commands, hooks, and a setup script that gives any repo a `/swarm` command for continuous codebase improvement. Designed for repos with many independent improvement opportunities.

## Architecture

```
Lead (orchestrator) — coordinates only, never writes code
  ├── Scout coordinators (×2)    — find gaps, write handoff files
  ├── Build coordinators (×2)    — implement in worktrees
  ├── Reviewer (×1)              — review, create PRs, auto-merge
  ├── PR Responder (×1)          — address review comments
  ├── Merger (×1)                — merge green PRs, handle drift
  ├── Validator (×1)             — verify merges actually helped
  ├── Improver-docs (×1)         — ADRs, changelog, friction log
  ├── Improver-tests (×1)        — mutants, flaky tests, coverage
  ├── Strategist (×1)            — priority alignment, roadmap
  └── Fixer (×1)                 — CI failures, regressions
```

**12 teammates, 10 operational layers, 4 persistence tiers.**

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

`setup.sh` gives you 25 portable agents + 15 skills. `/bootstrap-agents` explores YOUR codebase and generates ~25-30 domain-specific agents. Together: ~50 agents with full repo context pre-encoded, 12 named teammates, GitHub labels, issue/PR templates, and a self-improving learning loop.

## What Gets Installed

```
.claude/
  agents/
    # Core swarm (6) — lane coordinators
    swarm-scout.md        # Gap finder — priority-weighted, writes handoff files
    swarm-builder.md      # TDD implementer — reads handoffs, minimal subagent prompts
    swarm-reviewer.md     # Review + PR creation — labels, auto-merge
    swarm-fixer.md        # CI failure repair — known-pitfalls, agent-patches
    swarm-merger.md       # Sequential merge — signals validator, drift handling
    swarm-janitor.md      # Cleanup — consolidates all ops artifacts
    # Governance (3) — validation, strategy, review response
    swarm-validator.md        # Post-merge verification — catches regressions
    swarm-strategist.md       # Priority alignment — steers scouts, tracks roadmap
    swarm-pr-responder.md     # Review comment handler — addresses feedback
    # Improvers (4) — always-on background health
    swarm-improver-docs.md    # README, CHANGELOG, ADRs, friction log
    swarm-improver-tests.md   # Mutation survivors, flaky tests, coverage
    swarm-improver-devex.md   # Error messages, tooling, observability
    swarm-improver-infra.md   # Dependencies, security, dead code
    # Specialists (12) — focused capabilities
    swarm-bootstrapper.md     # Codebase discovery → domain agent generation
    review-standards.md       # Coding standards review lens
    review-security.md        # Security review lens
    review-scope.md           # Scope/focus review lens
    mutant-killer.md          # Kill mutation survivors
    coverage-filler.md        # Fill test coverage gaps
    adr-writer.md             # Architecture Decision Records
    friction-logger.md        # Friction log maintenance
    dep-cleaner.md            # Unused dependency removal
    dead-code.md              # Dead code removal
    explore-codebase.md       # Deep codebase exploration
    explore-issues.md         # GitHub issue/PR research
  commands/
    swarm.md              # /swarm — 12-teammate orchestrator with full data flows
    swarm-protocol.md     # /swarm-protocol — behavioral rules (loaded as skill)
    swarm-priorities.md   # /swarm-priorities — roadmap alignment + P0-P4 tiers
    swarm-status.md       # /swarm-status — current state aggregation
    swarm-report.md       # /swarm-report — daily check-in summary
    coding-standards.md   # /coding-standards — project standards
    pr-respond.md         # /pr-respond — address PR review comments
    bootstrap-agents.md   # /bootstrap-agents — generate domain agents
    green-merge.md        # /green-merge — drain passing PRs
    rebase-open.md        # /rebase-open — rebase conflicted PRs
    status-drift.md       # /status-drift — fix computed metrics
    queue-scout.md        # /queue-scout — launch discovery agents
    salvage-worktrees.md  # /salvage-worktrees — save dirty worktrees
  hooks/
    teammate-idle.sh      # Keeps teammates working
    task-completed.sh     # Quality gate on task completion
  settings.json           # Hook registrations
.ops/
  swarm-queue.json        # Overlap tracking
  known-pitfalls.md       # Failure knowledge base (fixer → scout/builder)
  completed-slices.md     # Scout dedup log (scout → merger)
  handoffs/               # Agent handoff files (scout → builder → reviewer)
```

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

### 3. Drift Protocol (`swarm-merger.md`, `status-drift.md`)

Replace the drift commands with your repo's computed metrics:

| perl-lsp | Your equivalent |
|----------|----------------|
| `python3 scripts/update-current-status.py` | Your status regeneration command |
| `just corpus-sweep-update` | Your baseline ratchet command |
| `just cpan-corpus-ratchet` | Your manifest ratchet command |

## Prerequisites

- Claude Code v2.1.32+ (for agent teams)
- Enable agent teams:
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
6. **Skills over file reads.** `/swarm-protocol` not `Read .claude/SWARM_PROTOCOL.md`.
7. **Handoffs carry context.** Next agent reads previous agent's summary, not raw sources.
8. **Minimal subagent prompts.** 7 lines pointing to files, not 100 lines inline.
9. **Per-unit verification.** Test the package you changed, not the workspace.

### Quality
10. **Validate merges.** Validator checks that work actually helped — regressions caught immediately.
11. **Every agent is a scout.** Discoveries outside scope become GitHub issues for fresh agents.
12. **~20% goes to improvement.** Docs, tests, devex, infra — always running, not just when idle.
13. **Review comments get addressed.** PR responder monitors and fixes feedback.

### Governance
14. **Priority-weighted discovery.** Scouts check roadmap, strategist steers away from drift.
15. **Self-improving.** Metrics analysis, agent patches, friction logs, ADRs — the system learns.
16. **4 persistence layers.** Handoffs (ephemeral) → ops files (cycle) → GitHub (permanent) → memories (cross-session).
17. **GitHub-native tracking.** Labels, issues, PR templates, auto-merge, `gh` CLI everywhere.

### Lifecycle
18. **Continuous, not batchy.** All lanes concurrent. Never batch-then-merge.
19. **Graceful shutdown.** `/swarm-wind-down` finishes work, `/swarm-stop` saves state.
20. **Session resumption.** Next `/swarm` picks up in-progress slices, open PRs, pending patches.
