---
tags: [shift-left, dap, hazard-invariants, deep-review, validation]
repos: [perl-lsp-swarm]
related: ["#1246", "#1340", "#1339", "#1219", "#1327", "#1337"]
portable: false
article_asset: true
search_terms: [shift-left, hazard-class-invariants, SPEC_UPDATE_CHECKLIST.md, 0-fix-deep-review, adversarial-tests, acceptance.md, test_evaluate_with_out_of_range_frameid_no_panic, test_evaluate_stopped_session_frame_not_found_returns_error, test_evaluate_stale_frameid_after_resume_rejected]
---

# Shift-left validated: first PR with hazard invariants front-loaded got 0-fix deep-review

**Date**: 2026-06
**Hazard class**: N/A (positive outcome -- shift-left working)
**Portable lesson**: [docs/concepts/shift-left-ladder.md](../concepts/shift-left-ladder.md)

## What happened

PR #1340 added hazard-class invariants to the spec system: six classes in
docs/agents/SPEC_UPDATE_CHECKLIST.md section 8, plus instructions to spec-planner,
red-tdd, and architecture-reviewer to enumerate and test applicable classes before
the builder starts. The first PR built under this regime was #1246 (DAP frameId
validation). The spec included explicit acceptance rows for bounds, protocol-safety,
and stale-after-resume. Red-tdd wrote adversarial tests for each. Deep-review (sonnet)
found zero correctness gaps: "found no correctness gaps beyond what the tests already
covered; shift-left was effective."

## Why it worked

Front-loading hazard invariants into the spec means the builder tests are designed
to catch adversarial cases before implementation, not just verify the happy path.
Deep-review becomes a confirmation net (verifying no gaps were missed) rather than a
discovery pass (finding the bugs for the first time). Contrast with same session,
pre-invariant PRs: #1219 (ref collision, fix required), #1337 (test-encodes-bug, fix
required), #1327 (scanner literal-blind, fix required).

## Fix

Not a bug fix -- a positive validation. The lesson: front-loading hazard invariants
into acceptance criteria and adversarial tests converts deep-review from discovery
(expensive, late) to confirmation (cheap, fast).

## Spec impact

This incident is the empirical proof that the approach in docs/concepts/shift-left-ladder.md
works. It motivates the ongoing discipline: for every change, spec-planner enumerates
applicable hazard classes; red-tdd writes the adversarial tests; architecture-reviewer
verifies the spec has the rows before the builder starts.

## Portable lesson

The shift-left ladder works: front-loading hazard invariants into spec acceptance criteria
and adversarial tests moves deep-review from primary catcher to confirmation net. The
cost of the front-load (one pass to enumerate applicable classes) is less than the cost
of one deep-review fix cycle.

- **Pattern**: [docs/concepts/shift-left-ladder.md](../concepts/shift-left-ladder.md)
- **Class**: Positive validation -- all six hazard classes
- **Generalization**: Spec acceptance criteria enumerated from recurring classes eliminate late discovery; one-time class enumeration beats per-PR triage.

## Related PRs

- [#1246](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1246) -- first PR built with hazard invariants front-loaded (0-fix deep-review)
- [#1340](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1340) -- added hazard-class invariants to spec system
- [#1339](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/1339) -- issue: front-load hazard invariants into spec system
- [#1219](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1219) -- pre-invariant: ref collision caught late by deep-review
- [#1327](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1327) -- pre-invariant: scanner blindness caught late by deep-review
- [#1337](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1337) -- pre-invariant: test-encodes-bug caught late by deep-review
