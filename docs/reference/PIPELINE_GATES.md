# Pipeline Gates — Gate Model Reference

**Status**: Active doctrine (introduced 2026-04-27)
**Related**: [CLAUDE.md](../../CLAUDE.md) | [SKILL_AND_AGENT_DESIGN.md](./SKILL_AND_AGENT_DESIGN.md) | [ISSUE_PLAN_DOCTRINE.md](./ISSUE_PLAN_DOCTRINE.md)

---

## Overview

The perl-lsp pipeline is organized into **7 gates** (coarse stages) with **multiple agents working within each gate**. This gate framing is the actual state machine; the labels and sign-off receipts tracked in CLAUDE.md are bookkeeping for what happened within each gate.

The key shift from a linear sequence to a gate model:

- **A gate** is a coarse stage with a clear "entry condition" and "exit condition."
- **Agents within a gate** are the workers that collectively satisfy the gate's exit condition.
- **Sequencing within a gate** is preferred when agents build on each other's output, but it is not strict — parallel agents within a gate are fine when they don't depend on each other.
- **Some gates may be skipped** when they are not relevant for a given PR's nature.

This document defines the 7 gates, the agents within each, when to skip, how agents within a gate sequence, and how the orchestrator routes using this model.

---

## Learning is continuous; consolidation is dedicated

**Every agent in every gate captures learning artifacts** when something novel is encountered. This is existing doctrine (CLAUDE.md: "Learning is continuous — every agent-wrapup captures what was learned.").

Agents produce learning artifacts as they work:
- Memory candidate notes ("this pattern recurred — worth saving")
- Doctrine gap comments ("the skill missed this; needs update")
- Follow-up issue candidates ("found an adjacent bug; should file separately")
- Prompt improvement notes ("the spec was ambiguous on X; future specs should clarify")
- Structured comments with `## Lesson:` or `## Pattern:` headers that later agents can read

**Gate 7 is not where learning happens — it is where captured learning is consolidated** into durable, actionable artifacts: memory entries, doctrine updates, follow-up issues, skill improvements. See Gate 7 for details.

---

## The 7 Gates

| Gate | Name | Exit condition | Agents |
|------|------|----------------|--------|
| **1** | Identify | Accurate, builder-ready problem statement filed | scout, accuracy-scout, research-verifier |
| **2** | Spec | Scoped, project-aligned approach ready for build | plan-reviewer, oppositional-planner, advocatus-diaboli, architecture-reviewer, maintainer-issue, spec-planner |
| **3** | Build | Well-tested, implemented PR created | red-tdd, builder, green-tdd |
| **4** | Review/improve | PR passes three-axis review | reviewer, maintainer-pr, refactor-planner, green-refactor, reviewer-deep, diff-auditor |
| **5** | CI green | Live CI actually green on current HEAD SHA | green-ci, pr-responder |
| **6** | Merge | Changes land on master | ops |
| **7** | Learn | Captured learning consolidated into durable artifacts | wisdom, memory-recalibrator, orchestrator-level filing |

---

## Gate 1 — Identify

**Purpose**: Get the issue right. File an accurate, builder-ready problem statement.

**Exit condition**: The issue body has correct file paths, correct function names, a verified root cause, and enough signal that a plan-reviewer can produce a spec without re-scouting.

**Agents within Gate 1**:

| Agent | Model | Role |
|-------|-------|------|
| **scout** | haiku | Broad discovery — find the problem, file roughly-right spec |
| **accuracy-scout** | haiku | Mechanical fact check — verify file:line, function names, issue status against master |
| **research-verifier** | haiku | External fact check — Perl semantics, LSP spec, crate API claims |

**Within-gate ordering**: accuracy-scout runs first. research-verifier reads corrected facts (accuracy-scout fixes the line numbers and function names that research-verifier needs to be grounded). This is a strict ordering within Gate 1.

**When to skip Gate 1 entirely**: Skip if the PR is a known-good fix-forward to a recent regression (e.g., master fmt cascade detected during queue investigation, root cause already known) or a maintainer-direct change where scope is unambiguous. In these cases the issue is effectively pre-filed by the session context.

**When to skip agents within Gate 1**:
- Skip accuracy-scout + research-verifier for trivially-scoped issues (1-line fmt cleanup, dependency version bump) where no external claims are being made.
- Skip research-verifier when the issue makes no Perl-semantics, LSP-spec, or external-API claims.

---

## Gate 2 — Spec

**Purpose**: Produce an accurate, scoped, project-aligned proposed approach. Make the build decision (BUILD/DEFER/CLOSE).

**Exit condition**: Issue has `plan-reviewed` label. Optionally: impl branch with `.spec/` files if spec-planner ran.

**Agents within Gate 2**:

| Agent | Model | Role |
|-------|-------|------|
| **plan-reviewer** | sonnet | Improve the plan — fills gaps, corrects root cause, adds edge cases |
| **oppositional-planner** | haiku | Surface objections, overlooked alternatives, risk flags |
| **advocatus-diaboli** | haiku | Challenge premise — should this exist at all? BUILD/DEFER/CLOSE |
| **architecture-reviewer** | haiku | Verify design fits microcrate layering, dependency direction, type placement |
| **maintainer-issue** | haiku | Project vision check — aligns with perl-lsp goals, roadmap, user base |
| **spec-planner** | haiku | Create impl branch, write `.spec/` checklist and acceptance files |

**Within-gate ordering**: The verification agents (oppositional, diaboli, architecture, maintainer-issue) read the accuracy-corrected issue from Gate 1 and each other's comments. Each builds on the previous. Plan-reviewer synthesizes all prior verdicts. Spec-planner runs after plan-reviewer, creating the impl branch. See CLAUDE.md Label-based routing for the full ordering.

**When to skip Gate 2 entirely**: Skip for trivially-mechanical PRs (fmt cascade fixes, version bumps) where the scope is definitionally unambiguous. The "plan" is "apply the formatter."

**When to skip agents within Gate 2**:
- Skip accuracy/research agents: already ran in Gate 1 (Gate 2 reads their output).
- Skip architecture-reviewer: for docs-only or test-only PRs that don't change crate boundaries or inter-crate dependencies.
- Skip maintainer-issue: if the issue's premise is uncontested and explicitly aligned with current direction (e.g., "fix the bug that just broke master" — the issue is trivially aligned).
- Skip oppositional-planner + advocatus-diaboli: for pure fix-forwards with one obvious correct approach and no design space.
- Skip spec-planner: for trivially small PRs where the builder can proceed directly from the issue spec (the "impl branch + `.spec/` files" overhead exceeds the planning value).

---

## Gate 3 — Build

**Purpose**: Produce a well-tested, implemented PR.

**Exit condition**: PR created with green tests, clean build, PR targets the right branch.

**Agents within Gate 3**:

| Agent | Model | Role |
|-------|-------|------|
| **red-tdd** | haiku | Write failing tests before implementation — define "done" before builder starts |
| **builder** | sonnet | Check out impl branch (spec + red tests), implement, verify, open PR |
| **green-tdd** | haiku | Add edge case, boundary, and regression tests after builder implements |

**Within-gate ordering**: strict. red-tdd → builder → green-tdd. Each step is a prerequisite for the next.

**When to skip agents within Gate 3**:
- Skip red-tdd: for trivially-mechanical PRs (fmt fix, docs-only) where there is nothing to test-drive.
- Skip green-tdd: for pure docs-only PRs or changes with no code logic. For code PRs, green-tdd is recommended — it's where edge cases that slip through builder testing are caught.

---

## Gate 4 — Review/Improve

**Purpose**: Ensure the PR is the right thing, built for what the codebase needs, and built the right way.

**Exit condition**: All three axes of the triangulation pass. See Three-Axis Triangulation below.

**Agents within Gate 4**:

| Agent | Model | Role |
|-------|-------|------|
| **reviewer** | haiku | Standards check — banned patterns, scope, formatting; pushes fixes directly |
| **maintainer-pr** | haiku | Project vision — does implementation fit perl-lsp's direction and quality bar? |
| **refactor-planner** | haiku | Refactor analysis — identify simplification, reuse, dead code, type tightness |
| **green-refactor** | sonnet | Execute refactor plan while keeping tests green |
| **reviewer-deep** | sonnet | Correctness check — does the logic work? edge cases? regressions? |
| **diff-auditor** | haiku | Final diff check — coherent, clean, matches spec, no cross-PR artifacts |

**Within-gate ordering**: reviewer runs first (standards are cheapest to fix early). maintainer-pr, refactor-planner, and diff-auditor can run in parallel once reviewer has finished. green-refactor follows refactor-planner. reviewer-deep is the final Gate 4 agent — it reads all prior Gate 4 comments.

**When to skip Gate 4 agents**:
- Skip refactor-planner + green-refactor: when the PR is already minimal and clean (e.g., a 1-line fix). These are value-adding only when there is refactoring opportunity.
- Skip reviewer-deep: for docs-only PRs (already handled in `reviewer-decide.md` fast-track). For all production-code PRs, reviewer-deep is mandatory — it is the correctness gate.
- Skip maintainer-pr: when the issue's alignment was already comprehensively reviewed in Gate 2 maintainer-issue and the PR is a direct implementation without scope drift.
- Never skip diff-auditor for PRs from external sources (claude-burst, codex-burst, diffguard-bot): cross-PR artifact contamination is a known failure mode.

### Three-Axis Triangulation

Gate 4 triangulates three axes. A PR that clears only one axis does not pass Gate 4.

**Axis 1 — Right thing** (matches user/issue intent):
- reviewer: did the builder implement what the issue asked for, with correct scope?
- maintainer-pr: does the implementation fit perl-lsp's direction and user base?

**Axis 2 — What the codebase needs** (matches project direction + architecture):
- architecture-reviewer (if Gate 2 was thin): does the design fit microcrate layering?
- refactor-planner: is the code as clean and reusable as it should be?

**Axis 3 — Right way** (correctness, idiomatic, regression-safe):
- reviewer-deep: does the logic work? edge cases handled? regressions introduced?
- diff-auditor: is the cumulative diff coherent, clean, and artifact-free?
- green-tdd: did the tests cover the key paths? any bugs exposed after the fact?

---

## Gate 5 — CI Green

**Purpose**: Verify live CI is actually green on the current HEAD SHA. Not a label from an earlier push — the real thing.

**Exit condition**: `ci-green` label is present and the HEAD SHA it was applied to matches the current PR head.

**Agents within Gate 5**:

| Agent | Model | Role |
|-------|-------|------|
| **green-ci** | haiku | Verify all checks pass on current HEAD SHA — no stale green |
| **pr-responder** | haiku | Fix CI failures, validate-title issues, linter warnings — iterate until green |

**Within-gate ordering**: green-ci first. If it finds failures, pr-responder fixes them, then green-ci re-verifies.

**Skip criteria**: Never skip Gate 5. Live CI is the merge gate. A stale `ci-green` label from a prior push does not count. When master gets a CI fix, use `gh pr update-branch` (not `gh run rerun`) to propagate it to queued PRs, then re-run green-ci.

---

## Gate 6 — Merge

**Purpose**: Land the changes on master safely.

**Exit condition**: PR is merged. Master stays green.

**Agents within Gate 6**:

| Agent | Role |
|-------|------|
| **ops** | Merge batch of up to 3 PRs, wait for green between batches, run corpus ratchet after parser fixes |

**Skip criteria**: Never skip Gate 6. The merge is the point.

**Ops protocol**: See CLAUDE.md Merge Queue Protocol. Key: batch of 3, wait for green between batches, `just cpan-corpus-ratchet` after parser fix merges.

---

## Gate 7 — Learn

**Purpose**: Take the learning artifacts captured throughout Gates 1-6 and consolidate them into durable, actionable information: memory entries, doctrine updates, follow-up issues, skill improvements.

**This is consolidation, not capture.** Capture happens continuously throughout all gates — every agent surfaces novel patterns as they work. Gate 7's job is to organize and elevate those captures, not to discover learning from scratch at the end of the pipeline.

**Exit condition**: Novel learning from the issue→PR→merge trail is preserved in memory, doctrine, or follow-up issues. Stale or duplicate memory entries are cleaned up on schedule.

**Agents within Gate 7**:

| Agent | Model | Role |
|-------|-------|------|
| **wisdom** | sonnet | Read the full issue→PR→merge trail; synthesize learning artifacts into memory entries, doctrine gaps, process improvements |
| **memory-recalibrator** | haiku | Periodic (cron) scan — consolidate fragments, retire stale entries, merge overlapping memories |
| **(orchestrator)** | — | Execute consolidation — file the issues, write memory entries, update CLAUDE.md, post doctrine corrections |

**Three-axis consolidation**: Gate 7 uses the same three axes as Gate 4, but applied to what was learned:
- Learning about the *thing* (the issue, the user need) → memory entries about domain patterns
- Learning about what the *codebase needs* (architecture, debt, patterns) → architecture issues, refactor follow-ups, debt-tracker updates
- Learning about *how we work* (process gaps, agent failures, doctrine gaps) → CLAUDE.md doctrine updates, skill improvements, agent definition updates, process issues

**What upstream agents capture** (not Gate 7's job to discover):
- reviewer-deep comments with `## Pattern:` or `## Lesson:` headers
- builder notes about spec ambiguity or unexpected coupling
- green-tdd notes about edge cases the builder missed
- oppositional-planner findings that turned out to be right
- diff-auditor catches of cross-PR artifact contamination patterns

**Gate 7 inputs**: all of the above, plus the merge commit context, and any memory-candidate notes surfaced in agent wrapups.

**When to run wisdom**:
- Run for: cluster winners, master-bit-rot fixes, novel failure modes, doctrine corrections, salvage decisions, any trail where upstream agents flagged `## Pattern:` or `## Lesson:` in their comments.
- Skip for: trivial mechanical PRs (1-line fmt cleanups, dependency version bumps, docs-only PRs with no novel learning captured by upstream agents).

**When to run memory-recalibrator**: On cron, not per-PR. This is the maintenance layer — it runs regardless of throughput pressure.

---

## Within-Gate Ordering

When to strictly sequence agents within a gate versus running them in parallel:

**Strict sequence required** (agent B reads agent A's output):
- Gate 1: accuracy-scout → research-verifier (research grounds itself on corrected facts)
- Gate 1: accuracy-scout → oppositional-planner (objections are more useful when facts are right)
- Gate 2: all verification agents → plan-reviewer (plan-reviewer synthesizes all prior verdicts)
- Gate 2: plan-reviewer → spec-planner (spec-planner works from the approved plan)
- Gate 3: red-tdd → builder → green-tdd
- Gate 4: reviewer → reviewer-deep (deep review reads reviewer's findings)
- Gate 4: refactor-planner → green-refactor
- Gate 5: green-ci → pr-responder → green-ci (iterate)

**Parallel acceptable** (agents don't read each other's output within the same pass):
- Gate 4: reviewer, maintainer-pr, and diff-auditor can run in parallel once reviewer has finished
- Gate 4: refactor-planner and diff-auditor can run in parallel
- Gate 7: wisdom and memory-recalibrator are independent (different triggers and cadences)

---

## Labels as Bookkeeping, Not State Machine

Labels track what happened within each gate. They are sign-off receipts, not the state machine itself.

**The state machine is the gate model**: "what gate is this PR in? what within that gate is needed for this PR's nature?"

**Labels answer the question**: "which agents within this gate have already completed their pass?"

This means:
- Presence of a label = that agent ran and signed off
- Absence of a label = that agent hasn't run yet (or was intentionally skipped)
- The orchestrator may intentionally skip a label if the gate check is trivially satisfied for this PR

Labels are not ordered steps that must all be collected before proceeding. The orchestrator checks: "is the exit condition for this gate met, given this PR's nature?" — not "have all possible labels been set?"

For the full label taxonomy, see CLAUDE.md Pipeline State Labels.

---

## Orchestrator Routing by Gate

The orchestrator asks: **"What gate is this PR in? What within that gate is needed for this PR's nature?"**

This replaces: "What label is missing? → Dispatch that agent."

### Routing by gate:

```
PR has no issue?                     → Gate 1: spawn scout
Issue filed but not verified?        → Gate 1: spawn accuracy-scout, then research-verifier
Issue verified, no plan?             → Gate 2: spawn verification agents, then plan-reviewer
Plan done, no impl branch?           → Gate 2: spawn spec-planner, then red-tdd
Red tests done, no PR?               → Gate 3: spawn builder
PR created, no review?               → Gate 4: spawn reviewer → (parallel) maintainer-pr, diff-auditor → reviewer-deep
Review done, CI not verified?        → Gate 5: spawn green-ci
CI green, not merged?                → Gate 6: spawn ops
Merged, novel learning captured?     → Gate 7: spawn wisdom
```

**Adapt routing to PR nature**:
- Trivial fmt fix → enter at Gate 3 (already identified, no spec needed)
- Docs-only PR → Gate 4 skips reviewer-deep
- External-source PR → Gate 4 never skips diff-auditor
- 1-line fix, no novel patterns → Gate 7 skipped

---

## Worked Examples

### Example 1 — Trivial 1-line fmt fix (like #7031)

**PR #7031** was a master fmt cascade fix: 5 files needed rustfmt line-wrapping after PR #6748 introduced over-width lines. Pure formatting, no logic change.

**Gate 1 — Identify**: Skipped. Root cause identified by orchestrator during queue-unlock investigation. Issue pre-filed with known scope. No accuracy-scout or research-verifier needed.

**Gate 2 — Spec**: Skipped. The "spec" is "run cargo xtask fmt on the affected files." No design space, no architecture questions, no alignment debate.

**Gate 3 — Build**: Builder applies formatter. No red-tdd (nothing to test-drive). No green-tdd (no logic). Gate 3 is minimal.

**Gate 4 — Review/Improve**: reviewer confirms scope (formatting only, no logic changes), diff-auditor confirms no cross-PR artifacts. Skip refactor-planner (nothing to refactor). Skip reviewer-deep (docs-only / mechanical fix fast-track). Skip maintainer-pr (alignment uncontested).

**Gate 5 — CI green**: green-ci verifies `cargo xtask fmt --check` passes on the PR HEAD SHA.

**Gate 6 — Merge**: ops merges. Then cascades `gh pr update-branch` to unblocked queue.

**Gate 7 — Learn**: Skipped (no novel learning captured by upstream agents). If this were the first time the master fmt cascade pattern was encountered, wisdom would run — but it's a documented pattern (`feedback_master_bit_rot_cascade_8plus_pattern`). Orchestrator notes no new learning.

---

### Example 2 — Docs-only PR (like refactoring this very document)

**Scenario**: A PR adding or rewriting a `docs/reference/` file. No Rust code changes.

**Gate 1 — Identify**: Minimal. Scout files issue. Accuracy-scout may verify that referenced file paths in the doc exist. Research-verifier may verify external spec references (e.g., LSP spec section numbers).

**Gate 2 — Spec**: Lighter than for code changes. Plan-reviewer approves the doc outline. Skip architecture-reviewer (no crate boundary changes). Skip advocatus-diaboli if the doc's existence is uncontested. Skip spec-planner if the doc structure is simple enough to proceed without a `.spec/` file.

**Gate 3 — Build**: Builder writes the doc. No red-tdd. No green-tdd (no testable logic). Gate 3 is a single builder pass.

**Gate 4 — Review/Improve**: reviewer checks for doc quality, correct cross-references, no broken links. Skip reviewer-deep (docs-only fast-track). Skip refactor-planner. maintainer-pr may run briefly to confirm the doc fits project direction. diff-auditor confirms no Rust code artifacts.

**Gate 5 — CI green**: green-ci verifies `cargo xtask fmt --check` passes (xtask fmt covers markdown in some configurations).

**Gate 6 — Merge**: ops merges.

**Gate 7 — Learn**: Gate 7 may produce one memory entry if the doc revealed a process gap (e.g., "we discovered the reference docs were missing the gate model — filing this as a doc debt pattern"). For a routine doc update, Gate 7 is typically skipped. No novel learning captured upstream → nothing to consolidate.

---

### Example 3 — Parser-correctness PR (like #7062 or a typical parser fix)

**Scenario**: A PR fixing a parser bug where a specific Perl construct is mishandled. Requires understanding of Perl semantics, LSP interaction, and potential for regressions across the corpus.

**Gate 1 — Identify**: Full Gate 1. Scout finds the misparse and files a rough spec. Accuracy-scout verifies file:line in the parser, confirms the function name is correct. Research-verifier checks Perl semantics — does Perl actually allow that construct in that context? Which perldoc section applies?

Throughout Gate 1, agents capture learning: "accuracy-scout found that the original line number in the scout's report was off by 40 lines — this is a recurring pattern with parser scouts." This gets noted in the agent wrapup.

**Gate 2 — Spec**: Full Gate 2. Oppositional-planner asks: is this the right fix location, or should the lexer handle it? Advocatus-diaboli checks: does this affect the user path, or is it a test-only regression? Architecture-reviewer confirms: fix goes in the parser, not as a workaround in the LSP layer. Maintainer-issue verifies: the affected construct is in the top-10 parsing failure bucket. Plan-reviewer synthesizes and produces a spec. Spec-planner creates impl branch.

**Gate 3 — Build**: Full Gate 3. red-tdd writes a test that fails with the current parser. Builder implements the fix and verifies the red test turns green. Green-tdd adds corpus-pressure tests and boundary cases (e.g., nested constructs, heredoc interaction).

During Gate 3, builder notes in their wrapup: "the spec didn't anticipate the heredoc interaction — had to extend the fix. Future specs for parser fixes should explicitly ask: does this construct interact with heredocs?" This is a `## Pattern:` candidate.

**Gate 4 — Review/Improve**: Full triangulation.
- Axis 1 (right thing): reviewer confirms fix matches the reported misparse; maintainer-pr confirms the fix doesn't narrow parser scope.
- Axis 2 (codebase needs): refactor-planner checks for deduplication opportunities in the new parser path. green-refactor applies them.
- Axis 3 (right way): reviewer-deep audits the parser logic for edge cases (does the fix interact with v2 parser compatibility? does it affect the PEG grammar alignment?); diff-auditor confirms the diff is clean and matches the spec.

**Gate 5 — CI green**: green-ci verifies. Parser PRs often trigger the corpus gate — green-ci verifies that `just cpan-corpus-check` passes too, not just unit tests.

**Gate 6 — Merge**: ops merges. Runs `just cpan-corpus-ratchet` after merge (parser fix merges require this).

**Gate 7 — Learn**: Gate 7 runs. Wisdom reads the full trail:
- Gate 1 pattern: "scout line numbers drift for parser bugs" → updates or creates a memory entry about accuracy-scout being especially important for parser scouts
- Gate 3 pattern: "spec didn't cover heredoc interaction" → files a follow-up issue: "improve parser fix spec template to require: 'does this interact with heredoc/string delimiters?'"
- Gate 4 finding: "reviewer-deep caught a PEG grammar alignment concern the builder missed" → notes in memory: "parser fixes need explicit PEG grammar check in spec"

These memory entries become inputs to future Gate 2 specs for similar parser fixes — the conveyor gets smarter.

---

## Relation to Existing CLAUDE.md Routing

The label-based routing in CLAUDE.md (Pre-plan-review, Pre-build, Post-build) is the **default sequence** within and across gates. That sequence is correct for full-gate PRs. The gate model is the **meta-frame** that explains when to follow the default and when to skip.

Use CLAUDE.md's label-based routing as the default. Consult this document when the PR's nature suggests a gate or agent within a gate is not relevant.

**Future work** (not in scope for this PR):
- Update each agent definition (`.claude/agents/*.md`) to reference its gate
- Add orchestrator routing logic that queries "what gate is this PR in?" rather than "what label is missing?"
- Drop redundant agents within a gate for simple PR classes
- Reconciler-derived typed routing labels (#7061)

---

*Filed as part of issue #7079: reframe pipeline routing: gates as primary structure, agents as workers within gates.*
