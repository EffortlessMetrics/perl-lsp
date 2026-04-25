# 2026-04-25 — Process & Repo Meta-Learnings

**Window**: Observations from the 2026-04-25 session and the prior 3-day arc that didn't fit naturally into the per-session forensics
**Operating mode**: Pattern-level analysis above the file-level memory entries

This doc captures **process patterns** and **repo state observations** that emerged during high-volume orchestration but are general-purpose enough to outlive any single session. Each pattern includes a "when does this matter" line so future operators can decide whether to apply it.

---

## Process patterns

### 1. Asymmetric two-LLM workflow: Codex bursts upstream, Anthropic ladder downstream

The current operating model is genuinely asymmetric:

- **Upstream** (Codex web, ChatGPT-side): operator produces 4-5 candidate PRs per issue via web threads. Cost: cheap per-PR (~5min wall-clock per Codex thread). Quality: hallucination-prone, scope-drift-prone, often deletes unrelated content.
- **Downstream** (Anthropic Claude Code): orchestrator runs the full verification ladder (research → oppositional → architecture → maintainer-issue → plan-review → standards → maintainer-PR → diff-audit → deep-review). Cost: expensive per-PR (10-30 sub-agent runs per cluster). Quality: ~100% bug-catch rate on real correctness issues.

The asymmetry is the point: cheap generation + expensive selection beats either approach alone. The 2026-04-25 session validated this with ~108 PRs closed (selection win) and 7 specific bugs caught by deep-review (verification win).

**When does this matter**: when planning the operator's time allocation. The 5-min Codex burst that produces 5 PRs needs ~30-min of orchestrator dispatch + ~2 hours of agent runtime to triage. Total throughput is ~25 PRs/hr if everything is healthy, dropping to 5-10 PRs/hr when the verification ladder catches a real bug requiring fix-forward.

---

### 2. Verifier-of-verifier: dispatching a second agent specifically to verify the first agent's claim

When an agent's verdict feels suspicious, dispatching a second agent narrowly scoped to "verify claim X" caught real false positives twice in one session:

- **Maintainer-PR batch L** claimed 8 PRs were "branch-contaminated" (3943 files, 213k+ adds for #6051). A verifier-of-verifier agent used REST `gh api repos/.../pulls/N` (authoritative additions/deletions) and confirmed all 8 were CLEAN at their actual diff size (#6051 = exactly 21 lines).
- **Hermes sweep** flagged 8 PRs for `.hermes/` cleanup. Corrective audit checked work-id attribution (does the subdir under `.hermes/conveyor/` match the PR's branch work-id?) and found #5750 was a false positive (legitimate self-attributed audit trail).

**Pattern**: when a cheap-model agent's verdict has high blast radius (closing PRs, stripping labels), a second agent at higher model tier ($N$ minutes runtime) is cheap insurance vs. the cost of correcting wrong-closures. The "expensive insurance against cheap verdicts" inversion is justified by blast-radius asymmetry.

**When does this matter**: any verdict that triggers a destructive action (close/strip-label/admin-merge). Read-only verdicts (add diff-audited, add review-reviewed) don't need the verifier-of-verifier pass.

---

### 3. Direct orchestrator API ops as the cheap finisher

When sub-agents are blocked (org quota, GitHub rate limit), the orchestrator can still execute high-leverage actions directly via `gh` CLI in 1-2 second per call. Saturday session demonstrated this when the org monthly Anthropic limit was hit:

- 16 direct gh CLI calls in ~4 minutes wall-clock
- Accomplished: 6 label applications, 1 label strip + comment, 2 ci-green labels, 3 needs-ci-fix labels, 3 PR closures with cross-refs, 1 promotion, 1 auto-merge invocation
- Equivalent sub-agent dispatch would have taken ~4 agents and 15+ minutes of agent-runtime + token cost

**Pattern**: for clearly-scoped mechanical actions (label changes, status comments, closures with predetermined rationale), the orchestrator should execute directly rather than dispatch sub-agents. Sub-agents earn their cost on judgment calls (which PR is the keeper, what's the bug, is this contaminated), not on label paperwork.

**When does this matter**: end-of-session label catchup, post-quota-recovery cleanup, executing the "TODO list" that prior sub-agents documented but couldn't push.

---

### 4. Forensics docs as session-level memory layer

The repo accumulated 7+ forensics docs over the 4-day arc (2026-04-22 through 2026-04-26 priorities). They serve a function distinct from the file-level memory entries:

| Layer | Lifetime | Granularity | Purpose |
|---|---|---|---|
| **Memory entries** (`feedback_*.md`, `project_*.md`) | Indefinite | Per-pattern | Persistent rules to apply |
| **Forensics docs** (`docs/forensics/YYYY-MM-DD-*.md`) | Session-bounded | Per-session | "This is what happened, here's the data" |
| **PR descriptions / commit messages** | Tied to commit | Per-change | "This is what this PR does" |

The forensics layer is what makes session-handoff actually work. A new operator (or the same operator after a quota reset) can read the prior session's forensics doc and pick up routing decisions from real data, not from re-discovering the queue state.

**When does this matter**: any session that produces non-trivial outcomes — write the forensics before tearing down. The 2026-04-25 session produced 4 forensics docs (3 by sub-agents during the session, 1 comprehensive after) and they materially shape next-session priorities.

---

### 5. Promotion-sweep iteration pattern

Promotion sweeps (find PRs with full sign-off chain, add merge-ready) have to iterate, not run once. Saturday session ran 4 sweeps:

| Sweep | Found | Why |
|---|---|---|
| #1 (early) | 0 | Initial chain-completion candidates already had merge-ready |
| #2 (mid-wave) | 2 (#6357, #6179) | Standards/maintainer reviews from this wave just landed |
| #3 (late wave) | 7 (#5541 + 6 test PRs) | Wave 2 maintainer-pr batch B's 7-PR cluster all chain-completed simultaneously |
| #4 (post-quota direct) | 1 (#5320) | Final maintainer-pr-reviewed catchup unlocked the chain |

**Pattern**: sign-offs land asynchronously across waves. Promotion sweeps need to run after each batch of label-applying agents return, not as a one-shot. Same applies to ops drain.

**When does this matter**: any wave that includes maintainer-pr or standards review agents. After they return, immediately re-run promotion sweep.

---

### 6. Conservative-close cost asymmetry

The cost of one wrong-closure (lost engineering work, sometimes weeks of context) far exceeds the cost of N kept-too-long PRs (some disk + queue length). Two cases this session:

- Parser closeout C12: an ensemble agent collapsed 3 distinct fixes (#5988/#5989/#5990) under "duplicate title" rule. A parallel agent caught this and reopened #5989 + #5990, recovering 2 distinct parser fixes that touched disjoint files.
- C21 Unicode: original ensemble agent skipped closure because "uncertain" — correct conservative posture. Subsequent plan-review agent did the close with documented rationale.

**Pattern**: "DO NOT close anything where you're uncertain" is load-bearing in cluster-triage prompts. Better to flag for follow-up than close incorrectly.

**When does this matter**: every ensemble-curator dispatch. The instruction needs to be explicit, not implicit.

---

### 7. Parallel-agent collisions on overlapping scope

High parallelism produces real collisions:

- **#5403**: maintainer-pr review and reviewer-deep both ran against it in same wave. First applied fix-forward (cherry-pick rebase + fixture-matrix update). Second saw same issues without the fix yet, returned SEND-BACK. Net effect harmless but wasted one agent's runtime.
- **#6090**: closed by lexer/parser ensemble (38-PR closure agent) as superseded. A later reviewer-deep batch picked it from the queue (which still showed it as a candidate), reviewed it, pushed regression tests to its branch, applied `deep-reviewed` label. Agent didn't notice the PR was already CLOSED.

**Mitigation patterns observed**:
- Narrow agent prompts with explicit "skip already-covered: #N1, #N2, #N3" lists
- "Check PR state at start" instruction in reviewer-style prompts
- Agent dispatch timing: stagger ensemble vs. reviewer agents to reduce simultaneous queries against the same PR set

**When does this matter**: any wave with >10 parallel agents touching the same PR queue. Solo waves of 1-3 agents don't have this problem.

---

## Repo state observations

### 8. Workspace size and worktree overhead

- 134 workspace members in `cargo metadata`
- 173+ locked worktrees in `H:/Code/Rust/perl-lsp/.claude/worktrees/` from prior sessions (each ~50-200MB)
- Estimated agent-worktree disk overhead: 10-30 GB

The locked worktrees are *correctly* not auto-cleaned (running agents need them) but accumulate across sessions when agents are killed by quota limits before their wrap-up step runs.

**Recommendation**: a periodic worktree cleanup pass (manual review + selective `git worktree remove --force`) is necessary every 2-3 sessions. The prior `feedback_swarm_worktree_contamination.md` entry covers operational hazards; this is the disk-overhead complement.

---

### 9. Master CI workflow trigger gaps

UX Regression Gate is `on: pull_request:` only. Master pushes never run it. Result: master can develop UX regressions invisibly between PR runs. The 2026-04-25 session caught this when the perl-dap perf cluster (8 PRs) all failed UX scenario_01 with identical "LSP startup hang" pattern — it turned out to be a *real* master regression introduced by `7943a13e` ("fix(perl-dap): harden bridge adapter lifecycle"), invisible to master CI.

**Recommendation**: audit `.github/workflows/*.yml` for any `on: pull_request:` workflow that does baseline-comparison logic. Add `on: push: branches: [master]` to those. Already codified in `feedback_ci_workflow_trigger_observability_gap.md`.

---

### 10. Sustainability question: closure rate vs. creation rate

Saturday session closed 111 PRs (108 cluster + 3 superseded) and merged 12. Net delta was -83 (410 → 327). But the discrepancy means **roughly 40 new PRs were created during the session** (Codex bursts continued upstream while triage ran downstream).

Steady-state arithmetic: if Codex bursts produce ~40 PRs/day and the verification ladder closes/merges ~125/day at full intensity, the queue shrinks by ~85/day. At current 327 open PRs, full drain would take ~4 sessions of comparable intensity. But Saturday was at 91% weekly quota — sustainable cadence is closer to 1 such session per 2-3 days.

**Open question**: is the goal queue-zero or steady-state-low? If steady-state-low, the 327 → 200ish range is probably the natural equilibrium given current upstream burst rate. If queue-zero, need to either throttle upstream Codex or run multiple high-intensity sessions per week.

**Recommendation**: the next operator decision point is whether to keep the high-intensity drain pattern or shift toward upstream throttling (e.g., fewer simultaneous Codex threads, longer per-thread research time to reduce hallucination rate).

---

### 11. The "checked-in tree-sitter C tree" + "fuzz" + "archive" workspace exclusions

CLAUDE.md notes 3 explicit workspace exclusions:
- `tree-sitter-perl/` (legacy C, retained for benchmarking)
- `fuzz/` (fuzz builds)
- `archive/` (archived)

These represent technical debt (legacy C tree at ~7000 files in `git checkout`) that doesn't affect builds but does affect:
- Disk size on every worktree (each agent-worktree carries the legacy)
- `git checkout` time (the master worktree creation took ~30s for 7487 files)
- IDE indexing time

**Observation**: not actionable in the current cycle (legacy is preserved deliberately), but worth re-examining for v0.13.0 if the legacy tree is genuinely no longer providing benchmark value.

---

### 12. Memory entry count growth pattern

Memory directory at session start: ~70 entries. End of Saturday session: ~75 (5 new). Growth rate ~1-2 entries per active session.

**Pattern**: most session-end memory writes are calibration updates to existing entries, not new entries. The 5 new entries this session were exceptional (master root rebuild, CRLF, CI workflow trigger gap, CI timeout headroom, triage-at-scale validation) — a normal session adds 0-2.

**Implication**: the memory system has reached a useful density. Further growth should focus on consolidation (merging related entries) rather than addition. This was hinted at by the parallel "memory consolidation review" agent dispatched late in the session — but it ran out of quota before producing the consolidation proposal. Worth re-running next session.

---

## Open observations not yet codified

A handful of patterns surfaced this session that aren't strong enough for memory entries yet but are worth tracking for the next instance:

- **Stale `--changedFiles` metadata**: GitHub's GraphQL `changedFiles` count appears to be cached and can lag the actual diff. REST `additions/deletions/changed_files` is authoritative. Saw this on at least 8 PRs in maintainer-pr batch L's false-positive contamination claim. Worth a feedback entry if it happens again.
- **Worktree branch drift**: at session end, the main checkout was on `cherry-5695-rebased` instead of the original `codex/improve-module-documentation-coverage-k85j8k` because some cherry-pick agent switched branches. The `feedback_nested_worktree_main_switch.md` entry covers the risk but there's no codified "always restore main checkout to its starting branch at session end" pattern. Worth adding.
- **The "8-line patch saved to /tmp"**: a sub-agent for #6447 produced an exact fix (sortText serialization in completion.rs) but couldn't push to PR #6447's branch — left it on the wrong branch. The orchestrator saved it to `/tmp/pr6447-sortText-fix.patch` during cleanup. This is a recurring "agent finishes work but can't push" pattern worth thinking about — maybe a "stash to PR-attached file" workflow.

---

## Cross-references

- Comprehensive 3-day arc economics: `2026-04-25-3day-arc-economics-and-learnings.md` (sibling)
- Saturday session report: `2026-04-25-pr-queue-drain-session.md` (sibling)
- Saturday final state snapshot: `2026-04-25-session-final-state.md` (sibling)
- Next-session priorities: `2026-04-26-session-priorities.md` (sibling)
- Prior-day forensics: `2026-04-22-continuous-codex-review-session.md`, `2026-04-23-tier-wiring-reviewer-fix-forward-session.md`, `2026-04-24-extended-throughput-session-retrospective.md`

These docs together form the project's session-handoff layer. Each is dated; each is self-contained for its window; together they form the operator's working memory across sessions.
