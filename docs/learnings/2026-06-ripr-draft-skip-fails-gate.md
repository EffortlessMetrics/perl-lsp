---
tags: [ci, gate-logic, ripr, routing, draft-pr]
repos: [perl-lsp-swarm]
related: ["#1578", "#1574", "#1556", "#1555", "#1512", "#1511", "#1558"]
portable: true
article_asset: false
search_terms: [ripr gate, draft PR, skip, route_result, ROUTE_RESULT, skipped, New Gap Gate, gate failure, draft evaluation, router skip, evaluator failure]
---

# ripr+ New Gap Gate fails on draft PRs when router skips them

**Date**: 2026-06
**Hazard class**: ci / gate-logic
**Portable lesson**: [docs/concepts/gate-names-must-match-failure-classes.md](../concepts/gate-names-must-match-failure-classes.md)

## What happened

Draft PRs systematically fail the `ripr+ New Gap Gate` check even when the code is locally correct and the ripr runner was never invoked. The pattern is repeatable: a draft PR is opened, GitHub dispatches the ripr workflow, the router evaluates the PR, returns `ROUTE_RESULT=skipped` (empty `ROUTER_TARGET`), and the downstream gate evaluator then FAILS the check with exit code 1. The failure appears on multiple draft PRs: #1578, #1574, #1556, #1555, #1512, #1511, #1558. All are in draft state with `needs-ci-fix` label and a red ripr gate.

The gate's name — "New Gap Gate" — implies it measures coverage gap discovery; the failure mode — returning failure when no route is taken — contradicts that semantic. A skipped dispatch should be neutral or passing (especially for drafts, which are not intended for CI), not a hard failure.

## Why

The workflow dispatch logic has two steps: (1) router determines which target(s) should run (if any), and (2) evaluator reads the result and synthesizes a gate status. The router correctly skips draft PRs (the dispatch routing decision is sound: drafts should not trigger the coverage gate). However, the evaluator treats `ROUTE_RESULT=skipped` as a failure condition, converting a skipped route into a failed gate.

The architectural assumption appears to be: "if there is no route target, the gate has failed." This is inverted from the intent: "if the route was intentionally skipped, the gate should pass" (or be neutral/skipped in the check rollup).

This is a gate-logic cross-cutting issue: the **evaluation loop's exit condition does not account for the semantics of intentional skips**. A skipped route is a routing decision, not an evaluation failure.

## Fix

**Not yet applied.** The fix is a routing/evaluation logic change, not a code change. The evaluator must distinguish between:

1. **Skipped route** (draft, or non-matching filter): gate should PASS or be marked SKIPPED in the check rollup
2. **Route executed, found issues**: gate should FAIL with the findings
3. **Route executed, no issues**: gate should PASS

Current behavior conflates (1) and (2). The fix is to emit a PASS or SKIPPED conclusion when `ROUTE_RESULT=skipped`, and FAIL only when the route executed and found gaps.

Likely location: `xtask/src/tasks/ripr_evidence.rs` or the ripr workflow file (`.github/workflows/ripr*.yml`), in the gate-evaluation section that reads `ROUTE_RESULT` and decides the final check conclusion.

## Spec impact

This incident motivates a new entry in `docs/agents/SPEC_UPDATE_CHECKLIST.md` (section 5, "Agent / workflow behavior"):

> When a workflow has a router stage that can skip (due to filter, draft status, opt-out label, etc.), the downstream evaluator must have an explicit condition for skipped routes. A skipped route is not a failure. The gate conclusion must distinguish: SKIPPED (route did not apply) vs. PASS (route applied, no findings) vs. FAIL (route applied, findings found).

Also: audit all gates that have a router/conditional dispatch step for this pattern. The issue may recur on other gates.

## Portable lesson

Gate evaluation logic that has a conditional router must have a ternary conclusion path, not a binary one. When the router says "I don't apply here," the evaluator must respect that and emit SKIPPED or PASS, not collapse it into FAIL. The gate's name must match its failure classes: "New Gap Gate" should fail on gaps, not on non-application.

- **Pattern**: [docs/concepts/gate-names-must-match-failure-classes.md](../concepts/gate-names-must-match-failure-classes.md)
- **Class**: CI gate-logic; routing evaluation closure
- **Generalization**: Skipped routes are routing decisions; gates must have distinct code paths for "route not applicable" vs. "route applied with findings."

## Related PRs

- [#1578](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1578) — test(lsp-folding); draft; ripr+ New Gap Gate FAILED
- [#1574](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1574) — test(document-links); draft; ripr+ New Gap Gate FAILED
- [#1556](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1556) — feat(lsp); draft; ripr+ New Gap Gate FAILED
- [#1555](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1555) — feat(lsp); draft; ripr+ New Gap Gate FAILED
- [#1512](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1512) — chore(branding); draft; ripr+ New Gap Gate FAILED
- [#1511](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1511) — docs(roadmap); draft; ripr+ New Gap Gate FAILED
- [#1558](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1558) — docs(NODEKIND); draft; ripr+ New Gap Gate FAILED
