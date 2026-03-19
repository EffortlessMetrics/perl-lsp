# Issue-PR Crosslink Archaeology
## How GitHub Links Became Recoverable Swarm Memory

This note goes one layer deeper than
[ISSUE_PR_GENEALOGY_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/ISSUE_PR_GENEALOGY_ARCHAEOLOGY.md).
The genealogy note shows that issues and PRs became a shared delivery ledger.
This note focuses on the mechanism that made that ledger recoverable later:
explicit crosslinks in PR bodies and issue bodies.

The important historical point is that the repository did not stop at "PRs
close issues." It also started creating issues whose job was to remember what a
PR meant:

- review-summary issues
- follow-up stabilization issues
- ops audit and friction issues
- learning issues
- article issues

That is why later sessions can often recover the work from GitHub itself rather
than from chat history.

All counts in this note were verified from local GitHub CLI snapshots on
`2026-03-19`:

- full PR ledger: `gh pr list --state all --limit 2000`
- issue sample ledger: `gh issue list --state all --limit 400`

---

## 1. The Loop Starts Immediately

The earliest explicit closure in the PR ledger is:

- [PR #20](https://github.com/EffortlessMetrics/perl-lsp/pull/20)
  `ci: fix flaky cancellation tests by conditionally ignoring in CI`
- body: `Fixes #15`

That already gives the PR a lineage anchor.

The issue side responds immediately too:

- [issue #16](https://github.com/EffortlessMetrics/perl-lsp/issues/16)
  `Lexer: support single-quote delimiters for s/// operator`
- body cites `PR #3`

- [issue #21](https://github.com/EffortlessMetrics/perl-lsp/issues/21)
  `Make LSP cancellation tests deterministic (remove cfg(ci) ignores)`
- body cites `PR #20` as the temporary fix and `PR #15` as the original attempt

So from the first week of visible history, the repo already has a loop:

- PR says what issue it resolves
- issue says what prior PR did and why it was insufficient

That is the seed of recoverable swarm memory.

---

## 2. Q3 Uses Issues To Remember Review Outcomes

By September 2025, the issue tracker is doing more than recording bugs. It is
also recording what PR review discovered.

The strongest early example is:

- [issue #157](https://github.com/EffortlessMetrics/perl-lsp/issues/157)
  `Integrative Review Summary: PR #153 findings and follow-up actions`

Its body is not a backlog request. It is an explicit memory artifact for PR
`#153`:

- overall merge verdict
- gate results
- follow-up issues created from the review
- tag trail from the integrative pipeline

That is historically important because the issue is storing review memory in a
searchable GitHub object rather than leaving it inside agent output or PR
comments alone.

This same pattern appears again in:

- [issue #198](https://github.com/EffortlessMetrics/perl-lsp/issues/198)
  `Stabilize Test Infrastructure: Fix 17 Ignored Tests from PR #176`

There the issue body turns a closed PR into a structured queue of remaining
stabilization work. The issue is downstream of the PR, not upstream.

That is a big shift. The issue tracker is no longer only where work starts. It
is also where the repo remembers what a PR left unfinished.

---

## 3. March 2026 Uses Issues As Ops Memory

By March 2026, crosslinks are doing operational memory work too.

Two representative examples:

- [issue #1667](https://github.com/EffortlessMetrics/perl-lsp/issues/1667)
  `audit(swarm): cycle 2 improvements & protocol gaps`
- [issue #1678](https://github.com/EffortlessMetrics/perl-lsp/issues/1678)
  `friction: cycle 2 operational friction log — 14 items`

These are not feature issues. They are swarm-memory issues.

They cite PRs as evidence for protocol or operational lessons:

- `#1667` cites PR `#1555` as the concrete red-CI example that exposed a
  protocol gap
- `#1678` cites PR `#1665` for the fixed file-split anti-pattern and PR
  `#1555` for merge-discipline friction

That means GitHub issues are now remembering:

- which PR exposed the problem
- what operational lesson it taught
- what protocol change should follow

This is a more advanced use of crosslinks than simple closure language. The
issue is functioning as an institutional memory object for the swarm.

---

## 4. The Archive Quantifies The Pattern

Across the full `2000`-PR archive slice:

- `71` PRs use explicit `Closes`, `Fixes`, or `Resolves` language

Across the sampled `400`-issue ledger:

- `32` issues mention PRs in the body

That sample is not just one issue type. It contains at least four distinct
memory functions:

1. **Classic follow-up**
   Example: `#21` cites `#20` and `#15`

2. **Review-summary memory**
   Example: `#157` cites `#153`

3. **Ops/protocol memory**
   Examples: `#1667`, `#1678`

4. **Learning/publication memory**
   Examples: `#2190`, `#2191`, `#2195`, `#2197`

That last category is where the repo becomes especially unusual.

---

## 5. Learning Issues Turn PRs Into Reusable Experience

The learning issues are explicit about their function:

- [issue #2190](https://github.com/EffortlessMetrics/perl-lsp/issues/2190)
  `learning: parser fix agent experience report (#1700)`
- body cites [PR #2040](https://github.com/EffortlessMetrics/perl-lsp/pull/2040)

- [issue #2191](https://github.com/EffortlessMetrics/perl-lsp/issues/2191)
  `learning: parser fix agent experience report (#1703)`
- body cites [PR #2180](https://github.com/EffortlessMetrics/perl-lsp/pull/2180)

These issues are not asking for more work. They are preserving:

- what debugging method worked
- what the real root cause was
- what helper or parser trap another agent should know next time

That is exactly the kind of context that is usually lost in chat-native work.
Here it survives because the issue cites the PR directly.

---

## 6. Article Issues Turn PRs Into Publication Receipts

The article issues use the same crosslink mechanism, but for launch-story
evidence.

Representative examples:

- [issue #2195](https://github.com/EffortlessMetrics/perl-lsp/issues/2195)
  `article: Corpus-Driven Parser Development — Testing Against 4,355 Real CPAN Files`
- body cites [PR #2039](https://github.com/EffortlessMetrics/perl-lsp/pull/2039)

- [issue #2197](https://github.com/EffortlessMetrics/perl-lsp/issues/2197)
  `article: The Self-Improving Swarm — How Our Development System Learns From Every Session`
- body cites issue and memory artifacts as evidence for cycle-to-cycle learning

That matters because article planning is no longer detached from implementation.
The writing issues are anchored in concrete PRs and operational artifacts.

So the crosslink system is doing double duty:

- engineering memory
- publication memory

---

## 7. Why This Makes Sessions Recoverable

The recovery mechanism is straightforward:

1. issue number preserves task or lesson identity
2. PR number preserves implementation identity
3. explicit close/fix language binds them mechanically
4. downstream issues preserve the next lesson, audit, or publication use

That means a later session can often reconstruct the story from GitHub alone:

- open the original issue
- open the linked PR
- open the follow-up learning or audit issue

This repo therefore ends up with a linked memory graph rather than a plain
backlog:

- issue -> PR
- PR -> follow-up issue
- PR -> learning issue
- PR -> article issue

That graph is one of the main reasons this codebase is so archaeologically
legible.

---

## Evidence Pointers

- [ISSUE_PR_GENEALOGY_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/ISSUE_PR_GENEALOGY_ARCHAEOLOGY.md)
- [ISSUE_ROUTING_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/ISSUE_ROUTING_ARCHAEOLOGY.md)
- [PR_LIFECYCLE_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/PR_LIFECYCLE_ARCHAEOLOGY.md)
- [PR_REVIEW_LOOP_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/PR_REVIEW_LOOP_ARCHAEOLOGY.md)
- [issue #21](https://github.com/EffortlessMetrics/perl-lsp/issues/21)
- [issue #157](https://github.com/EffortlessMetrics/perl-lsp/issues/157)
- [issue #198](https://github.com/EffortlessMetrics/perl-lsp/issues/198)
- [issue #1667](https://github.com/EffortlessMetrics/perl-lsp/issues/1667)
- [issue #1678](https://github.com/EffortlessMetrics/perl-lsp/issues/1678)
- [issue #2190](https://github.com/EffortlessMetrics/perl-lsp/issues/2190)
- [issue #2191](https://github.com/EffortlessMetrics/perl-lsp/issues/2191)
- [issue #2195](https://github.com/EffortlessMetrics/perl-lsp/issues/2195)
- [issue #2197](https://github.com/EffortlessMetrics/perl-lsp/issues/2197)
- `71` PRs with explicit close/fix/resolves language in the full archive snapshot
- `32` issues in the sampled archive with PR references in the body
