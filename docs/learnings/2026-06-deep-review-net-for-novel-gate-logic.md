---
tags: [coverage-integrity, ripr, deep-review, gate-logic, regression-test]
repos: [perl-lsp-swarm]
related: ["#1349", "#1346", "#1336"]
portable: true
article_asset: true
search_terms: [suppressed_unclassified, over-subtract, gate-neutering, severe_gaps, no-summary-fallback, deep-review, novel-gate-logic, ripr_pr_summary_counts]
---

# Deep-review remains the net for novel gate/infra logic even after shift-left

**Date**: 2026-06
**Hazard class**: coverage-integrity
**Portable lesson**: [docs/concepts/shift-left-ladder.md](../concepts/shift-left-ladder.md)

## What happened

PR #1349 fixed the ripr suppression-application gap (#1346) by introducing a
`suppressed_unclassified` counter and subtracting it from `severe_gaps`. The fix's
"no-summary fallback" path — taken when ripr emits no per-finding summary — could
over-subtract: if `suppressed_unclassified` exceeded `severe_gaps` due to a double-count,
`severe_gaps` would underflow (saturating at 0), masking a real gap that the gate should
have caught. This is gate-neutering via a fix that was itself logically overly aggressive.
Deep-review caught it and a regression test was added before merge.

## Why

The fix for #1346 introduced new accumulation logic (`suppressed_unclassified`) in a code
path that had not existed before. Shift-left (hazard invariants in spec, red-tdd adversarial
tests) is effective for known hazard classes applied to domain logic, but the #1349 change
was infrastructure/gate logic that created a NEW hazard class (gate-neutering via
over-subtraction) not enumerated in the spec. Deep-review is designed to catch exactly this:
novel logic paths whose failure mode wasn't anticipated by the spec's hazard enumeration.

## Fix

Deep-review added a regression test asserting that `severe_gaps` cannot underflow below 0
even when `suppressed_unclassified` is large, and that real gaps are still reported when
the gate encounters a mix of suppressed and unsuppressed unclassified findings. The fix
was: saturating subtraction (subtract-to-zero, not subtract-to-negative).

## Spec impact

Reinforces the rule in `docs/concepts/shift-left-ladder.md`: shift-left prevents late
discovery for known hazard classes; deep-review remains the net for novel gate/infra
changes that introduce hazard classes not enumerated in the spec. The two are complementary,
not substitutes.

## Portable lesson

Shift-left (hazard invariants in spec + adversarial red tests) covers known hazard classes
applied to domain logic. Deep-review catches novel gate/infra logic whose failure mode is
not in the enumerated class set. The investment in shift-left reduces deep-review to a
confirmation pass for domain changes; it does NOT eliminate deep-review for infrastructure
changes that introduce new failure modes.

- **Pattern**: [docs/concepts/shift-left-ladder.md](../concepts/shift-left-ladder.md)
- **Class**: Class 6 — Coverage/Measurement Integrity (gate-neutering sub-class)
- **Generalization**: Shift-left covers known classes; deep-review covers novel logic. Both are required — shift-left makes deep-review cheaper, not unnecessary.

## Related PRs

- [#1349](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1349) — the fix where over-subtract was caught
- [#1346](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/1346) — issue that motivated #1349
- [#1336](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1336) — prior layer of the same fix chain
