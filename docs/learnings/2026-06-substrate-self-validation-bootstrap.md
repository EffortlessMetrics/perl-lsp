---
tags: [ci, substrate, bootstrap, recursion, incident, gate-logic, self-validation]
repos: [perl-lsp-swarm]
related: ["#1469", "#1477", "#1478", "#1479", "#1484", "#1485"]
portable: true
article_asset: true
search_terms: [substrate fix broke master, gate change pre-merge testing, merge-gate skipped on PR, full tier validation, gate self-test recursion, cannot test gate through broken gate, cascading failures, planning.role, compile_all_targets timeout, gate-graph issue]
---

# You cannot validate a gate-fix through the broken gate

**Date**: 2026-06
**Hazard class**: CI / bootstrap / recursion / gate-logic
**Portable lesson**: [docs/concepts/enforcement-over-doctrine.md](../concepts/enforcement-over-doctrine.md), [docs/concepts/substrate-tax-and-red-is-a-smell.md](../concepts/substrate-tax-and-red-is-a-smell.md)

## What happened

PR #1469 aimed to make cheap gates (cargo check, clippy) run on PRs for early signal. The change touched the merge-gate tier definition. The PR landed on main without running the full merge-gate tier pre-merge (standard practice is to skip the merge-gate tier when reviewing PR-scoped changes, assuming the gate-change itself will be validated post-merge).

The fix broke master **three times in a row** with distinct failures:
1. **#1478**: Missing `planning.role` field in gate-graph serialization caused deserialization crash.
2. **#1479**: `compile_all_targets` timeout set too tight; PR timeouts on slower runners.
3. **#1484 / #1485**: Earlier gate-graph structure issues surfaced post-merge.

Each failure emerged in the live merge-gate tier, one at a time, discovered only after landing. The cascade took 3+ hours and 7+ fix-forward commits.

## Why

**CI/substrate changes have a unique risk profile** that code changes do not:

1. **Cannot be tested pre-merge the way code can.** A new function in a library PR can be unit-tested before merge. A gate-change cannot be validated through the broken gate itself — that is circular. The gate must be tested *outside* the broken gate or *through the working merge gate post-merge*, both of which are expensive or impossible.

2. **Affect every PR.** A gate-change immediately impacts the merge process for all downstream work. Failures block the entire pipeline, not just the PR that introduced the change.

3. **Failures cascade serially.** When a gate fix is wrong, the next PR merge discovers the next failure. There is no batching opportunity; each merge surfaces one failure at a time, in sequence.

4. **The substrate fix paid the very substrate tax it existed to remove.** PR #1469 aimed to reduce latency (add cheap checks on PRs to catch issues earlier). The fix's own flaw was invisible pre-merge, so it cascaded into the merge process, creating the exact problem it aimed to solve: slower signal, delayed feedback, repeated failures.

The root was a planning gap: substrate changes require a different validation contract than code changes. Skipping the full merge-gate tier pre-merge works for code PRs; it fails for gate-change PRs.

## Fix

Two levels of response:

**Immediate (within PR #1477→#1485)**: Each failure was diagnosed post-merge by reading the live gate logs, then fixed in a follow-up commit: added missing field, loosened timeout, fixed gate-graph structure.

**Durable (process change)**: Until the bootstrap loop is fixed, gate-change PRs must run the **full merge-gate tier locally before merge**, not just the gate tier that the PR changed. This is the opposite of the normal "trust the gates" doctrine: it is precisely where the automation cannot help (you cannot test the merge-gate *through the merge-gate*), so humans must validate.

And: **expect a cascade; do not assume one fix is the last.** If a gate-change merge surfaces a failure, assume there are 1–2 more failures waiting. Merge only one gate-change per merge window, with the full merge-gate tier validated beforehand.

## Spec impact

The MAINTAINER_AGENT_DOCTRINE should include a gate-change acceptance row:

> When reviewing a gate-change PR: Do not skip the merge-gate tier pre-merge. Run the full merge-gate locally on the PR branch before approving. Gate-changes cannot validate through the broken gate; they require pre-merge manual validation.

This rule is encoded as an agent-instruction trigger in the spec-planner and maintainer-pr docstrings, not merely in prose. See [enforcement-over-doctrine.md](../concepts/enforcement-over-doctrine.md) for why prose-only rules fail.

Additionally, the spec hazard row for gate-changes should be added to [SUBSYSTEM_HAZARD_DEFAULTS.md](../reference/SUBSYSTEM_HAZARD_DEFAULTS.md):

> **Gate-change acceptance criteria**:
> - Full merge-gate tier passes locally on the PR branch (not just the gate being changed)
> - No timeout changes without validation on slower runners (e.g., macOS CI tier)
> - Gate-graph serialization is backward-compatible with in-flight PRs
> - If the change affects merge-blocking gates, expect a 2–3 hour cascade post-merge (budget accordingly)

## Portable lesson

Substrate-layer changes (CI gates, build tooling, measurement instruments) have a bootstrap problem: you cannot validate a tool through itself when the tool is broken. Code changes enjoy the privilege of being testable pre-merge; substrate changes do not.

The workaround is constraint and manual validation: keep substrate changes small (one gate at a time), run the full stack locally before landing, and budget for a cascade of post-merge failures (each one teaches you what the next test should have caught).

The real fix is a *meta-gate*: a separate CI check that validates gate-changes by running them in isolation, on a test corpus, without the stakes of the live merge. Until that exists, treat gate-changes with higher caution than code changes.

- **Pattern**: [docs/concepts/enforcement-over-doctrine.md](../concepts/enforcement-over-doctrine.md) (rules without enforcement are theater) + [docs/concepts/substrate-tax-and-red-is-a-smell.md](../concepts/substrate-tax-and-red-is-a-smell.md) (the cost of moving bugs to the merge tier)
- **Class**: Bootstrap recursion. A layer trusted to validate itself cannot validate its own fixes. Substrate-layer tools pay this cost when they are the primary merge gate.
- **Generalization**: When a tool is responsible for its own validation (merge-gate validates merge-gate, coverage-tool validates coverage-tool, test-runner validates test-count), expect bootstrap asymmetries. Manual intervention or a separate validation layer is required.

## Related PRs

- [#1469](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1469) — Make cheap gates run on PRs (the change that broke master)
- [#1477](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1477) — Fix #1: missing planning.role field
- [#1478](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/1478) — Issue tracking: planning.role field gap
- [#1479](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1479) — Fix #2: compile_all_targets timeout too tight
- [#1484](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1484) — Fix #3: gate-graph structure issue
- [#1485](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1485) — Follow-up: additional gate-graph robustness
