---
description: Launch a wave of parallel agents for codebase improvement
argument-hint: "<category> e.g. 'parser-fixes', 'test-coverage', 'doc-updates', 'cleanup'"
---

# Wave: Parallel Agent Dispatch

Launch a wave of agents for: **$ARGUMENTS**

## Direct-Action Agent Template

Every agent spawned by a wave MUST use this template. No coordinator layers. No speculative spawning. The lead spawns worktree agents directly:

```
Agent(
  isolation: "worktree",
  prompt: "
    Goal: <one sentence>
    Crate: <crate name>
    Files: <exact files to edit — max 10>
    Branch: <branch name>
    Steps:
    1. <specific step>
    2. <specific step>
    3. Verify: <cargo command>
    4. Commit: <message>
    5. Push and create PR
    Optional: invoke /<skill> if branching needed
  "
)
```

**Rules:**
- Max 10 files per agent. If a task touches more, split into multiple agents with non-overlapping file surfaces.
- Each agent produces exactly one PR.
- No agent is active without: named worktree, branch, claimed file surface, verification command.
- Skills extend good execution trees — they do not replace task scoping. Invoke them only as a final optional step.

## Categories

### `parser-fixes`
For each known parser bug from `.ci/parser-corpus-baseline.json` error buckets:
- Launch an agent per fix using the direct-action template above
- Each in its own worktree (`isolation: "worktree"`)
- TDD: failing test → fix → verify
- Reference `docs/project/PARSER_EDGE_CASE_ROADMAP.md` for known issues

Example agent prompt:
```
Goal: Fix heredoc indentation parsing in perl-parser
Crate: perl-parser
Files: crates/perl-parser/src/heredoc.rs, crates/perl-parser/tests/heredoc.rs
Branch: fix-heredoc-indent
Steps:
1. Read the failing corpus test in crates/perl-parser/tests/
2. Add a regression test that reproduces the failure
3. Fix the parsing logic in crates/perl-parser/src/heredoc.rs
4. Verify: cargo fmt && cargo clippy -p perl-parser --tests && cargo test -p perl-parser
5. Commit: fix(parser): heredoc indentation edge case
6. Push and create PR with --label swarm-core
```

### `test-coverage`
Launch agents to improve test coverage across crates:
- `perl-parser-core` — CPAN-pattern tests (Moose, DBI, Try::Tiny)
- `perl-lexer` / `perl-tokenizer` — edge case coverage
- `perl-semantic-analyzer` — scope/import tests
- `perl-workspace-index` — dual indexing tests
- `perl-lsp` — LSP feature integration tests
- `perl-refactoring` — rename/extract tests
- `perl-dap` — DAP protocol tests

Example agent prompt:
```
Goal: Add Moose pattern tests to perl-parser-core
Crate: perl-parser-core
Files: crates/perl-parser-core/tests/moose_patterns.rs
Branch: test-parser-core-moose
Steps:
1. Read crates/perl-parser-core/src/ to understand current coverage
2. Add 5-10 tests covering Moose has/with/extends patterns
3. Verify: cargo fmt && cargo clippy -p perl-parser-core --tests && cargo test -p perl-parser-core
4. Commit: test(parser-core): add Moose pattern coverage
5. Push and create PR with --label swarm-improve-tests
```

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

## Spawning the Wave

For each item in the category, spawn using the direct-action template:
```
Agent(
  isolation: "worktree",
  prompt: "<filled-in template from above>",
  run_in_background: true,
  name: "<descriptive-name>"
)
```

Spawn 3-8 agents in parallel. Do not wait for one to finish before spawning the next.

## After wave completes

### Batch mode (default)
Run `/bulk-pr` to create PRs for all worktrees with changes.

### Continuous mode (`--continuous`)
Instead of bulk-PR after wave ends, feed finished builders directly into reviewers:
1. As each builder finishes, launch a `swarm-reviewer` agent on its worktree
2. Reviewer creates PR immediately if merge-ready
3. Keep `/green-merge` running to drain merged PRs
4. Keep launching new scouts via `/queue-scout` as capacity frees up
5. Run `/status-drift` after every ~5 merges

Related commands:
- `/queue-scout` — launch scouts to find new slices
- `/green-merge` — merge all passing PRs
- `/rebase-open` — rebase conflicted PRs
- `/status-drift` — fix computed metric drift
- `/salvage-worktrees` — clean up finished worktrees
