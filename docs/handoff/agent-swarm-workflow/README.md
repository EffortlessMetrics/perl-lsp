# Agent Swarm Workflow

A portable pattern for orchestrating multiple Claude Code agents in parallel to
accomplish large-scale codebase changes efficiently and safely.

## How it works

### The core idea

Instead of one agent doing everything sequentially, an **orchestrator thread**
launches **waves of focused agents**, each working in its own git worktree.
The agents run in parallel, each doing one well-scoped task. When a wave
completes, a second round of **PR agents** validates, commits, and publishes
each worktree's changes. The orchestrator tracks completions and launches the
next wave.

### Architecture

```
Orchestrator (main thread)
  |
  |-- Wave 1: Implementation agents (parallel, each in a worktree)
  |     |-- agent-fix-parser-bug      (worktree: agent-abc123)
  |     |-- agent-add-test-coverage   (worktree: agent-def456)
  |     |-- agent-update-docs         (worktree: agent-ghi789)
  |     '-- agent-cleanup-dead-code   (worktree: agent-jkl012)
  |
  |-- Bulk PR: PR agents validate + publish each worktree
  |     |-- pr-agent for agent-abc123  -> PR #101
  |     |-- pr-agent for agent-def456  -> PR #102
  |     |-- pr-agent for agent-ghi789  -> PR #103
  |     '-- pr-agent for agent-jkl012  -> PR #104
  |
  |-- Merge coordination: merge PRs, resolve conflicts
  |
  |-- Wave 2: Next category of work (repeat)
  |
  '-- Quality ratchet: update baselines so improvements stick
```

### Key principles

1. **Worktree isolation** -- Every agent that writes code operates in its own
   git worktree. This prevents merge conflicts between agents and lets them
   run truly in parallel.

2. **Orchestrator never writes code** -- The main thread only coordinates:
   dispatching agents, tracking completions, launching PR waves, and updating
   baselines.

3. **Focused agents** -- Each agent has a single, well-defined task.
   A bug fix agent does TDD: write failing test, fix, verify. A docs agent
   updates documentation. Small scope means higher success rates.

4. **PR pipeline** -- Implementation and PR creation are separate concerns.
   Implementation agents just write code. PR agents validate, branch, commit,
   push, and create the pull request.

5. **Wave pattern** -- Work is organized in waves by category (fixes, tests,
   docs, cleanup). This lets the orchestrator merge one category before
   starting the next, reducing conflict probability.

6. **Quality ratcheting** -- After merges, baselines are updated so metrics
   can only improve. Test counts, lint warnings, coverage -- once they get
   better, they stay better.

## Files in this directory

| File | Purpose |
|------|---------|
| `README.md` | This overview |
| `agent-patterns.md` | Patterns and anti-patterns for agent dispatch |
| `example-prompts.md` | Copy-paste agent prompts for common tasks |
| `setup-script.sh` | Bootstrap script for a new repository |
| `slash-commands/` | Portable slash command templates |

## Slash commands

| Command | Purpose |
|---------|---------|
| `slash-commands/worktree-pr.md` | PR a single worktree's changes |
| `slash-commands/tdd-fix.md` | TDD fix workflow (test-first) |
| `slash-commands/bulk-pr.md` | PR all worktrees at once |
| `slash-commands/wave.md` | Launch parallel agent waves |
| `slash-commands/quality-ratchet.md` | Baseline ratcheting after improvements |

## Getting started

1. Run `setup-script.sh` in your repository root
2. Customize the placeholders (`$TEST_CMD`, `$LINT_CMD`, etc.) in the copied
   slash commands
3. Start Claude Code and use `/wave` to launch your first swarm

## Language support

The templates use placeholder variables and work with any language:

| Variable | Rust example | Python example | TypeScript example | Go example |
|----------|-------------|----------------|-------------------|------------|
| `$TEST_CMD` | `cargo test` | `pytest` | `npm test` | `go test ./...` |
| `$LINT_CMD` | `cargo clippy` | `ruff check .` | `eslint .` | `golangci-lint run` |
| `$FMT_CMD` | `cargo fmt` | `ruff format .` | `prettier --write .` | `gofmt -w .` |
| `$BUILD_CMD` | `cargo build` | `python -m build` | `npm run build` | `go build ./...` |
| `$CHECK_CMD` | `cargo check` | `mypy .` | `tsc --noEmit` | `go vet ./...` |
| `$GATE_CMD` | `just ci-gate` | `make ci` | `npm run ci` | `make ci` |
