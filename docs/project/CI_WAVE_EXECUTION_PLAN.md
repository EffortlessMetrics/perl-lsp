# CI Wave Execution Plan (Post-Substrate)

This plan captures the **next independent implementation slices** after the substrate already merged in #7449, #7525, #7547, #7561, #7568, and #7569.

## Why this exists

The control-plane failure mode is no longer "missing infrastructure". The open risks are:

1. long-running status regeneration that can look idle to GitHub Actions (#7404), and
2. queue/projection logic making decisions without enough evidence.

This plan keeps each lane reviewable, testable, and independently mergeable.

## Landed substrate (do not redo)

- Per-gate timeout receipt coverage exists.
- Bounded build-plane storage contract exists (`cargo-safe`, `storage-doctor`, agent profile).
- UX receipt command registration and artifact upload are already wired.
- PR-fast planner matrix tests already cover widening/selection cases.
- Tokmd is intentionally advisory, not merge-blocking.

## Seven independent lanes (merge in this order)

1. **Update-status streaming (urgent)**  
   Fix `cargo xtask update-status --write` inactivity by streaming progress and naming subsystem failures with repro commands.
2. **CI trigger regression lint**  
   Keep label-trigger + cancel-in-progress regressions from re-entering merge-critical workflows.
3. **Expected-skip / stale-check normalizer**  
   Normalize checks into `passed|failed|pending|expected_skip|unexpected_skip|stale` for reconciler consumption.
4. **Review receipt → label projection**  
   Reconciler projects labels from current-SHA review receipts, repairs contradictory labels, and ignores stale receipts.
5. **PR disposition receipt / closure guard**  
   Require machine-readable evidence before duplicate/superseded/absorbed/extracted closure decisions.
6. **Merge-train planner / receipt**  
   Add train-level pre-merge verification protocol and receipt output; stop on stale/conflict/red/unsafe-skip.
7. **Tokmd advisory stabilization**  
   Keep tokmd useful and diagnosable while remaining non-required.

## Lane-level definition of done

Every lane should satisfy all of the following:

- One concern per PR, with focused tests.
- No workflow timeout-only workaround for #7404 (must stream progress).
- No "all skips are green" and no "all skips are red" shortcuts.
- No bulk stale/age closure logic without structured evidence links.
- No CI weakening and no required-check policy broadening unless explicitly scoped.

## Reviewability contract

For each lane PR:

- Include exact reproduction commands in failure paths and/or docs.
- Include fixtures/tests for the new decision surface.
- Keep advisory systems advisory unless a separate policy PR changes required checks.
- Prefer extending existing receipt/schema/normalization seams over introducing a parallel framework.

## Operator note

When spawning parallel agents, ensure each lane starts from current `master` and does not depend on sibling PRs.
