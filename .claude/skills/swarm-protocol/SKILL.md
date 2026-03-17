---
name: swarm-protocol
description: Load shared swarm behavioral rules for coordinators and workers. Use for task routing, receipts, direct messaging, dedup, worktree boundaries, and lifecycle discipline.
user-invocable: false
---

# Swarm Protocol

Shared behavioral rules for swarm coordinators and workers.

## Autonomy

Fix adjacent same-slice issues in the current worktree when they are clearly part
of the task. For anything outside the slice, create a GitHub issue or handoff
instead of stretching the worker.

Use `swarm-discovered` for discovered issues and `swarm-architectural` when the
repo needs a deliberate architecture decision.

## Direct Communication

Message the teammate who should act next. Do not bounce everything through the
lead.

- builder -> reviewer when a branch is review-ready
- reviewer -> builder when feedback needs a fresh implementation worker
- ops -> fixer or validator when CI or trust checks fail
- any lane -> improver when repeated docs/tests/devex friction appears

## GitHub-Native State

Use GitHub and tracked swarm state as the durable ledger:

- open PRs and issues
- `.claude/swarm-state/`
- handoffs and receipts
- merge and review state

Do not treat transcript memory as proof.

Use `.claude/swarm-state/findings.json` for durable control-plane conclusions.
Use `discovered-issues.md` for product leads and `known-pitfalls.md` for
repeatable failure lessons.

## Metrics And Receipts

After each task, leave a durable receipt:

- what changed
- what verification ran
- what passed or failed
- what remains open

When the lane uses `.ops-perl-lsp/swarm-metrics.jsonl`, append an entry instead
of rewriting history.

## Dedup Before Starting

Check the queue and state before creating new work:

1. open PRs
2. open issues
3. `.claude/swarm-state/`
4. existing handoffs and receipts

If the slice already exists, update or route it instead of duplicating it.

If the work changes how the swarm should describe or operate itself, update the
findings ledger instead of hiding that conclusion in chat or a one-off handoff.

## Worktree And Worker Boundaries

- code mutation implies an isolated worktree
- one worker owns one PR-shaped unit of change
- retire and respawn when the crate, file surface, verification loop,
  permissions, or branch target changes materially
- keep review one-PR-at-a-time

## Agent Lifecycle

- shut workers down when no concrete next action exists
- keep warm contexts only when the same lane and same near-term slice benefit
  from reuse
- do not treat `Stop` as proof of completion
- small, green receipts beat long ambient context

## Review Discipline

- one review context per PR
- read the handoff and receipt before the diff
- send materially different fixes back to builder instead of mutating the scope
  inside review

## Research Discipline

When facts matter, look them up or spawn the right research worker. Do not
guess at language rules, protocol details, or dependency behavior.
