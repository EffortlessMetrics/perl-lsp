---
description: Launch a wave of parallel agents for codebase improvement
argument-hint: "<category> e.g. 'bug-fixes', 'test-coverage', 'doc-updates', 'cleanup'"
---

# Wave: Parallel Agent Dispatch

Launch a wave of agents for: **$ARGUMENTS**

## Categories

### `bug-fixes`
For each known bug or failing test:
- Launch an agent per fix using `/tdd-fix` pattern
- Each in its own worktree (`isolation: "worktree"`)
- TDD: failing test, fix, verify

### `test-coverage`
Launch agents to improve test coverage across modules:
- Identify modules with low or missing test coverage
- One agent per module or logical area
- Each agent reads the module, writes tests, verifies they pass

### `doc-updates`
Launch agents to update documentation:
- README and getting-started guides
- API reference and doc comments
- Architecture and design docs
- Changelog and migration guides

### `cleanup`
Launch agents for codebase hygiene:
- Unused dependencies
- Lint warnings
- Dead code removal
- Obsolete file deletion
- Configuration updates

### `refactoring`
Launch agents for structural improvements:
- Extract common patterns into shared utilities
- Simplify complex functions
- Improve naming consistency
- Reduce duplication

## Pattern

For each item in the category:
```
Agent(
  prompt: "<specific task>",
  mode: "auto",
  isolation: "worktree",
  run_in_background: true,
  name: "<descriptive-name>"
)
```

## After wave completes

Run `/bulk-pr` to create PRs for all worktrees with changes.

## Guidelines

- Keep each agent's scope narrow and well-defined
- 3-8 agents per wave is the sweet spot
- Bug fixes first, then tests, then docs, then cleanup
- Merge one wave before starting the next to reduce conflicts
