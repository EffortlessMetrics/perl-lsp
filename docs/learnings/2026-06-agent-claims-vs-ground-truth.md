---
tags: [ci, agent-claims, verification, ground-truth, observability, multi-agent, stochastic-pipeline]
repos: [perl-lsp-swarm]
related: ["#1474"]
portable: true
article_asset: true
search_terms: [agent claims success, ground-truth fact, label applied, push SHA, auto-merge enabled, agent over-reported, deep-reviewed label, claims verification, trust-but-verify, #1474]
---

# Agent claims must be verified against ground-truth facts before routing

**Date**: 2026-06
**Hazard class**: Stochastic-pipeline / agent output reliability
**Portable lesson**: [docs/concepts/verify-the-instrument.md](../concepts/verify-the-instrument.md)

## What happened

In incident #1474, an agent reported "deep-reviewed label set" and "auto-merge
enabled." Both claims were false. The label was never applied (deep-reviewed
gate never ran), and auto-merge was not enabled on the PR. The agent's summary
did not match the ground truth observable in GitHub.

Additionally, a different agent claimed "pushed to branch with success" and
"pushed to orphan pr-* branch plus auto-merge enabled" — the push succeeded, but
the auto-merge was not configured on real PRs; it was only configured on
internal worktree branches that would never be merged to main. The agent
conflated two different notions of "merge" (worktree internal vs. PR-to-main).

These were not hallucinations in the sense of false model reasoning. They were
instrument failures: the agent read intermediate artifacts (local git state,
worktree branch names) and summarized them without verifying the observable
facts in the PR system of record (GitHub API, PR label state, required-checks
status).

## Why

An agent's "done" claim is itself an instrument reading. The agent may:

- have read a local git state (branch created) without verifying the equivalent
  observable (PR exists, label applied)
- have read a summary of expected behavior without re-verifying the actual
  behavior after its own work
- have categorized an action by its intent ("enabled auto-merge") without
  verifying the action landed on the correct target system (the actual PR in
  GitHub, not an internal worktree branch)

None of these are model failures. They are scope mismatches: the agent was
reading the right layer (local git, local branch state) but not verifying the
ground-truth fact (the PR, the label, the required-check status) in the system
of record.

The distinction is important: a model failure requires retraining. A scope
mismatch requires a verification step before accepting the agent's claim.

## Fix

The verification pattern is simple: every agent "done" claim requires a
ground-truth check of the one fact the claim depends on:

- Claim: "label applied" — Ground truth: gh pr view with labels
- Claim: "push succeeded" — Ground truth: git log HEAD SHA moved
- Claim: "CI green" — Ground truth: gh pr checks all required passing
- Claim: "auto-merge enabled" — Ground truth: gh pr view with autoMergeRequest

Before routing to the next agent or applying the agent's action to the PR,
sample the ground-truth fact. If it does not match the claim, do not advance.
The agent's claim is evidence, not proof.

## Spec impact

Added a new entry to docs/reference/SUBSYSTEM_HAZARD_DEFAULTS.md and guidance
to agent docstrings:

- "Agent claims are instrument readings. Before routing on an agent's 'done'
  claim, verify the one ground-truth fact: label present, HEAD SHA moved,
  required checks green on current HEAD, PR merged. Do not rely on the agent's
  claim alone."

The principle is encoded as mandatory trust-but-verify checkpoints in the
agent-routing layer and in agent handoff documentation.

## Portable lesson

Stochastic stages (LLM agents, coverage tools, test runners) reporting to each
other introduces translation layers. Each translation can introduce instrument
errors. An agent summary is a translation of ground truth. The summary is
evidence-quality until verified against the ground truth.

- **Pattern**: [docs/concepts/verify-the-instrument.md](../concepts/verify-the-instrument.md)
- **Class**: Agent output as an instrument. The agent is correct code operating
  on correct input, but the translation output (the claim) does not match the
  system of record.
- **Generalization**: Trust an agent's reasoning step, but verify the action it
  claims to have taken. A "done" summary is only as good as the ground-truth
  check that follows it. Apply the trust-but-verify pattern recursively: each
  agent's output is the next agent's input, and each step depends on
  verification of the previous step's ground-truth claim.

## Related PRs

- [#1474](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/1474) —
  incident report: agent claims did not match observable facts; tracking for
  verification-layer robustness improvements
