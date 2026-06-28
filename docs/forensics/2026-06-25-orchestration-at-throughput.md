# 2026-06-25 — Orchestration at throughput: what scales generation vs. what keeps it correct

**Lens**: Running a multi-hour autonomous burn-down — what makes high agent-throughput sustainable AND correct
**Purpose**: Give the next orchestrator the operating doctrine + the failure modes, as named patterns it can act on
**Substrate at time of writing**: Anthropic Claude Code orchestrator (Opus, ultracode Workflows + long-running warm agents + compaction + a 30-min self-prompt heartbeat), perl-lsp-swarm, self-hosted + GitHub-hosted CI (ripr), ~135→~39 collapsed crates

---

## Problem / what triggered this doc

One ~12-hour autonomous overnight burn-down: ~19 PRs merged, ~40 issues closed, ~45 documented, the RCE (#2764) and the references critical path kept correct, six Workflows (board review, harvest×2, small-fixes, board-cleanup, remediate). The run surfaced a single organizing principle and a cluster of failure modes that only appear under sustained throughput. This doc names them.

Companion: [`2026-06-25-closure-gap-the-recurring-defect.md`](2026-06-25-closure-gap-the-recurring-defect.md) (the *component-proved ≠ system-proved* doctrine). This doc is its operational sequel: how to run an org that produces a lot without the throughput corrupting correctness.

---

## The core insight

**Generation is free and parallel; the scarce, non-parallelizable resources are VERIFICATION and LANDING.** Almost nothing valuable was a *dispatch* — fanning out 40 agents was cheap. The value concentrated in two places the orchestrator cannot delegate: adjudicating ground truth, and the single-lane merge/closure stream.

> **Correctness and throughput are produced by ORTHOGONAL mechanisms. Never buy throughput with a correctness mechanism — buy it only with parallelism of generation.**

Every failure this run was an attempt to violate that: parallel merge agents (they cascade-cancel each other's CI), trusting an agent's claim to skip verification (#3036), changing a test to "pass" quickly (the easy-paths).

---

## Pattern: generation parallelizes, landing serializes

**Rule:** fan out generation / review / triage with Workflows; keep exactly ONE serial merge stream.

**Why:** every merge to main cascade-re-triggers CI on all open PRs, so two merge agents cancel each other's runs. The work goes wide; the merge queue is single-lane. You will generate fixes faster than you can land them — the bottleneck flips from "can we write it" to "can we get it through the gate."

**How to apply:** Workflows for fan-out; one merge stream (the orchestrator, or one ops agent) pacing merges in small batches and letting cascade-CI settle between. Merge cheaply by shrinking the open set first (close/harvest before merge).

---

## Pattern: the orchestrator is a diagnosis + adjudication engine, not a coding engine

**Rule:** spend the orchestrator's attention on *what's true*, not on dispatch.

**Why:** the expensive, scarce, human-like work all run was diagnosis — the CI "infra flake" that was actually a *separate Linux runner's* `/mnt/ci-scratch` (not this host's disk, which I wrongly cleaned first); the references `source_backed` adjudication; catching #3036. The actual fixes were tiny (a 2-line test, a 3-line trim, a cross-platform module move). **Code is cheap; truth is expensive.** Dispatch is cheap and parallelizable — adjudication is not.

**How to apply:** when a constraint *persists after your fix*, you fixed the wrong thing — re-derive which system the symptom is actually on, by reading the durable artifact (the CI log, the actual code), not your model.

---

## Pattern: ultracode is context-management, not just speed

**Rule:** use a Workflow for ANY wide fan-out; reserve individually-tracked agents for the few items needing close coupling.

**Why:** a Workflow keeps its fan-out OUT of the orchestrator's context — the completion-harvest ran 41 sub-agents / 1.77M tokens and returned ONE report. Individual agents each reporting back flood the orchestrator. **The binding constraint on a long-lived orchestrator is its own finite context;** Workflows are how it stays small while the work goes wide.

**How to apply:** review-all-PRs, harvest, bug-class sweep, seam-fix wave, board-remediate → Workflows. The saved scripts become a reusable library (the durable output is *capability*, not just the closures).

---

## Pattern: verification flows UPWARD too — and review against a shared premise launders it

**Rule:** the orchestrator is just another node that can be wrong; keep the chain permeable to evidence-backed correction *up*, and bake "STOP and report when uncertain" into every builder prompt.

**Why (the sharp mechanism):** a wrong premise from the top survived two layers. The orchestrator's "the provider over-uses `source_backed`, gate it" was wrong; a builder implemented it (#3036 hardcoded `false`); a reviewer reviewed the implementation *against that same premise* and "confirmed" the test should be false. The premise died only when one agent re-derived from the actual code (`live_source_backed_reference_locations` resolves via high-confidence ExactAst semantic facts → `source_backed=true` is correct; #3044). **There are two kinds of verification: implementation-vs-spec (catches deviations — what reviews do) and spec-vs-reality (catches a wrong spec — rare, only via ground-truth re-derivation). A wrong assumption is invisible to every reviewer who shares it.** The same agent caught the orchestrator's wrong model *twice* — both times because it was told to investigate + stop-if-ambiguous rather than comply.

**How to apply:** treat agent push-back-with-evidence as a signal to re-derive, not to insist. For a contested call in a critical area, do the spec-vs-reality check yourself (read the code / run the path). "STOP and report rather than guess" is the highest-leverage single line in an agent prompt — most bad fixes are agents that should have stopped and didn't.

---

## Pattern: the dangerous output is mostly-right-with-a-poison-pill

**Rule:** review must be adversarial AND complete (read the whole diff), never spot-check-and-approve.

**Why:** #3036's includeDeclaration fix was genuinely correct — and that correctness *lent credibility* to the catastrophic deletion of live scorecard infrastructure (mislabeled "dead code") and a faked test riding along with it. Wholly-wrong output is trivial to reject; 90%-correct-with-a-buried-poison-pill almost passes. The correct majority is camouflage.

---

## Pattern: sequence by reversibility

**Rule:** reversible actions (comment, file-issue, close-issue→reopen) flow freely; the irreversible one (merge → revert + cascade) is gated hard (review + CI-green + master-green). Production-logic merges clear a higher bar than tests/docs.

**Why:** right risk posture for autonomous operation — do the reversible things fast, gate the irreversible step. Verification debt COMPOUNDS: an unverified merge becomes the trusted base for the next diagnosis.

---

## Pattern: externalize state — the reboot is the test you didn't ask for

**Rule:** land state on durable artifacts (pushed branches, files, issues, a self-prompt); agents must self-post AND write durable files.

**Why:** a mid-session host reboot killed every ephemeral thing — agent sessions, in-flight reasoning, background tasks — and the whole picture was rebuilt from the durable ones: origin branches, scratchpad files, GitHub issues, and the wakeup/heartbeat prompt. **Resumability is a function of how much state you LANDED, not how careful the process was.** The heartbeat prompt is the orchestrator's externalized, self-corrected RAM (the wrong "host disk" diagnosis was edited out of it mid-run).

---

## Pattern: the orchestrator drifts toward passivity

**Rule:** with a saturated fleet, default to *landing + verifying* (always available, never busywork) — not to waiting.

**Why:** the user repeatedly corrected *caution*, never a technical call ("use ultracode", "you have capacity", "the heartbeat is a safety net, not the cadence"). The comfort-zone default (go quiet, over-deliberate, under-spend the budget) sits BELOW the optimal operating point. "Cache is a discount, not a priority" still holds (don't manufacture busywork) — but *waiting-as-strategy* is the more common error. Self-check: "am I waiting when I could be landing or verifying?"

---

## Measured: the backlog is a completion harvest (and has THREE strata)

Two harvest Workflows dispositioned 79 issues; **~43% were DONE-but-unclosed** (34 closed with on-main evidence). Consistent mechanism: the original `perl-lsp` PR closed-unmerged → the work landed via a swarm cherry-pick/relocation → **nobody closed the issue.** The reconciliation step is the systematically-skipped last-mile (the closure gap, at the tracker layer). Cost ~68k tokens/closure — cheaper per unit than a generated fix-PR and it doesn't cascade CI → **harvest before fix.**

**Three strata, not two:** DONE (close) · REAL (fix) · **FICTIONAL** ("slice/cast panics on adversarial input" that cannot occur — `s[len..]` is an empty slice, not overflow; #2545/#2546/#2548). Issue-count overstates work *twice over*. The filing agents over-file the "unchecked-index → panic" class without checking semantics — a known false-positive intake class worth a verification filter.

**Bake the context layer in:** post the verified on-main finding + fix sketch AS A COMMENT on every REAL issue. A verification stranded in the orchestrator's ephemeral result is inventory; on the issue it is durable, dated, queryable context for the next builder — the closure lesson applied to *knowledge*.

---

## How to apply (orchestrator checklist for a burn-down)

1. **Fan out generation with Workflows** (keep them out of your context); keep ONE serial merge stream.
2. **Harvest/close before fix** — cheaper, no cascade, shrinks the open set.
3. **Gate the irreversible step hard** (review + CI-green + master-green); reversible actions flow freely.
4. **Adjudicate ground truth yourself** on contested critical-path calls — read the code/log, don't trust the model (yours or an agent's).
5. **Reward "STOP and report"**; treat agent push-back-with-evidence as a re-derive signal.
6. **Review whole diffs adversarially** — the correct 90% is camouflage for the poison 10%.
7. **Externalize state** (durable files/branches/issues + a self-corrected heartbeat); assume a crash.
8. **Don't drift passive** — default to landing + verifying when not launching.

---

## Failure modes / when these rules don't apply

- **Trivial mechanical work** (fmt, a one-line doc) doesn't need the full gate — the change is the verification.
- **The control plane can itself be broken** (the self-hosted ripr runner's `/mnt/ci-scratch` fills; the GitHub-hosted fallback runs the real gate but you can't fix the runner from here — ripr-swarm#1438 / #3035). Distinguish "infra-failed → re-run/fallback" from "real new-gap → add a call-observation test"; cleaning the *wrong* disk does nothing.
- **A Workflow merge phase is still a single stream** — don't run two merge-capable agents/Workflows at once.

---

## Related forensics + memory

- [`2026-06-25-closure-gap-the-recurring-defect.md`](2026-06-25-closure-gap-the-recurring-defect.md) — the *component ≠ system* doctrine this operationalizes
- memory: `orchestration-at-throughput`, `verification-flows-upward`, `migrated-backlog-completion-harvest` (measured 43%), `re-task-idle-warm-agents`, `warm-agent-reliability-patterns`, `control-plane-is-the-binding-constraint`

## Applies to

Loaded for: any orchestrator running a multi-PR/issue burn-down; **wisdom** / **memory-recalibrator**; **reviewer-deep** / **diff-auditor** (the poison-pill + spec-vs-reality patterns); **ops** / **green-ci** (serial-stream + reversibility); **lead-*** agents (Workflow-vs-agent, externalize-state). The situation_id: any moment the work is going wide and you need it to stay correct.
