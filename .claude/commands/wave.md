---
description: Launch a wave of parallel agents for codebase improvement
argument-hint: "<category> e.g. 'parser-fixes', 'test-coverage', 'doc-updates', 'cleanup'"
---

# Wave: Parallel Agent Dispatch

Launch a wave of agents for: **$ARGUMENTS**

## Categories

### `parser-fixes`
For each known parser bug from `.ci/parser-corpus-baseline.json` error buckets:
- Launch an agent per fix using `/parser-fix` pattern
- Each in its own worktree (`isolation: "worktree"`)
- TDD: failing test → fix → verify
- Reference `docs/project/PARSER_EDGE_CASE_ROADMAP.md` for known issues

### `test-coverage`
Launch agents to improve test coverage across crates:
- `perl-parser-core` — CPAN-pattern tests (Moose, DBI, Try::Tiny)
- `perl-lexer` / `perl-tokenizer` — edge case coverage
- `perl-semantic-analyzer` — scope/import tests
- `perl-workspace-index` — dual indexing tests
- `perl-lsp` — LSP feature integration tests
- `perl-refactoring` — rename/extract tests
- `perl-dap` — DAP protocol tests

### `doc-updates`
Launch agents to update documentation:
- `COMMANDS_REFERENCE.md` — new commands
- `CLAUDE.md` — new tooling
- `CONTRIBUTING.md` — current practices
- `README.md` — feature claims freshness
- New project docs for undocumented architecture

### `cleanup`
Launch agents for codebase hygiene:
- `cargo machete` — unused dependencies
- `cargo clippy` — lint warnings
- Dead code removal
- Obsolete script deletion
- `.gitignore` updates

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
