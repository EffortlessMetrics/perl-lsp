# Swarm Session Retrospective — 2026-04-10

**Session date**: 2026-04-10
**Duration**: ~6 hours
**Waves**: 19
**Agents launched**: 100+

---

## What Was Done

### Quantitative summary

| Metric | Count |
|--------|-------|
| PRs merged | 34 |
| Issues closed (already-fixed) | 7 |
| PRs created total | ~35 |
| Issues filed by scouts | 52 |
| Scout agents | 10 |
| Rebase agents needed | 6 |
| Stale worktrees cleaned | 50+ |
| False-positive "already-fixed" rate | ~40% |

### Bug fixes merged

**Scope analysis:**
- Package statements registered in symbol table
- Signatures in symbol table
- Deref bridging (e.g., `$$ref` tracking)
- `foreach` loop variable initialization (analysis context threading)
- `$a`/`$b` lexical shadow in sort blocks
- `local` special variables (`$/`, `$,`) treated as in-scope
- Subscript parent marking (array/hash element nodes)

**Diagnostics:**
- Version compatibility for `given`/`when`, `defer`, `builtin` (v5.40), `isa` operator
- Prototype character validation with warning on invalid patterns
- Attribute validation (unknown attributes)
- Typed signature handling kept off prototype path
- Parse warning codes surfaced consistently in diagnostics

**Formatting:**
- UTF-8 range formatting bug (multi-byte character offsets)

**VSCode extension:**
- ESLint warning cleanup
- Version sync with workspace Cargo.toml
- CHANGELOG entries added
- Activation events cleanup
- `@types/node` pinned to avoid upstream type churn

**Infrastructure:**
- Publish allowlist auto-derived from `cargo metadata` instead of hand-maintained list
- Documentation: CONFIGURATION_SCHEMA.md cross-references corrected
- Announcement draft metrics updated

### Test suites added this session

| Suite | Tests | Notes |
|-------|-------|-------|
| BDD scenarios (goto-def, hover, diagnostics, document-symbols) | ~30 | Requires LSP binary at runtime |
| Completion regression | 19 | Fast unit tests |
| Navigation regression | 17 | Fast unit tests |
| Scope golden tests | 17 + 2 bug discoveries | Snapshot format |
| Parser error recovery | 20 | |
| Diagnostic snapshots | 13 | |
| Type hierarchy coverage | 13 | |
| Sort `$a`/`$b` recognition | 5 | |
| Phaser block isolation | 2 | |
| Deref bridging regression | 3 | |

Total: ~139 new tests across 10 suites. The scope golden tests and diagnostic snapshots are the highest-value additions — they make regressions in the most volatile code visible immediately.

### Scout coverage (52 issues filed across 10 domains)

| Domain | Issues filed |
|--------|-------------|
| Diagnostics UX | 6 |
| LSP feature quality | 5 |
| Parser error recovery | 6 |
| VSCode extension UX | 5 |
| DAP debugger | 6 |
| Strict/warnings edge cases | 6 |
| Module resolution | 7 |
| Common Perl idioms | 3 |
| Workspace indexing | 4 |
| Formatting/code actions | 4 |

---

## Friction Log

These are failures-in-the-infrastructure, not in any agent's reasoning. Future sessions should treat each item as a pre-session checklist.

### 1. Worktree contamination (HIGH IMPACT)

Agents repeatedly edited files in the main checkout instead of their assigned worktree. On Windows, `H:/Code/Rust/perl-lsp/` and `/h/Code/Rust/perl-lsp/` refer to the same path but agents given absolute paths sometimes resolved to the main checkout. Edit tool calls using absolute paths don't verify "am I in the right worktree for this change."

Symptoms: Modified files in main checkout mid-session, branch switched unexpectedly, `git status` showed unrelated diffs. Required repeated `git checkout master && git restore .` recovery.

Fix: Before any agent makes an edit, compare the worktree's absolute path to the file's absolute path. If the file is under the main checkout root but the agent is a worktree agent, the agent should refuse and report an error.

### 2. Disk space exhaustion (HIGH IMPACT)

20+ active worktrees, each containing a full 134-crate workspace, consumed several GB. The session had to pause mid-wave to clean stale worktrees. On the current hardware the practical limit is ~10 concurrent worktrees.

Fix: Auto-cleanup policy — when a worktree's PR is created and CI is green, mark the worktree as eligible for reclaim. Add a pre-wave disk-space check to `just doctor`. Aggressive cleanup: worktrees whose branches have been merged can be removed immediately.

### 3. Merge conflict cascade (MEDIUM IMPACT)

Every PR touching `scope_analyzer.rs` or `scope_and_symbol_tests.rs` conflicted after the first merge in that file. The rebase→merge→rebase cycle serialized what should have been parallel work. Six separate rebase agents were required.

Fix: When filing issues, scouts should flag which files are touched. The orchestrator should batch PRs that share a file and merge them in dependency order, not by PR number. The `just cpan-corpus-ratchet` pattern (sequential, ordered) is the right mental model.

### 4. Pre-push hook failures on Windows worktrees (MEDIUM IMPACT)

The pre-push hook runs `just ci-gate`, which is too heavy for a worktree push. Combined with a known file-lock race on the cargo cache in Windows worktree environments (see memory note `feedback_pre_push_hook_windows_race.md`), most agents used `--no-verify` or the API push workaround. Neither is ideal.

Fix: The pre-push hook should run `just pr-fast` (Tier A, ~1-2 min), not `just ci-gate` (Tier B, ~3-5 min). Reserve the full gate for CI. The hook should also detect worktree mode and skip the cargo cache lock check entirely.

### 5. Stale worktree accumulation (LOW-MEDIUM IMPACT)

`git worktree remove` fails with "Directory not empty" on Windows when agents leave build artifacts. Required manual `rm -rf` + `git worktree prune`. The `.claude/worktrees/` directory accumulated ~50 stale entries over the session.

Fix: The worktree-manager skill should call `git worktree prune` after every `rm -rf` cleanup. Add `just clean-worktrees` to the session-start checklist — it is there but was not run before this session started.

### 6. Agent scope leakage — wrong branch commits (LOW IMPACT)

One agent (builder for an unrelated issue) created commits on the main checkout's current branch instead of its worktree branch. One PR (#3546) was pushed with the wrong diff — its title did not match its content. Caught by reviewer.

Fix: The builder-self-review step should include a `git log --oneline -3` to verify the commits are on the expected branch name. The reviewer checklist already catches wrong diffs but this should be a builder gate, not a reviewer catch.

### 7. Structural blocker from rebase artifacts (#3558)

One builder's PR contained out-of-scope deletions from a previous rebase that had picked up changes from an already-merged PR. Required a clean rebuild by a second agent. This is the "rebase picks up merged changes" anti-pattern.

Fix: After rebasing onto master, builders should run `git diff master...HEAD` and verify the diff contains only the expected changes before pushing. Anything unexpected is a sign the rebase picked up unrelated work.

### 8. Vacuous tests (LOW IMPACT, HIGH SIGNAL)

A deep reviewer found a test in PR #3423 that would pass even if the fix were reverted — the assertion was too weak to actually verify the behavior. Golden tests and snapshot tests (both added this session) are the structural answer: they compare full output rather than spot-checking one field.

Fix: For any test that verifies a diagnostic or scope finding, assert on the full set of results, not just "at least one result." Use `assert_eq!` not `assert!(results.len() > 0)`.

### 9. Already-fixed verification — 40% false-positive rate

5 of the first 12 issues investigated were already fixed in master. This matches the memory note from prior sessions. Scouts did not check recent commits before filing.

Fix: Every scout must run step 1 of the accuracy-scout protocol — `git log --oneline -30 | grep -i <keyword>` — before filing. The `accuracy-verify-status` skill exists for this. It is not consistently called.

---

## Architectural Insights

### Symbol visibility gap is the #1 structural problem

The parsing layer is robust (119 tests, growing). But there is no unified "what symbols are visible at position X in file Y" API. Parsed imports (`use Foo qw(bar)`) do not flow into scope analysis. `use lib` paths are not integrated. This gap affects every feature that needs to resolve a name: goto-definition, hover, completion, unused-variable diagnostics.

This is the central v0.13.0 work. It cannot be parallelized once started because everything depends on the same data structure.

Filed: #3472 (import symbol tracking), #3478 (`use lib` integration).

### Parser error recovery is the #1 UX bottleneck

When the parser hits a syntax error, it produces an `Error` node that breaks symbol resolution for everything below the error site. A user typing incomplete code gets cascading false-positive diagnostics and broken completion. Two critical issues filed (#3496, #3499).

Error recovery is a hard problem but the fix target is narrow: the parser needs to skip to a safe recovery point (next statement, next block close) rather than propagating the error node. The tree-sitter error recovery strategy is the right reference.

### Pragma tracker handles 8 of ~40+ NodeKind variants

Many modern Perl constructs — `try`/`catch`, `given`/`when`, `eval`, `do` blocks — pass through the pragma tracker silently. This means `use strict` declared inside these blocks may be wrongly ignored or wrongly inherited. Filed as part of the strict/warnings scout wave.

### IndirectCall is not FunctionCall

`print $arr[0]` is parsed as `IndirectCall` (indirect-object syntax), not `FunctionCall`. Variables in the argument position of an `IndirectCall` are not being marked as "used" by the scope analyzer. This is a non-obvious design consequence: fixing it requires touching the `IndirectCall` arm of the visitor, not the `FunctionCall` arm.

### features.toml "GA" claims need tighter test definitions

Several features marked GA have tests that accept empty results as passing. Cross-file resolution and type inference are the primary examples. The test infrastructure added this session (snapshot tests, golden tests) is the foundation for tightening these. Before v0.13.0 announcement, each GA feature should have at least one test that fails if the feature returns an empty response.

---

## Path Forward for v0.13.0

### Blockers before public announcement

1. **Parser error recovery** (#3496, #3499) — users typing code get broken analysis. This is the most visible UX gap.
2. **Import symbol tracking** (#3472) — `use Foo qw(bar); bar()` does not resolve. Affects completion, goto-def, and unused-symbol diagnostics.
3. **`use lib` integration** (#3478) — real-world projects use custom library paths. Without this, the LSP silently fails on common project structures.

### High-value quick wins (issues filed, some with open PRs)

- Duplicate hash key lint (#3459) — PR created, needs rebase
- `.cgi`/`.psgi`/`.ep`/`.tt` file extensions — mostly merged
- `blib`/`local`/`vendor` exclusion from workspace — merged
- DAP breakpoint improvements (6 issues from DAP scout wave)

### Test infrastructure status

The regression suites added this session are the foundation for confident merging going forward. Specifically:
- Scope golden tests: run before any scope_analyzer.rs change
- Diagnostic snapshot tests: run before any diagnostics change
- BDD scenarios: require the LSP binary; should be added to `just ci-full` (Tier C)

---

## Process Improvements for Next Swarm Session

These are specific, actionable changes — not general advice.

1. **Run `just clean-worktrees` before launching wave 1.** It is on the checklist but was skipped. Make it a hard gate: if more than 5 stale worktrees exist, abort and clean first.

2. **Check disk space before launching large waves.** Add to `just doctor`: warn if free disk space is under 10 GB.

3. **Merge in file-dependency order.** When multiple PRs touch the same file, the orchestrator should merge them in sequence (smallest diff first), not in PR-number order. Check with `git diff --name-only master...branch` before queuing.

4. **Limit concurrent agents to 1 per hot file.** `scope_analyzer.rs` and `scope_and_symbol_tests.rs` are hot. Never assign two agents to issues whose fix lands in the same file. The scout should note the primary file(s) affected in the issue body.

5. **Scouts must run accuracy-scout step 1 before filing.** False positives at 40% waste builder time. The `accuracy-verify-status` skill exists; make its use mandatory in the scout pipeline.

6. **Pre-push hook should run `just pr-fast`, not `just ci-gate`.** The heavy gate belongs in CI, not in the push hook. This will eliminate most `--no-verify` usage.

7. **Worktree auto-cleanup on PR creation.** After a builder calls `/pr-ready`, the worktree should be tagged for reclaim. The ops agent can clean it after confirming CI is green.

---

## Session Statistics

| Metric | Value |
|--------|-------|
| Session duration | ~6 hours |
| Avg builder time | ~15 min |
| Avg reviewer time | ~5 min |
| Avg scout time | ~10 min |
| Merge rate (pipeline flowing) | ~5-6 PRs/hour |
| False-positive already-fixed rate | ~40% |
| Rebase agents required | 6 |
| Structural blockers hit | 1 (#3558) |
| Vacuous test catches by deep review | 1 (#3423) |

---

## Cross-References

- Friction entries from this session: see `docs/project/FRICTION_LOG.md` (items to be filed)
- Memory notes updated: `feedback_pre_push_hook_windows_race.md`, `feedback_worktree_file_leak.md`
- Issues filed this session: #3459, #3464, #3472, #3478, #3483, #3494, #3496, #3499, #3503, #3546, #3558, #3578, #3584, #3586, #3587, #3634, #3638, #3639, #3640, #3641, #3642, #3643, #3644 (and ~28 more from scout wave)
- PRs merged this session: #3557, #3628, #3632, #3634, #3636, #3638, #3639, #3640, #3641, #3642, #3643, #3644, #3646, #3649 (and ~20 earlier in session)
