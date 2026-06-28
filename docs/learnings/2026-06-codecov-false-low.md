---
tags: [coverage-integrity, codecov, ci, profdata, lib-vs-integration]
repos: [perl-lsp-swarm]
related: ["#1282", "#1263", "#1321", "#1327", "#1223", "#1238"]
portable: false
article_asset: true
search_terms: [workspace-lib, lib-profdata, patch-coverage, LCOV, coverage_filters, integration-test-undercounted, Codecov-Patch-95, coverage-proof-routed, quality_baseline.rs, cargo-llvm-cov, false-low-coverage]
---

# Codecov false-low: --lib profdata only; integration-test lines undercounted

**Date**: 2026-06
**Hazard class**: coverage-integrity
**Portable lesson**: [docs/concepts/hazard-class-invariants.md](../concepts/hazard-class-invariants.md) (Class 6)

## What happened

The Codecov / Patch 95 gate ran cargo llvm-cov with coverage_filters = ["workspace-lib"],
counting only --lib profdata. Integration tests in crates/*/tests/ ran and exercised
changed lines, but their coverage profdata was excluded from the patch-coverage measurement.
PRs with fixes genuinely covered by integration tests showed false-low patch coverage
(< 95%) even though the lines had 10+ LCOV hits from integration suites. Affected:
#1223 (method-decl hover), #1238 (workspace rename). A companion failure mode (same
session, different root cause): PR #1321 (test-only conformance matrix) failed Patch-95
because the gate measured coverage of the TEST LINES the PR added -- a dead branch inside
an inline cfg(test) block -- addressed by #1327.

## Why

The coverage_filters setting in quality_baseline.rs filtered to workspace-lib scope,
which excludes the --tests profdata. The filter was added to reduce noise from unrelated
crate coverage, but it had the side effect of excluding integration-test hits from the
patch-coverage percentage for the changed files.

## Fix

Workaround for affected PRs: use inline cfg(test) lib tests (counted by --lib profdata)
to cover changed lines. Do NOT use LCOV_EXCL_* padding -- that masks a real gap. The
systemic fix (including --tests profdata in patch-coverage LCOV merge) is tracked by
#1282 and #1263 (dual-impl consolidation).

## Spec impact

Motivated Class 6 (Coverage/Measurement Integrity) in docs/concepts/hazard-class-invariants.md.
Added to SPEC_UPDATE_CHECKLIST.md: coverage transformations must not drop production lines;
test that a known production line survives the transformation.

## Portable lesson

The measuring instrument is often the bug, not the code. When a required CI check blocks
correct, well-tested code, diagnose the measurement before assuming a code gap. Distinguish
"real gap" (line truly not covered) from "tool limitation" (profdata scope excludes the
runner that covers it).

- **Pattern**: [docs/concepts/hazard-class-invariants.md](../concepts/hazard-class-invariants.md)
- **Class**: Class 6 -- Coverage/Measurement Integrity
- **Generalization**: Coverage gate failures on correct code are a measurement calibration problem, not a coverage gap.

## Related PRs

- [#1282](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/1282) -- issue: coverage counts only --lib profdata
- [#1263](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/1263) -- refactor: consolidate dual coverage-routing implementations
- [#1327](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1327) -- fix: strip inline cfg(test) blocks from patch-coverage LCOV
- [#1223](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1223) -- affected PR: method-decl hover, integration-covered but measured low
