---
title: "Red-TDD produced invalid red: tests that passed immediately or failed for wrong reasons"
date: 2026-06
tags: [tdd, red-tdd, verification, stochastic-pipeline, test-validity]
related_prs: ["#1372", "#1445", "#1338"]
search_terms:
  - red-tdd-invalid-red
  - invalid-red
  - false-red
  - test-passes-immediately
  - verify-the-instrument
  - stochastic-ready-pipelines
  - red-tdd-verification-gate
  - builder-validation
  - green-tdd-validation
---

## What happened

In three separate PRs during the 2026-06 campaign, the red-tdd stage produced tests that were
not genuinely red in the required sense:

- **#1372**: The red-tdd agent wrote tests asserting the new behavior. One test passed immediately on the
  unmodified codebase because the test assertion was checking a condition that was already true. The test
  was red in title but green on current HEAD — the builder inherited a "failing" test that was already
  satisfied without any implementation.

- **#1445**: Tests were red, but failed for the wrong reason: a compilation error in the test file itself
  (a missing import), not a behavior gap. The builder spent a pass fixing the import rather than
  implementing the feature. The red was real but uninformative.

- **#1338**: A test was written as red against the wrong function signature. The function had been
  refactored between the time the red-tdd agent read the spec and the time it committed the tests.
  The test failed at compile time, not at runtime, for a signature mismatch — not a missing behavior.

## Why

Red-TDD is a stochastic pass: it reads a spec and writes tests without implementing the feature. It has
no feedback loop that checks whether the tests fail for the right reason. A test that fails at compile
time, or that passes on unmodified code, satisfies the surface requirement ("tests are red") without
satisfying the actual requirement ("tests are red because the specified behavior is absent").

The stage was producing claims ("these tests fail because the behavior is missing") that were not
verified against ground truth before being handed to the builder.

## Fix

The red-tdd agent should verify each test after writing it:

1. Run the test suite on the unmodified codebase.
2. Confirm each new test fails.
3. Confirm the failure reason is a behavior gap (assertion failure, return value mismatch), not a
   compilation error, import error, or pre-existing satisfied condition.
4. Only commit tests that are red for the correct reason.

The green-tdd agent (post-builder) should also flag tests that are trivially green — tests whose
assertions would pass on a removed implementation — as a secondary verification gate.

## Spec impact

The `SPEC_UPDATE_CHECKLIST.md` §Red-TDD validation row was updated to include an explicit criterion:
"Each red test must fail with a behavior assertion failure, not a compilation failure or a vacuously
satisfied condition." The `hazard-class-invariants.md` entry for test-validity (class 5) was extended
to cover "test passes before implementation" as a named failure mode.

## Portable lesson

In a stochastic pipeline, a stage that produces claims without verifying them is an instrument without
a calibration check. The red-tdd claim is "these tests define the missing behavior." That claim must
be verified against the current codebase before the next stage acts on it. The verification is cheap
(run the tests, read the failure message); the cost of skipping it is one full builder pass on a
mis-specified target.

See `docs/concepts/stochastic-ready-pipelines.md` for the broader posture: treat every pipeline
artifact — including test files — as a claim with a reliability profile. The red-tdd test file is
a claim that requires a ground-truth check before it is handed downstream.

## Related PRs

| PR | Description |
|----|-------------|
| #1372 | Red test passed on unmodified codebase; builder inherited phantom failing state |
| #1445 | Red test failed at compile time (wrong import), not at runtime |
| #1338 | Red test failed at signature mismatch after refactor, not behavior gap |
