---
tags: [ci, coverage, observability, misclassification, incident]
repos: [perl-lsp-swarm]
related: ["#1457", "#1470", "#1469", "#1232", "#1269", "#808"]
portable: true
article_asset: true
search_terms: ["Codecov / Patch 95", "all_kind_names_contains_every_variant", "NestedVariableList", "coverage-proof-routed", "just coverage-proof", "test failure masquerading as coverage", "check name lies", "coverage job runs tests", "integration test failure in coverage gate"]
---

# Coverage-named checks must not hide test failures

**Date**: 2026-06
**Hazard class**: coverage-integrity + observability + misclassification
**Portable lesson**: None — new pattern; doctrinal guidance needed for CI agents

## What happened

PR #1457 (adding `NodeKind::NestedVariableList` variant) appeared "Codecov / Patch 95"-red
and stayed red across approximately 5 fix attempts. Multiple agents and the orchestrator
initially chased patch-coverage percentage improvements, assuming the failure was a
coverage-measurement gap. The actual cause was a TEST FAILURE inside the coverage-proof job.

The in-house coverage-proof pipeline (`just coverage-proof-routed`) runs the test suite
to gather LLVM-cov data. During that test run, the integration test
`all_kind_names_contains_every_variant` (crate `perl-ast`) FAILED: PR #1457 added the new
`NodeKind::NestedVariableList` variant to the enum and updated the `ALL_KIND_NAMES` constant,
but not the hardcoded `all_variants` fixture vector in the test (off-by-one: 69 enumerated
variants in the fixture vs 70 actual enum members). Because the test failure surfaced under
a "Codecov" check name, the failure was misclassified as a coverage-measurement problem
rather than a test-correctness problem.

## Why

A gate should fail for exactly the hazard class its name asserts. When a coverage check runs
the full test suite to gather coverage data, and a test failure fails the coverage check,
agents diagnosing the failure look in the wrong place: they assume patch % or measurement
correctness, not test assertion correctness. The check name lied about the failure class.

This is an instance of the broader "measuring-the-instrument-is-the-bug" anti-pattern:
the tool that is supposed to measure coverage became the *only place* a test failure was
observed, so the failure was misattributed to the measurement system rather than the code
under measurement.

## Fix

- PR #1457: Updated the hardcoded `all_variants` fixture vector in the test to match the
  new enum member count (symptom fix).
- PR #1470: Decoupled coverage measurement from test validation. The `coverage-proof` job
  no longer runs the full test suite; instead, coverage is gathered by a separate,
  measurement-only job. Test failures are caught by dedicated cheaper PR gates that report
  failures under correctly-named checks (e.g., "test-all-libs", not "Codecov / Patch 95").
- PR #1469: Added correctly-named PR static and test gates so failures are caught where the
  check name says they should be caught.

## Spec impact

Motivated a new hazard class and doctrinal guidance for CI agents. Added to
[docs/reference/SUBSYSTEM_HAZARD_DEFAULTS.md](../reference/SUBSYSTEM_HAZARD_DEFAULTS.md)
section "Coverage / CI subsystem" as new rows COV-6 and COV-7. Agent-instruction guidance
added to [.claude/commands/green-ci-check.md](./.claude/commands/green-ci-check.md) and
parallel guidance for reviewer-deep and pr-responder agents.

## Portable lesson

The name of a CI check commits the agent diagnosing a failure to a root-cause hypothesis.
When a coverage-named check (e.g., `Codecov / Patch 95`) fails, agents must FIRST classify
the failure before assuming patch-coverage shortfall: is it (a) a coverage shortfall, (b) a
TEST failure hidden inside the coverage job, (c) a setup/tool failure, (d) a routing skip,
or (e) an artifact-upload failure? Reading the job log is the first diagnostic step —
agents must not assume the check name alone explains the failure.

More broadly, a measurement tool should never be the only gate that catches the thing being
measured. If a coverage job runs tests to gather coverage data, a test failure will fail the
coverage gate. If coverage is the *only* place test failures are caught, agents will
misdiagnose test failures as coverage problems. The cure is to (1) decouple measurement from
validation, and (2) run measurement-only jobs that never fail on test failures, only on
measurement correctness.

- **Pattern**: New — none existing; follows the "orchestrator-substrate-model" pattern
  (docs/concepts/orchestrator-substrate-model.md) in its emphasis on naming alignment
  between gates and failure classes.
- **Class**: Observability/misclassification — agents route based on check names; lying
  names cause misrouting and wasted investigation.
- **Generalization**: A gate-name contract commits every agent to a diagnosis path. When
  the actual failure is in a different subsystem, the name lies and agents waste time.
  Decoupling measurement from validation, and naming gates after the failure class they
  catch, is the insurance policy.

## Related PRs

- [#1457](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1457) -- trigger PR: added `NodeKind::NestedVariableList` variant, fixture off-by-one
- [#1470](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1470) -- fix: decouple coverage measurement from test validation
- [#1469](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1469) -- fix: add correctly-named test gates so failures are caught with honest check names
- [#1232](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1232) -- related: earlier coverage-measurement gap (integration test lines undercounted)
- [#1269](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/1269) -- related: coverage-proof job architecture issue
- [#808](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/808) -- related: historic coverage gate architecture discussions
