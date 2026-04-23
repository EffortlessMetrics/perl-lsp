# Agent Swarm Workflow

> Status: historical wave-pattern reference. The canonical swarm runtime for
> `perl-lsp` lives in the tracked `.claude/` surfaces plus
> [SWARM_DESIGN.md](../handoff/SWARM_DESIGN.md),
> [SKILL_AND_AGENT_DESIGN.md](../reference/SKILL_AND_AGENT_DESIGN.md), and
> [ADR-0033](../adr/0033-worktree-first-disposable-workers.md).
>
> This document is retained as operator history for the older `/wave` +
> `/bulk-pr` workflow.

A practical historical reference for the parallel-agent development methodology used in
perl-lsp. For background analysis, see
[AGENTIC_SWARM_ERA.md](AGENTIC_SWARM_ERA.md) and
[AGENTIC_DEVELOPMENT.md](AGENTIC_DEVELOPMENT.md).

---

## The Swarm Pattern

The core idea: launch 30-40+ Claude Code agents in parallel, each in its own
isolated git worktree, each focused on a single well-scoped task. The human
operator acts as architect and gatekeeper -- defining the wave, monitoring
progress, triaging results, and merging the output.

This pattern works because:

- **Worktree isolation** prevents merge conflicts between concurrent agents.
  Each agent gets a full filesystem checkout branched from master.
- **Focused scope** keeps agents on task. An agent fixing one parser bug or
  adding tests to one crate rarely needs to coordinate with others.
- **Mechanical gates** (`cargo fmt`, `cargo clippy`, `cargo test`) catch broken
  output before it reaches the PR stage.
- **Disposable attempts** mean a failed agent session is cheap. Close the
  worktree, try again.

### What it is good for

| Task type | Example | Typical wave size |
|-----------|---------|-------------------|
| Parser fixes | Fix each error category from the corpus baseline | 10-20 agents |
| Test coverage | Add unit tests to every crate in a family | 15-30 agents |
| Documentation | Update every project doc for a release | 10-15 agents |
| Code cleanup | Remove dead code, unused deps, lint fixes | 10-20 agents |
| SRP extraction | Extract focused microcrates from a large crate | 5-15 agents |
| Dependency updates | Bump and validate each dependency | 5-10 agents |

### What it is not good for

- Architectural decisions that require cross-crate coordination
- Features where the design is unclear or exploratory
- Work that depends on sequential ordering (use a single agent instead)

### Context Shift Rule

Spawn a fresh worker when the work stops being "same branch, same files, same
verification loop." In practice, that means a new worker for:

- a different crate or file surface
- a different PR target
- a different tool or permission profile
- a different verification command
- a different root-cause hypothesis

Do not reuse an implementation worker just because the next task looks nearby.
Write or update the handoff and spawn again.

---

## Workflow Phases

### Phase 1: Planning

Before launching agents, establish the work scope.

1. **Identify the improvement category.** Parser fixes? Test gaps? Doc
   freshness? Lint warnings?
2. **Read baseline metrics.** Check the corpus baseline, test counts, coverage
   reports, or whatever metric defines "before."
3. **Enumerate specific tasks.** Each task must be small enough for a single
   agent session (typically 30 seconds to 10 minutes of agent wall time).
4. **Write clear prompts.** Each agent gets a focused prompt describing exactly
   what to do, what files to touch, and what success looks like.

Example planning for a parser-fix wave:
```bash
# See current error buckets
cat .ci/parser-corpus-baseline.json | jq '.error_categories'

# Identify fixable categories
# Each becomes one agent task
```

### Phase 2: Wave Dispatch

Launch agents in parallel using the `/wave` slash command or manual Agent tool
calls. Each agent runs in its own worktree with `isolation: "worktree"` and
`run_in_background: true`.

```
Agent(
  prompt: "Fix parser handling of chained method calls after array deref: $obj->@*->method()",
  mode: "auto",
  isolation: "worktree",
  run_in_background: true,
  name: "fix-chained-method-deref"
)
```

Key parameters:
- **`isolation: "worktree"`** -- mandatory for swarm work. Creates a fresh git
  worktree under `.claude/worktrees/agent-<hash>/`.
- **`run_in_background: true`** -- allows launching the next agent without
  waiting for this one to finish.
- **`name`** -- descriptive name for tracking. Use the task category and
  specific target (e.g., `test-perl-lexer-edge-cases`, `fix-hash-subscript-bareword`).

### Phase 3: Monitoring

Track agent completions as background tasks finish.

```bash
# List all worktrees and their status
cd .claude/worktrees
for d in agent-*; do
  changes=$(cd "$d" && git diff --stat HEAD 2>/dev/null | tail -1)
  if [ -n "$changes" ]; then
    echo "READY: $d | $changes"
  else
    echo "CLEAN: $d"
  fi
done
```

Typical outcomes:
- **READY** -- agent produced changes; proceed to PR pipeline.
- **CLEAN** -- agent ran but made no changes (task may have been already done
  or the agent failed silently).
- **ERROR** -- agent hit a build/test failure it could not resolve.

### Phase 4: PR Pipeline

For each worktree with changes, launch a PR agent using `/bulk-pr` or
`/worktree-pr`. The PR agent:

1. Enters the worktree directory.
2. Reviews the diff to understand what changed.
3. Runs validation: `cargo fmt --all -- --check`, `cargo clippy --workspace --lib`,
   `cargo test --workspace --lib`.
4. Fixes any issues found.
5. Creates a descriptive feature branch (`fix/...`, `test/...`, `docs/...`, `chore/...`).
6. Commits with a conventional commit message.
7. Pushes and creates a PR via `gh pr create`.
8. Returns the PR URL.

The PR pipeline itself can be parallelized: `/bulk-pr` launches one PR agent per
ready worktree, all running in background.

### Phase 5: Merge and Ratchet

Merge PRs sequentially to avoid conflicts. After each merge:

1. Verify CI passes on the merged result.
2. If the change improved a tracked metric (corpus parse rate, test count),
   update the baseline using `/corpus-ratchet`.
3. The ratchet only moves forward -- baselines never regress.

```bash
# After merging parser fixes
just corpus-sweep              # See new state
just corpus-sweep-update       # Update baseline
just corpus-sweep-check        # Verify ratchet holds
git add .ci/parser-corpus-baseline.json
git commit -m "ci: ratchet corpus baseline after parser improvements"
```

### Phase 6: Next Wave

If more work remains:

1. Rebase any remaining unmerged worktrees onto the updated master.
2. Identify what the merged changes unblocked (new error categories now fixable,
   new crates now testable).
3. Launch the next wave with updated prompts.

Typical sessions run 2-4 waves, each building on the merged results of the
previous wave.

---

## Slash Commands

### `/wave <category>`

Launch a wave of parallel agents for a specific improvement category.

| Category | What it does |
|----------|-------------|
| `parser-fixes` | One agent per known parser error bucket from `.ci/parser-corpus-baseline.json`. Uses TDD: failing test, fix, verify. |
| `test-coverage` | One agent per crate or crate family. Adds unit tests for uncovered paths. |
| `doc-updates` | One agent per documentation file. Updates for accuracy and freshness. |
| `cleanup` | Agents for unused deps (`cargo machete`), lint fixes, dead code removal, `.gitignore` updates, obsolete script deletion. |

After the wave completes, run `/bulk-pr` to create PRs for all worktrees with
changes.

### `/bulk-pr`

Scan all agent worktrees for uncommitted changes and create PRs for each.

Steps:
1. Scans `.claude/worktrees/agent-*` for worktrees with diffs.
2. Launches a parallel PR agent for each ready worktree.
3. Each agent validates, branches, commits, pushes, and creates a PR.
4. Reports a summary table of all created PRs.

Options:
- `--dry-run` -- preview which worktrees have changes without creating PRs.
- `--filter <pattern>` -- limit to worktrees matching a pattern.

### `/worktree-pr <path>`

Create a PR from a single worktree's changes. Use this when you want to
validate and publish one specific worktree rather than the entire batch.

Steps:
1. Enter the worktree.
2. Examine the diff.
3. Run `cargo fmt`, `cargo clippy`, `cargo test`.
4. Fix any issues.
5. Create branch, commit, push, PR.
6. Return the PR URL.

Branch naming convention:
- `fix/...` -- parser fixes, bug fixes
- `test/...` -- test additions
- `docs/...` -- documentation changes
- `chore/...` -- cleanup, dependencies, CI

### `/parser-fix <bug>`

TDD-driven parser fix in a worktree. This is the most structured command,
following a strict protocol:

1. **Find the root cause** -- search parser source for relevant logic.
2. **Write failing tests first** -- add tests that demonstrate the bug.
3. **Implement minimal fix** -- change as little code as possible.
4. **Verify** -- `cargo fmt`, `cargo clippy`, `cargo test`.
5. **Create PR** -- branch, commit, push, `gh pr create`.

Coding standards are strictly enforced: no `unwrap()`, `expect()`, `panic!()`,
`todo!()`, or `unimplemented!()` in production code. Tests must use
`Result<()>` return types.

### `/corpus-ratchet`

Run the parser corpus sweep, compare against the baseline, and update manifests
if improvements are detected.

Modes:
- `--system` -- full system corpus sweep (default).
- `--cpan` -- CPAN top-1000 corpus.
- `--common` -- common corpus (strict, used in CI gate).
- `--update` -- update baseline after confirmed improvements.

Key files:
- `.ci/parser-corpus-baseline.json` -- system corpus ratchet floor.
- `.ci/common-corpus-manifest.txt` -- modules that must parse cleanly (CI gate).
- `.ci/cpan-corpus-manifest.txt` -- CPAN modules that must parse cleanly.

---

## Best Practices

### Always use worktree isolation

Never have agents modify the main checkout directly. Worktree isolation means:
- No merge conflicts between concurrent agents.
- Failed attempts are trivially discarded.
- The main checkout stays clean for monitoring and coordination.

### Name agents descriptively

Good names make monitoring tractable when 30+ agents are running:
- `fix-chained-method-after-deref` (specific parser fix)
- `test-perl-module-resolution-edge-cases` (specific test target)
- `docs-update-commands-reference` (specific doc update)

Avoid generic names like `agent-1`, `task-a`, or `cleanup`.

### Run builds and tests before committing

Every agent should validate its changes before they leave the worktree:
```bash
cargo fmt --all -- --check
cargo clippy --workspace --lib
cargo test --workspace --lib
```

This catches problems early and reduces PR rejection rates.

### PR before cleanup

Create the PR first, then clean up worktrees. PRs are the durable record;
worktrees are ephemeral. If something goes wrong during cleanup, the PR
preserves the work.

### Ratchets only move forward

Baselines (corpus parse rates, test counts, coverage thresholds) are
monotonically increasing. If a change regresses a metric, fix the regression
before merging. Never lower a baseline to accommodate a broken change.

### Keep agent tasks atomic

One task per agent. An agent that fixes a parser bug should not also refactor
the test framework or update documentation. Atomic tasks are easier to review,
easier to revert, and less likely to conflict with other agents' work.

### Accept some rejection

Not every agent session succeeds. A 60-80% success rate is normal and healthy.
The cost of a failed agent session is low (a few minutes of compute). The cost
of merging bad output is high (regressions, cleanup campaigns). Prefer false
negatives over false positives.

### Use conventional commit messages

All commits follow the `type(scope): description` format:
- `fix(parser): handle chained method calls after array deref`
- `test(perl-lexer): add edge case coverage for heredoc tokenization`
- `docs(commands): update COMMANDS_REFERENCE.md with new slash commands`
- `chore(deps): remove unused serde_yaml dependency`

This makes the git log parseable and PR titles consistent.

---

## Metrics and Session Examples

### Typical session profile

| Metric | Range |
|--------|-------|
| Agents per wave | 10-40 |
| Waves per session | 2-4 |
| Agent completion time | 30 seconds -- 10 minutes |
| PRs created per session | 15-30 |
| PR merge rate | 60-80% |
| Session wall time | 1-3 hours |

### Observed session: parser fix wave

- **Wave size**: 20 agents, each targeting a distinct error category.
- **Completion**: 18 agents produced changes, 2 found nothing to fix.
- **PRs created**: 18.
- **PRs merged**: 14 (4 had regressions or incomplete fixes).
- **Corpus improvement**: parse error rate dropped from 3.2% to 2.1%.
- **Ratchet update**: baseline advanced by 47 newly-clean files.

### Observed session: full swarm day

- **Total agents launched**: 40+.
- **Categories**: parser fixes, test coverage, doc updates, code cleanup.
- **PRs created**: 20+.
- **Coverage**: parser fixes across 12 error categories, test additions
  to 8 crates, 5 documentation updates, 3 dependency cleanups.
- **Wall time**: approximately 2 hours from first agent launch to last PR merge.

### Historical peak: March 4, 2026

The single busiest day in project history:
- 191 PRs created.
- 152 commits merged to master.
- 126 test-addition PRs from a systematic coverage campaign.
- 12 new microcrate extraction PRs.
- Multiple documentation and launch-preparation PRs.

This was achieved through multiple overlapping waves of agents, each wave
building on the merged results of the previous one.

---

## Troubleshooting

### Agent produces no changes

The agent may have determined the task was already done, or it may have failed
to understand the prompt. Check the agent's output log for diagnostics. Relaunch
with a more specific prompt if needed.

### Build failure in worktree

Worktrees share the same git objects but have independent working trees. If a
worktree has stale build artifacts, run `cargo clean` in the worktree. If the
worktree is out of date with master, rebase: `git rebase master`.

### Merge conflicts between PRs

This is rare when tasks are properly scoped (one file or one crate per agent).
When it happens, merge the PRs in dependency order -- fix the most foundational
change first, then rebase the dependent PRs.

### Too many worktrees

Worktrees consume disk space. Clean up after each session:
```bash
cd .claude/worktrees
for d in agent-*; do
  changes=$(cd "$d" && git diff --stat HEAD 2>/dev/null | tail -1)
  if [ -z "$changes" ]; then
    git worktree remove "$d"
  fi
done
```

### Agent modifies files outside its scope

This is a prompt quality issue. Make prompts explicit about which files or
crates the agent should touch. Include a "do not modify" list for sensitive
files like `CLAUDE.md`, `Cargo.toml` (root), or CI configuration.
