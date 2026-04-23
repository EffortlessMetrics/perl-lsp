# Era 7 Session 6 Economics: 59 PRs, 200+ Agents, Research-First Pipeline

**Date**: 2026-03-23 | **Duration**: ~7 hours

This was the most productive single session in project history. Not by a small margin.

## Session Snapshot

| Metric | Value |
|--------|-------|
| PRs merged | 59 |
| PRs created | 40+ |
| Issues filed | 20+ |
| Issues closed via triage | 35+ |
| Agents spawned | 200+ |
| Session budget consumed | ~26% of 5-hour window |
| Weekly quota delta | 73% → 81% (+8 points) |

Previous record was Era 7 Session 2: 58 PRs. This session exceeded it by one and delivered more infrastructure alongside.

## Economics at the Unit Level

### Cost per merged PR

The session consumed approximately 8 percentage points of weekly quota across 59 merged PRs.

```
0.08 / 59 = 0.14% weekly quota per merged PR
```

That is the base cost: 0.14% of a weekly Max plan budget per trusted, reviewed, merged change.

### Agents per PR

```
200+ agents / 59 merged PRs ≈ 3.4 agents per PR
```

The 3.4-agent composite breaks down as:

| Stage | Agent count | Purpose |
|-------|-------------|---------|
| Scout | ~0.5 | Filed the issue (many existed already) |
| Research-verifier | ~0.5 | Checked facts before build |
| Plan-reviewer | ~0.5 | Corrected the spec |
| Builder | ~1.0 | Implementation + test |
| Reviewer | ~0.5 | Standards pass |
| Deep-reviewer | ~0.5 | Correctness pass |

No single agent completes a PR. The pipeline is the unit of production.

### Research-to-build savings ratio

One hour of research (scout + research-verify + plan-review) prevented approximately four hours of builder time on wrong implementations.

```
Research cost : Avoided rework = 1 : 4
```

This ratio held across the session. Every plan-review that corrected a scout spec saved a builder from building the wrong thing. Builders are more expensive than reviewers; reviewers are more expensive than scouts.

## Pipeline Efficiency

The seven-stage pipeline ran at measurable quality at every layer.

### Research-verify pass

- 15+ research verifiers ran
- 8+ corrections caught before builders touched code
- Corrections included: stale file references, wrong function names, already-fixed conditions, phantom sub-patterns

### Plan-review pass

- 12+ plan reviews ran
- Corrected every scout spec it processed
- Typical corrections: wrong root cause, missing edge case, incomplete test spec, wrong crate

### Deep-review pass

- 10+ deep reviews ran
- Found bugs in every batch processed
- Not "sometimes" — every batch. This makes deep review structural, not optional.

## What Was Built

### Parser: 10 Fixes

Corpus target: ~87%+ after merge queue drains.

| Fix | Affected Files |
|-----|---------------|
| `hash_brace_depth` disambiguation | 147 |
| `prototype+` operator handling | 67 |
| `and`/`comma` RHS expression parsing | ~40 |
| `typeglob` in statement context | ~40 |
| `new()`/`PACKAGE`/`sort-grep` patterns | ~40 |
| `unclosed_paren_id` recovery | < 20 |
| `after_var_subscript` continuation | < 20 |
| Error recovery Phase 0 | infrastructure |
| Error recovery Phase 1 | infrastructure |

`hash_brace_depth` alone affected 147 files — the single largest corpus fix in project history.

### LSP UX: 12 Improvements

| Feature | Impact |
|---------|--------|
| Semicolon quick-fix | Auto-fix for the #1 user error |
| Cascade suppression | 60-80% noise reduction in diagnostics |
| Moose/Moo hover documentation | `has`, `extends`, `with` aware |
| 85 builtin hover docs | `push`, `pop`, `map`, `grep`, etc. |
| Completion sorting | Relevant items surface first |
| Hover documentation framework | Structured doc rendering |
| Pragma hints | `use strict`, `use warnings` context |
| VSCode health check command | `runHealthCheck` wired |
| VSCode walkthrough | Onboarding manifest |
| Formatting error notifications | Toast on formatter failure |
| Report Issue command | `vscode://` link to GitHub |
| Configuration grouping | Settings UI organized by category |

The semicolon quick-fix deserves emphasis. Across all Perl editors, a missing semicolon is the most common parse error. It now has a one-click fix in the IDE. That is a user experience change with immediate daily impact.

Cascade suppression is the second high-impact item. When the parser hits an error, it previously emitted one diagnostic per cascaded failure. Now it suppresses the cascade and shows only the root error. In practice this turns 5-10 red underlines into 1. Measured reduction: 60-80% of diagnostic noise eliminated.

### Infrastructure: 10 Improvements

| Change | Why It Mattered |
|--------|----------------|
| CURRENT_STATUS split into subsystem files | Eliminated 50% CI waste from merge cascade |
| Snapshot testing framework | Regression protection for LSP responses |
| Stale snapshot fixes | Unblocked PRs stuck in CI failure |
| Unwired scanner connected | 6,566 lines of existing code made active |
| `perl-test-must` crate | Cleaner test assertions across workspace |
| `perl-ast-utils` rename | Consistent naming convention |
| RELEASE.md | Release process documented |
| Walkthrough manifest | VSCode onboarding path |

The CURRENT_STATUS split deserves a full explanation, because it changed the economics of merging.

### CI Waste: The Merge Cascade Problem

Before this session, CURRENT_STATUS.md was a single generated file that every PR touched. When three PRs merged in quick succession, each one triggered a CI run. Each CI run regenerated CURRENT_STATUS.md. The regenerated file conflicted with the next PR in queue. The next CI run failed. The ops agent had to rebase, re-push, wait for CI again.

That cascade consumed approximately 50% of CI time in busy merge waves.

The fix: split CURRENT_STATUS.md into subsystem files (`lsp.md`, `tests.md`, `parser.md`, `quality.md`). Parser PRs only touch `parser.md`. LSP PRs only touch `lsp.md`. The merge conflict surface collapsed from N-to-N to near-zero.

This was the highest-leverage infrastructure change of the session: a documentation reorganization that halved CI cost.

### Strategic Architecture: 3 Directions

Three architectural decisions emerged from this session, driven partly by ChatGPT strategic input:

1. **Error recovery phases** — Phase 0 (panic recovery) and Phase 1 (token resync) built and shipped. These are the foundation for parsing files with errors instead of stopping.

2. **AST/symbol surface split** — The plan to separate AST traversal utilities from symbol resolution surfaces into distinct crates. Reduces cross-crate coupling.

3. **Test modularization** — Grouping parser tests by error category rather than file. Enables targeted regression runs instead of full suite.

None of these were on the original session agenda. All three emerged during the session and were captured as follow-up issues with concrete specs.

## Comparison to Previous Sessions

| Session | PRs Merged | Agents | Weekly Delta |
|---------|-----------|--------|--------------|
| Era 7 Session 1 | 16 | ~50 | — |
| Cycle 5 Session 3 | 26+ | ~60 | — |
| Era 7 Session 2 | 58 | ~150 | — |
| Era 7 Session 4 | 30 | 246 | +15% |
| **Era 7 Session 6** | **59** | **200+** | **+8%** |

Session 6 delivered more PRs than Session 4 at lower weekly cost (8 points vs 15 points). The efficiency gain came from the research-first pipeline reducing builder rework and from the CURRENT_STATUS fix reducing CI waste.

The comparison to Session 2 is notable: same PR count but the content was qualitatively different. Session 2 established the pipeline. Session 6 ran it at full maturity, with research-verifiers, plan-reviewers, deep-reviewers, and worktree isolation all working together.

## What Made It Work

### 1. Research-first is not optional

The mental model before this project: agents generate code, reviewers catch mistakes.

The actual model: agents generate wrong specs, research-verifiers catch wrong facts, plan-reviewers correct wrong approaches, builders implement the right thing, reviewers verify standards, deep-reviewers find edge cases.

Generation is cheap. Verification is what costs. The pipeline inverts the naive assumption: spend more on planning, spend less on rebuilding.

In this session, research-verifiers caught 8+ wrong facts before builders were spawned. Each catch saved one builder run. At 3.4 agents per merged PR, avoiding one builder run saves roughly 30% of that PR's total agent cost.

### 2. Deep review finds real bugs — every time

This is the most consistently validated finding across every session since Era 7 began:

- Era 7 Session 2: 13 PRs reviewed, 13 bugs found
- Era 7 Session 4: 11 PRs reviewed, 11 bugs found
- Era 7 Session 6: 10+ PRs reviewed, 10+ bugs found

The hit rate is 100%. Not "mostly." Not "often." Every deep review in every session found a real bug.

The implication is structural: deep review is not a quality check you run when you're worried. It is a stage you run on every PR because the prior probability of a bug is 100%.

Representative bugs found in this session:

- Completion sort key not serialized — feature silently inert
- Cascade suppression too aggressive — masking non-cascade errors
- Moose detection false-positive on non-Moose modules
- Builtin hover doc wrong signature for several functions
- VSCode command handler not connected to correct LSP method

### 3. Batch merges, rebase all at once

CI cascade is the #1 throughput limiter in merge-heavy sessions. The protocol that works:

1. Collect 3-5 merge-ready PRs
2. Rebase all against current master in one pass
3. Submit all for CI simultaneously
4. Merge when green, in order

This avoids the pattern of: merge PR 1, PR 2's base is now stale, rebase PR 2, CI starts over, merge PR 2, PR 3's base is now stale...

### 4. Snapshot drift blocks everything

Snapshot tests are regression tests for LSP responses. They compare actual JSON output against stored snapshots. When the LSP changes legitimately, the snapshot must be updated as part of the same commit.

This session had multiple PRs blocked in CI because the snapshot wasn't updated. The fix is simple: run `cargo test` locally, let the snapshot update, commit the update alongside the feature change.

The lesson: snapshot updates are implementation work, not cleanup. Treat them the same way as updating a test assertion.

### 5. Worktree isolation is a hard requirement

Two incidents in this session illustrated what happens without it:

- An agent writing to the main checkout (not its worktree) modified files that other agents were reading. The contamination cascaded: CI failures on unrelated branches, stale file states, agents building against wrong code.

- An agent using `git stash` to save work. The stash list is shared across all worktrees. When the stash was popped later by a different agent on a different PR, it restored the wrong changes into the wrong worktree.

Both failures are now checked by the preflight script. Any agent that fails preflight stops before touching code.

### 6. 200+ agents is sustainable

The infrastructure required to run 200+ concurrent agents safely:

- Git worktrees (one per agent, isolated copy of the repo)
- Per-worktree CARGO_TARGET_DIR (prevents build artifact collisions)
- No `git stash` (shared across worktrees)
- No writes to the main checkout
- Preflight check at every agent start
- Labels as state machine (agents cannot process work that another agent owns)

None of this is complex. But skipping any one item breaks the others.

At 200+ agents, the system is not bottlenecked by model capability. It is bottlenecked by control engineering: can you route work without conflicts, verify output without CI overload, and merge without cascade?

## The Pipeline in Full

```
Scout (haiku)
  → files rough spec
  → honest about uncertainty

Accuracy-Scout (haiku)
  → verifies mechanical facts
  → file paths, function names, issue status

Plan-Review (sonnet)
  → corrects approach
  → fills gaps, adds edge cases
  → never punts back to scout

Build (sonnet)
  → TDD: test first
  → implements minimal diff
  → adapts on small gaps

Review (haiku)
  → standards pass
  → pushes fixes directly to branch

Deep-Review (sonnet)
  → correctness pass
  → finds edge cases
  → not optional

Ops
  → batches 3-5 PRs
  → rebases, merges in order

Wisdom
  → captures what was learned
  → updates memory
  → logs patterns
```

Every layer catches what the previous one missed. The pipeline does not assume any individual agent is correct. It assumes the composition of all layers is.

Skipping any stage means shipping the errors that stage would have caught. The data across six sessions makes this concrete: deep review has 100% bug hit rate. Plan-review has corrected every scout spec it processed. Research-verify has caught stale facts in 8+ cases this session alone.

The pipeline is more valuable than the agents inside it.

## What This Enables

At 0.14% weekly quota per merged PR, a full weekly budget at this efficiency rate would support approximately 700 trusted, reviewed, merged changes.

That is not a projection — it requires the pipeline to run at session-6 efficiency for the full week, which requires routing quality, CI availability, and work availability to all hold. But it calibrates the ceiling.

The practical takeaway: the bottleneck is not cost, not model capability, and not code generation speed. The bottleneck is pipeline correctness. Get the verification chain right, and throughput scales.

## Transferable Patterns

### Research-first saves 4x builder time

For every hour spent on scout + research-verify + plan-review, approximately four hours of builder rework is avoided. Front-load the research.

### Every deep review finds a bug

Run it on every PR. The 100% hit rate makes skipping irrational.

### CI cascade is a documentation problem

CURRENT_STATUS.md was a single file that every PR touched. Splitting it into subsystem files cut merge conflict surface to near-zero. Any shared file that every PR modifies is a CI bottleneck in disguise.

### 200+ agents needs control engineering, not more agents

The gains at this session came from pipeline correctness, not agent count. Adding more agents to a broken pipeline adds more noise. Fix the pipeline first.

### Snapshot updates are implementation, not cleanup

Update snapshots in the same commit as the feature. A snapshot left stale is a CI failure waiting to happen.

### Stash is poison in a multi-agent environment

One stash pop from the wrong agent contaminated two branches. Prohibit it. Use `git restore` to discard or `git commit -m "wip"` to save.
