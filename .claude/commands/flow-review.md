---
description: "Flow: two-tier review of a PR (standards + correctness)"
argument-hint: "<pr-number>"
---

# Flow: Review

Run two-tier review on PR **#$ARGUMENTS**.

## Steps

1. **Tier 1 — Standards review (haiku, fast):**
   ```
   Agent(
     subagent_type: "reviewer",
     prompt: "Review PR #$ARGUMENTS. Follow your todo list.",
     model: "haiku",
     name: "reviewer-$ARGUMENTS"
   )
   ```
   Checks: banned patterns, scope, formatting, test presence.
   If fails → routes feedback to builder, stops flow.

2. **Tier 2 — Correctness review (sonnet, deep):**
   ```
   Agent(
     subagent_type: "reviewer-deep",
     prompt: "Deep review PR #$ARGUMENTS. Follow your todo list.",
     model: "sonnet",
     name: "reviewer-deep-$ARGUMENTS"
   )
   ```
   Checks: logic correctness, edge cases, regression risk.
   If fails → routes feedback to builder, stops flow.

3. If both tiers pass → PR is approved and marked ready.
   Auto-chain: the PR is now eligible for `/flow-merge`.

## What a successful flow produces

An approved, non-draft PR with:
- Standards review passed (haiku)
- Correctness review passed (sonnet)
- Edge case follow-ups filed as issues (if any)
- PR marked ready for merge
