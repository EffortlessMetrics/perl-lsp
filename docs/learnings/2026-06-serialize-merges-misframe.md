---
tags: [multi-agent, ci, serialization, merge-velocity, rebase-robustness]
repos: [perl-lsp-swarm]
related: ["#1206", "#1230", "#1364", "#1240"]
portable: true
article_asset: false
search_terms: [serialize-merges, hold-main-still, parallel-velocity, rebase-robustness, merge-cadence, cancellation-cascade, batch-of-3, main-moves]
---

# "Hold main still" misframe: parallel velocity + rebase-robustness is the correct doctrine

**Date**: 2026-06
**Hazard class**: other (process framing / doctrine)
**Portable lesson**: [docs/concepts/serialize-merges-and-cancellation.md](../concepts/serialize-merges-and-cancellation.md)

## What happened

During the 2026-06 convergence campaign, the framing "serialize merges to hold main still"
was applied to a multi-thread high-velocity repo where dozens of PRs are in flight across
independent crates. The intent was correct (prevent cancellation cascades, keep CI clean)
but the framing was wrong: "hold main still" is not achievable when multiple pipeline leads
are active and merging from their lanes simultaneously. Attempting to serialize all merges
into a single global queue across all threads became a bottleneck and created confusion
about when a thread was "allowed" to merge. The actual fixes needed were: (a) batches of 3
with CI wait between batches (already documented), (b) rebase-robustness in agent branches
(agents must handle rebasing without breaking), and (c) reserving serialization only for
real same-file conflicts, not as a global lock.

## Why

"Hold main still" implies a global lock on the main branch. In a microcrate architecture
(~30+ independent crates), most PRs touch non-overlapping files, so global serialization
has near-zero conflict-prevention benefit and maximum throughput cost. The cancellation
cascade pattern (docs/concepts/serialize-merges-and-cancellation.md) correctly identifies
the problem as rapid concurrent pushes to the SAME branch, not independent PRs to main.
The fix for cascades is CI-paced merge batches, not a global lock.

## Fix

No code change. Doctrine correction:

1. **Correct frame**: Parallel velocity + prompt merges + rebase-robustness is the
   right operating mode for a multi-thread microcrate repo. Agents must be designed
   to rebase cleanly (pull from origin/main before committing, use conflict-free file
   sets per crate).
2. **Serialization scope**: Serialize only when two PRs touch the same files
   (real conflict risk). Detect by diffing changed-file sets before merging.
3. **Cascade prevention**: The batch-of-3 / CI-wait cadence in ops.md is sufficient
   to prevent cancellation cascades. No global queue needed.
4. **Main moves**: Accept that main moves continuously; agent robustness to rebase
   (not global lock) is the correctness mechanism.

## Spec impact

The existing `docs/concepts/serialize-merges-and-cancellation.md` already documents the
cascade mechanism and batch cadence. Add clarification: the serialization advice applies
to rapid consecutive merges FROM THE SAME THREAD, not to concurrent merges from
independent threads on non-overlapping files. A global merge lock across all threads
is an anti-pattern in a microcrate architecture.

## Portable lesson

Distinguish cancellation-cascade serialization (sequential pushes to main from one thread,
solved by CI-wait batches) from global-lock serialization (blocking all threads while one
merges, wrong for microcrate architectures with non-overlapping files). The correct doctrine
is parallel velocity + rebase-robustness + CI-paced batches, not global lock + sequential
throughput.

- **Pattern**: [docs/concepts/serialize-merges-and-cancellation.md](../concepts/serialize-merges-and-cancellation.md)
- **Class**: Process / doctrine (not a code hazard class)
- **Generalization**: Merge serialization prevents cancellation cascades within a thread; it does not require a global lock across threads on a microcrate architecture.

## Related PRs

- [#1206](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/1206) — convergence tracking issue
- [#1230](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1230) — CI: decouple coverage from integration-test pass/fail (cascade mitigation)
- [#1364](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1364) — example: fast fix-forward after main moved
- [#1240](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1240) — example: merged while review in-flight (parallel velocity)
