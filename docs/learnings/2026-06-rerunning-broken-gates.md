---
tags: [ci, gate-logic, stochastic-pipeline, verification, instrument, observability]
repos: [perl-lsp-swarm]
related: ["#1457", "#1470", "#1469"]
portable: true
article_asset: true
search_terms: [gate failure, repeated failure, stale result, instrument broken, coverage gate, re-run loop, suspect the gate, rerunning PR, CI instability, #1457, #1470, #1469]
---

# A gate that fails repeatedly on verified-correct content is the bug

**Date**: 2026-06
**Hazard class**: CI pipeline / instrument configuration (measurement integrity)
**Portable lesson**: [docs/concepts/verify-the-instrument.md](../concepts/verify-the-instrument.md)

## What happened

PR #1457 (#1469 follow-up) added a new `NodeKind` variant and extended all
consumers. The implementation was locally verified to be correct by the builder
and reviewer. However, the CI coverage gate repeatedly failed on this PR over
~5 consecutive re-runs (~42 minutes total). Each time the PR was re-pushed, the
gate failed again. The failure was deterministic: the same gate, the same PR,
the same error.

The gate failure was eventually diagnosed as a false-positive in the coverage
toolchain's profdata collection scope — the gate was correct code operating on
incomplete/stale evidence. The PR itself was not broken; the instrument measuring
it was misconfigured.

## Why

When a CI gate fails, the reflex is to "run it again" or "push a fix to the code."
This heuristic is correct when failures are stochastic or when the code is
actually broken. However, when the same gate fails on the same branch over
multiple runs without any code changes between runs, the failure pattern indicates
a gate problem, not a code problem:

- If the code were broken, rerunning the tests without code changes would not fix
  the failure.
- If the code is correct and the gate is deterministic, the gate should pass on
  re-run.
- A gate that fails reproducibly on correct code is malfunctioning.

Rerunning a broken gate multiple times is wasted CI cycles and wall-clock time.
It also trains the team to rationalize gate failures as "noise" — which is how
catastrophic merges happen. Each false-positive teaches the team to discount
the gate's red signal.

## Fix

The diagnostic step was to **stop rerunning the gate and instead read the gate's
logic and scope**. The coverage gate's profdata collection was scoped narrowly
(integration tests were excluded from the profdata being analyzed), creating a
false-negative on coverage for changes that only touched library code. PR #1470
fixed the gate scope; PR #1457 then passed on the next CI run without any code
changes.

The fix was a substrate correction (gate scope), not a builder correction.

## Spec impact

Added a new entry to `docs/reference/SUBSYSTEM_HAZARD_DEFAULTS.md` (CI/process
row): "When a gate fails ≥2 times on the same PR without code changes, classify
the failure — suspect the gate, read its logic and scope, diagnose the failure
mode (profdata scope, target mismatch, stale cache, etc.), and fix the gate.
Do not blindly re-push until the gate itself is verified to be green on locally
verified-correct code."

## Portable lesson

The cost of a broken measurement instrument is higher than the cost of a
temporarily slower pipeline. Rerunning a gate N times to make it pass is
amortization in the wrong direction — it buys you one PR at the cost of
training the team to discount red signals forever.

- **Pattern**: [docs/concepts/verify-the-instrument.md](../concepts/verify-the-instrument.md)
- **Class**: CI/measurement integrity — the instrument reading (gate exit code)
  diverges from the ground truth (code correctness)
- **Generalization**: A gate that fails repeatedly on verified-correct content is
  an instrument failure, not a code failure. Suspect the gate, diagnose it,
  and fix it. Rerunning a broken gate trades immediate convenience for long-term
  gate credibility.

## Related PRs

- [#1457](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1457) — PR
  where the `NodeKind` variant was added; green-tdd added red tests and coverage
  gate repeatedly failed on this PR
- [#1470](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/1470) — fix-forward
  issue filed for the broken coverage gate scope; fixed in PR #1470
- [#1469](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1469) —
  spec/contract follow-up: static checks on PR branches to detect gate
  configuration drift
