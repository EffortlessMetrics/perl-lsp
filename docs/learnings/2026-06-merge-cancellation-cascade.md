---
tags: [ci, serialization, codecov, cancellation, merge-queue]
repos: [perl-lsp-swarm]
related: ["#1206", "#1230"]
portable: false
article_asset: true
search_terms: [Codecov-upload-cancelled, Codecov-Patch-95-failure, INPUT_TOKEN, concurrent-rebase, merge-queue-cancellation, upload-coverage, coverage-proof-routed]
---

# Concurrent merges triggered Codecov upload cancellation cascade

**Date**: 2026-06
**Hazard class**: ci-tooling
**Portable lesson**: [docs/concepts/serialize-merges-and-cancellation.md](../concepts/serialize-merges-and-cancellation.md)

## What happened

During the 2026-06 convergence campaign, multiple PRs were rebased and update-branches
pushed within short windows of each other. The Codecov upload step runs after the main CI
job completes and requires several minutes. Concurrent pushes caused the CI runner to
cancel still-running Codecov uploads when new CI runs were triggered. Codecov recorded a
failure for the cancelled run even though coverage was correct and the local quality gate
had passed. PR #1206 was the most affected, its Codecov step cancelled repeatedly by
concurrent activity on adjacent PRs.

## Why

The Codecov upload step is long-running and does not hold a CI slot after the main build
completes. Any push to the same repository triggers CI run cancellation at the runner
level, killing any still-running upload from a prior push. This is a property of how
GitHub Actions cancels in-progress runs on new pushes to the same branch.

## Fix

Serialize merges: complete one full CI cycle (including upload steps) before starting
the next merge. Merge one PR, watch all checks go green on the merge commit SHA, then
merge the next. Do not rebase adjacent PRs while one CI cycle is running.

## Spec impact

Motivated the merge-cadence guidance in 
and the "Merge Queue Protocol" section in  (batches of 3, wait for CI).
Also motivated .

## Portable lesson

In CI systems that cancel superseded runs, concurrent merges cause cascading cancellations
of long-running post-build steps. The fix is always structural: enforce a merge cadence
of one CI cycle at a time, not a workaround (sleep, retry, ignore the failure).

- **Pattern**: [docs/concepts/serialize-merges-and-cancellation.md](../concepts/serialize-merges-and-cancellation.md)
- **Class**: Process hazard (not a code hazard)
- **Generalization**: Long post-build steps in CI are vulnerable to concurrent-push cancellation; serialize merges to protect them.

## Related PRs

- [#1206](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1206) -- most-affected PR (Codecov cancelled repeatedly)
- [#1230](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/1230) -- issue: Codecov upload failure fails required check even when quality gate passed
