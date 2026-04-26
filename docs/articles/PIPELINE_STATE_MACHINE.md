# Pipeline State Machine

## What this doc is

The perl-lsp orchestration pipeline is a **label-driven state machine**. Labels on a GitHub issue or PR are the authoritative state — presence of a label means an agent signed off, absence means that pass hasn't happened yet. The orchestrator routes work purely by querying what's missing.

This doc makes the state machine explicit so agents don't have to re-derive it from CLAUDE.md prose or guess transitions.

## Why labels (not files, not a DB)

- **GitHub is the only surface multiple concurrent maintainers (human + AI swarm) can observe coherently.** Local `.claude/local/receipts/` files drift between worktrees and machines.
- **Label state is queryable cheaply.** `gh issue list --label X` replaces scraping comment bodies.
- **Labels are resumable.** A swarm cycle that crashes mid-pass leaves labels intact; the next orchestrator tick queries what's missing and picks up.
- **Labels survive amnesia.** New agents arrive with no memory of prior work but can reconstruct state from labels in one query.

This is load-bearing. `memory/feedback_labels_as_state_machine.md` captures the design lesson.

## Label taxonomy

### Sign-off labels (`<agent>-reviewed` = pass complete)

Set by the agent after its pass succeeds. Presence = signed off. Never remove except to re-queue a pass.

| Label | Set by | Means |
|---|---|---|
| `accuracy-reviewed` | accuracy-scout | Mechanical facts verified (file paths, function names, module existence) |
| `research-reviewed` | research-verifier | External claims verified (Perl semantics, LSP spec, crate APIs) |
| `oppositional-reviewed` | oppositional-planner | Approach challenged; alternatives surfaced |
| `diaboli-reviewed` | advocatus-diaboli | Existence challenged — BUILD/DEFER/CLOSE verdict |
| `architecture-reviewed` | architecture-reviewer | Design fits microcrate layering and dependency contracts |
| `maintainer-issue-reviewed` | maintainer-issue | Issue aligns with project goals, roadmap, user base |
| `plan-reviewed` | plan-reviewer | Spec refined and approved |
| `spec-reviewed` | spec-planner | Impl branch created with `.spec/` files |
| `red-tdd-reviewed` | red-tdd | Failing tests committed on impl branch |
| `green-tdd-reviewed` | green-tdd | Edge case and regression tests added |
| `review-reviewed` | reviewer | Standards pass complete (banned patterns, scope, formatting) |
| `maintainer-pr-reviewed` | maintainer-pr | PR implementation fits project direction and quality bar |
| `pr-responded` | pr-responder | Bot comments and CI failures addressed |
| `refactor-planner-reviewed` | refactor-planner | Simplification/reuse plan posted |
| `green-refactor-reviewed` | green-refactor | Implementation simplified while tests stay green |
| `deep-reviewed` | reviewer-deep | Correctness check passed — required before merge |
| `ci-green` | green-ci | All CI checks pass on current HEAD SHA |
| `diff-audited` | diff-auditor | Cumulative diff coherent, clean, matches spec |

### State labels (where is this now)

Mutually exclusive within a category. Exactly one is present at a time for a live work item.

| Label | Set by | Means |
|---|---|---|
| `builder-ready` | plan-reviewer | Spec finalized — ready for build pipeline |
| `in-build` | builder | Builder actively working |
| `in-review` | reviewer | PR in review process |
| `merge-ready` | orchestrator | All gates passed — ready for ops merge |
| `already-fixed` | any agent | Close without build |

### Routing labels (`needs-<action>` = work requested)

Set when a pass found a problem downstream can fix. Removed when the fix lands.

| Label | Set by | Means |
|---|---|---|
| `needs-plan-review` | scout | Entry to verification pipeline |
| `needs-deep-review` | reviewer | Standards done, deep review needed |
| `needs-builder-fix` | green-tdd | Edge case test found bug — back to builder |
| `needs-ci-fix` | green-ci | CI check failed or stale — to pr-responder |
| `needs-diff-fix` | diff-auditor | Diff has artifacts/regressions/drift — to pr-responder |

### Meta labels (non-state)

| Label | Purpose |
|---|---|
| `structural-blocker` | Blocks parallel work |
| `follow-up-recommended` | Needs follow-up issue |
| `swarm-discovered` | Found by automated sweep |
| `size/S`, `size/M`, `size/L` | Effort estimate |

## Issue pipeline (idea → builder-ready)

```
scout files issue
      │  sets needs-plan-review
      ▼
┌─────────────────────────────────────────────┐
│  Pre-plan-review verification (sequential)  │
│                                             │
│  accuracy-scout    → +accuracy-reviewed     │
│  research-verifier → +research-reviewed     │
│  oppositional-pl.  → +oppositional-reviewed │
│  advocatus-diaboli → +diaboli-reviewed      │
│    verdict: BUILD / DEFER / CLOSE           │
│  architecture-rev. → +architecture-reviewed │
│  maintainer-issue  → +maintainer-issue-r.   │
│    verdict: ALIGNED / DEFERRED / OUT OF SCOPE│
└─────────────────────────────────────────────┘
      │  all six sign-offs present
      ▼
plan-reviewer       → +plan-reviewed
      │  sets builder-ready (removes needs-plan-review)
      ▼
   builder pipeline →
```

The verification agents run **sequentially**, not in parallel — each reads and builds on the previous agent's comments. Running them out of order wastes tokens and produces worse synthesis. See `memory/feedback_verification_is_sequential.md`.

**Entry condition:** `needs-plan-review` present, no `-reviewed` labels yet.
**Exit condition:** all six `*-reviewed` labels + `plan-reviewed` present, `builder-ready` set.
**Abort paths:**
- diaboli verdict `CLOSE` → issue closed
- maintainer verdict `OUT OF SCOPE` or `MISALIGNED` → issue closed
- any `already-fixed` → issue closed
- diaboli `DEFER` / maintainer `DEFERRED` with named precursor → remove `needs-plan-review`, wait

## Build pipeline (builder-ready → PR opened)

```
issue has builder-ready
      ▼
spec-planner  → creates impl/ branch
              → writes .spec/ files
              → +spec-reviewed
      ▼
red-tdd       → commits failing tests on impl branch
              → +red-tdd-reviewed
      ▼
builder       → checks out impl branch (spec + red tests)
              → implements, makes tests green
              → opens PR
              → sets in-build on issue
              → in-review on PR
```

**Entry condition:** `builder-ready` on issue.
**Exit condition:** PR opened, `in-build` on issue, `in-review` on PR.

## PR pipeline (PR opened → merged)

```
PR has in-review
      ▼
┌───────────────────────────────────────────────┐
│  Post-build hardening (sequential)            │
│                                               │
│  green-tdd    → +green-tdd-reviewed           │
│                 (may set needs-builder-fix)   │
│  reviewer     → +review-reviewed              │
│                 (fixes-forward directly)      │
│  maintainer-pr→ +maintainer-pr-reviewed       │
│  pr-responder → +pr-responded                 │
│  refactor-pl. → +refactor-planner-reviewed    │
│  green-refact → +green-refactor-reviewed      │
└───────────────────────────────────────────────┘
      ▼
reviewer-deep → +deep-reviewed
              (final correctness gate)
      ▼
green-ci      → +ci-green  (verifies current HEAD SHA)
              (may set needs-ci-fix → back to pr-responder)
      ▼
diff-auditor  → +diff-audited
              (may set needs-diff-fix → back to pr-responder)
      ▼
orchestrator  → sets merge-ready
      ▼
ops           → waits for green on current SHA, merges
              → wisdom retrospective
```

**Entry condition:** `in-review` on PR, PR not draft.
**Exit condition:** PR merged, wisdom retrospective committed.
**Abort paths:**
- `needs-builder-fix` → back to builder, remove `green-tdd-reviewed` on fix
- `needs-ci-fix` → back to pr-responder, remove `ci-green`
- `needs-diff-fix` → back to pr-responder, remove `diff-audited`

## Invariants

These must hold; violations are bugs.

1. **`merge-ready` requires both `ci-green` and `diff-audited`.** Any agent that sets `merge-ready` without these two present is bypassing gates. reviewer-deep specifically must NOT set `merge-ready` — that's the orchestrator's job after green-ci + diff-auditor sign off. See `memory/feedback_deep_reviewer_premature_merge_ready.md`.

2. **Sign-off labels are only removed to re-queue.** If a label gets stripped, the associated pass must re-run. Never strip a sign-off as a shortcut to skip a pass.

3. **State labels are mutually exclusive within category.** An issue is `builder-ready` OR `in-build` OR `already-fixed`, not two at once.

4. **`needs-<X>` labels are transient.** They exist only while work is requested; the agent that fixes the problem removes the label when it commits the fix.

5. **Verification is sequential, not parallel.** The six pre-plan-review agents read each other's comments. Running them concurrently produces contradictory verdicts from agents that didn't see each other's work.

6. **Presence of a sign-off implies the SHA it was signed off on is still current.** `ci-green` on a stale SHA is not `ci-green`. green-ci verifies the current HEAD SHA, not a cached check-run result.

## Query patterns

The orchestrator reads state, never writes it directly. Queries:

```bash
# Issues entering the pipeline
gh issue list --label "needs-plan-review" --state open

# Issues fully verified, ready for plan-review
gh issue list --label "needs-plan-review" \
              --label "accuracy-reviewed" \
              --label "research-reviewed" \
              --label "oppositional-reviewed" \
              --label "diaboli-reviewed" \
              --label "architecture-reviewed" \
              --label "maintainer-issue-reviewed" --state open

# Builder-ready issues
gh issue list --label "builder-ready" --state open

# PRs ready to merge
gh pr list --search "label:merge-ready"

# Stale in-build (stuck pipeline)
gh issue list --label "in-build" --state open --search "updated:<2026-04-17"
```

## Failure modes this machine prevents

- **Double-build.** `in-build` is a mutex. Two builders can't both claim the same issue.
- **Bypassed verification.** An issue missing any of the six `*-reviewed` labels cannot get `builder-ready` — the orchestrator's query won't route it to plan-reviewer.
- **Stale green.** `ci-green` is a SHA-verified receipt. A merged PR with `ci-green` from an earlier SHA fails re-verification before ops merges.
- **LGTM-only reviews.** Multiple sign-off labels (`review-reviewed`, `deep-reviewed`) force at least two agents to engage, and each is required to push concrete improvements (see `feedback_deep_review_roi.md`).
- **Lost work at agent death.** Labels persist; if an agent crashes mid-pass, the next orchestrator tick sees the missing label and re-queues the pass.

## When the state machine needs to grow

Don't add labels speculatively. A new label pays rent only when:

1. A recurring failure mode isn't caught by existing labels.
2. An orchestrator query needs a distinction current labels don't express.
3. A new agent joins the pipeline with a distinct pass.

Adding a label is cheap in isolation but expensive cumulatively — each label adds one more thing the orchestrator and every agent must understand. Prefer reusing existing labels or carrying the distinction in comment bodies until the recurring failure pattern is clear.

## Related

- `CLAUDE.md` — operational pipeline overview (shorter, less formal)
- `memory/feedback_labels_as_state_machine.md` — design lesson
- `memory/feedback_verification_is_sequential.md` — why the six pre-plan-review agents run in order
- `memory/feedback_deep_reviewer_premature_merge_ready.md` — merge-ready invariant violation
- `memory/feedback_orchestrator_follows_pipeline.md` — no direct-merge shortcuts
