---
tags: [coverage-integrity, ripr, gate-logic, suppression]
repos: [perl-lsp-swarm]
related: ["#1346", "#1349", "#1336", "#1227"]
portable: true
article_asset: true
search_terms: [suppressed_by_policy, grip_class, ripr_finding_path, classification, suppressed_unclassified, static_unknown, infection_unknown, ripr_pr_summary_counts, path-suppression, continue]
---

# ripr suppression-application gap: path-check skipped before `continue` on unrecognized classification

**Date**: 2026-06
**Hazard class**: coverage-integrity
**Portable lesson**: [docs/concepts/hazard-class-invariants.md](../concepts/hazard-class-invariants.md) (Class 6)

## What happened

`xtask/src/tasks/ripr_evidence.rs` `ripr_pr_summary_counts` applied path-based policy
suppressions AFTER an early `continue` that fired on any finding whose `classification`
field was not in the four known canonical match arms (`static_unknown`, `infection_unknown`,
or any future unknown value). The finding still contributed to `severe_gaps` via ripr's own
summary totals — which are computed separately — but `suppressed_by_policy` stayed 0 for
those findings, producing false-positive `ripr+ New Gap Gate` failures on PRs whose coverage
gaps were legitimately suppressed by a `policy/ripr-suppressions.toml` entry.
Issue #1346 tracked this; PR #1349 fixed it.

## Why

The code's `let Some(canonical) = canonical else { continue };` guard was placed to skip
unclassified findings from the canonical per-bucket logic, but the path-suppression check
(`suppression_matches_finding`) lived below that guard. The architectural assumption — "if
we don't recognize the class, don't suppress" — was never stated and was wrong: path
suppressions are class-agnostic by design. The gap was invisible when #1336 landed (which
added dual-schema support for ripr 0.9.x) because no PR in the lane had a suppression for
a finding with an unrecognized classification until #1227 triggered it in CI.

## Fix

PR #1349 reorganized `ripr_pr_summary_counts`:

- **Part A (observability):** Write raw `ripr check --format json` output to
  `target/ripr/pr/raw-check.json` as a CI artifact, so the exact `findings[]` shape
  is available for future debugging without a throwaway debug branch.
- **Part B (fix):** Check `suppression_matches_finding` BEFORE the `continue`. Track
  unclassified-but-suppressed findings in a new `suppressed_unclassified` counter;
  subtract from `severe_gaps` after per-bucket suppression. All existing #1336 behavior
  (grip_class / seam.file dual-schema) is preserved.

Two new tests: `ripr_unrecognized_classification_with_suppressed_path_is_suppressed`
(suppression fires) and `ripr_unrecognized_classification_without_suppressed_path_produces_severe_gaps`
(gate retains teeth).

## Spec impact

Motivates a new checklist item for gate evaluation loops in
`docs/agents/SPEC_UPDATE_CHECKLIST.md`: "When adding a new `continue`/early-exit in a gate
evaluation loop, verify that cross-cutting policies (suppression, path exclusion) are applied
before the exit — the order of guards determines which policy silently doesn't fire."

## Portable lesson

Gate logic that processes findings in a loop must apply cross-cutting policies (suppression,
path-based exclusion) before class-specific routing. An early `continue` that is correct for
bucket accounting can silently break the cross-cutting layer if the two concerns share the
same loop iteration. The observable symptom — `suppressed_by_policy: 0` despite the path
being in the policy — is a distinct failure mode from parse errors and requires a raw artifact
(raw-check.json) to diagnose without a debug branch.

- **Pattern**: [docs/concepts/hazard-class-invariants.md](../concepts/hazard-class-invariants.md)
- **Class**: Class 6 — Coverage/Measurement Integrity
- **Generalization**: In evaluation loops, apply cross-cutting policies (suppression, path exclusion) before class-specific routing; guard order determines which policy silently fails.

## Related PRs

- [#1346](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/1346) — issue: identified the suppression-application gap
- [#1349](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1349) — fix: path-suppress before continue + raw-check.json artifact
- [#1336](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1336) — prior fix: dual-schema ripr 0.9.x suppression matching
- [#1227](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1227) — trigger PR: first to surface the gap in CI
