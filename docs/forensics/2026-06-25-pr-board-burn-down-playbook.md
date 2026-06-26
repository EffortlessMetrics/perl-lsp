# 2026-06-25 — PR-board burn-down: classify the whole board, then execute serially

**Lens**: Turning a large, noisy PR board into an executable queue in one focused session
**Purpose**: A reusable operating model for "review / clean up / improve / merge" at scale (50+ open PRs)
**Companions**: [`2026-06-25-closure-gap-the-recurring-defect.md`](2026-06-25-closure-gap-the-recurring-defect.md), [`2026-06-25-orchestration-at-throughput.md`](2026-06-25-orchestration-at-throughput.md)

---

## The core move

The unit of work is **not** "pick a PR." It's **classify the whole board, then route, then serial-merge only the adversarially-verified-green.** No PR leaves the first pass with "looked at" as its final state — every PR exits with exactly one disposition and an exact next-step.

This session: **55 open PRs → ~40** in the first hour, plus 27 issues closed — not by emptying the board, but by converting a noisy pile into a classified, executable queue.

---

## Phase 1 — the 3-lane census (parallel, fan-out kept out of the orchestrator's context)

Three Workflows run concurrently, each returning one consolidated report:

1. **classify-non-draft** — a 2-stage pipeline. Stage 1 (cheap model) buckets each PR: `MERGE_READY` / `REBASE_REQUIRED` / `FIX_REQUIRED` (with the *exact* blocker) / `SUPERSEDED_CLOSE` / `HELD_FOR_USER`. Stage 2 (stronger model) **adversarially verifies only the MERGE_READY ones** — this is the gate that catches deleted-live-code-as-"cleanup", hardcoded-to-pass assertions, and contested-semantics riders. A PR that's green is not the same as a PR that's correct.
2. **triage-drafts** — conservative. Close *only* drafts whose work is airtight-superseded by already-merged code (cross-link the SHA). Never auto-close live WIP.
3. **completion-harvest** — disposition issues in parallel (DONE / FICTIONAL / REAL / DUP), comment findings, close airtight cases.

## Phase 2 — execute, one serial merge stream

- **MERGE_READY (verified)** → serial-merge, re-checking each head's required checks on *current* main first. **Override the classifier on policy**: an infra dependency bump can be MERGE_READY on correctness but HELD by user policy (blast radius).
- **REBASE_REQUIRED / STALE_BASE** → rebase Workflow.
- **FIX_REQUIRED** → fix wave (genuine fixes only — see below).
- **SUPERSEDED** → close with evidence. **HELD / contested / dedup-keeper** → surface to the human.

---

## Three findings that only show up at board scale

### 1. Half the board is drafts — and drafts rot into two distinct piles
27 of 55 open PRs were drafts. ~⅓ were **superseded by already-merged work** (the feature landed via another PR; the draft was never closed — the closure gap again, at the PR layer). The *live* drafts contained **mutual-duplicate clusters**: three separate PRs each implementing the same feature (rename-to-reserved-keyword ×3; a lexer symbol table ×3). Superseded-by-*merged* triage misses these because they supersede *each other*. They need a **dedup-compare** pass: keep the one that's most complete / correct / actually-integrated (e.g. the symbol-table PR wired into the parser's constructor, not the API-only ones; the rename PR that correctly allows reserved words as *variable* names), close the rest, and **preserve salvage cherry-picks** as a comment on the keeper before closing.

### 2. STALE_BASE ≠ contested (the dangerous look-alike)
A PR whose branch predates a merged *behavioral* fix carries the **old** assertion and therefore fails the very gate the merge fixed — so it *looks* like it re-introduces a wrong model. Three PRs this session tripped the `source_backed` references-tier assertion and looked like a contested change that had already been rejected once. They were just **stale**: branched before the fix that flipped the assertion, and touching none of the relevant production code. The test: **merge-base predates the fix AND the PR doesn't touch that code → stale-base → a rebase clears it.** The rebase conflict rule: *take main's assertion, never revert it.* Don't escalate a stale branch to "contested" — and don't merge a genuinely-contested change as if it were stale. Verify which, by hand, on the actual diff.

### 3. The fix wave's failure mode is the faked test
The most common `FIX_REQUIRED` blocker is an uncovered seam ("add a test"). That is exactly where an agent will write a **vacuous test** that satisfies the gate without exercising the change. Two defenses: (a) instruct the fix to be a **call-observation test through a real production caller** (and prefer a test/extraction over a suppression where a caller actually reaches the seam); (b) route every fixed PR back through the **merge-time adversarial verify**, which is where a faked test gets caught. Faking only wastes a CI cycle; it never reaches main.

---

## How to apply (orchestrator checklist)

1. **Census first.** Three parallel Workflows (classify-non-draft, triage-drafts, harvest). Don't merge ahead of the adversarial verify.
2. **One serial merge stream.** Re-verify each head on current main; parallel merge writers cascade-cancel CI.
3. **Run harvest as a standing lane.** ~40–56% of the backlog is reliably done-but-unclosed — the cheapest closure (no codegen, no cascade).
4. **Dedup the live drafts**, don't just close superseded ones. Preserve salvage on the keeper.
5. **Distinguish stale-base from contested** via merge-base before routing.
6. **Surface the human-judgment set** (held infra deps, contested design, dedup keepers) and keep executing the mechanical lanes meanwhile.
7. **Hardcode Workflow input lists** — passing them via `args` is unreliable.

---

## Applies to

Loaded for: any orchestrator running a PR-board or issue burn-down; **lead-review** / **ops** (serial-merge + reversibility); **diff-auditor** / **reviewer-deep** (the verified-green-not-just-green and stale-base-vs-contested patterns); **ensemble-curator** (draft triage + dedup). situation_id: the board is large and noisy and you need it executable.
